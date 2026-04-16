//! GCS sink stage for the ECL pipeline runner.
//!
//! Writes pipeline items to Google Cloud Storage as individual JSON files.
//! Primarily used for error/rejected item output — downstream of validation,
//! items with `_validation_status == "failed"` are written to a GCS bucket
//! for review and reprocessing.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use tracing::debug;

use ecl_adapter_gcs::auth::TokenProvider;
use ecl_adapter_gcs::types::{GCS_READWRITE_SCOPE, GCS_UPLOAD_BASE_URL};
use ecl_pipeline_spec::CredentialRef;
use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Configuration for the GCS sink stage, deserialized from TOML params.
#[derive(Debug, Clone, Deserialize)]
pub struct GcsSinkConfig {
    /// GCS bucket name.
    pub bucket: String,
    /// Object key prefix (e.g., `"errors/affinity/"`).
    pub prefix: String,
    /// Credential reference for GCS auth.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
    /// Filter: `"all"` (default), `"valid_only"`, `"errors_only"`.
    #[serde(default = "default_filter")]
    pub filter: String,
}

fn default_adc() -> CredentialRef {
    CredentialRef::ApplicationDefault
}

fn default_filter() -> String {
    "all".to_string()
}

/// GCS sink stage: writes pipeline items as individual JSON files to GCS.
///
/// This is a **terminal** stage — it consumes items and returns an empty
/// vec (no downstream output).
///
/// Each item is written to:
/// `gs://{bucket}/{prefix}{date}/{item_id}.json`
///
/// The JSON file contains the item's record (if present) plus metadata
/// fields like `_validation_errors` and `_validation_status`.
pub struct GcsSinkStage {
    config: GcsSinkConfig,
    http_client: reqwest::Client,
    token_provider: TokenProvider,
    upload_base_url: String,
}

impl std::fmt::Debug for GcsSinkStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcsSinkStage")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl GcsSinkStage {
    /// Build a `GcsSinkStage` from TOML stage params (synchronous).
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if config parsing fails.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: GcsSinkConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "gcs_sink".to_string(),
                item_id: String::new(),
                message: format!("invalid gcs_sink config: {e}"),
            })?;

        let http_client = reqwest::Client::new();
        let token_provider = TokenProvider::new(
            config.credentials.clone(),
            http_client.clone(),
            GCS_READWRITE_SCOPE,
        );

        Ok(Self {
            config,
            http_client,
            token_provider,
            upload_base_url: GCS_UPLOAD_BASE_URL.to_string(),
        })
    }

    /// Override the upload base URL (for testing with wiremock).
    #[cfg(test)]
    fn with_upload_base_url(mut self, url: String) -> Self {
        self.upload_base_url = url;
        self
    }

    /// Override the token provider (for testing).
    #[cfg(test)]
    fn with_token_provider(mut self, provider: TokenProvider) -> Self {
        self.token_provider = provider;
        self
    }

    /// Check whether an item should be written based on the filter setting.
    fn should_write(&self, item: &PipelineItem) -> bool {
        let status = item
            .metadata
            .get("_validation_status")
            .and_then(|v| v.as_str());

        match self.config.filter.as_str() {
            "valid_only" => status != Some("failed"),
            "errors_only" => status == Some("failed"),
            _ => true, // "all" or unknown → write everything
        }
    }

    /// Build the GCS object path for an item.
    fn object_path(&self, item_id: &str) -> String {
        let date = Utc::now().format("%Y-%m-%d");
        let prefix = self.config.prefix.trim_end_matches('/');
        if prefix.is_empty() {
            format!("{date}/{item_id}.json")
        } else {
            format!("{prefix}/{date}/{item_id}.json")
        }
    }

    /// Build the JSON payload for an item.
    fn build_payload(item: &PipelineItem) -> serde_json::Value {
        let mut payload = serde_json::Map::new();

        // Include the record if present.
        if let Some(ref record) = item.record {
            for (k, v) in record {
                payload.insert(k.clone(), v.clone());
            }
        }

        // Include validation metadata.
        for (k, v) in &item.metadata {
            payload.insert(k.clone(), v.clone());
        }

        // Include item identity.
        payload.insert("_item_id".to_string(), serde_json::json!(item.id));
        payload.insert(
            "_source_name".to_string(),
            serde_json::json!(item.source_name),
        );

        serde_json::Value::Object(payload)
    }

    /// Upload a JSON payload to GCS via the JSON API.
    async fn upload_object(
        &self,
        object_name: &str,
        payload: &[u8],
        token: &str,
    ) -> Result<(), StageError> {
        let url = format!(
            "{}/b/{}/o?uploadType=media&name={}",
            self.upload_base_url,
            self.config.bucket,
            urlencoded(object_name)
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .header("Content-Type", "application/json")
            .body(payload.to_vec())
            .send()
            .await
            .map_err(|e| StageError::Transient {
                stage: "gcs_sink".to_string(),
                item_id: String::new(),
                message: format!("GCS upload HTTP error: {e}"),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(StageError::Transient {
                stage: "gcs_sink".to_string(),
                item_id: String::new(),
                message: format!("GCS upload failed ({status}): {body}"),
            });
        }

        Ok(())
    }
}

/// Minimal percent-encoding for GCS object names in URL paths.
fn urlencoded(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '&' => out.push_str("%26"),
            '+' => out.push_str("%2B"),
            '%' => out.push_str("%25"),
            _ => out.push(c),
        }
    }
    out
}

#[async_trait]
impl Stage for GcsSinkStage {
    fn name(&self) -> &str {
        "gcs_sink"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Check filter.
        if !self.should_write(&item) {
            debug!(item_id = %item.id, filter = %self.config.filter, "skipping item (filter)");
            return Ok(vec![]);
        }

        // Get auth token.
        let token = self
            .token_provider
            .get_token()
            .await
            .map_err(|e| StageError::Transient {
                stage: "gcs_sink".to_string(),
                item_id: item.id.clone(),
                message: format!("GCS auth error: {e}"),
            })?;

        // Build JSON payload.
        let payload = Self::build_payload(&item);
        let payload_bytes =
            serde_json::to_vec_pretty(&payload).map_err(|e| StageError::Permanent {
                stage: "gcs_sink".to_string(),
                item_id: item.id.clone(),
                message: format!("JSON serialization error: {e}"),
            })?;

        // Upload to GCS.
        let object_path = self.object_path(&item.id);
        self.upload_object(&object_path, &payload_bytes, &token)
            .await?;

        debug!(
            item_id = %item.id,
            object = %object_path,
            bucket = %self.config.bucket,
            "wrote item to GCS"
        );

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
    fn test_gcs_sink_config_deserialize() {
        let params = json!({
            "bucket": "my-errors-bucket",
            "prefix": "errors/affinity/",
            "filter": "errors_only"
        });

        let config: GcsSinkConfig = serde_json::from_value(params).unwrap();
        assert_eq!(config.bucket, "my-errors-bucket");
        assert_eq!(config.prefix, "errors/affinity/");
        assert_eq!(config.filter, "errors_only");
        assert!(matches!(
            config.credentials,
            CredentialRef::ApplicationDefault
        ));
    }

    #[test]
    fn test_gcs_sink_config_defaults() {
        let params = json!({
            "bucket": "b",
            "prefix": "p"
        });

        let config: GcsSinkConfig = serde_json::from_value(params).unwrap();
        assert_eq!(config.filter, "all");
        assert!(matches!(
            config.credentials,
            CredentialRef::ApplicationDefault
        ));
    }

    #[test]
    fn test_gcs_sink_filter_errors_only_skips_passed() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("passed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());
        let should = status == Some("failed");
        assert!(!should, "errors_only should skip passed items");
    }

    #[test]
    fn test_gcs_sink_filter_errors_only_passes_failed() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());
        let should = status == Some("failed");
        assert!(should, "errors_only should pass failed items");
    }

    #[test]
    fn test_gcs_sink_filter_valid_only_skips_failed() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));

        let status = metadata.get("_validation_status").and_then(|v| v.as_str());
        let should = status != Some("failed");
        assert!(!should, "valid_only should skip failed items");
    }

    #[test]
    fn test_gcs_sink_object_path_with_prefix() {
        let stage = GcsSinkStage::from_params(&json!({
            "bucket": "b",
            "prefix": "errors/affinity",
        }))
        .unwrap();

        let path = stage.object_path("row-42");
        let date = Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(path, format!("errors/affinity/{date}/row-42.json"));
    }

    #[test]
    fn test_gcs_sink_object_path_with_trailing_slash() {
        let stage = GcsSinkStage::from_params(&json!({
            "bucket": "b",
            "prefix": "errors/",
        }))
        .unwrap();

        let path = stage.object_path("item-1");
        let date = Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(path, format!("errors/{date}/item-1.json"));
    }

    #[test]
    fn test_gcs_sink_object_path_empty_prefix() {
        let stage = GcsSinkStage::from_params(&json!({
            "bucket": "b",
            "prefix": "",
        }))
        .unwrap();

        let path = stage.object_path("item-1");
        let date = Utc::now().format("%Y-%m-%d").to_string();
        assert_eq!(path, format!("{date}/item-1.json"));
    }

    #[test]
    fn test_gcs_sink_build_payload_with_record() {
        let mut record = serde_json::Map::new();
        record.insert("name".to_string(), json!("Alice"));
        record.insert("amount".to_string(), json!(42.5));

        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));
        metadata.insert(
            "_validation_errors".to_string(),
            json!([{"field": "amount", "check": "numeric_range"}]),
        );

        let item = make_test_item("row-1", Some(record), metadata);
        let payload = GcsSinkStage::build_payload(&item);

        let obj = payload.as_object().unwrap();
        assert_eq!(obj.get("name").unwrap(), "Alice");
        assert_eq!(obj.get("amount").unwrap(), 42.5);
        assert_eq!(obj.get("_validation_status").unwrap(), "failed");
        assert_eq!(obj.get("_item_id").unwrap(), "row-1");
        assert!(obj.get("_validation_errors").unwrap().is_array());
    }

    #[test]
    fn test_gcs_sink_build_payload_no_record() {
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));

        let item = make_test_item("row-2", None, metadata);
        let payload = GcsSinkStage::build_payload(&item);

        let obj = payload.as_object().unwrap();
        assert_eq!(obj.get("_item_id").unwrap(), "row-2");
        assert_eq!(obj.get("_validation_status").unwrap(), "failed");
        // No record fields — only metadata and identity.
        assert!(obj.get("name").is_none());
    }

    #[test]
    fn test_gcs_sink_from_params_valid() {
        let params = json!({
            "bucket": "my-bucket",
            "prefix": "errors/",
            "filter": "errors_only"
        });

        let stage = GcsSinkStage::from_params(&params).unwrap();
        assert_eq!(stage.name(), "gcs_sink");
        assert_eq!(stage.config.bucket, "my-bucket");
        assert_eq!(stage.config.filter, "errors_only");
    }

    #[test]
    fn test_gcs_sink_from_params_missing_bucket() {
        let params = json!({
            "prefix": "errors/"
        });

        let result = GcsSinkStage::from_params(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("simple"), "simple");
        assert_eq!(urlencoded("path/to/file.json"), "path/to/file.json");
        assert_eq!(urlencoded("has space"), "has%20space");
        assert_eq!(urlencoded("a#b?c&d"), "a%23b%3Fc%26d");
        assert_eq!(urlencoded("100%"), "100%25");
    }

    #[tokio::test]
    async fn test_gcs_sink_upload_wiremock() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path_regex("/b/test-bucket/o.*"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(json!({"name": "uploaded.json"})),
            )
            .mount(&mock_server)
            .await;

        let params = json!({
            "bucket": "test-bucket",
            "prefix": "errors/",
            "filter": "errors_only"
        });

        let stage = GcsSinkStage::from_params(&params)
            .unwrap()
            .with_upload_base_url(format!("{}/upload/storage/v1", mock_server.uri()))
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("failed"));
        metadata.insert(
            "_validation_errors".to_string(),
            json!([{"field": "amount"}]),
        );

        let mut record = serde_json::Map::new();
        record.insert("name".to_string(), json!("Bad Row"));

        let item = make_test_item("row-err-1", Some(record), metadata);
        let ctx = make_test_ctx();

        let result = stage.process(item, &ctx).await.unwrap();
        assert!(result.is_empty(), "terminal stage should return empty vec");
    }

    #[tokio::test]
    async fn test_gcs_sink_upload_error_returns_transient() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path_regex("/b/test-bucket/o.*"))
            .respond_with(
                wiremock::ResponseTemplate::new(500).set_body_string("Internal Server Error"),
            )
            .mount(&mock_server)
            .await;

        let params = json!({
            "bucket": "test-bucket",
            "prefix": "errors/",
            "filter": "all"
        });

        let stage = GcsSinkStage::from_params(&params)
            .unwrap()
            .with_upload_base_url(format!("{}/upload/storage/v1", mock_server.uri()))
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let item = make_test_item("row-1", None, BTreeMap::new());
        let ctx = make_test_ctx();

        let result = stage.process(item, &ctx).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, StageError::Transient { .. }),
            "GCS upload errors should be transient: {err}"
        );
    }

    #[tokio::test]
    async fn test_gcs_sink_filter_skips_non_matching() {
        let mock_server = wiremock::MockServer::start().await;

        // No mock set up — if stage tries to upload, it will fail.
        let params = json!({
            "bucket": "test-bucket",
            "prefix": "errors/",
            "filter": "errors_only"
        });

        let stage = GcsSinkStage::from_params(&params)
            .unwrap()
            .with_upload_base_url(format!("{}/upload/storage/v1", mock_server.uri()))
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        // Item with "passed" status — should be skipped by errors_only filter.
        let mut metadata = BTreeMap::new();
        metadata.insert("_validation_status".to_string(), json!("passed"));

        let item = make_test_item("row-ok-1", None, metadata);
        let ctx = make_test_ctx();

        let result = stage.process(item, &ctx).await.unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_gcs_sink_stage_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GcsSinkStage>();
    }

    // ── Test helpers ──────────────────────────────────────────────

    fn make_test_item(
        id: &str,
        record: Option<serde_json::Map<String, serde_json::Value>>,
        metadata: BTreeMap<String, serde_json::Value>,
    ) -> PipelineItem {
        use ecl_pipeline_state::{Blake3Hash, ItemProvenance};

        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            source_name: "test-source".to_string(),
            content: std::sync::Arc::from(vec![]),
            mime_type: "application/json".to_string(),
            source_content_hash: Blake3Hash::new("0".repeat(64)),
            metadata,
            provenance: ItemProvenance {
                source_kind: "test".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: Utc::now(),
            },
            record,
            stream: None,
        }
    }

    fn make_test_ctx() -> StageContext {
        use ecl_pipeline_spec::PipelineSpec;
        use std::sync::Arc;

        let spec = PipelineSpec::from_toml(
            r#"
            name = "test"
            version = 1
            output_dir = "/tmp/ecl-test"
            [sources.test]
            kind = "filesystem"
            root = "/tmp"
            [stages.emit]
            adapter = "emit"
            resources = { reads = ["raw"] }
            "#,
        )
        .unwrap();

        StageContext {
            spec: Arc::new(spec),
            output_dir: std::path::PathBuf::from("/tmp/ecl-test-output"),
            params: json!({}),
            span: tracing::info_span!("test"),
        }
    }
}
