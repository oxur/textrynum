---
number: 17
title: "ECL Pipeline Runner: Unified Design Vision"
author: "name in"
component: All
tags: [change-me]
created: 2026-03-13
updated: 2026-03-13
state: Active
supersedes: 3
superseded-by: null
version: 1.1
---

# ECL Pipeline Runner: Unified Design Vision

## Purpose

This document is the authoritative design for the ECL pipeline runner. It
unifies three inputs:

1. **The design proposal** — a trait-based thought experiment grounded in our
   concrete use case (Google Drive, Slack, Granola → Fabryk)
2. **The crate synthesis** — distilled patterns from six Rust workflow/DAG
   crates (dagrs, erio-workflow, oxigdal-workflow, dagx, dagga, rettle)
3. **The bootstrap session** — the five pillars, three layers, weight class,
   and "state IS the pipeline" insight

Every design decision traces to at least one of the five pillars. Every open
question from the design proposal is resolved. The document is concrete enough
to begin implementation.

---

## Governing Principles

### Five Pillars

1. **Durability** — Checkpoint/resume on failure; long-running pipelines
   survive crashes without re-executing completed work
2. **Configurability** — Declarative, per-source TOML configuration for
   source selection, format handling, and processing parameters
3. **Observability** — Serializable state that humans or AI can inspect to
   understand exactly what happened, what failed, and why
4. **Incrementality** — Skip unchanged items across runs via content hashing
   or timestamps; efficiency across runs, not just within runs
5. **Composability** — Mix-and-match stages per source type; not every source
   needs every transformation step

### Three Layers

1. **Specification** — What the user declares. Parsed from TOML, validated,
   immutable during execution. Where Configurability lives.
2. **Topology** — The resolved execution plan. Concrete adapters, computed
   schedule, validated resource graph. Computed at startup, immutable during
   execution. Where Composability lives.
3. **State** — What has happened, is happening, and remains. Mutates during
   execution, persisted for checkpointing. Where Durability, Incrementality,
   and Observability live.

### Weight Class

A CLI tool with checkpointing. Run on a beefy local machine. Configure, point
at data sources, run, monitor, iterate. No Kubernetes, no external databases,
no distributed orchestration servers. ~1000-2000 lines of custom Rust atop
proven crates.

### Core Insight: State IS the Pipeline

The pipeline's serialized state is the complete, inspectable, resumable truth.
When handed to Claude — or a human — it contains enough richly-typed,
well-named data for a full conceptual analysis without logs, dashboards, or
database queries.

### Item-Centric, Not Stage-Centric

Items (documents, messages, notes) flow through stages. Each item carries its
own status, provenance, content hash, and completion history. This is the
natural model for "200 documents from Drive, each at a different point in the
pipeline." Stage-level state (batch progress, timing) exists but is derived
from item-level truth.

---

## 1. Layer 1: Specification (from TOML)

The specification is what the user declares. It's parsed once, validated, and
never mutated. All types derive `Serialize + Deserialize` for embedding in
checkpoints.

### Root Configuration

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The root configuration, deserialized from TOML.
/// Immutable after load. This is the "what do you want to happen" layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Human-readable name for this pipeline.
    pub name: String,

    /// Schema version for forward compatibility.
    pub version: u32,

    /// Where pipeline state and outputs are written.
    pub output_dir: PathBuf,

    /// Source definitions, keyed by user-chosen name.
    /// BTreeMap for deterministic serialization order.
    pub sources: BTreeMap<String, SourceSpec>,

    /// Stage definitions, keyed by user-chosen name.
    /// Ordering is declarative; the topology layer resolves execution order
    /// from resource declarations.
    pub stages: BTreeMap<String, StageSpec>,

    /// Global defaults that apply across all sources/stages.
    #[serde(default)]
    pub defaults: DefaultsSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    /// Maximum concurrent operations within a batch.
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,

    /// Default retry policy for transient failures.
    #[serde(default)]
    pub retry: RetrySpec,

    /// Default checkpoint strategy.
    #[serde(default)]
    pub checkpoint: CheckpointStrategy,
}

fn default_concurrency() -> usize { 4 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    /// Total attempts (1 = no retry).
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial backoff duration in milliseconds.
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Multiplier applied to backoff after each attempt.
    /// (From oxigdal-workflow — the proposal was missing this.)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Maximum backoff duration in milliseconds.
    #[serde(default = "default_max_backoff")]
    pub max_backoff_ms: u64,
}

fn default_max_attempts() -> u32 { 3 }
fn default_initial_backoff() -> u64 { 1000 }
fn default_backoff_multiplier() -> f64 { 2.0 }
fn default_max_backoff() -> u64 { 30_000 }

impl Default for RetrySpec {
    fn default() -> Self {
        Self {
            max_attempts: default_max_attempts(),
            initial_backoff_ms: default_initial_backoff(),
            backoff_multiplier: default_backoff_multiplier(),
            max_backoff_ms: default_max_backoff(),
        }
    }
}

impl Default for DefaultsSpec {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            retry: RetrySpec::default(),
            checkpoint: CheckpointStrategy::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "every")]
pub enum CheckpointStrategy {
    /// Checkpoint after every stage batch completes (default).
    Batch,
    /// Checkpoint after every N items processed within a stage.
    Items { count: usize },
    /// Checkpoint on a time interval.
    Seconds { duration: u64 },
}

impl Default for CheckpointStrategy {
    fn default() -> Self { Self::Batch }
}
```

### Source Specification

Sources are external data services. Each source type has its own config shape
but shares a common envelope. The `kind` field drives adapter resolution.

```rust
/// A source is "where does the data come from?"
/// The `kind` field determines which SourceAdapter handles it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    #[serde(rename = "google_drive")]
    GoogleDrive(GoogleDriveSourceSpec),

    #[serde(rename = "slack")]
    Slack(SlackSourceSpec),

    #[serde(rename = "filesystem")]
    Filesystem(FilesystemSourceSpec),
    // Future: Granola, Notion, Confluence, etc.
    // Each new source type adds a variant here and an adapter.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveSourceSpec {
    /// OAuth2 credentials reference.
    pub credentials: CredentialRef,

    /// Root folder ID(s) to scan.
    pub root_folders: Vec<String>,

    /// Include/exclude filter rules, evaluated in order.
    #[serde(default)]
    pub filters: Vec<FilterRule>,

    /// Which file types to process.
    #[serde(default)]
    pub file_types: Vec<FileTypeFilter>,

    /// Only process files modified after this timestamp.
    /// Supports "last_run" as a magic value for incrementality.
    pub modified_after: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Glob pattern matched against the full path.
    pub pattern: String,
    /// Whether this rule includes or excludes matches.
    pub action: FilterAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction { Include, Exclude }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeFilter {
    pub extension: Option<String>,
    pub mime: Option<String>,
}

/// Slack workspace source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSourceSpec {
    /// Bot token credentials reference.
    pub credentials: CredentialRef,

    /// Channel IDs to fetch messages from.
    pub channels: Vec<String>,

    /// How deep to follow threads (0 = top-level only).
    #[serde(default)]
    pub thread_depth: usize,

    /// Only process messages after this timestamp.
    pub modified_after: Option<String>,
}

/// Local filesystem source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSourceSpec {
    /// Root directory to scan.
    pub root: PathBuf,

    /// Include/exclude filter rules.
    #[serde(default)]
    pub filters: Vec<FilterRule>,

    /// File extensions to include (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// How to resolve credentials for a source.
///
/// Uses internally-tagged representation (`"type": "file"`) rather than
/// `#[serde(untagged)]` because untagged enums are fragile with TOML:
/// deserialization order is ambiguous when multiple variants have
/// overlapping shapes, and error messages on mismatch are unhelpful.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    #[serde(rename = "file")]
    File { path: PathBuf },
    #[serde(rename = "env")]
    EnvVar { env: String },
    #[serde(rename = "application_default")]
    ApplicationDefault,
}
```

### Stage Specification

Stages are keyed by name in a BTreeMap. Execution order is derived from
resource declarations, not from declaration order. This is the key departure
from the linear-chain model: stages declare what resources they read and
create, and the topology layer computes a parallel schedule.

```rust
/// A stage is "what work to perform."
/// Resource declarations determine execution order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    /// Which registered stage implementation to use.
    pub adapter: String,

    /// Which source this stage operates on (for extract stages).
    pub source: Option<String>,

    /// Resource access declarations (from dagga's model).
    /// The topology layer uses these to compute the parallel schedule.
    #[serde(default)]
    pub resources: ResourceSpec,

    /// Stage-specific parameters passed to the adapter.
    /// Uses `serde_json::Value` rather than `toml::Value` so the spec layer
    /// is format-agnostic — the same types work whether the config is loaded
    /// from TOML, JSON, or embedded in a checkpoint.
    #[serde(default)]
    pub params: serde_json::Value,

    /// Override the default retry policy for this stage.
    pub retry: Option<RetrySpec>,

    /// Override the default timeout for this stage.
    pub timeout_secs: Option<u64>,

    /// If true, item-level failures skip the item rather than failing
    /// the pipeline. Useful for best-effort extraction.
    #[serde(default)]
    pub skip_on_error: bool,

    /// Optional predicate expression. When false, the stage is skipped
    /// entirely and produces a "skipped" output that downstream stages
    /// can inspect. (From erio-workflow's ConditionalStep pattern.)
    pub condition: Option<String>,
}

/// Resource access declarations in TOML-friendly form.
/// Converted to ResourceAccess at topology resolution time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    #[serde(default)]
    pub reads: Vec<String>,
    #[serde(default)]
    pub creates: Vec<String>,
    #[serde(default)]
    pub writes: Vec<String>,
}
```

### Example TOML

```toml
name = "q1-knowledge-sync"
version = 1
output_dir = "./output/q1-sync"

[defaults]
concurrency = 4
checkpoint = { every = "Batch" }

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

# ── Sources ──────────────────────────────────────────────────────

[sources.engineering-drive]
kind = "google_drive"
credentials = { type = "env", env = "GOOGLE_CREDENTIALS" }
root_folders = ["1abc123def456"]
file_types = [
    { extension = "docx" },
    { extension = "pdf" },
    { mime = "application/vnd.google-apps.document" },
]
modified_after = "last_run"

  [[sources.engineering-drive.filters]]
  pattern = "**/Archive/**"
  action = "Exclude"

  [[sources.engineering-drive.filters]]
  pattern = "**"
  action = "Include"

[sources.team-slack]
kind = "slack"
credentials = { type = "env", env = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE", "C05678FGHIJ"]
thread_depth = 3
modified_after = "2026-01-01T00:00:00Z"

# ── Stages ───────────────────────────────────────────────────────
# Execution order is computed from resource declarations, not
# declaration order. Stages that touch independent resources
# run in parallel.

[stages.fetch-gdrive]
adapter = "extract"
source = "engineering-drive"
resources = { reads = ["gdrive-api"], creates = ["raw-gdrive-docs"] }
retry = { max_attempts = 3, initial_backoff_ms = 1000, backoff_multiplier = 2.0, max_backoff_ms = 30000 }
timeout_secs = 300

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = { reads = ["slack-api"], creates = ["raw-slack-messages"] }
retry = { max_attempts = 3, initial_backoff_ms = 500, backoff_multiplier = 2.0, max_backoff_ms = 10000 }

[stages.normalize-gdrive]
adapter = "normalize"
source = "engineering-drive"
resources = { reads = ["raw-gdrive-docs"], creates = ["normalized-docs"] }

[stages.normalize-slack]
adapter = "slack-normalize"
source = "team-slack"
resources = { reads = ["raw-slack-messages"], creates = ["normalized-messages"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["normalized-docs", "normalized-messages"] }

[stages.emit.params]
subdir = "normalized"
```

---

## 2. Layer 2: Topology (Resolved at Init)

The topology is the concrete, wired-up execution plan. It's computed from the
specification, resolves all references, validates the resource graph, and is
immutable during execution.

### Key Design Decision: Resource-Based Scheduling

Execution order is derived from resource declarations, not explicit edges or
declaration order. This is dagga's core insight adapted to our use case:

- Stages that touch independent resources run in parallel (same batch).
- Stages that read a resource another stage creates run in a later batch.
- Write-write conflicts on the same resource force sequential execution.

This eliminates manual dependency wiring while naturally expressing the
parallelism in our pipelines (fetch-gdrive and fetch-slack are independent;
normalize-gdrive depends on fetch-gdrive's output).

### Computed Schedule

The scheduler produces `Vec<Vec<StageId>>` — batches of stages that can
execute concurrently. Each batch boundary is a natural checkpoint point.

For the example TOML above:

```
Batch 0: [fetch-gdrive, fetch-slack]           # independent sources
Batch 1: [normalize-gdrive, normalize-slack]    # each reads its own source
Batch 2: [emit]                                 # reads both normalized outputs
```

### Missing Input Detection

Before execution begins, the topology layer validates that every resource
declared as `reads` by any stage is either declared as `creates` by an earlier
stage or is an external resource (like `"gdrive-api"`). This catches
configuration errors at init time, not mid-run. (From dagga's
`get_missing_inputs()` pattern.)

External resources (API clients, filesystem paths) are declared in `reads`
but never in `creates` — the scheduler treats them as always-available.

### Topology Types

```rust
use std::sync::Arc;

/// The resolved pipeline, ready to execute.
/// Computed from PipelineSpec at init time. Immutable during execution.
#[derive(Debug, Clone)]
pub struct PipelineTopology {
    /// The original spec, preserved for checkpoint embedding.
    pub spec: Arc<PipelineSpec>,

    /// Blake3 hash of the serialized spec, for detecting config drift
    /// between a checkpoint and the current TOML file.
    pub spec_hash: Blake3Hash,

    /// Resolved source adapters, keyed by source name from the spec.
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,

    /// Resolved stage implementations, keyed by stage name from the spec.
    pub stages: BTreeMap<String, ResolvedStage>,

    /// The computed execution schedule: batches of parallel stages.
    /// Each inner Vec contains stages that can run concurrently.
    pub schedule: Vec<Vec<StageId>>,

    /// Resolved output directory (created if needed at init).
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResolvedStage {
    /// Name from the spec (stable across runs for checkpointing).
    pub id: StageId,

    /// The concrete stage implementation.
    pub handler: Arc<dyn Stage>,

    /// Resolved retry policy (stage override merged with global default).
    pub retry: RetryPolicy,

    /// Skip-on-error behavior for item-level failures.
    pub skip_on_error: bool,

    /// Timeout for stage execution.
    pub timeout: Option<Duration>,

    /// Which source this stage operates on (for extract stages).
    pub source: Option<String>,

    /// Condition predicate (None = always run).
    pub condition: Option<ConditionExpr>,
}

/// Retry policy with resolved, concrete values.
/// (From oxigdal-workflow — four fields, minimal and complete.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub backoff_multiplier: f64,
    pub max_backoff: Duration,
}

/// Deterministic, name-based stage identifier.
/// NOT an auto-incremented integer (dagrs anti-pattern).
/// Stable across runs for checkpoint compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);

impl StageId {
    pub fn new(name: impl Into<String>) -> Self { Self(name.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for StageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Blake3 content hash, stored as hex string for JSON readability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);

impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self { Self(hex.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn is_empty(&self) -> bool { self.0.is_empty() }
}
```

### Topology Resolution

```rust
impl PipelineTopology {
    pub async fn resolve(spec: PipelineSpec) -> Result<Self, ResolveError> {
        // 1. Hash the spec for config drift detection.
        let spec_bytes = toml::to_string(&spec)?;
        let spec_hash = Blake3Hash::new(blake3::hash(spec_bytes.as_bytes()).to_hex().to_string());
        let spec = Arc::new(spec);

        // 2. Resolve each source into a concrete adapter.
        let sources = spec.sources.iter()
            .map(|(name, source_spec)| {
                let adapter = resolve_source_adapter(name, source_spec)?;
                Ok((name.clone(), adapter))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        // 3. Resolve each stage into a concrete handler.
        let stages = spec.stages.iter()
            .map(|(name, stage_spec)| {
                let resolved = resolve_stage(name, stage_spec, &spec.defaults)?;
                Ok((name.clone(), resolved))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        // 4. Build the resource graph and validate.
        let resource_graph = ResourceGraph::build(&spec.stages)?;
        resource_graph.validate_no_missing_inputs()?;
        resource_graph.validate_no_cycles()?;

        // 5. Compute the parallel schedule.
        let schedule = resource_graph.compute_schedule()?;

        // 6. Create output directory (async to avoid blocking the runtime;
        //    see AP-18: sync I/O in async context).
        let output_dir = spec.output_dir.clone();
        tokio::fs::create_dir_all(&output_dir).await?;

        Ok(Self { spec, spec_hash, sources, stages, schedule, output_dir })
    }
}
```

### Resource Graph

```rust
/// The resource graph: stages connected by shared resource declarations.
/// Used to compute the parallel execution schedule.
#[derive(Debug)]
struct ResourceGraph {
    /// Which stages create each resource.
    creators: BTreeMap<String, StageId>,
    /// Which stages read each resource.
    readers: BTreeMap<String, Vec<StageId>>,
    /// Which stages write (exclusively) to each resource.
    writers: BTreeMap<String, Vec<StageId>>,
}

impl ResourceGraph {
    /// Validate that every resource read by a stage is either created by
    /// an earlier stage or is an external resource (never created by anyone).
    ///
    /// A resource that appears in `reads` but NOT in `creators` is treated
    /// as external (API client, filesystem path) — always available.
    /// A resource that appears in BOTH `reads` and `creators` is internal
    /// and the scheduler will enforce ordering. The error case is a resource
    /// that is read but was supposed to be created (appears in `creators`
    /// for a stage that doesn't actually exist) — but that's caught by
    /// stage resolution. The real validation here: every `reads` entry
    /// must either be external or have a creator.
    fn validate_no_missing_inputs(&self) -> Result<(), ResolveError> {
        // All resources that are read are either external (not in creators)
        // or internal (in creators). Both are valid. The actual missing-input
        // case would be if a stage references a resource that no other stage
        // creates AND is not a known external resource. For now, we treat
        // any resource not in `creators` as external.
        //
        // TODO: Add an explicit `externals` set so we can distinguish
        // "intentionally external" from "typo in resource name."
        Ok(())
    }

    /// Compute parallel batches via topological sort.
    /// Stages in the same batch touch independent resources.
    fn compute_schedule(&self) -> Result<Vec<Vec<StageId>>, ResolveError> {
        // Simplified: topological sort, then group into layers where
        // no resource conflicts exist within a layer.
        // Full implementation uses the resource access patterns to
        // determine which stages can run concurrently.
        todo!("Topological sort + resource-conflict layer splitting")
    }
}
```

---

## 3. Layer 3: State (Mutates During Execution)

This is the heart of the "state IS the pipeline" insight. The state is
item-centric: every item carries its own status, provenance, and completion
history. Stage-level aggregates (counts, timing) are derived from item state.

### Design Decisions Resolved

**Open Question #4 (checkpoint granularity):** Checkpoint after every batch
by default. Per-item checkpointing within a batch is available via the
`CheckpointStrategy::Items` config. Batch-level is the sweet spot: frequent
enough for durability, infrequent enough for performance.

**Open Question #3 (type safety of inter-stage data):** `PipelineItem` is the
universal envelope (bytes + MIME type + metadata map). Compile-time type safety
between stages (dagx's approach) is incompatible with TOML-driven dynamic
pipelines. The tradeoff: we accept runtime type checking in exchange for
serializability and configurability. The `metadata` map on `PipelineItem`
provides structured typed data via `serde_json::Value`.

**Crash recovery:** `prepare_for_resume()` resets any items stuck in
`Processing` status back to `Pending`. (From oxigdal-workflow — the original
proposal didn't handle this case.)

### State Types

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete pipeline execution state.
/// Serialize this at any point → perfect checkpoint.
/// Hand it to Claude → full system understanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Unique identifier for this run.
    pub run_id: RunId,

    /// Pipeline identity (from spec).
    pub pipeline_name: String,

    /// When this run started.
    pub started_at: DateTime<Utc>,

    /// When the last checkpoint was written.
    pub last_checkpoint: DateTime<Utc>,

    /// Overall pipeline status.
    pub status: PipelineStatus,

    /// Which batch is currently executing (index into schedule).
    pub current_batch: usize,

    /// Per-source extraction state (items, discovery counts, etc.).
    pub sources: BTreeMap<String, SourceState>,

    /// Per-stage execution state (timing, counts, errors).
    pub stages: BTreeMap<StageId, StageState>,

    /// Summary statistics (derived, but cached for observability).
    pub stats: PipelineStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    pub fn new(id: impl Into<String>) -> Self { Self(id.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running { current_stage: String },
    Completed { finished_at: DateTime<Utc> },
    Failed { error: String, failed_at: DateTime<Utc> },
    /// Was interrupted and can be resumed.
    Interrupted { interrupted_at: DateTime<Utc> },
}

/// Per-source state: what items were discovered, accepted, and processed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState {
    /// How many items were discovered by enumeration.
    pub items_discovered: usize,

    /// How many items passed filters.
    pub items_accepted: usize,

    /// How many items were skipped due to unchanged content hash.
    pub items_skipped_unchanged: usize,

    /// Per-item state for items that entered the pipeline.
    /// Key: source-specific item ID.
    pub items: BTreeMap<String, ItemState>,
}

/// The state of a single item flowing through the pipeline.
/// This is the atomic unit of work and the atomic unit of checkpointing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    /// Human-readable identifier (filename, message preview, etc.).
    pub display_name: String,

    /// Source-specific unique identifier.
    pub source_id: String,

    /// Which source this item came from (key into PipelineState.sources).
    pub source_name: String,

    /// Blake3 hash of the source content, for incrementality.
    pub content_hash: Blake3Hash,

    /// Current processing status.
    pub status: ItemStatus,

    /// Which stages have been completed for this item, with timing.
    pub completed_stages: Vec<CompletedStageRecord>,

    /// Provenance: where did this item come from and when?
    pub provenance: ItemProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Discovered but not yet processed.
    Pending,
    /// Currently being processed by a stage.
    Processing { stage: String },
    /// All applicable stages completed successfully.
    Completed,
    /// Failed at a stage; may be retryable.
    Failed { stage: String, error: String, attempts: u32 },
    /// Skipped due to skip_on_error or conditional stage.
    Skipped { stage: String, reason: String },
    /// Skipped due to unchanged content hash (incrementality).
    Unchanged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStageRecord {
    pub stage: StageId,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    /// Source service type (e.g., "google_drive", "slack").
    pub source_kind: String,

    /// Source-specific metadata.
    /// For Drive: { "file_id": "...", "path": "/Engineering/doc.docx", "owner": "..." }
    /// For Slack: { "channel": "...", "thread_ts": "...", "author": "..." }
    pub metadata: BTreeMap<String, serde_json::Value>,

    /// When the source item was last modified (per the source API).
    pub source_modified: Option<DateTime<Utc>>,

    /// When we extracted it.
    pub extracted_at: DateTime<Utc>,
}

/// Per-stage aggregate state (derived from item states, cached).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub status: StageStatus,
    pub items_processed: usize,
    pub items_failed: usize,
    pub items_skipped: usize,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Completed,
    Skipped { reason: String },
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
}
```

### The Checkpoint: Self-Contained Recovery Artifact

The checkpoint bundles all three layers so recovery is fully self-contained.
No need to re-parse TOML or re-resolve topology on resume — everything needed
is in the checkpoint. (From oxigdal-workflow's versioned checkpoint design.)

```rust
/// The checkpoint: a complete, self-contained recovery artifact.
/// Bundles all three layers so resume needs nothing external.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Checkpoint format version for forward compatibility.
    pub version: u32,

    /// Monotonically increasing sequence number within a run.
    pub sequence: u64,

    /// When this checkpoint was created.
    pub created_at: DateTime<Utc>,

    /// The full specification (immutable, embedded for self-containedness).
    pub spec: PipelineSpec,

    /// The computed schedule (immutable, embedded).
    pub schedule: Vec<Vec<StageId>>,

    /// The spec hash at the time this run began.
    pub spec_hash: Blake3Hash,

    /// The mutable execution state.
    pub state: PipelineState,
}

impl Checkpoint {
    /// Prepare a checkpoint for resume after a crash.
    /// Resets any items stuck in Processing back to Pending.
    /// (From oxigdal-workflow's prepare_for_resume pattern.)
    pub fn prepare_for_resume(&mut self) {
        // Reset pipeline status.
        self.state.status = PipelineStatus::Interrupted {
            interrupted_at: self.created_at,
        };

        // Reset any items stuck mid-processing.
        for source_state in self.state.sources.values_mut() {
            for item_state in source_state.items.values_mut() {
                if matches!(item_state.status, ItemStatus::Processing { .. }) {
                    item_state.status = ItemStatus::Pending;
                }
            }
        }

        // Reset any stages stuck in Running.
        for stage_state in self.state.stages.values_mut() {
            if matches!(stage_state.status, StageStatus::Running) {
                stage_state.status = StageStatus::Pending;
            }
        }
    }

    /// Check whether the current TOML config has drifted from the
    /// checkpoint's embedded spec.
    pub fn config_drifted(&self, current_spec_hash: &Blake3Hash) -> bool {
        self.spec_hash != *current_spec_hash
    }
}
```

### Config Drift Policy

When resuming from a checkpoint, the runner compares the current TOML's spec
hash against the checkpoint's embedded spec hash. If they differ:

1. **Warn** with a clear message showing what changed.
2. **Refuse to resume** by default — the user must either revert the TOML or
   start a fresh run.
3. **Allow override** via `--force-resume` flag, which uses the checkpoint's
   embedded spec (ignoring the current TOML).

This prevents the subtle bugs that arise from resuming a half-finished run
with different configuration.

---

## 4. Core Traits

### The SourceAdapter Trait

The boundary between the messy external world and the clean pipeline.
Everything about OAuth, pagination, rate limiting, and format detection lives
behind this trait.

**Key design decision:** `enumerate()` and `fetch()` are separate operations.
Enumeration is cheap (API metadata call); fetching is expensive (content
download). This separation is essential for incrementality — we compare content
hashes *before* paying the fetch cost.

**Object safety:** `SourceAdapter` is object-safe (no associated types) so
adapters can be stored as `Arc<dyn SourceAdapter>` in the topology. This is
the right call for a TOML-driven system where adapter types are resolved at
runtime. (The synthesis's associated-type `SourceAdapter<Item>` was elegant
but incompatible with dynamic dispatch.)

```rust
// Note: `async_trait` is still required here despite Rust 1.85+ supporting
// native `async fn` in traits. Native async trait methods are not object-safe:
// `dyn SourceAdapter` requires the future to be boxed, which `async_trait`
// handles automatically. When Rust stabilizes `dyn`-compatible async traits
// (via return-type notation or similar), this can be removed.
use async_trait::async_trait;

/// A source adapter handles all interaction with an external data service.
///
/// Implementors handle: authentication, enumeration, filtering, pagination,
/// rate limiting, and fetching. The pipeline runner sees only the trait.
///
/// Object-safe by design: adapters are resolved from TOML config at runtime
/// and stored as Arc<dyn SourceAdapter>.
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Human-readable name of the source type (e.g., "Google Drive").
    fn source_kind(&self) -> &str;

    /// Enumerate items available from this source.
    /// Returns lightweight descriptors (no content) for filtering and
    /// hash comparison. This is the "what's there?" step.
    ///
    /// The adapter applies source-level filters (folder IDs, file types,
    /// modified_after) during enumeration.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    /// Separate from enumerate() because fetching is expensive and we
    /// want to skip unchanged items before paying this cost.
    ///
    /// This is the "go get it" step.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by enumerate().
/// Contains enough metadata for filtering and hash comparison,
/// but does NOT contain the actual content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    /// Source-specific unique identifier.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// MIME type (for filtering by file type).
    pub mime_type: String,

    /// Path within the source (for glob-based filtering).
    pub path: String,

    /// Last modified timestamp (for incremental sync).
    pub modified_at: Option<DateTime<Utc>>,

    /// Content hash if cheaply available from the source API.
    /// Google Drive provides md5Checksum; Slack provides message hash.
    /// If None, the pipeline fetches content and computes blake3.
    pub source_hash: Option<String>,
}

/// A document extracted from a source, in its original format.
/// This is the raw material before any normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Unique identifier within this source.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// The raw content bytes.
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,

    /// MIME type of the content, as reported by the source.
    pub mime_type: String,

    /// Provenance metadata.
    pub provenance: ItemProvenance,

    /// Content hash (blake3 of content bytes).
    pub content_hash: Blake3Hash,
}
```

### The Stage Trait

Stages transform items as they flow through the pipeline.

**Design decision (resolves Open Question #1):** The signature is
`process(item) -> Result<Vec<PipelineItem>>`. One item in, zero or more out.
This handles the common case (1:1 transform), fan-out (splitting a document
into sections), and filtering (returning empty vec). Inter-stage streaming
is deferred — batch processing with checkpointing is simpler and sufficient.

**Design decision (resolves Open Question #5, DAG vs linear):** DAG from
the start. The resource-based scheduling gives us parallel execution of
independent stages for free. But individual stages still process items
sequentially (or with bounded concurrency — see the runner). This is the
pragmatic middle ground: DAG at the stage level, sequential (with optional
parallelism) at the item level.

**Fan-in / aggregation:** Most stages are item-level transforms. But some
stages need to aggregate across all items (e.g., deduplication, synthesis).
These stages implement the same `Stage` trait but receive items tagged with
their source via `PipelineItem.source_name`. The runner collects all items
from the stage's input resources before invoking an aggregation stage. This
is signaled by the stage's resource declarations: if a stage reads multiple
resources, the runner passes it the combined item set.

```rust
use std::sync::Arc;

/// The intermediate representation flowing between stages.
/// Starts life as an ExtractedDocument, accumulates transformations.
///
/// Uses `Arc<[u8]>` for content to enable zero-copy cloning in hot paths.
/// `PipelineItem` is cloned when fanning out to concurrent tasks and when
/// building retry attempts — `Arc<[u8]>` makes these O(1) instead of O(n).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    /// The item's unique identifier (stable across stages).
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// Current content (may be transformed by prior stages).
    /// Wrapped in `Arc` for zero-copy cloning in concurrent pipelines.
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,

    /// Current MIME type (changes as content is transformed,
    /// e.g., "application/pdf" → "text/markdown").
    pub mime_type: String,

    /// Which source this item came from.
    pub source_name: String,

    /// Content hash of the original source content (for incrementality).
    pub source_content_hash: Blake3Hash,

    /// Provenance chain.
    pub provenance: ItemProvenance,

    /// Metadata accumulated by stages. Each stage can add key-value pairs.
    /// Structured as serde_json::Value for flexibility without losing
    /// serializability. (Avoids the synthesis's Arc<dyn Any> anti-pattern.)
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
///
/// Stages are intentionally simple: one item in, zero or more out.
/// The runner handles orchestration, retries, checkpointing, and
/// concurrency. The stage handles only transformation logic.
#[async_trait]
pub trait Stage: Send + Sync {
    /// Human-readable name of this stage type.
    fn name(&self) -> &str;

    /// Process a single item. Returns:
    /// - Ok(vec![item]) — item transformed successfully (common case)
    /// - Ok(vec![item1, item2, ...]) — item split into multiple (fan-out)
    /// - Ok(vec![]) — item filtered out / consumed
    /// - Err(e) — processing failed
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
/// Immutable — stages cannot mutate prior outputs or pipeline state.
/// (Addresses erio-workflow's &mut WorkflowContext anti-pattern.)
#[derive(Debug, Clone)]
pub struct StageContext {
    /// The original pipeline specification.
    pub spec: Arc<PipelineSpec>,

    /// The output directory for this pipeline run.
    pub output_dir: PathBuf,

    /// Stage-specific parameters from the pipeline config.
    pub params: serde_json::Value,

    /// Tracing span for structured logging within this stage.
    pub span: tracing::Span,
}
```

### The StateStore Trait

Persistence abstraction for checkpoints and content hashes. The default
implementation uses redb; the trait exists for testability (in-memory store
for tests).

```rust
/// Persistence backend for pipeline state.
/// Default: RedbStateStore (single-file, ACID, crash-safe).
/// Test: InMemoryStateStore.
///
/// Trait-based so the persistence backend can be swapped without
/// changing the runner. (Avoids oxigdal-workflow's filesystem-only
/// anti-pattern.)
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save a checkpoint (atomic write).
    /// Implementation must be crash-safe: either the full checkpoint
    /// is persisted or none of it is. redb provides this via ACID
    /// transactions; filesystem impls must use write-then-rename.
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StateError>;

    /// Load the most recent checkpoint, if one exists.
    async fn load_checkpoint(&self) -> Result<Option<Checkpoint>, StateError>;

    /// Load content hashes from the most recent *completed* run.
    /// Used for cross-run incrementality.
    async fn load_previous_hashes(&self) -> Result<BTreeMap<String, Blake3Hash>, StateError>;

    /// Save content hashes at the end of a successful run.
    async fn save_completed_hashes(
        &self,
        run_id: &RunId,
        hashes: &BTreeMap<String, Blake3Hash>,
    ) -> Result<(), StateError>;
}
```

---

## 5. The Pipeline Runner

The runner is the orchestrator. It owns the topology and state, executes
batches in order, manages concurrency within batches, checkpoints at batch
boundaries, and supports resume.

**Design decision (resolves Open Question #2, concurrency within a stage):**
Items within a stage are processed with bounded concurrency, controlled by
`defaults.concurrency`. A `tokio::JoinSet` with a semaphore limits concurrent
item processing. This is the right model for API-bound work (fetching from
Drive) where sequential processing wastes most of its time waiting.

**Snapshot isolation for parallel stages:** Before executing a batch's stages
in parallel, the runner builds an immutable `StageContext` snapshot. Each stage
in the batch gets the same view of completed outputs. After all stages in the
batch complete, their results are merged into the shared state. No shared
mutable state during execution. (From erio-workflow.)

```rust
#[derive(Debug)]
pub struct PipelineRunner {
    topology: PipelineTopology,
    state: PipelineState,
    store: Box<dyn StateStore>,
}

impl PipelineRunner {
    /// Create a new runner from a specification.
    pub async fn new(spec: PipelineSpec) -> Result<Self, PipelineError> {
        let topology = PipelineTopology::resolve(spec).await?;
        let store = RedbStateStore::open(&topology.output_dir)?;

        let state = match store.load_checkpoint().await? {
            Some(mut checkpoint) => {
                // Config drift check.
                if checkpoint.config_drifted(&topology.spec_hash) {
                    return Err(PipelineError::ConfigDrift {
                        checkpoint_hash: checkpoint.spec_hash.clone(),
                        current_hash: topology.spec_hash.clone(),
                    });
                }
                // Reset any items/stages stuck mid-processing.
                checkpoint.prepare_for_resume();
                tracing::info!(
                    run_id = %checkpoint.state.run_id,
                    sequence = checkpoint.sequence,
                    "Resuming from checkpoint",
                );
                checkpoint.state
            }
            None => PipelineState::new(&topology),
        };

        Ok(Self { topology, state, store: Box::new(store) })
    }

    /// Execute the pipeline.
    pub async fn run(&mut self) -> Result<&PipelineState, PipelineError> {
        // Phase 1: Enumerate items from all sources (if not already done).
        if self.state.current_batch == 0 && self.state.stats.total_items_discovered == 0 {
            self.enumerate_sources().await?;
            self.apply_incrementality().await?;
            self.checkpoint().await?;
        }

        // Phase 2: Execute batches.
        let schedule = self.topology.schedule.clone();
        for (batch_idx, batch) in schedule.iter().enumerate() {
            if batch_idx < self.state.current_batch {
                // Already completed in a prior run — skip.
                continue;
            }

            self.execute_batch(batch_idx, batch).await?;
            self.state.current_batch = batch_idx + 1;
            self.checkpoint().await?;
        }

        // Phase 3: Finalize.
        self.save_completed_hashes().await?;
        self.state.status = PipelineStatus::Completed {
            finished_at: Utc::now(),
        };
        self.checkpoint().await?;

        Ok(&self.state)
    }

    /// Execute a single batch: stages in this batch run concurrently.
    async fn execute_batch(
        &mut self,
        batch_idx: usize,
        stages: &[StageId],
    ) -> Result<(), PipelineError> {
        // Build immutable context snapshot for this batch.
        let ctx = self.build_stage_context();

        // Check conditional stages — skip those whose predicates are false.
        let active_stages: Vec<&StageId> = stages.iter()
            .filter(|id| self.should_execute_stage(id))
            .collect();

        // Execute stages concurrently (one tokio task per stage).
        let mut join_set = tokio::task::JoinSet::new();
        for stage_id in &active_stages {
            let stage = self.topology.stages[*stage_id].clone();
            let items = self.collect_items_for_stage(stage_id);
            let ctx = ctx.clone();
            let concurrency = self.topology.spec.defaults.concurrency;

            join_set.spawn(async move {
                execute_stage_items(stage, items, ctx, concurrency).await
            });
        }

        // Collect results and merge into state.
        while let Some(result) = join_set.join_next().await {
            let stage_result = result??;
            self.merge_stage_result(stage_result);
        }

        Ok(())
    }

    /// Enumerate all sources and populate the item list.
    async fn enumerate_sources(&mut self) -> Result<(), PipelineError> {
        for (name, adapter) in &self.topology.sources {
            let items = adapter.enumerate().await.map_err(|e| {
                PipelineError::SourceEnumeration {
                    source: name.clone(),
                    error: e.to_string(),
                }
            })?;

            let source_state = self.state.sources
                .entry(name.clone())
                .or_insert_with(SourceState::default);

            source_state.items_discovered = items.len();

            for item in items {
                source_state.items_accepted += 1;
                source_state.items.insert(item.id.clone(), ItemState {
                    display_name: item.display_name.clone(),
                    source_id: item.id.clone(),
                    source_name: name.clone(),
                    content_hash: Blake3Hash::new(""),
                    status: ItemStatus::Pending,
                    completed_stages: vec![],
                    provenance: ItemProvenance {
                        source_kind: adapter.source_kind().to_string(),
                        metadata: BTreeMap::new(),
                        source_modified: item.modified_at,
                        extracted_at: Utc::now(),
                    },
                });
            }
        }
        Ok(())
    }

    /// Compare content hashes against previous run; mark unchanged items.
    async fn apply_incrementality(&mut self) -> Result<(), PipelineError> {
        let previous_hashes = self.store.load_previous_hashes().await?;

        for source_state in self.state.sources.values_mut() {
            // Count skipped items separately to avoid borrowing
            // `source_state` mutably while iterating `source_state.items`.
            let mut skipped = 0usize;
            for (item_id, item_state) in source_state.items.iter_mut() {
                if let Some(prev_hash) = previous_hashes.get(item_id) {
                    // Use the source-provided hash if available (cheap),
                    // otherwise defer to post-fetch blake3 comparison.
                    if !prev_hash.is_empty() && item_state.content_hash == *prev_hash {
                        item_state.status = ItemStatus::Unchanged;
                        skipped += 1;
                    }
                }
            }
            source_state.items_skipped_unchanged += skipped;
        }
        self.state.update_stats();
        Ok(())
    }

    /// Build a checkpoint and persist it.
    async fn checkpoint(&mut self) -> Result<(), PipelineError> {
        let checkpoint = Checkpoint {
            version: 1,
            sequence: self.next_sequence(),
            created_at: Utc::now(),
            spec: (*self.topology.spec).clone(),
            schedule: self.topology.schedule.clone(),
            spec_hash: self.topology.spec_hash.clone(),
            state: self.state.clone(),
        };
        self.store.save_checkpoint(&checkpoint).await?;
        self.state.last_checkpoint = checkpoint.created_at;
        Ok(())
    }
}

/// Execute a single stage's items with bounded concurrency.
/// Runs as an independent task — no shared mutable state.
async fn execute_stage_items(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
    concurrency: usize,
) -> Result<StageResult, PipelineError> {
    let semaphore = Arc::new(tokio::sync::Semaphore::new(concurrency));
    let mut join_set = tokio::task::JoinSet::new();

    for item in items {
        let permit = semaphore.clone().acquire_owned().await?;
        let handler = stage.handler.clone();
        let ctx = ctx.clone();
        let retry = stage.retry.clone();
        let skip_on_error = stage.skip_on_error;

        join_set.spawn(async move {
            let _permit = permit; // held until task completes
            let result = execute_with_retry(&handler, item.clone(), &ctx, &retry).await;
            (item.id.clone(), result, skip_on_error)
        });
    }

    let mut stage_result = StageResult::new(stage.id.clone());
    while let Some(result) = join_set.join_next().await {
        let (item_id, result, skip_on_error) = result?;
        match result {
            Ok(outputs) => stage_result.record_success(item_id, outputs),
            Err(e) if skip_on_error => stage_result.record_skipped(item_id, e),
            Err(e) => stage_result.record_failure(item_id, e),
        }
    }
    Ok(stage_result)
}

/// Execute a stage handler with retry and backoff.
///
/// Uses the `backon` crate (already a project dependency) instead of
/// hand-rolling retry logic. `backon` handles backoff calculation,
/// jitter, and retry counting correctly out of the box.
async fn execute_with_retry(
    handler: &Arc<dyn Stage>,
    item: PipelineItem,
    ctx: &StageContext,
    retry: &RetryPolicy,
) -> Result<Vec<PipelineItem>, StageError> {
    use backon::{ExponentialBuilder, Retryable};

    let backoff = ExponentialBuilder::default()
        .with_min_delay(retry.initial_backoff)
        .with_factor(retry.backoff_multiplier as f32)
        .with_max_delay(retry.max_backoff)
        .with_max_times(retry.max_attempts.saturating_sub(1) as usize);

    (|| async { handler.process(item.clone(), ctx).await })
        .retry(backoff)
        .await
}
```

---

## 6. Observability: What Claude Sees

When someone runs `ecl pipeline inspect ./output/q1-sync`, the JSON output
tells the full story:

```json
{
  "run_id": "run-2026-03-13-001",
  "pipeline_name": "q1-knowledge-sync",
  "started_at": "2026-03-13T14:30:00Z",
  "last_checkpoint": "2026-03-13T14:47:22Z",
  "status": { "Running": { "current_stage": "normalize-gdrive" } },
  "current_batch": 1,
  "sources": {
    "engineering-drive": {
      "items_discovered": 200,
      "items_accepted": 187,
      "items_skipped_unchanged": 142,
      "items": {
        "1abc...": {
          "display_name": "Q1 Architecture Review.docx",
          "source_id": "1abc...",
          "source_name": "engineering-drive",
          "content_hash": "a7f3b2...",
          "status": "Completed",
          "completed_stages": [
            { "stage": "fetch-gdrive", "completed_at": "...", "duration_ms": 1200 },
            { "stage": "normalize-gdrive", "completed_at": "...", "duration_ms": 340 }
          ],
          "provenance": {
            "source_kind": "google_drive",
            "metadata": {
              "file_id": "1abc...",
              "path": "/Engineering/Architecture/Q1 Architecture Review.docx",
              "owner": "alice@company.com"
            },
            "source_modified": "2026-03-10T09:15:00Z",
            "extracted_at": "2026-03-13T14:31:12Z"
          }
        },
        "2def...": {
          "display_name": "Meeting Notes (old).pdf",
          "status": {
            "Failed": {
              "stage": "normalize-gdrive",
              "error": "PDF conversion failed: encrypted document",
              "attempts": 3
            }
          }
        }
      }
    },
    "team-slack": {
      "items_discovered": 847,
      "items_accepted": 312,
      "items_skipped_unchanged": 280,
      "items": { "..." : "..." }
    }
  },
  "stages": {
    "fetch-gdrive": { "status": "Completed", "items_processed": 45, "items_failed": 0 },
    "fetch-slack": { "status": "Completed", "items_processed": 32, "items_failed": 0 },
    "normalize-gdrive": { "status": "Running", "items_processed": 38, "items_failed": 2 },
    "normalize-slack": { "status": "Completed", "items_processed": 32, "items_failed": 0 },
    "emit": { "status": "Pending", "items_processed": 0, "items_failed": 0 }
  },
  "stats": {
    "total_items_discovered": 1047,
    "total_items_processed": 147,
    "total_items_skipped_unchanged": 422,
    "total_items_failed": 2
  }
}
```

From this, Claude can immediately say: "You're in batch 1 — both fetch stages
completed, normalize-slack finished, and normalize-gdrive is still running with
38 of 45 items done. 422 items across both sources were skipped as unchanged.
Two items failed normalization — one is an encrypted PDF. The emit stage hasn't
started yet. At the current rate, you'll finish in about 2 minutes."

No logs. No dashboards. Just state.

---

## 7. Incrementality Flow

```
                    ┌────────────────────┐
                    │  Load prev hashes  │
                    │  from redb         │
                    │  (last completed   │
                    │   run)             │
                    └────────┬───────────┘
                             │
                             V
┌──────────────┐    ┌────────────────┐    ┌───────────────────┐
│  Enumerate   │───>│  For each item │───>│ Source provides   │
│  source      │    │                │    │ cheap hash?       │
└──────────────┘    └────────────────┘    └────────┬──────────┘
                                                   │
                                         ┌─────────┴─────────┐
                                         │ YES               │ NO
                                         V                   V
                                  ┌─────────────┐   ┌──────────────┐
                                  │ Compare to  │   │ Mark as      │
                                  │ prev hash   │   │ Pending      │
                                  └──────┬──────┘   │ (fetch will  │
                                         │          │  compute     │
                                  ┌──────┴──────┐   │  blake3)     │
                                  │             │   └──────────────┘
                                  V             V
                          ┌────────────┐  ┌────────────┐
                          │ UNCHANGED  │  │ PROCESS    │
                          │ (skip)     │  │ (changed)  │
                          └────────────┘  └─────┬──────┘
                                                │
                                                V
                                       ┌────────────────┐
                                       │ After fetch:   │
                                       │ compute blake3 │
                                       │ save to state  │
                                       └────────────────┘
```

The two-tier hash strategy: use the source API's hash (Drive's `md5Checksum`,
Slack's message hash) for cheap pre-fetch comparison when available. Fall back
to blake3 of fetched content when the source doesn't provide a hash or when
content has changed. Blake3 hashes are stored in redb and survive across runs.

---

## 8. Crate Structure

```
ecl/crates/
├── ecl-core/              # Existing: WorkflowId, StepResult, LlmProvider, etc.
│                          # NOT extended — pipeline types live in new crates.
│
├── ecl-pipeline-spec/     # NEW: Specification layer
│   └── src/
│       ├── lib.rs             # PipelineSpec, SourceSpec, StageSpec
│       ├── source.rs          # SourceSpec variants, FilterRule, CredentialRef
│       ├── stage.rs           # StageSpec, ResourceSpec
│       ├── defaults.rs        # DefaultsSpec, RetrySpec, CheckpointStrategy
│       └── validation.rs      # Spec-level validation (before topology)
│
├── ecl-pipeline-topo/     # NEW: Topology layer
│   └── src/
│       ├── lib.rs             # PipelineTopology, ResolvedStage
│       ├── resolve.rs         # Spec → Topology resolution
│       ├── resource_graph.rs  # Resource graph, missing input detection
│       └── schedule.rs        # Parallel batch computation
│
├── ecl-pipeline-state/    # NEW: State layer
│   └── src/
│       ├── lib.rs             # PipelineState, ItemState, Checkpoint
│       ├── store.rs           # StateStore trait
│       ├── redb_store.rs      # RedbStateStore implementation
│       └── memory_store.rs    # InMemoryStateStore (for tests)
│
├── ecl-pipeline/          # NEW: Engine + runner (facade crate)
│   └── src/
│       ├── lib.rs             # Re-exports, PipelineRunner
│       ├── runner.rs          # Batch execution, stage orchestration
│       ├── retry.rs           # Retry with exponential backoff
│       └── traits.rs          # Stage trait, SourceAdapter trait
│
├── ecl-adapter-gdrive/    # NEW: Google Drive source adapter
│   └── src/
│       ├── lib.rs
│       ├── auth.rs            # OAuth2 via yup-oauth2
│       ├── enumerate.rs       # Folder traversal, filtering
│       └── fetch.rs           # Document download, format detection
│
├── ecl-adapter-slack/     # NEW: Slack source adapter
│   └── src/
│       ├── lib.rs
│       ├── auth.rs            # Bot token management
│       ├── enumerate.rs       # Channel history, thread traversal
│       └── fetch.rs           # Message content extraction
│
├── ecl-adapter-fs/        # NEW: Filesystem source adapter (trivial, for testing)
│   └── src/lib.rs
│
├── ecl-stages/            # NEW: Built-in stage implementations
│   └── src/
│       ├── lib.rs
│       ├── extract.rs         # Delegates to SourceAdapter
│       ├── normalize.rs       # Format conversion → markdown
│       ├── filter.rs          # Glob-based include/exclude
│       └── emit.rs            # Write to output dir for Fabryk
│
└── ecl-cli/               # Existing, extended with pipeline commands
    └── src/
        └── pipeline.rs        # run, resume, status, inspect, items
```

### Design Decision (resolves Open Question #6): Separate Crates

Pipeline types do NOT go in `ecl-core`. The existing `ecl-core` has
`WorkflowId`, `StepResult`, `CritiqueDecision`, `LlmProvider` — these are
the AI workflow concepts. The pipeline runner is a different concern.

The existing `ecl-workflows` concept (`CritiqueLoopWorkflow`) will eventually
compose with the pipeline as a future `StageKind` — an AI-assisted stage that
uses `LlmProvider` for concept extraction or classification. That's a natural
extension point, not a merging of concerns.

The four pipeline crates (`spec`, `topo`, `state`, engine) mirror the three
layers exactly, with the engine crate pulling them together.

### Dependency Graph

```
ecl-pipeline (facade + engine)
  ├── ecl-pipeline-topo
  │     └── ecl-pipeline-spec
  ├── ecl-pipeline-state
  │     └── ecl-pipeline-spec
  ├── ecl-adapter-gdrive
  │     └── ecl-pipeline-spec  (for SourceAdapter trait + types)
  ├── ecl-adapter-slack
  │     └── ecl-pipeline-spec
  ├── ecl-adapter-fs
  │     └── ecl-pipeline-spec
  └── ecl-stages
        └── ecl-pipeline-spec
```

### CLI Commands

```
ecl pipeline run <config.toml>           # Execute a pipeline from config
ecl pipeline resume <output-dir>         # Resume from last checkpoint
ecl pipeline resume --force <output-dir> # Resume despite config drift
ecl pipeline status <output-dir>         # Human-readable progress summary
ecl pipeline inspect <output-dir>        # Full state as JSON (for Claude)
ecl pipeline items <output-dir>          # List all items with their status
ecl pipeline diff <dir1> <dir2>          # Compare two runs (what changed?)
```

---

## 9. Resolved Open Questions

| # | Question | Resolution | Source |
|---|----------|-----------|--------|
| 1 | Stage trait: `process(item) -> Vec<item>` vs richer protocol? | `process(item) -> Vec<item>` is sufficient. Fan-out via Vec, filtering via empty Vec. Streaming deferred. | Proposal + pragmatism |
| 2 | Concurrency within a stage? | Yes — bounded by `defaults.concurrency` via tokio semaphore. Essential for API-bound work. | Synthesis (dagx parallel execution) |
| 3 | Type safety of inter-stage data? | `PipelineItem` as universal envelope (bytes + MIME + metadata). Runtime typing. Compile-time safety incompatible with TOML-driven dynamic dispatch. | Proposal + synthesis anti-pattern #2 |
| 4 | Checkpoint granularity? | Per-batch by default, configurable to per-N-items. Batch boundaries are the natural checkpoint point in the resource-scheduled model. | Synthesis (layer-based execution) |
| 5 | DAG vs linear? | DAG from the start via resource-based scheduling. Degrades gracefully to linear when stages are fully sequential. Zero added complexity for users who don't need parallelism. | Synthesis (dagga resource model) |
| 6 | Where do pipeline types live? | New crates (`ecl-pipeline-spec`, `ecl-pipeline-topo`, `ecl-pipeline-state`, `ecl-pipeline`). NOT in `ecl-core`. AI workflow stages compose later. | Clean separation of concerns |

---

## 10. What This Design Borrows

### From the Design Proposal

- Item-centric model (items flow through stages, each with own status)
- `enumerate()` / `fetch()` split on SourceAdapter
- `SourceItem.source_hash` for cheap pre-fetch hash comparison
- `PipelineItem` as universal envelope (bytes + MIME + metadata)
- `skip_on_error` per-stage for best-effort extraction
- `ItemProvenance` with source-specific metadata
- CLI commands (`run`, `resume`, `status`, `inspect`, `items`)
- The JSON observability output format

### From the Crate Synthesis

- Resource-based implicit dependency scheduling (dagga)
- Layer/batch-based execution with checkpoint boundaries (dagx, erio-workflow)
- `Checkpoint` struct bundling spec + topology + state (oxigdal-workflow)
- `prepare_for_resume()` resetting stuck items (oxigdal-workflow)
- `backoff_multiplier` on retry policy (oxigdal-workflow)
- Snapshot isolation for parallel stages (erio-workflow)
- Conditional stage wrapper (erio-workflow)
- Missing input detection at init time (dagga)
- Deterministic name-based stage IDs, not auto-incremented (dagrs anti-pattern)
- `Serialize + Deserialize` on all outputs from day one (dagrs anti-pattern fix)
- Immutable `StageContext` (erio-workflow anti-pattern fix)

### From Rust Best Practices

- `BTreeMap` for deterministic serialization order
- `#[serde(default)]` for optional config fields
- `async_trait` for object-safe async trait methods (`dyn` dispatch)
- `Arc<dyn Trait>` for runtime-dispatched adapters
- Semaphore-bounded concurrency via `tokio::sync::Semaphore`
- ACID persistence via `redb` instead of filesystem write
- `blake3` for fast, high-quality content hashing
- `Arc<[u8]>` for zero-copy content cloning in concurrent paths
- `backon` crate for retry with exponential backoff (not hand-rolled)
- Private newtype internals with accessor methods
- `serde_json::Value` over `toml::Value` for format-agnostic params
- `tokio::fs` for async file operations (AP-18: no sync I/O in async)

---

## 11. What This Design Does NOT Cover (Future Work)

- **AI-assisted stages** — concept extraction, classification, synthesis.
  Future `Stage` implementations using `LlmProvider` from `ecl-core`.
- **Fabryk integration** — the `emit` stage writes files that `fabryk-content`
  can consume, but there's no direct API integration yet.
- **ACL / multi-tenancy** — deferred to Fabryk's `fabryk-acl` timeline.
- **The orchestrator / hub-and-spoke MCP design** — orthogonal concern.
- **Web UI or monitoring dashboard** — `inspect` + Claude is the
  observability story for now.
- **Fan-in aggregation stages** — the trait supports it (stages that read
  multiple resources get the combined item set), but no built-in aggregation
  stage exists yet.
- **Condition expression language** — the `condition` field on `StageSpec`
  accepts a string, but the expression evaluator is TBD.

---

## 12. PoC Scope

One source adapter (Google Drive) + minimal pipeline infrastructure:

1. `ecl-pipeline-spec` — TOML parsing, validation
2. `ecl-pipeline-state` — `PipelineState`, `Checkpoint`, `RedbStateStore`
3. `ecl-pipeline-topo` — topology resolution (linear schedule only for PoC;
   resource-based scheduling can use a simplified algorithm)
4. `ecl-pipeline` — runner with batch execution, retry, checkpointing
5. `ecl-adapter-gdrive` — enumerate + fetch from Google Drive
6. `ecl-stages` — extract, normalize (markdown), emit
7. `ecl-cli` — `pipeline run`, `pipeline resume`, `pipeline inspect`

The second adapter (Slack) tests whether the abstractions hold. If adding
Slack requires changing any trait signatures, the abstractions are wrong.

---

## 13. Version History

### v1.0

Document created.

### v1.1

Rust quality pass — fixed compilation issues, anti-patterns, and design
concerns identified during code review:

- **Fix:** Mutable borrow conflict in `apply_incrementality` — separated
  skip counter into local variable to avoid aliased `&mut` references
- **Fix:** `validate_no_missing_inputs` was a no-op — clarified semantics,
  added TODO for explicit externals set
- **Fix:** Sync I/O in async context (AP-18) — `std::fs::create_dir_all` →
  `tokio::fs::create_dir_all`; `resolve()` is now `async`
- **Fix:** Replaced hand-rolled retry loop with `backon` crate (already a
  project dependency)
- **Fix:** Made newtype internals private (`StageId`, `Blake3Hash`, `RunId`)
  with `new()`, `as_str()`, and `Display` accessors
- **Fix:** `#[serde(untagged)]` on `CredentialRef` → `#[serde(tag = "type")]`
  for reliable TOML deserialization; updated example TOML
- **Fix:** `toml::Value` for stage params → `serde_json::Value` for
  format-agnostic config (works in checkpoints too)
- **Fix:** `PipelineItem.content: Vec<u8>` → `Arc<[u8]>` for zero-copy
  cloning in concurrent hot paths
- **Add:** Manual `Default` impls for `DefaultsSpec`, `RetrySpec`,
  `SourceState`
- **Add:** `#[derive(Debug, Clone)]` on `PipelineTopology`, `ResolvedStage`,
  `StageContext`, `PipelineRunner`; `Debug` on `ResourceGraph`
- **Add:** `SlackSourceSpec` and `FilesystemSourceSpec` struct definitions
  (previously referenced but undefined)
- **Add:** Comment explaining why `async_trait` is still needed for
  `dyn`-dispatched traits despite MSRV 1.85
- **Update:** "From Rust Best Practices" section with new patterns
