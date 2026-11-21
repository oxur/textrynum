# 6. Milestone 9.5: S3 Sink Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1027-1167. Original bytes are recoverable from import commit 2d006a7. -->

## 6. Milestone 9.5: S3 Sink Stage

### 6.1 Scope

New crate `ecl-sink-s3` for writing to AWS S3 (needed for Chime, also useful generally).

### 6.2 Crate Structure

```
crates/ecl-sink-s3/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 6.3 Cargo.toml

```toml
[package]
name = "ecl-sink-s3"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

aws-sdk-s3 = "1"
aws-config = { version = "1", features = ["behavior-version-latest"] }

tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### 6.4 Implementation

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct S3SinkConfig {
    pub bucket: String,
    pub prefix: String,
    /// Output format: "raw", "json_lines", "csv"
    #[serde(default = "default_raw")]
    pub format: String,
    /// AWS region. Default: "us-west-2"
    #[serde(default = "default_region")]
    pub region: String,
    /// Credential source: env vars (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// or a secret name for JSON credentials.
    #[serde(default)]
    pub credentials: Option<CredentialRef>,
    /// Filter: "all", "valid_only", "errors_only"
    #[serde(default = "default_all")]
    pub filter: String,
}

#[derive(Debug)]
pub struct S3SinkStage {
    config: S3SinkConfig,
    client: aws_sdk_s3::Client,
}

impl S3SinkStage {
    pub async fn from_params(
        params: &serde_json::Value,
        secret_resolver: &dyn SecretResolver,
    ) -> Result<Self, StageError> {
        let config: S3SinkConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // Build AWS config (from env, instance profile, or explicit creds).
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&aws_config);

        Ok(Self { config, client })
    }
}

#[async_trait]
impl Stage for S3SinkStage {
    fn name(&self) -> &str { "s3_sink" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Apply filter.
        if !passes_filter(&item, &self.config.filter) {
            return Ok(vec![]);
        }

        let key = format!("{}{}", self.config.prefix, item.display_name);
        let body = match self.config.format.as_str() {
            "json_lines" => {
                let record = item.record.as_ref()
                    .ok_or_else(|| StageError::Permanent { /* ... */ })?;
                serde_json::to_vec(record)
                    .map_err(|e| StageError::Permanent { /* ... */ })?
            }
            _ => item.content.to_vec(),
        };

        self.client.put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(body.into())
            .send()
            .await
            .map_err(|e| StageError::Transient {
                stage: "s3_sink".to_string(),
                item_id: item.id.clone(),
                message: e.to_string(),
            })?;

        Ok(vec![])  // Terminal stage
    }
}
```

### 6.5 Tests

1. `test_s3_sink_config_deserialize`
2. `test_s3_sink_filter_logic`
3. `test_s3_sink_key_construction`

---

