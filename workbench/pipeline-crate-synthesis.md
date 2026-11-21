# Pipeline Crate Survey Analysis Synthesis

**Date:** 2026-03-13
**Input:** Six deep analyses of Rust DAG/workflow/pipeline crates
**Purpose:** Actionable recommendations for the ECL pipeline runner design

---

## 1. Concept Inventory

The best concepts across all six crates, deduplicated and ranked.

| # | Concept | Source Crate(s) | Description | Relevance | Pillar(s) |
|---|---------|----------------|-------------|-----------|-----------|
| 1 | **Checkpoint <-> Context separation** | erio-workflow, oxigdal-workflow | Persistence type (`Checkpoint`) is distinct from runtime type (`WorkflowContext`), with explicit conversion between them | HIGH | Durability, Observability |
| 2 | **Layer/wave-based execution** | dagx, erio-workflow, oxigdal-workflow | Topological sort into parallel layers; each layer is a natural checkpoint boundary | HIGH | Durability, Composability |
| 3 | **Resource-access implicit dependencies** | dagga | Dependencies derived from create/read/write/consume declarations on shared resources, not explicit edges | HIGH | Composability, Configurability |
| 4 | **Action/Node two-trait separation** | dagrs | Reusable logic trait (`Action`) composed into identity/wiring struct (`Node`); same logic, different positions | HIGH | Composability |
| 5 | **Type-state builder for DAG construction** | dagx | `TaskBuilder` consumed by `depends_on()` to produce immutable `TaskHandle`; compile-time cycle prevention | HIGH | Composability |
| 6 | **RetryPolicy struct** | oxigdal-workflow | Four fields: `max_attempts`, `delay_ms`, `backoff_multiplier`, `max_delay_ms` -- minimal and complete | HIGH | Durability |
| 7 | **Versioned checkpoints with sequence numbers** | oxigdal-workflow | Checkpoint bundles state + DAG + sequence + timestamp; `prepare_for_resume()` resets interrupted tasks | HIGH | Durability |
| 8 | **TypedAction with associated I/O types** | dagrs | `TypedAction<I, O>` with blanket impl bridging to untyped `Action`; compile-time safety, runtime erasure | HIGH | Composability |
| 9 | **ConditionalStep wrapper** | erio-workflow | Generic `ConditionalStep<S: Step>` that produces `skipped` output when predicate is false; downstream sees "completed" | MEDIUM | Composability |
| 10 | **Snapshot isolation for parallel steps** | erio-workflow | Clone context before spawning parallel tasks; merge results after all complete; no shared mutable state | MEDIUM | Durability, Composability |
| 11 | **Fill/Transform/Pour ETL taxonomy** | rettle | Three-phase categorization (Extract, Transform, Load) as distinct trait families | MEDIUM | Composability |
| 12 | **Batch-based parallel scheduling** | dagga, rettle | Output schedule as `Vec<Vec<T>>` -- batches are natural checkpoint + parallelism boundaries | MEDIUM | Durability, Composability |
| 13 | **Missing input detection** | dagga | `get_missing_inputs()` identifies resources consumed but never produced; catches config errors before execution | MEDIUM | Configurability, Observability |
| 14 | **Community adapter crate pattern** | rettle | Separate crates per data source/sink (cstea, elastictea, logtea) with pre-built Fill/Pour implementations | MEDIUM | Composability, Configurability |
| 15 | **Runtime-agnostic spawner** | dagx | User provides spawner closure; no dependency on any specific async runtime | LOW | Composability |

---

## 2. Trait Design Recommendations

### Core Traits

Based on the best patterns across all six crates, here are the recommended trait signatures for the ECL pipeline runner.

#### Stage Trait (the unit of work)

Combines dagrs's Action/Node separation with dagx's typed I/O and erio-workflow's checkpoint-aware design:

```rust
/// The core unit of pipeline work. Separates logic from identity/wiring.
/// Analogous to dagrs's `Action` + dagx's `Task<Input>`.
#[async_trait]
pub trait Stage: Send + Sync {
    /// Stable, deterministic identifier for checkpointing and config reference.
    /// NOT an auto-incremented integer (dagrs anti-pattern).
    fn id(&self) -> &StageId;

    /// Human-readable name for observability.
    fn name(&self) -> &str;

    /// Execute the stage. Receives immutable input context, returns typed output.
    /// The engine handles writing output to the state store.
    async fn execute(&self, ctx: &StageContext) -> Result<StageOutput, StageError>;

    /// Declare resource access patterns (from dagga).
    /// Used by the scheduler to compute parallel batches.
    fn resource_access(&self) -> ResourceAccess {
        ResourceAccess::default()
    }

    /// Optional retry policy (from oxigdal-workflow).
    fn retry_policy(&self) -> Option<RetryPolicy> {
        None
    }

    /// Optional timeout.
    fn timeout(&self) -> Option<Duration> {
        None
    }
}
```

#### StageContext (immutable input view)

Addresses erio-workflow's anti-pattern of `&mut WorkflowContext`:

```rust
/// Immutable snapshot of completed stage outputs, available to a running stage.
/// Analogous to erio-workflow's WorkflowContext but immutable (no footgun).
pub struct StageContext {
    /// Outputs from prior stages, keyed by StageId.
    outputs: HashMap<StageId, Arc<StageOutput>>,
    /// Pipeline-level configuration from the Specification layer.
    config: Arc<PipelineConfig>,
}

impl StageContext {
    /// Retrieve a prior stage's output by ID (typed access).
    pub fn output(&self, stage_id: &StageId) -> Option<&StageOutput> { ... }

    /// Retrieve pipeline config for this stage.
    pub fn stage_config(&self, stage_id: &StageId) -> Option<&toml::Value> { ... }
}
```

#### StageOutput (serializable from day one)

Avoids dagrs's `Arc<dyn Any>` anti-pattern:

```rust
/// The result of a stage execution. Always serializable.
/// Richer than erio-workflow's string-only StepOutput.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageOutput {
    /// Structured output data (not a String -- avoids erio AP-1).
    pub data: serde_json::Value,
    /// Content hash of the output for incrementality.
    pub content_hash: Blake3Hash,
    /// Timing and metadata for observability.
    pub metadata: OutputMetadata,
    /// Whether this stage was skipped (conditional execution).
    pub skipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputMetadata {
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub attempt: u32,
}
```

#### SourceAdapter (the E of ETL)

Inspired by rettle's Fill concept but without the orchestration leak:

```rust
/// Adapter for external data sources. Sources produce items; the framework
/// handles batching, parallelism, and checkpointing.
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Type of items this source produces.
    type Item: Serialize + DeserializeOwned + Send + Sync;

    /// Unique source identifier (matches TOML config key).
    fn source_id(&self) -> &str;

    /// Fetch items from the external source. The framework handles
    /// batching and dispatch -- the adapter only extracts.
    async fn fetch(&self, config: &SourceConfig) -> Result<Vec<Self::Item>, SourceError>;

    /// Content hash for incrementality. If the hash matches the last run,
    /// the source is skipped entirely.
    async fn content_fingerprint(&self, config: &SourceConfig) -> Result<Blake3Hash, SourceError>;
}
```

#### StateStore (persistence abstraction)

Avoids oxigdal-workflow's filesystem-only anti-pattern:

```rust
/// Trait-based persistence for checkpoints and run state.
/// Default implementation uses redb; trait allows future alternatives.
pub trait StateStore: Send + Sync {
    fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StateError>;
    fn load_checkpoint(&self, run_id: &RunId) -> Result<Option<Checkpoint>, StateError>;
    fn save_stage_result(&self, run_id: &RunId, stage_id: &StageId, output: &StageOutput) -> Result<(), StateError>;
    fn load_stage_result(&self, run_id: &RunId, stage_id: &StageId) -> Result<Option<StageOutput>, StateError>;
    fn load_content_hash(&self, run_id: &RunId, stage_id: &StageId) -> Result<Option<Blake3Hash>, StateError>;
}
```

#### ResourceAccess (from dagga)

```rust
/// Declares what shared resources a stage touches.
/// Used by the scheduler to compute parallel batches.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceAccess {
    /// Resources this stage creates (produces for the first time).
    pub creates: Vec<ResourceId>,
    /// Resources this stage reads (shared access, multiple readers OK).
    pub reads: Vec<ResourceId>,
    /// Resources this stage writes (exclusive access, no concurrent readers/writers).
    pub writes: Vec<ResourceId>,
    /// Resources this stage consumes (exclusive, not available after).
    pub consumes: Vec<ResourceId>,
}
```

#### RetryPolicy (from oxigdal-workflow)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub max_delay_ms: u64,
}
```

---

## 3. Execution Model Recommendation

### Recommended: Hybrid Layer + Resource Scheduling

Combine dagx's layer-based topological execution with dagga's resource-access scheduling. This gives us the best of both worlds.

#### What to build

1. **Resource-aware batch scheduler** (from dagga's model). Stages declare resource access patterns in TOML config. The scheduler computes parallel batches where no resource conflicts exist within a batch. This replaces manual edge wiring.

2. **Layer-based execution with checkpoint boundaries** (from dagx + erio-workflow). Execute batches sequentially. Within each batch, execute stages concurrently. After each batch completes, persist a checkpoint. On resume, skip completed batches.

3. **Conditional execution via wrapper** (from erio-workflow). `ConditionalStage<S>` wraps any stage with a predicate. When false, produces `StageOutput { skipped: true, .. }`. Downstream stages see this as "completed" and can inspect the skipped flag.

4. **Snapshot isolation** (from erio-workflow). Before executing a batch's stages in parallel, clone the `StageContext` as an immutable snapshot. Each stage gets the same view. After all complete, merge outputs into the shared state.

5. **Per-stage retry with backoff** (from oxigdal-workflow). The engine wraps stage execution in a retry loop using the stage's `RetryPolicy`. Checkpoints distinguish "failed after retries" from "not yet attempted."

#### What to skip

- **AC-3 constraint solver** (dagga). Overkill for our graph sizes (~10-50 stages). A simpler algorithm -- topological sort, then split layers by resource conflicts -- is sufficient and far easier to debug.
- **Compile-time cycle prevention** (dagx). Beautiful but incompatible with runtime config-driven pipelines. We need dynamic DAG construction from TOML.
- **Loop/cycle support** (dagrs). We don't need cyclic DAGs. Retry is handled at the stage level, not the graph level.
- **Channel-based inter-stage communication** (dagrs). Channels add complexity for async ordering. Our layer-based model with a shared state map is simpler and sufficient.
- **Runtime-agnostic spawner** (dagx). We're using Tokio. No need to abstract the runtime.

#### Execution flow

```
TOML Config (Specification)
    ↓ parse + validate
Topology (resolved stages, resource declarations)
    ↓ schedule (resource-aware batch computation)
Schedule: Vec<Vec<StageId>>
    ↓ execute
For each batch:
    1. Build StageContext snapshot from completed outputs
    2. For stages with incrementality: check input content hash vs last run
       → skip if unchanged
    3. Execute remaining stages in parallel (tokio::JoinSet)
    4. Collect results, update State
    5. Persist checkpoint to redb
    ↓
Final State (serializable, inspectable, resumable)
```

---

## 4. State Design Recommendation

### Three-Layer State Model

```
┌─────────────────────────────────────────────┐
│  Specification (immutable after load)       │
│  - PipelineConfig from TOML                 │
│  - Source definitions, stage parameters      │
│  - Retry policies, timeouts                 │
│  - Resource declarations                     │
├─────────────────────────────────────────────┤
│  Topology (immutable after init)            │
│  - Resolved stage instances                  │
│  - Computed schedule (Vec<Vec<StageId>>)     │
│  - Resource conflict map                     │
│  - Validated dependency graph               │
├─────────────────────────────────────────────┤
│  State (mutates during execution)           │
│  - Per-stage: status, output, content hash   │
│  - Per-run: run_id, started_at, progress    │
│  - Checkpoint: bundled state snapshot        │
│  - History: prior run hashes for increment.  │
└─────────────────────────────────────────────┘
```

### Example struct layouts

```rust
/// Specification layer -- deserialized from TOML, immutable during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub version: u32,
    pub name: String,
    pub sources: HashMap<String, SourceSpec>,
    pub stages: HashMap<StageId, StageSpec>,
    pub defaults: DefaultsSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    pub name: String,
    pub adapter: String,           // registered adapter name
    pub params: toml::Value,       // adapter-specific config
    pub resources: ResourceAccess, // what this stage touches
    pub retry: Option<RetryPolicy>,
    pub timeout_secs: Option<u64>,
    pub condition: Option<String>, // predicate expression
}

/// Topology layer -- computed at startup, immutable during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineTopology {
    pub schedule: Vec<Vec<StageId>>,    // batches of parallel stages
    pub resource_map: HashMap<ResourceId, Vec<StageId>>,
    pub stage_order: Vec<StageId>,      // deterministic total order
}

/// State layer -- mutates during execution, persisted as checkpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub run_id: RunId,
    pub spec_hash: Blake3Hash,          // detect config changes between runs
    pub status: PipelineStatus,
    pub started_at: DateTime<Utc>,
    pub current_batch: usize,
    pub stages: HashMap<StageId, StageState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub status: StageStatus,
    pub output: Option<StageOutput>,
    pub input_hash: Option<Blake3Hash>,  // for incrementality
    pub output_hash: Option<Blake3Hash>, // for incrementality
    pub attempts: u32,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped,
    Failed,
}

/// Checkpoint -- the full recovery artifact.
/// Bundles all three layers so recovery is self-contained
/// (from oxigdal-workflow's design).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub sequence: u64,
    pub spec: PipelineSpec,
    pub topology: PipelineTopology,
    pub state: PipelineState,
}

impl Checkpoint {
    /// Reset interrupted stages to Pending for retry
    /// (from oxigdal-workflow's prepare_for_resume).
    pub fn prepare_for_resume(&mut self) {
        for stage in self.state.stages.values_mut() {
            if stage.status == StageStatus::Running {
                stage.status = StageStatus::Pending;
                stage.attempts = stage.attempts.saturating_sub(1);
            }
        }
    }
}
```

### State IS the Pipeline

The `PipelineState` struct should be the complete, inspectable truth. When handed to an AI or human:

- Every stage has a status, timing data, content hash, and error message if failed
- The full config (spec) is embedded, so there's no question about "what config was this run using"
- The topology shows the computed execution plan
- Content hashes enable diffing across runs ("what changed since last time?")
- The checkpoint is a single serializable artifact -- dump it, email it, feed it to Claude

---

## 5. What We Should NOT Do

### 1. Global mutable ID allocators (dagrs)

**What they did:** `static AtomicUsize` for globally unique node IDs. Non-deterministic across runs, breaks checkpointing and testing.

**What to do instead:** Deterministic, content-based or name-based `StageId` values derived from config. IDs must be stable across runs for checkpoint compatibility.

### 2. `Arc<dyn Any>` for inter-stage data (dagrs, dagx)

**What they did:** Type-erased data containers that are fundamentally non-serializable. Makes checkpointing, observability, and incrementality impossible.

**What to do instead:** Require `Serialize + Deserialize` on all stage outputs from day one. Use `serde_json::Value` as the interchange format. Accept the serialization cost -- it's the price of our five pillars.

### 3. Mutable context passed to stages (erio-workflow)

**What they did:** `&mut WorkflowContext` lets a stage mutate prior stages' outputs. Footgun even with snapshot isolation for parallel stages.

**What to do instead:** Pass `&StageContext` (immutable). Stages return their output; the engine writes it.

### 4. Non-atomic checkpoint writes (erio-workflow)

**What they did:** `tokio::fs::write` directly to the checkpoint path. A crash during write corrupts the checkpoint.

**What to do instead:** Write to a temp file, then `fs::rename` (atomic on POSIX). Or use redb, which provides ACID transactions natively.

### 5. Kitchen-sink API surface (oxigdal-workflow)

**What they did:** 1,162 documented items at v0.1.1. Cron scheduling, Kafka integration, Airflow connectors, debugger sessions -- breadth over depth.

**What to do instead:** Ship the minimum viable surface: Stage trait, SourceAdapter trait, StateStore trait, PipelineRunner, Checkpoint. Add features when real use cases demand them.

### 6. String-typed outputs (erio-workflow)

**What they did:** `StepOutput.value` is always `String`. Structured data requires serialize-to-string-then-parse.

**What to do instead:** Use `serde_json::Value` as the primary interchange type. It's structured, serializable, and supports nested data without double-encoding.

### 7. Tea metaphors and non-standard terminology (rettle)

**What they did:** Pot, Brewery, Brewer, Steep, Fill, Pour, Tea. Charming but adds cognitive overhead.

**What to do instead:** Use domain-standard names. Stage, Source, Sink, Pipeline, Runner. Save creativity for the product name, not the API.

### 8. `unsafe impl Send/Sync` (dagrs, rettle)

**What they did:** Manually marked types as Send/Sync to work around function pointer/trait object limitations.

**What to do instead:** Design types that are automatically Send+Sync. If the compiler won't derive it, redesign the type.

### 9. Single recipe for all sources (rettle)

**What they did:** Every source goes through the same transformation chain.

**What to do instead:** Per-source pipeline definitions in TOML. A Google Drive document and a Slack message should have different stage sequences.

### 10. Removing config file support (dagrs v0.3 -> v0.5)

**What they did:** Had a `Parser` trait with YAML support, then removed it in the rewrite.

**What to do instead:** Config file support is a core requirement, not an optional feature. It lives in the Specification layer and is never removed.

---

## 6. Recommended Architecture Sketch

### Crate Structure

```
ecl-pipeline-spec     # Specification layer: TOML parsing, config types, validation
ecl-pipeline-topo     # Topology layer: schedule computation, resource conflict resolution
ecl-pipeline-state    # State layer: checkpoint, state store (redb), content hashing
ecl-pipeline-engine   # Execution engine: batch runner, retry, conditional execution
ecl-pipeline          # Facade crate: re-exports, CLI entry point
ecl-adapter-gdrive    # SourceAdapter for Google Drive
ecl-adapter-slack     # SourceAdapter for Slack
ecl-adapter-granola   # SourceAdapter for Granola
ecl-adapter-fs        # Sink adapter for filesystem output
```

### Dependency graph

```
ecl-pipeline (facade)
  ├── ecl-pipeline-engine
  │     ├── ecl-pipeline-topo
  │     │     └── ecl-pipeline-spec
  │     └── ecl-pipeline-state
  │           └── ecl-pipeline-spec
  ├── ecl-adapter-gdrive
  │     └── ecl-pipeline-spec (for SourceAdapter trait)
  ├── ecl-adapter-slack
  ├── ecl-adapter-granola
  └── ecl-adapter-fs
```

### Key traits and relationships

```
PipelineSpec (from TOML)
    ──parse──> PipelineTopology (schedule, resource map)
                   ──execute──> PipelineState (checkpointed progress)

Stage trait  ←── implemented by adapters (GDriveFetch, SlackFetch, MarkdownNormalizer, etc.)
    │
    ├── execute(&self, &StageContext) -> Result<StageOutput>
    ├── resource_access() -> ResourceAccess
    └── retry_policy() -> Option<RetryPolicy>

SourceAdapter trait  ←── implemented by source crates (ecl-adapter-gdrive, etc.)
    │
    ├── fetch(&self, &SourceConfig) -> Result<Vec<Item>>
    └── content_fingerprint(&self, &SourceConfig) -> Result<Blake3Hash>

StateStore trait  ←── implemented by RedbStateStore (default)
    │
    ├── save_checkpoint(&self, &Checkpoint)
    ├── load_checkpoint(&self, &RunId) -> Option<Checkpoint>
    └── save/load_stage_result(...)
```

### Three layers mapped to code

| Layer | Crate | Key Types | Mutability |
|-------|-------|-----------|------------|
| **Specification** | `ecl-pipeline-spec` | `PipelineSpec`, `StageSpec`, `SourceSpec`, `ResourceAccess`, `RetryPolicy` | Immutable after TOML parse |
| **Topology** | `ecl-pipeline-topo` | `PipelineTopology`, schedule (`Vec<Vec<StageId>>`), resource conflict map | Immutable after computation |
| **State** | `ecl-pipeline-state` | `PipelineState`, `StageState`, `Checkpoint`, `StateStore` | Mutates during execution, persisted to redb |

### How a Google Drive -> Markdown pipeline would be expressed

**TOML config (Specification):**

```toml
[pipeline]
name = "knowledge-ingest"
version = 1

[sources.gdrive]
adapter = "gdrive"
folder_id = "1abc..."
mime_types = ["application/vnd.google-apps.document", "application/pdf"]
credentials_path = "~/.config/ecl/gdrive-creds.json"

[sources.slack]
adapter = "slack"
channels = ["C0123GENERAL", "C0456ENGINEERING"]
lookback_days = 30

[stages.fetch-gdrive]
adapter = "gdrive-fetch"
source = "gdrive"
resources = { reads = ["gdrive-api"], creates = ["raw-gdrive-docs"] }
retry = { max_attempts = 3, initial_delay_ms = 1000, backoff_multiplier = 2.0, max_delay_ms = 30000 }
timeout_secs = 300

[stages.fetch-slack]
adapter = "slack-fetch"
source = "slack"
resources = { reads = ["slack-api"], creates = ["raw-slack-messages"] }
retry = { max_attempts = 3, initial_delay_ms = 500, backoff_multiplier = 2.0, max_delay_ms = 10000 }

[stages.normalize-gdrive]
adapter = "markdown-normalizer"
resources = { reads = ["raw-gdrive-docs"], creates = ["normalized-gdrive"] }
params = { output_format = "structured-markdown", preserve_headings = true }

[stages.normalize-slack]
adapter = "slack-normalizer"
resources = { reads = ["raw-slack-messages"], creates = ["normalized-slack"] }
params = { thread_mode = "inline", include_reactions = false }

[stages.synthesize]
adapter = "concept-extractor"
resources = { reads = ["normalized-gdrive", "normalized-slack"], creates = ["concept-cards"] }

[stages.index]
adapter = "fts-indexer"
resources = { reads = ["concept-cards"], writes = ["search-index"] }
```

**Computed schedule (Topology):**

```
Batch 0: [fetch-gdrive, fetch-slack]       # independent, different resources
Batch 1: [normalize-gdrive, normalize-slack] # independent, read different resources
Batch 2: [synthesize]                        # reads both normalized outputs
Batch 3: [index]                             # writes to search index
```

**Runtime (State after partial execution):**

```json
{
  "run_id": "run-2026-03-13-001",
  "status": "Failed",
  "current_batch": 1,
  "stages": {
    "fetch-gdrive": { "status": "Completed", "output_hash": "a1b2c3...", "duration_ms": 12340 },
    "fetch-slack": { "status": "Completed", "output_hash": "d4e5f6...", "duration_ms": 8920 },
    "normalize-gdrive": { "status": "Failed", "attempts": 3, "error": "Parse error in doc XYZ: unsupported table format" },
    "normalize-slack": { "status": "Completed", "output_hash": "g7h8i9...", "duration_ms": 2100 }
  }
}
```

On resume: fetch stages are skipped (completed), normalize-slack is skipped (completed), normalize-gdrive retries from where it left off. The state file tells you exactly what happened and why, without logs or dashboards.
