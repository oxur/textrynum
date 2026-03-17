# 0028 — Phase 1 Detailed Implementation Plan: "Make Affinity Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0026 (Requirements), 0027 (High-Level Plan)
**Goal:** Run the Affinity (Partner 290) pipeline end-to-end in ECL — GCS CSV files → parse → map → validate → Kafka (Avro) + GCS error output.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 7.1: Record Type & CSV Parse Stage](#2-milestone-71-record-type--csv-parse-stage)
3. [Milestone 7.2: Field Mapping Stage](#3-milestone-72-field-mapping-stage)
4. [Milestone 7.3: GCS Source Adapter](#4-milestone-73-gcs-source-adapter)
5. [Milestone 7.4: Validation Stage](#5-milestone-74-validation-stage)
6. [Milestone 7.5: Kafka Sink Stage](#6-milestone-75-kafka-sink-stage)
7. [Milestone 7.6: GCS Sink Stage](#7-milestone-76-gcs-sink-stage)
8. [Milestone 7.7: File Lifecycle Management](#8-milestone-77-file-lifecycle-management)
9. [Milestone 7.8: Affinity End-to-End Integration Test](#9-milestone-78-affinity-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Target Pipeline Shape

```
GCS Source (input/) ─→ CSV Parse ─→ Field Map ─→ Validate ─┬─→ Kafka Sink (valid)
                                                            └─→ GCS Sink (errors)
```

### 1.2 TOML Spec Preview (Affinity)

```toml
name = "affinity-290-transformation"
version = 1
output_dir = "./output/affinity-290"

[sources.transactions]
kind = "gcs"
bucket = "byn-prod-finx-290-affinity"
prefix = "staging/"
pattern = "Banyan-txn-file*.csv"
credentials = { type = "application_default" }

[stages.parse]
adapter = "csv_parse"
resources = { reads = ["gcs-files"], creates = ["parsed-records"] }
[stages.parse.params]
has_headers = true
delimiter = ","
columns = [
  { name = "Channel_Aggregator_ID", type = "string" },
  { name = "Program_ID", type = "string" },
  { name = "Account_ID", type = "string" },
  # ... (19 columns total)
]

[stages.map]
adapter = "field_map"
resources = { reads = ["parsed-records"], creates = ["mapped-records"] }
[stages.map.params]
rename = [
  { from = "Account_ID", to = "finx_consumer_token" },
  { from = "Transaction_Amount", to = "total_amount" },
  { from = "Merchant_Location_MID", to = "finx_mid" },
]
set = [
  { field = "country", value = "US" },
  { field = "currency", value = "USD" },
  { field = "byn_partner_id", value = 290 },
]
drop = ["Channel_Aggregator_ID", "Program_ID", "Card_BIN",
        "Account_Postal_Code", "Transaction_Settlement_Date",
        "Transaction_Code", "Merchant_Location_Category_Code"]

[stages.validate]
adapter = "validate"
resources = { reads = ["mapped-records"], creates = ["validated-records"] }
skip_on_error = true
[stages.validate.params]
rules = [
  { field = "finx_transaction_id", check = "required", severity = "hard" },
  { field = "card_last_four", check = "regex", pattern = "^\\d{4}$", severity = "hard" },
  { field = "purchase_ts", check = "date_range", min = "-10y", max = "+10y", severity = "hard" },
]

[stages.kafka_out]
adapter = "kafka_sink"
resources = { reads = ["validated-records"] }
[stages.kafka_out.params]
topic = "by_production_transaction_event_avro_0"
bootstrap_servers = "${KAFKA_BROKERS}"
schema_registry_url = "${SCHEMA_REGISTRY_URL}"
security_protocol = "SASL_SSL"
sasl_mechanism = "SCRAM-SHA-256"

[stages.error_out]
adapter = "gcs_sink"
resources = { reads = ["validated-records"] }
[stages.error_out.params]
bucket = "byn-prod-finx-290-affinity"
prefix = "error/"
format = "json_lines"
filter = "errors_only"
```

### 1.3 Key Design Decisions

**D1: Record as part of PipelineItem** — Add `record: Option<serde_json::Map<String, Value>>` to `PipelineItem`. Stages that operate on structured data read/write this field. Stages that operate on raw bytes use `content`. This preserves backward compatibility with existing stages (extract, normalize, filter, emit).

**D2: Fan-out model** — CSV parse is a fan-out stage: one PipelineItem (file) → N PipelineItems (rows). The Stage trait already supports this via `Result<Vec<PipelineItem>>`. Each output item gets a unique ID: `{file_id}:row:{line_number}`.

**D3: Validation produces metadata, not separate streams** — Failed validation adds `_validation_errors: [...]` to the record's metadata. Downstream stages (kafka_sink, gcs_sink) use a `filter` param to select valid-only or errors-only items. This avoids needing multi-stream support in Phase 1.

**D4: Sinks are regular stages** — Kafka and GCS sinks implement the `Stage` trait. They return `Ok(vec![])` (terminal, no output items). Registered in `stage_lookup_fn` like any other stage.

**D5: Environment variable interpolation** — Stage params support `${ENV_VAR}` syntax. Resolved at stage construction time in the registry, not at parse time. Keeps PipelineSpec serializable for checkpoints.

---

## 2. Milestone 7.1: Record Type & CSV Parse Stage

### 2.1 Scope

Add structured record support to `PipelineItem` and implement a configurable CSV parsing stage.

### 2.2 Changes to `ecl-pipeline-topo`

#### File: `crates/ecl-pipeline-topo/src/traits.rs`

**Add `record` field to `PipelineItem`:**

```rust
/// A structured data record flowing through the pipeline.
/// Stages that process tabular/structured data use this instead of raw `content`.
/// Backed by serde_json for maximum flexibility — fields can be strings, numbers,
/// booleans, arrays, or nested objects.
pub type Record = serde_json::Map<String, serde_json::Value>;

pub struct PipelineItem {
    // ... existing fields unchanged ...
    pub id: String,
    pub display_name: String,
    pub content: Arc<[u8]>,
    pub mime_type: String,
    pub source_name: String,
    pub source_content_hash: Blake3Hash,
    pub provenance: ItemProvenance,
    pub metadata: BTreeMap<String, serde_json::Value>,

    /// Structured record for tabular data. Set by CSV parse, consumed by
    /// field map, validate, and sink stages. None for document-oriented items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<Record>,
}
```

**Rationale:**
- `serde_json::Map<String, Value>` is already in our dependency tree
- Preserves insertion order (important for CSV column ordering)
- Serializable (embedded in checkpoints)
- AP-02: No `&String` — fields accessed via `&str` keys on Map
- ID-01: Type alias keeps API simple; no need for `#[non_exhaustive]` on a type alias

**Tests to add:**
- `test_pipeline_item_with_record_serde_roundtrip` — serialize/deserialize with populated record
- `test_pipeline_item_record_none_omitted_in_json` — `skip_serializing_if` works
- `test_pipeline_item_record_field_access` — get/set fields on Record

### 2.3 New Stage: `CsvParseStage`

#### File: `crates/ecl-stages/src/csv_parse.rs` (new)

**Configuration schema (from `ctx.params`):**

```rust
/// Parsed from stage params TOML/JSON.
#[derive(Debug, Clone, Deserialize)]
struct CsvParseConfig {
    /// Column definitions in order.
    columns: Vec<ColumnDef>,
    /// Field delimiter character. Default: ','
    #[serde(default = "default_delimiter")]
    delimiter: char,
    /// Quote character. Default: '"'
    #[serde(default = "default_quote")]
    quote_char: char,
    /// Whether the CSV has a header row. Default: true
    #[serde(default = "default_has_headers")]
    has_headers: bool,
    /// How to handle parse errors for individual rows.
    /// "skip" = skip bad rows (log warning), "fail" = fail the item.
    /// Default: "skip"
    #[serde(default = "default_on_error")]
    on_row_error: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ColumnDef {
    /// Column name (used as Record field key).
    name: String,
    /// Column type for conversion: "string", "integer", "float", "boolean".
    /// Default: "string"
    #[serde(default = "default_column_type")]
    r#type: String,
}
```

**Implementation:**

```rust
#[derive(Debug)]
pub struct CsvParseStage {
    config: CsvParseConfig,
}

impl CsvParseStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: CsvParseConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent {
                stage: "csv_parse".to_string(),
                item_id: String::new(),
                message: format!("invalid csv_parse config: {e}"),
            })?;
        Ok(Self { config })
    }
}

#[async_trait]
impl Stage for CsvParseStage {
    fn name(&self) -> &str { "csv_parse" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Build csv::ReaderBuilder from config
        // 2. Read item.content as &[u8]
        // 3. For each row:
        //    a. Map columns to Record fields with type conversion
        //    b. Create new PipelineItem with:
        //       - id: "{item.id}:row:{line_number}"
        //       - display_name: "{item.display_name}:{line_number}"
        //       - content: Arc::from(row_bytes)  (raw CSV row for debugging)
        //       - record: Some(parsed_record)
        //       - metadata: inherited from parent + { "_source_file": item.display_name, "_line_number": n }
        //       - All other fields cloned from parent item
        // 4. Return Vec of row items
        // 5. On row error: skip (with warning log) or fail per config
    }
}
```

**Type conversion logic:**
```rust
fn convert_value(raw: &str, col_type: &str) -> serde_json::Value {
    match col_type {
        "integer" => raw.parse::<i64>()
            .map(Value::from)
            .unwrap_or(Value::String(raw.to_string())),
        "float" => raw.parse::<f64>()
            .map(Value::from)
            .unwrap_or(Value::String(raw.to_string())),
        "boolean" => match raw.to_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => Value::Bool(true),
            "false" | "0" | "no" | "n" => Value::Bool(false),
            _ => Value::String(raw.to_string()),
        },
        _ => Value::String(raw.to_string()),  // "string" or unknown
    }
}
```

**Dependencies to add to `ecl-stages/Cargo.toml`:**
```toml
csv = "1"
```

**Tests (in `csv_parse.rs`):**
1. `test_csv_parse_basic_three_columns` — 3 rows, 3 string columns
2. `test_csv_parse_type_conversion` — integer, float, boolean columns
3. `test_csv_parse_type_conversion_fallback` — invalid integer stays string
4. `test_csv_parse_fan_out_ids` — verify `{file_id}:row:{n}` pattern
5. `test_csv_parse_with_headers` — header row skipped
6. `test_csv_parse_without_headers` — positional column assignment
7. `test_csv_parse_custom_delimiter` — tab-separated
8. `test_csv_parse_quoted_fields` — fields with commas inside quotes
9. `test_csv_parse_empty_file` — returns empty vec
10. `test_csv_parse_row_error_skip` — bad row skipped, others pass
11. `test_csv_parse_row_error_fail` — bad row fails entire item
12. `test_csv_parse_metadata_inheritance` — parent metadata preserved
13. `test_csv_parse_affinity_schema` — full 19-column Affinity schema
14. `test_csv_parse_from_params_invalid` — bad config → StageError::Permanent

**Rust patterns enforced:**
- AP-02: `content: &[u8]` not `&Vec<u8>` for CSV reader input
- AP-06: No `.unwrap()` — all errors mapped to `StageError`
- EH-01: `From` impls or `.map_err()` for csv::Error → StageError
- CA-05: No blocking — `csv::Reader` operates on in-memory `&[u8]`, not files
- TR-08: `CsvParseConfig` derives Debug, Clone, Deserialize
- ID-02: Consider `mem::take` if we need to move content out of item

### 2.4 Registry Integration

#### File: `crates/ecl-cli/src/pipeline/registry.rs`

Add to `stage_lookup_fn` match:
```rust
"csv_parse" => {
    let stage = ecl_stages::CsvParseStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
```

### 2.5 Module Wiring

#### File: `crates/ecl-stages/src/lib.rs`

Add:
```rust
pub mod csv_parse;
pub use csv_parse::CsvParseStage;
```

---

## 3. Milestone 7.2: Field Mapping Stage

### 3.1 Scope

Configurable stage that renames, drops, sets, and copies fields on Records.

### 3.2 Implementation

#### File: `crates/ecl-stages/src/field_map.rs` (new)

**Configuration schema:**

```rust
#[derive(Debug, Clone, Deserialize)]
struct FieldMapConfig {
    /// Rename fields: { from: "old_name", to: "new_name" }
    #[serde(default)]
    rename: Vec<RenameOp>,

    /// Drop fields by name.
    #[serde(default)]
    drop: Vec<String>,

    /// Set literal values: { field: "name", value: <json_value> }
    #[serde(default)]
    set: Vec<SetOp>,

    /// Copy a field: { from: "source", to: "dest" }
    #[serde(default)]
    copy: Vec<CopyOp>,

    /// Date parsing: { field: "Transaction_Date", format: "%m/%d/%Y", output: "purchase_ts", timezone: "UTC" }
    #[serde(default)]
    parse_dates: Vec<DateParseOp>,

    /// String padding: { field: "auth_code", width: 6, pad_char: "0", side: "left" }
    #[serde(default)]
    pad: Vec<PadOp>,

    /// Regex extract: { field: "Merchant_Location_Name", pattern: "WALGREENS\\s*#\\s*(\\d+)", output: "merchant_store_id", group: 1 }
    #[serde(default)]
    regex_extract: Vec<RegexExtractOp>,

    /// Nested object construction: { output: "payment", fields: { "total_amount": "total_amount", "card_last_four": "card_last_four" } }
    #[serde(default)]
    nest: Vec<NestOp>,
}

#[derive(Debug, Clone, Deserialize)]
struct RenameOp { from: String, to: String }

#[derive(Debug, Clone, Deserialize)]
struct SetOp { field: String, value: serde_json::Value }

#[derive(Debug, Clone, Deserialize)]
struct CopyOp { from: String, to: String }

#[derive(Debug, Clone, Deserialize)]
struct DateParseOp {
    field: String,
    format: String,        // strftime format
    output: String,        // output field name
    #[serde(default = "default_utc")]
    timezone: String,      // "UTC" or IANA timezone
}

#[derive(Debug, Clone, Deserialize)]
struct PadOp {
    field: String,
    width: usize,
    #[serde(default = "default_pad_char")]
    pad_char: String,
    #[serde(default = "default_pad_side")]
    side: String,          // "left" or "right"
}

#[derive(Debug, Clone, Deserialize)]
struct RegexExtractOp {
    field: String,
    pattern: String,
    output: String,
    #[serde(default = "default_group")]
    group: usize,
}

#[derive(Debug, Clone, Deserialize)]
struct NestOp {
    output: String,
    /// Map of output_field_name → source_field_name
    fields: BTreeMap<String, String>,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct FieldMapStage {
    config: FieldMapConfig,
    compiled_regexes: Vec<(usize, regex::Regex)>,  // index into regex_extract + compiled
}

impl FieldMapStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: FieldMapConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // Pre-compile regexes at construction time
        let compiled_regexes = config.regex_extract.iter().enumerate()
            .map(|(i, op)| {
                regex::Regex::new(&op.pattern)
                    .map(|r| (i, r))
                    .map_err(|e| StageError::Permanent { /* ... */ })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { config, compiled_regexes })
    }
}

#[async_trait]
impl Stage for FieldMapStage {
    fn name(&self) -> &str { "field_map" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut().ok_or_else(|| StageError::Permanent {
            stage: "field_map".to_string(),
            item_id: item.id.clone(),
            message: "field_map requires a record (did you run csv_parse first?)".to_string(),
        })?;

        // Apply operations in order:
        // 1. Renames (remove old key, insert new key)
        // 2. Copies (read source, insert dest)
        // 3. Sets (insert literal value)
        // 4. Date parsing (parse string → RFC3339)
        // 5. Padding (pad string fields)
        // 6. Regex extract (extract capture group)
        // 7. Nesting (group fields into sub-objects)
        // 8. Drops (remove fields last, after they may have been used)

        Ok(vec![item])
    }
}
```

**Operations detail:**

1. **Rename:** `record.remove("old")` → `record.insert("new", value)`
2. **Copy:** `record.get("src").cloned()` → `record.insert("dst", value)`
3. **Set:** `record.insert("field", literal_value)`
4. **Date parse:** Read string, parse with `chrono::NaiveDate::parse_from_str`, convert to RFC3339 string with timezone
5. **Pad:** Read string, apply `format!("{:0>width$}", val)` for left-pad
6. **Regex extract:** Match against field value string, extract capture group, insert result
7. **Nest:** Create sub-object from listed fields, insert as nested Value::Object
8. **Drop:** `record.remove("field")` for each

**Dependencies to add to `ecl-stages/Cargo.toml`:**
```toml
regex = "1"
chrono = { workspace = true }
```

**Tests:**
1. `test_field_map_rename_basic`
2. `test_field_map_rename_missing_field_no_error` — skip silently
3. `test_field_map_drop_fields`
4. `test_field_map_set_literal_string`
5. `test_field_map_set_literal_number`
6. `test_field_map_copy_field`
7. `test_field_map_date_parse_mm_dd_yyyy` — Affinity date format
8. `test_field_map_date_parse_invalid_returns_null`
9. `test_field_map_pad_left_six_digits` — auth code padding
10. `test_field_map_pad_already_at_width` — no-op
11. `test_field_map_regex_extract_store_id` — `WALGREENS #1234` → `1234`
12. `test_field_map_regex_no_match_sets_null`
13. `test_field_map_nest_creates_sub_object`
14. `test_field_map_no_record_returns_error`
15. `test_field_map_combined_affinity_pipeline` — full Affinity mapping
16. `test_field_map_from_params_invalid`

**Rust patterns:**
- PF-01: Compile regexes once in constructor, reuse per item
- AP-08: Accept `&serde_json::Value` not `&serde_json::Map` for params
- EH-02: Use `?` throughout, never unwrap

---

## 4. Milestone 7.3: GCS Source Adapter

### 4.1 Scope

New crate `ecl-adapter-gcs` implementing `SourceAdapter` for Google Cloud Storage.

### 4.2 Crate Structure

```
crates/ecl-adapter-gcs/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 4.3 Cargo.toml

```toml
[package]
name = "ecl-adapter-gcs"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

# GCS client
google-cloud-storage = { version = "0.22", features = ["default-tls"] }
google-cloud-auth = { version = "0.19", features = ["default-tls"] }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Hashing
blake3 = { workspace = true }

# Serde
serde = { workspace = true }
serde_json = { workspace = true }

# Matching
glob = { workspace = true }

# Logging
tracing = { workspace = true }

# Error handling
thiserror = { workspace = true }

# Time
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile = { workspace = true }
```

**Note:** If `google-cloud-storage` proves too heavy or has compile issues, fallback to raw `reqwest` + GCS JSON API (same pattern as `ecl-adapter-gdrive`). Evaluate during implementation.

### 4.4 SourceSpec Extension

#### File: `crates/ecl-pipeline-spec/src/source.rs`

Add new variant:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    // ... existing variants ...
    #[serde(rename = "gcs")]
    Gcs(GcsSourceSpec),
}

/// Google Cloud Storage source: list and fetch objects from a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsSourceSpec {
    /// GCS bucket name (without gs:// prefix).
    pub bucket: String,
    /// Object prefix to filter listing (e.g., "staging/").
    #[serde(default)]
    pub prefix: String,
    /// Glob pattern to match object names (e.g., "Banyan-txn-file*.csv").
    #[serde(default)]
    pub pattern: Option<String>,
    /// Credentials for authentication.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
}

fn default_adc() -> CredentialRef {
    CredentialRef::ApplicationDefault
}
```

### 4.5 Adapter Implementation

#### File: `crates/ecl-adapter-gcs/src/lib.rs`

```rust
#[derive(Debug)]
pub struct GcsAdapter {
    bucket: String,
    prefix: String,
    pattern: Option<glob::Pattern>,
    // Client initialized lazily or eagerly based on auth method
    client: google_cloud_storage::client::Client,
}

impl GcsAdapter {
    pub async fn from_spec(spec: &GcsSourceSpec) -> Result<Self, SourceError> {
        // 1. Resolve credentials (ADC, env var, file)
        // 2. Build GCS client
        // 3. Compile glob pattern if present
        // 4. Return adapter
    }
}

#[async_trait]
impl SourceAdapter for GcsAdapter {
    fn source_kind(&self) -> &str { "gcs" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        // 1. List objects with prefix
        // 2. Filter by glob pattern
        // 3. Map to SourceItem { id: object_name, display_name, mime_type, path, modified_at, source_hash (md5/crc32c) }
        // 4. Sort by name for determinism
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        // 1. Download object content as bytes
        // 2. Compute blake3 hash
        // 3. Build ItemProvenance with GCS metadata
        // 4. Return ExtractedDocument
    }
}
```

**Auth strategy:**
- `ApplicationDefault`: Use ADC (works on GCE, Cloud Run, local with `gcloud auth application-default login`)
- `EnvVar`: Read service account JSON from env var, parse, build credentials
- `File`: Read service account JSON from file path

**Error mapping:**
- 401/403 → `SourceError::AuthError`
- 404 → `SourceError::NotFound`
- 429 → `SourceError::RateLimited`
- 5xx / network → `SourceError::Transient`
- Other → `SourceError::Permanent`

**Tests:**
1. `test_gcs_adapter_from_spec_default_credentials`
2. `test_gcs_adapter_enumerate_filters_by_pattern`
3. `test_gcs_adapter_enumerate_no_pattern_returns_all`
4. `test_gcs_adapter_fetch_computes_hash`
5. `test_gcs_adapter_source_kind`
6. `test_gcs_adapter_serde_roundtrip` — GcsSourceSpec serialization

Note: Real GCS tests require either a test bucket or a mock. For unit tests, consider a thin HTTP abstraction that can be mocked (same approach as gdrive adapter with `wiremock`).

### 4.6 Registry Integration

#### File: `crates/ecl-cli/src/pipeline/registry.rs`

In `resolve_adapters`:
```rust
SourceSpec::Gcs(gcs_spec) => {
    let adapter = ecl_adapter_gcs::GcsAdapter::from_spec(gcs_spec).await
        .map_err(|e| ResolveError::UnknownAdapter {
            stage: name.to_string(),
            adapter: format!("gcs: {e}"),
        })?;
    adapters.insert(name.to_string(), Arc::new(adapter) as Arc<dyn SourceAdapter>);
}
```

In `source_kind`:
```rust
SourceSpec::Gcs(_) => "gcs",
```

### 4.7 Workspace Wiring

Add to root `Cargo.toml`:
```toml
[workspace]
members = [
    # ... existing ...
    "crates/ecl-adapter-gcs",
]
```

Add to `ecl-cli/Cargo.toml`:
```toml
ecl-adapter-gcs = { path = "../ecl-adapter-gcs" }
```

---

## 5. Milestone 7.4: Validation Stage

### 5.1 Scope

Configurable record validation with rule-based checking and error classification.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/validate.rs` (new)

**Configuration schema:**

```rust
#[derive(Debug, Clone, Deserialize)]
struct ValidateConfig {
    rules: Vec<ValidationRule>,
}

#[derive(Debug, Clone, Deserialize)]
struct ValidationRule {
    /// Field to validate.
    field: String,
    /// Check type: "required", "regex", "date_range", "length", "numeric_range"
    check: String,
    /// Severity: "hard" (reject record) or "soft" (warn, continue).
    #[serde(default = "default_hard")]
    severity: String,
    // Check-specific params (optional fields):
    /// Regex pattern (for "regex" check).
    #[serde(default)]
    pattern: Option<String>,
    /// Min value (for "date_range", "numeric_range", "length").
    #[serde(default)]
    min: Option<String>,
    /// Max value (for "date_range", "numeric_range", "length").
    #[serde(default)]
    max: Option<String>,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct ValidateStage {
    config: ValidateConfig,
    compiled_regexes: Vec<(usize, regex::Regex)>,
}

#[async_trait]
impl Stage for ValidateStage {
    fn name(&self) -> &str { "validate" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_ref().ok_or_else(|| StageError::Permanent { /* ... */ })?;

        let mut errors: Vec<serde_json::Value> = Vec::new();
        let mut has_hard_failure = false;

        for rule in &self.config.rules {
            if let Some(error) = self.check_rule(rule, record) {
                if rule.severity == "hard" {
                    has_hard_failure = true;
                }
                errors.push(error);
            }
        }

        // Always attach errors to metadata (even for soft failures)
        if !errors.is_empty() {
            item.metadata.insert(
                "_validation_errors".to_string(),
                serde_json::Value::Array(errors),
            );
            item.metadata.insert(
                "_validation_status".to_string(),
                if has_hard_failure {
                    serde_json::json!("failed")
                } else {
                    serde_json::json!("warned")
                },
            );
        } else {
            item.metadata.insert(
                "_validation_status".to_string(),
                serde_json::json!("passed"),
            );
        }

        // Item always passes through — downstream stages decide routing
        Ok(vec![item])
    }
}

impl ValidateStage {
    fn check_rule(
        &self,
        rule: &ValidationRule,
        record: &Record,
    ) -> Option<serde_json::Value> {
        let value = record.get(&rule.field);

        match rule.check.as_str() {
            "required" => {
                // Field must exist and not be null/empty string
                match value {
                    None | Some(Value::Null) => Some(json!({
                        "field": rule.field,
                        "check": "required",
                        "message": format!("field '{}' is required", rule.field),
                    })),
                    Some(Value::String(s)) if s.is_empty() => Some(json!({
                        "field": rule.field,
                        "check": "required",
                        "message": format!("field '{}' is empty", rule.field),
                    })),
                    _ => None,
                }
            }
            "regex" => {
                // Field must match regex pattern
                // ... find compiled regex, test against string value
            }
            "date_range" => {
                // Parse field as RFC3339, check within min..max
                // min/max support relative: "-10y", "+10y"
            }
            "length" => {
                // String length must be within min..max
            }
            "numeric_range" => {
                // Numeric value must be within min..max
            }
            _ => None, // Unknown check type → ignore (forward compatible)
        }
    }
}
```

**Tests:**
1. `test_validate_required_present` — passes
2. `test_validate_required_missing` — fails with error
3. `test_validate_required_null` — fails
4. `test_validate_required_empty_string` — fails
5. `test_validate_regex_matches` — passes
6. `test_validate_regex_no_match` — fails
7. `test_validate_date_range_in_bounds` — passes
8. `test_validate_date_range_too_old` — fails
9. `test_validate_hard_failure_sets_status`
10. `test_validate_soft_failure_sets_warned`
11. `test_validate_no_errors_sets_passed`
12. `test_validate_multiple_rules_all_checked`
13. `test_validate_no_record_returns_error`
14. `test_validate_affinity_rules` — full Affinity validation suite
15. `test_validate_from_params_invalid`

---

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

## 7. Milestone 7.6: GCS Sink Stage

### 7.1 Scope

New crate `ecl-sink-gcs` for writing records/content to GCS as files.

### 7.2 Crate Structure

```
crates/ecl-sink-gcs/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 7.3 Implementation

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GcsSinkConfig {
    pub bucket: String,
    pub prefix: String,
    /// Output format: "json_lines", "csv", "raw"
    #[serde(default = "default_json_lines")]
    pub format: String,
    /// Filename template. Default: "{timestamp}_{batch}"
    #[serde(default)]
    pub filename_template: Option<String>,
    /// Filter: "all", "valid_only", "errors_only"
    #[serde(default = "default_filter_all")]
    pub filter: String,
}
```

**Key behavior:**
- Collects all items from a batch, groups by filter criteria
- Writes one file per batch (all items serialized together)
- For `json_lines`: one JSON object per line
- For `csv`: header row + data rows
- For `errors_only`: only items with `_validation_status == "failed"`

**The stage accumulates items and writes on flush.** Since `Stage::process` receives one item at a time, the GCS sink buffers internally and writes on the last item or when the batch completes. Alternative: write per-item files (simpler, more files).

**Simpler approach (recommended for Phase 1):** Write one small JSON file per failed record. Path: `{prefix}/{timestamp}/{item_id}.json`. This avoids batch accumulation complexity.

**Tests:**
1. `test_gcs_sink_config_deserialize`
2. `test_gcs_sink_filter_errors_only`
3. `test_gcs_sink_json_format`

---

## 8. Milestone 7.7: File Lifecycle Management

### 8.1 Scope

Manage the `input/ → staging/ → historical/error/` file lifecycle for GCS-based pipelines.

### 8.2 Design

Add lifecycle configuration to PipelineSpec:

#### File: `crates/ecl-pipeline-spec/src/lib.rs`

```rust
pub struct PipelineSpec {
    // ... existing fields ...

    /// File lifecycle management for cloud storage sources.
    #[serde(default)]
    pub lifecycle: Option<LifecycleSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LifecycleSpec {
    /// GCS bucket for lifecycle operations.
    pub bucket: String,
    /// Prefix for staging files.
    #[serde(default = "default_staging")]
    pub staging_prefix: String,
    /// Prefix for successfully processed files.
    #[serde(default = "default_historical")]
    pub historical_prefix: String,
    /// Prefix for failed processing files.
    #[serde(default = "default_error")]
    pub error_prefix: String,
    /// What to do on pipeline success.
    #[serde(default = "default_move_historical")]
    pub on_success: String,  // "move_to_historical", "delete", "none"
    /// What to do on pipeline failure.
    #[serde(default = "default_move_input")]
    pub on_failure: String,  // "move_to_input", "move_to_error", "none"
}
```

### 8.3 Runner Hooks

#### File: `crates/ecl-pipeline/src/runner.rs`

Add pre-run and post-run lifecycle hooks:

```rust
impl PipelineRunner {
    async fn run(&mut self) -> Result<&PipelineState> {
        // ... existing phases ...

        // NEW: Post-run lifecycle
        if let Some(lifecycle) = &self.topology.spec.lifecycle {
            match &self.state.status {
                PipelineStatus::Completed { .. } => {
                    self.lifecycle_on_success(lifecycle).await?;
                }
                PipelineStatus::Failed { .. } => {
                    self.lifecycle_on_failure(lifecycle).await?;
                }
                _ => {}
            }
        }

        Ok(&self.state)
    }

    async fn lifecycle_on_success(&self, spec: &LifecycleSpec) -> Result<()> {
        // Move files from staging/ to historical/{run_id}/
        // Uses GCS client to copy + delete
    }

    async fn lifecycle_on_failure(&self, spec: &LifecycleSpec) -> Result<()> {
        // Move files from staging/ back to input/
    }
}
```

**Phase 1 simplification:** Since the runner doesn't currently own a GCS client, the lifecycle operations can be implemented as a special "lifecycle" stage that runs at the end of the pipeline. Or: the runner can construct a minimal GCS client from the lifecycle spec credentials. Recommend the latter for cleanliness.

**Tests:**
1. `test_lifecycle_spec_serde_roundtrip`
2. `test_lifecycle_defaults`
3. `test_runner_calls_lifecycle_on_success` (mock GCS)
4. `test_runner_calls_lifecycle_on_failure` (mock GCS)
5. `test_runner_no_lifecycle_no_error` — backward compatible

---

## 9. Milestone 7.8: Affinity End-to-End Integration Test

### 9.1 Scope

Full pipeline integration test that exercises the complete Affinity data flow.

### 9.2 Test Data

Create sample Affinity CSV at `crates/ecl-pipeline/tests/fixtures/affinity_sample.csv`:

```csv
Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code
AGG001,PRG001,ACCT-001,411111,1234,90210,MID-001,WALGREENS #1234,123 Main St,Los Angeles,CA,90001,5912,TXN-001,03/15/2026,03/16/2026,PUR,25.99,123
AGG001,PRG001,ACCT-002,422222,5678,10001,MID-002,CVS PHARMACY,456 Oak Ave,New York,NY,10002,5912,TXN-002,03/15/2026,03/16/2026,PUR,12.50,45
AGG001,PRG001,ACCT-003,433333,,90210,MID-003,WALGREENS #5678,789 Pine Rd,Chicago,IL,60601,5912,,03/15/2026,03/16/2026,PUR,8.75,789012
```

Row 3 has: empty `Card_Last_Four` (fails regex) and empty `Transaction_ID` (fails required).

### 9.3 Test Pipeline Spec

Create `crates/ecl-pipeline/tests/fixtures/affinity_pipeline.toml`:

Full TOML spec using filesystem source (substituting for GCS in tests), csv_parse, field_map, validate stages, with emit stage as output (substituting for Kafka in tests).

### 9.4 Test Scenarios

#### File: `crates/ecl-pipeline/tests/integration/affinity_e2e.rs` (new)

1. **`test_affinity_e2e_happy_path`**
   - Parse 3-row CSV
   - Map fields (rename, drop, set, date parse, pad)
   - Validate (required, regex, date_range)
   - Row 1 & 2: pass validation → emitted to output
   - Row 3: fail validation → metadata has `_validation_errors`
   - Verify output file content matches expected canonical format

2. **`test_affinity_e2e_empty_csv`**
   - Headers-only CSV file
   - Pipeline completes successfully with 0 items

3. **`test_affinity_e2e_all_valid`**
   - All rows pass validation
   - No `_validation_errors` in metadata

4. **`test_affinity_e2e_checkpoint_resume`**
   - Run pipeline, interrupt after parse stage
   - Resume from checkpoint
   - Verify items processed correctly

5. **`test_affinity_e2e_field_mapping_correctness`**
   - Verify specific field mappings:
     - `Account_ID` → `finx_consumer_token`
     - `Transaction_Date` (MM/DD/YYYY) → `purchase_ts` (RFC3339)
     - `Transaction_Auth_Code` → padded to 6 digits
     - `Merchant_Location_Name` → regex extract → `merchant_store_id`
     - `country` set to `"US"`, `currency` to `"USD"`, `byn_partner_id` to `290`

---

## 10. Cross-Cutting Concerns

### 10.1 Workspace Dependencies

Add to root `Cargo.toml` `[workspace.dependencies]`:
```toml
csv = "1"
regex = "1"
rdkafka = { version = "0.37", features = ["cmake-build", "ssl", "sasl"] }
apache-avro = "0.17"
google-cloud-storage = { version = "0.22", features = ["default-tls"] }
google-cloud-auth = { version = "0.19", features = ["default-tls"] }
```

### 10.2 Linting Rules

All new crates inherit workspace linting:
```toml
[lints.clippy]
unwrap_used = "deny"
panic = "deny"
```

All `#[cfg(test)]` modules use `#[allow(clippy::unwrap_used)]` per existing convention.

### 10.3 Error Handling Pattern

All new stages follow the established pattern:
- Parse errors → `StageError::Permanent` (config is wrong, won't fix itself)
- Network/transient errors → `StageError::Transient` (retry will help)
- Missing record → `StageError::Permanent` (pipeline is misconfigured)
- Individual row parse errors → skip or fail per config (not stage-level error)

### 10.4 Documentation

Each new public type gets a doc comment per TR-09 / DC-01:
- Types: one-line summary + field documentation
- Trait impls: note any non-obvious behavior
- Modules: `//!` header explaining purpose

### 10.5 Test Coverage Target

Per `CLAUDE-CODE-COVERAGE.md`: ≥95% coverage on all new code.

Each milestone's test list above is designed to cover:
- Happy path
- Error paths (invalid config, missing data, type conversion failures)
- Edge cases (empty input, single row, large input)
- Serde roundtrips for all config types
- Integration with existing types (`PipelineItem`, `StageContext`)

### 10.6 Backward Compatibility

**No breaking changes to existing types:**
- `PipelineItem` gains `record: Option<Record>` with `#[serde(default)]` — existing serialized checkpoints deserialize with `record: None`
- `SourceSpec` gains `Gcs` variant — existing TOML specs are unaffected
- `PipelineSpec` gains `lifecycle: Option<LifecycleSpec>` with `#[serde(default)]`
- All existing stages continue to work unchanged
- Existing integration tests must still pass

---

## 11. Verification Checklist

### Per-Milestone

- [ ] `make test` passes (all existing + new tests)
- [ ] `make lint` passes
- [ ] `make format` passes
- [ ] No compiler warnings
- [ ] Coverage ≥ 95% on new code
- [ ] Checked against AP-01 through AP-20
- [ ] Doc comments on all public items
- [ ] Serde roundtrip tests for all config types

### Phase 1 Complete

- [ ] Affinity pipeline spec parses from TOML
- [ ] GCS source lists and fetches objects
- [ ] CSV parse produces Records from file content
- [ ] Field map transforms Records per configuration
- [ ] Validation checks rules, classifies hard/soft
- [ ] Kafka sink serializes to Avro wire format and produces
- [ ] GCS sink writes error records
- [ ] File lifecycle moves files on success/failure
- [ ] End-to-end integration test passes
- [ ] Existing pipeline tests still pass (no regressions)

---

## Appendix A: Milestone Dependency Graph

```
7.1 Record + CSV Parse ──────────────────────────────┐
    │                                                 │
    ├── 7.2 Field Map ───────────────┐                │
    │                                │                │
    ├── 7.3 GCS Source ──────────────┤                │
    │                                │                │
    ├── 7.4 Validation ──────────────┤                │
    │                                │                │
    │   7.5 Kafka Sink ─────────────┤ (can parallel  │
    │                                │  with 7.3-7.4) │
    │   7.6 GCS Sink ──────────────┤                 │
    │                                │                │
    └───────────────────────────────┘                │
                                     │                │
                              7.7 File Lifecycle ─────┤
                                     │                │
                              7.8 Affinity E2E ───────┘
```

**Parallelizable work:**
- 7.2 + 7.3 can proceed in parallel after 7.1
- 7.4 can proceed in parallel with 7.3
- 7.5 + 7.6 can proceed in parallel with 7.2-7.4
- 7.7 depends on 7.3 (GCS operations)
- 7.8 depends on all prior milestones

## Appendix B: Files Changed/Created Summary

### New Files
| File | Milestone |
|------|-----------|
| `crates/ecl-stages/src/csv_parse.rs` | 7.1 |
| `crates/ecl-stages/src/field_map.rs` | 7.2 |
| `crates/ecl-adapter-gcs/Cargo.toml` | 7.3 |
| `crates/ecl-adapter-gcs/src/lib.rs` | 7.3 |
| `crates/ecl-stages/src/validate.rs` | 7.4 |
| `crates/ecl-sink-kafka/Cargo.toml` | 7.5 |
| `crates/ecl-sink-kafka/src/lib.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/producer.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/avro.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/registry.rs` | 7.5 |
| `crates/ecl-sink-gcs/Cargo.toml` | 7.6 |
| `crates/ecl-sink-gcs/src/lib.rs` | 7.6 |
| `tests/fixtures/affinity_sample.csv` | 7.8 |
| `tests/fixtures/affinity_pipeline.toml` | 7.8 |
| `tests/integration/affinity_e2e.rs` | 7.8 |

### Modified Files
| File | Milestone | Change |
|------|-----------|--------|
| `Cargo.toml` (workspace) | 7.3, 7.5, 7.6 | Add new crate members + deps |
| `crates/ecl-pipeline-topo/src/traits.rs` | 7.1 | Add `Record` type alias + `record` field to `PipelineItem` |
| `crates/ecl-pipeline-spec/src/source.rs` | 7.3 | Add `Gcs` variant + `GcsSourceSpec` |
| `crates/ecl-pipeline-spec/src/lib.rs` | 7.7 | Add `lifecycle` field + `LifecycleSpec` |
| `crates/ecl-stages/src/lib.rs` | 7.1, 7.2, 7.4 | Add module declarations + re-exports |
| `crates/ecl-stages/Cargo.toml` | 7.1, 7.2 | Add `csv`, `regex`, `chrono` deps |
| `crates/ecl-cli/Cargo.toml` | 7.3, 7.5, 7.6 | Add new crate deps |
| `crates/ecl-cli/src/pipeline/registry.rs` | 7.1-7.6 | Register new stages + GCS adapter |
| `crates/ecl-pipeline/src/runner.rs` | 7.7 | Add lifecycle hooks |
