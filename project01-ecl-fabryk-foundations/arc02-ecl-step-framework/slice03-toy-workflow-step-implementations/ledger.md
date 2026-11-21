# Stage 2.3: Toy Workflow Step Implementations — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0005-phase-2-03-toy-workflow-steps.md, lines 1615-1670. Original bytes are recoverable from import commit 3a39aea. -->

## Acceptance Criteria

### Functionality

- [ ] GenerateStep produces coherent output via Claude
- [ ] CritiqueStep parses JSON and returns correct decision
- [ ] ReviseStep improves content based on feedback
- [ ] All steps validate inputs before execution
- [ ] All steps validate outputs after execution
- [ ] Steps handle LLM errors gracefully
- [ ] Steps can be tested with `MockLlmProvider`

### Code Quality

- [ ] All code follows Rust anti-patterns guide
- [ ] No `unwrap()` in library code
- [ ] All errors use `thiserror`
- [ ] Parameters use `&str` not `&String`
- [ ] Public types use `#[non_exhaustive]`
- [ ] All async code uses async I/O
- [ ] No clippy warnings

### Documentation

- [ ] All public items have rustdoc comments
- [ ] Examples in docs compile
- [ ] Module-level documentation complete
- [ ] Prompt templates documented

### Testing

- [ ] Test coverage ≥95%
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Property tests pass
- [ ] Error paths covered
- [ ] Edge cases tested

### Integration

- [ ] Prompts are externalized in separate files
- [ ] Steps can be registered in `StepRegistry`
- [ ] Steps work with `StepExecutor`
- [ ] Tracing integration works
- [ ] Metadata recording works

### Build System

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `just test` passes
- [ ] `just coverage` shows ≥95%

---

