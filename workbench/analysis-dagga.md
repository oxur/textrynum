# Deep Analysis: dagga

## 1. Overview

### What problem does it solve?

dagga solves the problem of **scheduling directed acyclic graphs where dependencies are implicit via resource access patterns** rather than explicit task-to-task edges. Instead of declaring "task A must run before task B," you declare that task A *creates* resource X and task B *reads* resource X -- the scheduler infers the ordering automatically.

Given a set of nodes that declare which resources they create, read, write, or consume (move), dagga computes a valid parallel execution schedule organized into sequential batches where tasks within each batch can run concurrently without violating resource access constraints.

### How mature is it?

- **Version:** 0.2.3 (released October 15, 2025)
- **Total downloads:** ~42,500
- **Recent downloads:** ~2,800
- **Commits:** 29 total; last code commit March 2023, packaging update October 2025
- **Stars:** 30, Forks: 2
- **Size:** ~600 lines in `src/lib.rs` plus a `src/dot.rs` visualization module. Very compact.
- **Dependencies:** Minimal -- `rustc-hash`, `snafu`, `log`; optional `dot2` and `serde`
- **Author:** schell (Schell Carl Scivally), known in the Rust gamedev community (also author of `renderling`)
- **Maturity assessment:** Stable but minimal. Small, focused utility crate. Not widely adopted but used in the author's own rendering/game engine projects. The API is well-defined and unlikely to change significantly.

### What's the core abstraction?

The core abstraction is a **resource-access-annotated node** (`Node<N, E>`) combined with a **constraint satisfaction solver**. Nodes declare their relationship to typed resources via four access patterns (result/read/write/move). A `Dag<N, E>` collects nodes, derives pairwise ordering constraints from resource declarations, and uses the **AC-3 arc consistency algorithm** to assign each node to a batch index, producing a `Schedule<T>` of parallel batches.

---

## 2. Trait Design

### Key types (dagga defines no user-facing traits)

dagga is notably **trait-free** in its public API. The entire API is built on concrete generic structs with trait bounds on type parameters. There is no `Task` trait, no `Executable` trait, no `Resource` trait. This is a deliberate design choice -- the library's job is purely to *schedule*, not to *execute*.

### Core type signatures

**Node -- the task wrapper:**

```rust
pub struct Node<N, E> {
    node: N,                          // opaque user payload
    name: String,                     // human-readable identifier
    barrier: usize,                   // barrier group
    moves: FxHashSet<E>,              // resources consumed (ownership transfer)
    reads: FxHashSet<E>,              // resources borrowed immutably
    writes: FxHashSet<E>,             // resources borrowed mutably/exclusively
    results: FxHashSet<E>,            // resources created/produced
    run_before: FxHashSet<String>,    // explicit ordering constraints
    run_after: FxHashSet<String>,     // explicit ordering constraints
}
```

**Dag -- the graph builder:**

```rust
pub struct Dag<N, E> {
    barrier: usize,
    requires_root_nodes: bool,
    nodes: Vec<Node<N, E>>,
}
```

**Schedule -- the output:**

```rust
pub struct Schedule<T> {
    pub batches: Vec<Vec<T>>,
}
```

**Constraint -- the internal ordering primitive:**

```rust
pub struct Constraint {
    pub lhs: String,
    pub rhs: String,
    pub op: Op,                       // Gt, Lt, or Ne
    pub reasons: Vec<RequirementReason>,
}
```

### Type parameter strategy

- **`N`** -- the node payload type. Completely opaque to dagga. Can be `()`, a function pointer, a system descriptor, etc. No bounds required for scheduling.
- **`E`** -- the resource identifier type. Must be `Copy + PartialEq + Eq + Hash`. Commonly `usize`, `&'static str`, or `std::any::TypeId`.

This is a **generics-based** strategy with no trait objects and no enums. The library is parametric over both the task type and the resource identifier type.

### Associated types

None in the public API. The only trait implementations with associated types are internal: `dot2::Labeller` and `dot2::GraphWalk` in the visualization module.

### How are types composed?

No trait-based composition. The API is a **builder pattern** on concrete types. `Node` provides fluent methods (`with_read`, `with_write`, `with_move`, `with_result`, `with_name`, `run_before`, `run_after`). `Dag` provides `with_node`, `with_barrier`. `Schedule` provides `map` for transforming node types.

---

## 3. DAG / Execution Model

### How are tasks/steps defined?

Tasks are `Node<N, E>` instances wrapping a user payload `N`. The node declares its resource access patterns:

```rust
let create_a = Node::new(())
    .with_name("create-a")
    .with_result(a);           // creates resource `a`

let modify_a = Node::new(())
    .with_name("modify-a")
    .with_write(a)             // exclusive access to `a`
    .with_read(b)              // shared access to `b`
    .run_before("create-c");   // explicit ordering

let reduce = Node::new(())
    .with_name("reduce")
    .with_move(a)              // consumes `a`
    .with_move(b)              // consumes `b`
    .with_result(d);           // creates `d`
```

### How are dependencies expressed?

Dependencies are expressed through **three mechanisms**, combined into a unified constraint model:

**1. Implicit resource dependencies (the novel contribution):**

| Access Pattern | Semantics | Constraint Generated |
|----------------|-----------|---------------------|
| `with_result(x)` | Creates resource `x` | Nodes needing `x` must run after this node |
| `with_read(x)` | Shared borrow of `x` | Can coexist with other readers |
| `with_write(x)` | Exclusive borrow of `x` | Cannot coexist with any other accessor of `x` (`Op::Ne`) |
| `with_move(x)` | Consumes resource `x` | Must run after all other nodes needing `x`; only one mover per resource |

**2. Explicit ordering:** `run_before("name")` and `run_after("name")` add direct `Op::Lt` or `Op::Gt` constraints.

**3. Barriers:** Nodes added after `with_barrier()` are constrained to run after all nodes added before the barrier.

The constraint derivation logic in `Node::constraints()` compares nodes pairwise and produces `Constraint` values with three operations:
- `Op::Lt` -- left must run before right
- `Op::Gt` -- left must run after right
- `Op::Ne` -- must not run in the same batch (but relative order is unspecified)

Key rules from the constraint derivation:
1. If both nodes move the same resource: **error** (cannot consume twice)
2. If node A moves a resource that B needs: A runs **after** B (B gets to use it first)
3. If node A creates (`result`) a resource that B needs: A runs **before** B
4. If either node writes a resource the other accesses: they get `Op::Ne` (mutual exclusion)
5. Explicit ordering and barriers add `Op::Lt` / `Op::Gt` with their own `RequirementReason`

### How is execution ordered?

The scheduling algorithm is a **constraint satisfaction solver using the AC-3 (Arc Consistency 3) algorithm**:

1. Compute all pairwise constraints between nodes via `all_constraints()`.
2. Assign each node a domain of possible batch indices `[0, 1, ..., n-1]`.
3. Iteratively reduce domains using AC-3: for each constraint, remove domain values that have no compatible value in the other variable's domain.
4. When AC-3 converges but domains still have multiple values, heuristically pop a value from a multi-valued domain and re-run AC-3.
5. Repeat until every domain is single-valued.
6. Group nodes by their assigned batch index; empty batches are removed.

This is notably **not a topological sort**. The CSP approach naturally handles the `Ne` (mutual exclusion) constraints that topological sort cannot express -- two nodes that both write different resources can share a batch, but two nodes that write the *same* resource cannot.

### How is parallelism handled?

Parallelism is a **scheduling output, not an execution mechanism**. The `Schedule<T>` contains `batches: Vec<Vec<T>>` where:
- Batches are sequential (batch 0 completes before batch 1 begins)
- All items within a single batch can execute in parallel because the solver guarantees no conflicting resource accesses within a batch

dagga provides **no executor**. The caller iterates over batches and parallelizes within each batch using whatever mechanism they prefer (rayon, tokio, threads, etc.).

### How are results passed between steps?

**They are not.** dagga is a pure scheduler. Resource identifiers (`E`) are abstract tokens used only for constraint derivation -- they do not carry values. The actual passing of data between steps is entirely the caller's responsibility.

---

## 4. State Management

### Is there any checkpointing or persistence?

**No.** dagga is stateless after scheduling. There is no concept of execution progress, checkpointing, or recovery.

### How is inter-step data passed?

Not handled. dagga produces a schedule; the caller manages all runtime state and data flow.

### Is state serializable?

The crate has an optional `serde` feature flag. The `Schedule<T>` struct (`Vec<Vec<T>>`) is trivially serializable if `T` is serializable. The `Node` and `Dag` types derive `Clone` and `Debug`. The `Constraint` type implements `Display` with human-readable formatting.

### How are errors handled and propagated?

dagga uses the **snafu** crate for structured error handling:

```rust
pub enum DaggaError {
    MovedMoreThanOnce { here: String, there: String },
    NoRootNodes,
    MissingResult { result: usize },
    Cycle { start: String, path: Vec<String> },
    Duplicates { node_names: Vec<String> },
    Conflict { reqs: Vec<Constraint> },
    CannotSolve { constraint: Constraint },
}
```

Notable design: `BuildScheduleError<T, E>` wraps `DaggaError` and **returns the `Dag` back to the caller** on failure. This is a good Rust pattern for recoverable errors with owned data -- the caller retains ownership and can inspect or modify the DAG before retrying.

Each `Constraint` carries `Vec<RequirementReason>` explaining *why* the constraint exists (barrier, explicit order, or input requirement), enabling informative error messages.

---

## 5. Configuration

### How are pipelines configured?

**Code only.** There is no configuration file support. DAGs are built entirely through programmatic builder API.

### Is there a builder pattern?

Yes. Both `Node` and `Dag` use fluent builder patterns with `with_*` methods:

```rust
let dag = Dag::<(), usize>::default()
    .with_node(Node::new(()).with_name("create-a").with_result(a))
    .with_node(Node::new(()).with_name("create-b").with_result(b))
    .with_barrier()
    .with_node(Node::new(()).with_name("use-ab").with_read(a).with_read(b));
```

`Dag` also provides `add_node` / `add_nodes` for imperative-style construction.

### How flexible is the configuration?

Moderately flexible within its scope:
- Resource identifiers can be any `Copy + Eq + Hash` type
- Node payloads can be any type
- Explicit ordering and barriers provide escape hatches when resource semantics alone are insufficient
- `get_missing_inputs()` enables graph validation before scheduling
- `with_root_node_requirement()` optionally enforces that root nodes exist

Cannot express: conditional execution, retry logic, timeouts, weighted priorities, resource pools/capacities, or dynamic graph modification after construction.

---

## 6. Relevance to Our Five Pillars

### Durability

**Does dagga address it?** No. No checkpointing, no persistence, no crash recovery.

**What can we learn?** Two patterns are valuable:
1. The `BuildScheduleError` returning the owned `Dag` on failure -- our pipeline builder should return partially-built state on validation failure for inspection and retry.
2. The clean separation between "compute schedule" and "execute schedule" -- a durability layer can wrap the batch executor without modifying the scheduler.

**What would we do differently?** We need checkpoint/resume at batch boundaries. dagga's `Vec<Vec<T>>` batch structure is naturally amenable to this: persist the current batch index and per-node completion status. On crash recovery, skip completed batches and resume from the last incomplete one. Within a batch, individual node results can be cached independently since nodes in a batch are guaranteed independent.

### Configurability

**Does dagga address it?** No. Code-only API.

**What can we learn?** The resource-access vocabulary (create/read/write/consume) is inherently **declarative** -- it describes *what* a stage needs, not *how* to wire it. This maps naturally to TOML configuration.

**What would we do differently?** Expose resource declarations in TOML:
```toml
[stages.fetch-gdrive]
reads = ["gdrive-api-client"]
results = ["raw-documents"]

[stages.transform]
reads = ["raw-documents"]
writes = ["output-dir"]
results = ["processed-documents"]
```
The scheduler would derive ordering from these declarations without explicit `depends_on` edges.

### Observability

**Does dagga address it?** Partially. The `dot` module generates Graphviz visualizations with color-coded edges (green=read, blue=write, red=move). The `DagLegend` type carries human-readable resource labels. Error messages include detailed constraint explanations with `RequirementReason`.

**What can we learn?** The constraint explanation system is excellent. Every ordering decision carries a reason: "X runs before Y because of input requirements." The `conflict_reason()` function enumerates all conflicting constraints when scheduling fails. This "explain why" capability is directly applicable to pipeline debugging.

**What would we do differently?** We need *runtime* observability (progress, timing, failures, retries), not just *static* schedule visualization. But the ideas of (a) generating visual schedule representations and (b) attaching human-readable reasons to every ordering constraint are both directly transferable.

### Incrementality

**Does dagga address it?** No. No concept of change detection or skipping unchanged work.

**What can we learn?** The resource model provides natural cache keys. If we associate content hashes or timestamps with resource identifiers, we can determine at execution time which nodes can be skipped because their inputs haven't changed.

**What would we do differently?** Extend the resource model with versioning metadata. At batch execution time, compare each node's input resource hashes against the previous run's values. If unchanged, skip the node and reuse its cached output resources.

### Composability

**Does dagga address it?** Yes -- this is dagga's primary strength.

**What can we learn?** The key insight: **implicit dependencies via resource declarations are more composable than explicit task-to-task edges.** Adding a new stage only requires declaring what it reads/writes -- existing stages need no modification. Removing a stage is equally clean (though it may leave missing inputs that `get_missing_inputs()` can detect). This directly supports our "mix-and-match stages per source type" requirement.

**What would we do differently?** We would adopt this resource-declaration model for shared resources (API clients, filesystem paths, database handles) while retaining explicit ordering for sequential processing stages within a single source pipeline where the linear flow is the natural expression. We would also add resource namespacing for multi-source pipelines.

---

## 7. Extractable Concepts

### Priority 1: Resource-Access Dependency Model

**Concept:** Express dependencies implicitly through create/read/write/consume declarations on shared resources, rather than explicit task-to-task edges.

**Rationale:** This maps directly to our pipeline's shared resources. Stages contend on the Google Drive API client (rate-limited, exclusive), filesystem output directories (exclusive writes), and redb database handles (shared reads, exclusive writes). Declaring `writes = ["gdrive-client"]` on a fetch stage automatically prevents concurrent API access without manual edge wiring. Adding a new source type that also uses the API client automatically gets correct scheduling.

**Adaptation:** Define resource identifiers as typed keys. In TOML config, each stage declares its `reads`, `writes`, `moves`, and `results`. The scheduler computes ordering at startup during the Topology phase.

### Priority 2: Batch-Based Parallel Schedule as Checkpoint Boundaries

**Concept:** The schedule is `Vec<Vec<T>>` -- sequential batches of parallelizable work. Each batch boundary is a natural checkpoint.

**Rationale:** This maps to our durability model: complete batch N, persist state (batch index + per-node results), proceed to batch N+1. On crash, deserialize state, skip completed batches, resume. Within a batch, nodes are independent and individually retriable.

**Adaptation:** Use batch boundaries as checkpoint triggers. The State layer records which batches are complete and which node results are available.

### Priority 3: Rust Borrow-Checker Semantics for Resource Scheduling

**Concept:** Apply Rust's ownership model to resource scheduling -- shared borrows (reads) coexist, exclusive borrows (writes) exclude all others, moves transfer ownership.

**Rationale:** This is intuitive for Rust developers and correctly models our resource contention. A rate-limited API client needs exclusive access (write). A configuration file can be read concurrently (read). A temporary working directory can be consumed by a cleanup stage (move).

### Priority 4: Constraint Explanation with RequirementReason

**Concept:** Every ordering constraint carries a `RequirementReason` explaining *why* it exists.

**Rationale:** When a pipeline schedule doesn't look right or fails validation, users need to understand why. "Stage X runs before Stage Y because both access 'output-dir' exclusively" is the kind of diagnostic our observability layer should provide. This is especially important when ordering is derived automatically from TOML declarations.

### Priority 5: Missing Input Detection at Construction Time

**Concept:** `get_missing_inputs()` identifies resources consumed but never produced within the graph.

**Rationale:** At pipeline construction time (Topology phase), we can validate that all required resources are either provided externally or produced by some stage, giving clear error messages before execution begins. E.g., "Stage 'transform' reads 'raw-documents' but no stage produces it."

### Priority 6: Barrier-Based Phase Separation

**Concept:** Insert barriers that force all preceding nodes to complete before any subsequent nodes begin.

**Rationale:** Useful for pipeline phases: "fetch everything first, then transform, then publish." Barriers express global ordering that resource semantics alone cannot capture.

### Priority 7: Error Recovery with Ownership Return

**Concept:** `BuildScheduleError<T, E>` returns the `Dag` on failure so the caller retains ownership for inspection or retry.

**Rationale:** When pipeline topology validation fails, the builder should return the partially-resolved topology for debugging, not just an error message.

---

## 8. Anti-Patterns / Warnings

### String-Based Node Identity for Explicit Ordering

Nodes are identified by `name: String` for `run_before`/`run_after` constraints. A typo in a node name silently creates a constraint that matches nothing. Our system should use typed identifiers or validate all references at construction time, failing fast on unknown names.

### No Execution Model -- Scheduling Only

dagga is *only* a scheduler. It provides no task runner, no error handling during execution, no result passing, no retry logic, no timeout support. It covers at most the "Topology" portion of our three-layer architecture. We must not conflate "we have a schedule" with "we have a pipeline runner." The executor (State layer) is the bulk of the engineering work.

### Constraint Solver Opacity

The AC-3 solver is mathematically sound but can be opaque when it fails. The `CannotSolve` error points to a single unsatisfiable constraint, but for users debugging TOML-configured pipelines, understanding *why* their resource declarations are unsolvable may be difficult. We should invest in better diagnostics that suggest fixes (e.g., "Stages X and Y both write to 'output-dir' and a barrier requires X after Y -- consider making one stage read-only or removing the conflicting barrier").

### No Dynamic or Conditional Execution

The graph is fully static -- built, scheduled, and frozen. There is no support for:
- Conditional branches ("skip this stage if content unchanged")
- Dynamic graph modification during execution
- Early termination on failure
- Fan-out based on discovered work

Our pipeline needs all of these. We should design the executor to support per-node skip/retry decisions within the batch structure, without modifying the scheduling algorithm.

### O(n^2) Pairwise Constraint Generation

`all_constraints()` compares every node pair. For our likely graph sizes (tens of stages), this is fine. But the approach would not scale to thousands of nodes without optimization (e.g., index nodes by resource to avoid full pairwise comparison).

### Resource IDs Are Untyped

Resources are identified by opaque `E` values with no type safety ensuring compatibility between producers and consumers. In our system, we should use typed resource identifiers (e.g., a `ResourceKind` enum) to catch misconfiguration during TOML validation.

### No Resource Capacity or Pooling

Resources are binary -- they exist or they don't. There is no concept of a resource pool (e.g., "up to 3 concurrent API connections") or capacity limits. Our rate-limited API clients would need a richer model -- possibly extending the write/exclusive model with a semaphore-like capacity parameter.

### Solver May Not Find Optimal Solution

The AC-3 + `domain.0.pop()` fallback finds *a* valid schedule but not necessarily the one with the *minimum* number of batches. For our purposes a valid schedule is sufficient, but this means the schedule may have unnecessary serialization.
