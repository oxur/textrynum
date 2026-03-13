# Milestone 5.1: CLI Commands (`ecl-cli` pipeline subcommands)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-cli):`

## 1. Goal

Extend the existing `ecl-cli` crate with `ecl pipeline` subcommands. When done:

1. `ecl pipeline run <config.toml>` parses a TOML pipeline spec, registers
   adapters and stages, creates a `PipelineRunner`, executes the pipeline, and
   prints a summary (items processed/failed/skipped).
2. `ecl pipeline resume <output-dir>` loads the latest checkpoint from redb,
   optionally forces resume despite config drift with `--force`, and resumes
   execution from the last checkpoint.
3. `ecl pipeline status <output-dir>` prints a human-readable progress summary
   with one line per stage showing status, items processed, items failed, and
   timing.
4. `ecl pipeline inspect <output-dir>` loads the full `PipelineState` from redb
   and prints it as pretty-printed JSON (designed for Claude consumption).
5. `ecl pipeline items <output-dir>` lists all items across all sources with
   their status, source, content hash, and completed stages in table format.
6. `ecl pipeline diff <dir1> <dir2>` compares the state of two pipeline runs
   and reports new, changed (hash differs), and removed items, plus stage
   outcome differences.
7. Exit codes follow a convention: 0 = success (all items completed),
   1 = failure (pipeline error), 2 = partial (some items failed, pipeline
   completed).

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, CliError>;`
- **Test naming:** `test_<fn>_<scenario>_<expectation>`
- **Tests:** Inline `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { ... }`
- **Maps:** Always `BTreeMap` (not `HashMap`) for deterministic serialization
- **Params:** Use `&str` not `&String`, `&[T]` not `&Vec<T>` in function signatures
- **No `unwrap()`** in library code -- use `?` or `.ok_or()`
- **Doc comments** on all public items

### 2.2 Rust Guides to Load

Before writing any code, read these files (paths relative to workspace root):

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` (ALWAYS first)
2. `assets/ai/ai-rust/guides/01-core-idioms.md`
3. `assets/ai/ai-rust/guides/14-cli-tools/01-project-setup.md`
4. `assets/ai/ai-rust/guides/14-cli-tools/02-argument-parsing.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` -- Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` -- Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-cli/src/main.rs` -- Existing CLI entry point (this is what you extend)

## 3. Prior Art / Dependencies

This crate extends the existing `ecl-cli` and depends on all prior milestone
crates. Their complete public APIs relevant to this milestone are listed here
so you do not need to read any other files.

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
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- crate: ecl-pipeline-state ---

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

#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    fn source_kind(&self) -> &str;
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
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
```

### From `ecl-pipeline` (Milestone 2.2)

```rust
// --- crate: ecl-pipeline ---

/// The pipeline execution engine.
pub struct PipelineRunner {
    topology: PipelineTopology,
    state: PipelineState,
    state_store: Box<dyn StateStore>,
}

impl PipelineRunner {
    /// Create a new runner. Loads checkpoint if available.
    /// Detects config drift and errors unless force_resume is true.
    pub async fn new(
        topology: PipelineTopology,
        state_store: Box<dyn StateStore>,
        force_resume: bool,
    ) -> Result<Self>;

    /// Execute the full pipeline lifecycle.
    /// Returns the final PipelineState.
    pub async fn run(&mut self) -> Result<PipelineState>;
}

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

pub type Result<T> = std::result::Result<T, PipelineError>;
```

### From `ecl-pipeline-state` (Milestone 2.1 -- RedbStateStore)

```rust
// --- crate: ecl-pipeline-state ---

/// Redb-backed state store for production use.
pub struct RedbStateStore { /* ... */ }

impl RedbStateStore {
    /// Open or create a redb database at the given path.
    pub fn new(db_path: &Path) -> Result<Self>;
}
// Implements StateStore.
```

### From adapter/stage crates (Milestones 3.x, 4.x)

```rust
// --- crate: ecl-adapter-fs ---
pub struct FilesystemAdapter { /* ... */ }
impl FilesystemAdapter {
    pub fn new(spec: &FilesystemSourceSpec) -> Self;
}
// Implements SourceAdapter.

// --- crate: ecl-adapter-gdrive ---
pub struct GoogleDriveAdapter { /* ... */ }
impl GoogleDriveAdapter {
    pub async fn new(spec: &GoogleDriveSourceSpec) -> Result<Self, SourceError>;
}
// Implements SourceAdapter.

// --- crate: ecl-stages ---
pub struct ExtractStage;
pub struct NormalizeStage;
pub struct FilterStage;
pub struct EmitStage;
// Each implements Stage.
// Registry function to look up stage by adapter name:
pub fn create_stage(adapter_name: &str) -> Option<Arc<dyn Stage>>;
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/ecl-cli/Cargo.toml` | Modify | Add pipeline crate dependencies |
| `crates/ecl-cli/src/main.rs` | Modify | Add `Pipeline` subcommand variant, route to handler |
| `crates/ecl-cli/src/error.rs` | Create | `CliError` enum wrapping pipeline errors and I/O |
| `crates/ecl-cli/src/pipeline/mod.rs` | Create | `PipelineCmd` enum with subcommands, dispatch function |
| `crates/ecl-cli/src/pipeline/run.rs` | Create | `handle_run()` -- parse TOML, build topology, execute |
| `crates/ecl-cli/src/pipeline/resume.rs` | Create | `handle_resume()` -- load checkpoint, resume execution |
| `crates/ecl-cli/src/pipeline/status.rs` | Create | `handle_status()` -- load state, print table |
| `crates/ecl-cli/src/pipeline/inspect.rs` | Create | `handle_inspect()` -- load state, print JSON |
| `crates/ecl-cli/src/pipeline/items.rs` | Create | `handle_items()` -- load state, print item table |
| `crates/ecl-cli/src/pipeline/diff.rs` | Create | `handle_diff()` -- compare two run states |
| `crates/ecl-cli/src/pipeline/output.rs` | Create | Shared output formatting helpers (tables, JSON, summary) |

## 5. Cargo.toml

### Root Workspace Additions

No new workspace-level dependencies are needed. All required dependencies
already exist in `[workspace.dependencies]`.

No new workspace members are needed -- `ecl-cli` is already a member.

### Crate Cargo.toml Changes

Add these dependencies to `crates/ecl-cli/Cargo.toml`:

```toml
[dependencies]
# Internal dependencies (existing)
ecl-core = { version = "0.3.0", path = "../ecl-core" }
ecl-steps = { version = "0.3.0", path = "../ecl-steps" }
ecl-workflows = { version = "0.3.0", path = "../ecl-workflows" }

# Pipeline dependencies (NEW)
ecl-pipeline = { path = "../ecl-pipeline" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-adapter-fs = { path = "../ecl-adapter-fs" }
ecl-adapter-gdrive = { path = "../ecl-adapter-gdrive" }
ecl-stages = { path = "../ecl-stages" }

# Workspace dependencies (existing)
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
clap = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
confyg = { workspace = true }
twyg = { workspace = true }

# Workspace dependencies (NEW)
chrono = { workspace = true }
```

Also add the `missing_docs` lint:

```toml
[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown.

### `src/error.rs`

```rust
//! Error types for the ECL CLI.

use thiserror::Error;

/// Errors that can occur in the ECL CLI.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum CliError {
    /// Failed to read a file from disk.
    #[error("failed to read file '{path}': {source}")]
    FileRead {
        /// The file path that could not be read.
        path: String,
        /// The underlying I/O error.
        source: std::io::Error,
    },

    /// Pipeline specification parsing or validation failed.
    #[error("pipeline spec error: {0}")]
    Spec(#[from] ecl_pipeline_spec::SpecError),

    /// Pipeline execution error.
    #[error("pipeline error: {0}")]
    Pipeline(#[from] ecl_pipeline::PipelineError),

    /// State store error (e.g., redb open/read failure).
    #[error("state error: {0}")]
    State(#[from] ecl_pipeline_state::StateError),

    /// No checkpoint found when one was expected (e.g., resume or status).
    #[error("no checkpoint found in '{path}'")]
    NoCheckpoint {
        /// The output directory that was searched.
        path: String,
    },

    /// JSON serialization error (for inspect output).
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// The output directory does not exist.
    #[error("output directory does not exist: '{path}'")]
    OutputDirNotFound {
        /// The missing output directory path.
        path: String,
    },
}

/// Result type for CLI operations.
pub type Result<T> = std::result::Result<T, CliError>;
```

### `src/main.rs`

```rust
#![forbid(unsafe_code)]

//! ECL CLI
//!
//! Command-line interface for ECL.

mod error;
mod pipeline;

use anyhow::Result;
use clap::{Parser, Subcommand};

/// ECL Command-Line Interface
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Option<Commands>,
}

/// Top-level CLI subcommands.
#[derive(Subcommand, Debug)]
enum Commands {
    /// Pipeline management commands
    Pipeline(pipeline::PipelineCmd),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Pipeline(cmd)) => {
            let exit_code = pipeline::dispatch(cmd).await;
            std::process::exit(exit_code);
        }
        None => {
            println!("ECL CLI - Use --help for available commands");
            Ok(())
        }
    }?;

    Ok(())
}
```

### `src/pipeline/mod.rs`

```rust
//! Pipeline subcommand definitions and dispatch.

pub mod diff;
pub mod inspect;
pub mod items;
pub mod output;
pub mod resume;
pub mod run;
pub mod status;

use clap::{Args, Subcommand};
use std::path::PathBuf;

/// Pipeline management commands.
#[derive(Args, Debug)]
pub struct PipelineCmd {
    /// Pipeline subcommand to execute.
    #[command(subcommand)]
    pub command: PipelineSubcommand,
}

/// Available pipeline subcommands.
#[derive(Subcommand, Debug)]
pub enum PipelineSubcommand {
    /// Execute a pipeline from a TOML configuration file.
    Run {
        /// Path to the pipeline configuration TOML file.
        config: PathBuf,
    },

    /// Resume a pipeline from its last checkpoint.
    Resume {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,

        /// Resume even if the configuration has changed since the checkpoint.
        #[arg(long)]
        force: bool,
    },

    /// Show human-readable pipeline progress summary.
    Status {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,
    },

    /// Print full pipeline state as JSON (for Claude / programmatic use).
    Inspect {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,
    },

    /// List all items across all sources with their processing status.
    Items {
        /// Path to the pipeline output directory.
        output_dir: PathBuf,
    },

    /// Compare the state of two pipeline runs.
    Diff {
        /// Path to the first pipeline output directory.
        dir1: PathBuf,

        /// Path to the second pipeline output directory.
        dir2: PathBuf,
    },
}

/// Dispatch a pipeline subcommand. Returns the exit code.
///
/// Exit codes:
/// - 0: success (all items completed)
/// - 1: failure (pipeline error or CLI error)
/// - 2: partial (some items failed, pipeline completed)
pub async fn dispatch(cmd: PipelineCmd) -> crate::error::Result<i32> {
    match cmd.command {
        PipelineSubcommand::Run { config } => run::handle_run(&config).await,
        PipelineSubcommand::Resume { output_dir, force } => {
            resume::handle_resume(&output_dir, force).await
        }
        PipelineSubcommand::Status { output_dir } => status::handle_status(&output_dir).await,
        PipelineSubcommand::Inspect { output_dir } => inspect::handle_inspect(&output_dir).await,
        PipelineSubcommand::Items { output_dir } => items::handle_items(&output_dir).await,
        PipelineSubcommand::Diff { dir1, dir2 } => diff::handle_diff(&dir1, &dir2).await,
    }
}
```

### `src/pipeline/run.rs`

```rust
//! Handler for `ecl pipeline run <config.toml>`.

use std::path::Path;
use std::sync::Arc;

use ecl_pipeline::PipelineRunner;
use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::RedbStateStore;
use ecl_pipeline_topo::PipelineTopology;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// Execute a pipeline from a TOML configuration file.
///
/// Steps:
/// 1. Read and parse the TOML config file.
/// 2. Build adapters and stages based on the spec.
/// 3. Resolve the topology (spec -> topology with schedule).
/// 4. Open or create the redb state store in the output directory.
/// 5. Create a PipelineRunner and execute the pipeline.
/// 6. Print a summary of the run.
///
/// Returns exit code: 0 = success, 1 = failure, 2 = partial.
pub async fn handle_run(config_path: &Path) -> Result<i32> {
    // 1. Read TOML
    let toml_str = std::fs::read_to_string(config_path).map_err(|e| CliError::FileRead {
        path: config_path.display().to_string(),
        source: e,
    })?;

    // 2. Parse spec
    let spec = PipelineSpec::from_toml(&toml_str)?;
    let output_dir = spec.output_dir.clone();

    tracing::info!(pipeline = %spec.name, "Starting pipeline run");

    // 3. Create output directory if needed
    std::fs::create_dir_all(&output_dir).map_err(|e| CliError::FileRead {
        path: output_dir.display().to_string(),
        source: e,
    })?;

    // 4. Build topology: register adapters and stages from spec
    let topology = build_topology(spec)?;

    // 5. Open state store
    let db_path = output_dir.join("pipeline.redb");
    let state_store = Box::new(RedbStateStore::new(&db_path)?);

    // 6. Create runner and execute
    let mut runner = PipelineRunner::new(topology, state_store, false).await?;
    let final_state = runner.run().await?;

    // 7. Print summary and return exit code
    output::print_run_summary(&final_state);
    Ok(output::exit_code_from_state(&final_state))
}

/// Build a PipelineTopology from a PipelineSpec by registering all adapters
/// and stages referenced in the spec.
fn build_topology(spec: PipelineSpec) -> Result<PipelineTopology> {
    use ecl_adapter_fs::FilesystemAdapter;
    use ecl_adapter_gdrive::GoogleDriveAdapter;
    use ecl_pipeline_spec::SourceSpec;
    use ecl_pipeline_topo::TopologyBuilder;
    use ecl_stages::create_stage;

    let spec = Arc::new(spec);
    let mut builder = TopologyBuilder::new(spec.clone());

    // Register source adapters
    for (name, source_spec) in &spec.sources {
        match source_spec {
            SourceSpec::Filesystem(fs_spec) => {
                let adapter = FilesystemAdapter::new(fs_spec);
                builder.register_source(name, Arc::new(adapter));
            }
            SourceSpec::GoogleDrive(gdrive_spec) => {
                // GoogleDriveAdapter::new is async but we handle it here
                // For now, register synchronously; async init happens on first use
                let adapter = tokio::runtime::Handle::current()
                    .block_on(GoogleDriveAdapter::new(gdrive_spec))
                    .map_err(ecl_pipeline::PipelineError::Source)?;
                builder.register_source(name, Arc::new(adapter));
            }
            SourceSpec::Slack(_) => {
                tracing::warn!(source = %name, "Slack adapter not yet implemented, skipping");
            }
        }
    }

    // Register stages
    for (name, stage_spec) in &spec.stages {
        if let Some(stage) = create_stage(&stage_spec.adapter) {
            builder.register_stage(name, stage);
        } else {
            tracing::warn!(
                stage = %name,
                adapter = %stage_spec.adapter,
                "Unknown stage adapter, skipping"
            );
        }
    }

    // Resolve topology (computes schedule from resource declarations)
    let topology = builder.resolve()?;
    Ok(topology)
}
```

### `src/pipeline/resume.rs`

```rust
//! Handler for `ecl pipeline resume <output-dir>`.

use std::path::Path;

use ecl_pipeline::PipelineRunner;
use ecl_pipeline_state::RedbStateStore;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// Resume a pipeline from its last checkpoint.
///
/// Steps:
/// 1. Verify the output directory exists.
/// 2. Open the redb state store.
/// 3. Load the checkpoint. If none exists, return an error.
/// 4. The PipelineRunner handles config drift detection and --force logic.
/// 5. Print "Resuming from checkpoint (sequence N)" header.
/// 6. Execute the pipeline.
/// 7. Print summary and return exit code.
///
/// Returns exit code: 0 = success, 1 = failure, 2 = partial.
pub async fn handle_resume(output_dir: &Path, force: bool) -> Result<i32> {
    // 1. Verify directory
    if !output_dir.exists() {
        return Err(CliError::OutputDirNotFound {
            path: output_dir.display().to_string(),
        });
    }

    // 2. Open state store
    let db_path = output_dir.join("pipeline.redb");
    let state_store = Box::new(RedbStateStore::new(&db_path)?);

    // 3. Load checkpoint to print header info
    let checkpoint = state_store
        .load_checkpoint()
        .await?
        .ok_or_else(|| CliError::NoCheckpoint {
            path: output_dir.display().to_string(),
        })?;

    let sequence = checkpoint.sequence;
    let pipeline_name = checkpoint.state.pipeline_name.clone();

    tracing::info!(
        pipeline = %pipeline_name,
        sequence = sequence,
        force = force,
        "Resuming from checkpoint"
    );

    println!("Resuming pipeline '{}' from checkpoint (sequence {})", pipeline_name, sequence);
    if force {
        println!("  --force: ignoring config drift");
    }

    // 4. Rebuild topology from checkpoint spec
    let topology = super::run::build_topology_from_checkpoint(&checkpoint)?;

    // 5. Create runner with force_resume flag
    // Re-open state store since we consumed the first one to load checkpoint
    let state_store = Box::new(RedbStateStore::new(&db_path)?);
    let mut runner = PipelineRunner::new(topology, state_store, force).await?;

    // 6. Execute
    let final_state = runner.run().await?;

    // 7. Print summary
    output::print_run_summary(&final_state);
    Ok(output::exit_code_from_state(&final_state))
}
```

**Note on `build_topology_from_checkpoint`:** The `run.rs` module must also
export a helper that builds a topology from a `Checkpoint`'s embedded spec
(rather than from a TOML file). This reuses the same adapter/stage registration
logic from `build_topology` but uses `checkpoint.spec` as input:

```rust
// In src/pipeline/run.rs, add:

/// Build a PipelineTopology from a checkpoint's embedded spec.
/// Used by resume to reconstruct the topology without re-reading TOML.
pub fn build_topology_from_checkpoint(
    checkpoint: &ecl_pipeline_state::Checkpoint,
) -> Result<PipelineTopology> {
    build_topology(checkpoint.spec.clone())
}
```

### `src/pipeline/status.rs`

```rust
//! Handler for `ecl pipeline status <output-dir>`.

use std::path::Path;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// Print a human-readable pipeline status summary.
///
/// Output format:
/// ```text
/// Pipeline: q1-knowledge-sync
/// Status:   Running (batch 1, stage: normalize-gdrive)
/// Started:  2026-03-13 14:30:00 UTC
///
/// STAGE                STATUS      PROCESSED  FAILED  SKIPPED  DURATION
/// fetch-gdrive         Completed          45       0        0      12.3s
/// fetch-slack          Completed          32       0        0       8.1s
/// normalize-gdrive     Running            38       2        0      24.7s
/// normalize-slack      Completed          32       0        0       5.2s
/// emit                 Pending             0       0        0         --
///
/// Items: 147 processed, 422 skipped (unchanged), 2 failed, 1047 discovered
/// ```
///
/// Returns exit code: always 0 (status is informational).
pub async fn handle_status(output_dir: &Path) -> Result<i32> {
    if !output_dir.exists() {
        return Err(CliError::OutputDirNotFound {
            path: output_dir.display().to_string(),
        });
    }

    let state = output::load_state(output_dir).await?;
    output::print_status_table(&state);

    Ok(0)
}
```

### `src/pipeline/inspect.rs`

```rust
//! Handler for `ecl pipeline inspect <output-dir>`.

use std::path::Path;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// Print the full pipeline state as pretty-printed JSON.
///
/// This output is designed for programmatic consumption and for
/// handing to Claude for analysis. It contains the complete
/// `PipelineState` struct serialized as JSON.
///
/// Returns exit code: always 0 (inspect is informational).
pub async fn handle_inspect(output_dir: &Path) -> Result<i32> {
    if !output_dir.exists() {
        return Err(CliError::OutputDirNotFound {
            path: output_dir.display().to_string(),
        });
    }

    let state = output::load_state(output_dir).await?;
    let json = serde_json::to_string_pretty(&state)?;
    println!("{}", json);

    Ok(0)
}
```

### `src/pipeline/items.rs`

```rust
//! Handler for `ecl pipeline items <output-dir>`.

use std::path::Path;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// List all items across all sources with their processing status.
///
/// Output format:
/// ```text
/// SOURCE               ITEM                              STATUS       HASH        STAGES_COMPLETED
/// engineering-drive     Q1 Architecture Review.docx       Completed    a7f3b2...   fetch-gdrive, normalize-gdrive
/// engineering-drive     Meeting Notes (old).pdf            Failed       c9e1a4...   fetch-gdrive
/// team-slack            Thread: Architecture decision      Completed    b3d7f1...   fetch-slack, normalize-slack
/// team-slack            Thread: Sprint planning            Unchanged    --          --
/// ```
///
/// Returns exit code: always 0 (items listing is informational).
pub async fn handle_items(output_dir: &Path) -> Result<i32> {
    if !output_dir.exists() {
        return Err(CliError::OutputDirNotFound {
            path: output_dir.display().to_string(),
        });
    }

    let state = output::load_state(output_dir).await?;
    output::print_items_table(&state);

    Ok(0)
}
```

### `src/pipeline/diff.rs`

```rust
//! Handler for `ecl pipeline diff <dir1> <dir2>`.

use std::collections::BTreeSet;
use std::path::Path;

use ecl_pipeline_state::PipelineState;

use crate::error::{CliError, Result};
use crate::pipeline::output;

/// Compare the state of two pipeline runs and report differences.
///
/// Reports:
/// - NEW items: present in dir2 but not dir1
/// - REMOVED items: present in dir1 but not dir2
/// - CHANGED items: present in both but content_hash differs
/// - Stage outcome differences for items in both
///
/// Output format:
/// ```text
/// Comparing: ./output/run-001 vs ./output/run-002
///
/// NEW (3 items):
///   + engineering-drive/New Document.docx
///   + engineering-drive/Another Doc.pdf
///   + team-slack/Thread: New topic
///
/// REMOVED (1 item):
///   - engineering-drive/Old Archive.docx
///
/// CHANGED (2 items):
///   ~ engineering-drive/Q1 Review.docx  (hash: a7f3b2 -> c8d4e1)
///   ~ team-slack/Thread: Updated plan   (hash: b3d7f1 -> f2a9c3)
///
/// STAGE DIFFERENCES:
///   engineering-drive/Q1 Review.docx:
///     normalize-gdrive: Completed -> Failed (PDF conversion error)
/// ```
///
/// Returns exit code: always 0 (diff is informational).
pub async fn handle_diff(dir1: &Path, dir2: &Path) -> Result<i32> {
    if !dir1.exists() {
        return Err(CliError::OutputDirNotFound {
            path: dir1.display().to_string(),
        });
    }
    if !dir2.exists() {
        return Err(CliError::OutputDirNotFound {
            path: dir2.display().to_string(),
        });
    }

    let state1 = output::load_state(dir1).await?;
    let state2 = output::load_state(dir2).await?;

    println!(
        "Comparing: {} vs {}",
        dir1.display(),
        dir2.display()
    );
    println!();

    print_diff(&state1, &state2);

    Ok(0)
}

/// Collect all item keys (source_name/item_id) from a pipeline state.
fn collect_item_keys(state: &PipelineState) -> BTreeSet<(String, String)> {
    let mut keys = BTreeSet::new();
    for (source_name, source_state) in &state.sources {
        for item_id in source_state.items.keys() {
            keys.insert((source_name.clone(), item_id.clone()));
        }
    }
    keys
}

/// Print the diff between two pipeline states.
fn print_diff(state1: &PipelineState, state2: &PipelineState) {
    let keys1 = collect_item_keys(state1);
    let keys2 = collect_item_keys(state2);

    // NEW: in state2 but not state1
    let new_items: Vec<_> = keys2.difference(&keys1).collect();
    if !new_items.is_empty() {
        println!("NEW ({} item{}):", new_items.len(), if new_items.len() == 1 { "" } else { "s" });
        for (source, item_id) in &new_items {
            let display = state2.sources.get(source.as_str())
                .and_then(|s| s.items.get(item_id.as_str()))
                .map(|i| i.display_name.as_str())
                .unwrap_or(item_id.as_str());
            println!("  + {}/{}", source, display);
        }
        println!();
    }

    // REMOVED: in state1 but not state2
    let removed_items: Vec<_> = keys1.difference(&keys2).collect();
    if !removed_items.is_empty() {
        println!("REMOVED ({} item{}):", removed_items.len(), if removed_items.len() == 1 { "" } else { "s" });
        for (source, item_id) in &removed_items {
            let display = state1.sources.get(source.as_str())
                .and_then(|s| s.items.get(item_id.as_str()))
                .map(|i| i.display_name.as_str())
                .unwrap_or(item_id.as_str());
            println!("  - {}/{}", source, display);
        }
        println!();
    }

    // CHANGED: in both but hash differs
    let common: Vec<_> = keys1.intersection(&keys2).collect();
    let mut changed = Vec::new();
    let mut stage_diffs = Vec::new();

    for (source, item_id) in &common {
        let item1 = state1.sources.get(source.as_str())
            .and_then(|s| s.items.get(item_id.as_str()));
        let item2 = state2.sources.get(source.as_str())
            .and_then(|s| s.items.get(item_id.as_str()));

        if let (Some(i1), Some(i2)) = (item1, item2) {
            if i1.content_hash != i2.content_hash {
                changed.push((source, &i1.display_name, &i1.content_hash, &i2.content_hash));
            }
            // Check for stage outcome differences (status changed)
            let status1 = format!("{:?}", i1.status);
            let status2 = format!("{:?}", i2.status);
            if status1 != status2 {
                stage_diffs.push((source, &i1.display_name, status1, status2));
            }
        }
    }

    if !changed.is_empty() {
        println!("CHANGED ({} item{}):", changed.len(), if changed.len() == 1 { "" } else { "s" });
        for (source, display, hash1, hash2) in &changed {
            let h1 = truncate_hash(hash1.as_str(), 6);
            let h2 = truncate_hash(hash2.as_str(), 6);
            println!("  ~ {}/{}  (hash: {} -> {})", source, display, h1, h2);
        }
        println!();
    }

    if !stage_diffs.is_empty() {
        println!("STAGE DIFFERENCES:");
        for (source, display, s1, s2) in &stage_diffs {
            println!("  {}/{}:", source, display);
            println!("    {} -> {}", s1, s2);
        }
        println!();
    }

    if new_items.is_empty() && removed_items.is_empty() && changed.is_empty() && stage_diffs.is_empty() {
        println!("No differences found.");
    }
}

/// Truncate a hash string for display.
fn truncate_hash(hash: &str, len: usize) -> &str {
    if hash.len() > len {
        &hash[..len]
    } else {
        hash
    }
}
```

### `src/pipeline/output.rs`

```rust
//! Shared output formatting helpers for pipeline CLI commands.

use std::path::Path;

use ecl_pipeline_state::{
    ItemStatus, PipelineState, PipelineStatus, RedbStateStore, StageStatus,
};

use crate::error::{CliError, Result};

/// Load the pipeline state from a redb database in the given output directory.
///
/// Opens the `pipeline.redb` file, loads the latest checkpoint, and returns
/// the embedded `PipelineState`.
pub async fn load_state(output_dir: &Path) -> Result<PipelineState> {
    let db_path = output_dir.join("pipeline.redb");
    let store = RedbStateStore::new(&db_path)?;
    let checkpoint = store
        .load_checkpoint()
        .await?
        .ok_or_else(|| CliError::NoCheckpoint {
            path: output_dir.display().to_string(),
        })?;
    Ok(checkpoint.state)
}

/// Determine the exit code from the final pipeline state.
///
/// - 0: all items completed successfully
/// - 1: pipeline failed (error status)
/// - 2: pipeline completed but some items failed (partial success)
pub fn exit_code_from_state(state: &PipelineState) -> i32 {
    match &state.status {
        PipelineStatus::Completed { .. } => {
            if state.stats.total_items_failed > 0 {
                2 // Partial: completed with failures
            } else {
                0 // Full success
            }
        }
        PipelineStatus::Failed { .. } => 1,
        _ => 1, // Interrupted or other non-terminal states
    }
}

/// Print a run summary after pipeline execution completes.
///
/// Output format:
/// ```text
/// Pipeline 'q1-knowledge-sync' completed.
///   Items processed:  147
///   Items skipped:    422 (unchanged)
///   Items failed:       2
///   Total discovered: 1047
/// ```
pub fn print_run_summary(state: &PipelineState) {
    let status_str = match &state.status {
        PipelineStatus::Completed { .. } => "completed",
        PipelineStatus::Failed { error, .. } => {
            println!("Pipeline '{}' failed: {}", state.pipeline_name, error);
            return;
        }
        PipelineStatus::Interrupted { .. } => "interrupted",
        PipelineStatus::Running { .. } => "still running (unexpected)",
        PipelineStatus::Pending => "pending (unexpected)",
    };

    println!("Pipeline '{}' {}.", state.pipeline_name, status_str);
    println!("  Items processed:  {}", state.stats.total_items_processed);
    println!(
        "  Items skipped:    {} (unchanged)",
        state.stats.total_items_skipped_unchanged
    );
    println!("  Items failed:     {}", state.stats.total_items_failed);
    println!(
        "  Total discovered: {}",
        state.stats.total_items_discovered
    );
}

/// Print a human-readable status table.
///
/// Shows pipeline header info and one row per stage.
pub fn print_status_table(state: &PipelineState) {
    // Header
    println!("Pipeline: {}", state.pipeline_name);
    let status_desc = match &state.status {
        PipelineStatus::Pending => "Pending".to_string(),
        PipelineStatus::Running { current_stage } => {
            format!("Running (batch {}, stage: {})", state.current_batch, current_stage)
        }
        PipelineStatus::Completed { finished_at } => {
            format!("Completed ({})", finished_at.format("%Y-%m-%d %H:%M:%S UTC"))
        }
        PipelineStatus::Failed { error, .. } => format!("Failed: {}", error),
        PipelineStatus::Interrupted { interrupted_at } => {
            format!("Interrupted ({})", interrupted_at.format("%Y-%m-%d %H:%M:%S UTC"))
        }
    };
    println!("Status:   {}", status_desc);
    println!("Started:  {}", state.started_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();

    // Stage table header
    println!(
        "{:<24} {:<12} {:>10} {:>8} {:>8} {:>10}",
        "STAGE", "STATUS", "PROCESSED", "FAILED", "SKIPPED", "DURATION"
    );

    // Stage rows
    for (stage_id, stage_state) in &state.stages {
        let status_str = match &stage_state.status {
            StageStatus::Pending => "Pending",
            StageStatus::Running => "Running",
            StageStatus::Completed => "Completed",
            StageStatus::Skipped { .. } => "Skipped",
            StageStatus::Failed { .. } => "Failed",
        };

        let duration_str = match (stage_state.started_at, stage_state.completed_at) {
            (Some(start), Some(end)) => {
                let dur = end - start;
                format!("{:.1}s", dur.num_milliseconds() as f64 / 1000.0)
            }
            (Some(start), None) => {
                let dur = chrono::Utc::now() - start;
                format!("{:.1}s*", dur.num_milliseconds() as f64 / 1000.0)
            }
            _ => "--".to_string(),
        };

        println!(
            "{:<24} {:<12} {:>10} {:>8} {:>8} {:>10}",
            stage_id,
            status_str,
            stage_state.items_processed,
            stage_state.items_failed,
            stage_state.items_skipped,
            duration_str,
        );
    }

    // Summary footer
    println!();
    println!(
        "Items: {} processed, {} skipped (unchanged), {} failed, {} discovered",
        state.stats.total_items_processed,
        state.stats.total_items_skipped_unchanged,
        state.stats.total_items_failed,
        state.stats.total_items_discovered,
    );
}

/// Print an items table listing all items across all sources.
pub fn print_items_table(state: &PipelineState) {
    println!(
        "{:<20} {:<34} {:<12} {:<12} {}",
        "SOURCE", "ITEM", "STATUS", "HASH", "STAGES_COMPLETED"
    );

    for (source_name, source_state) in &state.sources {
        for (_item_id, item) in &source_state.items {
            let status_str = match &item.status {
                ItemStatus::Pending => "Pending".to_string(),
                ItemStatus::Processing { stage } => format!("Processing({})", stage),
                ItemStatus::Completed => "Completed".to_string(),
                ItemStatus::Failed { stage, .. } => format!("Failed({})", stage),
                ItemStatus::Skipped { stage, .. } => format!("Skipped({})", stage),
                ItemStatus::Unchanged => "Unchanged".to_string(),
            };

            let hash_str = if item.content_hash.is_empty() {
                "--".to_string()
            } else {
                let h = item.content_hash.as_str();
                if h.len() > 10 {
                    format!("{}...", &h[..10])
                } else {
                    h.to_string()
                }
            };

            let stages_str = if item.completed_stages.is_empty() {
                "--".to_string()
            } else {
                item.completed_stages
                    .iter()
                    .map(|s| s.stage.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            // Truncate display name for table
            let display = if item.display_name.len() > 32 {
                format!("{}...", &item.display_name[..29])
            } else {
                item.display_name.clone()
            };

            println!(
                "{:<20} {:<34} {:<12} {:<12} {}",
                source_name, display, status_str, hash_str, stages_str,
            );
        }
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold module structure and verify compilation

- [ ] Create `crates/ecl-cli/src/error.rs` with `CliError` enum (from section 6)
- [ ] Create `crates/ecl-cli/src/pipeline/mod.rs` with `PipelineCmd` and `PipelineSubcommand` enums (from section 6), and a stub `dispatch` function that returns `Ok(0)` for all subcommands
- [ ] Create stub files: `src/pipeline/run.rs`, `src/pipeline/resume.rs`, `src/pipeline/status.rs`, `src/pipeline/inspect.rs`, `src/pipeline/items.rs`, `src/pipeline/diff.rs`, `src/pipeline/output.rs` -- each with a stub handler that returns `Ok(0)`
- [ ] Modify `crates/ecl-cli/Cargo.toml` -- add pipeline crate dependencies (from section 5)
- [ ] Modify `crates/ecl-cli/src/main.rs` -- restructure to use `Cli` with subcommands (from section 6)
- [ ] Run `cargo check -p ecl-cli` -- must pass
- [ ] Commit: `feat(ecl-cli): scaffold pipeline subcommand structure`

### Step 2: Error types

- [ ] Implement full `CliError` enum in `src/error.rs` (from section 6)
- [ ] Write tests: `test_cli_error_display_file_read`, `test_cli_error_display_no_checkpoint`, `test_cli_error_display_output_dir_not_found`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): add CliError types`

### Step 3: Output helpers

- [ ] Implement `src/pipeline/output.rs` with `load_state()`, `exit_code_from_state()`, `print_run_summary()`, `print_status_table()`, `print_items_table()` (from section 6)
- [ ] Write tests: `test_exit_code_completed_no_failures_returns_0`, `test_exit_code_completed_with_failures_returns_2`, `test_exit_code_failed_returns_1`, `test_exit_code_interrupted_returns_1`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): add output formatting helpers`

### Step 4: Inspect command

- [ ] Implement `src/pipeline/inspect.rs` with `handle_inspect()` (from section 6)
- [ ] Write tests: `test_handle_inspect_nonexistent_dir_returns_error`, `test_handle_inspect_prints_valid_json` (using `InMemoryStateStore` with fixture data)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline inspect command`

### Step 5: Status command

- [ ] Implement `src/pipeline/status.rs` with `handle_status()` (from section 6)
- [ ] Write tests: `test_handle_status_nonexistent_dir_returns_error`, `test_handle_status_prints_stage_table` (capture stdout, verify format)
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline status command`

### Step 6: Items command

- [ ] Implement `src/pipeline/items.rs` with `handle_items()` (from section 6)
- [ ] Write tests: `test_handle_items_nonexistent_dir_returns_error`, `test_handle_items_prints_item_rows`, `test_handle_items_unchanged_items_show_dash_hash`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline items command`

### Step 7: Diff command

- [ ] Implement `src/pipeline/diff.rs` with `handle_diff()`, `collect_item_keys()`, `print_diff()`, `truncate_hash()` (from section 6)
- [ ] Write tests: `test_handle_diff_nonexistent_dir1_returns_error`, `test_handle_diff_nonexistent_dir2_returns_error`, `test_collect_item_keys_returns_all_source_item_pairs`, `test_print_diff_new_items`, `test_print_diff_removed_items`, `test_print_diff_changed_items`, `test_print_diff_no_differences`, `test_truncate_hash_short_input`, `test_truncate_hash_exact_length`, `test_truncate_hash_long_input`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline diff command`

### Step 8: Run command

- [ ] Implement `src/pipeline/run.rs` with `handle_run()` and `build_topology()` (from section 6)
- [ ] Also add `build_topology_from_checkpoint()` for use by resume
- [ ] Write tests: `test_handle_run_missing_config_returns_file_read_error`, `test_handle_run_invalid_toml_returns_spec_error`, `test_build_topology_registers_filesystem_adapter`, `test_build_topology_skips_unknown_adapters`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline run command`

### Step 9: Resume command

- [ ] Implement `src/pipeline/resume.rs` with `handle_resume()` (from section 6)
- [ ] Write tests: `test_handle_resume_nonexistent_dir_returns_error`, `test_handle_resume_no_checkpoint_returns_error`, `test_handle_resume_prints_sequence_header`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-cli): implement pipeline resume command`

### Step 10: Integration and final polish

- [ ] Write integration test: `test_cli_pipeline_run_help_output` -- verify `ecl pipeline run --help` produces expected usage text
- [ ] Write integration test: `test_cli_pipeline_subcommands_listed` -- verify `ecl pipeline --help` lists all 6 subcommands
- [ ] Run `make test`, `make lint`, `make format`
- [ ] Verify all public items have doc comments
- [ ] Verify no compiler warnings
- [ ] Commit: `feat(ecl-cli): add integration tests and polish`

## 8. Test Fixtures

### Fixture: Minimal PipelineState (for status/inspect/items tests)

```rust
use chrono::Utc;
use std::collections::BTreeMap;
use ecl_pipeline_state::{
    Blake3Hash, CompletedStageRecord, ItemProvenance, ItemState, ItemStatus,
    PipelineState, PipelineStats, PipelineStatus, RunId, SourceState,
    StageId, StageState, StageStatus,
};

fn fixture_pipeline_state() -> PipelineState {
    let now = Utc::now();

    let mut sources = BTreeMap::new();
    let mut items = BTreeMap::new();

    items.insert(
        "item-001".to_string(),
        ItemState {
            display_name: "Test Document.docx".to_string(),
            source_id: "item-001".to_string(),
            source_name: "local-fs".to_string(),
            content_hash: Blake3Hash::new("a7f3b2c9e1d4f5a6b8c0d2e4f6a8b0c2d4e6f8a0"),
            status: ItemStatus::Completed,
            completed_stages: vec![
                CompletedStageRecord {
                    stage: StageId::new("extract"),
                    completed_at: now,
                    duration_ms: 120,
                },
                CompletedStageRecord {
                    stage: StageId::new("emit"),
                    completed_at: now,
                    duration_ms: 45,
                },
            ],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: Some(now),
                extracted_at: now,
            },
        },
    );

    items.insert(
        "item-002".to_string(),
        ItemState {
            display_name: "Failed Doc.pdf".to_string(),
            source_id: "item-002".to_string(),
            source_name: "local-fs".to_string(),
            content_hash: Blake3Hash::new("c9e1a4b3d7f2e5a8c0b2d4f6a8e0c2d4f6b8a0c2"),
            status: ItemStatus::Failed {
                stage: "extract".to_string(),
                error: "file not readable".to_string(),
                attempts: 3,
            },
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: Some(now),
                extracted_at: now,
            },
        },
    );

    items.insert(
        "item-003".to_string(),
        ItemState {
            display_name: "Unchanged File.txt".to_string(),
            source_id: "item-003".to_string(),
            source_name: "local-fs".to_string(),
            content_hash: Blake3Hash::new(""),
            status: ItemStatus::Unchanged,
            completed_stages: vec![],
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: Some(now),
                extracted_at: now,
            },
        },
    );

    sources.insert(
        "local-fs".to_string(),
        SourceState {
            items_discovered: 10,
            items_accepted: 5,
            items_skipped_unchanged: 3,
            items,
        },
    );

    let mut stages = BTreeMap::new();
    stages.insert(
        StageId::new("extract"),
        StageState {
            status: StageStatus::Completed,
            items_processed: 2,
            items_failed: 1,
            items_skipped: 0,
            started_at: Some(now),
            completed_at: Some(now),
        },
    );
    stages.insert(
        StageId::new("emit"),
        StageState {
            status: StageStatus::Completed,
            items_processed: 1,
            items_failed: 0,
            items_skipped: 0,
            started_at: Some(now),
            completed_at: Some(now),
        },
    );

    PipelineState {
        run_id: RunId::new("run-test-001"),
        pipeline_name: "test-pipeline".to_string(),
        started_at: now,
        last_checkpoint: now,
        status: PipelineStatus::Completed { finished_at: now },
        current_batch: 0,
        sources,
        stages,
        stats: PipelineStats {
            total_items_discovered: 10,
            total_items_processed: 2,
            total_items_skipped_unchanged: 3,
            total_items_failed: 1,
        },
    }
}
```

### Fixture: Second PipelineState (for diff tests)

```rust
fn fixture_pipeline_state_v2() -> PipelineState {
    let mut state = fixture_pipeline_state();
    state.run_id = RunId::new("run-test-002");

    // Modify item-001's hash (simulates changed content)
    if let Some(source) = state.sources.get_mut("local-fs") {
        if let Some(item) = source.items.get_mut("item-001") {
            item.content_hash = Blake3Hash::new("different_hash_value_for_comparison");
        }

        // Remove item-002 (simulates removal)
        source.items.remove("item-002");

        // Add a new item (simulates addition)
        let now = chrono::Utc::now();
        source.items.insert(
            "item-004".to_string(),
            ItemState {
                display_name: "Brand New File.md".to_string(),
                source_id: "item-004".to_string(),
                source_name: "local-fs".to_string(),
                content_hash: Blake3Hash::new("newfilehash123456"),
                status: ItemStatus::Completed,
                completed_stages: vec![],
                provenance: ItemProvenance {
                    source_kind: "filesystem".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: Some(now),
                    extracted_at: now,
                },
            },
        );
    }

    state.stats.total_items_failed = 0;
    state
}
```

### Fixture: Minimal TOML config (for run tests)

```toml
name = "test-pipeline"
version = 1
output_dir = "./output/test"

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw-docs"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-docs"] }
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_cli_error_display_file_read` | `error` | `CliError::FileRead` includes path in message |
| `test_cli_error_display_no_checkpoint` | `error` | `CliError::NoCheckpoint` includes dir path |
| `test_cli_error_display_output_dir_not_found` | `error` | `CliError::OutputDirNotFound` includes path |
| `test_exit_code_completed_no_failures_returns_0` | `output` | Completed state with 0 failed -> exit code 0 |
| `test_exit_code_completed_with_failures_returns_2` | `output` | Completed state with >0 failed -> exit code 2 |
| `test_exit_code_failed_returns_1` | `output` | Failed state -> exit code 1 |
| `test_exit_code_interrupted_returns_1` | `output` | Interrupted state -> exit code 1 |
| `test_handle_inspect_nonexistent_dir_returns_error` | `inspect` | Non-existent dir -> `OutputDirNotFound` error |
| `test_handle_status_nonexistent_dir_returns_error` | `status` | Non-existent dir -> `OutputDirNotFound` error |
| `test_handle_items_nonexistent_dir_returns_error` | `items` | Non-existent dir -> `OutputDirNotFound` error |
| `test_handle_items_unchanged_items_show_dash_hash` | `items` | Unchanged items print "--" for hash |
| `test_handle_diff_nonexistent_dir1_returns_error` | `diff` | Non-existent dir1 -> `OutputDirNotFound` error |
| `test_handle_diff_nonexistent_dir2_returns_error` | `diff` | Non-existent dir2 -> `OutputDirNotFound` error |
| `test_collect_item_keys_returns_all_source_item_pairs` | `diff` | All (source, item_id) pairs collected |
| `test_print_diff_new_items` | `diff` | Items in state2 but not state1 printed as NEW |
| `test_print_diff_removed_items` | `diff` | Items in state1 but not state2 printed as REMOVED |
| `test_print_diff_changed_items` | `diff` | Items with different hashes printed as CHANGED |
| `test_print_diff_no_differences` | `diff` | Identical states print "No differences found." |
| `test_truncate_hash_short_input` | `diff` | Hash shorter than len returns full hash |
| `test_truncate_hash_exact_length` | `diff` | Hash exactly len returns full hash |
| `test_truncate_hash_long_input` | `diff` | Hash longer than len returns truncated |
| `test_handle_run_missing_config_returns_file_read_error` | `run` | Non-existent TOML file -> `FileRead` error |
| `test_handle_run_invalid_toml_returns_spec_error` | `run` | Invalid TOML content -> `Spec` error |
| `test_build_topology_registers_filesystem_adapter` | `run` | Filesystem source creates adapter in topology |
| `test_build_topology_skips_unknown_adapters` | `run` | Unknown adapter name logs warning, does not error |
| `test_handle_resume_nonexistent_dir_returns_error` | `resume` | Non-existent dir -> `OutputDirNotFound` error |
| `test_handle_resume_no_checkpoint_returns_error` | `resume` | Empty redb -> `NoCheckpoint` error |
| `test_handle_resume_prints_sequence_header` | `resume` | Output includes "Resuming...checkpoint (sequence N)" |
| `test_cli_pipeline_run_help_output` | integration | `ecl pipeline run --help` prints usage |
| `test_cli_pipeline_subcommands_listed` | integration | `ecl pipeline --help` lists run, resume, status, inspect, items, diff |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-cli` passes
- [ ] `cargo test -p ecl-cli` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] Exit codes verified: 0 for success, 1 for failure, 2 for partial
- [ ] `ecl pipeline --help` lists all 6 subcommands
- [ ] `ecl pipeline run --help` shows config argument
- [ ] `ecl pipeline resume --help` shows output-dir argument and --force flag
- [ ] `ecl pipeline inspect` output is valid JSON parseable by `jq`

### What NOT to Do

- Do NOT modify the pipeline runner internals (`ecl-pipeline` crate)
- Do NOT add new adapters or stages
- Do NOT implement web UI or dashboard
- Do NOT add new crates -- only extend `ecl-cli`
- Do NOT implement adapter authentication (that is handled by adapter crates)
- Do NOT change the `PipelineRunner` API or `StateStore` trait
- Do NOT add non-pipeline CLI commands in this milestone
