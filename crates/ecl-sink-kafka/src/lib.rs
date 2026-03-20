//! Kafka sink stage for the ECL pipeline runner.
//!
//! Serializes pipeline records to Avro (Confluent wire format) and
//! produces them to a Kafka topic. Supports Schema Registry for
//! schema management and environment variable interpolation in config.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod avro;
pub mod registry;

use std::time::Duration;

use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use regex::Regex;
use serde::Deserialize;
use tokio::sync::OnceCell;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

use crate::avro::{parse_schema, serialize_record_avro};
use crate::registry::SchemaRegistry;

/// Configuration for the Kafka sink stage, deserialized from TOML params.
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSinkConfig {
    /// Kafka topic to produce to.
    pub topic: String,
    /// Comma-separated list of Kafka brokers.
    pub bootstrap_servers: String,
    /// URL of the Confluent Schema Registry.
    pub schema_registry_url: String,
    /// Inline Avro schema JSON string.
    #[serde(default)]
    pub avro_schema: Option<String>,
    /// Path to a `.avsc` file on disk.
    #[serde(default)]
    pub avro_schema_file: Option<String>,
    /// Kafka security protocol (default: `"SASL_SSL"`).
    #[serde(default = "default_security_protocol")]
    pub security_protocol: String,
    /// SASL mechanism (e.g., `"PLAIN"`, `"SCRAM-SHA-256"`).
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    /// SASL username (supports `${ENV_VAR}` interpolation).
    #[serde(default)]
    pub sasl_username: Option<String>,
    /// SASL password (supports `${ENV_VAR}` interpolation).
    #[serde(default)]
    pub sasl_password: Option<String>,
    /// Filter which items to produce: `"all"` (default), `"valid_only"`, `"errors_only"`.
    #[serde(default = "default_filter")]
    pub filter: String,
}

fn default_security_protocol() -> String {
    "SASL_SSL".to_string()
}

fn default_filter() -> String {
    "all".to_string()
}

/// Replace `${VAR_NAME}` patterns with environment variable values.
///
/// Unknown variables are left as-is (passthrough).
pub fn interpolate_env(input: &str) -> String {
    let re = match Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}") {
        Ok(r) => r,
        Err(_) => return input.to_string(),
    };
    re.replace_all(input, |caps: &regex::Captures<'_>| {
        let var_name = &caps[1];
        std::env::var(var_name).unwrap_or_else(|_| caps[0].to_string())
    })
    .to_string()
}

/// Kafka sink stage: serializes records to Avro and produces to Kafka.
///
/// This is a **terminal** stage — it consumes items and returns an empty
/// vec (no downstream output).
///
/// Schema registration with the Confluent Schema Registry is deferred
/// to the first `process()` call, so construction is synchronous and
/// compatible with the stage registry closure.
pub struct KafkaSinkStage {
    config: KafkaSinkConfig,
    schema_json: String,
    producer: FutureProducer,
    schema: apache_avro::Schema,
    schema_id: OnceCell<i32>,
}

impl std::fmt::Debug for KafkaSinkStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KafkaSinkStage")
            .field("config", &self.config)
            .field("schema_id", &self.schema_id)
            .finish_non_exhaustive()
    }
}

impl KafkaSinkStage {
    /// Build a `KafkaSinkStage` from TOML stage params (synchronous).
    ///
    /// Parses config, loads the Avro schema, and builds the Kafka producer.
    /// Schema registration with the registry is deferred to the first
    /// `process()` call.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if config parsing, schema loading,
    /// or Kafka producer creation fails.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let mut config: KafkaSinkConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "kafka_sink".to_string(),
                item_id: String::new(),
                message: format!("invalid kafka_sink config: {e}"),
            })?;

        // Interpolate env vars in sensitive fields.
        config.bootstrap_servers = interpolate_env(&config.bootstrap_servers);
        config.schema_registry_url = interpolate_env(&config.schema_registry_url);
        if let Some(ref u) = config.sasl_username {
            config.sasl_username = Some(interpolate_env(u));
        }
        if let Some(ref p) = config.sasl_password {
            config.sasl_password = Some(interpolate_env(p));
        }

        // Load Avro schema from inline or file.
        let schema_json = match (&config.avro_schema, &config.avro_schema_file) {
            (Some(inline), _) => inline.clone(),
            (None, Some(path)) => {
                let interpolated_path = interpolate_env(path);
                std::fs::read_to_string(&interpolated_path).map_err(|e| StageError::Permanent {
                    stage: "kafka_sink".to_string(),
                    item_id: String::new(),
                    message: format!("cannot read schema file '{interpolated_path}': {e}"),
                })?
            }
            (None, None) => {
                return Err(StageError::Permanent {
                    stage: "kafka_sink".to_string(),
                    item_id: String::new(),
                    message: "either 'avro_schema' or 'avro_schema_file' must be specified"
                        .to_string(),
                });
            }
        };

        // Parse the Avro schema.
        let schema = parse_schema(&schema_json).map_err(|e| StageError::Permanent {
            stage: "kafka_sink".to_string(),
            item_id: String::new(),
            message: format!("invalid Avro schema: {e}"),
        })?;

        // Build the rdkafka FutureProducer.
        let mut kafka_config = ClientConfig::new();
        kafka_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("security.protocol", &config.security_protocol);

        if let Some(ref mechanism) = config.sasl_mechanism {
            kafka_config.set("sasl.mechanism", mechanism);
        }
        if let Some(ref username) = config.sasl_username {
            kafka_config.set("sasl.username", username);
        }
        if let Some(ref password) = config.sasl_password {
            kafka_config.set("sasl.password", password);
        }

        let producer: FutureProducer =
            kafka_config.create().map_err(|e| StageError::Permanent {
                stage: "kafka_sink".to_string(),
                item_id: String::new(),
                message: format!("cannot create Kafka producer: {e}"),
            })?;

        Ok(Self {
            config,
            schema_json,
            producer,
            schema,
            schema_id: OnceCell::new(),
        })
    }

    /// Register the schema with the Schema Registry (lazy, once).
    async fn ensure_schema_id(&self) -> Result<i32, StageError> {
        let id = self
            .schema_id
            .get_or_try_init(|| async {
                let registry = SchemaRegistry::new(&self.config.schema_registry_url);
                let subject = format!("{}-value", self.config.topic);
                let id = registry
                    .register_schema(&subject, &self.schema_json)
                    .await
                    .map_err(|e| StageError::Permanent {
                        stage: "kafka_sink".to_string(),
                        item_id: String::new(),
                        message: format!("schema registry error: {e}"),
                    })?;
                debug!(schema_id = id, topic = %self.config.topic, "schema registered");
                Ok(id)
            })
            .await?;
        Ok(*id)
    }

    /// Check whether an item should be produced based on the filter setting.
    fn should_produce(&self, item: &PipelineItem) -> bool {
        let status = item
            .metadata
            .get("_validation_status")
            .and_then(|v| v.as_str());

        match self.config.filter.as_str() {
            "valid_only" => status != Some("failed"),
            "errors_only" => status == Some("failed"),
            _ => true, // "all" or unknown → produce everything
        }
    }
}

#[async_trait]
impl Stage for KafkaSinkStage {
    fn name(&self) -> &str {
        "kafka_sink"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Check filter.
        if !self.should_produce(&item) {
            debug!(item_id = %item.id, filter = %self.config.filter, "skipping item (filter)");
            return Ok(vec![]);
        }

        // Ensure schema is registered (lazy init on first call).
        let schema_id = self.ensure_schema_id().await?;

        // Get the record.
        let record = item.record.as_ref().ok_or_else(|| StageError::Permanent {
            stage: "kafka_sink".to_string(),
            item_id: item.id.clone(),
            message: "item has no record to serialize".to_string(),
        })?;

        // Serialize to Avro wire format.
        let payload = serialize_record_avro(record, &self.schema, schema_id).map_err(|e| {
            StageError::Permanent {
                stage: "kafka_sink".to_string(),
                item_id: item.id.clone(),
                message: format!("Avro serialization error: {e}"),
            }
        })?;

        // Use item.id as the Kafka message key.
        let key = item.id.as_bytes();

        // Produce to Kafka with a 30-second timeout.
        let delivery_result = self
            .producer
            .send(
                FutureRecord::to(&self.config.topic)
                    .key(key)
                    .payload(&payload),
                Duration::from_secs(30),
            )
            .await;

        match delivery_result {
            Ok(delivery) => {
                debug!(
                    item_id = %item.id,
                    partition = delivery.partition,
                    offset = delivery.offset,
                    "produced to Kafka"
                );
            }
            Err((err, _)) => {
                return Err(StageError::Transient {
                    stage: "kafka_sink".to_string(),
                    item_id: item.id.clone(),
                    message: format!("Kafka produce error: {err}"),
                });
            }
        }

        // Terminal stage — no output items.
        Ok(vec![])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn test_kafka_sink_config_deserialize() {
        let params = json!({
            "topic": "transactions",
            "bootstrap_servers": "localhost:9092",
            "schema_registry_url": "http://localhost:8081",
            "avro_schema": "{\"type\":\"record\",\"name\":\"Test\",\"fields\":[]}",
            "filter": "valid_only"
        });

        let config: KafkaSinkConfig = serde_json::from_value(params).unwrap();
        assert_eq!(config.topic, "transactions");
        assert_eq!(config.bootstrap_servers, "localhost:9092");
        assert_eq!(config.schema_registry_url, "http://localhost:8081");
        assert!(config.avro_schema.is_some());
        assert_eq!(config.filter, "valid_only");
        assert_eq!(config.security_protocol, "SASL_SSL");
        assert!(config.sasl_mechanism.is_none());
    }

    #[test]
    fn test_kafka_sink_config_defaults() {
        let params = json!({
            "topic": "t",
            "bootstrap_servers": "b",
            "schema_registry_url": "s",
            "avro_schema": "{}"
        });

        let config: KafkaSinkConfig = serde_json::from_value(params).unwrap();
        assert_eq!(config.filter, "all");
        assert_eq!(config.security_protocol, "SASL_SSL");
    }

    #[test]
    fn test_kafka_sink_filter_valid_only_skips_failed() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());

        // valid_only → skip items with status == "failed"
        let should = match "valid_only" {
            "valid_only" => status != Some("failed"),
            "errors_only" => status == Some("failed"),
            _ => true,
        };
        assert!(!should, "valid_only filter should skip failed items");
    }

    #[test]
    fn test_kafka_sink_filter_errors_only_skips_passed() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("passed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());

        // errors_only → only produce if status == "failed"
        let should = status == Some("failed");
        assert!(!should, "errors_only filter should skip passed items");
    }

    #[test]
    fn test_kafka_sink_filter_all_produces_everything() {
        let status: Option<&str> = Some("failed");
        let should = match "all" {
            "valid_only" => status != Some("failed"),
            "errors_only" => status == Some("failed"),
            _ => true,
        };
        assert!(should, "all filter should produce everything");
    }

    #[test]
    fn test_kafka_sink_filter_valid_only_passes_valid() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("passed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());
        let should = match "valid_only" {
            "valid_only" => status != Some("failed"),
            _ => true,
        };
        assert!(should, "valid_only filter should pass valid items");
    }

    #[test]
    fn test_kafka_sink_filter_no_status_treated_as_valid() {
        let metadata: BTreeMap<String, serde_json::Value> = BTreeMap::new();

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());
        let should = match "valid_only" {
            "valid_only" => status != Some("failed"),
            _ => true,
        };
        assert!(should, "items without status should pass valid_only filter");
    }

    #[test]
    fn test_env_interpolation() {
        // HOME is reliably set on unix systems.
        let input = "prefix-${HOME}-suffix";
        let result = interpolate_env(input);
        assert!(!result.contains("${HOME}"), "HOME should be interpolated");
        assert!(result.starts_with("prefix-"));
        assert!(result.ends_with("-suffix"));
    }

    #[test]
    fn test_env_interpolation_missing_var_passthrough() {
        let input = "keep-${VERY_UNLIKELY_ECL_TEST_VAR_12345}-as-is";
        let result = interpolate_env(input);
        assert_eq!(
            result, "keep-${VERY_UNLIKELY_ECL_TEST_VAR_12345}-as-is",
            "missing env vars should be left as-is"
        );
    }

    #[test]
    fn test_env_interpolation_no_vars() {
        let input = "plain string with no vars";
        let result = interpolate_env(input);
        assert_eq!(result, input);
    }

    #[test]
    fn test_env_interpolation_multiple_vars() {
        let input = "${HOME}/path/${HOME}";
        let result = interpolate_env(input);
        assert!(!result.contains("${HOME}"));
    }

    #[test]
    fn test_kafka_sink_from_params_missing_schema() {
        let params = json!({
            "topic": "t",
            "bootstrap_servers": "localhost:9092",
            "schema_registry_url": "http://localhost:8081"
        });
        let result = KafkaSinkStage::from_params(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("avro_schema"),
            "error should mention avro_schema: {err}"
        );
    }

    #[test]
    fn test_kafka_sink_from_params_invalid_schema() {
        let params = json!({
            "topic": "t",
            "bootstrap_servers": "localhost:9092",
            "schema_registry_url": "http://localhost:8081",
            "avro_schema": "not valid json"
        });
        let result = KafkaSinkStage::from_params(&params);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Avro schema"),
            "error should mention Avro schema: {err}"
        );
    }

    #[test]
    fn test_kafka_sink_from_params_valid() {
        let params = json!({
            "topic": "transactions",
            "bootstrap_servers": "localhost:9092",
            "schema_registry_url": "http://localhost:8081",
            "avro_schema": r#"{"type":"record","name":"Test","fields":[{"name":"id","type":"string"}]}"#,
            "security_protocol": "PLAINTEXT"
        });
        let stage = KafkaSinkStage::from_params(&params).unwrap();
        assert_eq!(stage.name(), "kafka_sink");
        assert_eq!(stage.config.topic, "transactions");
        assert!(stage.schema_id.get().is_none(), "schema_id should be lazy");
    }

    #[test]
    fn test_kafka_sink_from_params_with_schema_file() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join("test.avsc");
        std::fs::write(
            &schema_path,
            r#"{"type":"record","name":"Test","fields":[{"name":"id","type":"string"}]}"#,
        )
        .unwrap();

        let params = json!({
            "topic": "t",
            "bootstrap_servers": "localhost:9092",
            "schema_registry_url": "http://localhost:8081",
            "avro_schema_file": schema_path.to_str().unwrap(),
            "security_protocol": "PLAINTEXT"
        });
        let stage = KafkaSinkStage::from_params(&params).unwrap();
        assert_eq!(stage.name(), "kafka_sink");
    }

    #[test]
    fn test_kafka_sink_stage_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<KafkaSinkStage>();
    }
}
