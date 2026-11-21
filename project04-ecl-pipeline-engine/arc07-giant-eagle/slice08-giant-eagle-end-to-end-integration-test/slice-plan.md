# 9. Milestone 8.8: Giant Eagle End-to-End Integration Test

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1563-1610. Original bytes are recoverable from import commit 3a39aea. -->

## 9. Milestone 8.8: Giant Eagle End-to-End Integration Test

### 9.1 Test Data

Create sample CSVs at `crates/ecl-pipeline/tests/fixtures/giant_eagle/`:

- `stores.csv` — 3 stores (no headers)
- `transactions.csv` — 5 transactions across 3 stores
- `items.csv` — 10 items across the 5 transactions
- `products.csv` — 8 products (2 items won't match = left join)
- `tenders.csv` — 7 tenders (some transactions have split payment)

Create a `giant_eagle.zip` containing all 5 CSVs for decompression testing.

### 9.2 Test Pipeline Spec

Create `crates/ecl-pipeline/tests/fixtures/giant_eagle_pipeline.toml` using filesystem sources (substituting for GCS) with all Phase 2 stages.

### 9.3 Test Scenarios

#### File: `crates/ecl-pipeline/tests/integration/giant_eagle_e2e.rs` (new)

1. **`test_ge_e2e_full_pipeline`**
   - Parse 5 CSV files into 5 streams
   - Map fields per stream
   - Lookup payment types and schemes
   - Join items with products by UPC
   - Aggregate tenders by transaction_id
   - Assemble receipts with nested store, items, payment
   - Validate
   - Emit to output directory
   - Verify: 5 receipt files, each with correct nested structure

2. **`test_ge_e2e_left_join_unmatched_products`**
   - Items with unknown UPCs still appear in receipts (null product fields)

3. **`test_ge_e2e_split_payment_aggregation`**
   - Transaction with 3 tenders → single payment object with payments array

4. **`test_ge_e2e_stream_isolation`**
   - Verify that store records never appear in item stages and vice versa

5. **`test_ge_e2e_decompression_to_receipt`**
   - Start from ZIP file → decompress → full pipeline
   - Verify same output as test 1

---

