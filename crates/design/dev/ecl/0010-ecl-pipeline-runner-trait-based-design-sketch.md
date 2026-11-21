---
number: TBD
title: "ECL Pipeline Runner: Trait-Based Design Sketch"
author: "Duncan McGreggor & Claude"
component: ecl-*
tags: [design, pipeline, traits, thought-experiment]
created: 2026-03-13
updated: 2026-03-13
state: Draft
supersedes: 4
superseded-by: null
version: 0.1
---

# ECL Pipeline Runner: Trait-Based Design Sketch

## A Concrete Thought Experiment

**Purpose**: Not a spec. A sufficiently concrete design to compare against
research results from the six-crate analysis. The goal is to have something
real enough to argue with.

**Governing Principles**:

- Five Pillars: Durability, Configurability, Observability, Incrementality,
  Composability
- Three Layers: Specification, Topology, State
- Weight Class: CLI tool, filesystem-based, no external services
- AI-Legible: Serialized state should be fully inspectable by Claude

---

## 1. The Three Layers as Rust Types

### Layer 1: Specification (from TOML)

The specification is what the user declares. It's parsed once, validated,
and never mutated.

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The root configuration, deserialized from TOML.
/// Immutable after load. This is the "what do you want to happen" layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Human-readable name for this pipeline run.
    pub name: String,

    /// Where pipeline state and outputs are written.
    pub output_dir: PathBuf,

    /// Source definitions, keyed by user-chosen name.
    /// e.g. { "engineering-drive": GoogleDriveSource { ... } }
    pub sources: BTreeMap<String, SourceSpec>,

    /// Stage definitions — which processing stages to apply.
    /// Ordering is declarative; the topology layer resolves execution order.
    pub stages: Vec<StageSpec>,

    /// Global settings that apply across all sources/stages.
    pub settings: PipelineSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSettings {
    /// Maximum concurrent operations (API calls, file processing, etc.)
    pub concurrency: usize,

    /// Default retry policy for transient failures.
    pub retry: RetrySpec,

    /// How often to write state checkpoints during execution.
    pub checkpoint_interval: CheckpointInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "every")]
pub enum CheckpointInterval {
    /// Checkpoint after every N items processed.
    Items { count: usize },
    /// Checkpoint after every N seconds elapsed.
    Seconds { duration: u64 },
    /// Checkpoint after every stage completes.
    Stage,
}
```

#### Source Specification

Sources are the external data services. Each source type has its own
config shape, but they share a common envelope.

```rust
/// A source is "where does the data come from."
/// The `kind` field determines which adapter handles it.
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
    // Each new source type adds a variant here and an adapter implementation.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveSourceSpec {
    /// OAuth2 credentials file path (or env var reference).
    pub credentials: CredentialRef,

    /// Root folder ID(s) to scan.
    pub root_folders: Vec<String>,

    /// Include/exclude rules, evaluated in order.
    pub filters: Vec<FilterRule>,

    /// Which file types to process.
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
pub enum FilterAction {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialRef {
    /// Path to a credentials JSON file.
    File(PathBuf),
    /// Environment variable name containing the credentials.
    EnvVar(String),
    /// Use application default credentials (GCP ADC).
    ApplicationDefault,
}
```

#### Stage Specification

```rust
/// A stage is "what transformation to apply."
/// Stages are composed into a pipeline in the order listed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    /// Unique name for this stage within the pipeline.
    pub name: String,

    /// Which stage implementation to use.
    #[serde(flatten)]
    pub kind: StageKind,

    /// Override global retry for this stage.
    pub retry: Option<RetrySpec>,

    /// If true, failure of this stage skips the item rather than
    /// failing the pipeline. Useful for best-effort extraction.
    #[serde(default)]
    pub skip_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StageKind {
    /// Fetch documents from the source service.
    #[serde(rename = "extract")]
    Extract,

    /// Convert source format to normalized markdown.
    #[serde(rename = "normalize")]
    Normalize {
        /// Target format. Currently only "markdown" is supported.
        target: String,
    },

    /// Apply include/exclude filters to extracted items.
    #[serde(rename = "filter")]
    Filter {
        rules: Vec<FilterRule>,
    },

    /// Write normalized documents to the output directory
    /// in a structure that fabryk-content can consume.
    #[serde(rename = "emit")]
    Emit {
        /// Output subdirectory within the pipeline's output_dir.
        subdir: Option<String>,
    },

    // Future stage kinds:
    // Deduplicate, Transform (LLM-assisted), Classify, etc.
}
```

#### Example TOML

```toml
name = "q1-knowledge-sync"
output_dir = "./output/q1-sync"

[settings]
concurrency = 4
checkpoint_interval = { every = "Stage" }

[settings.retry]
max_attempts = 3
initial_backoff_ms = 1000
max_backoff_ms = 30000

[sources.engineering-drive]
kind = "google_drive"
credentials = { EnvVar = "GOOGLE_CREDENTIALS" }
root_folders = ["1abc123def456"]
file_types = [{ extension = "docx" }, { extension = "pdf" }, { mime = "application/vnd.google-apps.document" }]
modified_after = "last_run"

  [[sources.engineering-drive.filters]]
  pattern = "**/Archive/**"
  action = "Exclude"

  [[sources.engineering-drive.filters]]
  pattern = "**"
  action = "Include"

[sources.team-slack]
kind = "slack"
credentials = { EnvVar = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE", "C05678FGHIJ"]
thread_depth = 3
modified_after = "2026-01-01T00:00:00Z"

[[stages]]
name = "extract"
kind = "extract"

[[stages]]
name = "normalize"
kind = "normalize"
target = "markdown"

[[stages]]
name = "emit"
kind = "emit"
subdir = "normalized"
```

---

### Layer 2: Topology (Resolved at Init)

The topology is the concrete, wired-up execution plan. It's computed from
the specification, resolves all references, and is immutable during execution.

```rust
use std::sync::Arc;

/// The resolved pipeline, ready to execute.
/// Computed from PipelineSpec at init time. Immutable during execution.
pub struct PipelineTopology {
    /// The original spec, preserved for observability.
    pub spec: Arc<PipelineSpec>,

    /// Resolved source adapters, keyed by source name.
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,

    /// Resolved stage chain.
    /// Each entry is a concrete stage implementation ready to execute.
    pub stages: Vec<ResolvedStage>,

    /// Resolved output directory (created if needed).
    pub output_dir: PathBuf,
}

pub struct ResolvedStage {
    /// Name from the spec.
    pub name: String,

    /// The concrete stage implementation.
    pub handler: Arc<dyn Stage>,

    /// Resolved retry policy (merged from stage override + global default).
    pub retry: RetryPolicy,

    /// Skip-on-error behavior.
    pub skip_on_error: bool,
}

/// Retry policy with resolved, concrete values.
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: std::time::Duration,
    pub max_backoff: std::time::Duration,
}
```

The topology is built by a resolver:

```rust
impl PipelineTopology {
    pub fn resolve(spec: PipelineSpec) -> Result<Self, ResolveError> {
        let spec = Arc::new(spec);

        // Resolve each source into a concrete adapter.
        let sources = spec.sources.iter()
            .map(|(name, source_spec)| {
                let adapter = resolve_source_adapter(source_spec)?;
                Ok((name.clone(), adapter))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        // Resolve each stage into a concrete handler.
        let stages = spec.stages.iter()
            .map(|stage_spec| resolve_stage(stage_spec, &spec.settings))
            .collect::<Result<Vec<_>, _>>()?;

        // Create output directory.
        let output_dir = spec.output_dir.clone();
        std::fs::create_dir_all(&output_dir)?;

        Ok(Self { spec, sources, stages, output_dir })
    }
}
```

---

### Layer 3: State (Mutates During Execution)

This is the core of the "state IS the pipeline" insight. The state struct
is the complete truth of where execution stands. It must be serializable,
inspectable, and resumable.

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Complete pipeline execution state.
/// Serialize this at any point → perfect checkpoint.
/// Hand it to Claude → full system understanding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    /// Pipeline identity.
    pub pipeline_name: String,

    /// When this run started.
    pub started_at: DateTime<Utc>,

    /// When the last checkpoint was written.
    pub last_checkpoint: DateTime<Utc>,

    /// Overall status.
    pub status: PipelineStatus,

    /// Per-source extraction state.
    pub sources: BTreeMap<String, SourceState>,

    /// Per-stage execution state.
    pub stages: Vec<StageState>,

    /// Content hashes from the previous run, used for incrementality.
    /// Key: item_id, Value: blake3 hash of content at last successful processing.
    pub content_hashes: BTreeMap<String, String>,

    /// Summary statistics.
    pub stats: PipelineStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    /// Not yet started.
    Pending,
    /// Currently executing.
    Running { current_stage: String },
    /// Completed successfully.
    Completed { finished_at: DateTime<Utc> },
    /// Failed with error.
    Failed { error: String, failed_at: DateTime<Utc> },
    /// Was interrupted and can be resumed.
    Interrupted { resumed_from: Option<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceState {
    /// How many items were discovered by enumeration.
    pub items_discovered: usize,

    /// How many items passed filters.
    pub items_accepted: usize,

    /// How many items were skipped due to unchanged content hash.
    pub items_skipped_unchanged: usize,

    /// Per-item status for items that entered the pipeline.
    pub items: BTreeMap<String, ItemState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    /// Human-readable identifier (filename, message ID, etc.)
    pub display_name: String,

    /// Source-specific unique identifier.
    pub source_id: String,

    /// Which source this item came from.
    pub source_name: String,

    /// Content hash (blake3) of the source content.
    pub content_hash: String,

    /// Current processing status.
    pub status: ItemStatus,

    /// Which stages have been completed for this item.
    pub completed_stages: Vec<CompletedStage>,

    /// Provenance: where did this item come from?
    pub provenance: ItemProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Discovered but not yet processed.
    Pending,
    /// Currently being processed by a stage.
    Processing { stage: String },
    /// All stages completed successfully.
    Completed,
    /// Failed at a stage; may be retryable.
    Failed { stage: String, error: String, attempts: u32 },
    /// Skipped (e.g., skip_on_error was true and a stage failed).
    Skipped { stage: String, reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedStage {
    pub stage_name: String,
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
    pub metadata: BTreeMap<String, String>,

    /// When the source item was last modified (according to the source).
    pub source_modified: Option<DateTime<Utc>>,

    /// When we extracted it.
    pub extracted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageState {
    pub name: String,
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
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
    pub total_duration_ms: u64,
}
```

---

## 2. Core Traits

### The SourceAdapter Trait

This is the boundary between the messy external world and our clean pipeline.
Everything about OAuth, pagination, rate limiting, format detection lives
behind this trait.

```rust
use async_trait::async_trait;

/// A document extracted from a source, in its original format.
/// This is the raw material before any normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Unique identifier within this source.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// The raw content bytes.
    pub content: Vec<u8>,

    /// MIME type of the content, as reported by the source.
    pub mime_type: String,

    /// Provenance metadata.
    pub provenance: ItemProvenance,

    /// Content hash (blake3 of content bytes).
    pub content_hash: String,
}

/// A source adapter handles all interaction with a single external service.
///
/// Implementors handle: authentication, enumeration, filtering, pagination,
/// rate limiting, and fetching. The pipeline runner sees only the trait.
#[async_trait]
pub trait SourceAdapter: Send + Sync {
    /// Human-readable name of the source type (e.g., "Google Drive").
    fn source_kind(&self) -> &str;

    /// Enumerate all items available from this source, respecting the
    /// configured filters. Returns item IDs and basic metadata without
    /// fetching content (cheap operation).
    ///
    /// This is the "what's there?" step.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    /// Separate from enumerate() because fetching is expensive and
    /// we want to skip unchanged items before paying this cost.
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
    /// (Google Drive provides md5Checksum, for example.)
    /// If None, the pipeline will fetch and compute blake3.
    pub source_hash: Option<String>,
}
```

### The Stage Trait

Stages transform items as they flow through the pipeline.

```rust
/// The intermediate representation flowing between stages.
/// Starts as ExtractedDocument, accumulates transformations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    /// The item's unique identifier.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// Current content (may be transformed by prior stages).
    /// Stored as bytes to support both binary and text content.
    pub content: Vec<u8>,

    /// Current MIME type (changes as content is transformed).
    pub mime_type: String,

    /// Content hash of the original source content (for incrementality).
    pub source_content_hash: String,

    /// Provenance chain.
    pub provenance: ItemProvenance,

    /// Metadata accumulated by stages.
    /// Each stage can add key-value pairs here.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
///
/// Stages are intentionally simple: one item in, one item out.
/// Fan-out (one item becoming many) is handled by returning a Vec.
/// Fan-in (aggregation across items) is a future concern.
#[async_trait]
pub trait Stage: Send + Sync {
    /// Human-readable name of this stage type.
    fn name(&self) -> &str;

    /// Process a single item. May return:
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

/// Context provided to stages during execution.
/// Read-only access to pipeline-wide resources.
pub struct StageContext {
    /// The original pipeline specification.
    pub spec: Arc<PipelineSpec>,

    /// The output directory for this pipeline run.
    pub output_dir: PathBuf,

    /// Structured logger for this stage.
    pub log: tracing::Span,
}
```

### The Checkpoint Trait

Persistence of pipeline state for durability and incrementality.

```rust
/// Persistence backend for pipeline state.
/// Default implementation uses redb; trait exists for testability.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Save the full pipeline state as a checkpoint.
    async fn save_checkpoint(&self, state: &PipelineState) -> Result<(), StateError>;

    /// Load the most recent checkpoint, if one exists.
    async fn load_checkpoint(&self) -> Result<Option<PipelineState>, StateError>;

    /// Load content hashes from the previous completed run.
    /// Used for incrementality: skip items whose hash hasn't changed.
    async fn load_previous_hashes(&self) -> Result<BTreeMap<String, String>, StateError>;

    /// Save content hashes at the end of a successful run.
    async fn save_hashes(&self, hashes: &BTreeMap<String, String>) -> Result<(), StateError>;
}
```

---

## 3. The Pipeline Runner

The runner is the orchestrator. It owns the topology and state, executes
stages in order, checkpoints periodically, and supports resume.

```rust
pub struct PipelineRunner {
    topology: PipelineTopology,
    state: PipelineState,
    store: Box<dyn StateStore>,
}

impl PipelineRunner {
    /// Create a new runner from a specification.
    /// If a checkpoint exists, offers to resume.
    pub async fn new(spec: PipelineSpec) -> Result<Self, PipelineError> {
        let topology = PipelineTopology::resolve(spec)?;
        let store = RedbStateStore::open(&topology.output_dir)?;

        // Check for existing checkpoint.
        let state = if let Some(checkpoint) = store.load_checkpoint().await? {
            // Resume from checkpoint.
            tracing::info!(
                "Resuming from checkpoint ({})",
                checkpoint.last_checkpoint,
            );
            checkpoint
        } else {
            // Fresh run.
            PipelineState::new(&topology)
        };

        Ok(Self { topology, state, store: Box::new(store) })
    }

    /// Execute the pipeline.
    pub async fn run(&mut self) -> Result<PipelineState, PipelineError> {
        self.state.status = PipelineStatus::Running {
            current_stage: "extract".into(),
        };

        // Phase 1: Enumerate items from all sources.
        self.enumerate_sources().await?;

        // Phase 2: Apply incrementality — skip unchanged items.
        self.apply_incrementality().await?;

        // Phase 3: Execute stages in order.
        for stage_idx in 0..self.topology.stages.len() {
            self.execute_stage(stage_idx).await?;
            self.checkpoint().await?;
        }

        // Phase 4: Save content hashes for next run.
        self.save_hashes().await?;

        self.state.status = PipelineStatus::Completed {
            finished_at: Utc::now(),
        };
        self.checkpoint().await?;

        Ok(self.state.clone())
    }

    /// Enumerate all sources and populate the item list.
    async fn enumerate_sources(&mut self) -> Result<(), PipelineError> {
        for (name, adapter) in &self.topology.sources {
            let items = adapter.enumerate().await?;
            let source_state = self.state.sources
                .entry(name.clone())
                .or_insert_with(|| SourceState::new());

            source_state.items_discovered = items.len();

            for item in items {
                source_state.items.insert(item.id.clone(), ItemState {
                    display_name: item.display_name.clone(),
                    source_id: item.id.clone(),
                    source_name: name.clone(),
                    content_hash: String::new(), // computed at fetch time
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

    /// Compare content hashes against previous run; mark unchanged items
    /// as skipped.
    async fn apply_incrementality(&mut self) -> Result<(), PipelineError> {
        let previous_hashes = self.store.load_previous_hashes().await?;

        for source_state in self.state.sources.values_mut() {
            for (item_id, item_state) in source_state.items.iter_mut() {
                if let Some(prev_hash) = previous_hashes.get(item_id) {
                    // If the source provides a cheap hash, compare it.
                    // Otherwise, we'll have to fetch to compare.
                    // This is a simplification — real impl would be smarter.
                    if item_state.content_hash == *prev_hash {
                        item_state.status = ItemStatus::Skipped {
                            stage: "incrementality".into(),
                            reason: "Content unchanged since last run".into(),
                        };
                        source_state.items_skipped_unchanged += 1;
                    }
                }
            }
        }
        Ok(())
    }

    /// Execute a single stage across all pending items.
    async fn execute_stage(&mut self, stage_idx: usize) -> Result<(), PipelineError> {
        let stage = &self.topology.stages[stage_idx];
        let stage_name = stage.name.clone();

        self.state.stages[stage_idx].status = StageStatus::Running;
        self.state.stages[stage_idx].started_at = Some(Utc::now());
        self.state.status = PipelineStatus::Running {
            current_stage: stage_name.clone(),
        };

        // Collect pending items for this stage.
        // (In a real impl, this would be more sophisticated —
        // handling concurrency, respecting checkpoint interval, etc.)
        let pending_items = self.collect_pending_items(&stage_name);

        for item_id in pending_items {
            let result = self.process_item(&stage_name, &item_id, stage_idx).await;

            match result {
                Ok(()) => {
                    self.state.stages[stage_idx].items_processed += 1;
                }
                Err(e) if stage.skip_on_error => {
                    self.mark_item_skipped(&item_id, &stage_name, &e.to_string());
                    self.state.stages[stage_idx].items_skipped += 1;
                }
                Err(e) => {
                    self.mark_item_failed(&item_id, &stage_name, &e.to_string());
                    self.state.stages[stage_idx].items_failed += 1;
                }
            }

            // Periodic checkpoint within a stage.
            self.maybe_checkpoint().await?;
        }

        self.state.stages[stage_idx].status = StageStatus::Completed;
        self.state.stages[stage_idx].completed_at = Some(Utc::now());
        Ok(())
    }

    /// Write a checkpoint to the state store.
    async fn checkpoint(&mut self) -> Result<(), PipelineError> {
        self.state.last_checkpoint = Utc::now();
        self.store.save_checkpoint(&self.state).await?;
        Ok(())
    }
}
```

---

## 4. Proposed Crate Structure

```
ecl/crates/
├── ecl-core/          # Existing: WorkflowId, StepResult, LlmProvider, etc.
│                      # Extended with: PipelineSpec, PipelineState, core traits
│
├── ecl-pipeline/      # NEW: The pipeline runner
│   ├── src/
│   │   ├── runner.rs      # PipelineRunner orchestration
│   │   ├── topology.rs    # PipelineTopology resolution
│   │   ├── state.rs       # PipelineState, ItemState, etc.
│   │   ├── checkpoint.rs  # StateStore trait + RedbStateStore
│   │   └── lib.rs
│   └── Cargo.toml         # deps: ecl-core, redb, blake3, tokio, serde
│
├── ecl-stages/        # NEW: Built-in stage implementations
│   ├── src/
│   │   ├── extract.rs     # ExtractStage (delegates to SourceAdapter)
│   │   ├── normalize.rs   # NormalizeStage (format conversion)
│   │   ├── filter.rs      # FilterStage (glob-based include/exclude)
│   │   ├── emit.rs        # EmitStage (write to output dir)
│   │   └── lib.rs
│   └── Cargo.toml         # deps: ecl-core, ecl-pipeline
│
├── ecl-adapters/      # NEW: Source adapter implementations
│   ├── src/
│   │   ├── google_drive.rs  # GoogleDriveAdapter
│   │   ├── slack.rs         # SlackAdapter
│   │   ├── filesystem.rs    # FilesystemAdapter (local files, trivial)
│   │   └── lib.rs
│   └── Cargo.toml           # deps: ecl-core, google-drive3, yup-oauth2,
│                            #        slack-morphism, reqwest-middleware
│
├── ecl-cli/           # Existing, extended with pipeline commands
│   ├── src/
│   │   ├── main.rs
│   │   ├── pipeline.rs    # `ecl pipeline run`, `ecl pipeline status`,
│   │   │                  # `ecl pipeline resume`, `ecl pipeline inspect`
│   │   └── ...
│   └── Cargo.toml
│
└── ecl/               # Existing umbrella, adds pipeline re-exports
```

### CLI Commands

```
ecl pipeline run <config.toml>       # Execute a pipeline from config
ecl pipeline resume <output-dir>     # Resume from last checkpoint
ecl pipeline status <output-dir>     # Show current state (human-readable)
ecl pipeline inspect <output-dir>    # Dump full state as JSON (for Claude)
ecl pipeline items <output-dir>      # List all items with their status
```

---

## 5. Incrementality Flow

```
                    ┌──────────────┐
                    │  Load prev   │
                    │   hashes     │
                    │  from redb   │
                    └──────┬───────┘
                           │
                           ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│  Enumerate   │───▶│  For each    │───▶│ Hash match?  │
│  source      │    │  item        │    │              │
└──────────────┘    └──────────────┘    └──────┬───────┘
                                               │
                                    ┌──────────┴──────────┐
                                    │                     │
                                    ▼                     ▼
                            ┌──────────────┐    ┌──────────────┐
                            │   SKIP       │    │   PROCESS    │
                            │  (unchanged) │    │  (new/modified)│
                            └──────────────┘    └──────────────┘
                                                       │
                                                       ▼
                                               ┌──────────────┐
                                               │ Save new hash│
                                               │  to state    │
                                               └──────────────┘
```

---

## 6. Observability: What Claude Sees

When someone runs `ecl pipeline inspect ./output/q1-sync`, the JSON output
would look something like:

```json
{
  "pipeline_name": "q1-knowledge-sync",
  "started_at": "2026-03-13T14:30:00Z",
  "last_checkpoint": "2026-03-13T14:47:22Z",
  "status": { "Running": { "current_stage": "normalize" } },
  "sources": {
    "engineering-drive": {
      "items_discovered": 200,
      "items_accepted": 187,
      "items_skipped_unchanged": 142,
      "items": {
        "1abc...": {
          "display_name": "Q1 Architecture Review.docx",
          "status": "Completed",
          "completed_stages": [
            { "stage_name": "extract", "completed_at": "...", "duration_ms": 1200 },
            { "stage_name": "normalize", "completed_at": "...", "duration_ms": 340 }
          ],
          "provenance": {
            "source_kind": "google_drive",
            "metadata": {
              "file_id": "1abc...",
              "path": "/Engineering/Architecture/Q1 Architecture Review.docx",
              "owner": "alice@company.com"
            }
          }
        },
        "2def...": {
          "display_name": "Meeting Notes (old).pdf",
          "status": { "Failed": {
            "stage": "normalize",
            "error": "PDF conversion failed: encrypted document",
            "attempts": 3
          }}
        }
      }
    }
  },
  "stages": [
    { "name": "extract", "status": "Completed", "items_processed": 45, "items_failed": 0 },
    { "name": "normalize", "status": "Running", "items_processed": 38, "items_failed": 2 }
  ],
  "stats": {
    "total_items_discovered": 200,
    "total_items_processed": 83,
    "total_items_skipped_unchanged": 142,
    "total_items_failed": 2
  }
}
```

From this, Claude can immediately say: "You're 45 documents into the normalize
stage. 142 of 200 items were skipped because they haven't changed since your
last run. Two items failed — one is an encrypted PDF, the other is [whatever].
38 items have been fully normalized and are ready for Fabryk ingestion. At the
current rate, you'll finish in about 3 minutes."

No logs. No dashboards. Just state.

---

## 7. Open Questions for Research Comparison

These are the design decisions I'm least certain about, and where the
six-crate analysis might change our direction:

1. **Stage trait: `process(item) -> Vec<item>` vs richer protocol.**
   Is one-item-in, vec-out sufficient? dagrs uses channels; dagx uses
   typed TaskHandles. Should we support inter-stage streaming?

2. **Concurrency model within a stage.** The current design processes
   items sequentially within a stage. Should we support parallel item
   processing within a stage (bounded by `settings.concurrency`)?
   dagx and dagrs both support parallel execution.

3. **Type safety of inter-stage data.** Currently `PipelineItem` is a
   universal envelope (bytes + mime type + metadata map). dagx achieves
   compile-time type safety between tasks. Is there a middle ground
   that preserves serializability for checkpointing?

4. **Error recovery granularity.** Currently we checkpoint per-stage.
   Should we checkpoint per-item? Per-batch? The research may reveal
   patterns we haven't considered.

5. **DAG vs linear pipeline.** The current design is a linear chain
   of stages. Should we support DAG-shaped pipelines from the start?
   Or is linear sufficient for the PoC and DAG is a later extension?

6. **Where does this live relative to existing `ecl-*` crates?** The
   current `ecl-core` has `WorkflowId`, `StepResult`, `CritiqueDecision`,
   `LlmProvider`. Do the pipeline types go in `ecl-core` (extending it)
   or in new crates? Does the existing `ecl-workflows` concept
   (CritiqueLoopWorkflow) compose with or sit alongside the pipeline?

---

## 8. What This Proposal Does NOT Cover

- **AI-assisted stages** (concept extraction, classification, synthesis) —
  these will be future `StageKind` variants that use `LlmProvider`
- **Fabryk integration** — the `emit` stage writes files that `fabryk-content`
  can consume, but there's no direct API integration yet
- **ACL / multi-tenancy** — deferred to Fabryk's `fabryk-acl` timeline
- **The orchestrator / hub-and-spoke MCP design** — orthogonal concern
- **Web UI or monitoring dashboard** — the `inspect` command + Claude is
  the observability story for now
