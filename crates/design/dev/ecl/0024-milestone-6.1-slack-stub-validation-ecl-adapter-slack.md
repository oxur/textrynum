# Milestone 6.1: Slack Stub Validation (`ecl-adapter-slack`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-adapter-slack):`

## 1. Goal

Create the `ecl-adapter-slack` crate containing a **stub** Slack source
adapter. This is a **validation milestone** -- the primary goal is to prove
that the `SourceAdapter` trait abstraction is correct by implementing a second
adapter **without changing any trait signatures**.

When done:

1. `SlackAdapter` implements the `SourceAdapter` trait from `ecl-pipeline-topo`.
2. `source_kind()` returns `"slack"`.
3. `enumerate()` returns hardcoded fixture data (not real Slack API calls).
4. `fetch()` returns fixture content (not real Slack API calls).
5. Fixture files in `crates/ecl-adapter-slack/fixtures/` provide the mock data.
6. An integration test runs a mixed-source pipeline with **both** filesystem
   and Slack sources in the same TOML config, demonstrating that:
   - Two different adapter types coexist in one pipeline.
   - Resource-based scheduling places both fetch stages in the same batch.
   - The emit stage receives items from both sources (fan-in).
   - **No trait signature changes were needed.**
7. All tests pass, lint passes, and coverage is >= 95%.

**Critical success criterion:** If implementing `SlackAdapter` requires ANY
changes to the `SourceAdapter` trait, the `Stage` trait, or any types in
`ecl-pipeline-topo`, the abstractions are wrong. Document what changed and why,
but first exhaust all alternatives to avoid trait changes.

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

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` -- Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` -- Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` -- Module structure, re-exports, doc comments
- `crates/ecl-adapter-fs/src/lib.rs` -- **Primary reference:** how `FilesystemAdapter` implements `SourceAdapter`

## 3. Prior Art / Dependencies

This crate depends on types from prior milestone crates. Below are the **exact
public APIs** you will use. You do NOT implement these -- they already exist.

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

/// How to resolve credentials for a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    #[serde(rename = "file")]   File { path: PathBuf },
    #[serde(rename = "env")]    EnvVar { env: String },
    #[serde(rename = "application_default")] ApplicationDefault,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceSpec {
    #[serde(default)] pub reads: Vec<String>,
    #[serde(default)] pub creates: Vec<String>,
    #[serde(default)] pub writes: Vec<String>,
}

pub type Result<T> = std::result::Result<T, SpecError>;
```

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
// --- crate: ecl-pipeline-topo ---

/// A source adapter handles all interaction with an external data service.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    /// Human-readable name of the source type (e.g., "slack").
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
    fn name(&self) -> &str;
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
#[derive(Debug, Clone)]
pub struct StageContext {
    pub spec: Arc<PipelineSpec>,
    pub output_dir: PathBuf,
    pub params: serde_json::Value,
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
}

/// Type alias for source adapter factory functions.
pub type AdapterFactory =
    Box<dyn Fn(&SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + Send + Sync>;

/// Registry of stage handler factories, keyed by stage `adapter` string.
#[derive(Default)]
pub struct StageRegistry { /* ... */ }

impl StageRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, adapter: &str, factory: StageFactory);
    pub fn resolve(
        &self, adapter: &str, stage_name: &str, spec: &StageSpec,
    ) -> Result<Arc<dyn Stage>, ResolveError>;
}

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

### From `ecl-adapter-fs` (Milestone 3.1)

```rust
// --- crate: ecl-adapter-fs ---

/// A source adapter that reads files from the local filesystem.
#[derive(Debug)]
pub struct FilesystemAdapter {
    root: PathBuf,
    extensions: Vec<String>,
    filters: Vec<FilterRule>,
}

impl FilesystemAdapter {
    pub fn new(spec: &FilesystemSourceSpec) -> Result<Self, SourceError>;
    pub fn from_source_spec(spec: &SourceSpec) -> Result<Self, SourceError>;
}

#[async_trait]
impl SourceAdapter for FilesystemAdapter {
    fn source_kind(&self) -> &str { "filesystem" }
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}
```

### From `ecl-stages` (Milestone 3.1)

```rust
// --- crate: ecl-stages ---

pub struct ExtractStage { adapter: Arc<dyn SourceAdapter> }
impl ExtractStage { pub fn new(adapter: Arc<dyn SourceAdapter>) -> Self; }
impl Stage for ExtractStage { /* delegates to adapter.fetch() */ }

pub struct NormalizeStage;
impl Stage for NormalizeStage { /* passthrough */ }

pub struct FilterStage;
impl Stage for FilterStage { /* glob-based include/exclude */ }

pub struct EmitStage;
impl Stage for EmitStage { /* writes content to output dir */ }
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add `crates/ecl-adapter-slack` to workspace members |
| `crates/ecl-adapter-slack/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-adapter-slack/src/lib.rs` | Create | `SlackAdapter` struct, `SourceAdapter` impl, module re-exports |
| `crates/ecl-adapter-slack/src/fixtures.rs` | Create | Fixture loading functions, `SlackMessage` struct |
| `crates/ecl-adapter-slack/fixtures/messages.json` | Create | Array of mock Slack messages |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace] members`:

```toml
"crates/ecl-adapter-slack",
```

No new workspace dependencies are needed -- all required deps already exist.

### Crate Cargo.toml

```toml
[package]
name = "ecl-adapter-slack"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Stub Slack source adapter for ECL pipeline runner (validation only)"

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }
ecl-pipeline = { path = "../ecl-pipeline" }
ecl-adapter-fs = { path = "../ecl-adapter-fs" }
ecl-stages = { path = "../ecl-stages" }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown.

### `crates/ecl-adapter-slack/fixtures/messages.json`

```json
[
  {
    "channel": "C01234ABCDE",
    "thread_ts": null,
    "author": "U001USER",
    "author_name": "Alice",
    "text": "Here is the Q1 roadmap update. We are on track for all milestones.",
    "timestamp": "1709294400.000100",
    "ts_datetime": "2026-03-01T12:00:00Z"
  },
  {
    "channel": "C01234ABCDE",
    "thread_ts": "1709294400.000100",
    "author": "U002USER",
    "author_name": "Bob",
    "text": "Great update! Can you share the dependency chart?",
    "timestamp": "1709294460.000200",
    "ts_datetime": "2026-03-01T12:01:00Z"
  },
  {
    "channel": "C01234ABCDE",
    "thread_ts": "1709294400.000100",
    "author": "U001USER",
    "author_name": "Alice",
    "text": "Sure, I will post it in the thread.",
    "timestamp": "1709294520.000300",
    "ts_datetime": "2026-03-01T12:02:00Z"
  },
  {
    "channel": "C05678FGHIJ",
    "thread_ts": null,
    "author": "U003USER",
    "author_name": "Charlie",
    "text": "Reminder: architecture review meeting at 3pm today.",
    "timestamp": "1709308800.000400",
    "ts_datetime": "2026-03-01T16:00:00Z"
  },
  {
    "channel": "C05678FGHIJ",
    "thread_ts": null,
    "author": "U004USER",
    "author_name": "Diana",
    "text": "The new CI pipeline is passing all checks. Ready for production deploy.",
    "timestamp": "1709312400.000500",
    "ts_datetime": "2026-03-01T17:00:00Z"
  }
]
```

### `crates/ecl-adapter-slack/src/fixtures.rs`

```rust
//! Fixture data loading for the stub Slack adapter.
//!
//! Loads mock Slack messages from the bundled `fixtures/messages.json` file.
//! This module exists only to support the stub adapter -- it will be removed
//! when real Slack API integration is implemented.

use serde::{Deserialize, Serialize};

/// A mock Slack message, deserialized from the fixture file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    /// Channel ID where the message was posted.
    pub channel: String,

    /// Thread timestamp (None for top-level messages).
    pub thread_ts: Option<String>,

    /// User ID of the message author.
    pub author: String,

    /// Display name of the message author.
    pub author_name: String,

    /// Message text content.
    pub text: String,

    /// Slack message timestamp (unique ID within the channel).
    pub timestamp: String,

    /// ISO 8601 datetime string for the message.
    pub ts_datetime: String,
}

/// Load all mock messages from the embedded fixture file.
///
/// The fixture file is embedded at compile time via `include_str!`, so
/// no filesystem access is needed at runtime.
///
/// # Errors
///
/// Returns an error if the fixture file cannot be parsed as JSON.
pub fn load_fixture_messages() -> Result<Vec<SlackMessage>, serde_json::Error> {
    let json_str = include_str!("../fixtures/messages.json");
    serde_json::from_str(json_str)
}
```

### `crates/ecl-adapter-slack/src/lib.rs`

```rust
//! Stub Slack source adapter for the ECL pipeline runner.
//!
//! This crate implements `SourceAdapter` for Slack workspace sources using
//! hardcoded fixture data. It exists to **validate** that the `SourceAdapter`
//! trait abstraction is correct -- a second adapter type can be implemented
//! without changing any trait signatures.
//!
//! **This is NOT a production Slack integration.** Real Slack API calls are
//! future work. This adapter returns fixture data for testing and validation.

pub mod fixtures;

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use ecl_pipeline_spec::source::SlackSourceSpec;
use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::SourceError;
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

use crate::fixtures::load_fixture_messages;

/// A stub source adapter for Slack workspaces.
///
/// Returns hardcoded fixture data rather than making real API calls.
/// Used to validate the `SourceAdapter` trait abstraction.
#[derive(Debug)]
pub struct SlackAdapter {
    /// Channel IDs to enumerate messages from.
    channels: Vec<String>,

    /// How deep to follow threads (0 = top-level only).
    thread_depth: usize,
}

impl SlackAdapter {
    /// Create a new `SlackAdapter` from a `SlackSourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::Permanent` if no channels are specified.
    pub fn new(spec: &SlackSourceSpec) -> Result<Self, SourceError> {
        if spec.channels.is_empty() {
            return Err(SourceError::Permanent {
                source: "slack".to_string(),
                message: "no channels specified in Slack source config".to_string(),
            });
        }
        Ok(Self {
            channels: spec.channels.clone(),
            thread_depth: spec.thread_depth,
        })
    }

    /// Create a `SlackAdapter` from a `SourceSpec`.
    ///
    /// Extracts the `SlackSourceSpec` from the `SourceSpec::Slack` variant.
    /// Returns an error if the spec is not a Slack source.
    pub fn from_source_spec(spec: &SourceSpec) -> Result<Self, SourceError> {
        match spec {
            SourceSpec::Slack(slack_spec) => Self::new(slack_spec),
            _ => Err(SourceError::Permanent {
                source: "slack".to_string(),
                message: "expected Slack source spec".to_string(),
            }),
        }
    }
}

#[async_trait]
impl SourceAdapter for SlackAdapter {
    fn source_kind(&self) -> &str {
        "slack"
    }

    /// Enumerate messages from fixture data, filtered by configured channels.
    ///
    /// Returns a `SourceItem` for each fixture message whose channel matches
    /// one of the configured channels. Thread messages (non-null `thread_ts`)
    /// are included only if `thread_depth > 0`.
    ///
    /// Each message's `id` is formatted as `{channel}:{timestamp}` to ensure
    /// uniqueness across channels.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        let messages = load_fixture_messages().map_err(|e| SourceError::Permanent {
            source: "slack".to_string(),
            message: format!("failed to load fixture messages: {e}"),
        })?;

        let mut items = Vec::new();

        for msg in &messages {
            // Filter by configured channels.
            if !self.channels.contains(&msg.channel) {
                continue;
            }

            // Filter thread messages based on thread_depth.
            if msg.thread_ts.is_some() && self.thread_depth == 0 {
                continue;
            }

            let id = format!("{}:{}", msg.channel, msg.timestamp);
            let display_name = format!(
                "{} in #{} at {}",
                msg.author_name, msg.channel, msg.ts_datetime
            );

            let modified_at = msg
                .ts_datetime
                .parse::<DateTime<Utc>>()
                .ok();

            // Use a simple hash of the message text as source_hash.
            let source_hash = Some(format!("{:x}", blake3_hash_bytes(msg.text.as_bytes())));

            items.push(SourceItem {
                id,
                display_name,
                mime_type: "text/plain".to_string(),
                path: format!("{}/{}", msg.channel, msg.timestamp),
                modified_at,
                source_hash,
            });
        }

        // Sort for deterministic ordering.
        items.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(items)
    }

    /// Fetch the full content of a single Slack message from fixture data.
    ///
    /// Looks up the message by parsing the `channel:timestamp` ID format,
    /// then returns the message text as content bytes.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let messages = load_fixture_messages().map_err(|e| SourceError::Permanent {
            source: "slack".to_string(),
            message: format!("failed to load fixture messages: {e}"),
        })?;

        // Parse the item ID to extract channel and timestamp.
        let (channel, timestamp) = item.id.split_once(':').ok_or_else(|| {
            SourceError::Permanent {
                source: "slack".to_string(),
                message: format!("invalid item ID format (expected channel:timestamp): {}", item.id),
            }
        })?;

        // Find the matching message.
        let msg = messages
            .iter()
            .find(|m| m.channel == channel && m.timestamp == timestamp)
            .ok_or_else(|| SourceError::NotFound {
                source: "slack".to_string(),
                item_id: item.id.clone(),
            })?;

        let content = msg.text.as_bytes().to_vec();
        let content_hash = Blake3Hash::new(format!(
            "{:x}",
            blake3_hash_bytes(&content)
        ));

        let modified_at = msg.ts_datetime.parse::<DateTime<Utc>>().ok();

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "channel".to_string(),
            serde_json::Value::String(msg.channel.clone()),
        );
        metadata.insert(
            "author".to_string(),
            serde_json::Value::String(msg.author.clone()),
        );
        metadata.insert(
            "author_name".to_string(),
            serde_json::Value::String(msg.author_name.clone()),
        );
        metadata.insert(
            "timestamp".to_string(),
            serde_json::Value::String(msg.timestamp.clone()),
        );
        if let Some(ref thread_ts) = msg.thread_ts {
            metadata.insert(
                "thread_ts".to_string(),
                serde_json::Value::String(thread_ts.clone()),
            );
        }

        let provenance = ItemProvenance {
            source_kind: "slack".to_string(),
            metadata,
            source_modified: modified_at,
            extracted_at: Utc::now(),
        };

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: "text/plain".to_string(),
            provenance,
            content_hash,
        })
    }
}

/// Compute a blake3 hash of the given bytes.
///
/// Helper to avoid depending on blake3 directly -- uses a simple wrapper
/// that returns the hash object for formatting.
fn blake3_hash_bytes(data: &[u8]) -> blake3::Hash {
    blake3::hash(data)
}
```

**Note on `blake3`:** The `blake3` crate must be added to the crate's
`[dependencies]` in `Cargo.toml`:

```toml
blake3 = { workspace = true }
```

Update the Cargo.toml in section 5 to include `blake3`:

```toml
[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
blake3 = { workspace = true }
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-adapter-slack/` directory structure
- [ ] Create `Cargo.toml` (from section 5, including `blake3`)
- [ ] Create minimal `src/lib.rs` with just `//! Stub Slack adapter crate.`
- [ ] Add `"crates/ecl-adapter-slack"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-adapter-slack` -- must pass
- [ ] Commit: `feat(ecl-adapter-slack): scaffold crate`

### Step 2: Fixture file and fixture loader

- [ ] Create `crates/ecl-adapter-slack/fixtures/messages.json` (from section 6)
- [ ] Create `src/fixtures.rs` with `SlackMessage` struct and `load_fixture_messages()` (from section 6)
- [ ] Add `pub mod fixtures;` to `lib.rs`
- [ ] Write tests:
  - `test_load_fixture_messages_returns_five_messages` -- verify fixture loads and has 5 messages
  - `test_load_fixture_messages_has_correct_channels` -- verify messages have expected channel IDs
  - `test_load_fixture_messages_thread_messages_have_thread_ts` -- verify thread replies have `thread_ts` set
  - `test_slack_message_serde_roundtrip` -- serialize and deserialize a `SlackMessage`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): add fixture data and loader`

### Step 3: SlackAdapter construction

- [ ] Implement `SlackAdapter::new()` and `SlackAdapter::from_source_spec()` in `lib.rs` (from section 6)
- [ ] Write tests:
  - `test_new_with_valid_spec_succeeds` -- construct with channels, verify fields
  - `test_new_with_empty_channels_returns_error` -- empty channels vec returns `SourceError::Permanent`
  - `test_from_source_spec_slack_variant_succeeds` -- construct from `SourceSpec::Slack`
  - `test_from_source_spec_filesystem_variant_returns_error` -- non-Slack variant returns error
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): add SlackAdapter construction`

### Step 4: SourceAdapter trait implementation -- source_kind()

- [ ] Implement `SourceAdapter for SlackAdapter` with `source_kind()` returning `"slack"`
- [ ] Write test: `test_source_kind_returns_slack` -- verify return value
- [ ] Stub `enumerate()` and `fetch()` to return empty vec and `NotFound` respectively
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): implement source_kind()`

### Step 5: SourceAdapter::enumerate() implementation

- [ ] Implement `enumerate()` as defined in section 6 -- loads fixture messages, filters by channels and thread_depth
- [ ] Write tests:
  - `test_enumerate_returns_items_for_configured_channels` -- configure one channel, verify only matching messages returned
  - `test_enumerate_filters_thread_messages_when_depth_zero` -- thread_depth=0 excludes thread replies
  - `test_enumerate_includes_thread_messages_when_depth_nonzero` -- thread_depth=1 includes replies
  - `test_enumerate_items_are_sorted_by_id` -- verify deterministic ordering
  - `test_enumerate_item_id_format` -- verify `{channel}:{timestamp}` format
  - `test_enumerate_item_has_source_hash` -- verify source_hash is populated
  - `test_enumerate_item_mime_type_is_text_plain` -- verify MIME type
  - `test_enumerate_no_matching_channels_returns_empty` -- channels not in fixture returns empty vec
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): implement enumerate()`

### Step 6: SourceAdapter::fetch() implementation

- [ ] Implement `fetch()` as defined in section 6 -- looks up message by ID, returns content
- [ ] Write tests:
  - `test_fetch_returns_message_content` -- content bytes match message text
  - `test_fetch_provenance_has_slack_metadata` -- provenance includes channel, author, timestamp
  - `test_fetch_provenance_thread_ts_present_for_replies` -- thread_ts in metadata for reply messages
  - `test_fetch_content_hash_is_blake3` -- verify hash is correct blake3 of content
  - `test_fetch_unknown_item_returns_not_found` -- item with bad ID returns `SourceError::NotFound`
  - `test_fetch_invalid_id_format_returns_error` -- ID without colon returns error
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): implement fetch()`

### Step 7: Trait compatibility validation

- [ ] Write test: `test_slack_adapter_is_object_safe` -- verify `Arc<dyn SourceAdapter>` works with `SlackAdapter`
- [ ] Write test: `test_slack_adapter_implements_send_sync` -- static assertion
- [ ] Write test: `test_slack_adapter_implements_debug` -- static assertion
- [ ] Write test: `test_enumerate_then_fetch_roundtrip` -- enumerate all items, fetch each, verify content matches fixture
- [ ] Verify NO changes were made to `SourceAdapter` trait, `Stage` trait, `SourceItem`, `ExtractedDocument`, or any `ecl-pipeline-topo` types
- [ ] Commit: `feat(ecl-adapter-slack): verify trait compatibility`

### Step 8: Mixed-source integration test

This is the **most important test** in this milestone. It validates that two
different adapter types work together in a single pipeline.

- [ ] Create an integration test (in the `#[cfg(test)]` module or a separate `tests/` directory) with the following TOML config:

```toml
name = "mixed-source-validation"
version = 1
output_dir = "{tempdir}/output"

[sources.local-files]
kind = "filesystem"
root = "{tempdir}/test-data"

[sources.team-slack]
kind = "slack"
credentials = { type = "env", env = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE"]

[stages.fetch-files]
adapter = "extract"
source = "local-files"
resources = { creates = ["raw-files"] }

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = { creates = ["raw-messages"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-files", "raw-messages"] }
```

- [ ] Write test: `test_mixed_source_pipeline_parses_both_sources` -- parse the TOML, verify both sources exist with correct kinds
- [ ] Write test: `test_mixed_source_both_adapters_enumerate` -- create both adapters, enumerate from each, verify items from both
- [ ] Write test: `test_mixed_source_both_adapters_fetch` -- enumerate and fetch from both adapters, verify content
- [ ] Write test: `test_mixed_source_resource_scheduling_parallel_fetches` -- parse the spec, build the resource graph, verify `fetch-files` and `fetch-slack` are in the same batch (both create independent resources)
- [ ] Write test: `test_mixed_source_emit_receives_from_both` -- verify the emit stage reads resources from both fetch stages
- [ ] Write test: `test_mixed_source_no_trait_changes_needed` -- this is a documentation test: a comment block asserting that `SlackAdapter` implements `SourceAdapter` with the same trait signature used by `FilesystemAdapter`, with no modifications to `ecl-pipeline-topo`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-slack): add mixed-source integration tests`

### Step 9: Coverage and final polish

- [ ] Run `make test` -- all tests pass
- [ ] Run `make lint` -- no warnings
- [ ] Run `make format` -- no changes
- [ ] Verify all public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] Check code coverage, achieve >= 95%
- [ ] Verify no compiler warnings
- [ ] Commit: `feat(ecl-adapter-slack): final polish and coverage`

## 8. Test Fixtures

### Fixture File: `crates/ecl-adapter-slack/fixtures/messages.json`

See section 6 for the complete fixture file. It contains 5 messages:

1. **Top-level message** in `C01234ABCDE` from Alice (Q1 roadmap update)
2. **Thread reply** in `C01234ABCDE` from Bob (requesting dependency chart)
3. **Thread reply** in `C01234ABCDE` from Alice (response to Bob)
4. **Top-level message** in `C05678FGHIJ` from Charlie (meeting reminder)
5. **Top-level message** in `C05678FGHIJ` from Diana (CI pipeline update)

### Mixed-Source TOML (for integration tests)

```toml
name = "mixed-source-validation"
version = 1
output_dir = "./output/mixed-test"

[sources.local-files]
kind = "filesystem"
root = "/tmp/test-data"

[sources.team-slack]
kind = "slack"
credentials = { type = "env", env = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE"]

[stages.fetch-files]
adapter = "extract"
source = "local-files"
resources = { creates = ["raw-files"] }

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = { creates = ["raw-messages"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-files", "raw-messages"] }
```

### Minimal Slack-Only TOML (for unit tests)

```toml
name = "slack-only-test"
version = 1
output_dir = "./output/slack-test"

[sources.team-slack]
kind = "slack"
credentials = { type = "env", env = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE"]

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = { creates = ["raw-messages"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-messages"] }
```

### Test Data for Filesystem Source (create at test time)

Integration tests that use the mixed-source config should create temporary
files in a `tempdir`:

```
{tempdir}/test-data/
  doc1.txt    -> "Hello from filesystem"
  doc2.txt    -> "Another filesystem document"
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_load_fixture_messages_returns_five_messages` | `fixtures` | Fixture file loads and contains 5 messages |
| `test_load_fixture_messages_has_correct_channels` | `fixtures` | Messages have expected channel IDs (`C01234ABCDE`, `C05678FGHIJ`) |
| `test_load_fixture_messages_thread_messages_have_thread_ts` | `fixtures` | Thread replies (messages 2, 3) have `thread_ts` set |
| `test_slack_message_serde_roundtrip` | `fixtures` | `SlackMessage` serializes/deserializes correctly |
| `test_new_with_valid_spec_succeeds` | `lib` | Constructor succeeds with valid `SlackSourceSpec` |
| `test_new_with_empty_channels_returns_error` | `lib` | Constructor returns `SourceError::Permanent` for empty channels |
| `test_from_source_spec_slack_variant_succeeds` | `lib` | `from_source_spec` with `SourceSpec::Slack` works |
| `test_from_source_spec_filesystem_variant_returns_error` | `lib` | `from_source_spec` with non-Slack variant returns error |
| `test_source_kind_returns_slack` | `lib` | `source_kind()` returns `"slack"` |
| `test_enumerate_returns_items_for_configured_channels` | `lib` | Only messages from configured channels are returned |
| `test_enumerate_filters_thread_messages_when_depth_zero` | `lib` | `thread_depth=0` excludes thread replies |
| `test_enumerate_includes_thread_messages_when_depth_nonzero` | `lib` | `thread_depth=1` includes thread replies |
| `test_enumerate_items_are_sorted_by_id` | `lib` | Items are returned in deterministic sorted order |
| `test_enumerate_item_id_format` | `lib` | Item IDs use `{channel}:{timestamp}` format |
| `test_enumerate_item_has_source_hash` | `lib` | `source_hash` field is populated |
| `test_enumerate_item_mime_type_is_text_plain` | `lib` | MIME type is `"text/plain"` |
| `test_enumerate_no_matching_channels_returns_empty` | `lib` | Non-matching channels yield empty vec |
| `test_fetch_returns_message_content` | `lib` | Content bytes match the fixture message text |
| `test_fetch_provenance_has_slack_metadata` | `lib` | Provenance metadata includes channel, author, timestamp |
| `test_fetch_provenance_thread_ts_present_for_replies` | `lib` | Thread replies have `thread_ts` in provenance metadata |
| `test_fetch_content_hash_is_blake3` | `lib` | Content hash matches blake3 of content bytes |
| `test_fetch_unknown_item_returns_not_found` | `lib` | Unknown item ID returns `SourceError::NotFound` |
| `test_fetch_invalid_id_format_returns_error` | `lib` | ID without colon separator returns `SourceError::Permanent` |
| `test_slack_adapter_is_object_safe` | `lib` | `Arc<dyn SourceAdapter>` compiles with `SlackAdapter` |
| `test_slack_adapter_implements_send_sync` | `lib` | Static assertion: `SlackAdapter: Send + Sync` |
| `test_slack_adapter_implements_debug` | `lib` | Static assertion: `SlackAdapter: Debug` |
| `test_enumerate_then_fetch_roundtrip` | `lib` | Enumerate all, fetch each, content matches fixture |
| `test_mixed_source_pipeline_parses_both_sources` | `integration` | TOML parses with both filesystem and slack sources |
| `test_mixed_source_both_adapters_enumerate` | `integration` | Both adapters enumerate items successfully |
| `test_mixed_source_both_adapters_fetch` | `integration` | Both adapters fetch content successfully |
| `test_mixed_source_resource_scheduling_parallel_fetches` | `integration` | Resource graph places both fetches in same batch |
| `test_mixed_source_emit_receives_from_both` | `integration` | Emit stage reads resources from both sources |
| `test_mixed_source_no_trait_changes_needed` | `integration` | Documentation test confirming no trait modifications |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-adapter-slack` passes
- [ ] `cargo test -p ecl-adapter-slack` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] No trait signature changes were needed (if any were, document what and why)
- [ ] `SlackAdapter` implements the exact same `SourceAdapter` trait as `FilesystemAdapter`
- [ ] Mixed-source integration test passes with both filesystem and Slack adapters in one pipeline

### What NOT to Do

- Do NOT implement real Slack API calls -- this is a STUB adapter using fixture data
- Do NOT modify any existing trait signatures (`SourceAdapter`, `Stage`, `SourceItem`, `ExtractedDocument`, etc.) -- if changes are needed, document them but exhaust all alternatives first
- Do NOT modify the pipeline runner (`ecl-pipeline` crate)
- Do NOT modify `ecl-pipeline-topo` or any of its types
- Do NOT modify `ecl-adapter-fs` or `ecl-stages`
- Do NOT add CLI commands
- Do NOT implement authentication or credential resolution (the stub ignores credentials)
- Do NOT implement pagination, rate limiting, or real HTTP calls
- Do NOT add any crates beyond `ecl-adapter-slack`
