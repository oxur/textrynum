# Phase 1 Implementation Plan: ECL Foundation

**Version:** 1.0
**Date:** 2026-01-22
**Phase:** 1 - Foundation
**Target Duration:** ~2 weeks
**Status:** Ready for Implementation

---

## Overview

This document provides detailed, actionable implementation steps for Phase 1 of the ECL project. Each stage includes:

- **Detailed Implementation Steps** - Specific coding tasks
- **File Contents Skeleton** - Complete file structures with signatures
- **Dependencies** - What must be completed before starting
- **Testing Strategy** - Specific tests and test data
- **Potential Blockers** - Risks and mitigation strategies
- **Estimated Effort** - Rough time per stage

**Critical Guidelines to Follow:**

1. **Load anti-patterns first**: Always read `assets/ai/ai-rust/guides/11-anti-patterns.md` before writing code
2. **Test coverage**: Maintain ≥95% coverage (see `assets/ai/CLAUDE-CODE-COVERAGE.md`)
3. **Error handling**: Use `thiserror` for errors, avoid `unwrap()` in library code
4. **Parameters**: Use `&str` not `&String`, `&[T]` not `&Vec<T>`
5. **Async**: Never use blocking I/O in async functions
6. **Public types**: Use `#[non_exhaustive]` on public enums and structs

---

## Stage 1.1: Project Scaffolding

**Goal:** Establish project structure, dependencies, and build configuration.

**Dependencies:** None (first stage)

### Detailed Implementation Steps

#### Step 1: Initialize Workspace Structure

```bash
# Create workspace root Cargo.toml
# Create crate directories
# Set up git structure if not already done
```

**Files to create:**

1. **`Cargo.toml` (workspace root)**

```toml
[workspace]
resolver = "2"
members = [
    "crates/ecl-core",
    "crates/ecl-steps",
    "crates/ecl-workflows",
    "crates/ecl-cli",
]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.75"  # Verify current stable
authors = ["ECL Contributors"]
license = "Apache-2.0"
repository = "https://github.com/oxur/ecl"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Restate SDK
restate-sdk = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "chrono", "uuid"] }

# LLM (verify version on crates.io)
llm = "0.2"

# Resilience
backon = "1"
failsafe = "1"

# Configuration
confyg = "0.3"

# Logging
twyg = "0.6"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Async trait support
async-trait = "0.1"

# CLI
clap = { version = "4", features = ["derive", "env"] }

# HTTP
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }

# Testing
proptest = "1"
mockall = "0.12"
tempfile = "3"

[profile.dev]
opt-level = 0
debug = true

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = true

[profile.test]
opt-level = 1
```

2. **`rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-unknown-linux-gnu", "x86_64-apple-darwin", "aarch64-apple-darwin"]
```

#### Step 2: Create Crate Scaffolding

For each crate, create:

**`crates/ecl-core/Cargo.toml`**

```toml
[package]
name = "ecl-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
# Workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
anyhow = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
async-trait = { workspace = true }
sqlx = { workspace = true }
backon = { workspace = true }
failsafe = { workspace = true }
twyg = { workspace = true }
tracing = { workspace = true }
llm = { workspace = true }

[dev-dependencies]
proptest = { workspace = true }
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

**`crates/ecl-core/src/lib.rs`**

```rust
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Core Library
//!
//! Core types, traits, and utilities for the ECL workflow orchestration system.

// Public modules
pub mod error;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use types::{StepId, WorkflowId, WorkflowState};
```

**Similar structure for:**
- `crates/ecl-steps/Cargo.toml` and `src/lib.rs`
- `crates/ecl-workflows/Cargo.toml` and `src/lib.rs`
- `crates/ecl-cli/Cargo.toml` and `src/main.rs`

#### Step 3: Development Tooling

**`justfile`**

```just
# ECL Development Commands

# Default: show available commands
default:
    @just --list

# Run all tests
test:
    cargo test --workspace --all-features

# Run tests with coverage
coverage:
    cargo llvm-cov --workspace --all-features --html
    @echo "Coverage report: target/llvm-cov/html/index.html"

# Check code quality
lint:
    cargo clippy --workspace --all-features --all-targets -- -D warnings
    cargo fmt --all -- --check

# Format code
format:
    cargo fmt --all

# Build all crates
build:
    cargo build --workspace --all-features

# Build for release
build-release:
    cargo build --workspace --all-features --release

# Run development environment (Restate + app)
dev:
    docker compose up -d restate
    cargo run --bin ecl-workflows

# Stop development environment
dev-stop:
    docker compose down

# Clean build artifacts
clean:
    cargo clean
    rm -rf target/

# Check for dependency updates
outdated:
    cargo outdated --workspace

# Run security audit
audit:
    cargo audit

# Generate documentation
docs:
    cargo doc --workspace --all-features --no-deps --open
```

**`.env.example`**

```bash
# ECL Configuration

# Environment
ECL_ENV=development
RUST_LOG=info,ecl=debug

# Database
DATABASE_URL=sqlite://ecl.db
# DATABASE_URL=postgres://ecl:password@localhost/ecl

# Claude API
ANTHROPIC_API_KEY=your_api_key_here

# Restate
RESTATE_URL=http://localhost:8080

# API Server
ECL_API_PORT=3000
ECL_API_KEY=dev_api_key_change_in_production

# Artifact Storage
ECL_ARTIFACT_PATH=./artifacts
```

**`docker-compose.yml`**

```yaml
version: '3.8'

services:
  restate:
    image: restatedev/restate:latest
    ports:
      - "8080:8080"  # Ingress
      - "9070:9070"  # Admin
    environment:
      - RUST_LOG=info
    volumes:
      - restate-data:/data

  postgres:
    image: postgres:16-alpine
    ports:
      - "5432:5432"
    environment:
      POSTGRES_DB: ecl
      POSTGRES_USER: ecl
      POSTGRES_PASSWORD: ecl_dev_password
    volumes:
      - postgres-data:/var/lib/postgresql/data

volumes:
  restate-data:
  postgres-data:
```

**`.gitignore`**

```gitignore
# Rust
/target/
**/*.rs.bk
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo

# Environment
.env
.env.local

# Data
*.db
*.db-shm
*.db-wal
/artifacts/

# Coverage
target/llvm-cov/
*.profraw
*.profdata

# OS
.DS_Store
Thumbs.db
```

#### Step 4: Basic Documentation

**`README.md`** (update existing or create)

```markdown
# ECL - Event-driven Codebase Lifecycle

Rust-based AI workflow orchestration system built on Restate.

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Docker and Docker Compose
- Just command runner: `cargo install just`

### Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and configure
3. Start dependencies: `just dev-stop && docker compose up -d`
4. Run tests: `just test`
5. Build: `just build`

### Development

```bash
# Run tests
just test

# Run with coverage
just coverage

# Format and lint
just format
just lint

# Start dev environment
just dev
```

## Project Structure

```
ecl/
├── crates/
│   ├── ecl-core/      # Core types, traits, error handling
│   ├── ecl-steps/     # Step implementations
│   ├── ecl-workflows/ # Restate workflow definitions
│   └── ecl-cli/       # Command-line interface
├── docs/              # Documentation
└── migrations/        # Database migrations
```

## Documentation

- [Getting Started](docs/getting-started.md)
- [Architecture](crates/design/docs/05-active/0004-ecl-project-plan.md)
- [API Documentation](https://docs.rs/ecl)

## License

Apache 2.0
```

### Testing Strategy

**Test files to create:**

1. **Workspace builds**: `cargo build --workspace`
2. **Linting passes**: `cargo clippy --workspace -- -D warnings`
3. **Formatting**: `cargo fmt --all -- --check`
4. **Dependency resolution**: All workspace dependencies resolve

**Test data needed:** None for this stage

**Tests to write:**

```rust
// crates/ecl-core/src/lib.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crate_compiles() {
        // This test just ensures the crate compiles
        // More substantive tests come in later stages
    }
}
```

### Potential Blockers

1. **Dependency version conflicts**
   - **Mitigation**: Pin workspace dependencies carefully
   - **Mitigation**: Test build before proceeding

2. **Restate SDK API changes**
   - **Mitigation**: Check crates.io for current version
   - **Mitigation**: Review Restate documentation

3. **`llm` crate availability/API**
   - **Mitigation**: Verify crate exists and supports Claude
   - **Fallback**: Implement direct HTTP client to Claude API

### Acceptance Criteria

- [ ] `cargo build --workspace` succeeds with no errors
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] All crates compile and link correctly
- [ ] Restate SDK dependency resolves
- [ ] `just test` runs successfully
- [ ] Documentation builds with `cargo doc --workspace --no-deps`
- [ ] Docker Compose brings up Restate server

### Estimated Effort

**Time:** 2-3 hours

**Breakdown:**
- Cargo.toml setup: 30 min
- Crate scaffolding: 45 min
- Tooling (justfile, docker-compose): 30 min
- Documentation: 30 min
- Testing and validation: 30 min

---

## Stage 1.2: Core Types and Error Handling

**Goal:** Define foundational types that all other code builds upon.

**Dependencies:** Stage 1.1 (project scaffolding)

### Detailed Implementation Steps

#### Step 1: Define Error Types

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/11-anti-patterns.md`
2. Load `assets/ai/ai-rust/guides/03-error-handling.md`
3. Review project error handling patterns

**`crates/ecl-core/src/error.rs`**

```rust
//! Error types for ECL core library.

use std::fmt;

/// Errors that can occur during ECL workflow execution.
///
/// All error variants are marked with `#[non_exhaustive]` to allow
/// adding new error types without breaking changes.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// LLM provider error (Claude API failures, rate limits, etc.)
    #[error("LLM error: {message}")]
    Llm {
        /// Human-readable error message
        message: String,
        /// Whether this error can be retried
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Step validation error
    #[error("Validation error: {message}")]
    Validation {
        /// Field or aspect that failed validation
        field: Option<String>,
        /// What went wrong
        message: String,
    },

    /// I/O error (file operations, network, etc.)
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Step execution timeout
    #[error("Step timed out after {seconds}s")]
    Timeout {
        /// Timeout duration in seconds
        seconds: u64,
    },

    /// Maximum revision iterations exceeded
    #[error("Maximum revisions exceeded: {attempts} attempts")]
    MaxRevisionsExceeded {
        /// Number of revision attempts made
        attempts: u32,
    },

    /// Database error
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Configuration error
    #[error("Configuration error: {message}")]
    Config {
        /// What configuration is problematic
        message: String,
    },

    /// Workflow not found
    #[error("Workflow not found: {id}")]
    WorkflowNotFound {
        /// Workflow ID that was not found
        id: String,
    },

    /// Step not found
    #[error("Step not found: {id}")]
    StepNotFound {
        /// Step ID that was not found
        id: String,
    },
}

/// Convenience `Result` type alias for ECL operations.
///
/// This is the standard Result type used throughout the ECL codebase.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Returns whether this error is retryable.
    ///
    /// Retryable errors include transient failures like rate limits,
    /// network timeouts, and temporary service unavailability.
    pub fn is_retryable(&self) -> bool {
        match self {
            Error::Llm { .. } => true,  // LLM errors are generally retryable
            Error::Io(_) => true,
            Error::Database(_) => true,  // Database errors may be transient
            Error::Timeout { .. } => true,
            Error::Validation { .. } => false,  // Validation errors are permanent
            Error::MaxRevisionsExceeded { .. } => false,
            Error::Serialization(_) => false,
            Error::Config { .. } => false,
            Error::WorkflowNotFound { .. } => false,
            Error::StepNotFound { .. } => false,
        }
    }

    /// Creates a new LLM error with a message.
    pub fn llm<S: Into<String>>(message: S) -> Self {
        Error::Llm {
            message: message.into(),
            source: None,
        }
    }

    /// Creates a new LLM error with a message and source error.
    pub fn llm_with_source<S, E>(message: S, source: E) -> Self
    where
        S: Into<String>,
        E: std::error::Error + Send + Sync + 'static,
    {
        Error::Llm {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    /// Creates a new validation error.
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Error::Validation {
            field: None,
            message: message.into(),
        }
    }

    /// Creates a new validation error with a field name.
    pub fn validation_field<F, M>(field: F, message: M) -> Self
    where
        F: Into<String>,
        M: Into<String>,
    {
        Error::Validation {
            field: Some(field.into()),
            message: message.into(),
        }
    }

    /// Creates a new configuration error.
    pub fn config<S: Into<String>>(message: S) -> Self {
        Error::Config {
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::llm("Rate limit exceeded");
        assert_eq!(err.to_string(), "LLM error: Rate limit exceeded");
    }

    #[test]
    fn test_retryable_classification() {
        assert!(Error::llm("test").is_retryable());
        assert!(Error::Timeout { seconds: 30 }.is_retryable());
        assert!(!Error::validation("test").is_retryable());
        assert!(!Error::MaxRevisionsExceeded { attempts: 3 }.is_retryable());
    }

    #[test]
    fn test_validation_error_with_field() {
        let err = Error::validation_field("topic", "must not be empty");
        match err {
            Error::Validation { field, message } => {
                assert_eq!(field, Some("topic".to_string()));
                assert_eq!(message, "must not be empty");
            }
            _ => panic!("Expected Validation error"),
        }
    }

    #[test]
    fn test_max_revisions_exceeded() {
        let err = Error::MaxRevisionsExceeded { attempts: 5 };
        assert_eq!(
            err.to_string(),
            "Maximum revisions exceeded: 5 attempts"
        );
    }

    #[test]
    fn test_error_implements_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }
}
```

#### Step 2: Define Core Type Newtypes

**`crates/ecl-core/src/types/ids.rs`**

```rust
//! Unique identifier types for workflows and steps.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a workflow instance.
///
/// Internally represented as a UUID v4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(Uuid);

impl WorkflowId {
    /// Creates a new random workflow ID.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::WorkflowId;
    ///
    /// let id = WorkflowId::new();
    /// println!("Workflow ID: {}", id);
    /// ```
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a workflow ID from a UUID.
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }

    /// Converts to the inner UUID.
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WorkflowId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<WorkflowId> for Uuid {
    fn from(id: WorkflowId) -> Self {
        id.0
    }
}

impl std::str::FromStr for WorkflowId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

/// Unique identifier for a workflow step.
///
/// Step IDs are human-readable strings like "generate", "critique", "revise".
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(String);

impl StepId {
    /// Creates a new step ID from a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::StepId;
    ///
    /// let id = StepId::new("generate");
    /// assert_eq!(id.as_str(), "generate");
    /// ```
    pub fn new<S: Into<String>>(id: S) -> Self {
        Self(id.into())
    }

    /// Returns the step ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for StepId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for StepId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for StepId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for StepId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_id_new() {
        let id1 = WorkflowId::new();
        let id2 = WorkflowId::new();
        assert_ne!(id1, id2, "Each new ID should be unique");
    }

    #[test]
    fn test_workflow_id_display() {
        let uuid = Uuid::new_v4();
        let id = WorkflowId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
    }

    #[test]
    fn test_workflow_id_roundtrip_serialization() {
        let id = WorkflowId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_workflow_id_from_str() {
        let uuid = Uuid::new_v4();
        let id: WorkflowId = uuid.to_string().parse().unwrap();
        assert_eq!(id.as_uuid(), &uuid);
    }

    #[test]
    fn test_step_id_creation() {
        let id = StepId::new("generate");
        assert_eq!(id.as_str(), "generate");
    }

    #[test]
    fn test_step_id_from_string() {
        let id = StepId::from("critique".to_string());
        assert_eq!(id.as_str(), "critique");
    }

    #[test]
    fn test_step_id_from_str() {
        let id = StepId::from("revise");
        assert_eq!(id.as_str(), "revise");
    }

    #[test]
    fn test_step_id_display() {
        let id = StepId::new("test-step");
        assert_eq!(id.to_string(), "test-step");
    }

    #[test]
    fn test_step_id_roundtrip_serialization() {
        let id = StepId::new("my-step");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: StepId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }
}
```

#### Step 3: Define Step Result and Workflow State

**`crates/ecl-core/src/types/step_result.rs`**

```rust
//! Step execution result types.

use serde::{Deserialize, Serialize};

/// The result of executing a workflow step.
///
/// Steps can succeed, request revision, or fail. This type captures
/// all possible outcomes along with their associated data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StepResult<T> {
    /// Step completed successfully.
    Success(T),

    /// Step completed but requests revision of prior work.
    ///
    /// This variant is used in feedback loops where a critique step
    /// determines that prior output needs improvement.
    NeedsRevision {
        /// The output from this step (e.g., critique text)
        output: T,
        /// Feedback explaining what needs to be revised
        feedback: String,
    },

    /// Step failed with an error.
    Failed {
        /// The error that caused the failure
        error: String,
        /// Whether this failure can be retried
        retryable: bool,
    },
}

impl<T> StepResult<T> {
    /// Returns `true` if the result is `Success`.
    pub fn is_success(&self) -> bool {
        matches!(self, StepResult::Success(_))
    }

    /// Returns `true` if the result is `NeedsRevision`.
    pub fn is_needs_revision(&self) -> bool {
        matches!(self, StepResult::NeedsRevision { .. })
    }

    /// Returns `true` if the result is `Failed`.
    pub fn is_failed(&self) -> bool {
        matches!(self, StepResult::Failed { .. })
    }

    /// Returns `true` if the result is a retryable failure.
    pub fn is_retryable(&self) -> bool {
        match self {
            StepResult::Failed { retryable, .. } => *retryable,
            _ => false,
        }
    }

    /// Extracts the success value, panicking if not successful.
    ///
    /// # Panics
    ///
    /// Panics if the result is not `Success`.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::StepResult;
    ///
    /// let result = StepResult::Success(42);
    /// assert_eq!(result.unwrap(), 42);
    /// ```
    pub fn unwrap(self) -> T {
        match self {
            StepResult::Success(value) => value,
            StepResult::NeedsRevision { .. } => panic!("called `unwrap()` on NeedsRevision"),
            StepResult::Failed { error, .. } => panic!("called `unwrap()` on Failed: {}", error),
        }
    }

    /// Returns the success value, or a default if not successful.
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            StepResult::Success(value) => value,
            _ => default,
        }
    }

    /// Returns the success value, or computes it from a closure.
    pub fn unwrap_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        match self {
            StepResult::Success(value) => value,
            _ => f(),
        }
    }

    /// Maps a `StepResult<T>` to `StepResult<U>` by applying a function to the success value.
    pub fn map<U, F>(self, f: F) -> StepResult<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            StepResult::Success(value) => StepResult::Success(f(value)),
            StepResult::NeedsRevision { output, feedback } => StepResult::NeedsRevision {
                output: f(output),
                feedback,
            },
            StepResult::Failed { error, retryable } => StepResult::Failed { error, retryable },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_variant() {
        let result = StepResult::Success(42);
        assert!(result.is_success());
        assert!(!result.is_needs_revision());
        assert!(!result.is_failed());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_needs_revision_variant() {
        let result: StepResult<String> = StepResult::NeedsRevision {
            output: "critique".to_string(),
            feedback: "needs work".to_string(),
        };
        assert!(!result.is_success());
        assert!(result.is_needs_revision());
        assert!(!result.is_failed());
        assert!(!result.is_retryable());
    }

    #[test]
    fn test_failed_variant() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "timeout".to_string(),
            retryable: true,
        };
        assert!(!result.is_success());
        assert!(!result.is_needs_revision());
        assert!(result.is_failed());
        assert!(result.is_retryable());
    }

    #[test]
    fn test_map() {
        let result = StepResult::Success(2);
        let mapped = result.map(|x| x * 2);
        assert_eq!(mapped.unwrap(), 4);
    }

    #[test]
    fn test_unwrap_or() {
        let success = StepResult::Success(10);
        assert_eq!(success.unwrap_or(0), 10);

        let failed: StepResult<i32> = StepResult::Failed {
            error: "error".to_string(),
            retryable: false,
        };
        assert_eq!(failed.unwrap_or(0), 0);
    }

    #[test]
    fn test_serialization() {
        let result = StepResult::Success("data".to_string());
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: StepResult<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(result, deserialized);
    }

    #[test]
    #[should_panic(expected = "called `unwrap()` on Failed")]
    fn test_unwrap_panics_on_failed() {
        let result: StepResult<i32> = StepResult::Failed {
            error: "test error".to_string(),
            retryable: false,
        };
        result.unwrap();
    }
}
```

**`crates/ecl-core/src/types/workflow_state.rs`**

```rust
//! Workflow state tracking types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{StepId, WorkflowId};

/// The current state of a workflow instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkflowState {
    /// Workflow has been created but not yet started.
    Pending,

    /// Workflow is actively executing steps.
    Running,

    /// Workflow is waiting for a revision to complete.
    WaitingForRevision,

    /// Workflow has completed successfully.
    Completed,

    /// Workflow has failed and cannot proceed.
    Failed,
}

impl WorkflowState {
    /// Returns `true` if the workflow is in a terminal state (Completed or Failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, WorkflowState::Completed | WorkflowState::Failed)
    }

    /// Returns `true` if the workflow is active (Running or WaitingForRevision).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            WorkflowState::Running | WorkflowState::WaitingForRevision
        )
    }
}

impl std::fmt::Display for WorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowState::Pending => write!(f, "pending"),
            WorkflowState::Running => write!(f, "running"),
            WorkflowState::WaitingForRevision => write!(f, "waiting_for_revision"),
            WorkflowState::Completed => write!(f, "completed"),
            WorkflowState::Failed => write!(f, "failed"),
        }
    }
}

/// Metadata about a step execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepMetadata {
    /// Unique identifier for this step
    pub step_id: StepId,

    /// When this step started executing
    pub started_at: DateTime<Utc>,

    /// When this step completed (if finished)
    pub completed_at: Option<DateTime<Utc>>,

    /// Attempt number (1-indexed)
    pub attempt: u32,

    /// Number of LLM tokens used (if applicable)
    pub llm_tokens_used: Option<u64>,
}

impl StepMetadata {
    /// Creates new step metadata for an initial execution.
    pub fn new(step_id: StepId) -> Self {
        Self {
            step_id,
            started_at: Utc::now(),
            completed_at: None,
            attempt: 1,
            llm_tokens_used: None,
        }
    }

    /// Marks this step as completed.
    pub fn mark_completed(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Returns the duration of this step execution.
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at
            .map(|end| end.signed_duration_since(self.started_at))
    }

    /// Returns `true` if the step has completed.
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_state_terminal() {
        assert!(WorkflowState::Completed.is_terminal());
        assert!(WorkflowState::Failed.is_terminal());
        assert!(!WorkflowState::Running.is_terminal());
        assert!(!WorkflowState::Pending.is_terminal());
        assert!(!WorkflowState::WaitingForRevision.is_terminal());
    }

    #[test]
    fn test_workflow_state_active() {
        assert!(WorkflowState::Running.is_active());
        assert!(WorkflowState::WaitingForRevision.is_active());
        assert!(!WorkflowState::Completed.is_active());
        assert!(!WorkflowState::Failed.is_active());
        assert!(!WorkflowState::Pending.is_active());
    }

    #[test]
    fn test_workflow_state_display() {
        assert_eq!(WorkflowState::Pending.to_string(), "pending");
        assert_eq!(WorkflowState::Running.to_string(), "running");
        assert_eq!(
            WorkflowState::WaitingForRevision.to_string(),
            "waiting_for_revision"
        );
        assert_eq!(WorkflowState::Completed.to_string(), "completed");
        assert_eq!(WorkflowState::Failed.to_string(), "failed");
    }

    #[test]
    fn test_step_metadata_new() {
        let step_id = StepId::new("test");
        let metadata = StepMetadata::new(step_id.clone());

        assert_eq!(metadata.step_id, step_id);
        assert_eq!(metadata.attempt, 1);
        assert!(metadata.completed_at.is_none());
        assert!(!metadata.is_completed());
    }

    #[test]
    fn test_step_metadata_mark_completed() {
        let mut metadata = StepMetadata::new(StepId::new("test"));
        assert!(!metadata.is_completed());

        metadata.mark_completed();
        assert!(metadata.is_completed());
        assert!(metadata.completed_at.is_some());
        assert!(metadata.duration().is_some());
    }

    #[test]
    fn test_step_metadata_serialization() {
        let metadata = StepMetadata::new(StepId::new("test"));
        let json = serde_json::to_string(&metadata).unwrap();
        let deserialized: StepMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(metadata.step_id, deserialized.step_id);
        assert_eq!(metadata.attempt, deserialized.attempt);
    }
}
```

#### Step 4: Update Module Structure

**`crates/ecl-core/src/types/mod.rs`**

```rust
//! Core types for ECL workflows.

mod ids;
mod step_result;
mod workflow_state;

pub use ids::{StepId, WorkflowId};
pub use step_result::StepResult;
pub use workflow_state::{StepMetadata, WorkflowState};
```

**Update `crates/ecl-core/src/lib.rs`**

```rust
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Core Library
//!
//! Core types, traits, and utilities for the ECL workflow orchestration system.

pub mod error;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use types::{StepId, StepMetadata, StepResult, WorkflowId, WorkflowState};
```

### Testing Strategy

**Test coverage goal:** ≥95% for all types and error handling

**Tests to write:**

1. **Error type tests** (see inline tests above):
   - Error display formatting
   - Retryability classification
   - Error variant construction
   - Send + Sync bounds
   - Serialization (if needed)

2. **ID type tests**:
   - UUID generation uniqueness
   - String parsing
   - Display formatting
   - Serialization roundtrips

3. **StepResult tests**:
   - All variant constructors
   - Is methods
   - Map/unwrap operations
   - Serialization

4. **WorkflowState tests**:
   - State transitions
   - Terminal/active checks
   - Display formatting

**Test data needed:**
- Sample UUIDs
- Sample error messages
- Sample step IDs

**Additional tests:**

```rust
// Property-based tests for ID generation
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_workflow_id_roundtrip(uuid in any::<u128>()) {
            let uuid = Uuid::from_u128(uuid);
            let id = WorkflowId::from_uuid(uuid);
            assert_eq!(id.into_uuid(), uuid);
        }

        #[test]
        fn test_step_id_roundtrip(s in "\\PC+") {
            let id = StepId::new(s.clone());
            assert_eq!(id.as_str(), &s);
        }
    }
}
```

### Potential Blockers

1. **Serde serialization issues**
   - **Mitigation**: Test serialization early
   - **Mitigation**: Use standard derives

2. **Type complexity**
   - **Mitigation**: Keep types simple and focused
   - **Mitigation**: Use builder pattern if needed

### Acceptance Criteria

- [ ] All types are `Clone`, `Debug`, `Serialize`, `Deserialize` where appropriate
- [ ] Error types implement `std::error::Error` with `thiserror`
- [ ] All types have comprehensive unit tests
- [ ] Test coverage ≥95% for this module
- [ ] All types are Send + Sync (where applicable)
- [ ] Public types use `#[non_exhaustive]`
- [ ] No `unwrap()` in library code (only in tests)
- [ ] Documentation exists for all public items

### Estimated Effort

**Time:** 3-4 hours

**Breakdown:**
- Error types: 1 hour
- ID types: 45 min
- StepResult/WorkflowState: 1 hour
- Testing: 1.5 hours

---

## Stage 1.3: LLM Abstraction Layer

**Goal:** Wrap LLM interaction behind a trait for testability and provider flexibility.

**Dependencies:** Stage 1.2 (core types and error handling)

### Detailed Implementation Steps

#### Step 1: Define LLM Provider Trait

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/06-traits.md`
2. Load `assets/ai/ai-rust/guides/07-concurrency-async.md`
3. Review async best practices (no blocking in async)

**Create directory structure:**

```bash
mkdir -p crates/ecl-core/src/llm
```

**`crates/ecl-core/src/llm/provider.rs`**

```rust
//! LLM provider abstraction.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

/// Abstraction over LLM providers (Claude, GPT, etc.).
///
/// This trait allows swapping LLM backends without changing workflow code.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Completes a prompt and returns the full response.
    ///
    /// This is a blocking call that waits for the entire response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;

    /// Completes a prompt with streaming response.
    ///
    /// Returns a stream of response chunks as they arrive.
    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream>;
}

/// A request to complete a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// System prompt (context/instructions)
    pub system_prompt: Option<String>,

    /// Conversation messages
    pub messages: Vec<Message>,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Temperature (0.0 = deterministic, 1.0 = creative)
    pub temperature: Option<f32>,

    /// Stop sequences
    pub stop_sequences: Vec<String>,
}

impl CompletionRequest {
    /// Creates a new completion request with default settings.
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            system_prompt: None,
            messages,
            max_tokens: 1024,
            temperature: None,
            stop_sequences: Vec::new(),
        }
    }

    /// Sets the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }

    /// Sets the maximum tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Sets the temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Adds a stop sequence.
    pub fn with_stop_sequence(mut self, sequence: impl Into<String>) -> Self {
        self.stop_sequences.push(sequence.into());
        self
    }
}

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,

    /// Message content
    pub content: String,
}

impl Message {
    /// Creates a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// Creates an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Response from an LLM completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated content
    pub content: String,

    /// Token usage statistics
    pub tokens_used: TokenUsage,

    /// Why the model stopped generating
    pub stop_reason: StopReason,
}

/// Token usage statistics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input tokens consumed
    pub input: u64,

    /// Output tokens generated
    pub output: u64,
}

impl TokenUsage {
    /// Total tokens used (input + output).
    pub fn total(&self) -> u64 {
        self.input + self.output
    }
}

/// Reason why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum StopReason {
    /// Reached the end of the response naturally
    EndTurn,

    /// Hit the maximum token limit
    MaxTokens,

    /// Encountered a stop sequence
    StopSequence,
}

/// Streaming response from an LLM completion.
///
/// This is a placeholder for now; full implementation in Phase 3.
pub struct CompletionStream {
    // Future: implement streaming using tokio::sync::mpsc or similar
    _private: (),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_constructors() {
        let user_msg = Message::user("Hello");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.content, "Hello");

        let asst_msg = Message::assistant("Hi there");
        assert_eq!(asst_msg.role, Role::Assistant);
        assert_eq!(asst_msg.content, "Hi there");
    }

    #[test]
    fn test_completion_request_builder() {
        let request = CompletionRequest::new(vec![Message::user("Test")])
            .with_system_prompt("You are helpful")
            .with_max_tokens(2048)
            .with_temperature(0.7)
            .with_stop_sequence("\n\n");

        assert_eq!(
            request.system_prompt,
            Some("You are helpful".to_string())
        );
        assert_eq!(request.max_tokens, 2048);
        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.stop_sequences, vec!["\n\n"]);
    }

    #[test]
    fn test_token_usage_total() {
        let usage = TokenUsage {
            input: 100,
            output: 200,
        };
        assert_eq!(usage.total(), 300);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("test content");
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
```

#### Step 2: Implement Claude Provider

**`crates/ecl-core/src/llm/claude.rs`**

```rust
//! Claude API provider implementation.

use async_trait::async_trait;

use super::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, StopReason, TokenUsage,
};
use crate::{Error, Result};

/// LLM provider using Anthropic's Claude API.
pub struct ClaudeProvider {
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl ClaudeProvider {
    /// Creates a new Claude provider.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Anthropic API key
    /// * `model` - Model ID (e.g., "claude-sonnet-4-20250514")
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        // Build Claude API request
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": request.max_tokens,
            "messages": request.messages,
        });

        if let Some(system) = request.system_prompt {
            body["system"] = serde_json::json!(system);
        }

        if let Some(temp) = request.temperature {
            body["temperature"] = serde_json::json!(temp);
        }

        if !request.stop_sequences.is_empty() {
            body["stop_sequences"] = serde_json::json!(request.stop_sequences);
        }

        // Make API request
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| Error::llm_with_source("Failed to call Claude API", e))?;

        // Check for errors
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::llm(format!(
                "Claude API error {}: {}",
                status, error_text
            )));
        }

        // Parse response
        let response_body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| Error::llm_with_source("Failed to parse Claude response", e))?;

        // Extract content
        let content = response_body["content"][0]["text"]
            .as_str()
            .ok_or_else(|| Error::llm("Missing content in Claude response"))?
            .to_string();

        // Extract token usage
        let usage = response_body["usage"].as_object()
            .ok_or_else(|| Error::llm("Missing usage data in Claude response"))?;

        let input_tokens = usage["input_tokens"]
            .as_u64()
            .ok_or_else(|| Error::llm("Invalid input_tokens"))?;
        let output_tokens = usage["output_tokens"]
            .as_u64()
            .ok_or_else(|| Error::llm("Invalid output_tokens"))?;

        // Extract stop reason
        let stop_reason_str = response_body["stop_reason"]
            .as_str()
            .ok_or_else(|| Error::llm("Missing stop_reason"))?;

        let stop_reason = match stop_reason_str {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            other => return Err(Error::llm(format!("Unknown stop reason: {}", other))),
        };

        Ok(CompletionResponse {
            content,
            tokens_used: TokenUsage {
                input: input_tokens,
                output: output_tokens,
            },
            stop_reason,
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming implementation deferred to Phase 3
        Err(Error::llm("Streaming not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Message;

    #[test]
    fn test_claude_provider_construction() {
        let provider = ClaudeProvider::new("test-key", "claude-3-opus");
        assert_eq!(provider.api_key, "test-key");
        assert_eq!(provider.model, "claude-3-opus");
    }

    // Integration test (requires API key, run manually)
    #[tokio::test]
    #[ignore]
    async fn test_claude_provider_integration() {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .expect("ANTHROPIC_API_KEY must be set for integration tests");

        let provider = ClaudeProvider::new(api_key, "claude-sonnet-4-20250514");

        let request = CompletionRequest::new(vec![Message::user("Say hello")])
            .with_max_tokens(100);

        let response = provider.complete(request).await.unwrap();

        assert!(!response.content.is_empty());
        assert!(response.tokens_used.output > 0);
    }
}
```

#### Step 3: Implement Mock Provider

**`crates/ecl-core/src/llm/mock.rs`**

```rust
//! Mock LLM provider for testing.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::provider::{
    CompletionRequest, CompletionResponse, CompletionStream, LlmProvider, StopReason, TokenUsage,
};
use crate::Result;

/// Mock LLM provider that returns canned responses.
///
/// Useful for testing without making actual API calls.
#[derive(Clone)]
pub struct MockLlmProvider {
    responses: Arc<Mutex<MockResponses>>,
}

struct MockResponses {
    canned: Vec<String>,
    index: usize,
}

impl MockLlmProvider {
    /// Creates a new mock provider with canned responses.
    ///
    /// Responses are returned in order. After all responses are used,
    /// the provider cycles back to the first response.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_core::llm::MockLlmProvider;
    ///
    /// let provider = MockLlmProvider::new(vec![
    ///     "First response".to_string(),
    ///     "Second response".to_string(),
    /// ]);
    /// ```
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(MockResponses {
                canned: responses,
                index: 0,
            })),
        }
    }

    /// Creates a mock provider with a single response.
    pub fn with_response(response: impl Into<String>) -> Self {
        Self::new(vec![response.into()])
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse> {
        let mut responses = self.responses.lock().await;

        // Get current response
        let content = responses.canned[responses.index].clone();

        // Advance to next response (cycling)
        responses.index = (responses.index + 1) % responses.canned.len();

        Ok(CompletionResponse {
            content,
            tokens_used: TokenUsage {
                input: 10,  // Mock values
                output: 20,
            },
            stop_reason: StopReason::EndTurn,
        })
    }

    async fn complete_streaming(
        &self,
        _request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming not implemented for mock
        Err(crate::Error::llm("Streaming not supported in mock provider"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Message;

    #[tokio::test]
    async fn test_mock_provider_single_response() {
        let provider = MockLlmProvider::with_response("Test response");

        let request = CompletionRequest::new(vec![Message::user("Hello")]);

        let response = provider.complete(request).await.unwrap();
        assert_eq!(response.content, "Test response");
    }

    #[tokio::test]
    async fn test_mock_provider_multiple_responses() {
        let provider = MockLlmProvider::new(vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string(),
        ]);

        let request = CompletionRequest::new(vec![Message::user("Test")]);

        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "First"
        );
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "Second"
        );
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "Third"
        );
        // Cycles back
        assert_eq!(
            provider.complete(request.clone()).await.unwrap().content,
            "First"
        );
    }

    #[tokio::test]
    async fn test_mock_provider_clone() {
        let provider = MockLlmProvider::with_response("Shared");
        let provider2 = provider.clone();

        let request = CompletionRequest::new(vec![Message::user("Test")]);

        // Both providers share the same state
        provider.complete(request.clone()).await.unwrap();
        // Would cycle if shared, but we have only one response so it's the same
        let response = provider2.complete(request).await.unwrap();
        assert_eq!(response.content, "Shared");
    }
}
```

#### Step 4: Implement Retry Logic

**`crates/ecl-core/src/llm/retry.rs`**

```rust
//! Retry wrapper for LLM providers.

use async_trait::async_trait;
use backon::{ExponentialBuilder, Retryable};
use std::sync::Arc;
use std::time::Duration;

use super::provider::{CompletionRequest, CompletionResponse, CompletionStream, LlmProvider};
use crate::{Error, Result};

/// Wraps an LLM provider with retry logic.
pub struct RetryWrapper {
    inner: Arc<dyn LlmProvider>,
    max_attempts: u32,
    initial_delay: Duration,
    max_delay: Duration,
}

impl RetryWrapper {
    /// Creates a new retry wrapper with default settings.
    ///
    /// Default settings:
    /// - Max attempts: 3
    /// - Initial delay: 1 second
    /// - Max delay: 10 seconds
    /// - Multiplier: 2.0 (exponential backoff)
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self {
            inner: provider,
            max_attempts: 3,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
        }
    }

    /// Sets the maximum number of attempts.
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Sets the initial delay between retries.
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay between retries.
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Determines if an error should be retried.
    fn should_retry(error: &Error) -> bool {
        error.is_retryable()
    }
}

#[async_trait]
impl LlmProvider for RetryWrapper {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let backoff = ExponentialBuilder::default()
            .with_min_delay(self.initial_delay)
            .with_max_delay(self.max_delay)
            .with_max_times(self.max_attempts as usize);

        let provider = self.inner.clone();
        let request_clone = request.clone();

        // Use backon for retry logic
        (|| async {
            provider.complete(request_clone.clone()).await
        })
        .retry(backoff)
        .when(Self::should_retry)
        .await
    }

    async fn complete_streaming(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionStream> {
        // Streaming with retry is complex, defer to Phase 3
        self.inner.complete_streaming(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_retry_wrapper_success() {
        let mock = Arc::new(MockLlmProvider::with_response("Success"));
        let retry = RetryWrapper::new(mock);

        let request = CompletionRequest::new(vec![crate::llm::Message::user("Test")]);
        let response = retry.complete(request).await.unwrap();

        assert_eq!(response.content, "Success");
    }

    #[test]
    fn test_retry_wrapper_builder() {
        let mock = Arc::new(MockLlmProvider::with_response("Test"));
        let retry = RetryWrapper::new(mock)
            .with_max_attempts(5)
            .with_initial_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(30));

        assert_eq!(retry.max_attempts, 5);
        assert_eq!(retry.initial_delay, Duration::from_millis(500));
        assert_eq!(retry.max_delay, Duration::from_secs(30));
    }
}
```

#### Step 5: Module Organization

**`crates/ecl-core/src/llm/mod.rs`**

```rust
//! LLM provider abstractions and implementations.

mod claude;
mod mock;
mod provider;
mod retry;

pub use claude::ClaudeProvider;
pub use mock::MockLlmProvider;
pub use provider::{
    CompletionRequest, CompletionResponse, LlmProvider, Message, Role, StopReason, TokenUsage,
};
pub use retry::RetryWrapper;
```

**Update `crates/ecl-core/src/lib.rs`**

```rust
pub mod llm;

// Add to re-exports
pub use llm::{CompletionRequest, CompletionResponse, LlmProvider, Message};
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Provider trait tests**:
   - Message construction
   - Request builder pattern
   - Token usage calculation
   - Serialization roundtrips

2. **Mock provider tests**:
   - Single response
   - Multiple responses with cycling
   - Clone behavior (shared state)

3. **Claude provider tests**:
   - Construction
   - Integration test (manual, with real API key)

4. **Retry wrapper tests**:
   - Success case (no retry needed)
   - Retry on transient failure
   - Max attempts exhausted
   - Non-retryable errors fail fast

**Integration test setup:**

```bash
# In justfile
test-integration:
    ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY} cargo test --workspace -- --ignored
```

### Potential Blockers

1. **`llm` crate doesn't support Claude or has different API**
   - **Mitigation**: Verified via crates.io research
   - **Fallback**: Use `reqwest` directly (as shown in ClaudeProvider)

2. **Retry logic complexity**
   - **Mitigation**: Use `backon` crate as planned
   - **Alternative**: Hand-roll simple exponential backoff

3. **Async trait object safety**
   - **Mitigation**: Use `async-trait` crate
   - **Note**: Requires boxing futures, minor perf cost

### Acceptance Criteria

- [ ] `LlmProvider` trait is object-safe with async-trait
- [ ] `ClaudeProvider` makes successful API calls (integration test)
- [ ] `MockLlmProvider` returns canned responses in tests
- [ ] Retry logic retries on transient errors
- [ ] Retry logic fails fast on permanent errors
- [ ] All types implement required Restate serialization traits
- [ ] Test coverage ≥95%
- [ ] No blocking I/O in async functions
- [ ] All public APIs documented

### Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- Provider trait and types: 1 hour
- Claude implementation: 1.5 hours
- Mock implementation: 30 min
- Retry logic: 1 hour
- Testing: 1.5 hours

---

## Stage 1.4: Basic Restate Workflow

**Goal:** Create minimal 2-step workflow demonstrating Restate integration.

**Dependencies:** Stage 1.3 (LLM abstraction layer)

### Detailed Implementation Steps

#### Step 1: Create Workflow Input/Output Types

**`crates/ecl-workflows/src/simple.rs`**

```rust
//! Simple 2-step workflow for Phase 1 validation.

use serde::{Deserialize, Serialize};
use ecl_core::{WorkflowId, Result};

/// Input for the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowInput {
    /// Unique ID for this workflow instance
    pub workflow_id: WorkflowId,

    /// Topic to generate content about
    pub topic: String,
}

impl SimpleWorkflowInput {
    /// Creates a new workflow input.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            workflow_id: WorkflowId::new(),
            topic: topic.into(),
        }
    }
}

/// Output from the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowOutput {
    /// Workflow ID that produced this output
    pub workflow_id: WorkflowId,

    /// Generated text from step 1
    pub generated_text: String,

    /// Critique of generated text from step 2
    pub critique: String,
}
```

#### Step 2: Implement Restate Service

**`crates/ecl-workflows/src/simple.rs`** (continued)

```rust
use restate_sdk::prelude::*;
use std::sync::Arc;
use ecl_core::llm::{LlmProvider, Message, CompletionRequest};

/// Simple workflow service demonstrating Restate integration.
pub struct SimpleWorkflowService {
    llm: Arc<dyn LlmProvider>,
}

impl SimpleWorkflowService {
    /// Creates a new simple workflow service.
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the simple 2-step workflow.
    pub async fn run_simple(
        &self,
        ctx: Context<'_>,
        input: SimpleWorkflowInput,
    ) -> Result<SimpleWorkflowOutput> {
        // Step 1: Generate content
        let generated_text = ctx
            .run("generate", || async {
                self.generate_step(&input.topic).await
            })
            .await?;

        // Step 2: Critique the generated content
        let critique = ctx
            .run("critique", || async {
                self.critique_step(&generated_text).await
            })
            .await?;

        Ok(SimpleWorkflowOutput {
            workflow_id: input.workflow_id,
            generated_text,
            critique,
        })
    }

    /// Step 1: Generate content on a topic.
    async fn generate_step(&self, topic: &str) -> Result<String> {
        tracing::info!(topic = %topic, "Generating content");

        let request = CompletionRequest::new(vec![
            Message::user(format!("Write a short paragraph about: {}", topic))
        ])
        .with_system_prompt("You are a helpful content generator. Write clear, concise paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;

        tracing::info!(
            tokens = response.tokens_used.total(),
            "Content generated"
        );

        Ok(response.content)
    }

    /// Step 2: Critique generated content.
    async fn critique_step(&self, content: &str) -> Result<String> {
        tracing::info!("Critiquing generated content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Please provide constructive criticism of the following text:\n\n{}",
                content
            ))
        ])
        .with_system_prompt("You are a helpful writing critic. Provide specific, actionable feedback.")
        .with_max_tokens(300);

        let response = self.llm.complete(request).await?;

        tracing::info!(
            tokens = response.tokens_used.total(),
            "Critique completed"
        );

        Ok(response.content)
    }
}
```

#### Step 3: Register Service with Restate

**`crates/ecl-workflows/src/main.rs`**

```rust
//! ECL Workflows service entry point.

use std::sync::Arc;
use ecl_core::llm::{ClaudeProvider, RetryWrapper};
use restate_sdk::prelude::*;

mod simple;
use simple::SimpleWorkflowService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,ecl=debug".into())
        )
        .init();

    tracing::info!("Starting ECL Workflows service");

    // Load configuration
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");
    let model = std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    // Create LLM provider with retry wrapper
    let claude = ClaudeProvider::new(api_key, model);
    let llm = Arc::new(RetryWrapper::new(Arc::new(claude)));

    // Create workflow service
    let service = SimpleWorkflowService::new(llm);

    // Build Restate endpoint
    let endpoint = Endpoint::builder()
        .bind(service)
        .build();

    // Start HTTP server
    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:9080".to_string());

    tracing::info!(address = %bind_address, "Starting Restate endpoint");

    HttpServer::new(endpoint)
        .listen_and_serve(bind_address.parse()?, tokio::signal::ctrl_c())
        .await?;

    tracing::info!("ECL Workflows service stopped");
    Ok(())
}
```

#### Step 4: Create Invocation Script

**`scripts/run-simple-workflow.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

RESTATE_URL="${RESTATE_URL:-http://localhost:8080}"
WORKFLOW_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
TOPIC="${1:-Benefits of Rust programming}"

echo "=== ECL Simple Workflow Test ==="
echo "Workflow ID: $WORKFLOW_ID"
echo "Topic: $TOPIC"
echo

# Invoke workflow
curl -X POST \
  "$RESTATE_URL/SimpleWorkflowService/run_simple" \
  -H "Content-Type: application/json" \
  -d "{
    \"workflow_id\": \"$WORKFLOW_ID\",
    \"topic\": \"$TOPIC\"
  }" | jq '.'
```

#### Step 5: Update Docker Compose

Add to `docker-compose.yml`:

```yaml
  ecl-workflows:
    build:
      context: .
      dockerfile: Dockerfile
    depends_on:
      - restate
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RUST_LOG=info,ecl=debug
    ports:
      - "9080:9080"
```

**`Dockerfile`**

```dockerfile
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin ecl-workflows

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ecl-workflows /usr/local/bin/
EXPOSE 9080
CMD ["ecl-workflows"]
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Workflow execution**:
   - Happy path (both steps complete)
   - Workflow survives restart mid-execution
   - Tracing logs capture execution

2. **Input/output validation**:
   - Valid inputs accepted
   - Invalid inputs rejected
   - Output serialization works

3. **Restate integration**:
   - Service registration works
   - Durable execution survives restarts
   - State persists correctly

### Potential Blockers

1. **Restate SDK API changes**
   - **Mitigation**: Carefully review Restate SDK 0.7 documentation
   - **Mitigation**: Test service registration early

2. **Docker networking issues**
   - **Mitigation**: Use docker-compose networking
   - **Mitigation**: Document port mappings clearly

3. **Serialization with Restate**
   - **Mitigation**: Ensure all types are Serialize + Deserialize
   - **Mitigation**: Test serialization early

### Acceptance Criteria

- [ ] Workflow completes successfully end-to-end
- [ ] Killing process mid-workflow and restarting resumes from last step
- [ ] Workflow output contains both generated text and critique
- [ ] Tracing logs show step execution with structured fields
- [ ] Docker Compose brings up full stack
- [ ] Service registers with Restate successfully
- [ ] Test coverage ≥90% (integration tests may be lower)

### Estimated Effort

**Time:** 5-6 hours

**Breakdown:**
- Restate service setup: 2 hours
- Workflow implementation: 2 hours
- Docker setup: 1 hour
- Testing and debugging: 2 hours

---

## Stage 1.5: Feedback Loop with Durable Promises

**Goal:** Extend workflow to demonstrate bounded revision cycles using Durable Promises.

**Dependencies:** Stage 1.4 (basic Restate workflow)

### Detailed Implementation Steps

#### Step 1: Define Critique Decision Type

**`crates/ecl-core/src/types/critique.rs`**

```rust
//! Types for critique and revision decisions.

use serde::{Deserialize, Serialize};

/// Decision from a critique step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CritiqueDecision {
    /// Content passes critique, no revision needed
    Pass,

    /// Content needs revision with specific feedback
    Revise {
        /// Specific feedback on what to improve
        feedback: String,
    },
}

impl CritiqueDecision {
    /// Returns `true` if the decision is Pass.
    pub fn is_pass(&self) -> bool {
        matches!(self, CritiqueDecision::Pass)
    }

    /// Returns `true` if the decision is Revise.
    pub fn needs_revision(&self) -> bool {
        matches!(self, CritiqueDecision::Revise { .. })
    }

    /// Extracts the feedback if this is a Revise decision.
    pub fn feedback(&self) -> Option<&str> {
        match self {
            CritiqueDecision::Revise { feedback } => Some(feedback),
            CritiqueDecision::Pass => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pass_decision() {
        let decision = CritiqueDecision::Pass;
        assert!(decision.is_pass());
        assert!(!decision.needs_revision());
        assert_eq!(decision.feedback(), None);
    }

    #[test]
    fn test_revise_decision() {
        let decision = CritiqueDecision::Revise {
            feedback: "Needs more detail".to_string(),
        };
        assert!(!decision.is_pass());
        assert!(decision.needs_revision());
        assert_eq!(decision.feedback(), Some("Needs more detail"));
    }

    #[test]
    fn test_critique_decision_serialization() {
        let decision = CritiqueDecision::Revise {
            feedback: "Test feedback".to_string(),
        };
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: CritiqueDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(decision, deserialized);
    }
}
```

Update `crates/ecl-core/src/types/mod.rs`:

```rust
mod critique;
pub use critique::CritiqueDecision;
```

#### Step 2: Implement Critique-Revise Workflow

**`crates/ecl-workflows/src/critique_loop.rs`**

```rust
//! Critique-Revise workflow with bounded feedback loop.

use serde::{Deserialize, Serialize};
use restate_sdk::prelude::*;
use std::sync::Arc;

use ecl_core::{
    WorkflowId, Result, Error,
    llm::{LlmProvider, Message, CompletionRequest},
    types::CritiqueDecision,
};

/// Maximum number of revision attempts before giving up.
const MAX_REVISIONS: u32 = 3;

/// Input for critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopInput {
    pub workflow_id: WorkflowId,
    pub topic: String,
}

/// Output from critique-revise workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueLoopOutput {
    pub workflow_id: WorkflowId,
    pub final_text: String,
    pub revision_count: u32,
    pub critiques: Vec<String>,
}

/// Workflow with critique and revision loop.
pub struct CritiqueLoopWorkflow {
    llm: Arc<dyn LlmProvider>,
}

impl CritiqueLoopWorkflow {
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the critique-revise workflow with bounded iteration.
    pub async fn run(
        &self,
        ctx: Context<'_>,
        input: CritiqueLoopInput,
    ) -> Result<CritiqueLoopOutput> {
        // Step 1: Generate initial draft
        let mut current_draft = ctx
            .run("generate", || async {
                self.generate_step(&input.topic).await
            })
            .await?;

        let mut revision_count = 0u32;
        let mut critiques = Vec::new();

        // Revision loop with bounded iterations
        loop {
            // Step 2: Critique current draft
            let (critique_text, decision) = ctx
                .run(&format!("critique_{}", revision_count), || async {
                    self.critique_step(&current_draft, revision_count).await
                })
                .await?;

            critiques.push(critique_text.clone());

            match decision {
                CritiqueDecision::Pass => {
                    tracing::info!(
                        revision_count,
                        "Critique passed, workflow complete"
                    );
                    break;
                }
                CritiqueDecision::Revise { feedback } => {
                    if revision_count >= MAX_REVISIONS {
                        return Err(Error::MaxRevisionsExceeded {
                            attempts: MAX_REVISIONS,
                        });
                    }

                    tracing::info!(
                        revision_count,
                        feedback = %feedback,
                        "Revision requested"
                    );

                    // Step 3: Revise based on feedback
                    current_draft = ctx
                        .run(&format!("revise_{}", revision_count), || async {
                            self.revise_step(&current_draft, &feedback, revision_count).await
                        })
                        .await?;

                    revision_count += 1;
                }
            }
        }

        Ok(CritiqueLoopOutput {
            workflow_id: input.workflow_id,
            final_text: current_draft,
            revision_count,
            critiques,
        })
    }

    /// Generate initial content.
    async fn generate_step(&self, topic: &str) -> Result<String> {
        tracing::info!(topic = %topic, "Generating initial content");

        let request = CompletionRequest::new(vec![
            Message::user(format!("Write a paragraph about: {}", topic))
        ])
        .with_system_prompt("You are a content generator. Write clear paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;
        Ok(response.content)
    }

    /// Critique content and decide if revision is needed.
    async fn critique_step(
        &self,
        content: &str,
        attempt: u32,
    ) -> Result<(String, CritiqueDecision)> {
        tracing::info!(attempt, "Critiquing content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Critique this text and decide if it needs revision.\n\
                Respond with JSON: {{\"decision\": \"pass\" or \"revise\", \"critique\": \"your critique\", \"feedback\": \"what to improve\"}}\n\n\
                Text:\n{}",
                content
            ))
        ])
        .with_system_prompt("You are a writing critic. Be helpful but thorough.")
        .with_max_tokens(400);

        let response = self.llm.complete(request).await?;

        // Parse JSON response
        let parsed: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|e| Error::validation(format!("Failed to parse critique JSON: {}", e)))?;

        let critique = parsed["critique"]
            .as_str()
            .ok_or_else(|| Error::validation("Missing critique field"))?
            .to_string();

        let decision = match parsed["decision"].as_str() {
            Some("pass") => CritiqueDecision::Pass,
            Some("revise") => {
                let feedback = parsed["feedback"]
                    .as_str()
                    .ok_or_else(|| Error::validation("Missing feedback for revise decision"))?
                    .to_string();
                CritiqueDecision::Revise { feedback }
            }
            _ => return Err(Error::validation("Invalid decision value")),
        };

        Ok((critique, decision))
    }

    /// Revise content based on feedback.
    async fn revise_step(
        &self,
        original: &str,
        feedback: &str,
        attempt: u32,
    ) -> Result<String> {
        tracing::info!(attempt, "Revising content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Revise this text based on the feedback:\n\n\
                Original:\n{}\n\n\
                Feedback:\n{}",
                original, feedback
            ))
        ])
        .with_system_prompt("You are an editor. Improve the text based on feedback.")
        .with_max_tokens(600);

        let response = self.llm.complete(request).await?;
        Ok(response.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critique_loop_input() {
        let input = CritiqueLoopInput {
            workflow_id: WorkflowId::new(),
            topic: "Test".to_string(),
        };
        assert_eq!(input.topic, "Test");
    }

    #[test]
    fn test_max_revisions_constant() {
        assert_eq!(MAX_REVISIONS, 3);
    }
}
```

#### Step 3: Add Workflow State Persistence

**`crates/ecl-workflows/src/critique_loop.rs`** (add to workflow impl)

```rust
/// State stored in Restate K/V for workflow persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowState {
    current_draft: String,
    revision_count: u32,
    critiques: Vec<String>,
}

impl CritiqueLoopWorkflow {
    /// Store workflow state in Restate K/V.
    async fn save_state(&self, ctx: &Context<'_>, state: &WorkflowState) -> Result<()> {
        ctx.set("workflow_state", state).await;
        Ok(())
    }

    /// Load workflow state from Restate K/V.
    async fn load_state(&self, ctx: &Context<'_>) -> Result<Option<WorkflowState>> {
        Ok(ctx.get::<WorkflowState>("workflow_state").await?)
    }
}
```

#### Step 4: Integration with Main

Update `crates/ecl-workflows/src/main.rs`:

```rust
mod critique_loop;
use critique_loop::CritiqueLoopWorkflow;

// In main():
let critique_workflow = CritiqueLoopWorkflow::new(llm.clone());

let endpoint = Endpoint::builder()
    .bind(service)
    .bind(critique_workflow)
    .build();
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Revision loop logic**:
   - Passes on first critique (no revisions)
   - Revises once and passes
   - Revises multiple times and passes
   - Hits MAX_REVISIONS and fails appropriately

2. **State persistence**:
   - State saves correctly
   - State loads after restart
   - Revision count persists

3. **Decision parsing**:
   - Valid pass decision
   - Valid revise decision with feedback
   - Invalid JSON rejected
   - Missing fields rejected

**Integration tests:**

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    use ecl_core::llm::MockLlmProvider;

    #[tokio::test]
    async fn test_workflow_passes_immediately() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Initial draft".to_string(),
            r#"{"decision": "pass", "critique": "Looks good!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Test execution...
    }

    #[tokio::test]
    async fn test_workflow_revision_loop() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Draft 1".to_string(),
            r#"{"decision": "revise", "critique": "Needs work", "feedback": "Add detail"}"#.to_string(),
            "Draft 2".to_string(),
            r#"{"decision": "pass", "critique": "Better!"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Test execution...
    }

    #[tokio::test]
    async fn test_max_revisions_exceeded() {
        let mock = Arc::new(MockLlmProvider::new(vec![
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Bad", "feedback": "Fix it"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Still bad", "feedback": "Fix more"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Nope", "feedback": "Again"}"#.to_string(),
            "Draft".to_string(),
            r#"{"decision": "revise", "critique": "Nope", "feedback": "Last try"}"#.to_string(),
        ]));

        let workflow = CritiqueLoopWorkflow::new(mock);
        // Should error with MaxRevisionsExceeded
    }
}
```

### Potential Blockers

1. **Restate state persistence API**
   - **Mitigation**: Review Restate K/V documentation
   - **Fallback**: Store in workflow-local memory for Phase 1

2. **JSON parsing from LLM**
   - **Mitigation**: Use structured prompts
   - **Mitigation**: Validate and provide helpful errors
   - **Fallback**: Use simpler text-based decisions

3. **Loop termination edge cases**
   - **Mitigation**: Comprehensive testing
   - **Mitigation**: Always check revision_count bounds

### Acceptance Criteria

- [ ] Workflow correctly loops on Revise decisions
- [ ] Loop terminates at MAX_REVISIONS with appropriate error
- [ ] Loop terminates on Pass decision
- [ ] Workflow survives restart mid-revision-loop
- [ ] State (revision_count, current_draft) persists across restarts
- [ ] All decision types handled correctly
- [ ] JSON parsing errors are graceful
- [ ] Test coverage ≥95%
- [ ] Tracing logs show loop iterations

### Estimated Effort

**Time:** 4-5 hours

**Breakdown:**
- Critique decision types: 1 hour
- Workflow implementation: 2 hours
- State persistence: 1 hour
- Testing: 1.5 hours

---

## Stage 1.6: Phase 1 Integration Test Suite

**Goal:** Comprehensive tests proving Phase 1 deliverables work correctly.

**Dependencies:** Stages 1.1-1.5 (all prior stages)

### Detailed Implementation Steps

#### Step 1: Create Test Harness

**`tests/common/mod.rs`**

```rust
//! Common test utilities and harness.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use ecl_core::llm::MockLlmProvider;

/// Test harness for integration tests.
pub struct TestHarness {
    pub restate_url: String,
    pub llm: Arc<MockLlmProvider>,
}

impl TestHarness {
    /// Creates a new test harness.
    pub fn new() -> Self {
        let restate_url = std::env::var("RESTATE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let llm = Arc::new(MockLlmProvider::with_response("Test response"));

        Self { restate_url, llm }
    }

    /// Waits for Restate to be ready.
    pub async fn wait_for_restate(&self) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/restate/health", self.restate_url);

        for _ in 0..30 {
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_secs(1)).await;
        }

        anyhow::bail!("Restate did not become healthy in time")
    }

    /// Invokes a workflow and waits for completion.
    pub async fn invoke_workflow<I, O>(
        &self,
        service: &str,
        method: &str,
        input: I,
    ) -> anyhow::Result<O>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let client = reqwest::Client::new();
        let url = format!("{}/{}/{}", self.restate_url, service, method);

        let response = client
            .post(&url)
            .json(&input)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Workflow invocation failed: {}", error_text);
        }

        Ok(response.json().await?)
    }
}

/// Helper to parse test data from files.
pub fn load_test_data(filename: &str) -> String {
    std::fs::read_to_string(format!("tests/data/{}", filename))
        .expect("Failed to load test data")
}
```

#### Step 2: Unit Tests

**`tests/unit/mod.rs`**

```rust
//! Unit tests for ECL core types.

mod types;
mod llm_mock;
mod error_handling;
```

**`tests/unit/types.rs`**

```rust
use ecl_core::{WorkflowId, StepId, StepResult, WorkflowState};
use proptest::prelude::*;

#[test]
fn test_workflow_id_uniqueness() {
    let id1 = WorkflowId::new();
    let id2 = WorkflowId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_step_result_map() {
    let result = StepResult::Success(10);
    let mapped = result.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_workflow_state_terminal() {
    assert!(WorkflowState::Completed.is_terminal());
    assert!(WorkflowState::Failed.is_terminal());
    assert!(!WorkflowState::Running.is_terminal());
}

// Property-based tests
proptest! {
    #[test]
    fn test_workflow_id_serialization_roundtrip(uuid in any::<u128>()) {
        use uuid::Uuid;
        let uuid = Uuid::from_u128(uuid);
        let id = WorkflowId::from_uuid(uuid);
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_step_id_roundtrip(s in "\\PC+") {
        let id = StepId::new(s.clone());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: StepId = serde_json::from_str(&json).unwrap();
        assert_eq!(id.as_str(), deserialized.as_str());
    }
}
```

**`tests/unit/llm_mock.rs`**

```rust
use ecl_core::llm::{MockLlmProvider, Message, CompletionRequest, LlmProvider};

#[tokio::test]
async fn test_mock_provider_responses() {
    let provider = MockLlmProvider::new(vec![
        "First".to_string(),
        "Second".to_string(),
    ]);

    let request = CompletionRequest::new(vec![Message::user("Test")]);

    let response1 = provider.complete(request.clone()).await.unwrap();
    assert_eq!(response1.content, "First");

    let response2 = provider.complete(request.clone()).await.unwrap();
    assert_eq!(response2.content, "Second");

    // Cycles
    let response3 = provider.complete(request).await.unwrap();
    assert_eq!(response3.content, "First");
}
```

**`tests/unit/error_handling.rs`**

```rust
use ecl_core::Error;

#[test]
fn test_error_retryability() {
    assert!(Error::llm("test").is_retryable());
    assert!(Error::Timeout { seconds: 10 }.is_retryable());
    assert!(!Error::validation("test").is_retryable());
    assert!(!Error::MaxRevisionsExceeded { attempts: 3 }.is_retryable());
}

#[test]
fn test_error_display() {
    let err = Error::MaxRevisionsExceeded { attempts: 5 };
    assert_eq!(err.to_string(), "Maximum revisions exceeded: 5 attempts");
}

#[test]
fn test_validation_error_with_field() {
    let err = Error::validation_field("email", "invalid format");
    match err {
        Error::Validation { field, message } => {
            assert_eq!(field, Some("email".to_string()));
            assert_eq!(message, "invalid format");
        }
        _ => panic!("Expected Validation error"),
    }
}
```

#### Step 3: Integration Tests

**`tests/integration/mod.rs`**

```rust
mod simple_workflow;
mod revision_loop;
mod llm_retry;
```

**`tests/integration/simple_workflow.rs`**

```rust
use ecl_workflows::{SimpleWorkflowInput, SimpleWorkflowOutput};

#[tokio::test]
#[ignore] // Requires Restate server
async fn test_simple_workflow_completes() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    let input = SimpleWorkflowInput::new("Rust programming");

    let output: SimpleWorkflowOutput = harness
        .invoke_workflow("SimpleWorkflowService", "run_simple", input)
        .await
        .unwrap();

    assert!(!output.generated_text.is_empty());
    assert!(!output.critique.is_empty());
}
```

**`tests/integration/revision_loop.rs`**

```rust
use ecl_workflows::{CritiqueLoopInput, CritiqueLoopOutput};
use ecl_core::WorkflowId;

#[tokio::test]
#[ignore] // Requires Restate server
async fn test_revision_loop_terminates() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    let input = CritiqueLoopInput {
        workflow_id: WorkflowId::new(),
        topic: "Test topic".to_string(),
    };

    let output: CritiqueLoopOutput = harness
        .invoke_workflow("CritiqueLoopWorkflow", "run", input)
        .await
        .unwrap();

    assert!(!output.final_text.is_empty());
    assert!(output.revision_count <= 3);
}

#[tokio::test]
#[ignore]
async fn test_max_revisions_exceeded() {
    // Test that workflow fails gracefully after max revisions
    // Use mock that always returns "revise" decision
    todo!("Implement test with mock LLM")
}
```

**`tests/integration/llm_retry.rs`**

```rust
use ecl_core::llm::{RetryWrapper, MockLlmProvider, LlmProvider, Message, CompletionRequest};
use std::sync::Arc;

#[tokio::test]
async fn test_retry_on_transient_failure() {
    // Mock that fails twice then succeeds
    // Note: This requires extending MockLlmProvider to support failure injection
    todo!("Implement retry test with failure injection")
}
```

#### Step 4: Chaos Tests

**`tests/chaos/recovery.rs`**

```rust
//! Tests for workflow recovery after process restart.

use std::process::{Command, Child};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to start ECL workflows service.
fn start_service() -> Child {
    Command::new("cargo")
        .args(&["run", "--bin", "ecl-workflows"])
        .spawn()
        .expect("Failed to start service")
}

#[tokio::test]
#[ignore] // Manual test, requires real Restate
async fn test_workflow_survives_restart() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    // Start service
    let mut service = start_service();
    sleep(Duration::from_secs(2)).await;

    // Start workflow
    let input = ecl_workflows::SimpleWorkflowInput::new("Test");
    // Don't await - let it run in background
    let _handle = tokio::spawn(async move {
        harness
            .invoke_workflow("SimpleWorkflowService", "run_simple", input)
            .await
    });

    // Wait for first step to complete
    sleep(Duration::from_secs(5)).await;

    // Kill service
    service.kill().unwrap();
    service.wait().unwrap();

    // Restart service
    let mut service = start_service();
    sleep(Duration::from_secs(2)).await;

    // Workflow should resume and complete
    // (Check via Restate API or workflow output)

    service.kill().unwrap();
}
```

#### Step 5: CI Configuration

**`.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run unit tests
        run: cargo test --workspace --lib

      - name: Run doc tests
        run: cargo test --workspace --doc

  lint:
    name: Linting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Run rustfmt
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace --all-features --all-targets -- -D warnings

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: true
```

#### Step 6: Coverage Report Generation

**`justfile`** (add):

```just
# Generate coverage report and check threshold
coverage-check:
    cargo llvm-cov --workspace --all-features --html
    @echo "Checking coverage threshold..."
    @COVERAGE=$(cargo llvm-cov --workspace --all-features --summary-only | grep -oP '\d+\.\d+(?=%)' | head -1); \
    if (( $(echo "$COVERAGE < 95" | bc -l) )); then \
        echo "Coverage $COVERAGE% is below 95% threshold"; \
        exit 1; \
    else \
        echo "Coverage $COVERAGE% meets 95% threshold ✓"; \
    fi
```

### Testing Strategy

**Test organization:**

```
tests/
├── common/          # Shared test utilities
│   └── mod.rs       # TestHarness, helpers
├── unit/            # Fast unit tests (no I/O)
│   ├── mod.rs
│   ├── types.rs
│   ├── llm_mock.rs
│   └── error_handling.rs
├── integration/     # Integration tests (requires services)
│   ├── mod.rs
│   ├── simple_workflow.rs
│   ├── revision_loop.rs
│   └── llm_retry.rs
└── chaos/           # Chaos/recovery tests
    └── recovery.rs
```

**Running tests:**

```bash
# Unit tests only (fast, no dependencies)
cargo test --workspace --lib

# All tests except ignored
cargo test --workspace

# Integration tests (requires Restate)
docker compose up -d
cargo test --workspace -- --ignored
docker compose down

# Coverage
just coverage-check
```

### Potential Blockers

1. **Integration tests are flaky**
   - **Mitigation**: Proper wait conditions
   - **Mitigation**: Retry logic in tests
   - **Mitigation**: Clear test isolation

2. **Coverage reporting issues**
   - **Mitigation**: Use cargo-llvm-cov as specified
   - **Mitigation**: Exclude generated code if needed

3. **CI timeout issues**
   - **Mitigation**: Run integration tests separately
   - **Mitigation**: Cache dependencies properly

### Acceptance Criteria

- [ ] All unit tests pass
- [ ] Integration tests pass locally with Restate server
- [ ] CI pipeline passes (unit tests + linting)
- [ ] Overall test coverage ≥95%
- [ ] No ignored tests except integration/chaos tests
- [ ] Coverage report generates successfully
- [ ] All error paths are tested
- [ ] Property-based tests exist for core types
- [ ] Chaos tests demonstrate recovery (manual)
- [ ] Test documentation is clear

### Estimated Effort

**Time:** 6-8 hours

**Breakdown:**
- Test harness: 1.5 hours
- Unit tests: 2 hours
- Integration tests: 2 hours
- Chaos tests: 1 hour
- CI configuration: 1 hour
- Coverage verification: 0.5 hours

---

## Cross-Stage Dependencies

```
Stage 1.1 (Scaffolding)
    ↓
Stage 1.2 (Core Types) ← Must have build system
    ↓
Stage 1.3 (LLM Layer) ← Needs Error and Result types
    ↓
Stage 1.4 (Basic Workflow) ← Needs LLM abstraction
    ↓
Stage 1.5 (Feedback Loop) ← Needs basic workflow
    ↓
Stage 1.6 (Integration Tests) ← Needs everything
```

## Summary Checklist

Before moving to Phase 2, verify:

**Code Quality:**
- [ ] All code follows Rust anti-patterns guide (11-anti-patterns.md)
- [ ] No `unwrap()` in library code
- [ ] All errors use `thiserror`
- [ ] Parameters use `&str` not `&String`, `&[T]` not `&Vec<T>`
- [ ] Public types use `#[non_exhaustive]`
- [ ] All async code uses async I/O (no blocking)

**Testing:**
- [ ] Overall test coverage ≥95%
- [ ] All unit tests pass
- [ ] Integration tests pass (locally with Restate)
- [ ] Property tests added for core types
- [ ] No ignored tests (except manual integration tests)

**Documentation:**
- [ ] All public items have rustdoc comments
- [ ] README is up to date
- [ ] Examples compile and run

**Build System:**
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `just test` passes
- [ ] `just coverage` shows ≥95%

**Infrastructure:**
- [ ] Docker Compose works
- [ ] Restate server starts and registers services
- [ ] Database migrations run successfully
- [ ] Environment variables documented

---

## Appendix: Key Rust Patterns to Apply

### From Anti-Patterns Guide (11-anti-patterns.md)

- **AP-02**: Use `&str` not `&String`, `&[T]` not `&Vec<T>` in parameters
- **AP-03**: Use `Box<dyn Error + Send + Sync>` for thread-safe errors
- **AP-04**: Always annotate `.collect()` with type
- **AP-06**: Keep fields private when invariants exist
- **AP-08**: Avoid `unwrap()` in library code

### From Core Idioms (01-core-idioms.md)

- **ID-01**: Use `#[non_exhaustive]` on public enums/structs
- **ID-02**: Use `mem::take` and `mem::replace` for enum transformations
- **ID-04**: Follow `as_/to_/into_` conventions

### From Error Handling (03-error-handling.md)

- **EH-02**: Use `?` operator for error propagation
- **EH-03**: Use `thiserror` for library errors
- **EH-05**: Provide Result type alias

### From Concurrency & Async (07-concurrency-async.md)

- **CA-03**: Never use blocking I/O in async functions
- **CA-01**: Ensure proper Send/Sync bounds

---

**Document End**

**Version:** 1.0
**Last Updated:** 2026-01-22
**Ready for Implementation:** ✓
