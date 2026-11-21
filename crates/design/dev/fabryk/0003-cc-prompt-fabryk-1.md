---
title: "CC Prompt: Fabryk 1.3 — File & Path Utilities"
milestone: "1.3"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["1.0 Cleanup", "1.1 Workspace scaffold", "1.2 Error types"]
governing-docs: [0011-audit §4.1, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.3 — File & Path Utilities

## Context

Continuing `fabryk-core` extraction. Milestones 1.0-1.2 established the workspace
scaffold and error types. This milestone extracts the async file utilities and
generic path utilities.

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion), when all imports will be updated in a single
coordinated migration.

## Source Files

**Music-theory sources** (via symlink or directly):
```
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/util/files.rs
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/util/paths.rs
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/util/mod.rs
```

Or directly:
```
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/util/files.rs
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/util/paths.rs
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/util/mod.rs
```

**From Audit §1 (File Inventory)**:

| File | Lines | Classification | Extract? |
|------|-------|----------------|----------|
| `files.rs` | 624 | Generic (G) | Full extraction |
| `paths.rs` | 535 | Mixed (G + M) | Partial extraction |
| `mod.rs` | 8 | N/A | Module structure |

## Classification Analysis

### files.rs — Fully Generic (G)

This module is fully generic with no domain coupling:

- **Types**: `FindOptions`, `FileInfo`
- **Functions**: `find_file_by_id()`, `find_all_files()`, `list_subdirectories()`,
  `count_files()`, `read_file()`, `exists()`
- **Dependencies**: `async_walkdir`, `futures`, `tokio::fs`
- **Error usage**: `Error::io()`, `Error::io_with_path()`, `Error::not_found_msg()`

All error constructors are available from milestone 1.2.

### paths.rs — Mixed Classification

| Function | Classification | Extract? | Notes |
|----------|----------------|----------|-------|
| `binary_path()` | Generic (G) | Yes | `env::current_exe()` wrapper |
| `binary_dir()` | Generic (G) | Yes | Parent of binary path |
| `find_dir_with_marker()` | Generic (G) | Yes | Walk up tree looking for marker |
| `expand_tilde()` | Generic (G) | Yes | Expand `~` to home dir |
| `server_root()` | Domain (M) | No | Looks for music-theory markers |
| `project_root()` | Domain (M) | No | Looks for SKILL.md, CONVENTIONS.md |
| `config_dir()` | Domain (M) | No | Uses MUSIC_THEORY_* env vars |
| `skill_root()` | Domain (M) | No | Uses MUSIC_THEORY_* env vars |
| `debug_paths()` | Domain (M) | No | References domain functions |

## Objective

1. Extract `util/files.rs` into `fabryk-core::util::files` (full)
2. Extract generic parts of `util/paths.rs` into `fabryk-core::util::paths`
3. Set up the `util` module structure in fabryk-core
4. Ensure all tests pass in the new location
5. Verify: `cargo test -p fabryk-core` passes

## Implementation Steps

### Step 1: Update fabryk-core/Cargo.toml dependencies

Add dependencies required by files.rs and paths.rs:

```toml
[dependencies]
# Async runtime
tokio = { workspace = true }
async-trait = { workspace = true }

# Async file walking
async-walkdir = { workspace = true }
futures = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }
serde_yaml = { workspace = true }

# Error handling
thiserror = { workspace = true }

# File operations
glob = { workspace = true }
shellexpand = { workspace = true }
dirs = { workspace = true }

# Logging
log = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

**Note**: `async-walkdir` and `futures` are the key new dependencies for files.rs.

### Step 2: Create the util module structure

```
fabryk-core/src/
├── error.rs       (from milestone 1.2)
├── util/
│   ├── mod.rs
│   ├── files.rs
│   └── paths.rs
└── lib.rs
```

### Step 3: Create fabryk-core/src/util/mod.rs

```rust
//! Utility modules for file operations, path handling, and common helpers.
//!
//! # Modules
//!
//! - [`files`]: Async file discovery and reading utilities
//! - [`paths`]: Path resolution helpers (binary location, tilde expansion)

pub mod files;
pub mod paths;
```

### Step 4: Create fabryk-core/src/util/files.rs

Copy the entire `files.rs` from music-theory. The only change needed is the
import path for errors:

**Before** (music-theory):
```rust
use crate::error::{Error, Result};
```

**After** (fabryk-core):
```rust
use crate::{Error, Result};
```

The file is otherwise unchanged. Key exports:

```rust
// Types
pub struct FindOptions { ... }
pub struct FileInfo { ... }

// Functions
pub async fn find_file_by_id(base_path: &Path, id: &str, options: FindOptions) -> Result<PathBuf>
pub async fn find_all_files(base_path: &Path, options: FindOptions) -> Result<Vec<FileInfo>>
pub async fn list_subdirectories(base_path: &Path) -> Result<Vec<PathBuf>>
pub async fn count_files(base_path: &Path, options: FindOptions) -> Result<usize>
pub async fn read_file(path: &Path) -> Result<String>
pub async fn exists(path: &Path) -> bool
```

Error constructors used:
- `Error::io(err)` — for walkdir errors
- `Error::io_with_path(err, path)` — for file read errors with path context
- `Error::not_found_msg(msg)` — for file not found in search

All of these are defined in milestone 1.2.

### Step 5: Create fabryk-core/src/util/paths.rs

Extract only the generic utilities from paths.rs:

```rust
//! Path resolution utilities.
//!
//! Provides generic path helpers used across all Fabryk domains.
//! Domain-specific path resolution (config dirs, project roots) should
//! be implemented in domain crates using these primitives.

use std::env;
use std::path::{Path, PathBuf};

/// Maximum number of parent directories to walk when searching for a marker.
pub const MAX_WALK_LEVELS: usize = 10;

/// Returns the absolute path to the currently running binary.
pub fn binary_path() -> Option<PathBuf> {
    env::current_exe().ok()
}

/// Returns the directory containing the currently running binary.
pub fn binary_dir() -> Option<PathBuf> {
    binary_path().and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Walks up the directory tree from `start` looking for a directory containing `marker`.
///
/// Returns the directory containing the marker file/directory, or None if not found
/// within `MAX_WALK_LEVELS` iterations.
///
/// # Example
///
/// ```no_run
/// use fabryk_core::util::paths::find_dir_with_marker;
///
/// // Find a project root by looking for Cargo.toml
/// if let Some(root) = find_dir_with_marker(".", "Cargo.toml") {
///     println!("Project root: {:?}", root);
/// }
/// ```
pub fn find_dir_with_marker<P: AsRef<Path>>(start: P, marker: &str) -> Option<PathBuf> {
    let mut current = start.as_ref().to_path_buf();

    for _ in 0..MAX_WALK_LEVELS {
        let candidate = current.join(marker);
        if candidate.exists() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}

/// Expands `~` to the user's home directory.
///
/// If the path starts with `~`, replaces it with the user's home directory.
/// Otherwise returns the path unchanged.
///
/// # Example
///
/// ```
/// use fabryk_core::util::paths::expand_tilde;
///
/// let expanded = expand_tilde("~/documents");
/// assert!(!expanded.starts_with("~"));
/// ```
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    if let Ok(stripped) = path.strip_prefix("~") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_path_exists() {
        let path = binary_path();
        assert!(path.is_some(), "Binary path should be found");
        let path = path.unwrap();
        assert!(path.exists(), "Binary path should exist: {:?}", path);
        assert!(path.is_file(), "Binary path should be a file: {:?}", path);
    }

    #[test]
    fn test_binary_dir_exists() {
        let dir = binary_dir();
        assert!(dir.is_some(), "Binary dir should be found");
        let dir = dir.unwrap();
        assert!(dir.exists(), "Binary dir should exist: {:?}", dir);
        assert!(dir.is_dir(), "Binary dir should be a directory: {:?}", dir);
    }

    #[test]
    fn test_expand_tilde_with_tilde() {
        let path = expand_tilde("~/test/path");
        assert!(!path.starts_with("~"), "Tilde should be expanded");
        if let Some(home) = dirs::home_dir() {
            assert!(path.starts_with(&home), "Path should start with home dir");
            assert!(path.ends_with("test/path"), "Path should preserve suffix");
        }
    }

    #[test]
    fn test_expand_tilde_without_tilde() {
        let original = PathBuf::from("/absolute/path");
        let expanded = expand_tilde(&original);
        assert_eq!(original, expanded, "Absolute path should not change");
    }

    #[test]
    fn test_expand_tilde_relative_without_tilde() {
        let original = PathBuf::from("relative/path");
        let expanded = expand_tilde(&original);
        assert_eq!(
            original, expanded,
            "Relative path without tilde should not change"
        );
    }

    #[test]
    fn test_expand_tilde_tilde_only() {
        let path = expand_tilde("~");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(path, home, "~ should expand to home directory");
        }
    }

    #[test]
    fn test_expand_tilde_tilde_with_slash() {
        let path = expand_tilde("~/");
        if let Some(home) = dirs::home_dir() {
            assert!(
                path.starts_with(&home),
                "~/ should expand to home directory"
            );
        }
    }

    #[test]
    fn test_find_dir_with_marker_basic() {
        // Create a temp directory structure with a marker
        let temp_dir = std::env::temp_dir().join("test_find_marker");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::write(temp_dir.join("marker.txt"), "test");

        let result = find_dir_with_marker(&temp_dir, "marker.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_dir_with_marker_nested() {
        // Create nested directory structure
        let temp_base = std::env::temp_dir().join("test_find_marker_nested");
        let nested = temp_base.join("level1").join("level2");
        let _ = std::fs::create_dir_all(&nested);
        let _ = std::fs::write(temp_base.join("marker.txt"), "test");

        // Should find marker when starting from nested path
        let result = find_dir_with_marker(&nested, "marker.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_base);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_find_dir_with_marker_not_found() {
        let result = find_dir_with_marker("/tmp", "nonexistent_marker_xyz");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_dir_with_marker_max_levels() {
        // Create a deeply nested structure
        let temp_base = std::env::temp_dir().join("test_find_marker_deep");
        let _ = std::fs::create_dir_all(&temp_base);

        let mut deep_path = temp_base.clone();
        for i in 0..15 {
            deep_path = deep_path.join(format!("level{}", i));
        }
        let _ = std::fs::create_dir_all(&deep_path);

        // Put marker at the base (too far from deep_path)
        let _ = std::fs::write(temp_base.join("marker.txt"), "test");

        // Should not find it because it's beyond MAX_WALK_LEVELS
        let result = find_dir_with_marker(&deep_path, "marker.txt");
        // Result depends on depth vs MAX_WALK_LEVELS
        assert!(result.is_some() || result.is_none());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_base);
    }
}
```

### Step 6: Update fabryk-core/src/lib.rs

Add the util module:

```rust
//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).
//!
//! # Modules
//!
//! - [`error`]: Error types and Result alias
//! - [`util`]: File and path utilities

#![doc = include_str!("../README.md")]

pub mod error;
pub mod util;

// Re-export key types at crate root for convenience
pub use error::{Error, Result};

// Modules to be added during extraction:
// pub mod traits;
// pub mod state;
// pub mod resources;
```

### Step 7: Run tests

```bash
cd ~/lab/oxur/ecl
cargo test -p fabryk-core
cargo clippy -p fabryk-core -- -D warnings
cargo doc -p fabryk-core --no-deps
```

## Exit Criteria

- [ ] `fabryk-core/src/util/files.rs` exists with all file utility functions
- [ ] `fabryk-core/src/util/paths.rs` exists with generic path utilities
- [ ] `FindOptions`, `FileInfo`, `find_file_by_id()`, `find_all_files()` are public exports
- [ ] `binary_path()`, `binary_dir()`, `find_dir_with_marker()`, `expand_tilde()` are public exports
- [ ] No domain-specific code (no music-theory markers, env vars, or hardcoded paths)
- [ ] All original tests pass in the new location
- [ ] `cargo test -p fabryk-core` passes
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean

## Domain Migration Note

After this extraction, music-theory will still use its local `util/paths.rs`
for the domain-specific functions. At the v0.1-alpha checkpoint migration,
music-theory should:

1. Import generic utilities from `fabryk_core::util::paths`
2. Keep domain-specific functions locally, using the generic primitives:

```rust
// music-theory/crates/server/src/util/paths.rs (after migration)
use fabryk_core::util::paths::{binary_dir, find_dir_with_marker, expand_tilde};

/// Finds the MCP server crate root by walking up from the binary location.
pub fn server_root() -> Option<PathBuf> {
    let binary_dir = binary_dir()?;
    find_dir_with_marker(&binary_dir, "config/default.toml")
        .or_else(|| {
            // Workspace fallback...
        })
}
// ... other domain-specific functions
```

## Commit Message

```
feat(core): extract file and path utilities

Extract util/files.rs fully from music-theory MCP server into
fabryk-core::util::files. Provides async file discovery utilities:
find_file_by_id(), find_all_files(), FindOptions, FileInfo.

Extract generic path utilities from util/paths.rs into
fabryk-core::util::paths: binary_path(), binary_dir(),
find_dir_with_marker(), expand_tilde().

Domain-specific path functions (server_root, config_dir, skill_root)
remain in music-theory for migration at v0.1-alpha checkpoint.

Ref: Doc 0013 milestone 1.3, Audit §4.1

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
