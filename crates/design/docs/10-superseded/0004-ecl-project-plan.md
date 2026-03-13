---
number: 4
title: "ECL Project Plan"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-01-22
updated: 2026-03-13
state: Superseded
supersedes: null
superseded-by: 18
version: 1.0
---

# ECL Project Plan

## Rust-Based AI Workflow Orchestration System

**Version**: 1.0
**Date**: January 2026
**Status**: Implementation Ready
**Audience**: Claude Code (Plan Mode → Implementation)

---

## Document Purpose

This project plan breaks down the ECL system into implementable phases and stages. Each phase is designed to be:

1. **Self-contained**: Produces working, testable functionality
2. **Incrementally valuable**: Builds on prior phases without requiring rework
3. **Claude Code ready**: Detailed enough for Plan Mode to generate implementation specs

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                          ECL System                             │
├─────────────────────────────────────────────────────────────────┤
│  Workflow Layer         │  Restate workflows + Step trait       │
│  LLM Layer              │  llm crate (Claude provider)          │
│  Persistence Layer      │  SQLx (SQLite dev / Postgres prod)    │
│  Resilience Layer       │  backon (retry), failsafe (circuit)   │
│  Config Layer           │  confyg                               │
│  Observability Layer    │  twyg                                 │
├─────────────────────────────────────────────────────────────────┤
│  External Dependencies  │  Restate Server, Claude API, Database │
└─────────────────────────────────────────────────────────────────┘
```

---

## Reference Workflows

### Toy Workflow: "Critique-Revise Loop"

Used for Phase 1-2 validation. Simple 3-step workflow demonstrating core patterns.

| Step | Name | Input | Output | Validation |
|------|------|-------|--------|------------|
| 1 | **Generate** | Topic string | Draft text | Non-empty, <2000 chars |
| 2 | **Critique** | Draft text | Critique + pass/revise decision | Valid JSON with decision field |
| 3 | **Revise** | Draft + Critique | Final text | Bounded: max 3 revision cycles |

**Key patterns exercised**:

- Sequential step execution
- Bounded feedback loop (Step 3 → Step 2, max 3 iterations)
- Durable Promise for revision signaling
- Basic validation gates

### Target Workflow: "Codebase Migration Analyzer"

The real workflow ECL is built for. Full implementation targeted by Phase 3+.

| Step | Name | Inputs | Outputs |
|------|------|--------|---------|
| 0 | **Intake** | Code path, intended use, vision doc, deploy configs | Validated input bundle |
| 1 | **Codebase Analysis** | Input bundle | Implementation grade report, current component breakdown, recommended refactor breakdown |
| 2 | **Architecture Design** | Step 1 docs + codebase access | Language-agnostic ideal architecture doc |
| 3 | **Project Planning** | Architecture doc + target config + SKILL.md | Phased project plan (phases → stages) |
| 4 | **Implementation Specs** | Project plan + docs | Per-phase implementation docs |
| 5 | **Quality Assessment** | New code repo + docs | Standards analysis, plan adherence report |
| 6 | **Delivery** | All artifacts | Final writeup, presentation, code package |

---

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

## Phase 2: Step Abstraction Framework

**Objective**: Build reusable step framework enabling declarative workflow definition
**Duration**: ~2 weeks
**Deliverable**: Reusable `Step` trait system with the Critique-Revise toy workflow fully implemented

### Stage 2.1: Step Trait Definition

**Goal**: Define the core `Step` trait that all workflow steps implement.

**Tasks**:

1. Define `Step` trait:

   ```rust
   #[async_trait]
   pub trait Step: Send + Sync {
       type Input: Serialize + DeserializeOwned + Send;
       type Output: Serialize + DeserializeOwned + Send;

       fn id(&self) -> &StepId;
       fn name(&self) -> &str;

       async fn execute(&self, ctx: &StepContext, input: Self::Input) -> StepResult<Self::Output>;

       async fn validate_input(&self, input: &Self::Input) -> Result<(), ValidationError> {
           Ok(()) // Default: no validation
       }

       async fn validate_output(&self, output: &Self::Output) -> Result<(), ValidationError> {
           Ok(()) // Default: no validation
       }

       fn max_revisions(&self) -> u32 {
           3 // Default revision limit
       }

       fn retry_policy(&self) -> RetryPolicy {
           RetryPolicy::default()
       }
   }
   ```

2. Define `StepContext` providing access to:
   - `LlmProvider` reference
   - Workflow metadata (workflow_id, current step, attempt number)
   - Tracing span
   - Artifact storage (placeholder for now)
3. Define `RetryPolicy`:

   ```rust
   pub struct RetryPolicy {
       pub max_attempts: u32,
       pub initial_delay: Duration,
       pub max_delay: Duration,
       pub multiplier: f64,
   }
   ```

4. Define `ValidationError` struct with field-level error details
5. Create `StepRegistry` for dynamic step lookup:

   ```rust
   pub struct StepRegistry {
       steps: HashMap<StepId, Arc<dyn Step<Input = Value, Output = Value>>>,
   }
   ```

**Acceptance Criteria**:

- [ ] `Step` trait compiles and is object-safe where needed
- [ ] `StepContext` provides all required dependencies
- [ ] `RetryPolicy` integrates with `backon`
- [ ] Unit tests for validation logic

**Files to Create**:

```
ecl-core/src/
├── step/
│   ├── mod.rs
│   ├── trait.rs      (Step trait definition)
│   ├── context.rs    (StepContext)
│   ├── policy.rs     (RetryPolicy)
│   ├── validation.rs (ValidationError, validators)
│   └── registry.rs   (StepRegistry)
```

---

### Stage 2.2: Step Executor

**Goal**: Create executor that runs steps with retry, validation, and observability.

**Tasks**:

1. Create `StepExecutor` struct:

   ```rust
   pub struct StepExecutor {
       llm: Arc<dyn LlmProvider>,
       registry: Arc<StepRegistry>,
   }
   ```

2. Implement `execute_step` method:

   ```rust
   pub async fn execute_step<S: Step>(
       &self,
       step: &S,
       input: S::Input,
       workflow_id: &WorkflowId,
   ) -> Result<StepExecution<S::Output>, StepError>
   ```

3. Execution flow:
   1. Validate input (fail fast if invalid)
   2. Execute with retry policy
   3. Validate output
   4. Return `StepExecution` with metadata
4. Define `StepExecution<T>`:

   ```rust
   pub struct StepExecution<T> {
       pub output: T,
       pub metadata: StepMetadata,
       pub traces: Vec<TraceEvent>,
   }
   ```

5. Integrate with Restate's `ctx.run()`:
   - Wrap execution in durable operation
   - Handle Restate replay (don't re-execute completed steps)
6. Add detailed logging:
   - Step start/end
   - LLM calls with token counts
   - Validation results
   - Retry attempts

**Acceptance Criteria**:

- [ ] Steps execute with proper retry behavior
- [ ] Input/output validation runs at correct points
- [ ] Tracing captures all execution details
- [ ] Failed validation produces clear error messages

**Files to Create**:

```
ecl-core/src/step/
├── executor.rs   (StepExecutor)
└── execution.rs  (StepExecution)
```

---

### Stage 2.3: Toy Workflow Step Implementations

**Goal**: Implement the three steps for the Critique-Revise toy workflow.

**Tasks**:

1. Implement `GenerateStep`:
   - Input: `GenerateInput { topic: String }`
   - Output: `GenerateOutput { draft: String }`
   - Prompt: System prompt for content generation
   - Validation: Output non-empty, under 2000 chars
2. Implement `CritiqueStep`:
   - Input: `CritiqueInput { draft: String }`
   - Output: `CritiqueOutput { critique: String, decision: CritiqueDecision }`
   - Prompt: System prompt for critical analysis with JSON output
   - Validation: Decision field present and valid
3. Implement `ReviseStep`:
   - Input: `ReviseInput { draft: String, feedback: String, attempt: u32 }`
   - Output: `ReviseOutput { revised_draft: String }`
   - Prompt: System prompt incorporating feedback
   - Validation: Output different from input (actual revision occurred)
4. Create prompt templates in separate files:
   - `prompts/generate.txt`
   - `prompts/critique.txt`
   - `prompts/revise.txt`
5. Register all steps in `StepRegistry`

**Acceptance Criteria**:

- [ ] Each step produces coherent output via Claude
- [ ] Validation catches malformed outputs
- [ ] Prompts are externalized and editable
- [ ] Steps can be tested with `MockLlmProvider`

**Files to Create**:

```
ecl-steps/src/
├── lib.rs
├── toy/
│   ├── mod.rs
│   ├── generate.rs
│   ├── critique.rs
│   └── revise.rs
└── prompts/
    ├── generate.txt
    ├── critique.txt
    └── revise.txt
```

---

### Stage 2.4: Workflow Orchestrator

**Goal**: Create orchestrator that composes steps into executable workflows.

**Tasks**:

1. Define `WorkflowDefinition`:

   ```rust
   pub struct WorkflowDefinition {
       pub id: String,
       pub name: String,
       pub steps: Vec<StepDefinition>,
       pub transitions: Vec<Transition>,
   }
   ```

2. Define `StepDefinition`:

   ```rust
   pub struct StepDefinition {
       pub step_id: StepId,
       pub depends_on: Vec<StepId>,
       pub revision_source: Option<StepId>, // Which step can request revision
   }
   ```

3. Define `Transition`:

   ```rust
   pub enum Transition {
       Sequential { from: StepId, to: StepId },
       Conditional { from: StepId, to: StepId, condition: String },
       RevisionLoop { reviser: StepId, validator: StepId, max_iterations: u32 },
   }
   ```

4. Implement `WorkflowOrchestrator`:

   ```rust
   pub struct WorkflowOrchestrator {
       executor: StepExecutor,
       definitions: HashMap<String, WorkflowDefinition>,
   }

   impl WorkflowOrchestrator {
       pub async fn run(&self, workflow_id: &str, input: Value) -> Result<WorkflowResult, WorkflowError>;
   }
   ```

5. Orchestrator execution logic:
   - Topologically sort steps by dependencies
   - Execute steps in order, passing outputs as inputs
   - Handle revision loops with bounded iteration
   - Aggregate all outputs into `WorkflowResult`

**Acceptance Criteria**:

- [ ] Workflows execute steps in correct order
- [ ] Dependencies are respected
- [ ] Revision loops work correctly
- [ ] `WorkflowResult` contains all step outputs and metadata

**Files to Create**:

```
ecl-workflows/src/
├── definition.rs    (WorkflowDefinition, StepDefinition, Transition)
├── orchestrator.rs  (WorkflowOrchestrator)
└── result.rs        (WorkflowResult)
```

---

### Stage 2.5: SQLx Persistence Layer

**Goal**: Add database persistence for workflow state and metadata.

**Tasks**:

1. Define database schema:

   ```sql
   -- workflows table
   CREATE TABLE workflows (
       id UUID PRIMARY KEY,
       definition_id TEXT NOT NULL,
       state TEXT NOT NULL,  -- WorkflowState enum as string
       input JSONB NOT NULL,
       output JSONB,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       completed_at TIMESTAMPTZ
   );

   -- step_executions table
   CREATE TABLE step_executions (
       id UUID PRIMARY KEY,
       workflow_id UUID NOT NULL REFERENCES workflows(id),
       step_id TEXT NOT NULL,
       attempt INTEGER NOT NULL,
       state TEXT NOT NULL,
       input JSONB NOT NULL,
       output JSONB,
       error TEXT,
       started_at TIMESTAMPTZ NOT NULL,
       completed_at TIMESTAMPTZ,
       tokens_used BIGINT
   );

   -- artifacts table (placeholder for Phase 3)
   CREATE TABLE artifacts (
       id UUID PRIMARY KEY,
       workflow_id UUID NOT NULL REFERENCES workflows(id),
       step_id TEXT NOT NULL,
       name TEXT NOT NULL,
       content_type TEXT NOT NULL,
       path TEXT NOT NULL,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   ```

2. Create SQLx migrations in `migrations/` directory
3. Implement `WorkflowRepository`:

   ```rust
   pub struct WorkflowRepository {
       pool: PgPool, // or SqlitePool for dev
   }

   impl WorkflowRepository {
       pub async fn create(&self, workflow: &Workflow) -> Result<(), DbError>;
       pub async fn get(&self, id: &WorkflowId) -> Result<Option<Workflow>, DbError>;
       pub async fn update_state(&self, id: &WorkflowId, state: WorkflowState) -> Result<(), DbError>;
       pub async fn list_by_state(&self, state: WorkflowState) -> Result<Vec<Workflow>, DbError>;
   }
   ```

4. Implement `StepExecutionRepository`:

   ```rust
   pub struct StepExecutionRepository { ... }

   impl StepExecutionRepository {
       pub async fn record(&self, execution: &StepExecution) -> Result<(), DbError>;
       pub async fn get_for_workflow(&self, workflow_id: &WorkflowId) -> Result<Vec<StepExecution>, DbError>;
   }
   ```

5. Configure SQLx for compile-time query checking
6. Add database URL to config (SQLite for dev, Postgres for prod)

**Acceptance Criteria**:

- [ ] Migrations run successfully on SQLite and Postgres
- [ ] Compile-time query validation passes
- [ ] CRUD operations work correctly
- [ ] Integration tests use in-memory SQLite

**Files to Create**:

```
ecl-core/src/
├── db/
│   ├── mod.rs
│   ├── workflow_repo.rs
│   ├── step_repo.rs
│   └── artifact_repo.rs

migrations/
├── 001_initial.sql
└── 002_artifacts.sql

.sqlx/  (query metadata for offline compilation)
```

---

### Stage 2.6: Observability Infrastructure

**Goal**: Basic but consistent logging patterns.

**Tasks**:

1. Add structured fields to all logging calls
2. Log other stats as discovered / determined as useful
3. Examine strengths / usefulness of SQL exporter for twyg possibly submit PR to projectin support of this

**Acceptance Criteria**:

- [ ] All operations produce structured log messages
- [ ] Traces can be correlated by workflow_id
- [ ] Log output is readable in dev, parseable in prod
- [ ] Performance overhead is minimal (<5%)

**Files to Create**:

```
ecl-core/src/
├── logging/
│   ├── mod.rs
│   ├── config.rs
│   ├── exporter.rs
│   └── spans.rs
```

---

### Stage 2.7: Toy Workflow Integration

**Goal**: Wire everything together into working Critique-Revise workflow.

**Tasks**:

1. Create `CritiqueReviseWorkflow` definition:

   ```rust
   pub fn critique_revise_workflow() -> WorkflowDefinition {
       WorkflowDefinition {
           id: "critique-revise".into(),
           name: "Critique and Revise Loop".into(),
           steps: vec![
               StepDefinition { step_id: "generate".into(), depends_on: vec![], revision_source: None },
               StepDefinition { step_id: "critique".into(), depends_on: vec!["generate".into()], revision_source: None },
               StepDefinition { step_id: "revise".into(), depends_on: vec!["critique".into()], revision_source: Some("critique".into()) },
           ],
           transitions: vec![
               Transition::Sequential { from: "generate".into(), to: "critique".into() },
               Transition::RevisionLoop { reviser: "revise".into(), validator: "critique".into(), max_iterations: 3 },
           ],
       }
   }
   ```

2. Integrate with Restate workflow:
   - Orchestrator calls map to `ctx.run()` blocks
   - State stored in Restate K/V
   - Revision loops use Durable Promises
3. Add CLI command to run workflow:

   ```bash
   ecl run critique-revise --topic "Benefits of Rust"
   ```

4. Add CLI command to check status:

   ```bash
   ecl status <workflow-id>
   ```

5. Write comprehensive integration tests:
   - Happy path (generates, critiques, passes)
   - Revision path (generates, critiques, revises, passes)
   - Max revisions (generates, critiques, revises x3, fails)
   - Recovery (kill mid-execution, verify resume)

**Acceptance Criteria**:

- [ ] Complete workflow runs end-to-end
- [ ] CLI provides clear output
- [ ] All states persisted to database
- [ ] Logs show full execution trace

**Files to Create/Modify**:

```
ecl-workflows/src/
├── definitions/
│   └── critique_revise.rs

ecl-cli/src/
├── main.rs
├── commands/
│   ├── mod.rs
│   ├── run.rs
│   └── status.rs
```

---

## Phase 3: API & Tooling

**Objective**: Make ECL usable for real workflows
**Duration**: ~2 weeks
**Deliverable**: HTTP API, CLI improvements, configuration system, documentation

### Stage 3.1: HTTP API Design

**Goal**: Define RESTful API for workflow management.

**Tasks**:

1. Design API endpoints:

   ```
   POST   /workflows                    - Submit new workflow
   GET    /workflows                    - List workflows (with filtering)
   GET    /workflows/{id}               - Get workflow details
   GET    /workflows/{id}/steps         - Get step executions
   GET    /workflows/{id}/artifacts     - List workflow artifacts
   DELETE /workflows/{id}               - Cancel workflow

   POST   /workflows/{id}/signal        - Send signal to workflow

   GET    /definitions                  - List workflow definitions
   GET    /definitions/{id}             - Get definition details

   GET    /health                       - Health check
   GET    /metrics                      - Prometheus metrics
   ```

2. Define request/response schemas (OpenAPI spec)
3. Choose HTTP framework (recommend `axum` for Tokio ecosystem alignment)
4. Design error response format:

   ```json
   {
     "error": {
       "code": "WORKFLOW_NOT_FOUND",
       "message": "Workflow with ID xyz not found",
       "details": {}
     }
   }
   ```

5. Plan authentication approach (API key for v1, JWT for future)

**Acceptance Criteria**:

- [ ] OpenAPI spec is complete and valid
- [ ] All endpoints have clear semantics
- [ ] Error codes are documented
- [ ] Pagination design for list endpoints

**Files to Create**:

```
docs/
└── api/
    └── openapi.yaml

ecl-api/
├── Cargo.toml
└── src/
    └── lib.rs (placeholder)
```

---

### Stage 3.2: HTTP API Implementation

**Goal**: Implement the HTTP API using axum.

**Tasks**:

1. Create `ecl-api` crate
2. Implement router with all endpoints
3. Implement request handlers:

   ```rust
   async fn submit_workflow(
       State(app): State<AppState>,
       Json(request): Json<SubmitWorkflowRequest>,
   ) -> Result<Json<WorkflowResponse>, ApiError> { ... }
   ```

4. Add request validation with `validator` crate
5. Implement pagination for list endpoints:
   - Cursor-based pagination
   - Configurable page size (default 20, max 100)
6. Add API key authentication middleware:
   - Header: `X-API-Key`
   - Validate against config
7. Add request logging (correlate with workflow logs)
8. Add graceful shutdown handling

**Acceptance Criteria**:

- [ ] All endpoints return correct responses
- [ ] Validation errors return 400 with details
- [ ] Authentication works correctly
- [ ] Requests are traced

**Files to Create**:

```
ecl-api/src/
├── lib.rs
├── router.rs
├── handlers/
│   ├── mod.rs
│   ├── workflows.rs
│   ├── definitions.rs
│   └── health.rs
├── middleware/
│   ├── mod.rs
│   ├── auth.rs
│   └── logging.rs
├── error.rs
└── models/
    ├── mod.rs
    ├── requests.rs
    └── responses.rs
```

---

### Stage 3.3: Configuration System

**Goal**: Comprehensive configuration using confyg with multiple sources.

**Tasks**:

1. Define configuration structure:

   ```rust
   #[derive(Deserialize)]
   pub struct Config {
       pub server: ServerConfig,
       pub database: DatabaseConfig,
       pub llm: LlmConfig,
       pub restate: RestateConfig,
       pub logging: LoggingConfig,
   }

   #[derive(Deserialize)]
   pub struct LlmConfig {
       pub provider: String,  // "claude"
       pub api_key: Secret<String>,
       pub model: String,     // "claude-sonnet-4-20250514"
       pub max_tokens: u32,
       pub timeout_secs: u64,
       pub retry: RetryConfig,
   }
   ```

2. Configure confyg to load from (in priority order):
   1. Environment variables (highest priority)
   2. XDG dir
   3. `config/local.toml` (gitignored)
   4. `config/{environment}.toml`
   5. `config/default.toml`
3. Add `Secret<T>` wrapper that redacts in logs/debug
4. Validate configuration at startup:
   - Required fields present
   - URLs are valid
   - Numeric ranges are sensible
5. Create example configuration files
6. Add `--config` CLI flag to specify config path

**Acceptance Criteria**:

- [ ] Configuration loads correctly from all sources
- [ ] Environment variables override file config
- [ ] Secrets are never logged
- [ ] Invalid config produces helpful error messages

**Files to Create**:

```
ecl-core/src/
├── config/
│   ├── mod.rs
│   ├── types.rs
│   ├── loader.rs
│   └── validation.rs

config/
├── default.toml
├── development.toml
├── production.toml
└── local.toml.example
```

---

### Stage 3.4: CLI Enhancement

**Goal**: Full-featured CLI for workflow management.

**Tasks**:

1. Use `clap` with derive macros for CLI definition
2. Implement commands:

   ```
   ecl run <definition> [--input <json>] [--input-file <path>]
   ecl status <workflow-id> [--watch]
   ecl list [--state <state>] [--limit <n>]
   ecl cancel <workflow-id>
   ecl logs <workflow-id> [--step <step-id>] [--follow]
   ecl definitions list
   ecl definitions show <definition-id>
   ecl server start [--port <port>]
   ecl migrate [--target <version>]
   ```

3. Add output format options:
   - `--format json` - JSON output for scripting
   - `--format table` - Pretty table (default)
   - `--format quiet` - Minimal output (just IDs)
4. Implement `--watch` mode for status:
   - Poll every 2 seconds
   - Show step progression
   - Exit when workflow completes
5. Add shell completion generation:

   ```
   ecl completions bash > /etc/bash_completion.d/ecl
   ```

6. Create man page generation

**Acceptance Criteria**:

- [ ] All commands work correctly
- [ ] Help text is clear and complete
- [ ] Output formats work as expected
- [ ] Shell completion works

**Files to Modify/Create**:

```
ecl-cli/src/
├── main.rs
├── commands/
│   ├── mod.rs
│   ├── run.rs
│   ├── status.rs
│   ├── list.rs
│   ├── cancel.rs
│   ├── logs.rs
│   ├── definitions.rs
│   ├── server.rs
│   └── migrate.rs
├── output/
│   ├── mod.rs
│   ├── json.rs
│   ├── table.rs
│   └── quiet.rs
└── completions.rs
```

---

### Stage 3.5: Artifact Storage

**Goal**: File-based artifact storage with abstraction for future S3 support.

**Tasks**:

1. Define `ArtifactStore` trait:

   ```rust
   #[async_trait]
   pub trait ArtifactStore: Send + Sync {
       async fn store(&self, artifact: &Artifact, content: &[u8]) -> Result<ArtifactRef, StoreError>;
       async fn retrieve(&self, reference: &ArtifactRef) -> Result<Vec<u8>, StoreError>;
       async fn delete(&self, reference: &ArtifactRef) -> Result<(), StoreError>;
       async fn list(&self, workflow_id: &WorkflowId) -> Result<Vec<ArtifactRef>, StoreError>;
   }
   ```

2. Define `Artifact` struct:

   ```rust
   pub struct Artifact {
       pub id: Uuid,
       pub workflow_id: WorkflowId,
       pub step_id: StepId,
       pub name: String,
       pub content_type: String,
       pub size: u64,
       pub created_at: DateTime<Utc>,
   }
   ```

3. Implement `LocalArtifactStore`:
   - Base path from config
   - Directory structure: `{base}/{workflow_id}/{step_id}/{artifact_id}`
   - Metadata in database, content on filesystem
4. Add artifact support to `StepContext`:

   ```rust
   impl StepContext {
       pub async fn store_artifact(&self, name: &str, content: &[u8], content_type: &str) -> Result<ArtifactRef, StepError>;
       pub async fn retrieve_artifact(&self, reference: &ArtifactRef) -> Result<Vec<u8>, StepError>;
   }
   ```

5. Add API endpoints for artifact access:
   - `GET /workflows/{id}/artifacts/{artifact_id}` - Download artifact
   - `GET /workflows/{id}/artifacts/{artifact_id}/metadata` - Get metadata only

**Acceptance Criteria**:

- [ ] Artifacts store and retrieve correctly
- [ ] Metadata persists in database
- [ ] Content persists on filesystem
- [ ] API serves artifact downloads

**Files to Create**:

```
ecl-core/src/
├── artifacts/
│   ├── mod.rs
│   ├── store.rs       (ArtifactStore trait)
│   ├── local.rs       (LocalArtifactStore)
│   └── types.rs       (Artifact, ArtifactRef)
```

---

### Stage 3.6: Documentation

**Goal**: Comprehensive documentation for users and developers.

**Tasks**:

1. Create user documentation:
   - Getting Started guide
   - Installation instructions
   - Configuration reference
   - CLI reference
   - API reference
2. Create developer documentation:
   - Architecture overview
   - Creating custom steps
   - Creating custom workflows
   - Contributing guidelines
3. Add rustdoc comments to all public APIs
4. Generate API documentation:
   - `cargo doc` for Rust docs
   - OpenAPI spec rendered as HTML
5. Create example workflows:
   - Critique-Revise (toy)
   - Document summarizer (simple real-world)
   - Code review (complex)
6. Add inline code examples in documentation

**Acceptance Criteria**:

- [ ] All public APIs have rustdoc
- [ ] Getting Started works for new users
- [ ] Examples compile and run
- [ ] Documentation builds without warnings

**Files to Create**:

```
docs/
├── getting-started.md
├── installation.md
├── configuration.md
├── cli-reference.md
├── api-reference.md
├── architecture.md
├── creating-steps.md
├── creating-workflows.md
├── CONTRIBUTING.md
└── examples/
    ├── critique-revise.md
    ├── document-summarizer.md
    └── code-review.md
```

---

## Phase 4: Hardening

**Objective**: Production readiness
**Duration**: ~2 weeks
**Deliverable**: Production-ready v1 with comprehensive error handling, testing, and deployment documentation

### Stage 4.1: Error Handling Audit

**Goal**: Comprehensive error handling throughout the codebase.

**Tasks**:

1. Audit all `unwrap()` and `expect()` calls:
   - Replace with proper error handling
   - Add context to errors using `anyhow::Context`
2. Ensure all errors are:
   - Logged with appropriate level
   - Traceable to source
   - User-friendly in API responses
3. Add error codes for all API errors
4. Implement graceful degradation:
   - LLM timeout → retry with backoff
   - Database unavailable → queue requests
   - Restate unavailable → clear error message
5. Add panic handling:
   - Catch panics in async tasks
   - Log panic details
   - Prevent cascade failures
6. Review all `?` propagation for appropriate context

**Acceptance Criteria**:

- [ ] No `unwrap()` on fallible operations
- [ ] All errors have context
- [ ] API errors have consistent format
- [ ] Panics don't crash the server

**Files to Modify**: All source files (audit)

---

### Stage 4.2: Circuit Breaker for Claude API

**Goal**: Protect against Claude API failures and prevent cascade failures.

**Tasks**:

1. Implement circuit breaker using `failsafe`:

   ```rust
   pub struct LlmCircuitBreaker {
       circuit: CircuitBreaker<LlmError>,
   }

   impl LlmCircuitBreaker {
       pub async fn call<F, T>(&self, f: F) -> Result<T, LlmError>
       where
           F: Future<Output = Result<T, LlmError>>;
   }
   ```

2. Configure circuit breaker:
   - Failure threshold: 5 failures in 60 seconds
   - Open duration: 30 seconds
   - Half-open: Allow 1 request to test recovery
3. Add metrics for circuit state:
   - Circuit open/closed/half-open events
   - Failure counts
   - Recovery time
4. Wrap `ClaudeProvider` with circuit breaker
5. Add health check endpoint that reports circuit state
6. Configure alerts for circuit open (documentation)

**Acceptance Criteria**:

- [ ] Circuit opens after threshold failures
- [ ] Circuit recovers after timeout
- [ ] Metrics accurately reflect state
- [ ] Health check reports circuit state

**Files to Create**:

```
ecl-core/src/
├── resilience/
│   ├── mod.rs
│   └── circuit_breaker.rs
```

---

### Stage 4.3: Comprehensive Test Suite

**Goal**: High confidence in system correctness.

**Tasks**:

1. Unit tests for all modules:
   - Types and serialization
   - Validation logic
   - Configuration parsing
   - Error handling
2. Integration tests:
   - Full workflow execution
   - API endpoints
   - Database operations
   - Artifact storage
3. Property-based tests using `proptest`:
   - Serialization roundtrips
   - Validation edge cases
   - ID generation uniqueness
4. Chaos tests:
   - Kill process mid-workflow
   - Simulate LLM failures
   - Database disconnection
   - Network partitions (where applicable)
5. Performance tests:
   - Workflow throughput
   - API latency
   - Memory usage
   - Database query performance
6. Add test coverage reporting

**Acceptance Criteria**:

- [ ] >80% test coverage
- [ ] All tests pass reliably
- [ ] Chaos tests verify recovery
- [ ] Performance baselines established

**Files to Create**:

```
tests/
├── unit/
│   └── (per-module tests)
├── integration/
│   ├── api/
│   ├── workflows/
│   └── database/
├── property/
│   └── types.rs
├── chaos/
│   └── recovery.rs
└── performance/
    └── benchmarks.rs
```

---

### Stage 4.4: Performance Optimization

**Goal**: Ensure system performs well under expected load.

**Tasks**:

1. Profile with `cargo flamegraph`:
   - Identify hot paths
   - Find unnecessary allocations
   - Locate blocking operations
2. Optimize database queries:
   - Add indexes for common queries
   - Use batch operations where possible
   - Optimize JSON serialization
3. Optimize LLM calls:
   - Prompt caching (if supported)
   - Response streaming
   - Parallel step execution (where dependencies allow)
4. Add connection pooling:
   - Database connection pool sizing
   - HTTP client connection reuse
5. Memory optimization:
   - Avoid cloning large structures
   - Use `Arc` for shared data
   - Stream large artifacts
6. Document performance characteristics

**Acceptance Criteria**:

- [ ] Workflow latency meets targets
- [ ] Memory usage is bounded
- [ ] No blocking operations in async code
- [ ] Performance documentation complete

**Files to Modify**: Various (optimization)

---

### Stage 4.5: Security Review

**Goal**: Ensure system is secure for production use.

**Tasks**:

1. Audit authentication:
   - API key storage (not in logs)
   - API key validation
   - Rate limiting
2. Audit authorization:
   - Workflow access control
   - Artifact access control
3. Input validation:
   - All API inputs validated
   - SQL injection prevention (parameterized queries)
   - Path traversal prevention
4. Secret management:
   - Claude API key handling
   - Database credentials
   - Config file permissions
5. Dependency audit:
   - `cargo audit` for vulnerabilities
   - Review dependency licenses
6. Add security documentation:
   - Deployment security checklist
   - Secret rotation procedures

**Acceptance Criteria**:

- [ ] No secrets in logs
- [ ] All inputs validated
- [ ] `cargo audit` passes
- [ ] Security documentation complete

**Files to Create**:

```
docs/
└── security/
    ├── deployment-checklist.md
    └── secret-management.md
```

---

### Stage 4.6: Deployment Packaging

**Goal**: Easy deployment to production environments.

**Tasks**:

1. Create Dockerfile:

   ```dockerfile
   FROM rust:1.XX as builder
   # Build steps

   FROM debian:bookworm-slim
   # Runtime image
   ```

2. Create docker-compose for full stack:
   - ECL application
   - Restate server
   - PostgreSQL
3. Create Kubernetes manifests:
   - Deployment
   - Service
   - ConfigMap
   - Secret
   - PersistentVolumeClaim (for artifacts)
4. Add health check endpoints:
   - `/health/live` - Process is running
   - `/health/ready` - Dependencies available
5. Create deployment documentation:
   - Docker deployment
   - Kubernetes deployment
   - Manual deployment
6. Add environment-specific configs

**Acceptance Criteria**:

- [ ] Docker image builds and runs
- [ ] docker-compose stack works
- [ ] K8s manifests are valid
- [ ] Deployment docs are complete

**Files to Create**:

```
Dockerfile
docker-compose.yml
docker-compose.prod.yml

deploy/
├── kubernetes/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   └── pvc.yaml
└── docs/
    ├── docker.md
    ├── kubernetes.md
    └── manual.md
```

---

### Stage 4.7: Release Preparation

**Goal**: Prepare for v1.0 release.

**Tasks**:

1. Version all crates at 1.0.0
2. Create CHANGELOG.md with all changes
3. Update all documentation for release
4. Create release checklist:
   - [ ] All tests pass
   - [ ] Documentation complete
   - [ ] CHANGELOG updated
   - [ ] Version bumped
   - [ ] Docker image tagged
   - [ ] Release notes written
5. Tag release in git
6. Publish crates to crates.io (if public)
7. Create GitHub release with binaries
8. Announce release (internal/external)

**Acceptance Criteria**:

- [ ] All checklist items complete
- [ ] Release is tagged
- [ ] Artifacts are published
- [ ] Documentation matches release

**Files to Create**:

```
CHANGELOG.md
RELEASE.md (checklist)
```

---

## Phase 5: Target Workflow Implementation

**Objective**: Implement the Codebase Migration Analyzer workflow
**Duration**: ~3-4 weeks
**Deliverable**: Full 7-step production workflow demonstrating ECL's capabilities

*Note: This phase should begin after Phase 4, when the framework is production-ready. Detailed stage breakdown to be created based on learnings from Phases 1-4.*

### Stage 5.1: Intake Step

Implement Step 0: Validate and bundle inputs (code path, vision doc, configs).

### Stage 5.2: Codebase Analysis Step

Implement Step 1: Analyze existing codebase, produce grading and breakdown documents.

### Stage 5.3: Architecture Design Step

Implement Step 2: Review analysis, propose ideal language-agnostic architecture.

### Stage 5.4: Project Planning Step

Implement Step 3: Generate phased project plan from architecture doc.

### Stage 5.5: Implementation Spec Generation

Implement Step 4: Generate per-phase Claude Code-ready implementation docs.

### Stage 5.6: Quality Assessment Step

Implement Step 5: Assess generated code against standards and plan.

### Stage 5.7: Delivery Step

Implement Step 6: Generate final writeup, assemble deliverables.

### Stage 5.8: End-to-End Validation

Full workflow test with real codebase (e.g., toy Redis clone in JS → Rust).

---

## Appendices

### Appendix A: Dependency Versions

| Crate | Version | Purpose |
|-------|---------|---------|
| restate-sdk | 0.7.x | Workflow orchestration |
| tokio | 1.x | Async runtime |
| sqlx | 0.8.x | Database access |
| llm | 0.2.x | LLM abstraction |
| backon | 1.x | Retry logic |
| failsafe | 0.x | Circuit breaker |
| confyg | 0.3.x | Configuration |
| twyg | 0.6.x | Logging |
| axum | 0.7.x | HTTP API |
| clap | 4.x | CLI |
| serde | 1.x | Serialization |
| uuid | 1.x | ID generation |
| chrono | 0.4.x | Date/time |
| thiserror | 2.x | Error handling |
| anyhow | 1.x | Error context |

### Appendix B: Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ECL_ENV` | Environment (development/production) | development |
| `ECL_LOG_LEVEL` | Log level (trace/debug/info/warn/error) | info |
| `ECL_DATABASE_URL` | Database connection URL | sqlite://ecl.db |
| `ECL_CLAUDE_API_KEY` | Anthropic API key | (required) |
| `ECL_RESTATE_URL` | Restate server URL | <http://localhost:8080> |
| `ECL_ARTIFACT_PATH` | Artifact storage base path | ./artifacts |
| `ECL_API_PORT` | HTTP API port | 3000 |
| `ECL_API_KEY` | API authentication key | (required in prod) |

### Appendix C: Success Criteria Checklist

From the original proposal:

- [ ] **Durability**: Workflow survives process restart mid-execution
- [ ] **Feedback Loops**: Step N+1 can request revision from Step N with bounded iteration
- [ ] **Observability**: Full trace of all steps, LLM calls, and decisions
- [ ] **Clean Separation**: LLM provider swappable via trait implementation
- [ ] **Developer Experience**: Simple workflow definition in Rust code

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2026-01 | Claude | Initial project plan |
