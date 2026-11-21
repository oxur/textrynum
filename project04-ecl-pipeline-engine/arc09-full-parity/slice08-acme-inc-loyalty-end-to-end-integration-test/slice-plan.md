# 9. Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 1159-1186. Original bytes are recoverable from import commit 3a39aea. -->

## 9. Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test

### 9.1 Test Data

- `wag_banyan_loyalty_file_20260315.csv` — 20 loyalty records:
  - 5 new (not in state)
  - 5 unchanged (same CAT)
  - 5 updated (different CAT)
  - 5 with NULL CAT (triggers delete if existing)

### 9.2 Test Scenarios

1. **`test_loyalty_e2e_first_run`**
   - Empty state → all non-null records are inserts
   - Verify Kafka messages (insert action)

2. **`test_loyalty_e2e_second_run_with_diffs`**
   - Pre-populate state from run 1
   - Run with mixed data
   - Verify: 5 inserts, 5 updates, 5 deletes, 5 unchanged (not emitted)

3. **`test_loyalty_e2e_state_persists`**
   - Run 1 populates state
   - Run 2 sees changes
   - Verify state store has correct values after both runs

---

