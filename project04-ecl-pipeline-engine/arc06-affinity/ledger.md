# Historical criteria — arc06-affinity

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1385-1410; full original recoverable from import commit 2d006a7. -->

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

