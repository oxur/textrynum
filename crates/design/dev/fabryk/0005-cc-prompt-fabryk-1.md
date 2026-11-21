---
title: "CC Prompt: Fabryk 1.5 — ConfigProvider Trait & AppState"
milestone: "1.5"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["1.0 Cleanup", "1.1 Workspace scaffold", "1.2 Error types", "1.3 File & path utilities", "1.4 ID utilities & PathResolver"]
governing-docs: [0011-audit §4.1, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.5 — ConfigProvider Trait & AppState

## Context

This is the most architecturally significant milestone in Phase 1. It defines
the `ConfigProvider` trait — the core abstraction that lets every Fabryk crate
work with any domain's configuration without knowing its specifics. It also
creates a minimal `AppState<C: ConfigProvider>` for config management.

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion), when all imports will be updated in a single
coordinated migration.

**Classification:** Parameterized (P) — requires trait abstraction.

## Source Files

```
~/lab/music-comp/ai-music-theory/crates/server/src/state.rs
~/lab/music-comp/ai-music-theory/crates/server/src/config.rs  (read-only reference)
```

**From Audit §1:**
- `state.rs`: 207 lines. Types: `AppState`, `GraphState`. Deps: `Arc`, `RwLock`.
- `config.rs`: 753 lines. Types: `Config`, `SourcesConfig`, `SearchConfig`.
  **This file stays in ai-music-theory** (Classification: D).

## Objective

1. Define `ConfigProvider` trait in `fabryk-core::traits`
2. Extract `state.rs` as `AppState<C: ConfigProvider>` in `fabryk-core::state`
3. Ensure `AppState` is generic but still provides the access patterns the
   rest of the codebase needs
4. Verify: trait defined, `AppState` generic, `fabryk-core` compiles

## Implementation Steps

### Step 1: Read both source files

```bash
cat ~/lab/music-comp/ai-music-theory/crates/server/src/config.rs
cat ~/lab/music-comp/ai-music-theory/crates/server/src/state.rs
```

**For `config.rs`:** Understand what the rest of the codebase asks of `Config`.
Look for:
- Which methods/fields are accessed from outside `config.rs`
- Patterns like `config.base_path()`, `config.content_dir()`, `config.sources`
- How `Config` is used in `AppState`

**For `state.rs`:** Understand:
- How `AppState` wraps `Config` (likely `Arc<Config>`)
- What other state it holds (search backend? graph state?)
- How it's accessed (`.config()`, `.search()`, `.graph()`)

### Step 2: Design the `ConfigProvider` trait

Based on the audit §4.1 and what the codebase actually accesses:

```rust
//! Core traits for Fabryk domain abstraction.
//!
//! These traits define the extension points that domain skills implement
//! to customise Fabryk's behaviour.

use std::path::PathBuf;
use crate::Result;

/// Trait for domain-specific configuration.
///
/// Every Fabryk-based application implements this trait to provide
/// the configuration that Fabryk crates need: paths to content,
/// project identity, and domain-specific settings.
///
/// # Examples
///
/// ```ignore
/// // In ai-music-theory:
/// impl ConfigProvider for MusicTheoryConfig {
///     fn project_name(&self) -> &str { "music-theory" }
///     fn base_path(&self) -> Result<PathBuf> { Ok(self.data_dir.clone()) }
///     fn content_path(&self, content_type: &str) -> Result<PathBuf> {
///         Ok(self.data_dir.join(content_type))
///     }
/// }
/// ```
pub trait ConfigProvider: Send + Sync + Clone + 'static {
    /// The project name, used for env var prefixes and default paths.
    ///
    /// Example: `"music-theory"` produces env vars like
    /// `MUSIC_THEORY_CONFIG_DIR`.
    fn project_name(&self) -> &str;

    /// Base path for all project data.
    fn base_path(&self) -> Result<PathBuf>;

    /// Path for a specific content type.
    ///
    /// `content_type` is a domain-defined key like `"concepts"`,
    /// `"sources"`, `"guides"`.
    fn content_path(&self, content_type: &str) -> Result<PathBuf>;
}
```

**Important:** Read `config.rs` thoroughly before finalising. The trait should
capture what the **rest of the codebase** needs from Config, not what Config
happens to contain. Look at every call site: `state.rs`, `server.rs`, `cli.rs`,
`graph/builder.rs`, `search/indexer.rs`, tool files, etc.

Common patterns to look for and potentially add to the trait:

```rust
/// Server configuration (host, port, etc.)
/// Only if needed by fabryk-mcp
fn server_host(&self) -> &str { "127.0.0.1" }
fn server_port(&self) -> u16 { 3000 }

/// Search configuration
/// Only if needed by fabryk-fts
fn search_index_path(&self) -> Result<PathBuf>;

/// Graph configuration
/// Only if needed by fabryk-graph
fn graph_cache_path(&self) -> Result<PathBuf>;
```

**However:** Be conservative. Only add methods that are clearly needed by
`fabryk-core` itself or that multiple downstream crates will need. Search and
graph config can live in their own crate-specific traits if needed. Start
minimal and expand based on actual usage in later milestones.

### Step 3: Extract and genericise `AppState`

Read `state.rs` to understand the current structure. It likely looks something
like:

```rust
// Current (domain-coupled)
pub struct AppState {
    pub config: Arc<Config>,
    pub search: Arc<RwLock<Option<SearchBackend>>>,
    pub graph: Arc<RwLock<Option<GraphState>>>,
}
```

Transform to:

```rust
//! Application state management.
//!
//! Provides `AppState<C>`, a thread-safe container for shared application
//! state that is generic over the configuration provider.

use std::sync::Arc;
use tokio::sync::RwLock;
use crate::traits::ConfigProvider;

/// Thread-safe shared application state.
///
/// Generic over `C: ConfigProvider` so that any domain can use it
/// with their own configuration type.
///
/// # Type Parameters
///
/// - `C` — The domain-specific configuration provider.
pub struct AppState<C: ConfigProvider> {
    config: Arc<C>,
}

impl<C: ConfigProvider> AppState<C> {
    /// Create a new AppState wrapping the given configuration.
    pub fn new(config: C) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Get a cloneable handle to the configuration.
    pub fn config_arc(&self) -> Arc<C> {
        Arc::clone(&self.config)
    }
}

impl<C: ConfigProvider> Clone for AppState<C> {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
        }
    }
}
```

**Key design question:** The current `AppState` likely holds more than just
config — it probably holds `SearchBackend`, `GraphState`, etc. For this
milestone, extract **only the config-related parts** into `fabryk-core`. The
search and graph state will be added by their respective crates or by the
domain application that composes them.

If `AppState` currently holds search and graph state, there are two approaches:

1. **Minimal AppState in fabryk-core** (recommended): Only holds `config`.
   Downstream crates provide their own state types or extension traits.
   The domain application composes them into a full state struct.

2. **Generic slots**: `AppState<C, S, G>` with optional search and graph
   state. Risks over-engineering at this stage.

Go with option 1 unless the source code reveals a strong reason for option 2.

### Step 4: Update `fabryk-core/src/lib.rs`

```rust
//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).

pub mod error;
pub mod state;
pub mod traits;
pub mod util;

pub use error::{Error, Result};
pub use traits::ConfigProvider;
```

### Step 5: Update `Cargo.toml` if needed

`state.rs` likely needs `tokio` for `RwLock` and/or `Arc`. These should already
be in the dependency list from milestone 1.3. Verify.

### Step 6: Add tests

```rust
// In traits.rs or a test module
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[derive(Clone)]
    struct TestConfig {
        name: String,
        base: PathBuf,
    }

    impl ConfigProvider for TestConfig {
        fn project_name(&self) -> &str {
            &self.name
        }

        fn base_path(&self) -> crate::Result<PathBuf> {
            Ok(self.base.clone())
        }

        fn content_path(&self, content_type: &str) -> crate::Result<PathBuf> {
            Ok(self.base.join(content_type))
        }
    }

    #[test]
    fn test_config_provider_project_name() {
        let config = TestConfig {
            name: "test-project".into(),
            base: PathBuf::from("/tmp/test"),
        };
        assert_eq!(config.project_name(), "test-project");
    }

    #[test]
    fn test_config_provider_content_path() {
        let config = TestConfig {
            name: "test".into(),
            base: PathBuf::from("/data"),
        };
        let path = config.content_path("concepts").unwrap();
        assert_eq!(path, PathBuf::from("/data/concepts"));
    }
}

// In state.rs
#[cfg(test)]
mod tests {
    use super::*;
    // Re-use TestConfig from traits tests or define inline

    #[test]
    fn test_app_state_creation() {
        let config = TestConfig { /* ... */ };
        let state = AppState::new(config);
        assert_eq!(state.config().project_name(), "test-project");
    }

    #[test]
    fn test_app_state_clone() {
        let config = TestConfig { /* ... */ };
        let state1 = AppState::new(config);
        let state2 = state1.clone();
        assert_eq!(
            state1.config().project_name(),
            state2.config().project_name()
        );
    }
}
```

### Step 7: Verify

```bash
cd ~/lab/music-comp/fabryk
cargo test -p fabryk-core
cargo clippy -p fabryk-core -- -D warnings
cargo doc -p fabryk-core --no-deps  # verify docs compile
```

## Exit Criteria

- [ ] `fabryk-core/src/traits.rs` exists with `ConfigProvider` trait
- [ ] `ConfigProvider` has at minimum: `project_name()`, `base_path()`, `content_path()`
- [ ] Trait bounds include `Send + Sync + Clone + 'static`
- [ ] `fabryk-core/src/state.rs` exists with `AppState<C: ConfigProvider>`
- [ ] `AppState` is `Clone` and provides `config()` and `config_arc()` accessors
- [ ] `AppState` does NOT hold search or graph state (that's for later phases)
- [ ] Test config implements `ConfigProvider` and tests pass
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

## Notes

- The `ConfigProvider` trait will likely grow as we discover what downstream
  crates need during Phases 2–6. That's expected. Start minimal.
- The current `AppState` in music-theory holds search and graph state. Those
  parts stay in the music-theory codebase for now and will be addressed in
  Phases 3–5 when those subsystems are extracted.
- If the existing `state.rs` has `GraphState` as a sub-type, leave it in the
  music-theory codebase. Only extract the config wrapper.

## Commit Message

```
feat(core): define ConfigProvider trait and generic AppState

Define ConfigProvider trait in fabryk-core::traits for domain-specific
configuration abstraction. Methods: project_name(), base_path(),
content_path().

Extract state.rs as AppState<C: ConfigProvider>, generic over the
configuration type. Holds config in Arc for thread-safe sharing.

Ref: Doc 0013 milestone 1.5, Audit §4.1
```
