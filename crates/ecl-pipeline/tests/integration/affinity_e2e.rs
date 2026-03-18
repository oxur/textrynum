//! Affinity end-to-end integration tests: CSV → parse → map → validate → emit.
//!
//! Exercises the complete Affinity fintech data flow using the stage chain
//! directly. Uses filesystem source + emit stage as substitutes for GCS + Kafka.

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ecl_adapter_fs::FilesystemAdapter;
use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::source::FilesystemSourceSpec;
use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec, ResourceSpec, SourceSpec, StageSpec};
use ecl_pipeline_state::{Blake3Hash, InMemoryStateStore, PipelineStatus, StageId};
use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{
    PipelineItem, PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter, Stage, StageContext,
};
use ecl_stages::{CsvParseStage, EmitStage, FieldMapStage, ValidateStage};
use serde_json::json;
use tempfile::TempDir;

/// The sample CSV content (same as fixtures/affinity_sample.csv).
const AFFINITY_CSV: &str = "\
Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code\n\
AGG001,PRG001,ACCT-001,411111,1234,90210,MID-001,ACME INC. #1234,123 Main St,Los Angeles,CA,90001,5912,TXN-001,03/15/2026,03/16/2026,PUR,25.99,123\n\
AGG001,PRG001,ACCT-002,422222,5678,10001,MID-002,CVS PHARMACY,456 Oak Ave,New York,NY,10002,5912,TXN-002,03/15/2026,03/16/2026,PUR,12.50,456\n\
AGG001,PRG001,ACCT-003,433333,,90210,MID-003,ACME INC. #5678,789 Pine Rd,Chicago,IL,60601,5912,,03/15/2026,03/16/2026,PUR,8.75,789012\n";

fn fast_retry() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 1,
        initial_backoff: Duration::from_millis(1),
        backoff_multiplier: 1.0,
        max_backoff: Duration::from_millis(10),
    }
}

/// CSV parse params matching the Affinity schema (19 columns).
fn csv_parse_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "Channel_Aggregator_ID", "type": "string" },
            { "name": "Program_ID", "type": "string" },
            { "name": "Account_ID", "type": "string" },
            { "name": "Card_BIN", "type": "string" },
            { "name": "Card_Last_Four", "type": "string" },
            { "name": "Account_Postal_Code", "type": "string" },
            { "name": "Merchant_Location_MID", "type": "string" },
            { "name": "Merchant_Location_Name", "type": "string" },
            { "name": "Merchant_Location_Street", "type": "string" },
            { "name": "Merchant_Location_City", "type": "string" },
            { "name": "Merchant_Location_State", "type": "string" },
            { "name": "Merchant_Location_Postal_Code", "type": "string" },
            { "name": "Merchant_Location_Category_Code", "type": "string" },
            { "name": "Transaction_ID", "type": "string" },
            { "name": "Transaction_Date", "type": "string" },
            { "name": "Transaction_Settlement_Date", "type": "string" },
            { "name": "Transaction_Code", "type": "string" },
            { "name": "Transaction_Amount", "type": "float" },
            { "name": "Transaction_Auth_Code", "type": "string" }
        ],
        "has_headers": true,
        "on_row_error": "skip"
    })
}

/// Field map params for the Affinity pipeline.
///
/// Mapping:
/// - Account_ID → finx_consumer_token
/// - Transaction_Date (MM/DD/YYYY) → purchase_ts (RFC 3339)
/// - Transaction_Auth_Code → padded to 6 digits
/// - Merchant_Location_Name → regex extract → merchant_store_id (e.g., "1234" from "ACME INC. #1234")
/// - Set country="US", currency="USD", byn_partner_id=290
fn field_map_params() -> serde_json::Value {
    json!({
        "rename": [
            { "from": "Account_ID", "to": "finx_consumer_token" },
            { "from": "Transaction_Amount", "to": "purchase_amount" },
            { "from": "Card_BIN", "to": "card_bin" },
            { "from": "Card_Last_Four", "to": "card_last_four" },
            { "from": "Merchant_Location_MID", "to": "merchant_mid" },
            { "from": "Merchant_Location_Category_Code", "to": "mcc" },
            { "from": "Transaction_Code", "to": "transaction_type" }
        ],
        "set": [
            { "field": "country", "value": "US" },
            { "field": "currency", "value": "USD" },
            { "field": "byn_partner_id", "value": 290 }
        ],
        "parse_dates": [
            {
                "field": "Transaction_Date",
                "format": "%m/%d/%Y",
                "output": "purchase_ts"
            },
            {
                "field": "Transaction_Settlement_Date",
                "format": "%m/%d/%Y",
                "output": "settlement_ts"
            }
        ],
        "pad": [
            { "field": "Transaction_Auth_Code", "width": 6, "pad_char": "0", "side": "left" }
        ],
        "regex_extract": [
            {
                "field": "Merchant_Location_Name",
                "pattern": "#(\\d+)",
                "output": "merchant_store_id",
                "group": 1
            }
        ],
        "drop": [
            "Channel_Aggregator_ID",
            "Program_ID",
            "Account_Postal_Code",
            "Transaction_Date",
            "Transaction_Settlement_Date"
        ]
    })
}

/// Validation params: check required fields, regex on card_last_four,
/// date range on purchase_ts.
fn validate_params() -> serde_json::Value {
    json!({
        "rules": [
            { "field": "finx_consumer_token", "check": "required", "severity": "hard" },
            { "field": "Transaction_ID", "check": "required", "severity": "hard" },
            {
                "field": "card_last_four",
                "check": "regex",
                "pattern": "^\\d{4}$",
                "severity": "hard"
            },
            {
                "field": "purchase_ts",
                "check": "date_range",
                "min": "2020-01-01T00:00:00Z",
                "max": "2030-12-31T23:59:59Z",
                "severity": "hard"
            }
        ]
    })
}

/// Make a PipelineItem from CSV content as if extracted from a source.
fn make_csv_item(csv_content: &str) -> PipelineItem {
    PipelineItem {
        id: "affinity_sample.csv".to_string(),
        display_name: "affinity_sample.csv".to_string(),
        content: Arc::from(csv_content.as_bytes()),
        mime_type: "text/csv".to_string(),
        source_name: "local".to_string(),
        source_content_hash: Blake3Hash::new("test"),
        provenance: ecl_pipeline_state::ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: BTreeMap::new(),
            source_modified: None,
            extracted_at: chrono::Utc::now(),
        },
        metadata: BTreeMap::new(),
        record: None,
    }
}

/// Build a StageContext with the given params and output dir.
fn make_context(output_dir: PathBuf, params: serde_json::Value) -> StageContext {
    StageContext {
        spec: Arc::new(PipelineSpec {
            name: "affinity-e2e".to_string(),
            version: 1,
            output_dir: output_dir.clone(),
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            defaults: DefaultsSpec::default(),
            lifecycle: None,
        }),
        output_dir,
        params,
        span: tracing::Span::none(),
    }
}

/// Run an item through a stage chain: csv_parse → field_map → validate.
/// Returns the final items after validation.
async fn run_stage_chain(csv_content: &str) -> Vec<PipelineItem> {
    let csv_parse = CsvParseStage::from_params(&csv_parse_params()).unwrap();
    let field_map = FieldMapStage::from_params(&field_map_params()).unwrap();
    let validate = ValidateStage::from_params(&validate_params()).unwrap();

    let output_dir = PathBuf::from("/tmp/affinity-test");

    // Step 1: CSV parse (1 file → N row items)
    let csv_item = make_csv_item(csv_content);
    let ctx = make_context(output_dir.clone(), csv_parse_params());
    let parsed_items = csv_parse.process(csv_item, &ctx).await.unwrap();

    // Step 2: Field map (each row item)
    let ctx = make_context(output_dir.clone(), field_map_params());
    let mut mapped_items = Vec::new();
    for item in parsed_items {
        let results = field_map.process(item, &ctx).await.unwrap();
        mapped_items.extend(results);
    }

    // Step 3: Validate (each mapped item)
    let ctx = make_context(output_dir, validate_params());
    let mut validated_items = Vec::new();
    for item in mapped_items {
        let results = validate.process(item, &ctx).await.unwrap();
        validated_items.extend(results);
    }

    validated_items
}

// ── Happy-path: 3-row CSV through full stage chain ──────────────────

#[tokio::test]
async fn test_affinity_e2e_happy_path() {
    let items = run_stage_chain(AFFINITY_CSV).await;

    // 3 rows parsed
    assert_eq!(items.len(), 3, "should parse 3 rows from CSV");

    // Row 1 and 2 should pass validation
    let row1_status = items[0]
        .metadata
        .get("_validation_status")
        .and_then(|v| v.as_str());
    let row2_status = items[1]
        .metadata
        .get("_validation_status")
        .and_then(|v| v.as_str());
    assert_eq!(row1_status, Some("passed"), "Row 1 should pass validation");
    assert_eq!(row2_status, Some("passed"), "Row 2 should pass validation");

    // Row 3 should fail validation (empty Card_Last_Four fails regex, empty Transaction_ID fails required)
    let row3_status = items[2]
        .metadata
        .get("_validation_status")
        .and_then(|v| v.as_str());
    assert_eq!(
        row3_status,
        Some("failed"),
        "Row 3 should fail validation"
    );

    // Row 3 should have validation errors
    let row3_errors = items[2]
        .metadata
        .get("_validation_errors")
        .and_then(|v| v.as_array());
    assert!(
        row3_errors.is_some(),
        "Row 3 should have _validation_errors"
    );
    let errors = row3_errors.unwrap();
    assert!(errors.len() >= 2, "Row 3 should have at least 2 validation errors (required + regex), got {}", errors.len());
}

#[tokio::test]
async fn test_affinity_e2e_field_mapping_correctness() {
    let items = run_stage_chain(AFFINITY_CSV).await;
    assert_eq!(items.len(), 3);

    // Check row 1 field mappings
    let record = items[0].record.as_ref().unwrap();

    // Account_ID → finx_consumer_token
    assert_eq!(
        record.get("finx_consumer_token").and_then(|v| v.as_str()),
        Some("ACCT-001"),
        "Account_ID should map to finx_consumer_token"
    );

    // Original Account_ID should be gone (renamed)
    assert!(
        record.get("Account_ID").is_none(),
        "Account_ID should be renamed (removed)"
    );

    // Transaction_Date → purchase_ts (RFC 3339)
    let purchase_ts = record.get("purchase_ts").and_then(|v| v.as_str()).unwrap();
    assert!(
        purchase_ts.contains("2026-03-15"),
        "purchase_ts should be RFC 3339 with date 2026-03-15, got: {purchase_ts}"
    );

    // Settlement date parsed
    let settlement_ts = record.get("settlement_ts").and_then(|v| v.as_str()).unwrap();
    assert!(
        settlement_ts.contains("2026-03-16"),
        "settlement_ts should contain 2026-03-16, got: {settlement_ts}"
    );

    // Transaction_Auth_Code → padded to 6 digits
    assert_eq!(
        record
            .get("Transaction_Auth_Code")
            .and_then(|v| v.as_str()),
        Some("000123"),
        "Auth code 123 should be left-padded to 000123"
    );

    // Merchant_Location_Name → merchant_store_id (regex extract "#(\d+)")
    assert_eq!(
        record.get("merchant_store_id").and_then(|v| v.as_str()),
        Some("1234"),
        "Should extract store ID 1234 from ACME INC. #1234"
    );

    // Set constants
    assert_eq!(
        record.get("country").and_then(|v| v.as_str()),
        Some("US")
    );
    assert_eq!(
        record.get("currency").and_then(|v| v.as_str()),
        Some("USD")
    );
    assert_eq!(
        record.get("byn_partner_id").and_then(|v| v.as_i64()),
        Some(290)
    );

    // Dropped fields should be absent
    assert!(record.get("Channel_Aggregator_ID").is_none(), "Dropped field should be absent");
    assert!(record.get("Program_ID").is_none(), "Dropped field should be absent");
    assert!(record.get("Account_Postal_Code").is_none(), "Dropped field should be absent");
    assert!(record.get("Transaction_Date").is_none(), "Dropped field should be absent");

    // Renamed fields
    assert_eq!(
        record.get("purchase_amount").and_then(|v| v.as_f64()),
        Some(25.99),
        "Transaction_Amount should map to purchase_amount"
    );
}

#[tokio::test]
async fn test_affinity_e2e_row2_field_mapping() {
    let items = run_stage_chain(AFFINITY_CSV).await;
    let record = items[1].record.as_ref().unwrap();

    // Row 2: CVS PHARMACY (no "#\d+" match) → merchant_store_id should be null
    assert_eq!(
        record.get("merchant_store_id"),
        Some(&serde_json::Value::Null),
        "CVS PHARMACY has no #<id> → merchant_store_id should be null"
    );

    assert_eq!(
        record.get("finx_consumer_token").and_then(|v| v.as_str()),
        Some("ACCT-002")
    );

    // Auth code "456" padded to "000456"
    assert_eq!(
        record
            .get("Transaction_Auth_Code")
            .and_then(|v| v.as_str()),
        Some("000456")
    );
}

#[tokio::test]
async fn test_affinity_e2e_row3_validation_errors() {
    let items = run_stage_chain(AFFINITY_CSV).await;
    let row3 = &items[2];

    let errors = row3
        .metadata
        .get("_validation_errors")
        .and_then(|v| v.as_array())
        .unwrap();

    // Should have at least: Transaction_ID required, card_last_four regex
    let error_fields: Vec<&str> = errors
        .iter()
        .filter_map(|e| e.get("field").and_then(|f| f.as_str()))
        .collect();

    assert!(
        error_fields.contains(&"Transaction_ID"),
        "Should fail on Transaction_ID required check, errors: {error_fields:?}"
    );
    assert!(
        error_fields.contains(&"card_last_four"),
        "Should fail on card_last_four regex check, errors: {error_fields:?}"
    );
}

#[tokio::test]
async fn test_affinity_e2e_empty_csv() {
    // Headers-only CSV
    let headers_only = "Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code\n";

    let items = run_stage_chain(headers_only).await;
    assert_eq!(items.len(), 0, "Headers-only CSV should produce 0 items");
}

#[tokio::test]
async fn test_affinity_e2e_all_valid() {
    // CSV with only valid rows (both have required fields and 4-digit card_last_four)
    let csv = "\
Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code\n\
AGG001,PRG001,ACCT-001,411111,1234,90210,MID-001,ACME INC. #1234,123 Main St,Los Angeles,CA,90001,5912,TXN-001,03/15/2026,03/16/2026,PUR,25.99,123\n\
AGG001,PRG001,ACCT-002,422222,5678,10001,MID-002,CVS PHARMACY,456 Oak Ave,New York,NY,10002,5912,TXN-002,03/15/2026,03/16/2026,PUR,12.50,456\n";

    let items = run_stage_chain(csv).await;
    assert_eq!(items.len(), 2);

    for (i, item) in items.iter().enumerate() {
        let status = item
            .metadata
            .get("_validation_status")
            .and_then(|v| v.as_str());
        assert_eq!(
            status,
            Some("passed"),
            "Row {} should pass validation",
            i + 1
        );
        assert!(
            !item.metadata.contains_key("_validation_errors")
                || item
                    .metadata
                    .get("_validation_errors")
                    .and_then(|v| v.as_array())
                    .is_none_or(|a| a.is_empty()),
            "Row {} should have no validation errors",
            i + 1
        );
    }
}

// ── Runner-level E2E test with combined stage ───────────────────────

/// A combined stage that chains extract → csv_parse → field_map → validate → emit.
///
/// Works around the runner's per-stage completion marking by performing the
/// entire Affinity pipeline in a single stage handler.
#[derive(Debug)]
struct AffinityPipelineStage {
    adapter: Arc<dyn SourceAdapter>,
    source_name: String,
    csv_parse: CsvParseStage,
    field_map: FieldMapStage,
    validate: ValidateStage,
    emit: EmitStage,
}

#[async_trait::async_trait]
impl Stage for AffinityPipelineStage {
    fn name(&self) -> &str {
        "affinity-pipeline"
    }

    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Extract content from source
        let source_item = ecl_pipeline_topo::SourceItem {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            mime_type: item.mime_type.clone(),
            path: item.id.clone(),
            modified_at: item.provenance.source_modified,
            source_hash: None,
        };

        let doc = self
            .adapter
            .fetch(&source_item)
            .await
            .map_err(|e| StageError::Permanent {
                stage: "affinity-pipeline".to_string(),
                item_id: item.id.clone(),
                message: format!("fetch failed: {e}"),
            })?;

        let extracted = PipelineItem {
            id: doc.id,
            display_name: doc.display_name,
            content: Arc::from(doc.content),
            mime_type: doc.mime_type,
            source_name: self.source_name.clone(),
            source_content_hash: doc.content_hash,
            provenance: doc.provenance,
            metadata: BTreeMap::new(),
            record: None,
        };

        // 2. CSV parse (fan-out)
        let parsed_items = self.csv_parse.process(extracted, ctx).await?;

        // 3. Field map each row
        let mut mapped_items = Vec::new();
        for parsed_item in parsed_items {
            let results = self.field_map.process(parsed_item, ctx).await?;
            mapped_items.extend(results);
        }

        // 4. Validate each row
        let mut validated_items = Vec::new();
        for mapped_item in mapped_items {
            let results = self.validate.process(mapped_item, ctx).await?;
            validated_items.extend(results);
        }

        // 5. Emit each row to output directory
        let mut emitted = Vec::new();
        for validated_item in validated_items {
            let results = self.emit.process(validated_item, ctx).await?;
            emitted.extend(results);
        }

        Ok(emitted)
    }
}

/// Build a full Affinity pipeline topology using the combined stage.
fn build_affinity_topo(
    input_dir: &std::path::Path,
    output_dir: &std::path::Path,
) -> PipelineTopology {
    let fs_spec = FilesystemSourceSpec {
        root: input_dir.to_path_buf(),
        filters: vec![],
        extensions: vec!["csv".to_string()],
    };
    let adapter: Arc<dyn SourceAdapter> =
        Arc::new(FilesystemAdapter::from_fs_spec("local", &fs_spec).unwrap());

    let stage: Arc<dyn Stage> = Arc::new(AffinityPipelineStage {
        adapter: adapter.clone(),
        source_name: "local".to_string(),
        csv_parse: CsvParseStage::from_params(&csv_parse_params()).unwrap(),
        field_map: FieldMapStage::from_params(&field_map_params()).unwrap(),
        validate: ValidateStage::from_params(&validate_params()).unwrap(),
        emit: EmitStage::new(),
    });

    let spec = Arc::new(PipelineSpec {
        name: "affinity-e2e".to_string(),
        version: 1,
        output_dir: output_dir.to_path_buf(),
        sources: BTreeMap::from([(
            "local".to_string(),
            SourceSpec::Filesystem(fs_spec),
        )]),
        stages: BTreeMap::from([(
            "affinity-pipeline".to_string(),
            StageSpec {
                adapter: "affinity-pipeline".to_string(),
                source: Some("local".to_string()),
                resources: ResourceSpec {
                    creates: vec!["output".to_string()],
                    reads: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        )]),
        defaults: DefaultsSpec::default(),
        lifecycle: None,
    });

    let spec_hash_bytes = serde_json::to_string(&*spec).unwrap();
    let spec_hash = Blake3Hash::new(blake3::hash(spec_hash_bytes.as_bytes()).to_hex().as_str());

    PipelineTopology {
        spec,
        spec_hash,
        sources: BTreeMap::from([("local".to_string(), adapter)]),
        stages: BTreeMap::from([(
            "affinity-pipeline".to_string(),
            ResolvedStage {
                id: StageId::new("affinity-pipeline"),
                handler: stage,
                retry: fast_retry(),
                skip_on_error: false,
                timeout: None,
                source: Some("local".to_string()),
                condition: None,
            },
        )]),
        push_sources: BTreeMap::new(),
        schedule: vec![vec![StageId::new("affinity-pipeline")]],
        output_dir: output_dir.to_path_buf(),
    }
}

#[tokio::test]
async fn test_affinity_e2e_runner_full_pipeline() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    // Write the sample CSV
    fs::write(input.path().join("affinity_sample.csv"), AFFINITY_CSV).unwrap();

    let topo = build_affinity_topo(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(
        matches!(state.status, PipelineStatus::Completed { .. }),
        "Pipeline should complete, was {:?}",
        state.status
    );

    // 1 CSV file discovered as source item
    assert_eq!(state.stats.total_items_discovered, 1);

    // Emit should have written 3 output files (one per CSV row)
    // Output filenames follow the pattern: affinity_sample.csv:row:N
    assert!(
        output.path().join("affinity_sample.csv:row:2").exists(),
        "Row 1 output should exist"
    );
    assert!(
        output.path().join("affinity_sample.csv:row:3").exists(),
        "Row 2 output should exist"
    );
    assert!(
        output.path().join("affinity_sample.csv:row:4").exists(),
        "Row 3 output should exist"
    );
}

#[tokio::test]
async fn test_affinity_e2e_runner_empty_input() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    // No CSV files in input
    let topo = build_affinity_topo(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    assert_eq!(state.stats.total_items_discovered, 0);
}

#[tokio::test]
async fn test_affinity_e2e_runner_headers_only_csv() {
    let input = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();

    let headers_only = "Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code\n";
    fs::write(input.path().join("empty.csv"), headers_only).unwrap();

    let topo = build_affinity_topo(input.path(), output.path());
    let store = Box::new(InMemoryStateStore::new());
    let mut runner = PipelineRunner::new(topo, store).await.unwrap();

    let state = runner.run().await.unwrap();
    assert!(matches!(state.status, PipelineStatus::Completed { .. }));
    assert_eq!(state.stats.total_items_discovered, 1);
}

#[tokio::test]
async fn test_affinity_e2e_row3_auth_code_no_padding_needed() {
    let items = run_stage_chain(AFFINITY_CSV).await;
    let record = items[2].record.as_ref().unwrap();

    // Row 3 auth code is "789012" (already 6 digits) — no padding needed
    assert_eq!(
        record
            .get("Transaction_Auth_Code")
            .and_then(|v| v.as_str()),
        Some("789012"),
        "Auth code 789012 needs no padding"
    );
}

#[tokio::test]
async fn test_affinity_e2e_fixture_file_matches() {
    // Verify that the fixture CSV file is readable and matches expected format
    let fixture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/affinity_sample.csv"
    );
    let content = fs::read_to_string(fixture_path).unwrap();
    assert!(content.starts_with("Channel_Aggregator_ID,"));
    assert!(content.contains("ACME INC. #1234"));
    assert!(content.contains("CVS PHARMACY"));

    // Parse the fixture through the stage chain
    let items = run_stage_chain(&content).await;
    assert_eq!(items.len(), 3, "Fixture CSV should produce 3 items");
}
