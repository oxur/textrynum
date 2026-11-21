# Stage 2.1: Step Trait Definition — historical criteria

> Historical criteria record, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

Criteria and checkbox states below are transcribed, not re-evaluated. No new row statuses, verification results or sign-offs are assigned.

<!-- Historical source: crates/design/dev/0003-phase-2-01-step-trait.md, lines 870-884. Original bytes are recoverable from import commit 2d006a7. -->

## Acceptance Criteria

- [ ] `Step` trait compiles with proper bounds
- [ ] `StepContext` provides all required dependencies
- [ ] `RetryPolicy` calculates delays correctly
- [ ] `StepRegistry` can register and retrieve steps
- [ ] All types implement required traits (Debug, Clone where needed)
- [ ] Test coverage ≥95%
- [ ] All public APIs have rustdoc comments
- [ ] Examples in docs compile
- [ ] No compiler warnings
- [ ] Follows Rust anti-patterns guide

---

