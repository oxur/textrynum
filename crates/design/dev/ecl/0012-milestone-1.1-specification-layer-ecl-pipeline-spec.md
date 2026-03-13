# Milestone 1.1: Specification Layer (`ecl-pipeline-spec`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline-spec):`

## 1. Goal

Create the `ecl-pipeline-spec` crate containing all TOML-driven pipeline
configuration types. When done, the full example TOML from section 8 below
parses into Rust types and round-trips through JSON without data loss.

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

- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, SpecError>;`
- **Test naming:** `test_<fn>_<scenario>_<expectation>`
- **Tests:** Inline `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { ... }`
- **Maps:** Always `BTreeMap` (not `HashMap`) for deterministic serialization
- **Params:** Use `&str` not `&String`, `&[T]` not `&Vec<T>` in function signatures
- **No `unwrap()`** in library code — use `?` or `.ok_or()`
- **Doc comments** on all public items

### 2.2 Rust Guides to Load

Before writing any code, read these files (paths relative to workspace root):

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` (ALWAYS first)
2. `assets/ai/ai-rust/guides/01-core-idioms.md`
3. `assets/ai/ai-rust/guides/05-type-design.md`
4. `assets/ai/ai-rust/guides/03-error-handling.md`
5. `assets/ai/ai-rust/guides/12-project-structure.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments

## 3. Prior Art / Dependencies

**None.** This is the first crate in the pipeline runner. It has no internal
dependencies — only workspace-level external crates.

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add workspace deps `redb`, `serde_bytes`; add `crates/ecl-pipeline-spec` to members |
| `crates/ecl-pipeline-spec/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-pipeline-spec/src/lib.rs` | Create | `PipelineSpec`, module declarations, re-exports |
| `crates/ecl-pipeline-spec/src/source.rs` | Create | `SourceSpec`, `GoogleDriveSourceSpec`, `SlackSourceSpec`, `FilesystemSourceSpec`, `CredentialRef`, `FilterRule`, `FilterAction`, `FileTypeFilter` |
| `crates/ecl-pipeline-spec/src/stage.rs` | Create | `StageSpec`, `ResourceSpec` |
| `crates/ecl-pipeline-spec/src/defaults.rs` | Create | `DefaultsSpec`, `RetrySpec`, `CheckpointStrategy`, default functions, `Default` impls |
| `crates/ecl-pipeline-spec/src/validation.rs` | Create | `validate()` function, validation logic |
| `crates/ecl-pipeline-spec/src/error.rs` | Create | `SpecError` enum |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml`:

```toml
redb = "3"
serde_bytes = "0.11"
```

Add to `[workspace] members`:

```toml
"crates/ecl-pipeline-spec",
```

### Crate Cargo.toml

```toml
[package]
name = "ecl-pipeline-spec"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Pipeline specification types for ECL pipeline runner"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
toml = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
tempfile = { workspace = true }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown. These are copied from
the authoritative design document.

### `src/lib.rs`

```rust
//! Pipeline specification types for the ECL pipeline runner.
//!
//! This crate defines the TOML-driven configuration layer. All types are
//! immutable after parsing and derive `Serialize + Deserialize` for
//! embedding in checkpoints.

pub mod defaults;
pub mod error;
pub mod source;
pub mod stage;
pub mod validation;

pub use defaults::{CheckpointStrategy, DefaultsSpec, RetrySpec};
pub use error::{Result, SpecError};
pub use source::{
    CredentialRef, FileTypeFilter, FilesystemSourceSpec, FilterAction, FilterRule,
    GoogleDriveSourceSpec, SlackSourceSpec, SourceSpec,
};
pub use stage::{ResourceSpec, StageSpec};

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

impl PipelineSpec {
    /// Parse a PipelineSpec from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let spec: Self = toml::from_str(toml_str).map_err(|e| SpecError::ParseError {
            message: e.to_string(),
        })?;
        spec.validate()?;
        Ok(spec)
    }

    /// Validate the spec (delegates to validation module).
    pub fn validate(&self) -> Result<()> {
        validation::validate(self)
    }
}
```

### `src/defaults.rs`

```rust
//! Global default configuration for the pipeline.

use serde::{Deserialize, Serialize};

/// Global defaults that apply across all sources/stages.
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

impl Default for DefaultsSpec {
    fn default() -> Self {
        Self {
            concurrency: default_concurrency(),
            retry: RetrySpec::default(),
            checkpoint: CheckpointStrategy::default(),
        }
    }
}

/// Retry policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    /// Total attempts (1 = no retry).
    #[serde(default = "default_max_attempts")]
    pub max_attempts: u32,

    /// Initial backoff duration in milliseconds.
    #[serde(default = "default_initial_backoff")]
    pub initial_backoff_ms: u64,

    /// Multiplier applied to backoff after each attempt.
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

/// When to write checkpoints during pipeline execution.
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

### `src/source.rs`

```rust
//! Source specification types for external data services.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A source is "where does the data come from?"
/// The `kind` field determines which SourceAdapter handles it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    /// Google Drive folder source.
    #[serde(rename = "google_drive")]
    GoogleDrive(GoogleDriveSourceSpec),

    /// Slack workspace source.
    #[serde(rename = "slack")]
    Slack(SlackSourceSpec),

    /// Local filesystem source.
    #[serde(rename = "filesystem")]
    Filesystem(FilesystemSourceSpec),
}

/// Google Drive source configuration.
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
/// `#[serde(untagged)]` because untagged enums are fragile with TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    /// Credentials from a file path.
    #[serde(rename = "file")]
    File {
        /// Path to credentials file.
        path: PathBuf,
    },
    /// Credentials from an environment variable.
    #[serde(rename = "env")]
    EnvVar {
        /// Environment variable name.
        env: String,
    },
    /// Use application default credentials.
    #[serde(rename = "application_default")]
    ApplicationDefault,
}

/// A filter rule for include/exclude patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Glob pattern matched against the full path.
    pub pattern: String,
    /// Whether this rule includes or excludes matches.
    pub action: FilterAction,
}

/// Whether a filter rule includes or excludes matches.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction {
    /// Include items matching the pattern.
    Include,
    /// Exclude items matching the pattern.
    Exclude,
}

/// Filter by file type (extension or MIME type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeFilter {
    /// File extension to match (e.g., "pdf").
    pub extension: Option<String>,
    /// MIME type to match.
    pub mime: Option<String>,
}
```

### `src/stage.rs`

```rust
//! Stage specification types.

use crate::defaults::RetrySpec;
use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub params: serde_json::Value,

    /// Override the default retry policy for this stage.
    pub retry: Option<RetrySpec>,

    /// Override the default timeout for this stage.
    pub timeout_secs: Option<u64>,

    /// If true, item-level failures skip the item rather than failing
    /// the pipeline.
    #[serde(default)]
    pub skip_on_error: bool,

    /// Optional predicate expression. When false, the stage is skipped.
    pub condition: Option<String>,
}

/// Resource access declarations in TOML-friendly form.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// Resources this stage reads (shared access).
    #[serde(default)]
    pub reads: Vec<String>,
    /// Resources this stage creates (produces for the first time).
    #[serde(default)]
    pub creates: Vec<String>,
    /// Resources this stage writes (exclusive access).
    #[serde(default)]
    pub writes: Vec<String>,
}
```

### `src/error.rs`

```rust
//! Error types for the specification layer.

use thiserror::Error;

/// Errors that can occur when parsing or validating a pipeline specification.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SpecError {
    /// TOML parsing failed.
    #[error("failed to parse pipeline spec: {message}")]
    ParseError {
        /// The parse error message.
        message: String,
    },

    /// A stage references a source that doesn't exist.
    #[error("stage '{stage}' references unknown source '{source}'")]
    UnknownSource {
        /// The stage that references the unknown source.
        stage: String,
        /// The source name that was referenced.
        source: String,
    },

    /// Duplicate stage name detected.
    #[error("duplicate stage name: '{name}'")]
    DuplicateStage {
        /// The duplicate stage name.
        name: String,
    },

    /// Pipeline has no stages defined.
    #[error("pipeline has no stages defined")]
    EmptyPipeline,

    /// Pipeline has no sources defined.
    #[error("pipeline has no sources defined")]
    EmptySources,

    /// Validation error with a custom message.
    #[error("validation error: {message}")]
    ValidationError {
        /// Description of the validation failure.
        message: String,
    },
}

/// Result type for specification operations.
pub type Result<T> = std::result::Result<T, SpecError>;
```

### `src/validation.rs`

```rust
//! Validation logic for pipeline specifications.

use crate::{PipelineSpec, SourceSpec};
use crate::error::{Result, SpecError};

/// Validate a pipeline specification.
///
/// Checks:
/// - Pipeline has at least one source
/// - Pipeline has at least one stage
/// - Every stage with a `source` field references an existing source
pub fn validate(spec: &PipelineSpec) -> Result<()> {
    if spec.sources.is_empty() {
        return Err(SpecError::EmptySources);
    }

    if spec.stages.is_empty() {
        return Err(SpecError::EmptyPipeline);
    }

    // Validate stage source references.
    for (stage_name, stage_spec) in &spec.stages {
        if let Some(ref source_name) = stage_spec.source {
            if !spec.sources.contains_key(source_name) {
                return Err(SpecError::UnknownSource {
                    stage: stage_name.clone(),
                    source: source_name.clone(),
                });
            }
        }
    }

    Ok(())
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-pipeline-spec/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Pipeline spec crate.`
- [ ] Add `redb = "3"` and `serde_bytes = "0.11"` to root `Cargo.toml` `[workspace.dependencies]`
- [ ] Add `"crates/ecl-pipeline-spec"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-pipeline-spec` — must pass
- [ ] Commit: `feat(ecl-pipeline-spec): scaffold crate`

### Step 2: Error types

- [ ] Create `src/error.rs` with `SpecError` enum (from section 6)
- [ ] Add `pub mod error;` and `pub use error::{Result, SpecError};` to `lib.rs`
- [ ] Write tests: `test_error_display_parse_error`, `test_error_display_unknown_source`, `test_error_implements_send_sync`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add SpecError types`

### Step 3: Defaults types

- [ ] Create `src/defaults.rs` with `DefaultsSpec`, `RetrySpec`, `CheckpointStrategy` (from section 6)
- [ ] Add module declaration and re-exports to `lib.rs`
- [ ] Write tests: `test_retry_spec_default_values`, `test_defaults_spec_default_values`, `test_checkpoint_strategy_default_is_batch`, `test_retry_spec_serde_roundtrip`, `test_checkpoint_strategy_serde_roundtrip_all_variants`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add defaults types`

### Step 4: Source types

- [ ] Create `src/source.rs` with all source types (from section 6)
- [ ] Add module declaration and re-exports to `lib.rs`
- [ ] Write tests: `test_source_spec_google_drive_serde_roundtrip`, `test_source_spec_slack_serde_roundtrip`, `test_source_spec_filesystem_serde_roundtrip`, `test_credential_ref_file_serde`, `test_credential_ref_env_serde`, `test_credential_ref_application_default_serde`, `test_filter_rule_serde`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add source types`

### Step 5: Stage types

- [ ] Create `src/stage.rs` with `StageSpec`, `ResourceSpec` (from section 6)
- [ ] Add module declaration and re-exports to `lib.rs`
- [ ] Write tests: `test_stage_spec_serde_roundtrip`, `test_resource_spec_default_is_empty`, `test_stage_spec_params_json_value`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add stage types`

### Step 6: PipelineSpec and TOML parsing

- [ ] Add `PipelineSpec` struct and `from_toml()` to `lib.rs` (from section 6)
- [ ] Write test: `test_pipeline_spec_from_example_toml` — parse the full example TOML from section 8
- [ ] Write test: `test_pipeline_spec_roundtrip_toml_json` — parse TOML, serialize to JSON, deserialize from JSON, compare
- [ ] Write test: `test_pipeline_spec_from_toml_invalid` — garbage input returns error
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add PipelineSpec with TOML parsing`

### Step 7: Validation

- [ ] Create `src/validation.rs` with `validate()` function (from section 6)
- [ ] Add module declaration to `lib.rs`
- [ ] Write tests: `test_validate_empty_sources_fails`, `test_validate_empty_stages_fails`, `test_validate_unknown_source_reference_fails`, `test_validate_valid_spec_passes`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-pipeline-spec): add spec validation`

### Step 8: Property tests and final polish

- [ ] Add proptest for `RetrySpec` — random values serialize/deserialize correctly
- [ ] Add proptest for `DefaultsSpec` — random concurrency values survive roundtrip
- [ ] Run `make test`, `make lint`, `make format`
- [ ] Verify all public items have doc comments
- [ ] Commit: `feat(ecl-pipeline-spec): add property tests`

## 8. Test Fixtures

### Full Example TOML (use in round-trip tests)

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

### Minimal Valid TOML (for simple tests)

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
| `test_error_display_parse_error` | `error` | `SpecError::ParseError` Display output |
| `test_error_display_unknown_source` | `error` | `SpecError::UnknownSource` includes stage and source names |
| `test_error_implements_send_sync` | `error` | `SpecError: Send + Sync` (required for async) |
| `test_retry_spec_default_values` | `defaults` | Default RetrySpec has max_attempts=3, initial_backoff_ms=1000, multiplier=2.0, max=30000 |
| `test_defaults_spec_default_values` | `defaults` | Default DefaultsSpec has concurrency=4 |
| `test_checkpoint_strategy_default_is_batch` | `defaults` | Default CheckpointStrategy is Batch |
| `test_retry_spec_serde_roundtrip` | `defaults` | Serialize RetrySpec to JSON and back, assert equal |
| `test_checkpoint_strategy_serde_roundtrip_all_variants` | `defaults` | All 3 CheckpointStrategy variants survive JSON roundtrip |
| `test_source_spec_google_drive_serde_roundtrip` | `source` | GoogleDrive variant with all fields round-trips through JSON |
| `test_source_spec_slack_serde_roundtrip` | `source` | Slack variant round-trips |
| `test_source_spec_filesystem_serde_roundtrip` | `source` | Filesystem variant round-trips |
| `test_credential_ref_file_serde` | `source` | `CredentialRef::File` tagged correctly in JSON |
| `test_credential_ref_env_serde` | `source` | `CredentialRef::EnvVar` tagged correctly |
| `test_credential_ref_application_default_serde` | `source` | `CredentialRef::ApplicationDefault` tagged correctly |
| `test_filter_rule_serde` | `source` | FilterRule with Include and Exclude actions |
| `test_stage_spec_serde_roundtrip` | `stage` | StageSpec with all fields round-trips |
| `test_resource_spec_default_is_empty` | `stage` | Default ResourceSpec has empty vecs |
| `test_stage_spec_params_json_value` | `stage` | params field accepts arbitrary JSON objects |
| `test_pipeline_spec_from_example_toml` | `lib` | Full example TOML parses without error |
| `test_pipeline_spec_roundtrip_toml_json` | `lib` | Parse TOML → serialize JSON → deserialize JSON → compare |
| `test_pipeline_spec_from_toml_invalid` | `lib` | Invalid TOML returns ParseError |
| `test_validate_empty_sources_fails` | `validation` | Spec with no sources returns EmptySources |
| `test_validate_empty_stages_fails` | `validation` | Spec with no stages returns EmptyPipeline |
| `test_validate_unknown_source_reference_fails` | `validation` | Stage referencing nonexistent source returns UnknownSource |
| `test_validate_valid_spec_passes` | `validation` | Valid spec passes validation |
| `test_retry_spec_proptest_roundtrip` | `defaults` | Random RetrySpec values survive serde roundtrip (proptest) |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline-spec` passes
- [ ] `cargo test -p ecl-pipeline-spec` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement execution logic (that is milestone 2.2)
- Do NOT implement topology resolution or resource graph (that is milestone 1.3)
- Do NOT implement state types or checkpointing (that is milestone 1.2)
- Do NOT implement CLI commands (that is milestone 5.1)
- Do NOT create adapter or stage implementations (that is milestone 3.1+)
- Do NOT add any crates beyond `ecl-pipeline-spec`
