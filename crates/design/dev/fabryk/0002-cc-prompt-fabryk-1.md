---
title: "CC Prompt: Fabryk 1.2 — Error Types & Result (Full Extraction)"
milestone: "1.2"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: ["1.0 Cleanup", "1.1 Workspace scaffold"]
governing-docs: [0011-audit §4.1, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.2 — Error Types & Result (Full Extraction)

## Context

Phase 1 extracts shared types, traits, errors, and utilities into `fabryk-core`.
Milestones 1.0 and 1.1 created the workspace scaffold with a **minimal error stub**
using `thiserror`. This milestone extracts the **full error implementation** from
the music-theory MCP server.

**Key difference from original audit classification:** The music-theory `error.rs`
contains MCP-specific code (`to_mcp_error()` method using `rmcp::model`). This
MCP integration belongs in `fabryk-mcp`, not `fabryk-core`. This milestone
separates the concerns:

- **fabryk-core**: Generic error types (I/O, config, not-found, path, parse)
- **fabryk-mcp**: MCP error mapping via extension trait

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion), when all imports will be updated in a single
coordinated migration.

## Source Files

**Music-theory source** (via symlink):
```
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/error.rs
```

Or directly:
```
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/error.rs
```

**From Audit §1 (File Inventory):** 381 lines (including tests). Types: `Error`,
`ErrorKind`, `Result<T>`. External deps: `thiserror` (for Fabryk), `rmcp` (stays
in music-theory).

**Classification:** Generic (G) for core error types, but MCP integration is
domain-specific.

## Objective

1. Expand `fabryk-core/src/error.rs` with full error variants from music-theory
2. Use `thiserror` derive (not custom struct) for cleaner API
3. Add inspector methods: `is_io()`, `is_not_found()`, `is_config()`
4. Add constructor helpers for all variants
5. Add `#[backtrace]` support via thiserror 2.x
6. **Separate MCP concerns**: Document that `to_mcp_error()` goes to `fabryk-mcp`
7. Bring over the extensive test suite
8. Verify: `cargo test -p fabryk-core` passes

## Architecture Decision: thiserror vs Custom Struct

**Music-theory uses**: Custom `Error` struct with internal `ErrorKind` enum +
manual `Backtrace::capture()` calls.

**Fabryk will use**: `thiserror` derive with `#[backtrace]` attribute.

**Rationale**:
- `thiserror` 2.x supports `#[backtrace]` attribute natively
- Cleaner, more idiomatic API
- Less boilerplate
- Same backtrace capability

## Implementation Steps

### Step 1: Analyse the source file

The music-theory `error.rs` has these variants:

| Variant | Description | Keep in fabryk-core? |
|---------|-------------|---------------------|
| `Io(std::io::Error)` | I/O operations | ✅ Yes |
| `Config(String)` | Configuration errors | ✅ Yes |
| `NotFound { path }` | File not found | ✅ Yes (modify) |
| `NotFoundMsg { message }` | Generic not found | ✅ Yes (merge with above) |
| `InvalidPath { path, reason }` | Path validation | ✅ Yes |
| `ParseError { message }` | Parsing failures | ✅ Yes |
| `SearchError { message }` | Search failures | ✅ Yes (rename to generic) |

**MCP-specific (does NOT go in fabryk-core)**:
- `to_mcp_error()` method — depends on `rmcp::model::{ErrorCode, ErrorData}`

### Step 2: Write `fabryk-core/src/error.rs`

Replace the stub from milestone 1.0 with the full implementation:

```rust
//! Error types for the Fabryk ecosystem.
//!
//! Provides a common `Error` type and `Result<T>` alias used across all Fabryk
//! crates. Uses `thiserror` for derive macros with backtrace support.
//!
//! # Error Categories
//!
//! - **I/O errors**: File operations, network, etc.
//! - **Configuration errors**: Invalid config, missing fields
//! - **Not found errors**: Missing resources (files, concepts, etc.)
//! - **Path errors**: Invalid paths, missing directories
//! - **Parse errors**: Malformed content, invalid format
//! - **Operation errors**: Generic operation failures
//!
//! # MCP Integration
//!
//! MCP-specific error mapping (converting to `ErrorData`) is provided by
//! `fabryk-mcp` via the `McpErrorExt` trait, keeping this crate free of
//! MCP dependencies.

use std::backtrace::Backtrace;
use std::path::PathBuf;

use thiserror::Error;

/// Common error type for Fabryk operations.
///
/// All Fabryk crates use this error type or wrap it in their own domain-specific
/// error types. The variants cover common infrastructure errors; domain-specific
/// errors should use `Operation` with a descriptive message or wrap this type.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error (file operations, network, etc.)
    #[error("I/O error: {0}")]
    Io(
        #[from]
        #[backtrace]
        std::io::Error,
    ),

    /// I/O error with path context.
    #[error("I/O error at {path}: {message}")]
    IoWithPath {
        path: PathBuf,
        message: String,
        #[backtrace]
        backtrace: Backtrace,
    },

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(
        #[from]
        #[backtrace]
        serde_json::Error,
    ),

    /// Resource not found (file, concept, source, etc.)
    #[error("{resource_type} not found: {id}")]
    NotFound {
        resource_type: String,
        id: String,
    },

    /// File not found at specific path.
    #[error("File not found: {}", path.display())]
    FileNotFound { path: PathBuf },

    /// Invalid path.
    #[error("Invalid path {}: {reason}", path.display())]
    InvalidPath { path: PathBuf, reason: String },

    /// Parse error (malformed content, invalid format).
    #[error("Parse error: {0}")]
    Parse(String),

    /// Generic operation error (escape hatch for domain-specific errors).
    #[error("{0}")]
    Operation(String),
}

impl Error {
    // ========================================================================
    // Constructor helpers
    // ========================================================================

    /// Create an I/O error.
    ///
    /// This is useful when you have an `std::io::Error` and want to convert
    /// it explicitly (as opposed to using `?` with `From` conversion).
    pub fn io(err: std::io::Error) -> Self {
        Self::Io(err)
    }

    /// Create an I/O error with path context.
    pub fn io_with_path(err: std::io::Error, path: impl Into<PathBuf>) -> Self {
        Self::IoWithPath {
            path: path.into(),
            message: err.to_string(),
            backtrace: Backtrace::capture(),
        }
    }

    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a not-found error with resource type and ID.
    pub fn not_found(resource_type: impl Into<String>, id: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: resource_type.into(),
            id: id.into(),
        }
    }

    /// Create a file-not-found error.
    pub fn file_not_found(path: impl Into<PathBuf>) -> Self {
        Self::FileNotFound { path: path.into() }
    }

    /// Create a not-found error with just a message.
    ///
    /// This is a convenience method for when you don't have a specific
    /// resource type, or the message already contains the context.
    pub fn not_found_msg(msg: impl Into<String>) -> Self {
        Self::NotFound {
            resource_type: "Resource".to_string(),
            id: msg.into(),
        }
    }

    /// Create an invalid path error.
    pub fn invalid_path(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::InvalidPath {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a parse error.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse(msg.into())
    }

    /// Create an operation error (generic domain-specific error).
    pub fn operation(msg: impl Into<String>) -> Self {
        Self::Operation(msg.into())
    }

    // ========================================================================
    // Inspector methods
    // ========================================================================

    /// Check if this is an I/O error.
    pub fn is_io(&self) -> bool {
        matches!(self, Self::Io(_) | Self::IoWithPath { .. })
    }

    /// Check if this is a not-found error (any variant).
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. } | Self::FileNotFound { .. })
    }

    /// Check if this is a configuration error.
    pub fn is_config(&self) -> bool {
        matches!(self, Self::Config(_))
    }

    /// Check if this is a path-related error.
    pub fn is_path_error(&self) -> bool {
        matches!(
            self,
            Self::InvalidPath { .. } | Self::FileNotFound { .. } | Self::IoWithPath { .. }
        )
    }

    /// Check if this is a parse error.
    pub fn is_parse(&self) -> bool {
        matches!(self, Self::Parse(_))
    }
}

/// Result type alias for Fabryk operations.
pub type Result<T> = std::result::Result<T, Error>;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as StdError;

    // ------------------------------------------------------------------------
    // Constructor tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_io_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert!(err.is_io());
        assert!(!err.is_not_found());
        assert!(!err.is_config());
        assert!(err.to_string().contains("I/O error"));
    }

    #[test]
    fn test_error_io_constructor() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::io(io_err);
        assert!(err.is_io());
        assert!(err.to_string().contains("I/O error"));
        assert!(err.to_string().contains("file not found"));
    }

    #[test]
    fn test_error_io_with_path() {
        let io_err =
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        let path = PathBuf::from("/test/path.txt");
        let err = Error::io_with_path(io_err, &path);
        assert!(err.is_io());
        assert!(err.is_path_error());
        let msg = err.to_string();
        assert!(msg.contains("I/O error at"));
        assert!(msg.contains("/test/path.txt"));
        assert!(msg.contains("permission denied"));
    }

    #[test]
    fn test_error_config() {
        let err = Error::config("invalid configuration");
        assert!(err.is_config());
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        assert!(err.to_string().contains("Configuration error"));
        assert!(err.to_string().contains("invalid configuration"));
    }

    #[test]
    fn test_error_not_found() {
        let err = Error::not_found("Concept", "major-triad");
        assert!(err.is_not_found());
        assert!(!err.is_io());
        assert!(!err.is_config());
        let msg = err.to_string();
        assert!(msg.contains("Concept not found"));
        assert!(msg.contains("major-triad"));
    }

    #[test]
    fn test_error_file_not_found() {
        let path = PathBuf::from("/missing/file.txt");
        let err = Error::file_not_found(&path);
        assert!(err.is_not_found());
        assert!(err.is_path_error());
        assert!(!err.is_io());
        let msg = err.to_string();
        assert!(msg.contains("File not found"));
        assert!(msg.contains("/missing/file.txt"));
    }

    #[test]
    fn test_error_not_found_msg() {
        let err = Error::not_found_msg("file with id 'xyz' not in cache");
        assert!(err.is_not_found());
        assert!(!err.is_io());
        let msg = err.to_string();
        assert!(msg.contains("Resource not found"));
        assert!(msg.contains("file with id 'xyz' not in cache"));
    }

    #[test]
    fn test_error_invalid_path() {
        let path = PathBuf::from("/bad/path");
        let err = Error::invalid_path(&path, "invalid characters");
        assert!(err.is_path_error());
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        let msg = err.to_string();
        assert!(msg.contains("Invalid path"));
        assert!(msg.contains("/bad/path"));
        assert!(msg.contains("invalid characters"));
    }

    #[test]
    fn test_error_parse() {
        let err = Error::parse("syntax error at line 5");
        assert!(err.is_parse());
        assert!(!err.is_io());
        assert!(!err.is_config());
        assert!(err.to_string().contains("Parse error"));
        assert!(err.to_string().contains("syntax error at line 5"));
    }

    #[test]
    fn test_error_operation() {
        let err = Error::operation("index corrupted");
        assert!(!err.is_io());
        assert!(!err.is_not_found());
        assert!(!err.is_config());
        assert!(err.to_string().contains("index corrupted"));
    }

    // ------------------------------------------------------------------------
    // From implementations
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection lost");
        let err: Error = io_err.into();
        assert!(err.is_io());
        assert!(err.to_string().contains("connection lost"));
    }

    #[test]
    fn test_error_from_json_error() {
        let json_str = "{ invalid json }";
        let json_err = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Json(_)));
        assert!(err.to_string().contains("JSON error"));
    }

    // ------------------------------------------------------------------------
    // Error trait implementation
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_source_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let err: Error = io_err.into();
        assert!(err.source().is_some());
    }

    #[test]
    fn test_error_source_non_io() {
        let err = Error::config("test");
        assert!(err.source().is_none());
    }

    // ------------------------------------------------------------------------
    // Display tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_error_display_all_variants() {
        let errors = vec![
            Error::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "io")),
            Error::io_with_path(
                std::io::Error::new(std::io::ErrorKind::NotFound, "io"),
                "/path",
            ),
            Error::config("config"),
            Error::not_found("Type", "id"),
            Error::file_not_found("/path"),
            Error::invalid_path("/path", "reason"),
            Error::parse("parse"),
            Error::operation("operation"),
        ];

        for err in errors {
            let display = err.to_string();
            assert!(
                !display.is_empty(),
                "Display should produce non-empty string for {:?}",
                err
            );
        }
    }

    // ------------------------------------------------------------------------
    // Result alias
    // ------------------------------------------------------------------------

    #[test]
    fn test_result_alias() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.unwrap(), 42);

        let err: Result<i32> = Err(Error::config("bad"));
        assert!(err.is_err());
    }
}
```

### Step 3: Update `fabryk-core/src/lib.rs`

Ensure the error module is exported:

```rust
//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).
//!
//! # Modules
//!
//! - [`error`]: Error types and Result alias

#![doc = include_str!("../README.md")]

pub mod error;

// Re-export key types at crate root for convenience
pub use error::{Error, Result};

// Modules to be added during extraction:
// pub mod util;
// pub mod traits;
// pub mod state;
// pub mod resources;
```

### Step 4: Update `fabryk-core/Cargo.toml`

Ensure dependencies are correct (should already be from milestone 1.1):

```toml
[dependencies]
# Error handling
thiserror = { workspace = true }

# Serialization (for JSON error conversion)
serde = { workspace = true }
serde_json = { workspace = true }

# ... other deps unchanged
```

### Step 5: Document MCP error extension for `fabryk-mcp`

Create a note for milestone 5.x (MCP extraction) about the error extension:

The `to_mcp_error()` method from music-theory depends on `rmcp::model`. When
extracting `fabryk-mcp`, add an extension trait:

```rust
// fabryk-mcp/src/error_ext.rs (future milestone)

use fabryk_core::Error;
use rmcp::model::{ErrorCode, ErrorData};

/// Extension trait for converting Fabryk errors to MCP errors.
pub trait McpErrorExt {
    /// Convert to MCP ErrorData with appropriate error code.
    fn to_mcp_error(&self, context: &str) -> ErrorData;
}

impl McpErrorExt for Error {
    fn to_mcp_error(&self, context: &str) -> ErrorData {
        let (code, msg) = if self.is_not_found() {
            (ErrorCode::RESOURCE_NOT_FOUND, format!("Not found: {}", self))
        } else if self.is_config() {
            (ErrorCode::INVALID_PARAMS, format!("Configuration error: {}", self))
        } else {
            (ErrorCode::INTERNAL_ERROR, format!("{}: {}", context, self))
        };
        ErrorData::new(code, msg, None)
    }
}
```

This keeps `fabryk-core` free of MCP dependencies while preserving the useful
error mapping functionality.

### Step 6: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-core
cargo test -p fabryk-core
cargo clippy -p fabryk-core -- -D warnings
cargo doc -p fabryk-core --no-deps
```

## Exit Criteria

- [ ] `fabryk-core/src/error.rs` has full `Error` enum with all variants
- [ ] Uses `thiserror` derive (not custom struct)
- [ ] Has `#[backtrace]` on `#[from]` conversions
- [ ] Constructor helpers: `io()`, `io_with_path()`, `config()`, `not_found()`,
      `not_found_msg()`, `file_not_found()`, `invalid_path()`, `parse()`,
      `operation()`
- [ ] Inspector methods: `is_io()`, `is_not_found()`, `is_config()`,
      `is_path_error()`, `is_parse()`
- [ ] `From<std::io::Error>` and `From<serde_json::Error>` implementations
- [ ] No MCP dependencies in `fabryk-core`
- [ ] Note added for `fabryk-mcp` error extension (future milestone)
- [ ] `cargo test -p fabryk-core` passes (all tests from music-theory adapted)
- [ ] `cargo clippy -p fabryk-core -- -D warnings` clean
- [ ] `cargo doc -p fabryk-core --no-deps` generates clean documentation

## Music-Theory Migration Note

After `fabryk-core` error types are ready, music-theory MCP server can:

1. Replace `use crate::error::{Error, Result}` with `use fabryk_core::{Error, Result}`
2. Keep `to_mcp_error()` as a local extension until `fabryk-mcp` is ready
3. Remove the custom `ErrorKind` enum and backtrace handling

This is a low-risk migration since the API is compatible.

## Commit Message

```
feat(core): extract full error types from music-theory

Expand fabryk-core error.rs with complete implementation:
- All error variants: Io, IoWithPath, Config, Json, NotFound,
  FileNotFound, InvalidPath, Parse, Operation
- Constructor helpers for all variants
- Inspector methods: is_io(), is_not_found(), is_config(), etc.
- Backtrace support via thiserror #[backtrace]
- Comprehensive test suite (adapted from music-theory)

MCP-specific error mapping (to_mcp_error) documented for fabryk-mcp
extraction in future milestone.

Ref: Doc 0013 milestone 1.2, Audit §4.1

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
