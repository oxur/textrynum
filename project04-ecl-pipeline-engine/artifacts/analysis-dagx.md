
<!-- Historical source: workbench/analysis-dagx.md, lines 1-357. Original bytes are recoverable from import commit 3a39aea. -->

# Deep Analysis: dagx

**Crate:** [dagx](https://github.com/swaits/dagx)
**Version:** 0.3.1
**License:** MIT
**Author:** Stephen Waits

---

## 1. Overview

### What problem does it solve?

dagx is a minimal, type-safe, runtime-agnostic async DAG executor. It lets you define a directed acyclic graph of async tasks with typed inputs and outputs, wire dependencies between them, and execute them with maximum parallelism -- all with **compile-time cycle prevention** via Rust's type system.

### How mature is it?

- **Version:** 0.3.1 (pre-1.0, actively developed)
- **Commits:** ~69 on main
- **Stars:** ~16
- **Rust edition:** 2021, MSRV 1.81
- **Dependencies:** Minimal -- `futures-util` (core), `dagx-macros` (optional proc macro), `tracing` (optional)
- **Test infrastructure:** Comprehensive -- uses `tarpaulin` for coverage, `criterion` for benchmarks, `test-case` for parameterized tests, tested against Tokio, smol, pollster, async-executor, and futures-executor

This is a young but well-engineered crate. The code quality is high, the API is thoughtfully designed, and the documentation is thorough. It is not yet widely adopted.

### What's the core abstraction?

The core abstraction is a three-type system:

1. **`Task<Input>`** -- A trait defining a unit of async work with typed inputs and a typed output.
2. **`DagRunner`** -- The builder/executor that holds the graph topology and orchestrates execution.
3. **`TaskHandle<T>`** / **`TaskBuilder<Input, Tk>`** -- Type-state pair that enforces dependency wiring at compile time.

---

## 2. Trait Design

### Key traits

**`Task<Input>`** -- The fundamental work unit:

```rust
pub trait Task<Input>: Send
where
    Input: Send + Sync + 'static,
{
    type Output: Send + Sync + 'static;
    fn run(self, input: TaskInput<Input>) -> impl Future<Output = Self::Output> + Send;
}
```

Notable: `run` takes `self` by value (consuming the task), not `&mut self`. Each task executes exactly once. The `Input` type parameter is a tuple encoding all dependency types.

**`TaskWire<Input>`** -- Internal factory trait that determines the return type of `add_task()`:

```rust
pub trait TaskWire<Input>: Task<Input> + Sync + 'static
where
    Input: Send + Sync + 'static,
{
    type Retval<'dag>;
    fn new_from_dag<'dag>(id: NodeId, dag: &'dag mut DagRunner) -> Self::Retval<'dag>;
}
```

For `Input = ()`, `Retval` is `TaskHandle<Output>` (no dependencies needed).
For `Input = (A, B, ...)`, `Retval` is `TaskBuilder<(A, B, ...), Tk>` (must call `depends_on`).

**`ExecutableNode`** -- Internal type-erasure trait for heterogeneous storage:

```rust
pub(crate) trait ExecutableNode: Send {
    fn execute_with_deps(
        self: Box<Self>,
        dependencies: Vec<Arc<dyn Any + Send + Sync>>,
    ) -> ExecuteFuture;
}
```

**`DepsTuple<Input>`** -- Converts tuple-of-handles into node IDs:

```rust
pub(crate) trait DepsTuple<Input> {
    fn to_node_ids(self) -> Vec<NodeId>;
}
```

### How are they composed?

The trait hierarchy is clean and layered:

1. User implements `Task<Input>` (via `#[task]` macro)
2. `TaskWire<Input>` is blanket-implemented based on whether `Input` is `()` or a tuple
3. `TypedNode<Input, T>` wraps a `Task` and implements `ExecutableNode` for type erasure
4. `DepsTuple<Input>` is implemented for tuples of `&TaskHandle<T>` via macro

### Type parameter strategy

**Generics for the public API, type erasure internally.** The user-facing API is fully generic -- `TaskHandle<T>` carries the output type, `TaskBuilder<Input, Tk>` carries both the input tuple type and the task type. Internally, `ExecutableNode` uses `Box<dyn ExecutableNode>` and `Arc<dyn Any + Send + Sync>` for heterogeneous storage. Type erasure occurs at exactly one boundary (`TypedNode` -> `ExecutableNode`).

### Associated types

`Task::Output` is the key associated type. It flows through the entire system:
- `Task<Input>` defines `Output`
- `TaskWire` uses `Tk::Output` to parameterize `TaskHandle<Tk::Output>`
- `DagOutput::get<T>(handle: TaskHandle<T>) -> T` recovers the concrete type

---

## 3. DAG / Execution Model

### How are tasks/steps defined?

Tasks are Rust structs with a `#[task]`-annotated impl block containing a `run()` method. Three patterns:

1. **Stateless** (unit struct, no `self`): `struct Add; #[task] impl Add { async fn run(a: &i32, b: &i32) -> i32 { ... } }`
2. **Read-only** (`&self`): Struct holds config, `run(&self, ...)` reads it
3. **Mutable** (`&mut self`): Struct holds accumulator, `run(&mut self, ...)` modifies it

The `#[task]` macro:
- Renames the user's `run()` to `run_impl()`
- Generates a `Task<Input>` impl with extraction logic that calls `TaskInput::next()` for each parameter
- Derives the `Input` tuple type from parameter types (stripping the `&`)
- Derives `Output` from the return type

### How are dependencies expressed?

Via the `depends_on()` method on `TaskBuilder`, accepting either a single `&TaskHandle<T>` or a tuple of handles:

```rust
let sum = dag.add_task(Add).depends_on((&x, &y));
```

The tuple types must match the `Input` type of the task exactly. This is enforced at compile time through `DepsTuple<Input>` trait bounds.

### How is execution ordered?

**Topological layer sort.** `compute_layers()` performs Kahn's algorithm:

1. Compute in-degrees for all nodes
2. Find all zero-in-degree nodes (sources) as the first layer
3. Peel layers: execute a layer, decrement in-degrees of dependents, add newly-zero nodes to next layer
4. Repeat until all nodes are processed

### How is parallelism handled?

**Layer-based parallelism with an inline fast-path:**

- **Single-task layers:** Execute inline (no spawn overhead) -- 10-100x faster for sequential chains
- **Multi-task layers:** Spawn all tasks in the layer via the user-provided spawner function, collect results via `FuturesUnordered`

The spawner function is the key to runtime agnosticism:

```rust
dag.run(|fut| async move { tokio::spawn(fut).await.unwrap() }).await
// or
dag.run(|fut| smol::spawn(fut)).await
```

### How are results passed between steps?

**Arc-wrapped outputs stored in a HashMap.** When a task completes, its output is wrapped in `Arc<T>` and stored in `PassThroughHashMap<NodeId, Arc<dyn Any + Send + Sync>>`. When a downstream task executes, its dependencies are gathered by cloning the relevant `Arc`s. The `TaskInput::next()` method downcasts `Arc<dyn Any>` to `&T` for the task's `run()` method.

This is efficient for fan-out: one output shared by N consumers costs N `Arc::clone()` operations (pointer bump), not N data clones.

---

## 4. State Management

### Is there any checkpointing or persistence?

**No.** dagx has no checkpointing, persistence, or resume capability. Execution is all-or-nothing. If a task panics, execution aborts after the current layer with `DagError::TaskPanicked`.

### How is inter-step data passed?

Via `Arc<dyn Any + Send + Sync>` stored in a central HashMap, keyed by `NodeId`. Tasks receive references extracted from Arc via `TaskInput::next()`. The `TaskInput` type is a linear type -- each dependency must be consumed exactly once via successive `next()` calls.

### Is state serializable?

**No.** Outputs are `Arc<dyn Any>`, which is not serializable. There is no serialization infrastructure.

### How are errors handled and propagated?

- Panics are caught via `FutureExt::catch_unwind()` + `AssertUnwindSafe` and converted to `DagError::TaskPanicked { task_id, panic_message }`
- Execution aborts after the first layer containing an error
- `DagOutput::get()` panics if the handle is from a different `DagRunner` (not a `Result`)
- The error type is `#[non_exhaustive]`, allowing future extension
- There is no user-level error return from tasks -- tasks must return their `Output` type, not `Result<Output, E>`

---

## 5. Configuration

### How are pipelines configured?

**Code only.** There is no configuration file support, no declarative pipeline definition. The DAG is built entirely through Rust code using the builder pattern.

### Is there a builder pattern?

**Yes, with type-state.** The flow is:

```rust
let mut dag = DagRunner::new();           // Create
let handle = dag.add_task(MyTask::new()); // Add (returns TaskHandle or TaskBuilder)
let handle = builder.depends_on(&dep);    // Wire (consumes TaskBuilder, returns TaskHandle)
let output = dag.run(spawner).await?;     // Execute (consumes DagRunner)
let value = output.get(handle);           // Retrieve (consumes TaskHandle)
```

Every step consumes the previous state, making misuse a compile error.

### How flexible is the configuration?

Within the code-only constraint, it is quite flexible:
- Tasks can hold arbitrary state (config, connections, etc.)
- Up to 8 dependencies per task
- Any async runtime via spawner
- Fan-in, fan-out, diamond patterns all supported
- Custom types for inputs/outputs with no trait bounds beyond `Send + Sync + 'static`

---

## 6. Relevance to Our Five Pillars

### Durability

**Not addressed.** dagx has no checkpointing, no persistence, no resume. Execution is ephemeral. A crash means re-executing everything.

**What we can learn:** The clean separation between topology (edges/dependents) and execution (outputs) suggests a natural insertion point for checkpointing: serialize the `outputs` map after each layer completes.

**What we'd do differently:** Add a `Checkpoint` trait or layer-completion hook that serializes completed outputs. The layer-based execution model is actually ideal for checkpoint boundaries.

### Configurability

**Not addressed.** Pipelines are code-only. No TOML, no declarative config.

**What we can learn:** The type-state builder pattern is excellent for programmatic configuration. The `TaskWire` trait's approach of returning different types based on input arity is clever.

**What we'd do differently:** We need a declarative layer on top. Our Specification layer would produce the equivalent of `DagRunner::add_task()` calls from TOML. The code-level API could serve as the Topology layer.

### Observability

**Partially addressed.** Optional `tracing` integration behind a feature flag provides structured logging at INFO/DEBUG/TRACE levels. Task IDs, layer counts, and execution paths are logged.

**What we can learn:** The zero-cost tracing pattern (feature-gated `#[cfg(feature = "tracing")]`) is the right approach. The structured fields (`task_id`, `layer_count`, etc.) are well-chosen.

**What we'd do differently:** We need serializable execution records, not just log lines. A `RunRecord` struct capturing task outcomes, durations, and errors that can be persisted and queried.

### Incrementality

**Not addressed.** Every run executes all tasks from scratch. No content hashing, no change detection, no skip logic.

**What we can learn:** The `NodeId`-keyed output map could be extended with content hashes. Before executing a task, check if its inputs match a cached hash; if so, load the cached output instead.

**What we'd do differently:** Add an `IncrementalCache` trait that maps `(task_id, input_hash) -> cached_output`. Integrate it into the layer execution loop.

### Composability

**Strongly addressed.** The generic `Task<Input>` trait with tuple-based multi-dependency is highly composable. Tasks are independent units that compose via typed handles. Fan-in, fan-out, and diamond patterns are first-class.

**What we can learn:** The tuple-based dependency system is elegant and should be adopted. The approach of encoding dependency types in the `Input` parameter (rather than a separate trait) keeps the API surface small.

**What we'd do differently:** We may need dynamic composability (runtime-determined pipelines from config), which the compile-time approach doesn't support. We'd need a parallel dynamic dispatch path.

---

## 7. Extractable Concepts

### Priority 1: Type-State Builder Pattern

The `TaskBuilder` -> `TaskHandle` transition via `depends_on()` is the most transferable pattern. It enforces correct construction order at compile time with zero runtime cost.

**Specifically:**
- `add_task()` returns different types based on whether the task needs dependencies (`TaskWire` trait)
- `depends_on()` consumes the builder and returns an immutable handle
- The handle carries the output type for downstream wiring

**Rationale:** This prevents an entire class of runtime errors (missing dependencies, cycles) at compile time.

### Priority 2: Tuple-Based Multi-Dependency

The `DepsTuple` trait with macro-generated impls for tuples of 1-8 elements is elegant. It allows `depends_on((&a, &b, &c))` where each handle can have a different output type, and the compiler verifies type alignment with the task's input signature.

**Rationale:** This is more ergonomic than builder chains (`depends_on(a).depends_on(b)`) and more type-safe than `Vec<Handle>`.

### Priority 3: Layer-Based Execution with Inline Fast-Path

The topological layer computation + single-task inline execution optimization is simple and effective. Layers are natural parallelism boundaries and natural checkpoint boundaries.

**Rationale:** Layer boundaries are the perfect place to insert our checkpointing, incrementality checks, and observability hooks.

### Priority 4: Runtime-Agnostic Spawner

The `spawner: S` closure parameter that the user provides to `run()` is a clean way to decouple the executor from the DAG logic. No dependency on Tokio, smol, or any specific runtime.

**Rationale:** Our CLI tool should work with any runtime. This pattern achieves that with minimal API surface.

### Priority 5: Arc-Wrapped Output Sharing

Automatic `Arc<T>` wrapping of task outputs for efficient fan-out sharing. Users return `T`, the framework handles sharing. `Arc::into_inner()` recovers owned values when only one consumer exists.

**Rationale:** Transparent to the user, efficient for fan-out, and the `Arc::into_inner` optimization for final retrieval is clever.

### Priority 6: Proc Macro for Ergonomics

The `#[task]` macro eliminates boilerplate by deriving `Input`/`Output` types from the `run()` signature and generating extraction logic. The generated code renames `run` to `run_impl` and wraps it with `TaskInput::next()` calls.

**Rationale:** Good DX, but we should evaluate whether the complexity of a proc macro is justified for our use case. A simpler derive macro or manual impl may suffice if our task signatures are more uniform.

### Priority 7: Linear TaskInput Type

`TaskInput` uses a linear consumption pattern -- each `next()` call returns the value and a new `TaskInput` for remaining dependencies. This prevents accidentally skipping or double-reading a dependency.

**Rationale:** Elegant type-level enforcement of correct input consumption order.

---

## 8. Anti-Patterns / Warnings

### No user-level error returns from tasks

Tasks must return `Output`, not `Result<Output, E>`. The only error path is panicking, which is caught and converted to `DagError::TaskPanicked`. This means error handling must be done inside the task (returning a `Result` as the output type), which pushes error handling to consumers.

**Our approach:** We should support `Result<T, E>` as a first-class task output with automatic DAG-level error propagation. A task returning `Err` should optionally abort downstream dependents without requiring panics.

### Type erasure via `dyn Any` internally

While the public API is type-safe, internal storage uses `Arc<dyn Any + Send + Sync>` with runtime downcasting. This means:
- A mismatched `TaskHandle` from a different `DagRunner` causes a panic (not a compile error)
- The type safety guarantee holds only within a single `DagRunner` instance

**Our approach:** This is acceptable for the internal implementation but we should ensure our error messages are clear when cross-runner handle misuse occurs. Consider a generation/epoch ID on handles.

### No dynamic/runtime pipeline construction

The compile-time type-state pattern means pipelines must be fully known at compile time. You cannot build a DAG from a config file at runtime using this approach, because the types must be resolved statically.

**Our approach:** We need both. The type-state pattern for the programmatic Topology layer, but a parallel dynamic dispatch path for the Specification layer (config-driven). This likely means a `DynTask` trait object variant alongside the generic `Task<Input>`.

### Consuming `DagRunner` on `run()`

`DagRunner::run(self)` takes ownership, so you cannot re-run a DAG. This is intentional (tasks are consumed during execution) but means no reuse.

**Our approach:** For incremental/repeated execution, we need a design where tasks are factories (producing executable instances) rather than being consumed themselves. Consider `TaskFactory` -> `TaskInstance` separation.

### No task naming or identification

Tasks are identified only by `NodeId(u32)`. There are no string names, labels, or metadata. This makes debugging and observability harder.

**Our approach:** Add a `name: &'static str` or similar metadata to tasks for logging, checkpointing keys, and human-readable state inspection.

### 8-dependency limit

The macro-generated tuple impls cap at 8 dependencies. While the docs suggest this is sufficient, our pipeline stages may need more inputs (e.g., a merge stage combining many sources).

**Our approach:** Support the tuple pattern for common cases but also provide a `Vec`-based dynamic dependency path for high-arity cases.
