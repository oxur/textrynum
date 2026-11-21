# 5. Milestone 10.4: BigQuery Source Adapter

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 582-798. Original bytes are recoverable from import commit 2d006a7. -->

## 5. Milestone 10.4: BigQuery Source Adapter

### 5.1 Scope

New crate `ecl-adapter-bigquery` implementing `SourceAdapter` for BigQuery query results. Unlike file-based sources, this adapter runs a SQL query and produces one `PipelineItem` per result row.

### 5.2 Crate Structure

```
crates/ecl-adapter-bigquery/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 5.3 Cargo.toml

```toml
[package]
name = "ecl-adapter-bigquery"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

gcp-bigquery-client = "0.25"

tokio = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### 5.4 SourceSpec Extension

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    // ... existing ...
    #[serde(rename = "bigquery")]
    BigQuery(BigQuerySourceSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigQuerySourceSpec {
    /// GCP project ID.
    pub project: String,
    /// SQL query to execute. Supports ${PARAM} interpolation.
    pub query: String,
    /// Query parameters (substituted into query).
    #[serde(default)]
    pub parameters: BTreeMap<String, serde_json::Value>,
    /// Credentials for authentication.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
    /// Named data stream.
    #[serde(default)]
    pub stream: Option<String>,
}
```

### 5.5 Adapter Implementation

BigQuery is different from file sources — there's no separate enumerate/fetch cycle. The query returns all data at once. We model this as:

- **`enumerate()`**: Execute the query, return one `SourceItem` per row (lightweight: just row index + hash)
- **`fetch()`**: Return the row data that was already fetched during enumerate (cached)

Alternative: enumerate returns a single "query result" SourceItem, and the CSV parse stage (or a new `bq_parse` stage) fans out into rows.

**Recommended: Single-item model.** Enumerate returns one `SourceItem` representing the query result. Fetch returns all rows as JSON lines. Then `csv_parse` or a new `json_lines_parse` stage fans out into records. This reuses existing infrastructure.

```rust
#[derive(Debug)]
pub struct BigQueryAdapter {
    project: String,
    query: String,
    parameters: BTreeMap<String, serde_json::Value>,
    stream: Option<String>,
    /// Cached query result (populated during enumerate).
    cached_result: tokio::sync::Mutex<Option<Vec<u8>>>,
}

#[async_trait]
impl SourceAdapter for BigQueryAdapter {
    fn source_kind(&self) -> &str { "bigquery" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        // 1. Interpolate parameters into query.
        // 2. Execute query via BigQuery API.
        // 3. Serialize result rows as JSON lines.
        // 4. Cache in self.cached_result.
        // 5. Return single SourceItem representing the query result.

        let client = gcp_bigquery_client::Client::from_application_default_credentials().await
            .map_err(|e| SourceError::AuthError { /* ... */ })?;

        let query = self.interpolate_query();
        let result = client.job().query(&self.project, query).await
            .map_err(|e| SourceError::Transient { /* ... */ })?;

        // Convert rows to JSON lines.
        let mut content = Vec::new();
        for row in &result.rows {
            let json_row = row_to_json(row, &result.schema);
            serde_json::to_writer(&mut content, &json_row)
                .map_err(|e| SourceError::Permanent { /* ... */ })?;
            content.push(b'\n');
        }

        let hash = blake3::hash(&content);
        *self.cached_result.lock().await = Some(content);

        Ok(vec![SourceItem {
            id: format!("bq:{}:{}", self.project, hash.to_hex()[..8].to_string()),
            display_name: format!("BigQuery query result ({} rows)", result.rows.len()),
            mime_type: "application/x-ndjson".to_string(),
            path: "query_result.jsonl".to_string(),
            modified_at: Some(Utc::now()),
            source_hash: Some(hash.to_hex().to_string()),
        }])
    }

    async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let content = self.cached_result.lock().await.take()
            .ok_or_else(|| SourceError::Permanent {
                source_name: "bigquery".to_string(),
                message: "no cached query result (call enumerate first)".to_string(),
            })?;

        let content_hash = Blake3Hash::new(&blake3::hash(&content).to_hex().to_string());

        Ok(ExtractedDocument {
            id: _item.id.clone(),
            display_name: _item.display_name.clone(),
            content,
            mime_type: "application/x-ndjson".to_string(),
            provenance: ItemProvenance {
                source_kind: "bigquery".to_string(),
                metadata: BTreeMap::from([
                    ("project".to_string(), json!(self.project)),
                ]),
                source_modified: Some(Utc::now()),
                extracted_at: Utc::now(),
            },
            content_hash,
        })
    }
}
```

### 5.6 JSON Lines Parse Stage

Since BigQuery results come as JSON lines (not CSV), we need a complementary parse stage:

#### File: `crates/ecl-stages/src/json_lines_parse.rs` (new)

```rust
#[derive(Debug)]
pub struct JsonLinesParseStage;

#[async_trait]
impl Stage for JsonLinesParseStage {
    fn name(&self) -> &str { "json_lines_parse" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let content = std::str::from_utf8(item.content.as_ref())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let mut results = Vec::new();
        for (i, line) in content.lines().enumerate() {
            if line.trim().is_empty() { continue; }
            let record: Record = serde_json::from_str(line)
                .map_err(|e| StageError::Permanent {
                    stage: "json_lines_parse".to_string(),
                    item_id: item.id.clone(),
                    message: format!("invalid JSON on line {}: {e}", i + 1),
                })?;
            results.push(PipelineItem {
                id: format!("{}:row:{}", item.id, i),
                display_name: format!("{}:{}", item.display_name, i),
                content: Arc::from(line.as_bytes()),
                record: Some(record),
                ..item.clone()
            });
        }
        Ok(results)
    }
}
```

### 5.7 Tests

1. `test_bq_spec_serde_roundtrip`
2. `test_bq_query_interpolation` — parameter substitution
3. `test_json_lines_parse_basic` — 3 JSON lines → 3 items
4. `test_json_lines_parse_empty_lines_skipped`
5. `test_json_lines_parse_invalid_json_error`
6. `test_json_lines_parse_fan_out_ids`

---

