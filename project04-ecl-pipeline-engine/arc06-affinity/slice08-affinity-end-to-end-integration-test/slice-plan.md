# 9. Milestone 7.8: Affinity End-to-End Integration Test

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1257-1316. Original bytes are recoverable from import commit 3a39aea. -->

## 9. Milestone 7.8: Affinity End-to-End Integration Test

### 9.1 Scope

Full pipeline integration test that exercises the complete Affinity data flow.

### 9.2 Test Data

Create sample Affinity CSV at `crates/ecl-pipeline/tests/fixtures/affinity_sample.csv`:

```csv
Channel_Aggregator_ID,Program_ID,Account_ID,Card_BIN,Card_Last_Four,Account_Postal_Code,Merchant_Location_MID,Merchant_Location_Name,Merchant_Location_Street,Merchant_Location_City,Merchant_Location_State,Merchant_Location_Postal_Code,Merchant_Location_Category_Code,Transaction_ID,Transaction_Date,Transaction_Settlement_Date,Transaction_Code,Transaction_Amount,Transaction_Auth_Code
AGG001,PRG001,ACCT-001,411111,1234,90210,MID-001,ACME INC. #1234,123 Main St,Los Angeles,CA,90001,5912,TXN-001,03/15/2026,03/16/2026,PUR,25.99,123
AGG001,PRG001,ACCT-002,422222,5678,10001,MID-002,CVS PHARMACY,456 Oak Ave,New York,NY,10002,5912,TXN-002,03/15/2026,03/16/2026,PUR,12.50,45
AGG001,PRG001,ACCT-003,433333,,90210,MID-003,ACME INC. #5678,789 Pine Rd,Chicago,IL,60601,5912,,03/15/2026,03/16/2026,PUR,8.75,789012
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

