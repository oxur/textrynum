# Historical criteria — arc08-acme-inc

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1545-1569; full original recoverable from import commit 2d006a7. -->

## 11. Verification Checklist

### Per-Milestone

- [ ] `make test` passes
- [ ] `make lint` passes
- [ ] `make format` passes
- [ ] No compiler warnings
- [ ] Coverage ≥ 95% on new code
- [ ] Checked against AP-01 through AP-20

### Phase 3 Complete

- [ ] Secret resolver resolves from env, file, and GCP Secret Manager
- [ ] SFTP adapter lists and fetches files from SFTP server
- [ ] PGP decryption stage decrypts encrypted files
- [ ] Deduplication stage removes duplicate files by content hash
- [ ] S3 sink writes files to AWS S3
- [ ] Pipeline chaining triggers child pipelines on completion
- [ ] Scheduler runs pipelines on cron schedule
- [ ] Acme Inc. E2E integration test passes (decrypt → dedup → transform → output)
- [ ] Phase 1 Affinity E2E still passes
- [ ] Phase 2 Giant Eagle E2E still passes
- [ ] All pre-Phase-1 tests still pass

