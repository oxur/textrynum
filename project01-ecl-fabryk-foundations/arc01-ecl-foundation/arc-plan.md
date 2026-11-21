# ECL Phase 1 — Foundation

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 90-461. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 1: Foundation

**Objective**: Validate core architecture with minimal implementation
**Duration**: ~2 weeks
**Deliverable**: Working prototype exercising all core patterns

### Stage 1.1: Project Scaffolding

**Goal**: Establish project structure, dependencies, and build configuration.

**Tasks**:

1. Initialize Cargo workspace with the following crates:
   - `ecl-core`: Core types, traits, error handling
   - `ecl-steps`: Step implementations
   - `ecl-workflows`: Restate workflow definitions
   - `ecl-cli`: Command-line interface (placeholder)
2. Configure dependencies in workspace `Cargo.toml`:

   ```toml
   [workspace.dependencies]
   restate-sdk = "0.7"
   tokio = { version = "1", features = ["full"] }
   sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres"] }
   llm = "0.2"  # or current version - verify crates.io
   backon = "1"
   confyg = { version = "0.3" }
   twyg = "0.6"
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   thiserror = "2"
   anyhow = "1"
   uuid = { version = "1", features = ["v4", "serde"] }
   chrono = { version = "0.4", features = ["serde"] }
   ```

3. Create `rust-toolchain.toml` pinning stable Rust version
4. Set up basic `justfile` or `Makefile` with commands:
   - `dev`: Start Restate server + application
   - `test`: Run test suite
   - `lint`: Run clippy + rustfmt check
5. Create `.env.example` with required environment variables
6. Add basic README with setup instructions

**Acceptance Criteria**:

- [ ] `cargo build` succeeds with no errors
- [ ] `cargo clippy` passes with no warnings
- [ ] All crates compile and link correctly
- [ ] Restate SDK dependency resolves

**Files to Create**:

```
ecl/
├── Cargo.toml (workspace)
├── rust-toolchain.toml
├── justfile
├── .env.example
├── README.md
├── ecl-core/
│   ├── Cargo.toml
│   └── src/lib.rs
├── ecl-steps/
│   ├── Cargo.toml
│   └── src/lib.rs
├── ecl-workflows/
│   ├── Cargo.toml
│   └── src/lib.rs
└── ecl-cli/
    ├── Cargo.toml
    └── src/main.rs
```

---

### Stage 1.2: Core Types and Error Handling

**Goal**: Define foundational types that all other code builds upon.

**Tasks**:

1. Define `WorkflowId` newtype (wraps UUID)
2. Define `StepId` newtype (wraps String, e.g., "generate", "critique")
3. Define `StepResult<T>` enum:

   ```rust
   pub enum StepResult<T> {
       Success(T),
       NeedsRevision { output: T, feedback: String },
       Failed { error: StepError, retryable: bool },
   }
   ```

4. Define `StepError` enum with variants:
   - `LlmError(String)` - Claude API failures
   - `ValidationError(String)` - Output validation failures
   - `IoError(String)` - File/network issues
   - `TimeoutError` - Step exceeded time limit
   - `MaxRevisionsExceeded { attempts: u32 }` - Bounded iteration limit
5. Define `WorkflowState` enum:
   - `Pending`, `Running`, `WaitingForRevision`, `Completed`, `Failed`
6. Define `StepMetadata` struct:

   ```rust
   pub struct StepMetadata {
       pub step_id: StepId,
       pub started_at: DateTime<Utc>,
       pub completed_at: Option<DateTime<Utc>>,
       pub attempt: u32,
       pub llm_tokens_used: Option<u64>,
   }
   ```

7. Implement `std::error::Error` for `StepError` using `thiserror`
8. Add `From` implementations for common error conversions

**Acceptance Criteria**:

- [ ] All types are `Clone`, `Debug`, `Serialize`, `Deserialize` where appropriate
- [ ] Error types implement `std::error::Error`
- [ ] Unit tests for serialization roundtrips

**Files to Create/Modify**:

```
ecl-core/src/
├── lib.rs
├── types.rs      (WorkflowId, StepId, StepResult, WorkflowState)
├── error.rs      (StepError, error conversions)
└── metadata.rs   (StepMetadata)
```

---

### Stage 1.3: LLM Abstraction Layer

**Goal**: Wrap the `llm` crate behind a trait for testability and future provider flexibility.

**Tasks**:

1. Define `LlmProvider` trait:

   ```rust
   #[async_trait]
   pub trait LlmProvider: Send + Sync {
       async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
       async fn complete_streaming(&self, request: CompletionRequest) -> Result<CompletionStream, LlmError>;
   }
   ```

2. Define `CompletionRequest` struct:
   - `system_prompt: Option<String>`
   - `messages: Vec<Message>`
   - `max_tokens: u32`
   - `temperature: Option<f32>`
   - `stop_sequences: Vec<String>`
3. Define `CompletionResponse` struct:
   - `content: String`
   - `tokens_used: TokenUsage { input: u64, output: u64 }`
   - `stop_reason: StopReason`
4. Implement `ClaudeProvider` struct wrapping `llm` crate:
   - Constructor takes API key from config
   - Implements `LlmProvider` trait
   - Maps `llm` types to our abstraction
5. Implement `MockLlmProvider` for testing:
   - Constructor takes `Vec<String>` of canned responses
   - Returns responses in order, cycles if exhausted
6. Add retry wrapper using `backon`:
   - Exponential backoff: 1s, 2s, 4s, max 3 attempts
   - Retry on rate limit (429) and server errors (5xx)
   - No retry on auth errors (401) or bad request (400)

**Acceptance Criteria**:

- [ ] `ClaudeProvider` successfully calls Claude API (integration test, can be ignored in CI)
- [ ] `MockLlmProvider` returns expected responses in unit tests
- [ ] Retry logic correctly handles retriable vs non-retriable errors
- [ ] All types implement required traits for Restate serialization

**Files to Create**:

```
ecl-core/src/
├── llm/
│   ├── mod.rs
│   ├── provider.rs    (LlmProvider trait, request/response types)
│   ├── claude.rs      (ClaudeProvider implementation)
│   ├── mock.rs        (MockLlmProvider for testing)
│   └── retry.rs       (retry wrapper with backon)
```

---

### Stage 1.4: Basic Restate Workflow

**Goal**: Create minimal 2-step workflow demonstrating Restate integration.

**Tasks**:

1. Create Restate service definition with `#[restate_sdk::service]`:
   - Service name: `EclWorkflow`
   - Method: `run_simple(input: SimpleWorkflowInput) -> SimpleWorkflowOutput`
2. Define `SimpleWorkflowInput`:
   - `workflow_id: WorkflowId`
   - `topic: String`
3. Define `SimpleWorkflowOutput`:
   - `workflow_id: WorkflowId`
   - `generated_text: String`
   - `critique: String`
4. Implement workflow with two durable steps:

   ```rust
   // Step 1: Generate
   let draft = ctx.run(|| async {
       llm.complete(generate_prompt(&input.topic)).await
   }).await?;

   // Step 2: Critique
   let critique = ctx.run(|| async {
       llm.complete(critique_prompt(&draft)).await
   }).await?;
   ```

5. Add `main.rs` that:
   - Initializes twyg logging
   - Loads config (API key from env for now)
   - Creates `ClaudeProvider`
   - Starts Restate HTTP endpoint
6. Create `docker-compose.yml` for local Restate server
7. Write shell script to:
   - Start Restate server
   - Register service with Restate
   - Invoke test workflow

**Acceptance Criteria**:

- [ ] Workflow completes successfully end-to-end
- [ ] Killing process mid-workflow and restarting resumes from last completed step
- [ ] Workflow output contains both generated text and critique
- [ ] Tracing logs show step execution

**Files to Create**:

```
ecl-workflows/src/
├── lib.rs
├── simple.rs     (SimpleWorkflow service)
└── main.rs       (entrypoint)

docker-compose.yml
scripts/
└── run-simple-workflow.sh
```

---

### Stage 1.5: Feedback Loop with Durable Promises

**Goal**: Extend workflow to demonstrate bounded revision cycles using Durable Promises.

**Tasks**:

1. Upgrade to `#[restate_sdk::workflow]` (from `#[service]`)
2. Add workflow state in Restate K/V:
   - `revision_count: u32`
   - `current_draft: String`
   - `decision: Option<CritiqueDecision>`
3. Define `CritiqueDecision` enum: `Pass`, `Revise { feedback: String }`
4. Implement revision loop:

   ```rust
   const MAX_REVISIONS: u32 = 3;

   loop {
       let critique_result = ctx.run(|| critique_step(&draft)).await?;

       match critique_result.decision {
           CritiqueDecision::Pass => break,
           CritiqueDecision::Revise { feedback } if revision_count < MAX_REVISIONS => {
               draft = ctx.run(|| revise_step(&draft, &feedback)).await?;
               revision_count += 1;
           }
           CritiqueDecision::Revise { .. } => {
               return Err(StepError::MaxRevisionsExceeded { attempts: MAX_REVISIONS });
           }
       }
   }
   ```

5. Add shared handler for external signaling (future use):

   ```rust
   #[shared]
   async fn signal_revision(&self, ctx: SharedWorkflowContext, feedback: String) {
       ctx.resolve_promise("external_revision", feedback);
   }
   ```

6. Write integration test that:
   - Starts workflow with topic that requires revision
   - Verifies revision count increments
   - Verifies loop terminates (either Pass or MaxRevisions)

**Acceptance Criteria**:

- [ ] Workflow correctly loops on `Revise` decisions
- [ ] Loop terminates at `MAX_REVISIONS` with appropriate error
- [ ] Workflow survives restart mid-revision-loop
- [ ] State (revision_count, current_draft) persists across restarts

**Files to Modify**:

```
ecl-workflows/src/simple.rs → ecl-workflows/src/critique_loop.rs
```

---

### Stage 1.6: Phase 1 Integration Test Suite

**Goal**: Comprehensive tests proving Phase 1 deliverables work correctly.

**Tasks**:

1. Create `tests/` directory with integration tests
2. Implement test harness:
   - Spawns Restate server in Docker (or uses testcontainers-rs)
   - Registers ECL service
   - Provides helper functions to invoke workflows and wait for completion
3. Write integration tests:
   - `test_simple_workflow_completes`: Happy path
   - `test_workflow_survives_restart`: Kill mid-execution, verify resume
   - `test_revision_loop_terminates`: Verify MAX_REVISIONS enforced
   - `test_llm_retry_on_rate_limit`: Mock 429, verify retry
   - `test_llm_no_retry_on_auth_error`: Mock 401, verify immediate failure
4. Add unit tests for:
   - Error serialization/deserialization
   - Type conversions
   - Mock LLM provider behavior
5. Configure CI (GitHub Actions) to:
   - Run unit tests (no external dependencies)
   - Skip integration tests in CI (require Restate server)
   - Run clippy and rustfmt checks

**Acceptance Criteria**:

- [ ] All unit tests pass
- [ ] Integration tests pass locally with Restate server
- [ ] CI pipeline passes (unit tests + linting)
- [ ] Test coverage report generated

**Files to Create**:

```
tests/
├── common/
│   └── mod.rs          (test harness, helpers)
├── integration/
│   ├── mod.rs
│   ├── simple_workflow.rs
│   └── revision_loop.rs
└── unit/
    ├── mod.rs
    ├── types.rs
    └── llm_mock.rs

.github/workflows/ci.yml
```

---


<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 1-32. Original bytes are recoverable from import commit 2d006a7. -->

# Phase 1 Implementation Plan: ECL Foundation

**Version:** 1.0
**Date:** 2026-01-22
**Phase:** 1 - Foundation
**Target Duration:** ~2 weeks
**Status:** Ready for Implementation

---

## Overview

This document provides detailed, actionable implementation steps for Phase 1 of the ECL project. Each stage includes:

- **Detailed Implementation Steps** - Specific coding tasks
- **File Contents Skeleton** - Complete file structures with signatures
- **Dependencies** - What must be completed before starting
- **Testing Strategy** - Specific tests and test data
- **Potential Blockers** - Risks and mitigation strategies
- **Estimated Effort** - Rough time per stage

**Critical Guidelines to Follow:**

1. **Load anti-patterns first**: Always read `assets/ai/ai-rust/guides/11-anti-patterns.md` before writing code
2. **Test coverage**: Maintain ≥95% coverage (see `assets/ai/CLAUDE-CODE-COVERAGE.md`)
3. **Error handling**: Use `thiserror` for errors, avoid `unwrap()` in library code
4. **Parameters**: Use `&str` not `&String`, `&[T]` not `&Vec<T>`
5. **Async**: Never use blocking I/O in async functions
6. **Public types**: Use `#[non_exhaustive]` on public enums and structs

---


<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 3517-3605. Original bytes are recoverable from import commit 2d006a7. -->

## Cross-Stage Dependencies

```
Stage 1.1 (Scaffolding)
    ↓
Stage 1.2 (Core Types) ← Must have build system
    ↓
Stage 1.3 (LLM Layer) ← Needs Error and Result types
    ↓
Stage 1.4 (Basic Workflow) ← Needs LLM abstraction
    ↓
Stage 1.5 (Feedback Loop) ← Needs basic workflow
    ↓
Stage 1.6 (Integration Tests) ← Needs everything
```

## Summary Checklist

Before moving to Phase 2, verify:

**Code Quality:**
- [ ] All code follows Rust anti-patterns guide (11-anti-patterns.md)
- [ ] No `unwrap()` in library code
- [ ] All errors use `thiserror`
- [ ] Parameters use `&str` not `&String`, `&[T]` not `&Vec<T>`
- [ ] Public types use `#[non_exhaustive]`
- [ ] All async code uses async I/O (no blocking)

**Testing:**
- [ ] Overall test coverage ≥95%
- [ ] All unit tests pass
- [ ] Integration tests pass (locally with Restate)
- [ ] Property tests added for core types
- [ ] No ignored tests (except manual integration tests)

**Documentation:**
- [ ] All public items have rustdoc comments
- [ ] README is up to date
- [ ] Examples compile and run

**Build System:**
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `just test` passes
- [ ] `just coverage` shows ≥95%

**Infrastructure:**
- [ ] Docker Compose works
- [ ] Restate server starts and registers services
- [ ] Database migrations run successfully
- [ ] Environment variables documented

---

## Appendix: Key Rust Patterns to Apply

### From Anti-Patterns Guide (11-anti-patterns.md)

- **AP-02**: Use `&str` not `&String`, `&[T]` not `&Vec<T>` in parameters
- **AP-03**: Use `Box<dyn Error + Send + Sync>` for thread-safe errors
- **AP-04**: Always annotate `.collect()` with type
- **AP-06**: Keep fields private when invariants exist
- **AP-08**: Avoid `unwrap()` in library code

### From Core Idioms (01-core-idioms.md)

- **ID-01**: Use `#[non_exhaustive]` on public enums/structs
- **ID-02**: Use `mem::take` and `mem::replace` for enum transformations
- **ID-04**: Follow `as_/to_/into_` conventions

### From Error Handling (03-error-handling.md)

- **EH-02**: Use `?` operator for error propagation
- **EH-03**: Use `thiserror` for library errors
- **EH-05**: Provide Result type alias

### From Concurrency & Async (07-concurrency-async.md)

- **CA-03**: Never use blocking I/O in async functions
- **CA-01**: Ensure proper Send/Sync bounds

---

**Document End**

**Version:** 1.0
**Last Updated:** 2026-01-22
**Ready for Implementation:** ✓

## Reconstructed slice locations

- [Stage 1.1: Project Scaffolding](slice01-project-scaffolding/slice-plan.md)
- [Stage 1.2: Core Types and Error Handling](slice02-core-types-and-error-handling/slice-plan.md)
- [Stage 1.3: LLM Abstraction Layer](slice03-llm-abstraction-layer/slice-plan.md)
- [Stage 1.4: Basic Restate Workflow](slice04-basic-restate-workflow/slice-plan.md)
- [Stage 1.5: Feedback Loop with Durable Promises](slice05-feedback-loop-with-durable-promises/slice-plan.md)
- [Stage 1.6: Phase 1 Integration Test Suite](slice06-phase-1-integration-test-suite/slice-plan.md)
