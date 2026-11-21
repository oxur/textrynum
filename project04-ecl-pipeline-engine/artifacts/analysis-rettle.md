
<!-- Historical source: workbench/analysis-rettle.md, lines 1-344. Original bytes are recoverable from import commit 2d006a7. -->

# Deep Analysis: rettle

## 1. Overview

### What problem does it solve?

rettle is a multithreaded ETL (Extract, Transform, Load) framework for Rust, drawing explicit inspiration from Keras's sequential model paradigm. It provides a declarative, builder-style API for defining data pipelines where "Tea" objects (user-defined data structs) flow through a sequence of processing stages: Fill (extract), Steep (transform), Skim (field removal), and Pour (load). Work is distributed across a threadpool of "Brewers" managed by a "Brewery."

### How mature is it?

- **Version:** 2.0.0 (current on crates.io)
- **Activity:** Very low. The CHANGELOG only documents up to 1.0.3 plus staged changes. The repository has 144 total commits, 25 stars, 0 forks. The `Transfuse` and `Skim` ingredient types are still marked "Not Implemented Yet" despite being in the API surface.
- **Edition:** Rust 2018
- **Dependencies:** Only `serde` and `serde_json` (minimal footprint)
- **Community crates:** `cstea` (CSV), `elastictea` (Elasticsearch), `logtea` (log files) -- all by the same author.

This is effectively an abandoned proof-of-concept. The v2.0 bump introduced generics (replacing a `Tea` trait object approach from v1), but the project appears to have stalled shortly after.

### What's the core abstraction?

A **Pot** holds a list of **Sources** (Fill ingredients that produce data) and a **Recipe** (an ordered Vec of Ingredient trait objects -- Steep, Skim, Pour). The Pot's `brew()` method iterates over sources, each of which generates batches of typed "Tea" structs and dispatches them to a **Brewery** (threadpool). Each batch is processed sequentially through the recipe steps by a **Brewer** (worker thread).

---

## 2. Trait Design

### Key Traits

**`Ingredient<T: Send>`** -- The core pipeline step trait:

```rust
pub trait Ingredient<T: Send> {
    fn exec(&self, tea_batch: Vec<T>) -> Vec<T>;
    fn print(&self);
    fn as_any(&self) -> &dyn Any;
    fn get_name(&self) -> &str;
}
```

**`Source<T: Send>`** -- The data extraction trait:

```rust
pub trait Source<T: Send> {
    fn collect(&self, brewery: &Brewery, recipe: Arc<RwLock<Vec<Box<dyn Ingredient<T> + Send + Sync>>>>);
    fn as_any(&self) -> &dyn Any;
    fn print(&self);
    fn get_name(&self) -> &str;
    fn get_source(&self) -> &str;
}
```

**`Argument`** -- Optional parameter passing trait:

```rust
pub trait Argument {
    fn as_any(&self) -> &dyn Any;
}
```

**`FnBox`** -- Internal trait for calling boxed closures (a workaround for pre-stabilized `Box<dyn FnOnce>`):

```rust
trait FnBox {
    fn call_box(self: Box<Self>);
}
```

### How are they composed?

Composition is linear only. The `Pot` stores:
- `sources: Vec<Box<dyn Source<T>>>` -- fill operations
- `recipe: Arc<RwLock<Vec<Box<dyn Ingredient<T> + Send + Sync>>>>` -- transform/load steps

Sources are iterated sequentially. Each source generates batches and pushes them into the Brewery's channel. The recipe is shared (via `Arc<RwLock>`) across all worker threads, which read-lock it to process each batch.

### Type parameter strategy

rettle v2 uses **generics** -- `Pot<T>`, `Ingredient<T>`, etc. are all parameterized by the user's "Tea" type `T: Send`. However, the `Argument` trait uses **trait objects with `Any` downcasting**:

```rust
pub struct Steep<T: Send> {
    pub name: String,
    pub computation: Box<fn(Vec<T>, &Option<Box<dyn Argument + Send>>) -> Vec<T>>,
    pub params: Option<Box<dyn Argument + Send>>,
}
```

Parameters are retrieved via `downcast_ref::<ConcreteArgs>()`, which is type-unsafe at runtime. There are no associated types.

### Unsafe markers

The crate manually implements `Send` and `Sync` for `Steep<T>`, `Skim<T>`, and `Pour<T>`:

```rust
unsafe impl<T: Send> Send for Steep<T> {}
unsafe impl<T: Send> Sync for Steep<T> {}
```

This is needed because the structs contain `Box<fn(...)>` with trait object parameters that the compiler cannot automatically prove are Send/Sync.

---

## 3. DAG / Execution Model

### How are tasks/steps defined?

Steps are concrete structs (`Fill`, `Steep`, `Skim`, `Pour`) that each hold:
- A `name: String`
- A `computation: Box<fn(...)>` -- a boxed function pointer
- An optional `params: Option<Box<dyn Argument + Send>>`

Fill additionally holds a `source: String` identifier.

### How are dependencies expressed?

**There is no DAG.** The pipeline is strictly **linear/sequential**. Steps execute in the order they are added to the Pot. There is no way to express branching, conditional execution, fan-out, or fan-in.

### How is execution ordered?

1. `Pot::brew()` iterates over sources sequentially
2. Each source's `collect()` method generates batches of `Vec<T>` and sends them to the Brewery via `take_order()`
3. Each order is a closure that calls `make_tea(batch, recipe)`
4. `make_tea()` iterates the recipe Vec in order, calling `exec()` on each step
5. Step dispatch uses `Any` downcasting in a chain of `if let Some(steep) = step.as_any().downcast_ref::<Steep<T>>()` checks

### How is parallelism handled?

The **Brewery** is a classic threadpool implementation:
- An MPSC channel distributes work to N Brewer threads
- Each Brewer loops on `receiver.lock().unwrap().recv()` (shared `Arc<Mutex<Receiver>>`)
- Batches are processed independently and in parallel -- no cross-batch coordination
- Graceful shutdown via `Drop`: sends N `Terminate` messages, then `join()`s all threads

Parallelism is **batch-level only**. Within a batch, steps run sequentially. Multiple batches from the same or different sources can run concurrently on different Brewers.

### How are results passed between steps?

Within `make_tea()`, the `tea_batch: Vec<T>` is threaded through each step:

```rust
for step in recipe.iter() {
    if let Some(steep) = step.as_any().downcast_ref::<Steep<T>>() {
        tea_batch = steep.exec(tea_batch);
    } else if let Some(skim) = step.as_any().downcast_ref::<Skim<T>>() {
        tea_batch = skim.exec(tea_batch);
    } else if let Some(pour) = step.as_any().downcast_ref::<Pour<T>>() {
        tea_batch = pour.exec(tea_batch);
    }
}
```

Each step takes ownership of the Vec and returns a new Vec. The batch is the unit of data flow.

---

## 4. State Management

### Is there any checkpointing or persistence?

**No.** There is zero checkpointing, no state persistence, no crash recovery. If the process dies, all work is lost. There is no concept of a "run" that can be resumed.

### How is inter-step data passed?

The `Vec<T>` batch flows through steps by ownership transfer. Steps transform the batch and return a new Vec. There is no shared context, no side-channel, and no typed output mapping between steps. The only inter-step communication is the batch itself.

For the Argument/params mechanism, parameters are set at construction time and are immutable during execution. They are accessed via `&Option<Box<dyn Argument + Send>>` references.

### Is state serializable?

The Tea type parameter `T` has no `Serialize` bound in the library itself, though the Cargo.toml includes `serde` as a dependency and the example derives `Serialize, Deserialize` on the Tea struct. The library does not serialize or deserialize any state internally.

### How are errors handled and propagated?

**Poorly.** The codebase uses `.unwrap()` pervasively:
- `recipe.read().unwrap()` in `make_tea()`
- `receiver.lock().unwrap().recv().unwrap()` in Brewer
- `self.sender.send(...).unwrap()` in `take_order()`
- `downcast_ref::<T>().unwrap()` for argument extraction

There is no `Result` propagation, no error types, and no recovery mechanism. A panic in any step will poison the mutex/rwlock and cascade failures. A commented-out test in pot.rs notes "TODO: Readd test after returning Result" -- this was never implemented.

---

## 5. Configuration

### How are pipelines configured?

**Code only.** There are no configuration files, no TOML/YAML/JSON parsing, no declarative pipeline definitions. Everything is constructed programmatically using struct literals and the builder-style `add_source()` / `add_ingredient()` methods on `Pot`.

### Is there a builder pattern?

A lightweight one. `Pot` methods consume and return `self` for chaining:

```rust
let new_pot = Pot::new()
    .add_source(Box::new(Fill { ... }))
    .add_ingredient(Box::new(Steep { ... }))
    .add_ingredient(Box::new(Pour { ... }));
```

However, `Fill`, `Steep`, `Pour`, and `Skim` are constructed via struct literal syntax with public fields -- there are no builders for the ingredients themselves.

### How flexible is the configuration?

Limited. The user controls:
- Number of Brewer threads (Brewery::new(size))
- Order of ingredients in the recipe
- Which sources to add
- Runtime parameters via the Argument trait

The batch size is determined by the Fill computation (hardcoded in user code). There is no way to configure retry behavior, timeouts, logging levels, or any operational concerns.

---

## 6. Relevance to Our Five Pillars

### Durability

**Not addressed.** No checkpointing, no persistence, no crash recovery. If the process terminates, all in-flight batches are lost and the pipeline must restart from scratch.

**What we can learn:** The batch-level processing model naturally creates checkpoint boundaries. Each batch completion is a potential checkpoint event. We could wrap the `make_tea()` equivalent with a checkpoint-on-completion hook.

**What we'd do differently:** Introduce a `RunState` that tracks which batches from which sources have been processed, serializable to disk. On restart, skip completed batches.

### Configurability

**Minimally addressed.** Configuration is code-only. The Argument trait provides runtime parameterization, but there is no declarative config layer.

**What we can learn:** The separation of "computation" (the function) from "params" (the Argument) is a useful pattern -- it separates the algorithm from its configuration. The community crate pattern (cstea, elastictea) shows how Fill/Pour can be parameterized for specific backends.

**What we'd do differently:** Define ingredients in TOML with typed parameter schemas. The computation functions would be registered by name, with parameters deserialized from config rather than constructed in code.

### Observability

**Barely addressed.** The only observability is `println!` statements and an elapsed-time metric printed on Brewery drop. There are no structured logs, no metrics, no tracing, no visibility into batch progress or step timing.

**What we can learn:** The `start_time` / elapsed pattern shows awareness of the need for timing. The `get_name()` on every ingredient provides a hook for labeling.

**What we'd do differently:** Emit structured events (step started/completed, batch processed, errors encountered) with timing, batch IDs, and source attribution. Make state queryable.

### Incrementality

**Not addressed.** Every run processes all data from all sources from scratch. There is no content hashing, no change detection, no "last processed" markers.

**What we can learn:** Nothing directly. However, the source-level iteration in `brew()` provides a natural place to inject incrementality checks -- per-source, per-batch skip logic.

**What we'd do differently:** Add content hashing or timestamp-based change detection at the source level. Store processed-item hashes in the run state.

### Composability

**Partially addressed.** The ingredient model is composable in a limited way: you can add any combination of Steep/Skim/Pour steps in any order to a recipe, and you can have multiple sources. The community crate pattern (cstea, elastictea, logtea) shows how sources and sinks can be packaged as reusable components.

**What we can learn:** The Fill/Steep/Pour taxonomy is a clean mental model. The community crate pattern -- where each adapter is a separate crate providing pre-built Fill and Pour ingredients -- is a good extensibility model.

**What we'd do differently:** Support per-source recipes (not all sources need the same transformation chain). Allow branching/conditional steps. Define adapter interfaces that are more structured than raw function pointers.

---

## 7. Extractable Concepts

### Priority 1: Fill / Transform / Pour Taxonomy

The three-phase ETL categorization (Fill=extract, Steep=transform, Pour=load) maps cleanly to pipeline stages. Even though rettle's implementation is simplistic, the naming and conceptual separation is sound. We should adopt a similar categorization for our adapter traits, though we might prefer more standard names (Extract/Transform/Load or Source/Transform/Sink).

### Priority 2: Batch-Oriented Processing

The model of processing data in batches (Vec<T>) rather than individual items is practical for throughput and provides natural checkpoint boundaries. Each batch completion is an atomic unit of progress. We should adopt batch processing as our primary execution unit.

### Priority 3: Community Adapter Crate Pattern

The pattern of separate crates (cstea, elastictea, logtea) that provide pre-built Fill/Pour implementations for specific backends is a clean extensibility model. Each adapter crate wraps the complexity of a specific data source/sink. We should design our adapter trait surface to enable this pattern for Google Drive, Slack, Granola, etc.

### Priority 4: Computation + Parameters Separation

The separation of `computation` (the function pointer) from `params` (the Argument struct) is useful -- it means the same transformation logic can be reused with different parameters. In our design, this maps to configurable pipeline steps where the algorithm is code but the parameters come from TOML.

### Priority 5: Threadpool for Batch Parallelism

The Brewery/Brewer threadpool with MPSC channel distribution is a clean, simple model for batch-level parallelism. While we'd use `tokio` or `rayon` rather than hand-rolling a threadpool, the architectural pattern -- distribute independent batches across workers while keeping per-batch step execution sequential -- is sound.

---

## 8. Anti-Patterns / Warnings

### AP-1: Pervasive `unwrap()` and No Error Handling

The codebase uses `.unwrap()` everywhere with zero `Result` propagation. Any failure (lock poisoning, downcast failure, channel send failure) causes a panic that can cascade through the threadpool. This is the single biggest design flaw.

**Our approach:** Every step must return `Result<Vec<T>, StepError>`. Errors must propagate cleanly and be recorded in run state.

### AP-2: `Any` Downcasting for Type Dispatch

The `make_tea()` function uses a chain of `downcast_ref` calls to determine which ingredient type to execute. This loses compile-time type safety and is fragile -- if a new ingredient type is added and the chain isn't updated, it silently does nothing.

```rust
if let Some(steep) = step.as_any().downcast_ref::<Steep<T>>() {
    tea_batch = steep.exec(tea_batch);
} else if let Some(skim) = step.as_any().downcast_ref::<Skim<T>>() {
    ...
```

**Our approach:** Use an enum dispatch or trait-based polymorphism where the compiler enforces exhaustive handling.

### AP-3: `unsafe impl Send/Sync`

The manual `unsafe impl Send for Steep<T>` markers are a red flag. They exist because `Box<fn(..., &Option<Box<dyn Argument + Send>>)>` contains a reference to a trait object that the compiler can't automatically prove is Send. A better design would make the computation type provably Send without unsafe.

**Our approach:** Design computation types that are automatically Send+Sync, potentially using generic closures with appropriate bounds rather than function pointers with trait object parameters.

### AP-4: Single Recipe for All Sources

All sources share the same recipe (the same `Arc<RwLock<Vec<...>>>`). There is no way to have source-specific transformation chains. Every document from every source goes through the same sequence of steps.

**Our approach:** Per-source pipeline definitions. A Google Drive document and a Slack message should not necessarily go through the same transformation chain.

### AP-5: Fill Computation Has Too Much Responsibility

The Fill computation is a function that receives the Brewery and recipe directly, and is responsible for batching, iteration, and dispatching work:

```rust
pub computation: Box<fn(&Option<Box<dyn Argument + Send>>, &Brewery, Arc<RwLock<Vec<Box<dyn Ingredient<T> + Send + Sync>>>>)>,
```

This means every Fill implementation must manually manage batching and work distribution. This is an abstraction leak -- the orchestration concern should be separated from the data extraction concern.

**Our approach:** Sources should only be responsible for producing items (or batches). Batching strategy and work distribution should be handled by the framework.

### AP-6: No Backpressure

The Brewery's MPSC channel is unbounded. A fast Fill source can flood the channel with batches faster than Brewers can process them, leading to unbounded memory growth. The example creates 5,000+ batches of 200 items each with no flow control.

**Our approach:** Use bounded channels or explicit backpressure mechanisms to prevent memory exhaustion.

### AP-7: Metaphor Over Clarity

The tea-brewing metaphor (Pot, Brewery, Brewer, Fill, Steep, Pour, Tea) is charming but adds cognitive overhead. When discussing the system, you must constantly translate between domain concepts (ETL) and the metaphor. "The Brewer reads the Recipe from the Pot and steeps the Tea" means "the worker thread reads the pipeline steps and transforms the data batch."

**Our approach:** Use domain-standard terminology. Source, Transform, Sink (or Extract, Transform, Load) are universally understood. Metaphors are fine for marketing but not for API surfaces.

### AP-8: Stale / Incomplete Implementation

`Transfuse` and `Skim` are declared but marked "Not Implemented Yet." `Skim` has struct and trait implementations but is functionally identical to `Steep` (both take `Vec<T>` and return `Vec<T>` -- there is no actual field-removal semantics). This suggests the abstraction boundaries were not fully thought through.

**Our approach:** Only expose what is implemented. Design trait signatures that enforce semantic differences between step types.
