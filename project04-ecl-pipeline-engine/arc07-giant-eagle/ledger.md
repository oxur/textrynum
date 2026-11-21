# Historical criteria — arc07-giant-eagle

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1716-1741; full original recoverable from import commit 2d006a7. -->

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

