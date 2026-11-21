# Historical criteria — arc09-full-parity

> These are original criteria or verification instructions, reorganized on 2026-11-21. They are not current acceptance evidence. Original checkbox states are preserved; no new statuses, sign-offs or successful verification results have been assigned.


<!-- Historical criteria excerpt: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 1246-1273; full original recoverable from import commit 3a39aea. -->

## 11. Verification Checklist

### Phase 4 Complete

- [ ] Expression evaluation works (arithmetic, conditionals, functions)
- [ ] UPC check digit correctly computes GS1 algorithm
- [ ] Auth code normalization strips/pads correctly
- [ ] BigQuery adapter executes queries and returns results
- [ ] JSON lines parse fans out query results into records
- [ ] Environment overlays deep-merge specs correctly
- [ ] Stateful diff detects inserts, updates, deletes across runs
- [ ] Chime E2E: BINs + BQ data → reward calculation → CSV output
- [ ] Acme Inc. loyalty E2E: CSV → diff → Kafka (insert/update/delete)
- [ ] All Phase 1/2/3 tests still pass

### Full Project Complete

- [ ] **Affinity** (Phase 1): GCS CSV → parse → map → validate → Kafka
- [ ] **Giant Eagle** (Phase 2): 5 CSVs → multi-stream → join → aggregate → assemble → Kafka
- [ ] **Acme Inc. Receipt** (Phase 3): SFTP → PGP decrypt → dedup → transform → Kafka
- [ ] **Chime** (Phase 4): BINs + BigQuery → reward calculation → CSV/S3
- [ ] **Acme Inc. Loyalty** (Phase 4): CSV → stateful diff → Kafka
- [ ] **Environment overlays**: One spec, three deployments
- [ ] **Scheduling**: Cron-based pipeline execution
- [ ] **Pipeline chaining**: Ingestion triggers transformation
- [ ] **Secret management**: GCP Secret Manager integration
- [ ] **Zero customer-specific Rust code**: All pipelines are TOML configurations

