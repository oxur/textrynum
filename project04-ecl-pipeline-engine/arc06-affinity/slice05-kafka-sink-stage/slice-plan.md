# 6. Milestone 7.5: Kafka Sink Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 908-1109. Original bytes are recoverable from import commit 3a39aea. -->

## 6. Milestone 7.5: Kafka Sink Stage

### 6.1 Scope

New crate `ecl-sink-kafka` with Kafka producer, Avro serialization, and Schema Registry integration.

### 6.2 Crate Structure

```
crates/ecl-sink-kafka/
├── Cargo.toml
└── src/
    ├── lib.rs          # KafkaSinkStage implementation
    ├── producer.rs     # Kafka producer wrapper
    ├── avro.rs         # Avro serialization
    └── registry.rs     # Schema Registry HTTP client
```

### 6.3 Cargo.toml

```toml
[package]
name = "ecl-sink-kafka"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

# Kafka
rdkafka = { version = "0.37", features = ["cmake-build", "ssl", "sasl"] }

# Avro
apache-avro = "0.17"

# Schema Registry
reqwest = { workspace = true, features = ["json"] }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serde
serde = { workspace = true }
serde_json = { workspace = true }

# Logging
tracing = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
wiremock = "0.6"
```

**Build note:** `rdkafka` with `cmake-build` compiles librdkafka from source. This adds ~30s to first build but avoids system library dependency. If this is problematic, consider `rskafka` (pure Rust) as P1 fallback.

### 6.4 Schema Registry Client

#### File: `crates/ecl-sink-kafka/src/registry.rs`

```rust
/// Confluent Schema Registry HTTP client.
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    base_url: String,
    client: reqwest::Client,
}

impl SchemaRegistry {
    pub fn new(base_url: &str) -> Self { /* ... */ }

    /// Register an Avro schema, returns schema ID.
    pub async fn register_schema(
        &self,
        subject: &str,
        schema_json: &str,
    ) -> Result<i32, RegistryError> {
        // POST /subjects/{subject}/versions
        // Body: { "schema": "<escaped_schema_json>", "schemaType": "AVRO" }
        // Response: { "id": 123 }
    }

    /// Get schema by ID.
    pub async fn get_schema(&self, id: i32) -> Result<String, RegistryError> {
        // GET /schemas/ids/{id}
    }
}
```

### 6.5 Avro Serialization

#### File: `crates/ecl-sink-kafka/src/avro.rs`

```rust
/// Serialize a Record to Confluent wire format:
/// [0x00] [4-byte schema ID big-endian] [avro binary payload]
pub fn serialize_record_avro(
    record: &serde_json::Map<String, serde_json::Value>,
    schema: &apache_avro::Schema,
    schema_id: i32,
) -> Result<Vec<u8>, AvroError> {
    // 1. Convert serde_json::Value → apache_avro::types::Value
    // 2. Encode with apache_avro::to_avro_datum
    // 3. Prepend magic byte (0x00) + schema_id (4 bytes BE)
    // 4. Return bytes
}
```

### 6.6 KafkaSinkStage

#### File: `crates/ecl-sink-kafka/src/lib.rs`

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct KafkaSinkConfig {
    pub topic: String,
    pub bootstrap_servers: String,
    pub schema_registry_url: String,
    pub avro_schema: Option<String>,       // inline schema JSON
    pub avro_schema_file: Option<String>,   // path to .avsc file
    #[serde(default = "default_sasl_ssl")]
    pub security_protocol: String,
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    #[serde(default)]
    pub sasl_username: Option<String>,
    #[serde(default)]
    pub sasl_password: Option<String>,
    /// "all" (default), "valid_only", "errors_only"
    #[serde(default = "default_filter_all")]
    pub filter: String,
}

#[derive(Debug)]
pub struct KafkaSinkStage {
    config: KafkaSinkConfig,
    producer: rdkafka::producer::FutureProducer,
    schema: apache_avro::Schema,
    schema_id: i32,
}

impl KafkaSinkStage {
    pub async fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        // 1. Deserialize config (with env var interpolation for ${...})
        // 2. Load/parse Avro schema
        // 3. Register schema with Schema Registry → get schema_id
        // 4. Build rdkafka FutureProducer with config
        // 5. Return stage
    }
}

#[async_trait]
impl Stage for KafkaSinkStage {
    fn name(&self) -> &str { "kafka_sink" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Check filter (valid_only → skip items with _validation_status == "failed")
        // 2. Get record from item
        // 3. Serialize to Avro wire format
        // 4. Produce to Kafka topic
        // 5. Await delivery confirmation
        // 6. Return empty vec (terminal stage)

        Ok(vec![])  // Terminal — no output items
    }
}
```

**Env var interpolation helper:**

```rust
fn interpolate_env(s: &str) -> String {
    // Replace ${VAR_NAME} with std::env::var("VAR_NAME")
    // Leave literal if var not set (log warning)
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\$\{([^}]+)\}").unwrap());
    RE.replace_all(s, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_else(|_| caps[0].to_string())
    }).to_string()
}
```

**Tests:**

1. `test_kafka_sink_config_deserialize`
2. `test_kafka_sink_filter_valid_only_skips_failed`
3. `test_kafka_sink_filter_errors_only_skips_passed`
4. `test_avro_serialize_simple_record`
5. `test_avro_serialize_wire_format_header`
6. `test_schema_registry_register` (wiremock)
7. `test_schema_registry_get` (wiremock)
8. `test_env_interpolation`
9. `test_env_interpolation_missing_var_passthrough`

---

