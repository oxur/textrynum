# Deep Analysis: erio-workflow

## 1. Overview

### What problem does it solve?

erio-workflow provides a DAG-based workflow engine for orchestrating async steps
with dependencies, primarily targeting "plan-then-execute style agents" in AI
agent runtimes. It sequences steps based on a dependency graph, runs independent
steps in parallel, passes typed outputs between steps, and supports
checkpoint/resume to survive crashes.

### How mature is it?

- **Version:** 0.1.0 (pre-release)
- **Activity:** Active development; part of the erio monorepo
  (github.com/NomanworkGroup/erio) with seven crates
- **Rust edition:** 2024, minimum Rust 1.93.0
- **License:** Apache-2.0
- **Downloads:** Not yet published to crates.io (no evidence of publication)
- **Test coverage:** Comprehensive unit tests including property-based testing
  with proptest for the DAG module

The crate is young but well-structured. Code quality is high with thorough doc
comments, clear module boundaries, and extensive tests.

### What's the core abstraction?

A **Workflow** is a validated collection of **Steps** arranged in a **Dag**.
The **WorkflowEngine** resolves the DAG into parallel groups and executes them,
threading a **WorkflowContext** (a typed key-value store of step outputs)
between steps. A **Checkpoint** can persist progress to disk as JSON for
crash recovery.

---

## 2. Trait Design

### Key traits

There is exactly one trait in the crate:

```rust
#[async_trait::async_trait]
pub trait Step: Send + Sync {
    fn id(&self) -> &str;
    async fn execute(&self, ctx: &mut WorkflowContext)
        -> Result<StepOutput, WorkflowError>;
}
```

This is the sole extension point. Everything else is concrete types.

### Composition

- `ConditionalStep<S: Step>` wraps any `Step` with a runtime predicate,
  producing `StepOutput::skipped()` when the condition is false.
- The `Workflow` stores steps as `Arc<dyn Step>`, using trait objects for
  heterogeneous step collections.
- `WorkflowBuilder` accepts `impl Step + 'static`, boxing and arc-wrapping
  internally.

### Type parameter strategy

**Trait objects for storage, generics for construction.**

- `WorkflowBuilder::step()` takes `impl Step + 'static` (generic, zero-cost at
  the call site)
- `Workflow.steps` stores `HashMap<String, Arc<dyn Step>>` (dynamic dispatch
  for heterogeneous collections)
- `ConditionalStep<S: Step>` uses a generic parameter for the inner step
  (preserving the concrete type) but is itself erased to `dyn Step` when added
  to a workflow

### Associated types

None. The `Step` trait has no associated types. All steps produce the same
`StepOutput` type. This is a deliberate simplification: outputs are
string-valued with optional JSON metadata, not generically typed.

---

## 3. DAG / Execution Model

### How are steps defined?

Steps implement the `Step` trait. Each has a string ID and an async `execute`
method that receives a mutable reference to `WorkflowContext`.

`StepOutput` is the universal return type:

```rust
pub struct StepOutput {
    value: String,
    metadata: Option<serde_json::Value>,
    skipped: bool,
}
```

### How are dependencies expressed?

Dependencies are declared at build time as string ID references:

```rust
Workflow::builder()
    .step(StepA::new("a", "val"), &[])         // no deps
    .step(StepB::new("b", "val"), &["a"])       // depends on "a"
    .step(StepC::new("c", "val"), &["a", "b"])  // depends on "a" and "b"
    .build()?;
```

The builder validates at `build()` time that:
1. No duplicate step IDs
2. All dependency IDs refer to previously added steps
3. The graph is acyclic (via topological sort)

### How is execution ordered?

**Kahn's algorithm** (BFS-based topological sort) with deterministic ordering
via insertion-order tracking. The `Dag` maintains both a `HashMap<String,
Vec<String>>` for dependencies and a `Vec<String>` for insertion order,
ensuring reproducible results.

### How is parallelism handled?

`Dag::parallel_groups()` computes execution levels: each level contains nodes
whose dependencies are all in previous levels. The engine then:

- **Single-step groups:** Executed directly (no spawn overhead)
- **Multi-step groups:** Each step is `tokio::spawn`ed; results are collected
  via join handles

For parallel execution, each spawned task receives a **cloned snapshot** of the
context (not a shared reference), which means parallel steps cannot see each
other's outputs — only outputs from prior groups. After all tasks in a group
complete, outputs are merged back into the shared context.

### How are results passed between steps?

Via `WorkflowContext`, a `HashMap<String, StepOutput>`. Steps read prior outputs
by step ID:

```rust
async fn execute(&self, ctx: &mut WorkflowContext) -> Result<StepOutput, WorkflowError> {
    let prev = ctx.output("prior_step").unwrap();
    // use prev.value(), prev.metadata()
    Ok(StepOutput::new("result"))
}
```

---

## 4. State Management

### Checkpointing and persistence

This is the standout feature. The `Checkpoint` struct:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Checkpoint {
    completed: HashMap<String, StepOutput>,
}
```

Key methods:
- `mark_completed(step_id, output)` — records a step's completion
- `save(path)` — async JSON write via `tokio::fs::write`
- `load(path)` — async JSON read via `tokio::fs::read_to_string`
- `into_context()` — converts to a `WorkflowContext` for resumption

The engine has a dedicated `run_with_checkpoint()` method that:

1. Loads an existing checkpoint file if present
2. Converts it to a `WorkflowContext` (restoring prior outputs)
3. For each parallel group, filters out already-completed steps
4. After each group completes, saves the checkpoint to disk
5. On error, saves the checkpoint before returning the error

This provides **group-level granularity** — if step 3 of 5 in a group fails,
steps 1-2 are checkpointed and won't re-execute on retry.

### Inter-step data passing

`WorkflowContext` is a `HashMap<String, StepOutput>`. It is:
- Cloned for parallel step execution (snapshot isolation)
- Mutably borrowed for sequential execution
- Serializable (through `Checkpoint` which stores the same data)

### Serialization

`StepOutput` derives `Serialize, Deserialize`. `Checkpoint` derives
`Serialize, Deserialize`. State is persisted as pretty-printed JSON.

### Error handling

`WorkflowError` is a `thiserror` enum with variants:
- `CycleDetected { steps }` — DAG validation
- `StepNotFound { step_id }` — lookup failure
- `DuplicateStep { step_id }` — builder validation
- `StepFailed { step_id, message }` — runtime step failure
- `DependencyFailed { step_id, dependency_id }` — cascade (defined but not used
  in current engine — see Anti-Patterns)
- `MissingDependency { step_id, dependency_id }` — builder validation
- `EmptyWorkflow` — builder validation
- `Checkpoint { message }` — I/O or serde failures

Error propagation: first failure in a group aborts the workflow. The engine does
not currently support partial failure / continue-on-error semantics.

---

## 5. Configuration

### How are pipelines configured?

**Code-only.** There is no TOML/YAML/JSON configuration for workflow
definitions. Workflows are constructed programmatically via the builder pattern:

```rust
let workflow = Workflow::builder()
    .step(step_a, &[])
    .step(step_b, &["a"])
    .build()?;
```

### Builder pattern

The `WorkflowBuilder` is a classic consuming builder:
- `step(impl Step + 'static, &[&str])` — adds a step with dependencies,
  returns `self` for chaining
- `build()` — validates and produces a `Workflow`

The builder validates eagerly: dependencies must refer to steps already added
(forward references are not supported).

### Flexibility

Limited to what can be expressed in code. No runtime reconfiguration, no
conditional topology (the `ConditionalStep` controls execution but not graph
structure), no parameterized templates.

---

## 6. Relevance to Our Five Pillars

### Durability

**Directly addressed.** The checkpoint module is the primary value of this
crate for our purposes.

- **What we can learn:** Group-level checkpointing after each parallel batch is
  a practical granularity. The `Checkpoint` <-> `WorkflowContext` conversion
  pattern (one for persistence, one for runtime) cleanly separates concerns.
  Saving on error (before returning) ensures no work is lost.

- **What we'd do differently:**
  - Use a more structured format than flat JSON (e.g., include timestamps,
    run IDs, schema version for forward compatibility)
  - Add step-level checkpointing within groups, not just group-level
  - Support pluggable storage backends (not just local filesystem)
  - Include a content hash or version field for incrementality checks

### Configurability

**Not addressed.** Workflows are code-only. No declarative configuration.

- **What we can learn:** The builder pattern is clean and validates eagerly.
  The dependency-as-string-id approach is simple and works.

- **What we'd do differently:** We need TOML-based declarative pipeline
  definitions. The builder pattern is useful as the *internal* API that a
  config loader targets, but we need a config layer on top.

### Observability

**Minimally addressed.** `StepOutput` has optional JSON metadata. The
checkpoint file is human-readable JSON. But there are no events, no logging
hooks, no timing data, no structured trace output.

- **What we can learn:** The metadata field on `StepOutput` is a reasonable
  escape hatch for step-level diagnostics.

- **What we'd do differently:**
  - Emit structured events (step started, completed, failed, skipped)
  - Record wall-clock timing per step
  - Include the full execution trace in the checkpoint/state file
  - Make state inspectable by AI (structured, with context about failures)

### Incrementality

**Not addressed.** There is no content hashing, no comparison of inputs across
runs, no "skip if unchanged" logic. The checkpoint only tracks
completed/not-completed, not whether inputs have changed.

- **What we can learn:** The checkpoint's `is_completed()` check is a
  degenerate case of incrementality (completed = skip). We can extend this
  pattern with input hashing.

- **What we'd do differently:** Add input fingerprints to checkpoint entries.
  A step is re-executed if its input hash differs from the checkpoint, even if
  it was previously "completed."

### Composability

**Partially addressed.** Steps are trait objects, so different step types can
be mixed freely. `ConditionalStep` adds runtime branching. The DAG structure
inherently supports mix-and-match topologies.

- **What we can learn:** The `ConditionalStep` wrapper pattern is elegant —
  it produces `StepOutput::skipped()` so downstream steps still see it as
  "completed" (avoiding dependency failures for optional steps). Generic
  wrapper `ConditionalStep<S: Step>` preserves the inner type while itself
  implementing `Step`.

- **What we'd do differently:**
  - Support per-source pipeline definitions where each source type gets a
    different sequence of steps
  - Allow dynamic topology construction from config, not just code
  - Consider step factories or registries for plugin-style composition

---

## 7. Extractable Concepts

### Priority 1 (High Value)

1. **Checkpoint <-> Context separation pattern.** The `Checkpoint` (persistence)
   and `WorkflowContext` (runtime) are separate types with explicit conversion
   (`into_context()`). This cleanly separates what's serialized from what's used
   at runtime. We should adopt this pattern for our State layer.

2. **Group-level checkpoint persistence.** Saving after each parallel group
   completes (and on error before returning) is the right granularity for our
   use case. It balances durability against I/O overhead.

3. **DAG parallel_groups() computation.** The level-based grouping algorithm
   (compute each node's level as max(dependency levels) + 1, then group by
   level) is simple, correct, and efficient. We should adopt this directly.

4. **Kahn's algorithm with insertion-order determinism.** Maintaining a
   separate insertion-order vector alongside the dependency map ensures
   reproducible execution order across runs. This matters for debugging and
   for checkpoint compatibility.

5. **Snapshot isolation for parallel steps.** Cloning the context before
   spawning parallel tasks, then merging results after all complete, is a
   clean concurrency model. It avoids shared mutable state during execution
   while ensuring all parallel steps see the same inputs.

### Priority 2 (Moderate Value)

6. **ConditionalStep wrapper pattern.** A generic wrapper that produces
   `StepOutput::skipped()` on false conditions, keeping the step "completed"
   in the DAG. This avoids cascade failures for optional processing stages.

7. **Eager builder validation.** Checking for duplicate IDs, missing
   dependencies, and cycles at `build()` time (not at execution time) catches
   configuration errors early. The "dependencies must already exist" constraint
   (no forward references) simplifies validation.

8. **StepOutput with metadata escape hatch.** A string value plus optional
   `serde_json::Value` metadata is a pragmatic choice: simple for common cases,
   extensible for complex ones. The `skipped` flag is also useful.

9. **Single-step group optimization.** Running single-step groups without
   `tokio::spawn` avoids unnecessary task overhead. A small but practical
   optimization.

### Priority 3 (Reference Value)

10. **`WorkflowError` enum design.** Clean `thiserror` enum with structured
    variants carrying context (step IDs, dependency IDs, messages). The
    `Checkpoint` variant wraps I/O and serde errors with descriptive messages.

11. **Property-based testing for DAG correctness.** Using proptest to generate
    random acyclic and cyclic graphs validates invariants across hundreds of
    cases. Worth adopting for our own DAG implementation.

---

## 8. Anti-Patterns / Warnings

### AP-1: String-typed outputs (no type safety)

`StepOutput.value` is always a `String`. Steps that need to pass structured
data must serialize to string and deserialize on the other end. This loses
compile-time type safety and adds runtime serialization overhead. The
`metadata: Option<serde_json::Value>` mitigates this somewhat but is still
dynamically typed.

**Our approach:** Consider a `Box<dyn Any + Send + Sync>` context store with
typed accessors (like `TypeMap`), or use `serde_json::Value` as the primary
value type instead of `String`. The key insight is that for a CLI pipeline
runner, we likely want `serde_json::Value` or `Vec<u8>` as the interchange
format, not `String`.

### AP-2: No retry support in the workflow engine

Despite `erio-core` having a well-designed `RetryConfig` with exponential
backoff, the workflow engine has no retry integration. A failed step immediately
aborts the workflow. For our use case (network calls, API rate limits), built-in
retry at the engine level is essential.

**Our approach:** Integrate retry configuration per-step, with the engine
handling retry loops and backoff. The checkpoint should distinguish
"failed after retries" from "not yet attempted."

### AP-3: Unused error variants

`WorkflowError::DependencyFailed` is defined but never constructed in the
engine code. The `StepNotFound` variant is similarly unused (the engine uses
`.expect()` instead). This suggests the error model was designed speculatively
rather than driven by actual error paths.

**Our approach:** Define error variants only when they have concrete producers
and handlers. Every variant should be constructable and matchable in real code
paths.

### AP-4: Mutable context reference in the Step trait

```rust
async fn execute(&self, ctx: &mut WorkflowContext) -> Result<StepOutput, WorkflowError>;
```

Passing `&mut WorkflowContext` to steps means a step could modify outputs from
prior steps (mutating shared state). The engine mitigates this for parallel
steps by cloning, but sequential steps have full mutable access. This is a
footgun.

**Our approach:** Pass an immutable context snapshot plus a separate output
channel, or pass `&WorkflowContext` (immutable) and have steps return their
output without direct context mutation.

### AP-5: No timeout support

Steps can run forever with no timeout mechanism. For a CLI pipeline runner,
this is unacceptable.

**Our approach:** Add per-step timeout configuration, enforced by the engine
via `tokio::time::timeout`.

### AP-6: Checkpoint file is not atomic

`tokio::fs::write` is not atomic — a crash during write could corrupt the
checkpoint. For durability, writes should use write-to-temp-then-rename.

**Our approach:** Use atomic file writes (write to `.tmp`, then `fs::rename`)
for checkpoint persistence. Consider also keeping the previous checkpoint as
a backup.

### AP-7: No checkpoint versioning or schema evolution

The checkpoint JSON has no version field. If `StepOutput` fields change between
releases, old checkpoints become unreadable with no migration path.

**Our approach:** Include a schema version in the checkpoint file. Implement
forward-compatible deserialization or explicit migration logic.

### AP-8: `Arc<Mutex<WorkflowContext>>` for parallel execution

The engine wraps the context in `Arc<Mutex<_>>` and clones the entire context
for each parallel step. For workflows with many steps and large outputs, this
could be expensive. The lock is held during the clone, creating a serialization
point.

**Our approach:** Consider a lock-free approach where each group receives an
immutable `Arc<WorkflowContext>` snapshot taken once before the group starts,
and outputs are collected into a `Vec` without mutex contention.
