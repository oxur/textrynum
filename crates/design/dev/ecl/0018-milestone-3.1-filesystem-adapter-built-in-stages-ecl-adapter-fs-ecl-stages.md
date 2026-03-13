# Milestone 3.1: Filesystem Adapter + Built-in Stages (`ecl-adapter-fs`, `ecl-stages`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefixes:** `feat(ecl-adapter-fs):` for the filesystem adapter crate,
`feat(ecl-stages):` for the built-in stages crate

## 1. Goal

Create two new crates: `ecl-adapter-fs` (filesystem source adapter) and
`ecl-stages` (built-in stage implementations). When done:

1. `FilesystemAdapter` implements the `SourceAdapter` trait, walks a directory
   recursively, applies extension filters, and fetches file content with blake3
   hashing.
2. `ExtractStage` delegates to a `SourceAdapter` to fetch content for each
   pipeline item.
3. `NormalizeStage` is a passthrough placeholder (returns items unchanged).
4. `FilterStage` applies glob-based include/exclude rules from stage params.
5. `EmitStage` writes pipeline item content to the output directory.
6. Both crates compile, all tests pass, lint passes, and coverage is >= 95%.
7. An end-to-end integration test demonstrates a complete pipeline: filesystem
   source -> extract -> filter -> normalize -> emit, with incrementality
   verification.

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, TheError>;`
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
3. `assets/ai/ai-rust/guides/06-traits.md`
4. `assets/ai/ai-rust/guides/12-project-structure.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` -- Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` -- Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` -- Module structure, re-exports, doc comments
- `crates/ecl-core/src/llm/provider.rs` -- `async_trait` pattern for object-safe traits

## 3. Prior Art / Dependencies

These two crates depend on types from prior milestone crates. Below are the
**exact public APIs** you will use. You do NOT implement these -- they already
exist.

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
    /// Root directory to scan.
    pub root: PathBuf,
    /// Include/exclude filter rules.
    #[serde(default)]
    pub filters: Vec<FilterRule>,
    /// File extensions to include (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
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
    Include,
    Exclude,
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

pub type Result<T> = std::result::Result<T, SpecError>;
```

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
// --- crate: ecl-pipeline-topo ---

/// A source adapter handles all interaction with an external data service.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    /// Human-readable name of the source type (e.g., "filesystem").
    fn source_kind(&self) -> &str;

    /// Enumerate items available from this source.
    /// Returns lightweight descriptors (no content) for filtering and
    /// hash comparison.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by `SourceAdapter::enumerate()`.
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
    pub source_hash: Option<String>,
}

/// A document extracted from a source, in its original format.
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

/// The intermediate representation flowing between stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    /// The item's unique identifier (stable across stages).
    pub id: String,
    /// Human-readable name.
    pub display_name: String,
    /// Current content (may be transformed by prior stages).
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,
    /// Current MIME type.
    pub mime_type: String,
    /// Which source this item came from.
    pub source_name: String,
    /// Content hash of the original source content (for incrementality).
    pub source_content_hash: Blake3Hash,
    /// Provenance chain.
    pub provenance: ItemProvenance,
    /// Metadata accumulated by stages.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    /// Human-readable name of this stage type.
    fn name(&self) -> &str;

    /// Process a single item. Returns:
    /// - `Ok(vec![item])` -- item transformed successfully
    /// - `Ok(vec![])` -- item filtered out / consumed
    /// - `Err(e)` -- processing failed
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
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

// Error types
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    #[error("authentication failed for source '{source}': {message}")]
    AuthError { source: String, message: String },
    #[error("rate limited by '{source}': retry after {retry_after_secs}s")]
    RateLimited { source: String, retry_after_secs: u64 },
    #[error("item '{item_id}' not found in source '{source}'")]
    NotFound { source: String, item_id: String },
    #[error("transient error from source '{source}': {message}")]
    Transient { source: String, message: String },
    #[error("permanent error from source '{source}': {message}")]
    Permanent { source: String, message: String },
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum StageError {
    #[error("stage '{stage}' cannot process item '{item_id}': {message}")]
    UnsupportedContent { stage: String, item_id: String, message: String },
    #[error("transient error in stage '{stage}' for item '{item_id}': {message}")]
    Transient { stage: String, item_id: String, message: String },
    #[error("permanent error in stage '{stage}' for item '{item_id}': {message}")]
    Permanent { stage: String, item_id: String, message: String },
    #[error("stage '{stage}' timed out after {timeout_secs}s for item '{item_id}'")]
    Timeout { stage: String, item_id: String, timeout_secs: u64 },
}

pub type ResolveResult<T> = std::result::Result<T, ResolveError>;
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
// --- crate: ecl-pipeline-state ---

/// Blake3 content hash, stored as hex string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);
impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
    pub fn is_empty(&self) -> bool;
}

/// Provenance information for a pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    pub source_kind: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub source_modified: Option<DateTime<Utc>>,
    pub extracted_at: DateTime<Utc>,
}
```

### From `ecl-pipeline` (Milestone 2.2 / 2.3)

```rust
// --- crate: ecl-pipeline ---

/// Registry of source adapter factories, keyed by source `kind` string.
#[derive(Default)]
pub struct AdapterRegistry { /* ... */ }

impl AdapterRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, kind: &str, factory: AdapterFactory);
    pub fn resolve(
        &self, kind: &str, source_name: &str, spec: &SourceSpec,
    ) -> Result<Arc<dyn SourceAdapter>, ResolveError>;
    pub fn contains(&self, kind: &str) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

/// Registry of stage handler factories, keyed by stage `adapter` string.
#[derive(Default)]
pub struct StageRegistry { /* ... */ }

impl StageRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, adapter: &str, factory: StageFactory);
    pub fn resolve(
        &self, adapter: &str, stage_name: &str, spec: &StageSpec,
    ) -> Result<Arc<dyn Stage>, ResolveError>;
    pub fn contains(&self, adapter: &str) -> bool;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
}

/// Type alias for source adapter factory functions.
pub type AdapterFactory =
    Box<dyn Fn(&SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + Send + Sync>;

/// Type alias for stage handler factory functions.
pub type StageFactory =
    Box<dyn Fn(&StageSpec) -> Result<Arc<dyn Stage>, ResolveError> + Send + Sync>;

/// PipelineRunner executes the full pipeline lifecycle.
pub struct PipelineRunner { /* topology, state, store */ }

impl PipelineRunner {
    pub async fn new(
        topology: PipelineTopology,
        store: Box<dyn StateStore>,
    ) -> Result<Self>;
    pub async fn run(&mut self) -> Result<PipelineState>;
}
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add `async-walkdir`, `glob` to workspace deps; add both crates to members |
| `crates/ecl-adapter-fs/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-adapter-fs/src/lib.rs` | Create | `FilesystemAdapter` struct, `SourceAdapter` impl, module re-exports |
| `crates/ecl-stages/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-stages/src/lib.rs` | Create | Module declarations, re-exports |
| `crates/ecl-stages/src/extract.rs` | Create | `ExtractStage` -- delegates to `SourceAdapter::fetch()` |
| `crates/ecl-stages/src/normalize.rs` | Create | `NormalizeStage` -- passthrough placeholder |
| `crates/ecl-stages/src/filter.rs` | Create | `FilterStage` -- glob include/exclude |
| `crates/ecl-stages/src/emit.rs` | Create | `EmitStage` -- writes content to output dir |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml`:

```toml
async-walkdir = "2"
glob = "0.3"
```

Add to `[workspace] members`:

```toml
"crates/ecl-adapter-fs",
"crates/ecl-stages",
```

### `crates/ecl-adapter-fs/Cargo.toml`

```toml
[package]
name = "ecl-adapter-fs"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Filesystem source adapter for ECL pipeline runner"

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
tokio = { workspace = true }
blake3 = { workspace = true }
async-walkdir = { workspace = true }
glob = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

### `crates/ecl-stages/Cargo.toml`

```toml
[package]
name = "ecl-stages"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Built-in stage implementations for ECL pipeline runner"

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
tokio = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
glob = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
blake3 = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown. These are the
authoritative signatures for this milestone.

### `crates/ecl-adapter-fs/src/lib.rs`

```rust
//! Filesystem source adapter for the ECL pipeline runner.
//!
//! This crate implements `SourceAdapter` for local filesystem sources.
//! It recursively walks a directory, applies extension filters, and reads
//! file content with blake3 hashing.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use ecl_pipeline_spec::source::{FilesystemSourceSpec, FilterAction, FilterRule};
use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::SourceError;
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

/// A source adapter that reads files from the local filesystem.
///
/// Constructed from a `FilesystemSourceSpec`. Walks the `root` directory
/// recursively, applying extension filters during enumeration and reading
/// file content during fetch.
#[derive(Debug)]
pub struct FilesystemAdapter {
    /// Root directory to scan.
    root: PathBuf,
    /// File extensions to include (empty = all files).
    extensions: Vec<String>,
    /// Include/exclude filter rules applied during enumeration.
    filters: Vec<FilterRule>,
}

impl FilesystemAdapter {
    /// Create a new `FilesystemAdapter` from a `FilesystemSourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Permanent` if the root directory does not exist
    /// or is not a directory.
    pub fn new(spec: &FilesystemSourceSpec) -> Result<Self, SourceError> {
        if !spec.root.exists() {
            return Err(SourceError::Permanent {
                source: "filesystem".to_string(),
                message: format!("root directory does not exist: {}", spec.root.display()),
            });
        }
        if !spec.root.is_dir() {
            return Err(SourceError::Permanent {
                source: "filesystem".to_string(),
                message: format!("root path is not a directory: {}", spec.root.display()),
            });
        }
        Ok(Self {
            root: spec.root.clone(),
            extensions: spec.extensions.clone(),
            filters: spec.filters.clone(),
        })
    }

    /// Create a `FilesystemAdapter` from a `SourceSpec`.
    ///
    /// Extracts the `FilesystemSourceSpec` from the `SourceSpec::Filesystem`
    /// variant. Returns an error if the spec is not a filesystem source.
    pub fn from_source_spec(spec: &SourceSpec) -> Result<Self, SourceError> {
        match spec {
            SourceSpec::Filesystem(fs_spec) => Self::new(fs_spec),
            _ => Err(SourceError::Permanent {
                source: "filesystem".to_string(),
                message: "expected filesystem source spec".to_string(),
            }),
        }
    }

    /// Check whether a file path passes the extension filter.
    ///
    /// If `extensions` is empty, all files pass. Otherwise, the file's
    /// extension must match one of the allowed extensions (case-insensitive).
    fn passes_extension_filter(&self, path: &std::path::Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => self.extensions.iter().any(|allowed| {
                allowed.eq_ignore_ascii_case(ext)
            }),
            None => false,
        }
    }

    /// Check whether a path passes the include/exclude filter rules.
    ///
    /// Rules are evaluated in order. The first matching rule determines
    /// whether the path is included or excluded. If no rules match, the
    /// path is included by default.
    fn passes_filter_rules(&self, path_str: &str) -> bool {
        for rule in &self.filters {
            if let Ok(pattern) = glob::Pattern::new(&rule.pattern) {
                if pattern.matches(path_str) {
                    return matches!(rule.action, FilterAction::Include);
                }
            }
        }
        // Default: include if no rules matched.
        true
    }

    /// Derive a MIME type from a file extension.
    ///
    /// Returns a basic MIME type for common file types, falling back to
    /// `"application/octet-stream"` for unknown extensions.
    fn mime_from_extension(path: &std::path::Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("txt") => "text/plain".to_string(),
            Some("md") | Some("markdown") => "text/markdown".to_string(),
            Some("html") | Some("htm") => "text/html".to_string(),
            Some("json") => "application/json".to_string(),
            Some("toml") => "application/toml".to_string(),
            Some("yaml") | Some("yml") => "application/yaml".to_string(),
            Some("xml") => "application/xml".to_string(),
            Some("pdf") => "application/pdf".to_string(),
            Some("csv") => "text/csv".to_string(),
            Some("rs") => "text/x-rust".to_string(),
            Some("py") => "text/x-python".to_string(),
            Some("js") => "text/javascript".to_string(),
            Some("ts") => "text/typescript".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
}

#[async_trait]
impl SourceAdapter for FilesystemAdapter {
    fn source_kind(&self) -> &str {
        "filesystem"
    }

    /// Enumerate all files under `root` that pass extension and filter rules.
    ///
    /// Walks the directory tree recursively using `async-walkdir`. For each
    /// file, reads filesystem metadata to populate `modified_at`. The `id`
    /// field is the relative path from the root directory.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        use async_walkdir::WalkDir;
        use tokio_stream::StreamExt;

        let mut items = Vec::new();
        let mut walker = WalkDir::new(&self.root);

        while let Some(entry_result) = walker.next().await {
            let entry = entry_result.map_err(|e| SourceError::Transient {
                source: "filesystem".to_string(),
                message: format!("directory walk error: {e}"),
            })?;

            let path = entry.path();

            // Skip directories -- we only enumerate files.
            let file_type = entry.file_type().await.map_err(|e| SourceError::Transient {
                source: "filesystem".to_string(),
                message: format!("failed to read file type for {}: {e}", path.display()),
            })?;
            if !file_type.is_file() {
                continue;
            }

            // Apply extension filter.
            if !self.passes_extension_filter(&path) {
                continue;
            }

            // Compute relative path for filter rules and as the item ID.
            let relative_path = path
                .strip_prefix(&self.root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Apply include/exclude filter rules.
            if !self.passes_filter_rules(&relative_path) {
                continue;
            }

            // Read filesystem metadata for modified_at.
            let metadata = tokio::fs::metadata(&path).await.map_err(|e| {
                SourceError::Transient {
                    source: "filesystem".to_string(),
                    message: format!("failed to read metadata for {}: {e}", path.display()),
                }
            })?;

            let modified_at = metadata.modified().ok().map(|t| {
                DateTime::<Utc>::from(t)
            });

            let display_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| relative_path.clone());

            let mime_type = Self::mime_from_extension(&path);

            items.push(SourceItem {
                id: relative_path.clone(),
                display_name,
                mime_type,
                path: relative_path,
                modified_at,
                source_hash: None, // Filesystem doesn't provide a cheap pre-fetch hash.
            });
        }

        // Sort for deterministic ordering.
        items.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(items)
    }

    /// Fetch the full content of a file identified by the `SourceItem`.
    ///
    /// Reads the file at `root / item.id`, computes a blake3 hash of the
    /// content, and builds an `ExtractedDocument`.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let full_path = self.root.join(&item.id);

        let content = tokio::fs::read(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SourceError::NotFound {
                    source: "filesystem".to_string(),
                    item_id: item.id.clone(),
                }
            } else {
                SourceError::Transient {
                    source: "filesystem".to_string(),
                    message: format!("failed to read {}: {e}", full_path.display()),
                }
            }
        })?;

        let hash = blake3::hash(&content);
        let content_hash = Blake3Hash::new(hash.to_hex().to_string());

        let metadata = tokio::fs::metadata(&full_path).await.map_err(|e| {
            SourceError::Transient {
                source: "filesystem".to_string(),
                message: format!("failed to read metadata for {}: {e}", full_path.display()),
            }
        })?;

        let source_modified = metadata.modified().ok().map(DateTime::<Utc>::from);

        let provenance = ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: {
                let mut m = BTreeMap::new();
                m.insert(
                    "path".to_string(),
                    serde_json::Value::String(full_path.to_string_lossy().to_string()),
                );
                m
            },
            source_modified,
            extracted_at: Utc::now(),
        };

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: item.mime_type.clone(),
            provenance,
            content_hash,
        })
    }
}
```

### `crates/ecl-stages/src/lib.rs`

```rust
//! Built-in stage implementations for the ECL pipeline runner.
//!
//! This crate provides the standard stages:
//! - `ExtractStage` -- delegates to a `SourceAdapter` to fetch content
//! - `NormalizeStage` -- passthrough placeholder (format conversion deferred)
//! - `FilterStage` -- glob-based include/exclude filtering
//! - `EmitStage` -- writes content to the output directory

pub mod emit;
pub mod extract;
pub mod filter;
pub mod normalize;

pub use emit::EmitStage;
pub use extract::ExtractStage;
pub use filter::FilterStage;
pub use normalize::NormalizeStage;
```

### `crates/ecl-stages/src/extract.rs`

```rust
//! Extract stage: delegates to a SourceAdapter to fetch content.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;

use ecl_pipeline_state::Blake3Hash;
use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, SourceAdapter, SourceItem, Stage, StageContext};

/// A stage that extracts content by delegating to a `SourceAdapter`.
///
/// For each `PipelineItem`, reconstructs a `SourceItem` from the item's ID
/// and metadata, then calls `SourceAdapter::fetch()` to retrieve the full
/// content. The fetched content replaces the item's content.
///
/// This stage is the bridge between the source enumeration (which produces
/// lightweight descriptors) and the processing pipeline (which needs content).
#[derive(Debug)]
pub struct ExtractStage {
    /// The source adapter to delegate fetch operations to.
    adapter: Arc<dyn SourceAdapter>,
}

impl ExtractStage {
    /// Create a new `ExtractStage` with the given source adapter.
    pub fn new(adapter: Arc<dyn SourceAdapter>) -> Self {
        Self { adapter }
    }
}

#[async_trait]
impl Stage for ExtractStage {
    fn name(&self) -> &str {
        "extract"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Reconstruct a SourceItem from the PipelineItem's fields.
        let source_item = SourceItem {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            mime_type: item.mime_type.clone(),
            path: item.id.clone(),
            modified_at: item.provenance.source_modified,
            source_hash: None,
        };

        // Fetch the full content via the adapter.
        let doc = self.adapter.fetch(&source_item).await.map_err(|e| {
            StageError::Transient {
                stage: "extract".to_string(),
                item_id: item.id.clone(),
                message: format!("{e}"),
            }
        })?;

        // Build the output PipelineItem with the fetched content.
        let output = PipelineItem {
            id: doc.id,
            display_name: doc.display_name,
            content: Arc::from(doc.content.as_slice()),
            mime_type: doc.mime_type,
            source_name: item.source_name,
            source_content_hash: doc.content_hash,
            provenance: doc.provenance,
            metadata: item.metadata,
        };

        Ok(vec![output])
    }
}
```

### `crates/ecl-stages/src/normalize.rs`

```rust
//! Normalize stage: passthrough placeholder.
//!
//! This stage is a placeholder for future format conversion logic
//! (e.g., PDF -> markdown, DOCX -> markdown). Currently, it returns
//! items unchanged.

use async_trait::async_trait;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// A passthrough stage that returns items unchanged.
///
/// This is a placeholder for future normalization logic. When real format
/// conversion is implemented, this stage will detect the input MIME type
/// and convert content to a normalized format (e.g., markdown).
#[derive(Debug)]
pub struct NormalizeStage;

impl NormalizeStage {
    /// Create a new `NormalizeStage`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NormalizeStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for NormalizeStage {
    fn name(&self) -> &str {
        "normalize"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Passthrough: return the item unchanged.
        Ok(vec![item])
    }
}
```

### `crates/ecl-stages/src/filter.rs`

```rust
//! Filter stage: glob-based include/exclude filtering.

use async_trait::async_trait;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// A stage that filters pipeline items based on glob include/exclude rules.
///
/// Filter rules are read from `StageContext.params` at process time:
///
/// ```json
/// {
///   "include": ["**/*.md", "**/*.txt"],
///   "exclude": ["**/drafts/**"]
/// }
/// ```
///
/// Evaluation order:
/// 1. If `exclude` patterns are defined and the item's path matches any,
///    the item is excluded (returns empty vec).
/// 2. If `include` patterns are defined and the item's path matches none,
///    the item is excluded.
/// 3. Otherwise, the item passes through.
///
/// If neither `include` nor `exclude` is specified, all items pass.
#[derive(Debug)]
pub struct FilterStage;

impl FilterStage {
    /// Create a new `FilterStage`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for FilterStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for FilterStage {
    fn name(&self) -> &str {
        "filter"
    }

    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let path = &item.id;

        // Parse exclude patterns from params.
        let exclude_patterns = extract_glob_patterns(&ctx.params, "exclude");
        let include_patterns = extract_glob_patterns(&ctx.params, "include");

        // Step 1: Check exclude patterns.
        for pattern in &exclude_patterns {
            if pattern.matches(path) {
                tracing::debug!(
                    item_id = %item.id,
                    pattern = %pattern.as_str(),
                    "item excluded by exclude pattern"
                );
                return Ok(vec![]);
            }
        }

        // Step 2: Check include patterns (if any are defined).
        if !include_patterns.is_empty() {
            let matches_any = include_patterns.iter().any(|p| p.matches(path));
            if !matches_any {
                tracing::debug!(
                    item_id = %item.id,
                    "item excluded: did not match any include pattern"
                );
                return Ok(vec![]);
            }
        }

        // Item passes all filters.
        Ok(vec![item])
    }
}

/// Extract glob patterns from a JSON value by key.
///
/// The value at `key` should be an array of strings. Each string is parsed
/// as a glob pattern. Invalid patterns are logged and skipped.
fn extract_glob_patterns(params: &serde_json::Value, key: &str) -> Vec<glob::Pattern> {
    params
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| match glob::Pattern::new(s) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        tracing::warn!(
                            pattern = s,
                            error = %e,
                            "invalid glob pattern in filter stage params, skipping"
                        );
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}
```

### `crates/ecl-stages/src/emit.rs`

```rust
//! Emit stage: writes pipeline item content to the output directory.

use std::path::PathBuf;

use async_trait::async_trait;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// A stage that writes pipeline item content to the filesystem.
///
/// Reads the target subdirectory from `StageContext.params`:
///
/// ```json
/// {
///   "subdir": "normalized"
/// }
/// ```
///
/// Output path: `{ctx.output_dir}/{subdir}/{item.id}`
///
/// Parent directories are created as needed. The item is returned unchanged
/// after writing (pass-through), so downstream stages can still access it.
#[derive(Debug)]
pub struct EmitStage;

impl EmitStage {
    /// Create a new `EmitStage`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EmitStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for EmitStage {
    fn name(&self) -> &str {
        "emit"
    }

    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Determine the output subdirectory from params.
        let subdir = ctx
            .params
            .get("subdir")
            .and_then(|v| v.as_str())
            .unwrap_or("output");

        // Build the full output path.
        let output_path: PathBuf = ctx.output_dir.join(subdir).join(&item.id);

        // Create parent directories if needed (async to avoid AP-18).
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                StageError::Permanent {
                    stage: "emit".to_string(),
                    item_id: item.id.clone(),
                    message: format!(
                        "failed to create output directory {}: {e}",
                        parent.display()
                    ),
                }
            })?;
        }

        // Write the content to the output file.
        tokio::fs::write(&output_path, &*item.content)
            .await
            .map_err(|e| StageError::Permanent {
                stage: "emit".to_string(),
                item_id: item.id.clone(),
                message: format!(
                    "failed to write output file {}: {e}",
                    output_path.display()
                ),
            })?;

        tracing::debug!(
            item_id = %item.id,
            output_path = %output_path.display(),
            bytes = item.content.len(),
            "emitted item to filesystem"
        );

        // Return the item unchanged (pass-through).
        Ok(vec![item])
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold `ecl-adapter-fs` and verify compilation

- [ ] Create `crates/ecl-adapter-fs/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Filesystem source adapter.`
- [ ] Add `async-walkdir = "2"` and `glob = "0.3"` to root `Cargo.toml` `[workspace.dependencies]`
- [ ] Add `"crates/ecl-adapter-fs"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-adapter-fs` -- must pass
- [ ] Commit: `feat(ecl-adapter-fs): scaffold crate`

### Step 2: `FilesystemAdapter` constructor and helper methods

- [ ] Implement `FilesystemAdapter::new()`, `from_source_spec()`, `passes_extension_filter()`, `passes_filter_rules()`, `mime_from_extension()` (from section 6)
- [ ] Write test: `test_new_valid_directory_succeeds` -- create tempdir, construct adapter, verify no error
- [ ] Write test: `test_new_nonexistent_directory_returns_error` -- pass nonexistent path, verify `SourceError::Permanent`
- [ ] Write test: `test_new_file_not_directory_returns_error` -- pass a file path, verify error
- [ ] Write test: `test_from_source_spec_filesystem_succeeds` -- wrap in `SourceSpec::Filesystem`, verify extraction
- [ ] Write test: `test_from_source_spec_wrong_variant_returns_error` -- pass `SourceSpec::GoogleDrive`, verify error
- [ ] Write test: `test_passes_extension_filter_empty_allows_all` -- empty extensions list, any file passes
- [ ] Write test: `test_passes_extension_filter_matching_extension` -- `["txt"]` filter, `.txt` file passes
- [ ] Write test: `test_passes_extension_filter_non_matching_extension` -- `["txt"]` filter, `.pdf` file rejected
- [ ] Write test: `test_passes_extension_filter_case_insensitive` -- `["TXT"]` filter, `.txt` file passes
- [ ] Write test: `test_passes_filter_rules_exclude_match` -- exclude rule matches, file rejected
- [ ] Write test: `test_passes_filter_rules_include_match` -- include rule matches, file accepted
- [ ] Write test: `test_passes_filter_rules_no_rules_allows_all` -- empty rules, file passes
- [ ] Write test: `test_mime_from_extension_known_types` -- verify txt, md, pdf, json return correct MIME
- [ ] Write test: `test_mime_from_extension_unknown_returns_octet_stream` -- unknown extension returns `application/octet-stream`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-fs): add FilesystemAdapter constructor and helpers`

### Step 3: `FilesystemAdapter::enumerate()`

- [ ] Implement `enumerate()` method (from section 6)
- [ ] Write test: `test_enumerate_empty_directory_returns_empty` -- empty tempdir, verify empty vec
- [ ] Write test: `test_enumerate_finds_all_files` -- tempdir with 3 files in nested dirs, verify 3 items
- [ ] Write test: `test_enumerate_applies_extension_filter` -- tempdir with `.txt` and `.pdf` files, filter to `.txt` only
- [ ] Write test: `test_enumerate_applies_filter_rules` -- tempdir with files, exclude rule, verify filtered results
- [ ] Write test: `test_enumerate_skips_directories` -- tempdir with subdirs, verify only files returned
- [ ] Write test: `test_enumerate_relative_paths` -- verify `SourceItem.id` is relative to root
- [ ] Write test: `test_enumerate_deterministic_order` -- verify items are sorted by id
- [ ] Write test: `test_enumerate_populates_modified_at` -- verify `modified_at` is set
- [ ] Write test: `test_enumerate_source_kind_is_filesystem` -- verify `source_kind()` returns `"filesystem"`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-fs): implement enumerate()`

### Step 4: `FilesystemAdapter::fetch()`

- [ ] Implement `fetch()` method (from section 6)
- [ ] Write test: `test_fetch_reads_file_content` -- write known content, fetch, verify content matches
- [ ] Write test: `test_fetch_computes_blake3_hash` -- write content, fetch, verify hash is non-empty and correct
- [ ] Write test: `test_fetch_nonexistent_file_returns_not_found` -- fetch with bad id, verify `SourceError::NotFound`
- [ ] Write test: `test_fetch_populates_provenance` -- verify provenance has `source_kind = "filesystem"` and path metadata
- [ ] Write test: `test_fetch_preserves_mime_type` -- verify fetched doc has same MIME as source item
- [ ] Run tests, verify pass
- [ ] Run `make test`, `make lint`, `make format` for ecl-adapter-fs
- [ ] Commit: `feat(ecl-adapter-fs): implement fetch()`

### Step 5: Scaffold `ecl-stages` and verify compilation

- [ ] Create `crates/ecl-stages/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create `src/lib.rs` with module declarations (from section 6)
- [ ] Create empty files: `src/extract.rs`, `src/normalize.rs`, `src/filter.rs`, `src/emit.rs`
- [ ] Add `"crates/ecl-stages"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-stages` -- must pass
- [ ] Commit: `feat(ecl-stages): scaffold crate`

### Step 6: `ExtractStage`

- [ ] Implement `ExtractStage` (from section 6)
- [ ] Write test: `test_extract_stage_name` -- verify `name()` returns `"extract"`
- [ ] Write test: `test_extract_stage_fetches_content` -- create a mock adapter (inline struct implementing `SourceAdapter`), process item, verify content from adapter's `fetch()`
- [ ] Write test: `test_extract_stage_returns_one_item` -- verify `process()` returns exactly one item
- [ ] Write test: `test_extract_stage_preserves_source_name` -- verify `source_name` is preserved
- [ ] Write test: `test_extract_stage_propagates_fetch_error` -- mock adapter returns error, verify `StageError::Transient`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-stages): implement ExtractStage`

### Step 7: `NormalizeStage`

- [ ] Implement `NormalizeStage` (from section 6)
- [ ] Write test: `test_normalize_stage_name` -- verify `name()` returns `"normalize"`
- [ ] Write test: `test_normalize_stage_passthrough` -- process item, verify output equals input
- [ ] Write test: `test_normalize_stage_returns_one_item` -- verify exactly one item returned
- [ ] Write test: `test_normalize_stage_default` -- verify `Default::default()` works
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-stages): implement NormalizeStage`

### Step 8: `FilterStage`

- [ ] Implement `FilterStage` (from section 6)
- [ ] Write test: `test_filter_stage_name` -- verify `name()` returns `"filter"`
- [ ] Write test: `test_filter_stage_no_params_passes_all` -- empty params, item passes through
- [ ] Write test: `test_filter_stage_include_match_passes` -- include `["**/*.md"]`, item with `.md` passes
- [ ] Write test: `test_filter_stage_include_no_match_excludes` -- include `["**/*.md"]`, item with `.txt` excluded
- [ ] Write test: `test_filter_stage_exclude_match_excludes` -- exclude `["**/drafts/**"]`, matching item excluded
- [ ] Write test: `test_filter_stage_exclude_no_match_passes` -- exclude `["**/drafts/**"]`, non-matching passes
- [ ] Write test: `test_filter_stage_exclude_overrides_include` -- exclude matches take precedence
- [ ] Write test: `test_filter_stage_invalid_glob_skipped` -- invalid glob in params is silently skipped
- [ ] Write test: `test_filter_stage_default` -- verify `Default::default()` works
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-stages): implement FilterStage`

### Step 9: `EmitStage`

- [ ] Implement `EmitStage` (from section 6)
- [ ] Write test: `test_emit_stage_name` -- verify `name()` returns `"emit"`
- [ ] Write test: `test_emit_stage_writes_file` -- process item, verify file exists with correct content
- [ ] Write test: `test_emit_stage_creates_parent_directories` -- item with nested path, verify dirs created
- [ ] Write test: `test_emit_stage_uses_subdir_from_params` -- set `subdir = "normalized"`, verify path
- [ ] Write test: `test_emit_stage_default_subdir` -- no subdir in params, verify `"output"` used
- [ ] Write test: `test_emit_stage_returns_item_unchanged` -- verify pass-through behavior
- [ ] Write test: `test_emit_stage_overwrites_existing_file` -- write twice, verify latest content
- [ ] Write test: `test_emit_stage_default` -- verify `Default::default()` works
- [ ] Run tests, verify pass
- [ ] Run `make test`, `make lint`, `make format` for ecl-stages
- [ ] Commit: `feat(ecl-stages): implement EmitStage`

### Step 10: End-to-end integration test

This step creates a test (inside `ecl-stages`) that exercises the full
pipeline path. This does NOT use `PipelineRunner` (which is in a different
crate) -- it manually chains the stages to verify correctness.

- [ ] Write test: `test_end_to_end_filesystem_pipeline` in `ecl-stages/src/lib.rs`
  - Create a tempdir with 5 `.txt` files containing known content
  - Create a `FilesystemAdapter` pointing at the tempdir
  - Create `ExtractStage`, `FilterStage`, `NormalizeStage`, `EmitStage`
  - Enumerate items via the adapter
  - For each item, construct a `PipelineItem` with minimal fields
  - Run each item through: extract -> filter -> normalize -> emit
  - Verify output files exist with correct content
  - Verify 5 files were emitted

- [ ] Write test: `test_end_to_end_filter_reduces_items`
  - Create tempdir with 3 `.txt` and 2 `.md` files
  - Use `FilterStage` with `include = ["**/*.md"]`
  - Verify only 2 files pass through the filter

- [ ] Run `make test`, `make lint`, `make format` (workspace-wide)
- [ ] Commit: `feat(ecl-stages): add end-to-end integration tests`

### Step 11: Final polish and coverage

- [ ] Run `make coverage` and identify any gaps below 95%
- [ ] Add tests for any uncovered code paths
- [ ] Verify all public items have doc comments
- [ ] Verify no compiler warnings
- [ ] Run `make test`, `make lint`, `make format` one final time
- [ ] Commit: `feat(ecl-adapter-fs,ecl-stages): final polish and coverage`

## 8. Test Fixtures

### Helper: Create a test `PipelineItem`

Use this helper in stage tests to create minimal `PipelineItem` values:

```rust
use std::collections::BTreeMap;
use std::sync::Arc;
use chrono::Utc;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::PipelineItem;

fn make_test_item(id: &str, content: &[u8]) -> PipelineItem {
    PipelineItem {
        id: id.to_string(),
        display_name: id.to_string(),
        content: Arc::from(content),
        mime_type: "text/plain".to_string(),
        source_name: "test-source".to_string(),
        source_content_hash: Blake3Hash::new(
            blake3::hash(content).to_hex().to_string()
        ),
        provenance: ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: BTreeMap::new(),
            source_modified: None,
            extracted_at: Utc::now(),
        },
        metadata: BTreeMap::new(),
    }
}
```

### Helper: Create a test `StageContext`

```rust
use std::path::PathBuf;
use std::sync::Arc;
use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_topo::StageContext;

fn make_test_context(output_dir: PathBuf, params: serde_json::Value) -> StageContext {
    // Use a minimal spec -- the stages under test don't inspect the spec.
    let toml_str = r#"
name = "test"
version = 1
output_dir = "./output"

[sources.local]
kind = "filesystem"
root = "/tmp/test"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw"] }
"#;
    let spec = PipelineSpec::from_toml(toml_str).unwrap();

    StageContext {
        spec: Arc::new(spec),
        output_dir,
        params,
        span: tracing::Span::none(),
    }
}
```

### Helper: Create a mock `SourceAdapter` for `ExtractStage` tests

```rust
use std::collections::BTreeMap;
use std::sync::Arc;
use async_trait::async_trait;
use chrono::Utc;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::SourceError;
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

#[derive(Debug)]
struct MockAdapter {
    content: Vec<u8>,
    should_fail: bool,
}

#[async_trait]
impl SourceAdapter for MockAdapter {
    fn source_kind(&self) -> &str {
        "mock"
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        Ok(vec![])
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        if self.should_fail {
            return Err(SourceError::Transient {
                source: "mock".to_string(),
                message: "mock failure".to_string(),
            });
        }
        let hash = blake3::hash(&self.content);
        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content: self.content.clone(),
            mime_type: item.mime_type.clone(),
            provenance: ItemProvenance {
                source_kind: "mock".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: Utc::now(),
            },
            content_hash: Blake3Hash::new(hash.to_hex().to_string()),
        })
    }
}
```

### Tempdir with test files

Use this pattern in filesystem adapter tests:

```rust
use tempfile::TempDir;
use tokio::fs;

async fn create_test_files(dir: &std::path::Path, files: &[(&str, &str)]) {
    for (name, content) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.unwrap();
        }
        fs::write(&path, content).await.unwrap();
    }
}
```

### Minimal TOML for integration tests

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

[stages.filter]
adapter = "filter"
resources = { reads = ["raw-docs"], creates = ["filtered-docs"] }

[stages.filter.params]
include = ["**/*.txt", "**/*.md"]

[stages.normalize]
adapter = "normalize"
resources = { reads = ["filtered-docs"], creates = ["normalized-docs"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["normalized-docs"] }

[stages.emit.params]
subdir = "normalized"
```

## 9. Test Specifications

### `ecl-adapter-fs` Tests

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_new_valid_directory_succeeds` | `lib` | Constructor succeeds with valid directory |
| `test_new_nonexistent_directory_returns_error` | `lib` | Constructor fails with `SourceError::Permanent` for missing dir |
| `test_new_file_not_directory_returns_error` | `lib` | Constructor fails when path is a file, not a directory |
| `test_from_source_spec_filesystem_succeeds` | `lib` | `from_source_spec()` extracts `FilesystemSourceSpec` correctly |
| `test_from_source_spec_wrong_variant_returns_error` | `lib` | `from_source_spec()` rejects non-filesystem variants |
| `test_passes_extension_filter_empty_allows_all` | `lib` | Empty extensions list allows all files |
| `test_passes_extension_filter_matching_extension` | `lib` | Matching extension passes filter |
| `test_passes_extension_filter_non_matching_extension` | `lib` | Non-matching extension is rejected |
| `test_passes_extension_filter_case_insensitive` | `lib` | Extension matching is case-insensitive |
| `test_passes_filter_rules_exclude_match` | `lib` | Exclude rule rejects matching path |
| `test_passes_filter_rules_include_match` | `lib` | Include rule accepts matching path |
| `test_passes_filter_rules_no_rules_allows_all` | `lib` | No filter rules means all paths pass |
| `test_mime_from_extension_known_types` | `lib` | Known extensions return correct MIME types |
| `test_mime_from_extension_unknown_returns_octet_stream` | `lib` | Unknown extension returns `application/octet-stream` |
| `test_enumerate_empty_directory_returns_empty` | `lib` | Empty directory returns empty vec |
| `test_enumerate_finds_all_files` | `lib` | All files in nested dirs are discovered |
| `test_enumerate_applies_extension_filter` | `lib` | Extension filter limits enumeration results |
| `test_enumerate_applies_filter_rules` | `lib` | Include/exclude rules applied during enumeration |
| `test_enumerate_skips_directories` | `lib` | Only files are returned, not directories |
| `test_enumerate_relative_paths` | `lib` | Item IDs are relative paths from root |
| `test_enumerate_deterministic_order` | `lib` | Items are sorted by ID |
| `test_enumerate_populates_modified_at` | `lib` | `modified_at` is set from filesystem metadata |
| `test_enumerate_source_kind_is_filesystem` | `lib` | `source_kind()` returns `"filesystem"` |
| `test_fetch_reads_file_content` | `lib` | Fetched content matches file content |
| `test_fetch_computes_blake3_hash` | `lib` | Content hash is correct blake3 of content |
| `test_fetch_nonexistent_file_returns_not_found` | `lib` | Missing file returns `SourceError::NotFound` |
| `test_fetch_populates_provenance` | `lib` | Provenance has source_kind and path metadata |
| `test_fetch_preserves_mime_type` | `lib` | Fetched document has same MIME as source item |

### `ecl-stages` Tests

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_extract_stage_name` | `extract` | `name()` returns `"extract"` |
| `test_extract_stage_fetches_content` | `extract` | Content comes from adapter's `fetch()` |
| `test_extract_stage_returns_one_item` | `extract` | Exactly one item returned per input |
| `test_extract_stage_preserves_source_name` | `extract` | `source_name` field is preserved |
| `test_extract_stage_propagates_fetch_error` | `extract` | Adapter error becomes `StageError::Transient` |
| `test_normalize_stage_name` | `normalize` | `name()` returns `"normalize"` |
| `test_normalize_stage_passthrough` | `normalize` | Output item equals input item |
| `test_normalize_stage_returns_one_item` | `normalize` | Exactly one item returned |
| `test_normalize_stage_default` | `normalize` | `Default::default()` compiles and works |
| `test_filter_stage_name` | `filter` | `name()` returns `"filter"` |
| `test_filter_stage_no_params_passes_all` | `filter` | Empty params: item passes through |
| `test_filter_stage_include_match_passes` | `filter` | Matching include pattern: item passes |
| `test_filter_stage_include_no_match_excludes` | `filter` | Non-matching include: item excluded (empty vec) |
| `test_filter_stage_exclude_match_excludes` | `filter` | Matching exclude pattern: item excluded |
| `test_filter_stage_exclude_no_match_passes` | `filter` | Non-matching exclude: item passes |
| `test_filter_stage_exclude_overrides_include` | `filter` | Exclude checked before include |
| `test_filter_stage_invalid_glob_skipped` | `filter` | Invalid glob pattern silently ignored |
| `test_filter_stage_default` | `filter` | `Default::default()` compiles and works |
| `test_emit_stage_name` | `emit` | `name()` returns `"emit"` |
| `test_emit_stage_writes_file` | `emit` | File exists with correct content after process |
| `test_emit_stage_creates_parent_directories` | `emit` | Nested path directories are created |
| `test_emit_stage_uses_subdir_from_params` | `emit` | `subdir` param controls output path |
| `test_emit_stage_default_subdir` | `emit` | No subdir param defaults to `"output"` |
| `test_emit_stage_returns_item_unchanged` | `emit` | Item passes through unchanged |
| `test_emit_stage_overwrites_existing_file` | `emit` | Second write overwrites first |
| `test_emit_stage_default` | `emit` | `Default::default()` compiles and works |
| `test_end_to_end_filesystem_pipeline` | `lib` | Full extract -> filter -> normalize -> emit pipeline |
| `test_end_to_end_filter_reduces_items` | `lib` | Filter stage reduces item count |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-adapter-fs` passes
- [ ] `cargo check -p ecl-stages` passes
- [ ] `cargo test -p ecl-adapter-fs` passes (all tests green)
- [ ] `cargo test -p ecl-stages` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement Google Drive or Slack adapters (those are future milestones)
- Do NOT implement real format conversion in `NormalizeStage` (it is a passthrough placeholder)
- Do NOT implement CLI commands (that is milestone 5.1)
- Do NOT modify any existing crate code -- only create new crates and modify the root `Cargo.toml`
- Do NOT implement `PipelineRunner` integration -- the end-to-end test manually chains stages
- Do NOT add any crates beyond `ecl-adapter-fs` and `ecl-stages`
- Do NOT implement state store or checkpointing logic
- Do NOT implement retry logic in stages -- the runner handles retries
