# 9. Milestone 9.8: Acme Inc. End-to-End Integration Test

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1361-1437. Original bytes are recoverable from import commit 2d006a7. -->

## 9. Milestone 9.8: Acme Inc. End-to-End Integration Test

### 9.1 Test Data

Create fixture directory `crates/ecl-pipeline/tests/fixtures/acme-inc/`:

Generate PGP-encrypted test CSVs:

- `wag_banyan_transaction_file_20260315143022.csv.pgp` — 5 transactions
- `wag_banyan_payment_file_20260315143022.csv.pgp` — 8 tenders (some split)
- `wag_banyan_item_file_20260315143022.csv.pgp` — 12 items
- `wag_banyan_store_file_20260315143022.csv.pgp` — 3 stores
- `wag_banyan_product_file_20260315143022.csv.pgp` — 10 products

Plus a duplicate transaction file (same content, different timestamp):

- `wag_banyan_transaction_file_20260315153022.csv.pgp` — duplicate for dedup testing

Test data includes:

- Digital transaction (prices in cents, needs /100)
- BOPIS transaction (bopisFlag = 'Y')
- Transaction with online store 5995 (UTC timezone)
- Item with hot-key UPC 999999999999
- Split payment tender (2 payment methods on one transaction)
- Debit Card/Cashback negative amount

### 9.2 Test Pipeline Specs

Create two specs:

- `acme_inc_ingestion.toml` — filesystem source → GCS sink (mocked)
- `acme_inc_transformation.toml` — filesystem source (pre-decrypted for speed), full pipeline

### 9.3 Test Scenarios

#### File: `crates/ecl-pipeline/tests/integration/acme_inc_e2e.rs` (new)

1. **`test_wag_e2e_pgp_decrypt`**
   - Encrypt test CSV with test PGP key
   - Run decrypt stage
   - Verify decrypted content matches original

2. **`test_wag_e2e_deduplication`**
   - 2 transaction files with same content, different timestamps
   - After dedup: only 1 file remains (newest)

3. **`test_wag_e2e_full_transformation`**
   - Parse 5 CSV types into 5 streams
   - Map fields per stream (price correction, payment mapping, UPC check digit, timezone)
   - Join items with products
   - Aggregate tenders by transaction
   - Assemble receipts
   - Validate
   - Emit to output directory

4. **`test_wag_e2e_digital_price_correction`**
   - Digital file amounts divided by 100

5. **`test_wag_e2e_bopis_detection`**
   - bopisFlag='Y' → order_type='web', fulfillment_type='in_store'

6. **`test_wag_e2e_payment_type_mapping`**
   - Credit Card→CREDIT, Visa→VI, etc.

7. **`test_wag_e2e_pipeline_chaining`**
   - Ingestion pipeline triggers transformation pipeline
   - Both complete successfully

8. **`test_wag_e2e_lifecycle_on_success`**
   - Verify files moved from staging to historical

9. **`test_wag_e2e_lifecycle_on_failure`**
   - Inject failure, verify files moved back to input

---

