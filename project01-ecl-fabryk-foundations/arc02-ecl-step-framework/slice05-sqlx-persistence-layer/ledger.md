# Stage 2.5: SQLx Persistence Layer — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0007-phase-2-05-sqlx-persistence.md, lines 1422-1439. Original bytes are recoverable from import commit 3a39aea. -->

## Acceptance Criteria

- [ ] **Schema Created**: Migrations run successfully on SQLite and Postgres
- [ ] **Compile-Time Validation**: SQLx queries validated with `cargo sqlx prepare`
- [ ] **WorkflowRepository Complete**: All CRUD operations implemented and tested
- [ ] **StepExecutionRepository Complete**: History tracking works correctly
- [ ] **Error Handling**: All database errors use `DbError` enum
- [ ] **Foreign Keys Enabled**: CASCADE deletes work in tests
- [ ] **In-Memory Testing**: Tests use `connect_in_memory()` for fast execution
- [ ] **Unit Tests**: Repository methods covered ≥95%
- [ ] **Integration Tests**: Full workflow lifecycle tested
- [ ] **Property Tests**: State serialization roundtrip verified
- [ ] **Documentation**: All public API items have rustdoc
- [ ] **No Panics**: No `unwrap()` or `expect()` in repository code
- [ ] **Configuration**: Database URL configurable via config/env

---

