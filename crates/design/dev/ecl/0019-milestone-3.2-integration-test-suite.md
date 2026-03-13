# Milestone 3.2: Integration Test Suite

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `test(ecl-pipeline):`

## 1. Goal

Create a comprehensive integration test suite for the `ecl-pipeline` crate.
These tests exercise the full pipeline lifecycle end-to-end using real
`FilesystemAdapter` and built-in stages (`extract`, `normalize`, `emit`) from
the `ecl-adapter-fs` and `ecl-stages` crates. When done:

1. A happy-path test drives 10 text files through a 3-stage pipeline
   (extract, normalize, emit) and verifies all items complete with output files
   on disk.
2. A checkpoint/resume test simulates a mid-pipeline crash using a
   `FailingStage`, resumes, and verifies that previously completed batches are
   not re-executed.
3. An incrementality test runs the pipeline twice, modifies one file between
   runs, and verifies that only the modified file is reprocessed in the second
   run.
4. An error-handling test exercises `skip_on_error=true`, verifying that one
   failing item does not prevent the rest from completing.
5. A concurrent-stages test verifies that two independent extract stages (with
   independent resources) execute in parallel within the same batch.
6. A fan-in test verifies that an emit stage reading multiple resources
   receives the combined item set from both upstream sources.
7. A state-inspection test serializes `PipelineState` to JSON after completion
   and verifies the structure matches the observability shape defined in design
   doc section 6.

This milestone creates **no new crates**. All files live in
`crates/ecl-pipeline/tests/`.

## 2. Context

### 2.1 Project Conventions

- **Edition:** 2024, **Rust version:** 1.85
- **Lints** (in every crate's `Cargo.toml`):
  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  missing_docs = "warn"

  [lints.clippy]
  unwrap_used = "deny"
  expect_used = "warn"
  panic = "deny"
  ```
- **Test naming:** `test_<fn>_<scenario>_<expectation>`
- **Integration tests:** Located in `crates/ecl-pipeline/tests/`, each file
  is a separate test binary. Shared helpers live in `tests/common/mod.rs`.
- **Maps:** Always `BTreeMap` (not `HashMap`) for deterministic serialization
- **No `unwrap()` in library code** — but `#[allow(clippy::unwrap_used)]` is
  permitted at the top of test modules for ergonomics.

### 2.2 Rust Guides to Load

Before writing any code, read these files (paths relative to workspace root):

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` (ALWAYS first)
2. `assets/ai/ai-rust/guides/01-core-idioms.md`
3. `assets/ai/ai-rust/guides/03-error-handling.md`
4. `assets/ai/ai-rust/guides/07-concurrency-async.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)

## 3. Prior Art / Dependencies

This milestone depends on four prior milestone crates plus two milestone 3.1
crates. Their complete public APIs relevant to integration testing are listed
here so you do not need to read any other files.

### From `ecl-pipeline-spec` (Milestone 1.1)

```rust
// --- crate: ecl-pipeline-spec ---

/// The root configuration, deserialized from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    pub name: String,
    pub version: u32,
    pub output_dir: PathBuf,
    pub sources: BTreeMap<String, SourceSpec>,
    pub stages: BTreeMap<String, StageSpec>,
    #[serde(default)]
    pub defaults: DefaultsSpec,
}

impl PipelineSpec {
    pub fn from_toml(toml_str: &str) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
}

/// Global defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    #[serde(default = "default_concurrency")]  // default: 4
    pub concurrency: usize,
    #[serde(default)]
    pub retry: RetrySpec,
    #[serde(default)]
    pub checkpoint: CheckpointStrategy,
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u32,       // default: 3
    pub initial_backoff_ms: u64, // default: 1000
    pub backoff_multiplier: f64, // default: 2.0
    pub max_backoff_ms: u64,     // default: 30_000
}

/// A source specification (internally-tagged by `kind`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    #[serde(rename = "google_drive")] GoogleDrive(GoogleDriveSourceSpec),
    #[serde(rename = "slack")]        Slack(SlackSourceSpec),
    #[serde(rename = "filesystem")]   Filesystem(FilesystemSourceSpec),
}

/// Local filesystem source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSourceSpec {
    pub root: PathBuf,
    #[serde(default)]
    pub filters: Vec<FilterRule>,
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// A stage specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageSpec {
    pub adapter: String,
    pub source: Option<String>,
    #[serde(default)]
    pub resources: ResourceSpec,
    #[serde(default)]
    pub params: serde_json::Value,
    pub retry: Option<RetrySpec>,
    pub timeout_secs: Option<u64>,
    #[serde(default)]
    pub skip_on_error: bool,
    pub condition: Option<String>,
}

/// Resource access declarations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    #[serde(default)] pub reads: Vec<String>,
    #[serde(default)] pub creates: Vec<String>,
    #[serde(default)] pub writes: Vec<String>,
}

pub enum CheckpointStrategy { Batch, Items { count: usize }, Seconds { duration: u64 } }
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- crate: ecl-pipeline-state ---

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);
impl StageId {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);
impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn is_empty(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(String);

/// Complete pipeline execution state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineState {
    pub run_id: RunId,
    pub pipeline_name: String,
    pub started_at: DateTime<Utc>,
    pub last_checkpoint: DateTime<Utc>,
    pub status: PipelineStatus,
    pub current_batch: usize,
    pub sources: BTreeMap<String, SourceState>,
    pub stages: BTreeMap<StageId, StageState>,
    pub stats: PipelineStats,
}

impl PipelineState {
    pub fn update_stats(&mut self);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineStatus {
    Pending,
    Running { current_stage: String },
    Completed { finished_at: DateTime<Utc> },
    Failed { error: String, failed_at: DateTime<Utc> },
    Interrupted { interrupted_at: DateTime<Utc> },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceState {
    pub items_discovered: usize,
    pub items_accepted: usize,
    pub items_skipped_unchanged: usize,
    pub items: BTreeMap<String, ItemState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemState {
    pub display_name: String,
    pub source_id: String,
    pub source_name: String,
    pub content_hash: Blake3Hash,
    pub status: ItemStatus,
    pub completed_stages: Vec<CompletedStageRecord>,
    pub provenance: ItemProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemStatus {
    Pending,
    Processing { stage: String },
    Completed,
    Failed { stage: String, error: String, attempts: u32 },
    Skipped { stage: String, reason: String },
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
    pub source_kind: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub source_modified: Option<DateTime<Utc>>,
    pub extracted_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineStats {
    pub total_items_discovered: usize,
    pub total_items_processed: usize,
    pub total_items_skipped_unchanged: usize,
    pub total_items_failed: usize,
}

/// Self-contained recovery artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub version: u32,
    pub sequence: u64,
    pub created_at: DateTime<Utc>,
    pub spec: PipelineSpec,
    pub schedule: Vec<Vec<StageId>>,
    pub spec_hash: Blake3Hash,
    pub state: PipelineState,
}

impl Checkpoint {
    pub fn prepare_for_resume(&mut self);
    pub fn config_drifted(&self, current_spec_hash: &Blake3Hash) -> bool;
}

/// Persistence backend for pipeline state.
#[async_trait]
pub trait StateStore: Send + Sync {
    async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<(), StateError>;
    async fn load_checkpoint(&self) -> Result<Option<Checkpoint>, StateError>;
    async fn load_previous_hashes(&self) -> Result<BTreeMap<String, Blake3Hash>, StateError>;
    async fn save_completed_hashes(
        &self, run_id: &RunId, hashes: &BTreeMap<String, Blake3Hash>,
    ) -> Result<(), StateError>;
}

/// In-memory state store for testing.
#[derive(Debug, Default)]
pub struct InMemoryStateStore { /* ... */ }
impl InMemoryStateStore {
    pub fn new() -> Self;
}
// Implements StateStore.
```

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
// --- crate: ecl-pipeline-topo ---

#[derive(Debug, Clone)]
pub struct PipelineTopology {
    pub spec: Arc<PipelineSpec>,
    pub spec_hash: Blake3Hash,
    pub sources: BTreeMap<String, Arc<dyn SourceAdapter>>,
    pub stages: BTreeMap<String, ResolvedStage>,
    pub schedule: Vec<Vec<StageId>>,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ResolvedStage {
    pub id: StageId,
    pub handler: Arc<dyn Stage>,
    pub retry: RetryPolicy,
    pub skip_on_error: bool,
    pub timeout: Option<Duration>,
    pub source: Option<String>,
    pub condition: Option<ConditionExpr>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub backoff_multiplier: f64,
    pub max_backoff: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_millis(30_000),
        }
    }
}

#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    fn source_kind(&self) -> &str;
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub id: String,
    pub display_name: String,
    pub mime_type: String,
    pub path: String,
    pub modified_at: Option<DateTime<Utc>>,
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
    pub mime_type: String,
    pub provenance: ItemProvenance,
    pub content_hash: Blake3Hash,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,
    pub mime_type: String,
    pub source_name: String,
    pub source_content_hash: Blake3Hash,
    pub provenance: ItemProvenance,
    pub metadata: BTreeMap<String, serde_json::Value>,
}

#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

#[derive(Debug, Clone)]
pub struct StageContext {
    pub spec: Arc<PipelineSpec>,
    pub output_dir: PathBuf,
    pub params: serde_json::Value,
    pub span: tracing::Span,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StageError {
    UnsupportedContent { stage: String, item_id: String, message: String },
    Transient { stage: String, item_id: String, message: String },
    Permanent { stage: String, item_id: String, message: String },
    Timeout { stage: String, item_id: String, timeout_secs: u64 },
}
```

### From `ecl-pipeline` (Milestone 2.2)

```rust
// --- crate: ecl-pipeline ---

/// The pipeline runner: orchestrates enumeration, incrementality,
/// batch execution, checkpointing, and resume.
#[derive(Debug)]
pub struct PipelineRunner {
    topology: PipelineTopology,
    state: PipelineState,
    store: Box<dyn StateStore>,
    checkpoint_sequence: u64,
}

impl PipelineRunner {
    pub async fn new(
        topology: PipelineTopology,
        store: Box<dyn StateStore>,
    ) -> Result<Self>;

    pub async fn run(&mut self) -> Result<&PipelineState>;

    pub fn state(&self) -> &PipelineState;
    pub fn topology(&self) -> &PipelineTopology;
}

#[derive(Debug)]
pub struct StageResult {
    pub stage_id: StageId,
    pub successes: Vec<StageItemSuccess>,
    pub skipped: Vec<StageItemSkipped>,
    pub failures: Vec<StageItemFailure>,
}

pub async fn execute_stage_items(
    stage: ResolvedStage,
    items: Vec<PipelineItem>,
    ctx: StageContext,
    concurrency: usize,
) -> Result<StageResult, PipelineError>;

pub async fn execute_with_retry(
    handler: &Arc<dyn Stage>,
    item: PipelineItem,
    ctx: &StageContext,
    retry: &RetryPolicy,
) -> Result<Vec<PipelineItem>, StageError>;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PipelineError {
    SourceEnumeration { source: String, error: String },
    StageExecution(#[from] StageError),
    StateStore(#[from] StateError),
    Resolve(#[from] ResolveError),
    Source(#[from] SourceError),
    ConfigDrift { checkpoint_hash: String, current_hash: String },
    JoinError(#[from] tokio::task::JoinError),
    SemaphoreError(#[from] tokio::sync::AcquireError),
    ItemFailed { stage: String, item_id: String, error: String },
}
```

### From `ecl-adapter-fs` (Milestone 3.1)

```rust
// --- crate: ecl-adapter-fs ---

/// Filesystem source adapter. Enumerates files in a directory tree
/// and provides their content via fetch().
#[derive(Debug)]
pub struct FilesystemAdapter {
    root: PathBuf,
    extensions: Vec<String>,
    filters: Vec<FilterRule>,
}

impl FilesystemAdapter {
    pub fn new(spec: &FilesystemSourceSpec) -> Self;
}

// Implements SourceAdapter:
//   source_kind() -> "filesystem"
//   enumerate() -> walks root dir, applies extension/filter rules, returns SourceItems
//   fetch() -> reads file content, computes blake3 hash, returns ExtractedDocument
```

### From `ecl-stages` (Milestone 3.1)

```rust
// --- crate: ecl-stages ---

/// Extract stage: delegates to SourceAdapter::fetch() for each item.
/// Reads source content and produces PipelineItems with content populated.
#[derive(Debug)]
pub struct ExtractStage { /* source adapter ref */ }

/// Normalize stage: converts content to a normalized text/markdown form.
/// For plain text files, this is essentially a pass-through with metadata.
#[derive(Debug)]
pub struct NormalizeStage { /* ... */ }

/// Emit stage: writes each item's content to the output directory.
/// Creates files in output_dir/params.subdir/.
#[derive(Debug)]
pub struct EmitStage { /* ... */ }

// Each implements the Stage trait.
// Registration helpers may exist for building topologies with these stages.
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/ecl-pipeline/Cargo.toml` | Modify | Add `ecl-adapter-fs`, `ecl-stages`, `tempfile` to `[dev-dependencies]` |
| `crates/ecl-pipeline/tests/common/mod.rs` | Create | Shared test helpers: create temp files, write TOML, build runner |
| `crates/ecl-pipeline/tests/full_pipeline.rs` | Create | Happy-path end-to-end, concurrent stages, state inspection |
| `crates/ecl-pipeline/tests/checkpoint_resume.rs` | Create | Crash-and-resume scenario |
| `crates/ecl-pipeline/tests/incrementality.rs` | Create | Cross-run hash comparison |
| `crates/ecl-pipeline/tests/error_handling.rs` | Create | Retry, skip_on_error, stage failure |
| `crates/ecl-pipeline/tests/fan_in.rs` | Create | Stage reading multiple resources gets combined items |

## 5. Cargo.toml

### Modifications to `crates/ecl-pipeline/Cargo.toml`

Add these entries to the existing `[dev-dependencies]` section:

```toml
[dev-dependencies]
ecl-adapter-fs = { path = "../ecl-adapter-fs" }
ecl-stages = { path = "../ecl-stages" }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }
serde_json = { workspace = true }
```

No changes to the root workspace `Cargo.toml`. No new crates are created.

## 6. Type Definitions and Signatures

This milestone creates no new library types. All code lives in integration
test files. The following are the test helper function signatures in
`tests/common/mod.rs`.

### `tests/common/mod.rs`

```rust
//! Shared integration test helpers for ecl-pipeline.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::collections::BTreeMap;

use tempfile::TempDir;

use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::{
    DefaultsSpec, FilesystemSourceSpec, PipelineSpec, ResourceSpec,
    SourceSpec, StageSpec,
};
use ecl_pipeline_state::{Blake3Hash, InMemoryStateStore, StageId, StateStore};
use ecl_pipeline_topo::{
    PipelineItem, PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter,
    Stage, StageContext, StageError,
};

/// Create `count` numbered text files in `dir`.
///
/// Files are named `file_000.txt`, `file_001.txt`, etc.
/// Each file contains "Content of file_NNN" as its body.
/// Returns the list of created file paths.
pub fn create_test_files(dir: &Path, count: usize) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(count);
    for i in 0..count {
        let name = format!("file_{:03}.txt", i);
        let path = dir.join(&name);
        std::fs::write(&path, format!("Content of {}", name)).unwrap();
        paths.push(path);
    }
    paths
}

/// Write a minimal pipeline TOML that uses the given `source_dir` as a
/// filesystem source with `stage_count` stages (extract -> normalize -> emit).
///
/// Returns the path to the written TOML file.
pub fn write_test_toml(dir: &Path, source_dir: &Path) -> PathBuf {
    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let toml_content = format!(
        r#"name = "integration-test"
version = 1
output_dir = "{output}"

[defaults]
concurrency = 4
checkpoint = {{ every = "Batch" }}

[defaults.retry]
max_attempts = 1
initial_backoff_ms = 10
backoff_multiplier = 1.0
max_backoff_ms = 10

[sources.local]
kind = "filesystem"
root = "{root}"
extensions = ["txt"]

[stages.extract]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}

[stages.normalize]
adapter = "normalize"
resources = {{ reads = ["raw-docs"], creates = ["normalized-docs"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["normalized-docs"] }}

[stages.emit.params]
subdir = "normalized"
"#,
        output = output_dir.display(),
        root = source_dir.display(),
    );

    let toml_path = dir.join("pipeline.toml");
    std::fs::write(&toml_path, toml_content).unwrap();
    toml_path
}

/// Write a TOML with two independent filesystem sources and two extract
/// stages that can run concurrently (same batch due to independent resources).
pub fn write_concurrent_toml(
    dir: &Path,
    source_dir_a: &Path,
    source_dir_b: &Path,
) -> PathBuf {
    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let toml_content = format!(
        r#"name = "concurrent-test"
version = 1
output_dir = "{output}"

[defaults]
concurrency = 4

[defaults.retry]
max_attempts = 1
initial_backoff_ms = 10
backoff_multiplier = 1.0
max_backoff_ms = 10

[sources.source-a]
kind = "filesystem"
root = "{root_a}"
extensions = ["txt"]

[sources.source-b]
kind = "filesystem"
root = "{root_b}"
extensions = ["txt"]

[stages.extract-a]
adapter = "extract"
source = "source-a"
resources = {{ creates = ["raw-a"] }}

[stages.extract-b]
adapter = "extract"
source = "source-b"
resources = {{ creates = ["raw-b"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["raw-a", "raw-b"] }}

[stages.emit.params]
subdir = "combined"
"#,
        output = output_dir.display(),
        root_a = source_dir_a.display(),
        root_b = source_dir_b.display(),
    );

    let toml_path = dir.join("pipeline.toml");
    std::fs::write(&toml_path, toml_content).unwrap();
    toml_path
}

/// Write a TOML where one stage has skip_on_error = true.
pub fn write_skip_on_error_toml(dir: &Path, source_dir: &Path) -> PathBuf {
    let output_dir = dir.join("output");
    std::fs::create_dir_all(&output_dir).unwrap();

    let toml_content = format!(
        r#"name = "error-test"
version = 1
output_dir = "{output}"

[defaults]
concurrency = 4

[defaults.retry]
max_attempts = 1
initial_backoff_ms = 10
backoff_multiplier = 1.0
max_backoff_ms = 10

[sources.local]
kind = "filesystem"
root = "{root}"
extensions = ["txt"]

[stages.extract]
adapter = "extract"
source = "local"
resources = {{ creates = ["raw-docs"] }}

[stages.normalize]
adapter = "normalize"
skip_on_error = true
resources = {{ reads = ["raw-docs"], creates = ["normalized-docs"] }}

[stages.emit]
adapter = "emit"
resources = {{ reads = ["normalized-docs"] }}

[stages.emit.params]
subdir = "normalized"
"#,
        output = output_dir.display(),
        root = source_dir.display(),
    );

    let toml_path = dir.join("pipeline.toml");
    std::fs::write(&toml_path, toml_content).unwrap();
    toml_path
}

/// Parse TOML and build a `PipelineRunner` from it using real adapters
/// and stages (FilesystemAdapter, ExtractStage, NormalizeStage, EmitStage).
///
/// Uses `InMemoryStateStore` for state persistence.
///
/// Returns `(PipelineRunner, Box<InMemoryStateStore>)` — the store is
/// returned separately so tests can inspect checkpoints after the run.
pub fn build_test_runner(toml_path: &Path) -> (PipelineRunner, Arc<InMemoryStateStore>) {
    // Implementation: read TOML file, parse PipelineSpec, create
    // FilesystemAdapter for each filesystem source, register built-in
    // stages, resolve topology, create runner with InMemoryStateStore.
    //
    // The exact construction depends on the milestone 3.1 and 2.3
    // APIs. The general pattern:
    //
    //   let toml_str = std::fs::read_to_string(toml_path).unwrap();
    //   let spec = PipelineSpec::from_toml(&toml_str).unwrap();
    //   let store = Arc::new(InMemoryStateStore::new());
    //   // Build topology from spec using ecl-adapter-fs and ecl-stages
    //   // (register adapters and stages, resolve)
    //   let topology = /* resolve topology */;
    //   let runner = tokio::runtime::Runtime::new().unwrap()
    //       .block_on(PipelineRunner::new(topology, Box::new(store.clone())))
    //       .unwrap();
    //   (runner, store)
    todo!("Implement once milestone 2.3 and 3.1 APIs are finalized")
}

/// Build a test runner from TOML, sharing an existing state store
/// (for resume tests that need to reuse the same store across runs).
pub fn build_test_runner_with_store(
    toml_path: &Path,
    store: Arc<InMemoryStateStore>,
) -> PipelineRunner {
    // Same as build_test_runner but uses the provided store instead
    // of creating a new one.
    todo!("Implement once milestone 2.3 and 3.1 APIs are finalized")
}

/// A stage that always fails with a Permanent error.
/// Used for testing skip_on_error and failure propagation.
#[derive(Debug)]
pub struct AlwaysFailingStage {
    pub name: String,
}

impl AlwaysFailingStage {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

#[async_trait::async_trait]
impl Stage for AlwaysFailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: self.name.clone(),
            item_id: item.id.clone(),
            message: "always fails".to_string(),
        })
    }
}

/// A stage that fails for the first N calls, then succeeds.
/// Used for testing checkpoint/resume: the pipeline will fail mid-run,
/// then on resume (with fail_count reset) it will succeed.
#[derive(Debug)]
pub struct FailingStage {
    pub name: String,
    pub fail_count: std::sync::atomic::AtomicU32,
    pub fail_until: u32,
}

impl FailingStage {
    pub fn new(name: &str, fail_until: u32) -> Self {
        Self {
            name: name.to_string(),
            fail_count: std::sync::atomic::AtomicU32::new(0),
            fail_until,
        }
    }
}

#[async_trait::async_trait]
impl Stage for FailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        let count = self.fail_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if count < self.fail_until {
            Err(StageError::Transient {
                stage: self.name.clone(),
                item_id: item.id.clone(),
                message: format!("simulated failure #{}", count + 1),
            })
        } else {
            Ok(vec![item])
        }
    }
}

/// A stage that fails only for items whose IDs contain a given substring.
/// Used for selective failure testing (e.g., one bad file among many).
#[derive(Debug)]
pub struct SelectiveFailingStage {
    pub name: String,
    pub fail_pattern: String,
}

impl SelectiveFailingStage {
    pub fn new(name: &str, fail_pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            fail_pattern: fail_pattern.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Stage for SelectiveFailingStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> std::result::Result<Vec<PipelineItem>, StageError> {
        if item.id.contains(&self.fail_pattern) {
            Err(StageError::Permanent {
                stage: self.name.clone(),
                item_id: item.id.clone(),
                message: format!("item {} matches fail pattern", item.id),
            })
        } else {
            Ok(vec![item])
        }
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Add dev-dependencies and scaffold test directory

- [ ] Add `ecl-adapter-fs`, `ecl-stages`, `tempfile`, `serde_json` to
      `[dev-dependencies]` in `crates/ecl-pipeline/Cargo.toml` (from section 5)
- [ ] Create `crates/ecl-pipeline/tests/` directory
- [ ] Create `crates/ecl-pipeline/tests/common/mod.rs` with the helper
      functions from section 6 (initially using `todo!()` for `build_test_runner`
      and `build_test_runner_with_store` — fill in once you verify the exact
      APIs of milestones 2.3 and 3.1)
- [ ] Create a minimal `crates/ecl-pipeline/tests/full_pipeline.rs` with a
      single `#[test]` that calls `create_test_files` to verify helpers compile
- [ ] Run `cargo test -p ecl-pipeline --test full_pipeline` — must compile
      (test may be trivial at this point)
- [ ] Commit: `test(ecl-pipeline): scaffold integration test directory`

### Step 2: Implement test helpers

- [ ] Implement `create_test_files()` — creates `file_000.txt` through
      `file_NNN.txt` in the given directory, each containing
      `"Content of file_NNN"`
- [ ] Implement `write_test_toml()` — writes a pipeline TOML with one
      filesystem source and three stages (extract, normalize, emit)
- [ ] Implement `write_concurrent_toml()` — writes a pipeline TOML with two
      filesystem sources and concurrent extract stages
- [ ] Implement `write_skip_on_error_toml()` — writes a pipeline TOML where
      the normalize stage has `skip_on_error = true`
- [ ] Implement `build_test_runner()` — parses TOML, creates
      `FilesystemAdapter` for each filesystem source, registers built-in stages
      (extract, normalize, emit), resolves topology, creates runner with
      `InMemoryStateStore`
- [ ] Implement `build_test_runner_with_store()` — same but accepts an
      existing `Arc<InMemoryStateStore>` for resume testing
- [ ] Add `AlwaysFailingStage`, `FailingStage`, and `SelectiveFailingStage`
      to `common/mod.rs` (from section 6)
- [ ] Write a smoke test in `full_pipeline.rs` that creates a temp dir, writes
      test files, writes TOML, and builds a runner (verifying the helper chain)
- [ ] Run `cargo test -p ecl-pipeline --test full_pipeline` — must pass
- [ ] Commit: `test(ecl-pipeline): implement integration test helpers`

### Step 3: Happy-path end-to-end test (full_pipeline.rs)

- [ ] Write `test_full_pipeline_happy_path_completes`:
  - Create `TempDir`
  - Create 10 text files via `create_test_files()`
  - Write TOML via `write_test_toml()`
  - Build runner via `build_test_runner()`
  - Call `runner.run().await`
  - Assert `state.status` is `PipelineStatus::Completed`
  - Assert `state.stats.total_items_discovered == 10`
  - Assert `state.stats.total_items_processed == 10`
  - Assert `state.stats.total_items_failed == 0`
  - Assert all items in `state.sources["local"].items` have
    `ItemStatus::Completed`
  - Assert output files exist in `output/normalized/` directory (10 files)
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add happy-path end-to-end test`

### Step 4: Concurrent stages test (full_pipeline.rs)

- [ ] Write `test_full_pipeline_concurrent_stages_both_execute`:
  - Create `TempDir` with two subdirectories (`source-a/`, `source-b/`)
  - Create 3 files in `source-a/`, 3 files in `source-b/`
  - Write TOML via `write_concurrent_toml()`
  - Build runner and execute
  - Assert both `source-a` and `source-b` appear in `state.sources`
  - Assert both `extract-a` and `extract-b` stages have
    `StageStatus::Completed`
  - Assert `state.stats.total_items_discovered == 6`
  - Assert the `extract-a` and `extract-b` stages are in the same batch in the
    topology schedule (verifying they could run concurrently)
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add concurrent stages test`

### Step 5: State inspection test (full_pipeline.rs)

- [ ] Write `test_full_pipeline_state_serializes_to_json`:
  - Run the happy-path pipeline (10 files, 3 stages)
  - Serialize `runner.state()` to JSON via `serde_json::to_value()`
  - Assert the JSON object has the expected top-level keys:
    `run_id`, `pipeline_name`, `started_at`, `last_checkpoint`, `status`,
    `current_batch`, `sources`, `stages`, `stats`
  - Assert `json["status"]` matches `{"Completed": {"finished_at": "..."}}`
  - Assert `json["sources"]["local"]["items_discovered"]` is `10`
  - Assert `json["stages"]` contains entries for `"extract"`, `"normalize"`,
    `"emit"`
  - Assert `json["stats"]["total_items_processed"]` is `10`
  - This verifies the JSON shape matches design doc section 6
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add state JSON inspection test`

### Step 6: Checkpoint and resume test (checkpoint_resume.rs)

- [ ] Write `test_checkpoint_resume_skips_completed_batches`:
  - Create `TempDir` with 5 text files
  - Build a custom topology that uses a `FailingStage` as the normalize stage
    (fails on the first 3 calls, then succeeds)
  - Use a shared `Arc<InMemoryStateStore>` for both runs
  - **Run 1:** Execute the pipeline — it should fail mid-normalize (after
    extract completes successfully for all items)
  - Assert the error is `PipelineError::ItemFailed`
  - Verify a checkpoint was saved (store has Some checkpoint)
  - Verify the checkpoint's `current_batch` indicates extract batch completed
  - **Run 2:** Build a new runner with the same store (the FailingStage's
    counter is reset, so it now succeeds)
  - Execute `runner.run().await`
  - Assert the pipeline completes with `PipelineStatus::Completed`
  - Verify that the extract stage was NOT re-run (the checkpoint resume skips
    batch 0, which is the extract batch)
  - Verify items_processed matches the expected count
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add checkpoint/resume test`

### Step 7: Incrementality test (incrementality.rs)

- [ ] Write `test_incrementality_skips_unchanged_files`:
  - Create `TempDir` with 5 text files
  - Use a shared `Arc<InMemoryStateStore>` for both runs
  - **Run 1:** Execute pipeline to completion
  - Assert all 5 items processed, `total_items_processed == 5`
  - Assert completed hashes were saved (via store)
  - **Modify one file:** Change the content of `file_002.txt`
  - **Run 2:** Build a new runner with the same store, execute
  - Assert `state.stats.total_items_skipped_unchanged == 4` (the 4 unchanged
    files were detected via hash comparison)
  - Assert `state.stats.total_items_processed == 1` (only the modified file)
  - Assert the item for `file_002.txt` has `ItemStatus::Completed` (not
    `Unchanged`)
  - Assert the items for the other 4 files have `ItemStatus::Unchanged`
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add incrementality test`

### Step 8: Error handling tests (error_handling.rs)

- [ ] Write `test_error_skip_on_error_continues_pipeline`:
  - Create `TempDir` with 5 text files
  - Build a topology where the normalize stage uses a
    `SelectiveFailingStage` that fails for items containing `"file_002"` in
    their ID, with `skip_on_error = true`
  - Execute the pipeline
  - Assert the pipeline completes with `PipelineStatus::Completed`
  - Assert the item for `file_002.txt` has `ItemStatus::Skipped`
  - Assert the other 4 items have `ItemStatus::Completed`
  - Assert `state.stats.total_items_failed == 0` (skipped, not failed)
  - Assert `state.stats.total_items_processed == 4`

- [ ] Write `test_error_stage_failure_without_skip_propagates`:
  - Create `TempDir` with 5 text files
  - Build a topology where the normalize stage uses an `AlwaysFailingStage`
    with `skip_on_error = false`
  - Execute the pipeline
  - Assert the result is `Err(PipelineError::ItemFailed { ... })`
  - Assert the pipeline state has at least one item with `ItemStatus::Failed`

- [ ] Write `test_error_partial_failure_with_skip_records_correctly`:
  - Create `TempDir` with 10 text files
  - Build a topology where the normalize stage uses a
    `SelectiveFailingStage` that fails for items containing `"file_003"` or
    `"file_007"`, with `skip_on_error = true`
  - Execute the pipeline
  - Assert exactly 2 items are `Skipped`, 8 items are `Completed`
  - Assert the stage state for normalize shows `items_skipped == 2`

- [ ] Run tests, verify pass
- [ ] Commit: `test(ecl-pipeline): add error handling tests`

### Step 9: Fan-in aggregation test (fan_in.rs)

- [ ] Write `test_fan_in_emit_receives_combined_items`:
  - Create `TempDir` with two source directories: `source-a/` (3 files) and
    `source-b/` (2 files)
  - Write a TOML with two filesystem sources, two extract stages creating
    `raw-a` and `raw-b`, two normalize stages creating `normalized-a` and
    `normalized-b`, and one emit stage that reads
    `["normalized-a", "normalized-b"]`
  - Execute the pipeline
  - Assert `state.stats.total_items_discovered == 5`
  - Assert `state.stats.total_items_processed == 5`
  - Assert output files in the emit directory total 5 files (combined from
    both sources)
  - Assert the emit stage state shows `items_processed == 5`
- [ ] Run test, verify pass
- [ ] Commit: `test(ecl-pipeline): add fan-in aggregation test`

### Step 10: Final polish and coverage

- [ ] Run `cargo test -p ecl-pipeline` — all unit and integration tests pass
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Check code coverage across the `ecl-pipeline` crate (including coverage
      contributed by integration tests hitting library code paths)
- [ ] Add additional edge-case tests if coverage gaps exist:
  - Empty source directory (0 files)
  - Single file pipeline
  - Pipeline where all items are unchanged on second run (0 to process)
- [ ] Verify no compiler warnings
- [ ] Commit: `test(ecl-pipeline): integration test polish and coverage`

## 8. Test Fixtures

### Directory Structure (created by helpers)

For the happy-path test, the temp directory looks like:

```
$TMPDIR/ecl-test-XXXXX/
├── sources/
│   ├── file_000.txt    # "Content of file_000"
│   ├── file_001.txt    # "Content of file_001"
│   ├── ...
│   └── file_009.txt    # "Content of file_009"
├── output/
│   └── normalized/     # Created by emit stage
│       ├── file_000.txt
│       ├── ...
│       └── file_009.txt
└── pipeline.toml
```

### Minimal Pipeline TOML (single source, 3 stages)

```toml
name = "integration-test"
version = 1
output_dir = "$TMPDIR/output"

[defaults]
concurrency = 4
checkpoint = { every = "Batch" }

[defaults.retry]
max_attempts = 1
initial_backoff_ms = 10
backoff_multiplier = 1.0
max_backoff_ms = 10

[sources.local]
kind = "filesystem"
root = "$TMPDIR/sources"
extensions = ["txt"]

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw-docs"] }

[stages.normalize]
adapter = "normalize"
resources = { reads = ["raw-docs"], creates = ["normalized-docs"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["normalized-docs"] }

[stages.emit.params]
subdir = "normalized"
```

### Concurrent Pipeline TOML (two sources, parallel extracts)

```toml
name = "concurrent-test"
version = 1
output_dir = "$TMPDIR/output"

[defaults]
concurrency = 4

[sources.source-a]
kind = "filesystem"
root = "$TMPDIR/source-a"
extensions = ["txt"]

[sources.source-b]
kind = "filesystem"
root = "$TMPDIR/source-b"
extensions = ["txt"]

[stages.extract-a]
adapter = "extract"
source = "source-a"
resources = { creates = ["raw-a"] }

[stages.extract-b]
adapter = "extract"
source = "source-b"
resources = { creates = ["raw-b"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-a", "raw-b"] }

[stages.emit.params]
subdir = "combined"
```

### Fan-In Pipeline TOML (two sources, per-source stages, combined emit)

```toml
name = "fan-in-test"
version = 1
output_dir = "$TMPDIR/output"

[defaults]
concurrency = 4

[sources.source-a]
kind = "filesystem"
root = "$TMPDIR/source-a"
extensions = ["txt"]

[sources.source-b]
kind = "filesystem"
root = "$TMPDIR/source-b"
extensions = ["txt"]

[stages.extract-a]
adapter = "extract"
source = "source-a"
resources = { creates = ["raw-a"] }

[stages.extract-b]
adapter = "extract"
source = "source-b"
resources = { creates = ["raw-b"] }

[stages.normalize-a]
adapter = "normalize"
resources = { reads = ["raw-a"], creates = ["normalized-a"] }

[stages.normalize-b]
adapter = "normalize"
resources = { reads = ["raw-b"], creates = ["normalized-b"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["normalized-a", "normalized-b"] }

[stages.emit.params]
subdir = "combined"
```

## 9. Test Specifications

| Test Name | File | What It Verifies |
|-----------|------|-----------------|
| `test_full_pipeline_happy_path_completes` | `full_pipeline.rs` | 10 text files through 3 stages (extract, normalize, emit); all items Completed; output files exist on disk |
| `test_full_pipeline_concurrent_stages_both_execute` | `full_pipeline.rs` | Two independent extract stages (source-a, source-b) both complete; 6 items discovered across both sources; stages in same batch |
| `test_full_pipeline_state_serializes_to_json` | `full_pipeline.rs` | PipelineState serializes to JSON with keys matching design doc section 6 shape: run_id, pipeline_name, status, sources, stages, stats |
| `test_checkpoint_resume_skips_completed_batches` | `checkpoint_resume.rs` | Run 1 fails mid-normalize (FailingStage), checkpoint saved; Run 2 resumes from checkpoint, extract batch NOT re-run, pipeline completes |
| `test_incrementality_skips_unchanged_files` | `incrementality.rs` | Run 1 processes 5 files; modify 1 file; Run 2 skips 4 unchanged (hash match), processes only the modified file |
| `test_error_skip_on_error_continues_pipeline` | `error_handling.rs` | One file causes stage failure with skip_on_error=true; pipeline completes; failing item Skipped; other 4 items Completed |
| `test_error_stage_failure_without_skip_propagates` | `error_handling.rs` | AlwaysFailingStage with skip_on_error=false; run() returns PipelineError::ItemFailed; items marked Failed in state |
| `test_error_partial_failure_with_skip_records_correctly` | `error_handling.rs` | 2 of 10 items fail with skip_on_error=true; exactly 2 Skipped, 8 Completed; stage state reflects items_skipped=2 |
| `test_fan_in_emit_receives_combined_items` | `fan_in.rs` | Emit stage reads ["normalized-a", "normalized-b"]; receives combined 5 items from both sources; output directory has 5 files |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo test -p ecl-pipeline` passes (all unit tests and integration tests green)
- [ ] `cargo test -p ecl-pipeline --test full_pipeline` passes
- [ ] `cargo test -p ecl-pipeline --test checkpoint_resume` passes
- [ ] `cargo test -p ecl-pipeline --test incrementality` passes
- [ ] `cargo test -p ecl-pipeline --test error_handling` passes
- [ ] `cargo test -p ecl-pipeline --test fan_in` passes
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] Integration tests exercise all major `PipelineRunner` code paths:
  - Fresh run (no checkpoint)
  - Resume from checkpoint
  - Incrementality (hash comparison and unchanged skipping)
  - Batch execution with concurrent stages
  - skip_on_error item handling
  - Hard failure propagation
  - Fan-in resource aggregation
  - State serialization to JSON

### What NOT to Do

- Do NOT modify any existing crate source code (no changes to `ecl-pipeline/src/`, `ecl-adapter-fs/src/`, `ecl-stages/src/`, etc.)
- Do NOT add new features to the pipeline runner
- Do NOT implement CLI commands
- Do NOT create new crates
- Do NOT implement new stage types in library code (test-only mock stages in `tests/common/mod.rs` are fine)
- Do NOT add non-dev dependencies to any crate
- Do NOT modify the root workspace `Cargo.toml` (only `crates/ecl-pipeline/Cargo.toml` dev-dependencies)
- These are integration tests ONLY — they verify existing functionality, not add new functionality
