# 2. Milestone 7.1: Record Type & CSV Parse Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 125-339. Original bytes are recoverable from import commit 3a39aea. -->

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

