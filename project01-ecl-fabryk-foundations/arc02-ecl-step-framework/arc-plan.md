# ECL Phase 2 — Step abstraction framework

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 462-950. Original bytes are recoverable from import commit 3a39aea. -->

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


<!-- Historical source: crates/design/dev/0002-phase-2-00-overview.md, lines 1-244. Original bytes are recoverable from import commit 3a39aea. -->

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

## Reconstructed slice locations

- [Stage 2.1: Step Trait Definition](slice01-step-trait-definition/slice-plan.md)
- [Stage 2.2: Step Executor](slice02-step-executor/slice-plan.md)
- [Stage 2.3: Toy Workflow Step Implementations](slice03-toy-workflow-step-implementations/slice-plan.md)
- [Stage 2.4: Workflow Orchestrator](slice04-workflow-orchestrator/slice-plan.md)
- [Stage 2.5: SQLx Persistence Layer](slice05-sqlx-persistence-layer/slice-plan.md)
- [Stage 2.6: Observability Infrastructure](slice06-observability-infrastructure/slice-plan.md)
- [Stage 2.7: Toy Workflow Integration](slice07-toy-workflow-integration/slice-plan.md)

## Operator disposition — 2026-11-21

Cancelled as prospective work in favor of project06-pipeline. Preserve this historical record; do not issue or resume its old assignments.
