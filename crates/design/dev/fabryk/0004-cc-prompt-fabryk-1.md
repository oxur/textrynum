---
title: "CC Prompt: Fabryk 1.4 — ID Utilities & PathResolver"
milestone: "1.4"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["1.0 Cleanup", "1.1 Workspace scaffold", "1.2 Error types", "1.3 File & path utilities"]
governing-docs: [0011-audit §4.1, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.4 — ID Utilities & PathResolver

## Context

Continuing `fabryk-core` extraction. Milestone 1.3 extracted the generic path
utilities (`expand_tilde`, `binary_path`, `find_dir_with_marker`). This milestone
adds two complementary features:

1. **ID utilities**: Generic ID normalization extracted from `graph/parser.rs`
2. **PathResolver**: Configurable domain path resolution using primitives from 1.3

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion), when all imports will be updated in a single
coordinated migration.

## Source Analysis

### normalize_concept_id() — Generic (G)

**Location**: `mcp-server/crates/server/src/graph/parser.rs:156`

```rust
fn normalize_concept_id(id: &str) -> String {
    id.trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join("-")
}
```

This is fully generic — no domain coupling. Rename to `normalize_id()` for Fabryk.

### Domain-specific path functions — Parameterized (P)

**Location**: `mcp-server/crates/server/src/util/paths.rs`

Functions like `config_dir()`, `skill_root()`, `server_root()` use hardcoded
`MUSIC_THEORY_*` environment variables. These should become parameterizable
through a `PathResolver` struct that domains can configure.

## Objective

1. Create `fabryk-core::util::ids` module with `normalize_id()` function
2. Create `fabryk-core::util::resolver` module with `PathResolver` struct
3. `PathResolver` uses the generic utilities from milestone 1.3
4. Add comprehensive tests
5. Verify: `cargo test -p fabryk-core` passes

## Implementation Steps

### Step 1: Create fabryk-core/src/util/ids.rs

```rust
//! ID normalization utilities.
//!
//! Provides functions for normalizing string identifiers to consistent
//! kebab-case format. Used by graph builders, content loaders, and
//! anywhere stable IDs are needed.

/// Normalize an identifier to lowercase kebab-case.
///
/// Performs the following transformations:
/// 1. Trims leading/trailing whitespace
/// 2. Converts to lowercase
/// 3. Replaces underscores with hyphens
/// 4. Collapses multiple whitespace into single hyphens
///
/// # Examples
///
/// ```
/// use fabryk_core::util::ids::normalize_id;
///
/// assert_eq!(normalize_id("Voice Leading"), "voice-leading");
/// assert_eq!(normalize_id("non_chord_tone"), "non-chord-tone");
/// assert_eq!(normalize_id("  Mixed   Case  "), "mixed-case");
/// assert_eq!(normalize_id("UPPERCASE"), "uppercase");
/// ```
pub fn normalize_id(id: &str) -> String {
    id.trim()
        .to_lowercase()
        .replace('_', " ")  // Convert underscores to spaces first
        .split_whitespace() // Split on any whitespace, collapsing multiples
        .collect::<Vec<&str>>()
        .join("-")
}

/// Compute an ID from a file path's stem.
///
/// Extracts the file stem (filename without extension) and normalizes it.
/// Returns `None` if the path has no file stem.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use fabryk_core::util::ids::id_from_path;
///
/// assert_eq!(
///     id_from_path(Path::new("/data/concepts/Voice_Leading.md")),
///     Some("voice-leading".to_string())
/// );
/// assert_eq!(
///     id_from_path(Path::new("/data/Major Scale.md")),
///     Some("major-scale".to_string())
/// );
/// assert_eq!(id_from_path(Path::new("/")), None);
/// ```
pub fn id_from_path(path: &std::path::Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(normalize_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // -------------------------------------------------------------------------
    // normalize_id tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_normalize_id_simple() {
        assert_eq!(normalize_id("dissonance"), "dissonance");
    }

    #[test]
    fn test_normalize_id_with_spaces() {
        assert_eq!(normalize_id("Voice Leading"), "voice-leading");
    }

    #[test]
    fn test_normalize_id_with_underscores() {
        assert_eq!(normalize_id("non_chord_tone"), "non-chord-tone");
    }

    #[test]
    fn test_normalize_id_mixed_case() {
        assert_eq!(normalize_id("PicardyThird"), "picardythird");
    }

    #[test]
    fn test_normalize_id_with_whitespace() {
        assert_eq!(normalize_id("  Mixed   Case  "), "mixed-case");
    }

    #[test]
    fn test_normalize_id_already_normalized() {
        assert_eq!(normalize_id("voice-leading"), "voice-leading");
    }

    #[test]
    fn test_normalize_id_uppercase() {
        assert_eq!(normalize_id("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_normalize_id_empty() {
        assert_eq!(normalize_id(""), "");
        assert_eq!(normalize_id("   "), "");
    }

    #[test]
    fn test_normalize_id_mixed_separators() {
        assert_eq!(normalize_id("foo_bar baz"), "foo-bar-baz");
    }

    // -------------------------------------------------------------------------
    // id_from_path tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_id_from_path_simple() {
        let path = Path::new("/data/concepts/dissonance.md");
        assert_eq!(id_from_path(path), Some("dissonance".to_string()));
    }

    #[test]
    fn test_id_from_path_with_underscores() {
        let path = Path::new("/data/Voice_Leading.md");
        assert_eq!(id_from_path(path), Some("voice-leading".to_string()));
    }

    #[test]
    fn test_id_from_path_nested() {
        let path = Path::new("/data/harmony/chord-progressions/ii-V-I.md");
        assert_eq!(id_from_path(path), Some("ii-v-i".to_string()));
    }

    #[test]
    fn test_id_from_path_no_extension() {
        let path = Path::new("/data/README");
        assert_eq!(id_from_path(path), Some("readme".to_string()));
    }

    #[test]
    fn test_id_from_path_no_stem() {
        let path = Path::new("/");
        assert_eq!(id_from_path(path), None);
    }

    #[test]
    fn test_id_from_path_hidden_file() {
        let path = Path::new("/data/.hidden");
        assert_eq!(id_from_path(path), Some(".hidden".to_string()));
    }
}
```

### Step 2: Create fabryk-core/src/util/resolver.rs

```rust
//! Configurable path resolver for domain-specific directories.
//!
//! `PathResolver` provides a configurable way to locate project directories
//! using environment variables, directory markers, and fallback paths.
//! Each domain (music-theory, another-project, etc.) creates its own resolver
//! with appropriate configuration.
//!
//! # Example
//!
//! ```no_run
//! use fabryk_core::util::resolver::PathResolver;
//!
//! // Create a resolver for "music-theory" project
//! let resolver = PathResolver::new("music-theory")
//!     .with_config_marker("config/default.toml")
//!     .with_project_markers(&["SKILL.md", "CONVENTIONS.md"]);
//!
//! // These check MUSIC_THEORY_CONFIG_DIR, then search for markers
//! if let Some(config) = resolver.config_dir() {
//!     println!("Config: {:?}", config);
//! }
//! ```

use std::env;
use std::path::PathBuf;

use crate::util::paths::{binary_dir, expand_tilde, find_dir_with_marker};

/// Configurable path resolver for a specific project/domain.
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Project name (e.g., "music-theory")
    project_name: String,
    /// Environment variable prefix (e.g., "MUSIC_THEORY")
    env_prefix: String,
    /// Marker file/dir to identify config directory (e.g., "config/default.toml")
    config_marker: Option<String>,
    /// Marker files to identify project root (e.g., ["SKILL.md", "Cargo.toml"])
    project_markers: Vec<String>,
    /// Fallback config path (expanded with tilde)
    config_fallback: Option<PathBuf>,
    /// Fallback project root (expanded with tilde)
    project_fallback: Option<PathBuf>,
}

impl PathResolver {
    /// Create a new resolver for the given project name.
    ///
    /// The project name is converted to an environment variable prefix:
    /// - "music-theory" → "MUSIC_THEORY"
    /// - "my_project" → "MY_PROJECT"
    pub fn new(project_name: &str) -> Self {
        let env_prefix = project_name
            .to_uppercase()
            .replace('-', "_")
            .replace(' ', "_");

        Self {
            project_name: project_name.to_string(),
            env_prefix,
            config_marker: None,
            project_markers: vec![],
            config_fallback: None,
            project_fallback: None,
        }
    }

    /// Set the marker file/directory that identifies a config directory.
    pub fn with_config_marker(mut self, marker: &str) -> Self {
        self.config_marker = Some(marker.to_string());
        self
    }

    /// Set marker files that identify the project root.
    pub fn with_project_markers(mut self, markers: &[&str]) -> Self {
        self.project_markers = markers.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set a fallback path for config directory (supports ~ expansion).
    pub fn with_config_fallback(mut self, path: &str) -> Self {
        self.config_fallback = Some(expand_tilde(path));
        self
    }

    /// Set a fallback path for project root (supports ~ expansion).
    pub fn with_project_fallback(mut self, path: &str) -> Self {
        self.project_fallback = Some(expand_tilde(path));
        self
    }

    /// Get the environment variable name for a given suffix.
    ///
    /// # Example
    /// ```
    /// use fabryk_core::util::resolver::PathResolver;
    ///
    /// let resolver = PathResolver::new("music-theory");
    /// assert_eq!(resolver.env_var("CONFIG_DIR"), "MUSIC_THEORY_CONFIG_DIR");
    /// ```
    pub fn env_var(&self, suffix: &str) -> String {
        format!("{}_{}", self.env_prefix, suffix)
    }

    /// Resolve the config directory.
    ///
    /// Checks in order:
    /// 1. `{PROJECT}_CONFIG_DIR` environment variable
    /// 2. Walk up from binary looking for config marker
    /// 3. Fallback path (if configured)
    pub fn config_dir(&self) -> Option<PathBuf> {
        // 1. Check environment variable
        let env_var = self.env_var("CONFIG_DIR");
        if let Ok(path) = env::var(&env_var) {
            let path = expand_tilde(&path);
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Walk up from binary location
        if let (Some(bin_dir), Some(marker)) = (binary_dir(), &self.config_marker) {
            if let Some(root) = find_dir_with_marker(&bin_dir, marker) {
                // The marker might be nested (e.g., "config/default.toml")
                // Return the directory containing the first component
                if let Some(first_component) = marker.split('/').next() {
                    let config_path = root.join(first_component);
                    if config_path.exists() {
                        return Some(config_path);
                    }
                }
                return Some(root);
            }
        }

        // 3. Try fallback
        if let Some(fallback) = &self.config_fallback {
            if fallback.exists() {
                return Some(fallback.clone());
            }
        }

        None
    }

    /// Resolve the project root directory.
    ///
    /// Checks in order:
    /// 1. `{PROJECT}_ROOT` environment variable
    /// 2. Walk up from binary looking for project markers
    /// 3. Fallback path (if configured)
    pub fn project_root(&self) -> Option<PathBuf> {
        // 1. Check environment variable
        let env_var = self.env_var("ROOT");
        if let Ok(path) = env::var(&env_var) {
            let path = expand_tilde(&path);
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Walk up from binary location, trying each marker
        if let Some(bin_dir) = binary_dir() {
            for marker in &self.project_markers {
                if let Some(root) = find_dir_with_marker(&bin_dir, marker) {
                    return Some(root);
                }
            }
        }

        // 3. Try fallback
        if let Some(fallback) = &self.project_fallback {
            if fallback.exists() {
                return Some(fallback.clone());
            }
        }

        None
    }

    /// Get the project name.
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// Get the environment variable prefix.
    pub fn env_prefix(&self) -> &str {
        &self.env_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_simple_name() {
        let resolver = PathResolver::new("myproject");
        assert_eq!(resolver.project_name(), "myproject");
        assert_eq!(resolver.env_prefix(), "MYPROJECT");
    }

    #[test]
    fn test_new_kebab_case_name() {
        let resolver = PathResolver::new("music-theory");
        assert_eq!(resolver.project_name(), "music-theory");
        assert_eq!(resolver.env_prefix(), "MUSIC_THEORY");
    }

    #[test]
    fn test_new_snake_case_name() {
        let resolver = PathResolver::new("my_project");
        assert_eq!(resolver.project_name(), "my_project");
        assert_eq!(resolver.env_prefix(), "MY_PROJECT");
    }

    #[test]
    fn test_env_var() {
        let resolver = PathResolver::new("music-theory");
        assert_eq!(resolver.env_var("CONFIG_DIR"), "MUSIC_THEORY_CONFIG_DIR");
        assert_eq!(resolver.env_var("ROOT"), "MUSIC_THEORY_ROOT");
        assert_eq!(resolver.env_var("DATA_DIR"), "MUSIC_THEORY_DATA_DIR");
    }

    #[test]
    fn test_config_dir_from_env() {
        let resolver = PathResolver::new("test-project");

        // Create temp directory
        let temp_dir = std::env::temp_dir().join("test_config_resolver");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Set env var
        env::set_var("TEST_PROJECT_CONFIG_DIR", &temp_dir);

        let result = resolver.config_dir();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        env::remove_var("TEST_PROJECT_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_project_root_from_env() {
        let resolver = PathResolver::new("test-project");

        // Create temp directory
        let temp_dir = std::env::temp_dir().join("test_root_resolver");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Set env var
        env::set_var("TEST_PROJECT_ROOT", &temp_dir);

        let result = resolver.project_root();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        env::remove_var("TEST_PROJECT_ROOT");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_config_dir_with_fallback() {
        let temp_dir = std::env::temp_dir().join("test_config_fallback");
        let _ = std::fs::create_dir_all(&temp_dir);

        let resolver = PathResolver::new("nonexistent-project")
            .with_config_fallback(&temp_dir.to_string_lossy());

        let result = resolver.config_dir();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_builder_pattern() {
        let resolver = PathResolver::new("my-app")
            .with_config_marker("config/settings.toml")
            .with_project_markers(&["Cargo.toml", "package.json"])
            .with_config_fallback("~/.config/my-app")
            .with_project_fallback("~/projects/my-app");

        assert_eq!(resolver.project_name(), "my-app");
        assert_eq!(resolver.env_prefix(), "MY_APP");
    }
}
```

### Step 3: Update fabryk-core/src/util/mod.rs

```rust
//! Utility modules for file operations, path handling, ID computation,
//! and common helpers.
//!
//! # Modules
//!
//! - [`files`]: Async file discovery and reading utilities
//! - [`ids`]: ID normalization and computation
//! - [`paths`]: Generic path utilities (binary location, tilde expansion)
//! - [`resolver`]: Configurable domain path resolution

pub mod files;
pub mod ids;
pub mod paths;
pub mod resolver;
```

### Step 4: Update fabryk-core/src/lib.rs

Add convenience re-exports:

```rust
//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).
//!
//! # Modules
//!
//! - [`error`]: Error types and Result alias
//! - [`util`]: File, path, and ID utilities

#![doc = include_str!("../README.md")]

pub mod error;
pub mod util;

// Re-export key types at crate root for convenience
pub use error::{Error, Result};

// Convenience re-exports from util
pub use util::ids::{id_from_path, normalize_id};
pub use util::resolver::PathResolver;

// Modules to be added during extraction:
// pub mod traits;
// pub mod state;
// pub mod resources;
```

### Step 5: Verify

```bash
cd ~/lab/oxur/ecl
cargo test -p fabryk-core
cargo clippy -p fabryk-core -- -D warnings
cargo doc -p fabryk-core --no-deps
```

## Exit Criteria

- [ ] `fabryk-core/src/util/ids.rs` exists with `normalize_id()` and `id_from_path()`
- [ ] `fabryk-core/src/util/resolver.rs` exists with `PathResolver` struct
- [ ] `PathResolver` uses generic utilities from milestone 1.3
- [ ] `PathResolver` supports env var, marker search, and fallback resolution
- [ ] No hardcoded domain-specific strings (MUSIC_THEORY, etc.)
- [ ] Convenience re-exports at crate root: `normalize_id`, `id_from_path`, `PathResolver`
- [ ] All tests pass
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

## Domain Migration Note

After this milestone, music-theory can use the PathResolver for its path needs:

```rust
// music-theory/crates/server/src/util/paths.rs (after v0.1-alpha migration)
use fabryk_core::PathResolver;

lazy_static! {
    static ref RESOLVER: PathResolver = PathResolver::new("music-theory")
        .with_config_marker("config/default.toml")
        .with_project_markers(&["SKILL.md", "CONVENTIONS.md", "SCOPE.md"])
        .with_config_fallback("~/lab/music-comp/ai-music-theory/mcp-server/crates/server/config")
        .with_project_fallback("~/lab/music-comp/ai-music-theory");
}

pub fn config_dir() -> Option<PathBuf> {
    RESOLVER.config_dir()
}

pub fn skill_root() -> Option<PathBuf> {
    RESOLVER.project_root()
}
```

## Commit Message

```
feat(core): add ID utilities and PathResolver

Add fabryk-core::util::ids with:
- normalize_id(): kebab-case normalization
- id_from_path(): extract normalized ID from file path

Add fabryk-core::util::resolver with PathResolver:
- Configurable env var prefix per project
- Config/project directory resolution
- Uses generic utilities from milestone 1.3

Ref: Doc 0013 milestone 1.4, Audit §4.1

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
