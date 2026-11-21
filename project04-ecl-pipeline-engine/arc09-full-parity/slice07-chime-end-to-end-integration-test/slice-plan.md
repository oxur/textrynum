# 8. Milestone 10.7: Chime End-to-End Integration Test

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 1122-1158. Original bytes are recoverable from import commit 2d006a7. -->

## 8. Milestone 10.7: Chime End-to-End Integration Test

### 8.1 Test Data

- `chime_bins.csv` — 5 BIN numbers
- Mock BigQuery result as JSON lines file (simulating query output):
  - 10 payment records matching Chime BINs at Acme Inc.
  - Include private label and non-private label items
  - Include a split-payment transaction

### 8.2 Test Pipeline Spec

`crates/ecl-pipeline/tests/fixtures/chime_pipeline.toml`:
- Filesystem source for BIN CSV (stand-in for GCS)
- Filesystem source for mock BQ result (stand-in for BigQuery)
- CSV parse for BINs
- JSON lines parse for BQ data
- Expression stage for reward calculation
- CSV emit for output (stand-in for S3)

### 8.3 Test Scenarios

1. **`test_chime_e2e_reward_calculation`**
   - Parse BINs and mock BQ data
   - Calculate rewards: 5% private label, 3% non-private, $5 cap
   - Verify output CSV has correct reward amounts

2. **`test_chime_e2e_cap_at_five_dollars`**
   - Receipt where raw reward exceeds $5
   - Verify capped at $5

3. **`test_chime_e2e_split_payment_proportional`**
   - Two payments on one receipt
   - Reward distributed proportionally

---

