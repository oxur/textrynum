# Stage 2.4: Workflow Orchestrator — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0006-phase-2-04-workflow-orchestrator.md, lines 1410-1429. Original bytes are recoverable from import commit 3a39aea. -->

## Acceptance Criteria

- [ ] `WorkflowDefinition` can express multi-step workflows with dependencies
- [ ] `WorkflowDefinition::validate()` catches structural errors
- [ ] Topological sort correctly orders steps by dependencies
- [ ] Topological sort detects circular dependencies
- [ ] `WorkflowOrchestrator` executes steps in correct order
- [ ] Revision loops have bounded iteration with `max_iterations`
- [ ] `WorkflowResult` captures all step outputs and metadata
- [ ] `WorkflowResult` records success/failure status
- [ ] All unit tests pass with ≥95% coverage
- [ ] Integration tests demonstrate end-to-end orchestration
- [ ] Property tests validate topological sort invariants
- [ ] No `unwrap()` in library code
- [ ] All public items have rustdoc comments
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test --package ecl-workflows` passes

---

