# Banyan Phase 2 — Make Giant Eagle work

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1-211. Original bytes are recoverable from import commit 3a39aea. -->

# Phase 2 Detailed Implementation Plan: "Make Giant Eagle Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1 Plan)
**Goal:** Run the Giant Eagle (Partner 296) pipeline end-to-end in ECL — multiple CSV file types parsed independently, joined by key, aggregated, assembled into receipts, validated, and output to Kafka.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 8.1: Named Data Streams](#2-milestone-81-named-data-streams)
3. [Milestone 8.2: Batch Stage Trait & Join Stage](#3-milestone-82-batch-stage-trait--join-stage)
4. [Milestone 8.3: Aggregation Stage](#4-milestone-83-aggregation-stage)
5. [Milestone 8.4: Lookup Table Stage](#5-milestone-84-lookup-table-stage)
6. [Milestone 8.5: Date/Time & Timezone Stages](#6-milestone-85-datetime--timezone-stages)
7. [Milestone 8.6: ZIP Decompression Stage](#7-milestone-86-zip-decompression-stage)
8. [Milestone 8.7: Receipt Assembly Stage](#8-milestone-87-receipt-assembly-stage)
9. [Milestone 8.8: Giant Eagle End-to-End Integration Test](#9-milestone-88-giant-eagle-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Target Pipeline Shape (Giant Eagle)

```
GCS (stores.csv)       ─→ decompress ─→ parse ─→ map ─→ tz_convert ─────────────────────────┐
GCS (transactions.csv) ─→ decompress ─→ parse ─→ map ─→ date_parse ─────────────────────────┤
GCS (items.csv)        ─→ decompress ─→ parse ─→ map ───────┐                               │
GCS (products.csv)     ─→ decompress ─→ parse ─→ map ───────┤                               │
                                                              ├─→ join(items+products by UPC) │
                                                              │                               │
GCS (tenders.csv)      ─→ decompress ─→ parse ─→ map ─→ lookup(payment types) ──────────────┤
                                                              │                               │
                                                              ├─→ aggregate(tenders by txn)   │
                                                              │                               │
                                                              └───────────────────────────────┤
                                                                                              │
                                    assemble_receipt ←────────────────────────────────────────┘
                                         │
                                         ├─→ validate ─→ kafka_sink (receipts)
                                         └─→ taxonomy_extract ─→ kafka_sink (taxonomy)
```

### 1.2 TOML Spec Preview (Giant Eagle, abbreviated)

```toml
name = "giant-eagle-296-transformation"
version = 1
output_dir = "./output/giant-eagle-296"

# ── Sources (one per file type) ─────────────────────────
[sources.stores]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*store*.csv"
stream = "stores"

[sources.transactions]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*transaction*.csv"
stream = "transactions"

[sources.items]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*item*.csv"
stream = "items"

[sources.products]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*product*.csv"
stream = "products"

[sources.tenders]
kind = "gcs"
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*tender*.csv"
stream = "tenders"

# ── Per-stream parse stages ─────────────────────────────
[stages.parse-stores]
adapter = "csv_parse"
input_streams = ["stores"]
output_stream = "parsed-stores"
resources = { reads = ["stores-files"], creates = ["parsed-stores"] }
[stages.parse-stores.params]
has_headers = false
columns = [
  { name = "merchant_name", type = "string" },
  { name = "store_id", type = "string" },
  # ... 12 columns
]

[stages.parse-transactions]
adapter = "csv_parse"
input_streams = ["transactions"]
output_stream = "parsed-transactions"
resources = { reads = ["txn-files"], creates = ["parsed-transactions"] }
[stages.parse-transactions.params]
has_headers = true
columns = [
  { name = "TRANSACTION_ID", type = "string" },
  # ... 12 columns
]

# ... similar for items, products, tenders

# ── Lookup stage (payment types) ────────────────────────
[stages.map-payment-types]
adapter = "lookup"
input_streams = ["mapped-tenders"]
output_stream = "typed-tenders"
resources = { reads = ["mapped-tenders"], creates = ["typed-tenders"] }
[stages.map-payment-types.params]
lookups = [
  { field = "payment_type", output = "canonical_payment_type", default = "OTHER",
    table = { "Credit" = "CREDIT", "Debit" = "DEBIT", "Cash" = "CASH", "EBT" = "EBT" } },
  { field = "payment_scheme", output = "canonical_scheme",
    table = { "Visa" = "VI", "V" = "VI", "Mastercard" = "MC", "M" = "MC",
              "Discover" = "DS", "D" = "DS", "Amex" = "AX" } },
]

# ── Join stage (items + products by UPC) ────────────────
[stages.join-items-products]
adapter = "join"
input_streams = ["mapped-items", "mapped-products"]
output_stream = "enriched-items"
resources = { reads = ["mapped-items", "mapped-products"], creates = ["enriched-items"] }
[stages.join-items-products.params]
join_type = "left"
left_stream = "mapped-items"
right_stream = "mapped-products"
left_key = "upc"
right_key = "upc"

# ── Aggregation stage (tenders by transaction) ──────────
[stages.aggregate-tenders]
adapter = "aggregate"
input_streams = ["typed-tenders"]
output_stream = "aggregated-tenders"
resources = { reads = ["typed-tenders"], creates = ["aggregated-tenders"] }
[stages.aggregate-tenders.params]
group_by = ["transaction_id"]
aggregates = [
  { field = "payment_amt", function = "sum", output = "total_charged" },
  { field = "tax_amount", function = "max", output = "tax_amount" },
  { field = "shipping_amount", function = "max", output = "shipping_amount" },
]
collect_arrays = [
  { output = "payments", fields = ["payment_type", "scheme", "card_last_four", "bin", "auth_code", "arn", "payment_amt"] },
]

# ── Assembly stage ──────────────────────────────────────
[stages.assemble]
adapter = "assemble"
input_streams = ["parsed-stores", "parsed-transactions", "enriched-items", "aggregated-tenders"]
output_stream = "receipts"
resources = { reads = ["parsed-stores", "parsed-transactions", "enriched-items", "aggregated-tenders"],
              creates = ["receipts"] }
[stages.assemble.params]
primary_stream = "parsed-transactions"
primary_key = "transaction_id"
joins = [
  { stream = "parsed-stores", key = "store_id", foreign_key = "store_id", nest_as = "store" },
  { stream = "enriched-items", key = "transaction_id", foreign_key = "transaction_id", nest_as = "items", collect = true },
  { stream = "aggregated-tenders", key = "transaction_id", foreign_key = "transaction_id", nest_as = "payment" },
]

# ── Validate + Output ──────────────────────────────────
[stages.validate]
adapter = "validate"
input_streams = ["receipts"]
output_stream = "validated"
resources = { reads = ["receipts"], creates = ["validated"] }

[stages.kafka-receipts]
adapter = "kafka_sink"
input_streams = ["validated"]
resources = { reads = ["validated"] }
[stages.kafka-receipts.params]
topic = "by_production_296_giant-eagle-inc_receipt-accept_canonical_batch_avro_0"
filter = "valid_only"
# ... kafka config
```

### 1.3 Key Design Decisions

**D1: Stream as a tag on PipelineItem, not a separate collection.** Items carry a `stream: Option<String>` field. Sources tag items at creation. Stages declare `input_streams` and `output_stream` to control routing. The runner filters items by stream when collecting for a stage.

**D2: Batch stages via `process_batch` on Stage trait.** Add a default method `process_batch(items: Vec<PipelineItem>, ctx: &StageContext) -> Result<Vec<PipelineItem>>` that calls `process` per item. Join and Aggregate stages override this to process all items at once. A `requires_batch() -> bool` flag tells the runner which path to take.

**D3: Stage output items get the stage's `output_stream` tag.** When a stage produces output items, the runner tags them with the stage's configured `output_stream`. This keeps stream routing out of individual stage implementations.

**D4: Items accumulate across batches.** After a batch executes, its output items are added to the item pool. The next batch's stages see both the original source items and any newly produced items, filtered by their `input_streams`. Items that have been consumed (Completed status) are still visible to later stages if they share the same stream — but the `collect_items_for_stage` method checks both stream AND Pending status.

**D5: Fan-out stages (CSV parse) produce items in the output_stream.** A csv_parse stage reading from stream "transactions" produces items in stream "parsed-transactions". This makes stream routing explicit in the TOML spec.

---


Historical milestone moved to [2. Milestone 8.1: Named Data Streams](slice01-named-data-streams/slice-plan.md).

Historical milestone moved to [3. Milestone 8.2: Batch Stage Trait & Join Stage](slice02-batch-stage-trait-join-stage/slice-plan.md).

Historical milestone moved to [4. Milestone 8.3: Aggregation Stage](slice03-aggregation-stage/slice-plan.md).

Historical milestone moved to [5. Milestone 8.4: Lookup Table Stage](slice04-lookup-table-stage/slice-plan.md).

Historical milestone moved to [6. Milestone 8.5: Date/Time & Timezone Stages](slice05-date-time-timezone-stages/slice-plan.md).

Historical milestone moved to [7. Milestone 8.6: ZIP Decompression Stage](slice06-zip-decompression-stage/slice-plan.md).

Historical milestone moved to [8. Milestone 8.7: Receipt Assembly Stage](slice07-receipt-assembly-stage/slice-plan.md).

Historical milestone moved to [9. Milestone 8.8: Giant Eagle End-to-End Integration Test](slice08-giant-eagle-end-to-end-integration-test/slice-plan.md).

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1611-1828. Original bytes are recoverable from import commit 3a39aea. -->

## 10. Cross-Cutting Concerns

### 10.1 StageError: Add Clone Derive

`StageError` needs `Clone` for the batch execution model (errors shared across item IDs):

```rust
#[derive(Debug, Clone, thiserror::Error)]  // Add Clone
pub enum StageError { /* ... */ }
```

All fields are `String` and `u64`, so this is trivially derivable.

### 10.2 New Dependencies

**Root `Cargo.toml` workspace deps:**

```toml
zip = "2"
flate2 = "1"
chrono-tz = "0.10"
```

**`ecl-stages/Cargo.toml` additions:**

```toml
zip = { workspace = true }
flate2 = { workspace = true }
chrono-tz = { workspace = true }
```

### 10.3 Registry Updates

#### File: `crates/ecl-cli/src/pipeline/registry.rs`

Add to `stage_lookup_fn`:

```rust
"join" => {
    let stage = ecl_stages::JoinStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"aggregate" => {
    let stage = ecl_stages::AggregateStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"lookup" => {
    let stage = ecl_stages::LookupStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"date_parse" => {
    let stage = ecl_stages::DateParseStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"timezone" => {
    let stage = ecl_stages::TimezoneStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"decompress" => {
    let stage = ecl_stages::DecompressStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
"assemble" => {
    let stage = ecl_stages::AssembleStage::from_params(&spec.params)?;
    Ok(Arc::new(stage) as Arc<dyn Stage>)
}
```

### 10.4 Module Wiring in ecl-stages

```rust
// In crates/ecl-stages/src/lib.rs:
pub mod join;
pub mod aggregate;
pub mod lookup;
pub mod date_parse;
pub mod timezone;
pub mod decompress;
pub mod assemble;

pub use join::JoinStage;
pub use aggregate::AggregateStage;
pub use lookup::LookupStage;
pub use date_parse::DateParseStage;
pub use timezone::TimezoneStage;
pub use decompress::DecompressStage;
pub use assemble::AssembleStage;
```

### 10.5 Backward Compatibility

All changes are additive:

- `PipelineItem` gains `stream: Option<String>` with `#[serde(default)]`
- `StageSpec` gains `input_streams: Vec<String>` and `output_stream: Option<String>` with `#[serde(default)]`
- `Stage` trait gains `requires_batch()` and `process_batch()` with defaults
- Source specs gain `stream: Option<String>` with `#[serde(default)]`
- `PipelineRunner` gains `active_items: Vec<PipelineItem>` (internal)

**Phase 1 pipelines (no streams) continue to work unchanged.** Empty `input_streams` = all items visible. No `output_stream` = items retain their tags.

**Existing tests must pass.** The `collect_items_for_stage` change is backward compatible because empty `input_streams` accepts all items.

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

### Phase 2 Complete

- [ ] Named streams route items correctly
- [ ] Join stage merges two streams by key (inner, left, full)
- [ ] Aggregate stage groups and computes (sum, max, min, count, collect)
- [ ] Lookup stage maps values from static tables
- [ ] Date parse converts string → RFC3339
- [ ] Timezone converts local → UTC by ZIP code
- [ ] Decompress extracts ZIP archives with fan-out
- [ ] Assemble merges multiple streams into nested receipts
- [ ] Giant Eagle E2E integration test passes
- [ ] Phase 1 Affinity E2E test still passes (no regressions)
- [ ] Existing pre-Phase-1 tests still pass

---

## Appendix A: Milestone Dependency Graph

```
8.1 Named Streams ───────────────────────────────────┐
    │                                                 │
    ├── 8.2 Batch Trait + Join ──────────────┐       │
    │                                         │       │
    ├── 8.3 Aggregation ─────────────────────┤       │
    │                                         │       │
    ├── 8.4 Lookup ──────────────────────────┤       │
    │                                         │       │
    ├── 8.5 Date/Time + Timezone ────────────┤       │
    │                                         │       │
    ├── 8.6 ZIP Decompression ───────────────┤       │
    │                                         │       │
    └─────────────────────────────────────────┘       │
                              │                        │
                       8.7 Assembly ──────────────────┤
                              │                        │
                       8.8 Giant Eagle E2E ───────────┘
```

**Parallelizable work:**

- 8.2, 8.3, 8.4, 8.5, 8.6 can ALL proceed in parallel after 8.1
- 8.7 depends on 8.2 + 8.3 (uses batch mode + multi-stream assembly)
- 8.8 depends on all prior milestones

## Appendix B: Files Changed/Created Summary

### New Files

| File | Milestone |
|------|-----------|
| `crates/ecl-stages/src/join.rs` | 8.2 |
| `crates/ecl-stages/src/aggregate.rs` | 8.3 |
| `crates/ecl-stages/src/lookup.rs` | 8.4 |
| `crates/ecl-stages/src/date_parse.rs` | 8.5 |
| `crates/ecl-stages/src/timezone.rs` | 8.5 |
| `crates/ecl-stages/src/decompress.rs` | 8.6 |
| `crates/ecl-stages/src/assemble.rs` | 8.7 |
| `tests/fixtures/giant_eagle/*.csv` | 8.8 |
| `tests/fixtures/giant_eagle_pipeline.toml` | 8.8 |
| `tests/integration/giant_eagle_e2e.rs` | 8.8 |

### Modified Files

| File | Milestone | Change |
|------|-----------|--------|
| `crates/ecl-pipeline-topo/src/traits.rs` | 8.1, 8.2 | Add `stream` to PipelineItem, add `requires_batch`/`process_batch` to Stage |
| `crates/ecl-pipeline-topo/src/error.rs` | 8.2 | Add Clone to StageError |
| `crates/ecl-pipeline-spec/src/source.rs` | 8.1 | Add `stream` to all source specs |
| `crates/ecl-pipeline-spec/src/stage.rs` | 8.1 | Add `input_streams`, `output_stream` |
| `crates/ecl-pipeline/src/runner.rs` | 8.1 | Add `active_items`, stream-aware collection, output propagation |
| `crates/ecl-pipeline/src/batch.rs` | 8.2 | Add `execute_stage_batch` function |
| `crates/ecl-stages/src/lib.rs` | 8.2-8.7 | Module declarations + re-exports |
| `crates/ecl-stages/Cargo.toml` | 8.5, 8.6 | Add zip, flate2, chrono-tz deps |
| `crates/ecl-cli/src/pipeline/registry.rs` | 8.2-8.7 | Register 7 new stages |
| `Cargo.toml` (workspace) | 8.5, 8.6 | Add workspace deps |

## Appendix C: Runner State Model Evolution

### Before Phase 2 (Current)

```
Sources enumerate → ItemState (Pending) → Stage processes → ItemState (Completed)
                                           ↓
                                    No output propagation
```

### After Phase 2

```
Sources enumerate → active_items [stream tagged]
                         │
                    Batch 0: stages consume/produce active_items
                         │
                    Batch 1: stages see previous batch outputs
                         │
                    Batch N: terminal stages (sinks) consume, produce nothing
                         │
                    Final: active_items should be empty (all consumed)
```

This is the key architectural shift. Items are a flowing pool, not a fixed set.

## Reconstructed slice locations

- [2. Milestone 8.1: Named Data Streams](slice01-named-data-streams/slice-plan.md)
- [3. Milestone 8.2: Batch Stage Trait & Join Stage](slice02-batch-stage-trait-join-stage/slice-plan.md)
- [4. Milestone 8.3: Aggregation Stage](slice03-aggregation-stage/slice-plan.md)
- [5. Milestone 8.4: Lookup Table Stage](slice04-lookup-table-stage/slice-plan.md)
- [6. Milestone 8.5: Date/Time & Timezone Stages](slice05-date-time-timezone-stages/slice-plan.md)
- [7. Milestone 8.6: ZIP Decompression Stage](slice06-zip-decompression-stage/slice-plan.md)
- [8. Milestone 8.7: Receipt Assembly Stage](slice07-receipt-assembly-stage/slice-plan.md)
- [9. Milestone 8.8: Giant Eagle End-to-End Integration Test](slice08-giant-eagle-end-to-end-integration-test/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
