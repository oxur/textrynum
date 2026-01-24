# Phase 2 Implementation Plan: Step Abstraction Framework

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2 - Step Abstraction Framework
**Target Duration:** ~2 weeks
**Status:** Ready for Implementation

---

## Overview

This is the main planning document for Phase 2 of the ECL project. Phase 2 builds upon Phase 1's foundation to create a reusable, declarative step framework that enables flexible workflow composition.

**Phase 2 Deliverable:** Reusable `Step` trait system with the Critique-Revise toy workflow fully implemented using the new framework.

### What Phase 2 Achieves

- **Abstraction**: Define `Step` trait for reusable workflow components
- **Execution**: Build robust step executor with retry, validation, and observability
- **Implementation**: Create concrete step implementations for the toy workflow
- **Orchestration**: Build workflow orchestrator for composing steps
- **Persistence**: Add SQLx-based database layer for workflow state
- **Observability**: Implement structured logging and tracing
- **Integration**: Wire everything together into a working end-to-end system

### Prerequisites

Phase 1 must be complete:

- ✅ Project scaffolding with all crates
- ✅ Core types (WorkflowId, StepId, Error, etc.)
- ✅ LLM abstraction layer (LlmProvider trait + implementations)
- ✅ Basic Restate integration working
- ✅ Test coverage ≥95%

### Critical Guidelines

1. **Load anti-patterns first**: Always read `assets/ai/ai-rust/guides/11-anti-patterns.md`
2. **Test coverage**: Maintain ≥95% (see `assets/ai/CLAUDE-CODE-COVERAGE.md`)
3. **Error handling**: Use `thiserror`, avoid `unwrap()` in library code
4. **Parameters**: Use `&str` not `&String`, `&[T]` not `&Vec<T>`
5. **Async**: Never use blocking I/O in async functions
6. **Public types**: Use `#[non_exhaustive]` on public enums/structs
7. **Trait objects**: Use `dyn Trait` where needed, ensure object-safety

---

## Stage Documents

Each stage has its own detailed implementation document:

### [Stage 2.1: Step Trait Definition](./0003-phase-2-01-step-trait.md)

**Duration:** 4-5 hours

Define the core `Step` trait that all workflow steps implement:

- Step trait with associated types (Input, Output)
- StepContext providing runtime dependencies
- RetryPolicy with exponential backoff
- StepRegistry for dynamic step lookup
- ValidationError with field-level details

---

### [Stage 2.2: Step Executor](./0004-phase-2-02-step-executor.md)

**Duration:** 4-5 hours

Create executor that runs steps with retry, validation, and observability:

- StepExecution result type with traces
- StepExecutor with full retry logic
- Integration with backon for exponential backoff
- Trace collection and metadata recording

---

### [Stage 2.3: Toy Workflow Step Implementations](./0005-phase-2-03-toy-workflow-steps.md)

**Duration:** 5-6 hours

Implement the three steps for the Critique-Revise toy workflow:

- GenerateStep: Creates initial content
- CritiqueStep: Analyzes and decides pass/revise
- ReviseStep: Improves content based on feedback
- Prompt templates externalized as files

---

### [Stage 2.4: Workflow Orchestrator](./0006-phase-2-04-workflow-orchestrator.md)

**Duration:** 6-7 hours

Create orchestrator that composes steps into executable workflows:

- WorkflowDefinition with declarative step composition
- StepDefinition with dependency tracking
- Transition types (Sequential, Conditional, RevisionLoop)
- Topological sorting and execution order
- Revision loop handling with bounded iteration

---

### [Stage 2.5: SQLx Persistence Layer](./0007-phase-2-05-sqlx-persistence.md)

**Duration:** 5-6 hours

Add database persistence for workflow state and metadata:

- Database schema (workflows, step_executions, artifacts)
- SQLx migrations for SQLite and Postgres
- WorkflowRepository for workflow CRUD
- StepExecutionRepository for execution history
- Compile-time query validation

---

### [Stage 2.6: Observability Infrastructure](./0008-phase-2-06-observability.md)

**Duration:** 3-4 hours

Implement structured logging and tracing:

- Structured logging with twyg
- Tracing spans for workflow and step execution
- Log correlation by workflow_id
- Performance monitoring
- Optional: SQL exporter for twyg

---

### [Stage 2.7: Toy Workflow Integration](./0009-phase-2-07-integration.md)

**Duration:** 6-8 hours

Wire everything together into working Critique-Revise workflow:

- Complete workflow definition
- Restate integration with orchestrator
- CLI commands (run, status)
- Comprehensive integration tests
- End-to-end validation

---

## Phase 2 Timeline

```
Week 1:
├── Day 1-2: Stages 2.1-2.2 (Step trait and executor)
├── Day 3-4: Stage 2.3 (Toy workflow steps)
└── Day 5: Stage 2.4 (Orchestrator - start)

Week 2:
├── Day 1: Stage 2.4 (Orchestrator - complete)
├── Day 2-3: Stage 2.5 (SQLx persistence)
├── Day 4: Stage 2.6 (Observability)
└── Day 5: Stage 2.7 (Integration and testing)
```

**Total Estimated Effort:** 33-40 hours (~2 weeks)

---

## Cross-Stage Dependencies

```
Phase 1 (Complete)
    ↓
Stage 2.1 (Step Trait) ← Foundation for all steps
    ↓
Stage 2.2 (Executor) ← Uses Step trait
    ↓
Stage 2.3 (Step Implementations) ← Implements Step trait
    ↓
Stage 2.4 (Orchestrator) ← Composes steps
    ↓
Stage 2.5 (Persistence) ← Stores workflow state
    ↓
Stage 2.6 (Observability) ← Monitors execution
    ↓
Stage 2.7 (Integration) ← Brings everything together
```

---

## Final Checklist

Before moving to Phase 3, verify:

### Code Quality

- [ ] All code follows Rust anti-patterns guide
- [ ] No `unwrap()` in library code
- [ ] All errors use `thiserror`
- [ ] Parameters use `&str` not `&String`
- [ ] Public types use `#[non_exhaustive]`
- [ ] All async code uses async I/O

### Testing

- [ ] Overall test coverage ≥95%
- [ ] All unit tests pass
- [ ] Integration tests pass
- [ ] Property tests for core types
- [ ] No ignored tests (except manual integration)

### Documentation

- [ ] All public items have rustdoc
- [ ] Examples compile
- [ ] README updated

### Build System

- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `just test` passes
- [ ] `just coverage` shows ≥95%

### Functionality

- [ ] Critique-Revise workflow runs end-to-end
- [ ] Steps can be composed declaratively
- [ ] Revision loops work with bounded iteration
- [ ] Database persistence works
- [ ] Restate integration works
- [ ] CLI commands work

---

## Next Steps

Start with **[Stage 2.1: Step Trait Definition](./0003-phase-2-01-step-trait.md)**

---

**Document Version:** 1.0
**Last Updated:** 2026-01-23
**Status:** Ready for Implementation
