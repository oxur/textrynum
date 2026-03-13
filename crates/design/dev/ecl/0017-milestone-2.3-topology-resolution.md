# Milestone 2.3: Topology Resolution

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-pipeline-topo):` for topology crate changes,
`feat(ecl-pipeline):` for engine crate changes

## 1. Goal

Complete the topology resolution system by implementing two things:

1. **Adapter and Stage registries** (in `ecl-pipeline`) -- runtime maps from
   string kind/adapter names to factory functions that produce concrete
   `SourceAdapter` and `Stage` implementations.
2. **Full `PipelineTopology::resolve()`** (in `ecl-pipeline-topo`) -- replace
   the `todo!()` stub in `resolve.rs` with the complete implementation that
   hashes the spec, resolves sources via the adapter registry, resolves stages
   via the stage registry, merges retry policies, builds the resource graph,
   computes the schedule, and creates the output directory.

When done:

- `AdapterRegistry` maps source `kind` strings to factory functions that
  produce `Arc<dyn SourceAdapter>`.
- `StageRegistry` maps stage `adapter` strings to factory functions that
  produce `Arc<dyn Stage>`.
- `PipelineTopology::resolve()` takes a `PipelineSpec` plus both registries
  and returns a fully resolved `PipelineTopology` -- no more `todo!()`.
- Retry merging logic is correct: stage-level retry overrides global defaults
  field-by-field, and `RetrySpec` millisecond values are converted to
  `Duration` in `RetryPolicy`.
- All tests pass, lint passes, coverage >= 95%.

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
4. `assets/ai/ai-rust/guides/02-api-design.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:
- `crates/ecl-core/Cargo.toml` -- Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` -- Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` -- Module structure, re-exports, doc comments
- `crates/ecl-core/src/llm/provider.rs` -- `async_trait` pattern for object-safe traits

## 3. Prior Art / Dependencies

This milestone MODIFIES two existing crates. It does NOT create a new crate.

### Crate 1: `ecl-pipeline-topo` (Milestone 1.3 -- exists)

This crate already contains:

- `src/lib.rs` -- `PipelineTopology`, `ResolvedStage`, `RetryPolicy`,
  `ConditionExpr`, `Blake3Hash`, `StageId` re-exports
- `src/error.rs` -- `ResolveError` (with variants: `SerializeError`,
  `UnknownSource`, `MissingResource`, `CycleDetected`, `DuplicateCreator`,
  `Io`, `UnknownAdapter`), `SourceError`, `StageError`
- `src/resolve.rs` -- **STUB** with `todo!()` -- this is what we complete
- `src/resource_graph.rs` -- `ResourceGraph::build()`,
  `validate_no_missing_inputs()`, `validate_no_cycles()`, `compute_schedule()`
- `src/schedule.rs` -- Kahn's algorithm topological sort
- `src/traits.rs` -- `SourceAdapter` trait, `Stage` trait, `StageContext`,
  `PipelineItem`, `SourceItem`, `ExtractedDocument`

The current `resolve.rs` stub:

```rust
//! Topology resolution: converting a PipelineSpec into a PipelineTopology.
//!
//! **STUB:** This module contains only the function signature in this
//! milestone. The full implementation (resolving adapters, building the
//! resource graph, computing the schedule) is implemented in milestone 2.3.

use ecl_pipeline_spec::PipelineSpec;

use crate::error::ResolveError;
use crate::PipelineTopology;

/// Resolve a `PipelineSpec` into a `PipelineTopology`.
pub async fn resolve(_spec: PipelineSpec) -> Result<PipelineTopology, ResolveError> {
    // Full implementation deferred to milestone 2.3.
    todo!("Full topology resolution implemented in milestone 2.3")
}
```

**Key types already defined in `ecl-pipeline-topo/src/lib.rs`:**

```rust
/// The resolved pipeline, ready to execute.
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

impl RetryPolicy {
    pub fn from_spec(spec: &ecl_pipeline_spec::RetrySpec) -> Self {
        Self {
            max_attempts: spec.max_attempts,
            initial_backoff: Duration::from_millis(spec.initial_backoff_ms),
            backoff_multiplier: spec.backoff_multiplier,
            max_backoff: Duration::from_millis(spec.max_backoff_ms),
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionExpr(String);

impl ConditionExpr {
    pub fn new(expr: impl Into<String>) -> Self { Self(expr.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}
```

### Crate 2: `ecl-pipeline` (Milestone 2.2 -- exists)

This crate is the facade/engine crate. It already contains the `PipelineRunner`
and execution logic. We are adding the registry types here because the engine
crate is where concrete adapters and stages are wired together.

**Note:** If `ecl-pipeline` does not yet exist when you start this milestone,
you must create it. See section 5 for the `Cargo.toml` definition.

### From `ecl-pipeline-spec` (Milestone 1.1 -- exists)

```rust
// Key types used by this milestone:

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefaultsSpec {
    pub concurrency: usize,        // default: 4
    pub retry: RetrySpec,
    pub checkpoint: CheckpointStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrySpec {
    pub max_attempts: u32,         // default: 3
    pub initial_backoff_ms: u64,   // default: 1000
    pub backoff_multiplier: f64,   // default: 2.0
    pub max_backoff_ms: u64,       // default: 30_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    #[serde(rename = "google_drive")] GoogleDrive(GoogleDriveSourceSpec),
    #[serde(rename = "slack")]        Slack(SlackSourceSpec),
    #[serde(rename = "filesystem")]   Filesystem(FilesystemSourceSpec),
}

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
```

### From `ecl-pipeline-state` (Milestone 1.2 -- exists)

```rust
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
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/ecl-pipeline/Cargo.toml` | Create (if missing) or Modify | Add `blake3` dependency |
| `crates/ecl-pipeline/src/registry.rs` | Create | `AdapterRegistry`, `StageRegistry` |
| `crates/ecl-pipeline/src/lib.rs` | Modify | Add `pub mod registry;` and re-exports |
| `crates/ecl-pipeline-topo/Cargo.toml` | Modify | Add `toml` dependency (needed for serializing spec to hash) |
| `crates/ecl-pipeline-topo/src/resolve.rs` | Modify | **Replace stub** with full implementation |
| `crates/ecl-pipeline-topo/src/lib.rs` | Modify | Add `resolve_retry_policy()` helper (if not placing it in resolve.rs) |

## 5. Cargo.toml

### `ecl-pipeline/Cargo.toml` (Create if not present)

If `crates/ecl-pipeline` does not already exist, create it with this
`Cargo.toml`. If it already exists (from milestone 2.2), just ensure `blake3`
is in the dependencies.

```toml
[package]
name = "ecl-pipeline"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "ECL pipeline runner: engine, registries, and facade"

[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

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

Also ensure `"crates/ecl-pipeline"` is in the root `Cargo.toml`
`[workspace] members` list (it should already be there from 2.2).

### `ecl-pipeline-topo/Cargo.toml` Modification

Add `toml` to the `[dependencies]` section (needed for serializing the spec
to compute the blake3 hash):

```toml
toml = { workspace = true }
```

This dependency should be added alongside the existing dependencies.

## 6. Type Definitions and Signatures

### `ecl-pipeline/src/registry.rs`

```rust
//! Adapter and stage registries for runtime resolution.
//!
//! These registries map string identifiers (from TOML configuration) to
//! factory functions that produce concrete trait implementations. This is
//! the bridge between the declarative specification layer and the concrete
//! topology layer.

use std::collections::BTreeMap;
use std::sync::Arc;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::stage::StageSpec;
use ecl_pipeline_topo::error::ResolveError;
use ecl_pipeline_topo::{SourceAdapter, Stage};

/// Type alias for source adapter factory functions.
///
/// A factory takes a `&SourceSpec` and returns a concrete `SourceAdapter`
/// implementation wrapped in `Arc` for shared ownership in the topology.
pub type AdapterFactory =
    Box<dyn Fn(&SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + Send + Sync>;

/// Type alias for stage handler factory functions.
///
/// A factory takes a `&StageSpec` and returns a concrete `Stage`
/// implementation wrapped in `Arc` for shared ownership in the topology.
pub type StageFactory =
    Box<dyn Fn(&StageSpec) -> Result<Arc<dyn Stage>, ResolveError> + Send + Sync>;

/// Registry of source adapter factories, keyed by source `kind` string.
///
/// The `kind` string matches the `SourceSpec` variant tag in TOML:
/// - `"google_drive"` -> Google Drive adapter
/// - `"slack"` -> Slack adapter
/// - `"filesystem"` -> Filesystem adapter
///
/// # Example
///
/// ```ignore
/// let mut registry = AdapterRegistry::new();
/// registry.register("filesystem", Box::new(|spec| {
///     Ok(Arc::new(FilesystemAdapter::from_spec(spec)?))
/// }));
/// ```
#[derive(Default)]
pub struct AdapterRegistry {
    factories: BTreeMap<String, AdapterFactory>,
}

impl AdapterRegistry {
    /// Create an empty adapter registry.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Register a factory function for a source kind.
    ///
    /// If a factory was already registered for this kind, it is replaced.
    pub fn register(&mut self, kind: &str, factory: AdapterFactory) {
        self.factories.insert(kind.to_string(), factory);
    }

    /// Look up and invoke the factory for the given source kind.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if no factory is registered
    /// for the given kind. The `stage` field in the error will contain the
    /// source name (passed as `source_name`), and the `adapter` field will
    /// contain the kind string.
    pub fn resolve(
        &self,
        kind: &str,
        source_name: &str,
        spec: &SourceSpec,
    ) -> Result<Arc<dyn SourceAdapter>, ResolveError> {
        let factory = self.factories.get(kind).ok_or_else(|| {
            ResolveError::UnknownAdapter {
                stage: source_name.to_string(),
                adapter: kind.to_string(),
            }
        })?;
        factory(spec)
    }

    /// Returns true if a factory is registered for the given kind.
    pub fn contains(&self, kind: &str) -> bool {
        self.factories.contains_key(kind)
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl std::fmt::Debug for AdapterRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterRegistry")
            .field("registered_kinds", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Registry of stage handler factories, keyed by stage `adapter` string.
///
/// The `adapter` string matches `StageSpec.adapter` in TOML:
/// - `"extract"` -> Extract stage (delegates to SourceAdapter)
/// - `"normalize"` -> Normalization stage
/// - `"emit"` -> Emit/output stage
///
/// # Example
///
/// ```ignore
/// let mut registry = StageRegistry::new();
/// registry.register("extract", Box::new(|spec| {
///     Ok(Arc::new(ExtractStage::from_spec(spec)?))
/// }));
/// ```
#[derive(Default)]
pub struct StageRegistry {
    factories: BTreeMap<String, StageFactory>,
}

impl StageRegistry {
    /// Create an empty stage registry.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Register a factory function for a stage adapter name.
    ///
    /// If a factory was already registered for this adapter, it is replaced.
    pub fn register(&mut self, adapter: &str, factory: StageFactory) {
        self.factories.insert(adapter.to_string(), factory);
    }

    /// Look up and invoke the factory for the given stage adapter name.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if no factory is registered
    /// for the given adapter name.
    pub fn resolve(
        &self,
        adapter: &str,
        stage_name: &str,
        spec: &StageSpec,
    ) -> Result<Arc<dyn Stage>, ResolveError> {
        let factory = self.factories.get(adapter).ok_or_else(|| {
            ResolveError::UnknownAdapter {
                stage: stage_name.to_string(),
                adapter: adapter.to_string(),
            }
        })?;
        factory(spec)
    }

    /// Returns true if a factory is registered for the given adapter.
    pub fn contains(&self, adapter: &str) -> bool {
        self.factories.contains_key(adapter)
    }

    /// Returns the number of registered factories.
    pub fn len(&self) -> usize {
        self.factories.len()
    }

    /// Returns true if no factories are registered.
    pub fn is_empty(&self) -> bool {
        self.factories.is_empty()
    }
}

impl std::fmt::Debug for StageRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StageRegistry")
            .field("registered_adapters", &self.factories.keys().collect::<Vec<_>>())
            .finish()
    }
}
```

### `ecl-pipeline-topo/src/resolve.rs` (Full Implementation)

Replace the entire stub with:

```rust
//! Topology resolution: converting a PipelineSpec into a PipelineTopology.
//!
//! This module implements the core resolution logic:
//! 1. Hash the spec for config drift detection (blake3).
//! 2. Resolve each source into a concrete adapter via the adapter registry.
//! 3. Resolve each stage into a concrete handler via the stage registry.
//! 4. Merge stage-level retry overrides with global defaults.
//! 5. Build the resource graph and validate.
//! 6. Compute the parallel execution schedule.
//! 7. Create the output directory (async).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ecl_pipeline_spec::{DefaultsSpec, PipelineSpec, RetrySpec};
use ecl_pipeline_spec::source::SourceSpec;
use ecl_pipeline_spec::stage::StageSpec;
use ecl_pipeline_state::{Blake3Hash, StageId};

use crate::error::ResolveError;
use crate::resource_graph::ResourceGraph;
use crate::{ConditionExpr, PipelineTopology, ResolvedStage, RetryPolicy, SourceAdapter, Stage};

/// Type alias for source adapter factory functions.
///
/// Used by `resolve()` to convert `SourceSpec` values into concrete
/// `SourceAdapter` implementations at init time.
pub type AdapterFactory =
    Box<dyn Fn(&SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError> + Send + Sync>;

/// Type alias for stage handler factory functions.
///
/// Used by `resolve()` to convert `StageSpec` values into concrete
/// `Stage` implementations at init time.
pub type StageFactory =
    Box<dyn Fn(&StageSpec) -> Result<Arc<dyn Stage>, ResolveError> + Send + Sync>;

/// Resolve a `PipelineSpec` into a `PipelineTopology`.
///
/// This is the main entry point for topology construction:
/// 1. Hash the spec for config drift detection.
/// 2. Resolve each source into a concrete `SourceAdapter` via the adapter
///    registry lookup function.
/// 3. Resolve each stage into a concrete `Stage` handler via the stage
///    registry lookup function.
/// 4. Merge retry policies (stage override > global default).
/// 5. Build the resource graph and validate (no missing inputs, no cycles).
/// 6. Compute the parallel schedule.
/// 7. Create the output directory.
///
/// # Arguments
///
/// * `spec` -- The parsed pipeline specification.
/// * `adapter_lookup` -- A function that takes (source_name, &SourceSpec) and
///   returns a concrete adapter. Typically backed by an `AdapterRegistry`.
/// * `stage_lookup` -- A function that takes (stage_name, &StageSpec) and
///   returns a concrete stage handler. Typically backed by a `StageRegistry`.
///
/// # Errors
///
/// Returns `ResolveError` if any step fails (unknown adapter, cycle,
/// missing resource, I/O error, etc.).
pub async fn resolve<F, G>(
    spec: PipelineSpec,
    adapter_lookup: F,
    stage_lookup: G,
) -> Result<PipelineTopology, ResolveError>
where
    F: Fn(&str, &SourceSpec) -> Result<Arc<dyn SourceAdapter>, ResolveError>,
    G: Fn(&str, &StageSpec) -> Result<Arc<dyn Stage>, ResolveError>,
{
    // 1. Hash the spec for config drift detection.
    let spec_bytes = toml::to_string(&spec).map_err(|e| ResolveError::SerializeError {
        message: e.to_string(),
    })?;
    let spec_hash = Blake3Hash::new(blake3::hash(spec_bytes.as_bytes()).to_hex().to_string());
    let spec = Arc::new(spec);

    // 2. Resolve each source into a concrete adapter.
    let mut sources: BTreeMap<String, Arc<dyn SourceAdapter>> = BTreeMap::new();
    for (name, source_spec) in &spec.sources {
        let kind = source_kind(source_spec);
        let adapter = adapter_lookup(name, source_spec).map_err(|e| match e {
            ResolveError::UnknownAdapter { .. } => ResolveError::UnknownAdapter {
                stage: name.clone(),
                adapter: kind.to_string(),
            },
            other => other,
        })?;
        sources.insert(name.clone(), adapter);
    }

    // 3. Resolve each stage into a concrete handler.
    let mut stages: BTreeMap<String, ResolvedStage> = BTreeMap::new();
    for (name, stage_spec) in &spec.stages {
        let handler = stage_lookup(name, stage_spec)?;
        let resolved = resolve_stage(name, stage_spec, handler, &spec.defaults)?;
        stages.insert(name.clone(), resolved);
    }

    // 4. Build the resource graph and validate.
    let resource_graph = ResourceGraph::build(&spec.stages)?;
    resource_graph.validate_no_missing_inputs()?;
    resource_graph.validate_no_cycles()?;

    // 5. Compute the parallel schedule.
    let schedule = resource_graph.compute_schedule()?;

    // 6. Create output directory (async to avoid blocking the runtime;
    //    see AP-18: sync I/O in async context).
    let output_dir = spec.output_dir.clone();
    tokio::fs::create_dir_all(&output_dir).await.map_err(ResolveError::Io)?;

    Ok(PipelineTopology {
        spec,
        spec_hash,
        sources,
        stages,
        schedule,
        output_dir,
    })
}

/// Extract the `kind` string from a `SourceSpec` variant.
///
/// This maps each variant to the string used as the registry key:
/// - `SourceSpec::GoogleDrive(..)` -> `"google_drive"`
/// - `SourceSpec::Slack(..)` -> `"slack"`
/// - `SourceSpec::Filesystem(..)` -> `"filesystem"`
fn source_kind(spec: &SourceSpec) -> &'static str {
    match spec {
        SourceSpec::GoogleDrive(_) => "google_drive",
        SourceSpec::Slack(_) => "slack",
        SourceSpec::Filesystem(_) => "filesystem",
    }
}

/// Resolve a single stage: create the `ResolvedStage` by merging the
/// stage-level configuration with global defaults.
fn resolve_stage(
    name: &str,
    stage_spec: &StageSpec,
    handler: Arc<dyn Stage>,
    defaults: &DefaultsSpec,
) -> Result<ResolvedStage, ResolveError> {
    // Merge retry: stage override > global default.
    let retry = resolve_retry_policy(stage_spec.retry.as_ref(), &defaults.retry);

    // Resolve timeout from seconds to Duration.
    let timeout = stage_spec.timeout_secs.map(Duration::from_secs);

    // Resolve condition expression.
    let condition = stage_spec.condition.as_ref().map(|s| ConditionExpr::new(s));

    Ok(ResolvedStage {
        id: StageId::new(name),
        handler,
        retry,
        skip_on_error: stage_spec.skip_on_error,
        timeout,
        source: stage_spec.source.clone(),
        condition,
    })
}

/// Merge a stage-level `RetrySpec` override with the global default
/// `RetrySpec`, producing a resolved `RetryPolicy` with `Duration` values.
///
/// If `stage_retry` is `Some`, its values are used. If `None`, the global
/// defaults are used. This is a wholesale override (the entire `RetrySpec`
/// from the stage replaces the global default), not a field-by-field merge,
/// because `RetrySpec` fields all have their own serde defaults -- a stage
/// that specifies `retry = { max_attempts = 5 }` in TOML will get
/// `max_attempts = 5` with default values for the other fields, which is
/// the correct behavior.
///
/// # Arguments
///
/// * `stage_retry` -- The stage's retry override, if any.
/// * `global_retry` -- The global default retry spec.
pub fn resolve_retry_policy(
    stage_retry: Option<&RetrySpec>,
    global_retry: &RetrySpec,
) -> RetryPolicy {
    let spec = stage_retry.unwrap_or(global_retry);
    RetryPolicy::from_spec(spec)
}
```

### `ecl-pipeline/src/lib.rs` Modifications

Add the registry module and re-exports. If the file already exists from
milestone 2.2, add these lines:

```rust
pub mod registry;

pub use registry::{AdapterRegistry, StageRegistry};
```

If creating the crate from scratch, the full `lib.rs`:

```rust
//! ECL pipeline runner: engine, registries, and facade.
//!
//! This crate ties together the specification, topology, and state layers
//! into a runnable pipeline. It provides:
//! - `AdapterRegistry` and `StageRegistry` for runtime adapter/stage resolution
//! - `PipelineRunner` for batch execution with checkpointing (milestone 2.2)

pub mod registry;

pub use registry::{AdapterRegistry, StageRegistry};
```

## 7. Implementation Steps (TDD Order)

### Step 0: Prerequisites check

- [ ] Confirm `crates/ecl-pipeline-spec` exists and `cargo check -p ecl-pipeline-spec` passes
- [ ] Confirm `crates/ecl-pipeline-state` exists and `cargo check -p ecl-pipeline-state` passes
- [ ] Confirm `crates/ecl-pipeline-topo` exists and `cargo check -p ecl-pipeline-topo` passes
- [ ] If any prerequisite crate is missing, STOP and report. They must be implemented first (milestones 1.1, 1.2, 1.3).
- [ ] If `crates/ecl-pipeline` does not exist, create it (see section 5 for Cargo.toml, section 6 for lib.rs). Add it to the root workspace members.
- [ ] Add `toml = { workspace = true }` to `crates/ecl-pipeline-topo/Cargo.toml` dependencies (if not already present)
- [ ] Run `cargo check --workspace` -- must pass
- [ ] Commit: `chore: prepare workspace for milestone 2.3`

### Step 1: AdapterRegistry (in ecl-pipeline)

- [ ] Create `crates/ecl-pipeline/src/registry.rs` with `AdapterRegistry` struct (from section 6)
  - Include: `AdapterFactory` type alias, `new()`, `register()`, `resolve()`, `contains()`, `len()`, `is_empty()`, `Debug` impl
- [ ] Add `pub mod registry;` and `pub use registry::AdapterRegistry;` to `lib.rs`
- [ ] Write tests in `registry.rs`:
  - `test_adapter_registry_new_is_empty` -- new registry has `len() == 0` and `is_empty() == true`
  - `test_adapter_registry_register_and_contains` -- after registering "filesystem", `contains("filesystem")` is true
  - `test_adapter_registry_resolve_unknown_kind_returns_error` -- resolving unregistered kind returns `ResolveError::UnknownAdapter`
  - `test_adapter_registry_resolve_calls_factory` -- register a mock factory, resolve returns the mock adapter
  - `test_adapter_registry_debug_shows_registered_kinds` -- `format!("{:?}", registry)` contains the registered kind names
- [ ] Run `cargo test -p ecl-pipeline` -- must pass
- [ ] Commit: `feat(ecl-pipeline): add AdapterRegistry`

### Step 2: StageRegistry (in ecl-pipeline)

- [ ] Add `StageRegistry` struct to `registry.rs` (from section 6)
  - Include: `StageFactory` type alias, `new()`, `register()`, `resolve()`, `contains()`, `len()`, `is_empty()`, `Debug` impl
- [ ] Add `pub use registry::StageRegistry;` to `lib.rs`
- [ ] Write tests in `registry.rs`:
  - `test_stage_registry_new_is_empty` -- new registry has `len() == 0`
  - `test_stage_registry_register_and_contains` -- after registering "extract", `contains("extract")` is true
  - `test_stage_registry_resolve_unknown_adapter_returns_error` -- resolving unregistered adapter returns `ResolveError::UnknownAdapter`
  - `test_stage_registry_resolve_calls_factory` -- register a mock factory, resolve returns the mock stage
  - `test_stage_registry_register_overwrites` -- registering the same adapter twice replaces the first factory
- [ ] Run `cargo test -p ecl-pipeline` -- must pass
- [ ] Commit: `feat(ecl-pipeline): add StageRegistry`

### Step 3: resolve_retry_policy helper (in ecl-pipeline-topo)

- [ ] Add the `resolve_retry_policy()` function to `crates/ecl-pipeline-topo/src/resolve.rs`
  - Keep the existing stub `resolve()` function for now (we replace it in Step 4)
  - Add the `source_kind()` helper function
  - Add the `resolve_stage()` helper function
- [ ] Write tests in `resolve.rs`:
  - `test_resolve_retry_policy_uses_global_when_no_override` -- `None` stage retry -> uses global defaults
  - `test_resolve_retry_policy_uses_stage_override` -- `Some(RetrySpec{..})` -> uses stage values
  - `test_resolve_retry_policy_converts_ms_to_duration` -- verify ms->Duration conversion: 1000ms -> Duration::from_millis(1000)
  - `test_resolve_retry_policy_custom_stage_values` -- stage with max_attempts=5, initial_backoff_ms=500 -> RetryPolicy with those values
  - `test_source_kind_google_drive` -- `source_kind(&SourceSpec::GoogleDrive(..))` returns `"google_drive"`
  - `test_source_kind_slack` -- `source_kind(&SourceSpec::Slack(..))` returns `"slack"`
  - `test_source_kind_filesystem` -- `source_kind(&SourceSpec::Filesystem(..))` returns `"filesystem"`
- [ ] Run `cargo test -p ecl-pipeline-topo` -- must pass
- [ ] Commit: `feat(ecl-pipeline-topo): add resolve_retry_policy and helpers`

### Step 4: Full resolve() implementation (in ecl-pipeline-topo)

- [ ] Replace the `resolve()` stub with the full implementation (from section 6)
  - Change the function signature to accept `adapter_lookup` and `stage_lookup` closures
  - Implement all 7 steps: hash, resolve sources, resolve stages, build graph, validate, schedule, create output dir
- [ ] **Update any code in `ecl-pipeline` or other crates that calls `resolve()`** to pass the new closure arguments. If the old `resolve(spec)` signature was used by `PipelineRunner::new()`, update it to pass registry-backed closures.
- [ ] Write tests in `resolve.rs` (all `#[tokio::test]`):
  - `test_resolve_full_example_spec_returns_topology` -- parse the example TOML (from section 8), provide mock adapter/stage factories, verify the returned `PipelineTopology` has correct sources, stages, schedule, and output_dir
  - `test_resolve_unknown_source_kind_returns_error` -- provide an adapter lookup that returns `UnknownAdapter`, verify the error
  - `test_resolve_unknown_stage_adapter_returns_error` -- provide a stage lookup that returns `UnknownAdapter`, verify the error
  - `test_resolve_creates_output_directory` -- use `tempfile::tempdir()`, verify the output dir is created
  - `test_resolve_spec_hash_is_deterministic` -- resolve the same spec twice, verify the `spec_hash` is identical
  - `test_resolve_spec_hash_changes_with_spec` -- resolve two different specs, verify the hashes differ
  - `test_resolve_stages_have_correct_retry_policy` -- spec with stage retry override, verify the resolved stage has the overridden policy
  - `test_resolve_stages_use_global_retry_when_no_override` -- spec with no stage retry, verify global defaults
  - `test_resolve_stages_have_correct_timeout` -- stage with `timeout_secs = 300` -> `timeout = Some(Duration::from_secs(300))`
  - `test_resolve_stages_have_correct_skip_on_error` -- stage with `skip_on_error = true` -> resolved stage has `true`
  - `test_resolve_stages_have_correct_condition` -- stage with `condition = Some("x > 1")` -> resolved stage has `Some(ConditionExpr::new("x > 1"))`
  - `test_resolve_schedule_matches_expected` -- for the 5-stage example, verify 3 batches with correct stage grouping
- [ ] Remove the old `#[should_panic] test_resolve_stub_is_todo` test (the stub no longer exists)
- [ ] Run `cargo test -p ecl-pipeline-topo` -- must pass
- [ ] Run `cargo test --workspace` -- must pass (no regressions)
- [ ] Commit: `feat(ecl-pipeline-topo): implement full topology resolution`

### Step 5: Integration test and final polish

- [ ] Write an integration test (in `ecl-pipeline-topo/tests/` or inline) that exercises the full resolve path end-to-end:
  - Parse the example TOML from section 8
  - Create mock adapter and stage factories
  - Call `resolve()` with a temp output dir
  - Verify: spec_hash is non-empty, sources map has 2 entries, stages map has 5 entries, schedule has 3 batches, output directory was created
- [ ] Run `make test` -- all tests pass
- [ ] Run `make lint` -- no warnings
- [ ] Run `make format` -- no changes
- [ ] Verify all public items have doc comments
- [ ] Verify no compiler warnings
- [ ] Verify test coverage >= 95% per `assets/ai/CLAUDE-CODE-COVERAGE.md`
- [ ] Commit: `feat(ecl-pipeline-topo): add integration tests for topology resolution`

## 8. Test Fixtures

### Mock SourceAdapter and Mock Stage

Use these mock implementations in all tests that need to resolve sources and
stages. They are minimal implementations that satisfy the trait contracts.

```rust
use std::sync::Arc;
use async_trait::async_trait;
use ecl_pipeline_topo::{
    SourceAdapter, Stage, SourceItem, ExtractedDocument, PipelineItem, StageContext,
};
use ecl_pipeline_topo::error::{SourceError, StageError};

#[derive(Debug)]
struct MockSourceAdapter {
    kind: String,
}

impl MockSourceAdapter {
    fn new(kind: &str) -> Self {
        Self { kind: kind.to_string() }
    }
}

#[async_trait]
impl SourceAdapter for MockSourceAdapter {
    fn source_kind(&self) -> &str {
        &self.kind
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        Ok(vec![])
    }

    async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        Err(SourceError::NotFound {
            source: self.kind.clone(),
            item_id: "none".to_string(),
        })
    }
}

#[derive(Debug)]
struct MockStage {
    name: String,
}

impl MockStage {
    fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

#[async_trait]
impl Stage for MockStage {
    fn name(&self) -> &str {
        &self.name
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Ok(vec![item])
    }
}
```

### Mock Factory Helpers for resolve() Tests

```rust
use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::stage::StageSpec;
use ecl_pipeline_topo::error::ResolveError;
use ecl_pipeline_topo::{SourceAdapter, Stage};

/// Creates a mock adapter lookup function that returns MockSourceAdapter
/// for any source kind.
fn mock_adapter_lookup(
    name: &str,
    spec: &SourceSpec,
) -> Result<Arc<dyn SourceAdapter>, ResolveError> {
    let kind = match spec {
        SourceSpec::GoogleDrive(_) => "google_drive",
        SourceSpec::Slack(_) => "slack",
        SourceSpec::Filesystem(_) => "filesystem",
    };
    Ok(Arc::new(MockSourceAdapter::new(kind)))
}

/// Creates a mock stage lookup function that returns MockStage for any
/// adapter name.
fn mock_stage_lookup(
    name: &str,
    spec: &StageSpec,
) -> Result<Arc<dyn Stage>, ResolveError> {
    Ok(Arc::new(MockStage::new(&spec.adapter)))
}
```

### Minimal Valid TOML (for simple resolve tests)

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

### Full Example TOML (for comprehensive resolve tests)

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

### TOML with Stage Retry Override (for retry merge tests)

```toml
name = "retry-test"
version = 1
output_dir = "./output/retry-test"

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.fast-retry]
adapter = "extract"
source = "local"
resources = { creates = ["raw-docs"] }
retry = { max_attempts = 5, initial_backoff_ms = 500, backoff_multiplier = 1.5, max_backoff_ms = 10000 }

[stages.default-retry]
adapter = "emit"
resources = { reads = ["raw-docs"] }
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_adapter_registry_new_is_empty` | `ecl-pipeline::registry` | New registry has zero entries |
| `test_adapter_registry_register_and_contains` | `ecl-pipeline::registry` | Registration makes kind findable |
| `test_adapter_registry_resolve_unknown_kind_returns_error` | `ecl-pipeline::registry` | Unregistered kind returns `UnknownAdapter` error |
| `test_adapter_registry_resolve_calls_factory` | `ecl-pipeline::registry` | Factory function is invoked, mock adapter returned |
| `test_adapter_registry_debug_shows_registered_kinds` | `ecl-pipeline::registry` | Debug output lists registered kinds |
| `test_stage_registry_new_is_empty` | `ecl-pipeline::registry` | New registry has zero entries |
| `test_stage_registry_register_and_contains` | `ecl-pipeline::registry` | Registration makes adapter findable |
| `test_stage_registry_resolve_unknown_adapter_returns_error` | `ecl-pipeline::registry` | Unregistered adapter returns `UnknownAdapter` error |
| `test_stage_registry_resolve_calls_factory` | `ecl-pipeline::registry` | Factory function is invoked, mock stage returned |
| `test_stage_registry_register_overwrites` | `ecl-pipeline::registry` | Re-registering same key replaces the factory |
| `test_resolve_retry_policy_uses_global_when_no_override` | `ecl-pipeline-topo::resolve` | `None` stage retry uses global defaults |
| `test_resolve_retry_policy_uses_stage_override` | `ecl-pipeline-topo::resolve` | `Some(RetrySpec)` uses stage values |
| `test_resolve_retry_policy_converts_ms_to_duration` | `ecl-pipeline-topo::resolve` | Milliseconds correctly become `Duration` values |
| `test_resolve_retry_policy_custom_stage_values` | `ecl-pipeline-topo::resolve` | Custom stage retry produces matching `RetryPolicy` |
| `test_source_kind_google_drive` | `ecl-pipeline-topo::resolve` | Google Drive variant returns `"google_drive"` |
| `test_source_kind_slack` | `ecl-pipeline-topo::resolve` | Slack variant returns `"slack"` |
| `test_source_kind_filesystem` | `ecl-pipeline-topo::resolve` | Filesystem variant returns `"filesystem"` |
| `test_resolve_full_example_spec_returns_topology` | `ecl-pipeline-topo::resolve` | Full TOML resolves to correct topology with 2 sources, 5 stages, 3 batches |
| `test_resolve_unknown_source_kind_returns_error` | `ecl-pipeline-topo::resolve` | Adapter lookup failure propagates as `UnknownAdapter` |
| `test_resolve_unknown_stage_adapter_returns_error` | `ecl-pipeline-topo::resolve` | Stage lookup failure propagates as `UnknownAdapter` |
| `test_resolve_creates_output_directory` | `ecl-pipeline-topo::resolve` | Output dir is created on disk |
| `test_resolve_spec_hash_is_deterministic` | `ecl-pipeline-topo::resolve` | Same spec produces same hash |
| `test_resolve_spec_hash_changes_with_spec` | `ecl-pipeline-topo::resolve` | Different specs produce different hashes |
| `test_resolve_stages_have_correct_retry_policy` | `ecl-pipeline-topo::resolve` | Stage with retry override has overridden values |
| `test_resolve_stages_use_global_retry_when_no_override` | `ecl-pipeline-topo::resolve` | Stage without retry uses global defaults |
| `test_resolve_stages_have_correct_timeout` | `ecl-pipeline-topo::resolve` | `timeout_secs = 300` becomes `Some(Duration::from_secs(300))` |
| `test_resolve_stages_have_correct_skip_on_error` | `ecl-pipeline-topo::resolve` | `skip_on_error = true` propagates correctly |
| `test_resolve_stages_have_correct_condition` | `ecl-pipeline-topo::resolve` | `condition = "x > 1"` becomes `Some(ConditionExpr::new("x > 1"))` |
| `test_resolve_schedule_matches_expected` | `ecl-pipeline-topo::resolve` | 5-stage example produces 3-batch schedule |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-pipeline` passes
- [ ] `cargo check -p ecl-pipeline-topo` passes
- [ ] `cargo test -p ecl-pipeline` passes (all tests green)
- [ ] `cargo test -p ecl-pipeline-topo` passes (all tests green)
- [ ] `cargo test --workspace` passes (no regressions in other crates)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] The `todo!()` in `resolve.rs` is gone -- `resolve()` is fully implemented
- [ ] `AdapterRegistry` and `StageRegistry` are usable with mock factories
- [ ] `resolve_retry_policy()` correctly merges stage retry > global default
- [ ] `resolve()` signature accepts lookup closures (not hardcoded registries) for testability
- [ ] The old `test_resolve_stub_is_todo` test is removed
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement concrete source adapters (Google Drive, Slack, Filesystem) -- that is milestone 3.1+
- Do NOT implement concrete stage handlers (extract, normalize, emit) -- that is milestone 3.1+
- Do NOT modify the runner's execution logic in `ecl-pipeline` -- milestone 2.2 is done
- Do NOT add CLI commands -- that is milestone 5.1
- Do NOT create any new crates -- this milestone only modifies `ecl-pipeline-topo` and `ecl-pipeline`
- Do NOT change `ResourceGraph`, `schedule.rs`, or the existing topology types (they are complete from 1.3)
- Do NOT implement condition expression evaluation -- `ConditionExpr` remains a newtype wrapper
- Do NOT implement the `StateStore` trait or checkpoint persistence -- that is milestone 1.2
