# Stage 2.7: Toy Workflow Integration — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0009-phase-2-07-integration.md, lines 1625-1669. Original bytes are recoverable from import commit 2d006a7. -->

## Acceptance Criteria

### Functionality
- [ ] Complete Critique-Revise workflow executes end-to-end
- [ ] Happy path works (generate → critique → pass)
- [ ] Revision path works (generate → critique → revise → critique → pass)
- [ ] Max revisions enforced (fails after 3 revisions)
- [ ] Recovery works (can resume after crash/pause)

### CLI
- [ ] `ecl run critique-revise --topic "X"` starts workflow
- [ ] `ecl status <workflow-id>` shows current state
- [ ] `ecl list` shows available workflows
- [ ] Clear, actionable error messages
- [ ] Progress indicators for long-running operations

### Integration
- [ ] Restate service correctly manages workflow state
- [ ] Database records created for workflows and steps
- [ ] Observability logs capture all execution details
- [ ] Error handling propagates through all layers

### Testing
- [ ] All integration tests pass
- [ ] Test coverage ≥95%
- [ ] E2E tests work with Restate runtime
- [ ] Mock tests run without external dependencies
- [ ] Property tests verify invariants

### Code Quality
- [ ] No `unwrap()` in library code
- [ ] All public APIs have rustdoc
- [ ] Follows Rust anti-patterns guide
- [ ] No compiler warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` check passes

### Documentation
- [ ] README includes usage examples
- [ ] CLI help text is clear
- [ ] API documentation is complete
- [ ] Examples compile and run

---

