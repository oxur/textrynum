---
title: "CC Prompt: Fabryk 1.0 & 1.1 — Cleanup & Workspace Scaffold"
milestone: "1.0, 1.1"
phase: 1
author: "Claude (Opus 4.5)"
created: 2026-02-03
updated: 2026-02-03
prerequisites: None (first milestones)
governing-docs: [0011-audit, 0012-amendment, 0013-project-plan]
---

# CC Prompt: Fabryk 1.0 & 1.1 — Cleanup & Workspace Scaffold

## Context

This document covers the first two milestones of the Fabryk extraction — a project
to refactor domain-agnostic infrastructure out of the music-theory MCP server into
a set of reusable Rust crates. See Doc 0013 (Project Plan Overview) for the full
phase breakdown and Doc 0011 (Extraction Audit) §6.1 for the workspace layout.

**Phase 1** creates `fabryk-core`, the foundation crate with zero internal Fabryk
dependencies. Before we can scaffold the new crates, we must first clean up the
existing ECL workspace which contains legacy Fabryk stubs from an earlier
architectural iteration.

**Important**: The Fabryk crates live within the ECL workspace at
`~/lab/oxur/ecl/crates/fabryk-*`. This is the chosen approach per the Unified
Ecosystem Vision v2 document.

**Music-Theory Migration**: These milestones extract code to Fabryk only.
Music-theory continues using its local copy of all modules until the v0.1-alpha
checkpoint (after Phase 3 completion), when all imports will be updated in a
single coordinated migration. This checkpoint-based approach avoids the
complexity of incremental dual-maintenance.

## Current ECL Workspace State

The ECL repo (`~/lab/oxur/ecl`) contains Fabryk crate stubs from an earlier
architectural iteration:

| Crate | Action | Reason |
|-------|--------|--------|
| `fabryk` | **Keep & gut** | Umbrella re-export crate |
| `fabryk-core` | **Keep & gut** | Has old content (`identity.rs`, `item.rs`, etc.) that doesn't match extraction targets |
| `fabryk-acl` | **Keep & gut** | Has premature implementation; will be placeholder for v0.2/v0.3 |
| `fabryk-mcp` | **Keep & gut** | Has different architecture; needs complete rewrite |
| `fabryk-cli` | **Keep & gut** | Has different architecture; needs complete rewrite |
| `fabryk-storage` | **Delete** | Not in extraction plan |
| `fabryk-query` | **Delete** | Not in extraction plan |
| `fabryk-api` | **Delete** | Not in extraction plan |
| `fabryk-client` | **Delete** | Not in extraction plan |

**Crates to create** (not currently in ECL):

| Crate | Purpose |
|-------|---------|
| `fabryk-content` | Markdown parsing, frontmatter extraction |
| `fabryk-fts` | Full-text search (Tantivy) |
| `fabryk-graph` | Knowledge graph, algorithms, persistence |
| `fabryk-mcp-content` | Content & source MCP tools |
| `fabryk-mcp-fts` | FTS MCP tools |
| `fabryk-mcp-graph` | Graph MCP tools |

---

# Milestone 1.0: ECL Workspace Cleanup

## Objective

Prepare the ECL workspace for Fabryk extraction by:

1. Removing crates that are not part of the extraction plan
2. Gutting existing stubs to empty shells
3. Updating the workspace `Cargo.toml` with new structure and dependencies

## Implementation Steps

### Step 1.0.1: Remove legacy crates

Remove crates that are not part of the extraction plan:

```bash
cd ~/lab/oxur/ecl

# Remove from workspace members in Cargo.toml first (manual edit)
# Then delete the directories:
rm -rf crates/fabryk-storage
rm -rf crates/fabryk-query
rm -rf crates/fabryk-api
rm -rf crates/fabryk-client
```

### Step 1.0.2: Gut existing stubs

For each crate we're keeping (`fabryk`, `fabryk-core`, `fabryk-acl`, `fabryk-mcp`,
`fabryk-cli`), replace all source files with minimal stubs.

**fabryk-core/src/lib.rs**:
```rust
//! Core types, traits, errors, and utilities for the Fabryk knowledge fabric.
//!
//! This crate provides the foundational building blocks used by all other
//! Fabryk crates. It has no internal Fabryk dependencies.
//!
//! # Features
//!
//! - Error types and `Result` alias
//! - File and path utilities
//! - `ConfigProvider` trait for domain configuration
//! - `AppState` for application state management

#![doc = include_str!("../README.md")]

pub mod error;

pub use error::{Error, Result};

// Modules to be added during extraction:
// pub mod util;
// pub mod traits;
// pub mod state;
// pub mod resources;
```

**fabryk-core/src/error.rs**:

> **Note**: This is a minimal stub using `thiserror` to allow the workspace to
> compile. **Milestone 1.2** expands this with the full error type extracted from
> music-theory, including additional variants (`InvalidPath`, `ParseError`, etc.),
> backtrace support via `#[backtrace]`, and inspector methods (`is_io()`,
> `is_not_found()`, `is_config()`).

```rust
//! Error types for Fabryk operations.
//!
//! This module provides a common `Error` type and `Result<T>` alias used across
//! all Fabryk crates. Uses `thiserror` for derive macros.
//!
//! **Note**: This is a minimal stub. See milestone 1.2 for the full extraction.

use thiserror::Error;

/// Errors that can occur in Fabryk operations.
///
/// This is a minimal stub — milestone 1.2 adds additional variants,
/// backtrace support, and inspector methods.
#[derive(Error, Debug)]
pub enum Error {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Content not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid data or format.
    #[error("Invalid data: {0}")]
    InvalidData(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl Error {
    /// Create a configuration error.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a not found error.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    /// Create an invalid data error.
    pub fn invalid_data(msg: impl Into<String>) -> Self {
        Self::InvalidData(msg.into())
    }
}

/// Result type alias using Fabryk's Error type.
pub type Result<T> = std::result::Result<T, Error>;
```

Remove old files from `fabryk-core/src/`:
```bash
cd ~/lab/oxur/ecl/crates/fabryk-core/src
rm -f identity.rs item.rs partition.rs tag.rs traits.rs
```

**Similar gutting for other kept crates** — each gets a minimal `lib.rs` with doc
comment and placeholder structure.

### Step 1.0.3: Update workspace Cargo.toml

Edit `~/lab/oxur/ecl/Cargo.toml`:

1. Remove deleted crates from `members`
2. Add new Fabryk crates to `members`
3. Add missing dependencies to `[workspace.dependencies]`
4. Update `[workspace.package]` version to `0.1.0-alpha.0`

```toml
[workspace]
resolver = "2"
members = [
    # ECL crates
    "crates/ecl",
    "crates/design",
    "crates/ecl-core",
    "crates/ecl-steps",
    "crates/ecl-workflows",
    "crates/ecl-cli",
    # Fabryk crates
    "crates/fabryk",
    "crates/fabryk-core",
    "crates/fabryk-content",
    "crates/fabryk-fts",
    "crates/fabryk-graph",
    "crates/fabryk-acl",
    "crates/fabryk-mcp",
    "crates/fabryk-mcp-content",
    "crates/fabryk-mcp-fts",
    "crates/fabryk-mcp-graph",
    "crates/fabryk-cli",
]

[workspace.package]
version = "0.1.0-alpha.0"
edition = "2021"
rust-version = "1.75"
authors = ["Duncan McGreggor <oubiwann@gmail.com>"]
license = "Apache-2.0"
repository = "https://github.com/oxur/ecl"
homepage = "https://github.com/oxur/ecl"
description = "ECL & Fabryk — Workflow Orchestration and Knowledge Fabric"

[workspace.dependencies]
# ============================================================
# ECL Dependencies (existing)
# ============================================================

# Design documentation management
oxur-odm = { version = "0.3.0", path = "../oxur/crates/oxur-odm" }

# Restate SDK
restate-sdk = "0.7"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres", "chrono", "uuid"] }

# LLM
llm = "1.3"

# Resilience
backon = "1"
failsafe = "1"

# HTTP
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["trace", "cors"] }
reqwest = { version = "0.12", features = ["json"] }

# Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
uuid = { version = "1", features = ["v4", "serde"] }
anyhow = "1"

# Testing (ECL)
proptest = "1"
mockall = "0.12"

# ============================================================
# Shared Dependencies (ECL & Fabryk)
# ============================================================

# Async runtime
tokio = { version = "1", features = ["full", "fs", "sync"] }
futures = "0.3"
async-trait = "0.1"

# Configuration and logging
confyg = "0.3"
twyg = "0.6"
log = "0.4"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
thiserror = "2"

# Time
chrono = { version = "0.4", features = ["serde"] }

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Testing
tempfile = "3"

# ============================================================
# Fabryk-Specific Dependencies
# ============================================================

# MCP SDK
rmcp = { version = "0.14", features = ["server", "transport-io"] }

# Schema support
schemars = "1.2"

# Markdown and frontmatter
pulldown-cmark = "0.9"
serde_yaml = "0.9"

# File operations
glob = "0.3"
shellexpand = "3.1"
dirs = "5"
async-walkdir = "1"

# Regular expressions
regex = "1"

# Full-text search
tantivy = "0.22"
stop-words = "0.8"

# String matching (for source resolution)
strsim = "0.11"
toml_edit = "0.22"

# Graph database
petgraph = "0.6"
rkyv = { version = "0.7", features = ["validation"] }
memmap2 = "0.9"
blake3 = "1.5"

# Dev dependencies
tokio-test = "0.4"

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

### Step 1.0.4: Verify workspace compiles

```bash
cd ~/lab/oxur/ecl
cargo check --workspace
```

This will fail until we create the new crate stubs in Milestone 1.1, but the
existing crates should still compile.

## Exit Criteria (Milestone 1.0)

- [ ] `fabryk-storage`, `fabryk-query`, `fabryk-api`, `fabryk-client` directories deleted
- [ ] Workspace `Cargo.toml` updated with new member list
- [ ] Workspace `Cargo.toml` updated with all Fabryk dependencies
- [ ] Workspace version is `0.1.0-alpha.0`
- [ ] Existing Fabryk crates gutted to minimal stubs
- [ ] `fabryk-core` has proper `Error` enum and `Result` type alias
- [ ] `cargo check` passes for existing crates (ignoring missing new crates)

## Commit Message (Milestone 1.0)

```
chore(fabryk): clean up ECL workspace for Fabryk extraction

Remove legacy Fabryk stubs that don't match the extraction plan:
- fabryk-storage, fabryk-query, fabryk-api, fabryk-client

Gut existing stubs to minimal shells:
- fabryk, fabryk-core, fabryk-acl, fabryk-mcp, fabryk-cli

Update workspace Cargo.toml:
- Add all Fabryk dependencies from music-theory project
- Update version to 0.1.0-alpha.0
- Prepare member list for new crates

Ref: Doc 0013 milestone 1.0
```

---

# Milestone 1.1: Workspace Scaffold

## Prerequisites

- Milestone 1.0 complete (ECL workspace cleaned up)

## Objective

Create the remaining Fabryk crate stubs so that subsequent milestones can focus
on extraction without scaffolding overhead:

- Create 6 new crate stubs: `fabryk-content`, `fabryk-fts`, `fabryk-graph`,
  `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`
- Update all Fabryk crate `Cargo.toml` files with proper dependencies
- Create minimal `README.md` for each crate
- Verify: `cargo check --workspace` succeeds

## Implementation Steps

### Step 1.1.1: Create new crate directories

```bash
cd ~/lab/oxur/ecl/crates

# Create new crate structures
for crate in fabryk-content fabryk-fts fabryk-graph \
             fabryk-mcp-content fabryk-mcp-fts fabryk-mcp-graph; do
    mkdir -p "$crate/src"
done
```

### Step 1.1.2: Create crate Cargo.toml files

Each `Cargo.toml` inherits from workspace where possible. Below are the complete
files for all 11 Fabryk crates.

**Note on dependency versions:** All versions match the music-theory MCP server's
current dependencies to avoid conflicts during extraction. Dependency updates
will be done incrementally after extraction is verified working.

---

#### fabryk/Cargo.toml (umbrella re-export crate)

```toml
[package]
name = "fabryk"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Fabryk knowledge fabric — umbrella crate re-exporting all components"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-content = { path = "../fabryk-content" }
# Feature-gated dependencies
fabryk-fts = { path = "../fabryk-fts", optional = true }
fabryk-graph = { path = "../fabryk-graph", optional = true }
fabryk-mcp = { path = "../fabryk-mcp", optional = true }
fabryk-mcp-content = { path = "../fabryk-mcp-content", optional = true }
fabryk-mcp-fts = { path = "../fabryk-mcp-fts", optional = true }
fabryk-mcp-graph = { path = "../fabryk-mcp-graph", optional = true }
fabryk-cli = { path = "../fabryk-cli", optional = true }

[features]
default = []
full = ["fts", "graph", "mcp", "cli"]
fts = ["dep:fabryk-fts", "fts-tantivy"]
fts-tantivy = ["fabryk-fts?/fts-tantivy"]
graph = ["dep:fabryk-graph", "graph-rkyv-cache"]
graph-rkyv-cache = ["fabryk-graph?/graph-rkyv-cache"]
mcp = ["dep:fabryk-mcp", "dep:fabryk-mcp-content", "dep:fabryk-mcp-fts", "dep:fabryk-mcp-graph"]
cli = ["dep:fabryk-cli"]
```

---

#### fabryk-core/Cargo.toml

```toml
[package]
name = "fabryk-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Core types, traits, errors, and utilities for the Fabryk knowledge fabric"

[dependencies]
# Async runtime
tokio = { workspace = true }
async-trait = { workspace = true }

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
async-walkdir = { workspace = true }

# Logging
log = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-content/Cargo.toml

```toml
[package]
name = "fabryk-content"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Markdown parsing, frontmatter extraction, and content utilities for Fabryk"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# Markdown parsing
pulldown-cmark = { workspace = true }

# Frontmatter
serde_yaml = { workspace = true }

# Serialization
serde = { workspace = true }

# Regex for content extraction
regex = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

---

#### fabryk-fts/Cargo.toml

```toml
[package]
name = "fabryk-fts"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Full-text search infrastructure for Fabryk (Tantivy backend)"

[features]
default = []
fts-tantivy = ["dep:tantivy", "dep:stop-words"]

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Logging
log = { workspace = true }

# Full-text search (feature-gated)
tantivy = { workspace = true, optional = true }
stop-words = { workspace = true, optional = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-graph/Cargo.toml

```toml
[package]
name = "fabryk-graph"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Knowledge graph infrastructure for Fabryk (petgraph + rkyv persistence)"

[features]
default = []
graph-rkyv-cache = ["dep:rkyv", "dep:memmap2", "dep:blake3"]

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-content = { path = "../fabryk-content" }

# Graph
petgraph = { workspace = true }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Time
chrono = { workspace = true }

# Logging
log = { workspace = true }

# Persistence (feature-gated)
rkyv = { workspace = true, optional = true }
memmap2 = { workspace = true, optional = true }
blake3 = { workspace = true, optional = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-acl/Cargo.toml

```toml
[package]
name = "fabryk-acl"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Access control layer for Fabryk (placeholder for v0.2/v0.3)"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# Async
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }

[dev-dependencies]
# None yet - placeholder crate
```

---

#### fabryk-mcp/Cargo.toml

```toml
[package]
name = "fabryk-mcp"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Core MCP server infrastructure for Fabryk"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# MCP SDK
rmcp = { workspace = true }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Schema
schemars = { workspace = true }

# Logging
log = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-mcp-content/Cargo.toml

```toml
[package]
name = "fabryk-mcp-content"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Content and source MCP tools for Fabryk (ContentItemProvider, SourceProvider)"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-content = { path = "../fabryk-content" }
fabryk-mcp = { path = "../fabryk-mcp" }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Schema
schemars = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-mcp-fts/Cargo.toml

```toml
[package]
name = "fabryk-mcp-fts"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Full-text search MCP tools for Fabryk"

[features]
default = []
fts-tantivy = ["fabryk-fts/fts-tantivy"]

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-fts = { path = "../fabryk-fts" }
fabryk-mcp = { path = "../fabryk-mcp" }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Schema
schemars = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-mcp-graph/Cargo.toml

```toml
[package]
name = "fabryk-mcp-graph"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Graph query MCP tools for Fabryk"

[features]
default = []
graph-rkyv-cache = ["fabryk-graph/graph-rkyv-cache"]

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-graph = { path = "../fabryk-graph" }
fabryk-mcp = { path = "../fabryk-mcp" }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Schema
schemars = { workspace = true }

# Graph (for types)
petgraph = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

---

#### fabryk-cli/Cargo.toml

```toml
[package]
name = "fabryk-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "CLI framework for Fabryk-based applications"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# CLI
clap = { workspace = true }

# Async
tokio = { workspace = true }

# Logging
twyg = { workspace = true }
log = { workspace = true }

# Serialization (for config)
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

---

### Step 1.1.3: Create lib.rs stubs for new crates

Each `lib.rs` follows the pattern with doc comments and README inclusion.

**fabryk/src/lib.rs**:
```rust
//! Fabryk knowledge fabric — umbrella crate.
//!
//! This crate re-exports all Fabryk components for convenience.
//! Use feature flags to enable specific functionality.

#![doc = include_str!("../README.md")]

pub use fabryk_core as core;
pub use fabryk_content as content;

#[cfg(feature = "fts")]
pub use fabryk_fts as fts;

#[cfg(feature = "graph")]
pub use fabryk_graph as graph;

#[cfg(feature = "mcp")]
pub use fabryk_mcp as mcp;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_content as mcp_content;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_fts as mcp_fts;

#[cfg(feature = "mcp")]
pub use fabryk_mcp_graph as mcp_graph;

#[cfg(feature = "cli")]
pub use fabryk_cli as cli;
```

**fabryk-content/src/lib.rs**:
```rust
//! Markdown parsing, frontmatter extraction, and content utilities.
//!
//! This crate provides generic content processing used by all Fabryk
//! domains.
//!
//! # Features
//!
//! - Frontmatter extraction from markdown files
//! - Markdown parsing and section extraction
//! - Helper utilities for content processing

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod markdown;
// pub mod helpers;
```

**fabryk-fts/src/lib.rs**:
```rust
//! Full-text search infrastructure for Fabryk.
//!
//! This crate provides search functionality with a Tantivy backend
//! (feature-gated).
//!
//! # Features
//!
//! - `fts-tantivy`: Enable Tantivy-based full-text search
//!
//! # Default Schema
//!
//! Fabryk provides a sensible default schema suitable for knowledge
//! domains. Custom schemas can be added in future versions.

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod backend;
// pub mod schema;
// pub mod document;
// pub mod query;
// #[cfg(feature = "fts-tantivy")]
// pub mod tantivy_search;
// #[cfg(feature = "fts-tantivy")]
// pub mod indexer;
```

**fabryk-graph/src/lib.rs**:
```rust
//! Knowledge graph infrastructure for Fabryk.
//!
//! This crate provides graph storage, traversal algorithms, and
//! persistence using petgraph and optional rkyv caching.
//!
//! # Features
//!
//! - `graph-rkyv-cache`: Enable rkyv-based graph persistence with
//!   content-hash cache validation
//!
//! # Key Abstractions
//!
//! - `GraphExtractor` trait: Domain implementations provide this to
//!   extract nodes and edges from content files
//! - `Relationship` enum: Common relationship types with `Custom(String)`
//!   for domain-specific relationships

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod types;
// pub mod extractor;
// pub mod builder;
// pub mod algorithms;
// pub mod persistence;
// pub mod query;
// pub mod stats;
// pub mod validation;
```

**fabryk-acl/src/lib.rs**:
```rust
//! Access control layer for Fabryk.
//!
//! This crate is a placeholder for v0.2/v0.3. It will provide:
//!
//! - User/tenant identification
//! - Permission checking
//! - Resource ownership
//! - Multi-tenancy isolation

#![doc = include_str!("../README.md")]

// Placeholder - implementation deferred to v0.2/v0.3
```

**fabryk-mcp/src/lib.rs**:
```rust
//! Core MCP server infrastructure for Fabryk.
//!
//! This crate provides the foundational MCP server setup, tool
//! registration, and the health check tool.
//!
//! # Key Abstractions
//!
//! - `FabrykMcpServer<C>`: Generic MCP server parameterized over config
//! - `ToolRegistry` trait: Domain implementations register their tools

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod server;
// pub mod registry;
// pub mod tools;
```

**fabryk-mcp-content/src/lib.rs**:
```rust
//! Content and source MCP tools for Fabryk.
//!
//! This crate provides MCP tools for listing and retrieving content
//! items and source materials.
//!
//! # Key Abstractions
//!
//! - `ContentItemProvider` trait: Domain implementations provide item
//!   listing and retrieval
//! - `SourceProvider` trait: Domain implementations provide source
//!   material access

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod traits;
// pub mod tools;
```

**fabryk-mcp-fts/src/lib.rs**:
```rust
//! Full-text search MCP tools for Fabryk.
//!
//! This crate provides MCP tools for searching content.

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod tools;
```

**fabryk-mcp-graph/src/lib.rs**:
```rust
//! Graph query MCP tools for Fabryk.
//!
//! This crate provides MCP tools for graph inspection and queries.

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod tools;
```

**fabryk-cli/src/lib.rs**:
```rust
//! CLI framework for Fabryk-based applications.
//!
//! This crate provides a generic CLI structure that domain applications
//! can extend with their own commands.
//!
//! # Key Abstractions
//!
//! - `FabrykCli<C>`: Generic CLI parameterized over config provider
//! - Support for domain-specific subcommand registration

#![doc = include_str!("../README.md")]

// Modules to be added during extraction:
// pub mod cli;
// pub mod commands;
```

### Step 1.1.4: Create README.md files

Create a minimal `README.md` for each crate. Example for `fabryk-core`:

**fabryk-core/README.md**:
```markdown
# fabryk-core

Core types, traits, errors, and utilities for the Fabryk knowledge fabric.

## Status

Under construction — being extracted from the music-theory MCP server.

## Features

- Error types and `Result` alias
- File and path utilities
- `ConfigProvider` trait for domain configuration
- `AppState` for application state management

## License

Apache-2.0
```

Create similar READMEs for all 11 crates.

### Step 1.1.5: Update fabryk-core with proper Error and Result

Ensure `fabryk-core/src/error.rs` contains the full error implementation
(as shown in Step 1.0.2) and `lib.rs` exports it properly.

### Step 1.1.6: Verify workspace compiles

```bash
cd ~/lab/oxur/ecl
cargo check --workspace
cargo clippy --workspace -- -D warnings
```

Both should pass with the stub implementations.

## Exit Criteria (Milestone 1.1)

- [ ] All 11 Fabryk crates exist with `Cargo.toml` and `src/lib.rs`
- [ ] `cargo check --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` passes
- [ ] Each crate has a `Cargo.toml` with appropriate workspace inheritance
- [ ] Feature flags configured: `fts-tantivy` (fabryk-fts), `graph-rkyv-cache` (fabryk-graph)
- [ ] Each crate has a `README.md`
- [ ] `fabryk-core` has proper `Error` enum and `Result` type alias
- [ ] All `lib.rs` files include `#![doc = include_str!("../README.md")]`

## Commit Message (Milestone 1.1)

```
feat(fabryk): scaffold all Fabryk crates for extraction

Create the complete Fabryk crate structure:
- fabryk (umbrella), fabryk-core, fabryk-content, fabryk-fts,
  fabryk-graph, fabryk-acl, fabryk-mcp, fabryk-mcp-content,
  fabryk-mcp-fts, fabryk-mcp-graph, fabryk-cli

All crates are stubs with Cargo.toml, lib.rs, and README.md.
Workspace dependencies centralised. Feature flags configured:
- fts-tantivy (fabryk-fts, fabryk-mcp-fts)
- graph-rkyv-cache (fabryk-graph, fabryk-mcp-graph)

Ref: Doc 0013 milestone 1.1, Audit §6.1
```

---

## Appendix: Dependency Version Reference

These versions are from the music-theory MCP server and should be used for
all Fabryk crates to ensure compatibility during extraction:

| Dependency | Version | Used By |
|------------|---------|---------|
| `tokio` | `1` (full, fs, sync) | All |
| `async-trait` | `0.1` | All |
| `serde` | `1` (derive) | All |
| `serde_json` | `1` | All |
| `serde_yaml` | `0.9` | core, content |
| `thiserror` | `2` | core |
| `rmcp` | `0.14` | mcp |
| `schemars` | `1.2` | mcp-* |
| `pulldown-cmark` | `0.9` | content |
| `tantivy` | `0.22` | fts |
| `stop-words` | `0.8` | fts |
| `petgraph` | `0.6` | graph |
| `rkyv` | `0.7` (validation) | graph |
| `memmap2` | `0.9` | graph |
| `blake3` | `1.5` | graph |
| `chrono` | `0.4` (serde) | graph |
| `clap` | `4` (derive, env) | cli |
| `twyg` | `0.6` | cli |
| `log` | `0.4` | All |
| `glob` | `0.3` | core |
| `shellexpand` | `3.1` | core |
| `dirs` | `5` | core |
| `async-walkdir` | `1` | core |
| `regex` | `1` | content |
| `strsim` | `0.11` | (future: sources) |
| `toml_edit` | `0.22` | (future: sources) |
| `tempfile` | `3` | dev |
| `tokio-test` | `0.4` | dev |
