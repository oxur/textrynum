
<!-- Historical source: workbench/analysis-oxigdal-workflow.md, lines 1-414. Original bytes are recoverable from import commit 3a39aea. -->

# Analysis: oxigdal-workflow

**Crate:** `oxigdal-workflow` v0.1.1
**Repository:** https://github.com/cool-japan/oxigdal
**License:** Apache-2.0
**Published:** 2026-03-11 (v0.1.1), 2026-02-22 (v0.1.0)
**Source size:** 759.77 KB | 100% doc coverage (1,162 items)
**Analysis date:** 2026-03-13
**Analysis basis:** docs.rs API documentation and GitHub README (source not cloned; full API surface analyzed from published docs)

---

## 1. Overview

### What problem does it solve?

oxigdal-workflow is a DAG-based workflow engine designed for orchestrating complex geospatial processing pipelines. It is part of the larger OxiGDAL project -- a pure-Rust geospatial data abstraction library spanning 69 workspace crates and ~480K lines of Rust. The workflow crate provides:

- DAG construction, validation, and execution planning
- Parallel task execution with resource-aware scheduling
- Checkpoint-based state persistence and crash recovery
- Flexible scheduling (cron, interval, event-driven)
- Conditional branching and loop support
- Monitoring, debugging, and visualization
- Optional integrations (Kafka, HTTP, Airflow/Prefect/Temporal connectors)
- Optional RESTful API server (Axum)

### How mature is it?

**Very early.** v0.1.1 was published 2 days ago (2026-03-11). The parent repository has 43 stars and 3 forks. The crate has only two published versions (0.1.0 and 0.1.1). Despite this, the API surface is remarkably broad -- 1,162 documented items across 9 modules -- which raises questions about whether breadth has outpaced depth and real-world hardening.

The project claims 7,486 passing tests across the entire OxiGDAL workspace, but it is unclear how many target the workflow crate specifically.

### What's the core abstraction?

A `WorkflowDag` containing `TaskNode` instances connected by `TaskEdge` dependencies, executed by a `WorkflowExecutor` parameterized over a `TaskExecutor` trait. State is tracked via `WorkflowState` and persisted through `StatePersistence` with versioned `WorkflowCheckpoint` support.

---

## 2. Trait Design

### Key traits

The crate's primary extension point is a single trait:

```rust
// TaskExecutor -- the only trait users must implement
pub trait TaskExecutor: Send + Sync {
    async fn execute(
        &self,
        task: &TaskNode,
        context: &ExecutionContext,
    ) -> Result<TaskOutput>;
}
```

(Rendered via `async-trait` as a pinned boxed future in the actual signature.)

### Composition strategy

The design uses **generics on the executor** and **concrete types everywhere else**:

- `WorkflowExecutor<E: TaskExecutor>` -- generic over the task execution implementation
- `WorkflowDag`, `TaskNode`, `WorkflowState`, `WorkflowCheckpoint` -- all concrete structs
- `TaskNode.config` is `serde_json::Value` -- untyped JSON for task-specific configuration
- `TaskOutput.data` is `Option<Value>` -- untyped JSON for task results

### Type parameter strategy

**Minimal generics, heavy use of concrete types with JSON escape hatches.** The only generic boundary is `E: TaskExecutor` on `WorkflowExecutor`. All inter-task data passing goes through `serde_json::Value`, which sacrifices compile-time type safety for flexibility. There are no associated types on `TaskExecutor`.

### Assessment

This is a pragmatic but unambitious trait design. A single trait with a single method is easy to implement but pushes all domain logic into the implementor. The `Value`-typed config and output mean the framework cannot validate or typecheck data flow between tasks at compile time.

---

## 3. DAG / Execution Model

### Task/step definition

Tasks are defined as `TaskNode` structs:

```rust
pub struct TaskNode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub config: Value,            // JSON blob -- task-specific configuration
    pub retry: RetryPolicy,
    pub timeout_secs: Option<u64>,
    pub resources: ResourceRequirements,
    pub metadata: HashMap<String, String>,
}
```

### Dependency expression

Dependencies are edges in a `petgraph`-backed directed graph:

```rust
dag.add_task(task_node)?;
dag.add_dependency("task_a", "task_b", edge)?;
```

Edges are typed (`TaskEdge` with `EdgeType`), and the DAG supports querying dependencies, dependents, transitive closures, and subgraph extraction.

### Execution ordering

The `dag` module exports:
- `topological_sort()` -- standard topological ordering
- `create_execution_plan()` -- produces an `ExecutionPlan` (ordered task list)
- `critical_path()` -- identifies the longest dependency chain
- `create_parallel_schedule()` / `optimize_schedule()` -- groups tasks into `ExecutionWave` instances for parallel execution

The parallel scheduler produces a `ParallelSchedule` composed of `ExecutionWave` layers, where each wave contains tasks whose dependencies are satisfied and that can run concurrently within resource constraints.

### Parallelism

Controlled by `ExecutorConfig.max_concurrent_tasks` and constrained by `ResourcePool` / `ResourceRequirements`:

```rust
pub struct ResourceRequirements {
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub gpu: bool,
    pub disk_mb: u64,
    pub custom: HashMap<String, f64>,
}
```

The scheduler respects resource availability when forming execution waves, tracked via `AvailableResources` and `ResourceUtilization`.

### Inter-step data passing

Via `TaskOutput`:

```rust
pub struct TaskOutput {
    pub data: Option<Value>,  // JSON output
    pub logs: Vec<String>,    // execution logs
}
```

Results are stored in `WorkflowState.task_states` (a `HashMap<String, TaskState>`) and accessible via the `ExecutionContext` passed to subsequent task executions. All data passes through `serde_json::Value` -- no typed channels or generics.

---

## 4. State Management

### Checkpointing and persistence

This is the crate's strongest feature and most relevant to our use case.

**StatePersistence** provides filesystem-based state storage:

```rust
pub struct StatePersistence { /* private fields */ }

impl StatePersistence {
    pub fn new(state_dir: String) -> Self;

    // State CRUD
    pub async fn save(&self, state: &WorkflowState) -> Result<()>;
    pub async fn load(&self, execution_id: &str) -> Result<WorkflowState>;
    pub async fn delete(&self, execution_id: &str) -> Result<()>;
    pub async fn list(&self) -> Result<Vec<String>>;

    // Checkpoint management (versioned)
    pub async fn save_checkpoint(&self, checkpoint: &WorkflowCheckpoint) -> Result<()>;
    pub async fn load_checkpoint(&self, execution_id: &str) -> Result<WorkflowCheckpoint>;
    pub async fn load_checkpoint_by_sequence(&self, execution_id: &str, sequence: u64) -> Result<WorkflowCheckpoint>;
    pub async fn delete_checkpoint(&self, execution_id: &str, sequence: u64) -> Result<()>;
    pub async fn delete_all_checkpoints(&self, execution_id: &str) -> Result<()>;
    pub async fn list_checkpoints(&self, execution_id: &str) -> Result<Vec<u64>>;
    pub fn checkpoint_exists(&self, execution_id: &str) -> bool;
}
```

**WorkflowCheckpoint** bundles state and DAG for full recovery:

```rust
pub struct WorkflowCheckpoint {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub sequence: u64,          // incrementing counter
    pub state: WorkflowState,
    pub dag: WorkflowDag,
}

impl WorkflowCheckpoint {
    pub fn get_pending_tasks() -> ...;
    pub fn get_completed_tasks() -> ...;
    pub fn get_failed_tasks() -> ...;
    pub fn get_interrupted_tasks() -> ...;
    pub fn get_ready_tasks() -> ...;
    pub fn are_dependencies_satisfied() -> ...;
    pub fn prepare_for_resume() -> ...;   // resets interrupted tasks
}
```

**WorkflowState** tracks per-task progress:

```rust
pub struct WorkflowState {
    pub workflow_id: String,
    pub execution_id: String,
    pub status: WorkflowStatus,
    pub task_states: HashMap<String, TaskState>,
    pub metadata: WorkflowMetadata,
    pub context: ExecutionContext,
}

impl WorkflowState {
    // Lifecycle transitions
    pub fn start(&mut self);
    pub fn complete(&mut self);
    pub fn fail(&mut self);
    pub fn cancel(&mut self);

    // Task-level transitions
    pub fn init_task(&mut self, ...);
    pub fn start_task(&mut self, ...);
    pub fn complete_task(&mut self, ...);
    pub fn fail_task(&mut self, ...);
    pub fn skip_task(&mut self, ...);
    pub fn add_task_log(&mut self, ...);

    // Variable sharing
    pub fn set_variable(&mut self, ...);
    pub fn get_variable(&self, ...) -> ...;

    pub fn is_terminal(&self) -> bool;
}
```

### Recovery

`WorkflowExecutor` supports resume:

```rust
pub async fn resume(&self, execution_id: String) -> Result<WorkflowState>;
pub async fn resume_from_sequence(&self, execution_id: String, sequence: u64) -> Result<WorkflowState>;
pub async fn get_recovery_info(&self, execution_id: &str) -> Result<RecoveryInfo>;
pub async fn cleanup_checkpoints(&self, execution_id: &str, keep_count: usize) -> Result<usize>;
```

Checkpointing frequency is controlled by `ExecutorConfig.checkpoint_interval` (save every N tasks).

### Serializability

All core types (`WorkflowDag`, `TaskNode`, `WorkflowState`, `WorkflowCheckpoint`, `RetryPolicy`, `ResourceRequirements`) derive `Serialize` and `Deserialize`. The entire workflow graph and execution state can be serialized to disk and restored.

### Error handling

Two main error enums (`WorkflowError`, `DagError`) with a `Result<T>` type alias. The `thiserror` crate is used for derivation. Specific variant definitions were not available in the docs pages inspected.

---

## 5. Configuration

### How are pipelines configured?

**Code-first with JSON config blobs.** Pipelines are built programmatically:

```rust
let mut dag = WorkflowDag::new();
dag.add_task(TaskNode { id: "fetch".into(), config: json!({...}), ... })?;
dag.add_dependency("fetch", "transform", edge)?;

let config = ExecutorConfig {
    max_concurrent_tasks: 4,
    enable_persistence: true,
    state_dir: "/tmp/state".into(),
    checkpoint_interval: 5,
    enable_checkpointing: true,
    ..Default::default()
};

let executor = WorkflowExecutor::new(config, my_task_executor);
let state = executor.execute("wf-1".into(), "exec-1".into(), dag).await?;
```

There is no declarative config file format (no TOML/YAML pipeline definitions). The `toml` dependency exists but appears to be used internally rather than for user-facing pipeline configuration.

### Builder pattern

No explicit builder pattern is visible. Types use public fields and `Default` derives, enabling struct literal construction.

### Templates

The `templates` module provides `WorkflowTemplate` and `WorkflowTemplateLibrary` for reusable workflow definitions, though the exact API surface was not deeply inspected.

---

## 6. Relevance to Our Five Pillars

### Durability

**Strong.** This is where oxigdal-workflow shines relative to our needs:

- Versioned checkpoints with sequence numbers enable point-in-time recovery
- `WorkflowCheckpoint` bundles both state and DAG, so recovery is self-contained
- `prepare_for_resume()` handles the tricky problem of resetting interrupted tasks
- `checkpoint_interval` controls persistence frequency (every N tasks)
- `resume()` and `resume_from_sequence()` provide flexible recovery entry points
- `cleanup_checkpoints()` manages storage growth

**What we can learn:** The checkpoint design is well-thought-out. Bundling the DAG with state ensures recovery doesn't depend on the original configuration being unchanged. Versioned sequences allow rollback. The `prepare_for_resume()` method that resets interrupted tasks to a retryable state is a pattern worth adopting.

**What we'd do differently:** We'd use TOML-serialized state files (not JSON) for human readability/editability. We'd separate checkpoint storage from execution, making the persistence layer pluggable rather than hardcoded to filesystem. We'd also want content-hash-based change detection rather than just sequence-based checkpointing.

### Configurability

**Weak for our purposes.** oxigdal-workflow is code-first. Pipeline definition requires Rust code -- there's no declarative TOML/YAML configuration for defining pipelines, stages, or per-source parameters.

**What we can learn:** The `ExecutorConfig` struct with clear fields like `checkpoint_interval`, `enable_checkpointing`, `enable_persistence`, `stop_on_failure` is a good model for runtime configuration.

**What we'd do differently:** Our entire pipeline definition must be TOML-driven, with the three-layer architecture (Specification -> Topology -> State). This is a fundamental departure from oxigdal-workflow's approach.

### Observability

**Moderate.** The `monitoring` module is broad:
- `WorkflowMetrics` and `MetricsCollector` for metrics
- `ExecutionHistory` and `TaskExecutionRecord` for history
- `PerformanceMetrics` and `BottleneckAnalysis` for profiling
- `DagVisualizer` with multiple `GraphFormat` options
- `WorkflowLogger` with `LogEntry` and `LogLevel`
- A `debugging` submodule with `Debugger`, `DebugSession`, `DebugInfo`

**What we can learn:** The separation of metrics, history, and visualization is clean. The bottleneck analysis feature (identifying resource constraints) is interesting for pipeline optimization.

**What we'd do differently:** Our observability must be centered on serializable state files that humans or AI can inspect. We want "grep the state directory to understand what happened" rather than a metrics collection API. The monitoring module here is runtime-centric, not post-mortem-centric.

### Incrementality

**Absent.** There is no content hashing, no change detection, no mechanism to skip unchanged items across runs. The crate's checkpointing is about crash recovery, not about skipping work that's already been done in a previous successful run.

**What we'd do differently:** This is a gap we must fill ourselves. Content hashing, timestamp comparison, and output caching are core to our incrementality requirement.

### Composability

**Moderate.** The DAG model inherently supports composability:
- Tasks can be mixed and matched via dependency edges
- `EdgeType` allows typed dependencies
- Conditional branching (`ConditionalBranch`, `Expression`, `ExecutionDecision`) enables dynamic workflow paths
- Subgraph extraction allows isolating parts of a pipeline
- Templates provide reusable workflow fragments

**What we can learn:** The conditional execution model (Expression evaluation -> ExecutionDecision) is a useful pattern for per-source stage selection. The subgraph extraction capability is relevant if we want to run partial pipelines.

**What we'd do differently:** Our composability is TOML-driven: per-source stage lists, not programmatic graph construction. We'd want to define which stages apply to which source types declaratively.

---

## 7. Extractable Concepts

**Priority 1 -- High value, directly applicable:**

1. **Checkpoint design pattern.** The `WorkflowCheckpoint` struct bundling state + DAG + sequence number + timestamp is an excellent pattern. Adopt this for `ecl-state`: a checkpoint should be a self-contained recovery artifact, not a diff or partial state.

2. **`prepare_for_resume()` semantics.** The idea of a method that inspects a checkpoint and resets interrupted/in-progress tasks to a retryable state is crucial for crash recovery. This separates "loading state" from "preparing state for re-execution."

3. **Checkpoint interval configuration.** `checkpoint_interval: usize` (save every N tasks) is a simple, effective control for balancing durability against I/O overhead. Consider adopting this alongside time-based intervals.

4. **`RetryPolicy` struct.** The four-field design (max_attempts, delay_ms, backoff_multiplier, max_delay_ms) is minimal and complete for exponential backoff. Directly adoptable.

5. **`ResourceRequirements` with custom resources.** The `custom: HashMap<String, f64>` field is a good extensibility mechanism for resource types the framework doesn't anticipate. Consider for our adapter resource declarations.

**Priority 2 -- Worth considering:**

6. **Execution waves for parallel scheduling.** The `ExecutionWave` concept (grouping dependency-satisfied tasks into concurrent batches) is a clean mental model for parallel execution, more intuitive than raw topological sort.

7. **`RecoveryInfo` as a recovery diagnostic.** Having a dedicated struct that summarizes recovery state (what completed, what failed, what's resumable) before actually resuming is good UX for CLI tools.

8. **Task state machine.** The explicit lifecycle methods (`init_task`, `start_task`, `complete_task`, `fail_task`, `skip_task`) on `WorkflowState` enforce valid state transitions. Consider a similar state machine for our pipeline stages.

9. **DAG validation as a separate step.** `dag.validate()` as an explicit pre-execution check is a good pattern -- fail fast before any work begins.

10. **Checkpoint cleanup.** `cleanup_checkpoints(keep_count)` prevents unbounded storage growth. Essential for long-running systems.

**Priority 3 -- Background influence:**

11. **Separation of DAG structure from execution engine.** The `dag` module handles graph construction and planning; the `engine` module handles execution. This separation of concerns is sound.

12. **Critical path analysis.** `critical_path()` could be useful for estimating pipeline completion time or identifying optimization targets.

---

## 8. Anti-Patterns / Warnings

1. **Untyped data flow (`serde_json::Value` everywhere).** `TaskNode.config` and `TaskOutput.data` are both `Value`. This means no compile-time validation of inter-task data contracts. For our use case, we should define typed adapter inputs/outputs and validate at the Topology layer, not at runtime via JSON.

2. **Scope creep / kitchen-sink API.** For a v0.1.1 crate, the breadth is suspicious: DAG execution, scheduling (cron/interval/event), conditional branching, loop support, monitoring with bottleneck analysis, DAG visualization, debugging sessions, workflow templates, workflow versioning, and integrations with Airflow/Prefect/Temporal/Kafka. This is ~760KB of source at version 0.1.1. The risk is that many of these features are thin facades rather than battle-tested implementations. **We should mine concepts but not assume robustness.**

3. **No declarative configuration.** Code-only pipeline definition is a non-starter for our use case. This is a design disagreement, not a bug -- they're building a library for Rust developers, we're building a tool for pipeline operators.

4. **No incrementality support.** The checkpoint system is for crash recovery only. There's no content hashing, no "skip if unchanged" mechanism. If we adopted this pattern wholesale, we'd still have to build incrementality from scratch.

5. **Filesystem-only persistence.** `StatePersistence` takes a `state_dir: String` and writes to disk. There's no trait abstraction for the persistence backend. For our needs, the persistence layer should be trait-based so we could (eventually) swap in SQLite or other backends.

6. **Public fields on all structs.** `TaskNode`, `WorkflowState`, `ExecutorConfig`, `RetryPolicy`, etc., all use `pub` fields. This prevents future invariant enforcement. For our crate, builder patterns or constructor validation would be safer for types where field relationships matter.

7. **Async everywhere.** All persistence operations are `async`. For a CLI tool writing to local files, sync I/O is simpler and avoids the async runtime dependency. Consider whether our checkpoint operations need to be async.

8. **Heavy dependency tree.** 29 dependencies including `crossbeam`, `dashmap`, `parking_lot`, `futures`, `petgraph`, `cron`, `regex`, `uuid`, `ahash`, plus optional `axum`, `rdkafka`, `reqwest`. For a lightweight CLI pipeline runner, this is heavier than necessary. We should be selective about which patterns we adopt versus which dependencies we accept.

---

## Summary

oxigdal-workflow is an ambitious, very new (2 weeks old) DAG workflow engine with a remarkably broad API surface. Its strongest contribution to our design thinking is the **checkpoint/recovery pattern**: versioned checkpoints bundling state + DAG, explicit resume semantics, and the `prepare_for_resume()` concept for handling interrupted tasks. The `RetryPolicy` and `ResourceRequirements` designs are clean and directly adoptable.

Its weaknesses relative to our needs are the lack of declarative configuration (code-only), absence of incrementality, untyped data flow, and the sheer breadth of its API at such an early version. We should treat it as a **concept mine** for durability patterns, not as a reference implementation for overall architecture.
