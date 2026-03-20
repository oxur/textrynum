//! Giant Eagle end-to-end integration test: CSV parse → field map → lookup →
//! join → aggregate → assemble.
//!
//! Exercises a multi-stream grocery receipt pipeline using the stage chain
//! directly (not through PipelineRunner, since runner wiring for all Phase 2
//! stages is not yet complete).

#![allow(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec};
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};
use ecl_stages::{
    AggregateStage, AssembleStage, CsvParseStage, FieldMapStage, JoinStage, LookupStage,
};
use serde_json::json;

// ── Fixture paths ────────────────────────────────────────────────────

const STORES_CSV: &str = include_str!("../fixtures/giant_eagle/stores.csv");
const TRANSACTIONS_CSV: &str = include_str!("../fixtures/giant_eagle/transactions.csv");
const ITEMS_CSV: &str = include_str!("../fixtures/giant_eagle/items.csv");
const PRODUCTS_CSV: &str = include_str!("../fixtures/giant_eagle/products.csv");
const TENDERS_CSV: &str = include_str!("../fixtures/giant_eagle/tenders.csv");

// ── Helpers ──────────────────────────────────────────────────────────

fn make_csv_item(id: &str, csv_content: &str, stream: &str) -> PipelineItem {
    PipelineItem {
        id: id.to_string(),
        display_name: id.to_string(),
        content: Arc::from(csv_content.as_bytes()),
        mime_type: "text/csv".to_string(),
        source_name: "local".to_string(),
        source_content_hash: Blake3Hash::new("test"),
        provenance: ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: BTreeMap::new(),
            source_modified: None,
            extracted_at: chrono::Utc::now(),
        },
        metadata: BTreeMap::new(),
        record: None,
        stream: Some(stream.to_string()),
    }
}

fn make_context(params: serde_json::Value) -> StageContext {
    let output_dir = PathBuf::from("/tmp/giant-eagle-test");
    StageContext {
        spec: Arc::new(PipelineSpec {
            name: "giant-eagle-e2e".to_string(),
            version: 1,
            output_dir: output_dir.clone(),
            sources: BTreeMap::new(),
            stages: BTreeMap::new(),
            defaults: DefaultsSpec::default(),
            lifecycle: None,
            secrets: Default::default(),
            triggers: None,
            schedule: None,
        }),
        output_dir,
        params,
        span: tracing::Span::none(),
    }
}

// ── CSV parse params (headerless) ────────────────────────────────────

fn stores_csv_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "store_id", "type": "string" },
            { "name": "name", "type": "string" },
            { "name": "address", "type": "string" },
            { "name": "city", "type": "string" },
            { "name": "state", "type": "string" },
            { "name": "zip", "type": "string" }
        ],
        "has_headers": false
    })
}

fn transactions_csv_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "txn_id", "type": "string" },
            { "name": "store_id", "type": "string" },
            { "name": "txn_date", "type": "string" },
            { "name": "total", "type": "float" }
        ],
        "has_headers": false
    })
}

fn items_csv_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "txn_id", "type": "string" },
            { "name": "upc", "type": "string" },
            { "name": "description", "type": "string" },
            { "name": "quantity", "type": "integer" },
            { "name": "price", "type": "float" }
        ],
        "has_headers": false
    })
}

fn products_csv_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "upc", "type": "string" },
            { "name": "product_name", "type": "string" },
            { "name": "category", "type": "string" },
            { "name": "department", "type": "string" }
        ],
        "has_headers": false
    })
}

fn tenders_csv_params() -> serde_json::Value {
    json!({
        "columns": [
            { "name": "txn_id", "type": "string" },
            { "name": "tender_type", "type": "string" },
            { "name": "amount", "type": "float" },
            { "name": "card_scheme", "type": "string" }
        ],
        "has_headers": false
    })
}

// ── Field map params (simple renames to standardize) ─────────────────

fn transactions_field_map_params() -> serde_json::Value {
    json!({
        "rename": [
            { "from": "txn_date", "to": "transaction_date" }
        ],
        "parse_dates": [{
            "field": "transaction_date",
            "format": "%m/%d/%Y",
            "output": "transaction_ts"
        }]
    })
}

fn tenders_field_map_params() -> serde_json::Value {
    json!({
        "rename": [
            { "from": "amount", "to": "tender_amount" }
        ]
    })
}

// ── Lookup params ────────────────────────────────────────────────────

fn tender_type_lookup_params() -> serde_json::Value {
    json!({
        "lookups": [{
            "field": "tender_type",
            "output": "payment_method",
            "table": {
                "01": "Cash",
                "02": "Check",
                "03": "Credit Card",
                "04": "Debit Card",
                "05": "EBT",
                "06": "Gift Card"
            },
            "default": "Other"
        }]
    })
}

fn card_scheme_lookup_params() -> serde_json::Value {
    json!({
        "lookups": [{
            "field": "card_scheme",
            "output": "card_network",
            "table": {
                "vi": "Visa",
                "mc": "MasterCard",
                "ax": "AmEx",
                "di": "Discover"
            },
            "case_insensitive": true,
            "default": null
        }]
    })
}

// ── Join params ──────────────────────────────────────────────────────

fn items_products_join_params() -> serde_json::Value {
    json!({
        "join_type": "left",
        "left_stream": "items",
        "right_stream": "products",
        "left_key": "upc",
        "right_key": "upc",
        "right_prefix": "product_"
    })
}

// ── Aggregate params ─────────────────────────────────────────────────

fn tenders_aggregate_params() -> serde_json::Value {
    json!({
        "group_by": ["txn_id"],
        "aggregates": [
            { "field": "tender_amount", "function": "sum", "output": "total_tendered" },
            { "field": "tender_amount", "function": "count", "output": "tender_count" }
        ],
        "collect_arrays": [{
            "output": "payment_details",
            "fields": ["tender_type", "payment_method", "tender_amount", "card_scheme", "card_network"]
        }]
    })
}

// ── Assemble params ──────────────────────────────────────────────────

fn receipt_assemble_params() -> serde_json::Value {
    json!({
        "primary_stream": "transactions",
        "primary_key": "txn_id",
        "joins": [
            {
                "stream": "stores",
                "key": "store_id",
                "foreign_key": "store_id",
                "nest_as": "store",
                "collect": false
            },
            {
                "stream": "items",
                "key": "txn_id",
                "foreign_key": "txn_id",
                "nest_as": "line_items",
                "collect": true
            },
            {
                "stream": "tenders",
                "key": "txn_id",
                "foreign_key": "txn_id",
                "nest_as": "payments",
                "collect": true
            }
        ]
    })
}

// ── Stage chain helper ───────────────────────────────────────────────

/// Parse a headerless CSV through CsvParseStage, tagging each output item
/// with the given stream name.
async fn parse_csv(
    csv_content: &str,
    file_id: &str,
    stream: &str,
    params: &serde_json::Value,
) -> Vec<PipelineItem> {
    let stage = CsvParseStage::from_params(params).unwrap();
    let item = make_csv_item(file_id, csv_content, stream);
    let ctx = make_context(params.clone());
    let mut items = stage.process(item, &ctx).await.unwrap();
    // Ensure all parsed items carry the stream tag
    for item in &mut items {
        item.stream = Some(stream.to_string());
    }
    items
}

/// Apply FieldMapStage to each item in the list.
async fn apply_field_map(
    items: Vec<PipelineItem>,
    params: &serde_json::Value,
) -> Vec<PipelineItem> {
    let stage = FieldMapStage::from_params(params).unwrap();
    let ctx = make_context(params.clone());
    let mut results = Vec::new();
    for item in items {
        let mapped = stage.process(item, &ctx).await.unwrap();
        results.extend(mapped);
    }
    results
}

/// Apply LookupStage to each item in the list.
async fn apply_lookup(items: Vec<PipelineItem>, params: &serde_json::Value) -> Vec<PipelineItem> {
    let stage = LookupStage::from_params(params).unwrap();
    let ctx = make_context(params.clone());
    let mut results = Vec::new();
    for item in items {
        let looked = stage.process(item, &ctx).await.unwrap();
        results.extend(looked);
    }
    results
}

/// Run the full Giant Eagle pipeline and return the assembled receipts.
async fn run_full_pipeline() -> Vec<PipelineItem> {
    // Step 1: Parse all CSV files
    let stores = parse_csv(STORES_CSV, "stores.csv", "stores", &stores_csv_params()).await;
    let mut transactions = parse_csv(
        TRANSACTIONS_CSV,
        "transactions.csv",
        "transactions",
        &transactions_csv_params(),
    )
    .await;
    let mut items = parse_csv(ITEMS_CSV, "items.csv", "items", &items_csv_params()).await;
    let products = parse_csv(
        PRODUCTS_CSV,
        "products.csv",
        "products",
        &products_csv_params(),
    )
    .await;
    let mut tenders = parse_csv(TENDERS_CSV, "tenders.csv", "tenders", &tenders_csv_params()).await;

    assert_eq!(stores.len(), 3, "should parse 3 stores");
    assert_eq!(transactions.len(), 5, "should parse 5 transactions");
    assert_eq!(items.len(), 10, "should parse 10 items");
    assert_eq!(products.len(), 8, "should parse 8 products");
    assert_eq!(tenders.len(), 7, "should parse 7 tenders");

    // Step 2: Field map transactions (rename txn_date, parse date)
    transactions = apply_field_map(transactions, &transactions_field_map_params()).await;

    // Step 3: Field map tenders (rename amount -> tender_amount)
    tenders = apply_field_map(tenders, &tenders_field_map_params()).await;

    // Step 4: Lookup tender types
    tenders = apply_lookup(tenders, &tender_type_lookup_params()).await;

    // Step 5: Lookup card schemes
    tenders = apply_lookup(tenders, &card_scheme_lookup_params()).await;

    // Step 6: Join items with products by UPC (left join)
    let join_stage = JoinStage::from_params(&items_products_join_params()).unwrap();
    let ctx = make_context(json!(null));
    let mut join_input: Vec<PipelineItem> = Vec::new();
    join_input.extend(items);
    join_input.extend(products);
    items = join_stage.process_batch(join_input, &ctx).await.unwrap();

    // Step 7: Aggregate tenders by txn_id
    let agg_stage = AggregateStage::from_params(&tenders_aggregate_params()).unwrap();
    let agg_tenders = agg_stage.process_batch(tenders, &ctx).await.unwrap();

    // Step 8: Assemble receipts
    // Re-tag aggregated tenders as "tenders" stream for assemble
    let mut assemble_input: Vec<PipelineItem> = Vec::new();
    assemble_input.extend(transactions);
    assemble_input.extend(stores);
    assemble_input.extend(items);
    for mut t in agg_tenders {
        t.stream = Some("tenders".to_string());
        assemble_input.push(t);
    }

    let assemble_stage = AssembleStage::from_params(&receipt_assemble_params()).unwrap();
    assemble_stage
        .process_batch(assemble_input, &ctx)
        .await
        .unwrap()
}

// ── Tests ────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_ge_e2e_full_pipeline() {
    let receipts = run_full_pipeline().await;

    // 5 transactions → 5 receipts
    assert_eq!(receipts.len(), 5, "should produce 5 receipts");

    // Every receipt should have store, line_items, and payments
    for receipt in &receipts {
        let rec = receipt.record.as_ref().unwrap();
        assert!(
            rec.contains_key("store"),
            "receipt {} missing 'store'",
            receipt.id
        );
        assert!(
            rec.contains_key("line_items"),
            "receipt {} missing 'line_items'",
            receipt.id
        );
        assert!(
            rec.contains_key("payments"),
            "receipt {} missing 'payments'",
            receipt.id
        );
    }

    // Verify T001: 2 line items (Organic Milk + Wheat Bread), 2 payments (split)
    let t001 = receipts
        .iter()
        .find(|r| {
            r.record
                .as_ref()
                .unwrap()
                .get("txn_id")
                .and_then(|v| v.as_str())
                == Some("T001")
        })
        .expect("should find T001 receipt");
    let t001_rec = t001.record.as_ref().unwrap();
    let t001_items = t001_rec["line_items"].as_array().unwrap();
    assert_eq!(t001_items.len(), 2, "T001 should have 2 line items");
    let t001_payments = t001_rec["payments"].as_array().unwrap();
    assert_eq!(
        t001_payments.len(),
        1,
        "T001 should have 1 aggregated payment record"
    );
    // The aggregated tender record should show tender_count=2 (split payment)
    let t001_pay = &t001_payments[0];
    assert_eq!(
        t001_pay.get("tender_count").and_then(|v| v.as_i64()),
        Some(2),
        "T001 aggregated tenders should have count=2"
    );

    // Verify T003 has item with UPC 999999999999 that didn't match products (left join)
    let t003 = receipts
        .iter()
        .find(|r| {
            r.record
                .as_ref()
                .unwrap()
                .get("txn_id")
                .and_then(|v| v.as_str())
                == Some("T003")
        })
        .expect("should find T003 receipt");
    let t003_items = t003.record.as_ref().unwrap()["line_items"]
        .as_array()
        .unwrap();
    assert_eq!(t003_items.len(), 3, "T003 should have 3 line items");
    let unknown_item = t003_items
        .iter()
        .find(|i| i.get("upc").and_then(|v| v.as_str()) == Some("999999999999"))
        .expect("T003 should have the unknown UPC item");
    // Left join: no product_* fields should be present (item kept as-is)
    assert!(
        unknown_item.get("product_product_name").is_none()
            || unknown_item.get("product_product_name").unwrap().is_null(),
        "Unknown UPC item should not have product_product_name, got: {:?}",
        unknown_item.get("product_product_name")
    );

    // Verify store nesting for T001 (store_id=0101)
    let store = t001_rec["store"].as_object().unwrap();
    assert_eq!(
        store.get("name").and_then(|v| v.as_str()),
        Some("Giant Eagle #0101"),
        "T001 should be at Giant Eagle #0101"
    );
    assert_eq!(store.get("zip").and_then(|v| v.as_str()), Some("15213"),);
}

#[tokio::test]
async fn test_ge_e2e_left_join_unmatched_products() {
    // Parse items and products, then left join
    let items = parse_csv(ITEMS_CSV, "items.csv", "items", &items_csv_params()).await;
    let products = parse_csv(
        PRODUCTS_CSV,
        "products.csv",
        "products",
        &products_csv_params(),
    )
    .await;

    let join_stage = JoinStage::from_params(&items_products_join_params()).unwrap();
    let ctx = make_context(json!(null));

    let mut join_input: Vec<PipelineItem> = Vec::new();
    join_input.extend(items);
    join_input.extend(products);
    let joined = join_stage.process_batch(join_input, &ctx).await.unwrap();

    // All 10 items should remain (left join keeps unmatched left items)
    assert_eq!(joined.len(), 10, "left join should keep all 10 items");

    // Find the item with UPC 999999999999 (unknown product)
    let unknown = joined
        .iter()
        .find(|i| {
            i.record
                .as_ref()
                .unwrap()
                .get("upc")
                .and_then(|v| v.as_str())
                == Some("999999999999")
        })
        .expect("should find unknown UPC item after left join");

    let rec = unknown.record.as_ref().unwrap();
    // No product_* fields added since there was no match
    assert!(
        !rec.contains_key("product_product_name"),
        "unmatched item should not have product_product_name"
    );
    assert!(
        !rec.contains_key("product_category"),
        "unmatched item should not have product_category"
    );

    // Matched items should have product fields
    let matched = joined
        .iter()
        .find(|i| {
            i.record
                .as_ref()
                .unwrap()
                .get("upc")
                .and_then(|v| v.as_str())
                == Some("041196010459")
        })
        .expect("should find Organic Milk item");
    let mrec = matched.record.as_ref().unwrap();
    assert_eq!(
        mrec.get("product_product_name").and_then(|v| v.as_str()),
        Some("Organic Milk 1 Gallon")
    );
    assert_eq!(
        mrec.get("product_category").and_then(|v| v.as_str()),
        Some("Dairy")
    );
}

#[tokio::test]
async fn test_ge_e2e_split_payment_aggregation() {
    // Parse tenders, apply field map + lookups, then aggregate
    let mut tenders = parse_csv(TENDERS_CSV, "tenders.csv", "tenders", &tenders_csv_params()).await;
    assert_eq!(tenders.len(), 7);

    tenders = apply_field_map(tenders, &tenders_field_map_params()).await;
    tenders = apply_lookup(tenders, &tender_type_lookup_params()).await;
    tenders = apply_lookup(tenders, &card_scheme_lookup_params()).await;

    let agg_stage = AggregateStage::from_params(&tenders_aggregate_params()).unwrap();
    let ctx = make_context(json!(null));
    let agg = agg_stage.process_batch(tenders, &ctx).await.unwrap();

    // 5 unique txn_ids → 5 groups
    assert_eq!(agg.len(), 5, "should have 5 aggregated tender groups");

    // T001 has 2 tenders (Credit Card 50.00 + Gift Card 37.32)
    let t001 = agg
        .iter()
        .find(|i| {
            i.record
                .as_ref()
                .unwrap()
                .get("txn_id")
                .and_then(|v| v.as_str())
                == Some("T001")
        })
        .expect("should find T001");
    let t001_rec = t001.record.as_ref().unwrap();
    assert_eq!(
        t001_rec["tender_count"],
        json!(2),
        "T001 should have 2 tenders"
    );
    // 50.00 + 37.32 = 87.32
    let total = t001_rec["total_tendered"].as_f64().unwrap();
    assert!(
        (total - 87.32).abs() < 0.01,
        "T001 total should be ~87.32, got {total}"
    );

    // T005 has 2 tenders (Debit Card 10.00 + Credit Card 5.99)
    let t005 = agg
        .iter()
        .find(|i| {
            i.record
                .as_ref()
                .unwrap()
                .get("txn_id")
                .and_then(|v| v.as_str())
                == Some("T005")
        })
        .expect("should find T005");
    let t005_rec = t005.record.as_ref().unwrap();
    assert_eq!(
        t005_rec["tender_count"],
        json!(2),
        "T005 should have 2 tenders"
    );
    let total5 = t005_rec["total_tendered"].as_f64().unwrap();
    assert!(
        (total5 - 15.99).abs() < 0.01,
        "T005 total should be ~15.99, got {total5}"
    );

    // T002 has only 1 tender
    let t002 = agg
        .iter()
        .find(|i| {
            i.record
                .as_ref()
                .unwrap()
                .get("txn_id")
                .and_then(|v| v.as_str())
                == Some("T002")
        })
        .expect("should find T002");
    assert_eq!(t002.record.as_ref().unwrap()["tender_count"], json!(1));

    // Verify payment_details array for T001
    let details = t001_rec["payment_details"].as_array().unwrap();
    assert_eq!(details.len(), 2);
    let methods: Vec<&str> = details
        .iter()
        .filter_map(|d| d.get("payment_method").and_then(|v| v.as_str()))
        .collect();
    assert!(
        methods.contains(&"Credit Card"),
        "T001 should have Credit Card payment"
    );
    assert!(
        methods.contains(&"Gift Card"),
        "T001 should have Gift Card payment"
    );
}

#[tokio::test]
async fn test_ge_e2e_stream_isolation() {
    // Verify that stream tagging keeps data separated.
    let stores = parse_csv(STORES_CSV, "stores.csv", "stores", &stores_csv_params()).await;
    let items = parse_csv(ITEMS_CSV, "items.csv", "items", &items_csv_params()).await;
    let transactions = parse_csv(
        TRANSACTIONS_CSV,
        "transactions.csv",
        "transactions",
        &transactions_csv_params(),
    )
    .await;

    // All stores should have stream = "stores"
    for store in &stores {
        assert_eq!(
            store.stream.as_deref(),
            Some("stores"),
            "store item {} should have stream=stores",
            store.id
        );
    }

    // All items should have stream = "items"
    for item in &items {
        assert_eq!(
            item.stream.as_deref(),
            Some("items"),
            "item {} should have stream=items",
            item.id
        );
    }

    // All transactions should have stream = "transactions"
    for txn in &transactions {
        assert_eq!(
            txn.stream.as_deref(),
            Some("transactions"),
            "transaction {} should have stream=transactions",
            txn.id
        );
    }

    // When we assemble, only items matching the correct stream are used.
    // Create an assemble with stores + transactions, but NO items stream.
    let assemble_params = json!({
        "primary_stream": "transactions",
        "primary_key": "txn_id",
        "joins": [
            {
                "stream": "stores",
                "key": "store_id",
                "foreign_key": "store_id",
                "nest_as": "store",
                "collect": false
            },
            {
                "stream": "items",
                "key": "txn_id",
                "foreign_key": "txn_id",
                "nest_as": "line_items",
                "collect": true
            }
        ]
    });
    let assemble_stage = AssembleStage::from_params(&assemble_params).unwrap();
    let ctx = make_context(json!(null));

    // Only pass transactions and stores (no items)
    let mut input: Vec<PipelineItem> = Vec::new();
    input.extend(transactions);
    input.extend(stores);

    let receipts = assemble_stage.process_batch(input, &ctx).await.unwrap();
    assert_eq!(receipts.len(), 5);

    // All receipts should have empty line_items since no items were in the batch
    for receipt in &receipts {
        let rec = receipt.record.as_ref().unwrap();
        let line_items = rec["line_items"].as_array().unwrap();
        assert!(
            line_items.is_empty(),
            "receipt {} should have empty line_items when items stream is absent, got {} items",
            receipt.id,
            line_items.len()
        );
        // But store should still be populated
        assert!(
            rec["store"].is_object(),
            "receipt {} should still have a store object",
            receipt.id
        );
    }
}
