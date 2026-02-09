---
title: "CC Prompt: Fabryk 3.1 — FTS Crate Scaffold"
milestone: "3.1"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 2 complete (2.1-2.4)"]
governing-docs: [0011-audit §4.3, 0012-amendment §2d, 0013-project-plan]
---

# CC Prompt: Fabryk 3.1 — FTS Crate Scaffold

## Context

Phase 3 extracts full-text search infrastructure from music-theory to
`fabryk-fts`. This is the largest extraction phase before the graph (Phase 4),
with ~1,620 lines across 11 files.

**Crate:** `fabryk-fts`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Medium (Tantivy integration complexity)

**Key architectural decision (Amendment §2d):**

> Defer `SearchSchemaProvider` to v0.2. Ship v0.1 with a sensible default schema.
> The current music theory schema fields are knowledge-domain-general — they
> describe "a piece of knowledge with metadata," not "a music theory concept."

This means `fabryk-fts` ships with a hardcoded default schema covering:
- Full-text: title, description, content
- Facets: category, source, tags
- Stored: id, path, chapter, part, author, date, content_type, section

**Music-Theory Migration**: Code is extracted to Fabryk; music-theory continues
using local copies until the v0.1-alpha checkpoint (after this phase).

## Source Files Overview

From research of music-theory `search/` directory:

| File | Lines | Classification | Target |
|------|-------|----------------|--------|
| `mod.rs` | 52 | N/A | Organization |
| `backend.rs` | 153 | G (Generic) | fabryk-fts |
| `schema.rs` | 309 | G (Generic) | fabryk-fts |
| `document.rs` | 489 | P (Parameterized) | fabryk-fts |
| `query.rs` | 729 | P (Parameterized) | fabryk-fts |
| `indexer.rs` | 455 | P (Parameterized) | fabryk-fts |
| `builder.rs` | 548 | P (Parameterized) | fabryk-fts |
| `tantivy_search.rs` | 767 | P (Parameterized) | fabryk-fts |
| `simple_search.rs` | 195 | D (Domain) | Adapt or remove |
| `freshness.rs` | 453 | D (Domain) | Adapt |
| `stopwords.rs` | 368 | G (Generic) | fabryk-fts |
| **Total** | ~1,620 | | |

**Test coverage:** ~120 tests across all modules.

## Objective

1. Expand `fabryk-fts` crate stub (from Phase 1) with proper module structure
2. Set up feature flags (`fts-tantivy`)
3. Configure dependencies (Tantivy, stop-words, etc.)
4. Create module organization matching extraction targets
5. Verify: `cargo check -p fabryk-fts` passes

## Implementation Steps

### Step 1: Update `fabryk-fts/Cargo.toml`

Expand the stub from milestone 1.1 with complete dependencies:

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
futures = { workspace = true }

# Serialization
serde = { workspace = true }
serde_json = { workspace = true }

# Logging
log = { workspace = true }

# File operations (for freshness hashing)
async-walkdir = { workspace = true }

# Full-text search (feature-gated)
tantivy = { workspace = true, optional = true }
stop-words = { workspace = true, optional = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio-test = { workspace = true }
```

### Step 2: Create module structure

```bash
cd ~/lab/oxur/ecl/crates/fabryk-fts
mkdir -p src
```

### Step 3: Create `fabryk-fts/src/lib.rs`

```rust
//! Full-text search infrastructure for Fabryk.
//!
//! This crate provides search functionality with a Tantivy backend (feature-gated).
//! It includes a default schema suitable for knowledge domains, query building,
//! indexing, and search execution.
//!
//! # Features
//!
//! - `fts-tantivy`: Enable Tantivy-based full-text search (recommended)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      fabryk-fts                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SearchBackend trait                                        │
//! │  ├── SimpleSearch (linear scan fallback)                    │
//! │  └── TantivySearch (full-text with Tantivy)                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SearchSchema (default 14-field schema)                     │
//! │  SearchDocument (indexed document representation)           │
//! │  QueryBuilder (weighted multi-field queries)                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Indexer (Tantivy index writer)                            │
//! │  IndexBuilder (batch indexing orchestration)               │
//! │  IndexFreshness (content hash validation)                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Default Schema
//!
//! Per Amendment §2d, Fabryk ships with a sensible default schema suitable for
//! any knowledge domain:
//!
//! | Field | Type | Purpose |
//! |-------|------|---------|
//! | `id` | STRING | Unique identifier |
//! | `path` | STORED | File path |
//! | `title` | TEXT | Full-text, boosted 3.0x |
//! | `description` | TEXT | Full-text, boosted 2.0x |
//! | `content` | TEXT | Full-text, boosted 1.0x |
//! | `category` | STRING | Facet filtering |
//! | `source` | STRING | Facet filtering |
//! | `tags` | STRING | Facet filtering |
//! | `chapter` | STORED | Metadata |
//! | `part` | STORED | Metadata |
//! | `author` | STORED | Metadata |
//! | `date` | STORED | Metadata |
//! | `content_type` | STRING | Content type classification |
//! | `section` | STORED | Section reference |
//!
//! Custom schemas can be added via `SearchSchemaProvider` trait in future
//! versions (v0.2+).
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::{SearchBackend, SearchParams, create_search_backend};
//!
//! // Create backend (uses config to choose Tantivy or SimpleSearch)
//! let backend = create_search_backend(&config).await?;
//!
//! // Execute search
//! let params = SearchParams {
//!     query: "functional harmony".to_string(),
//!     limit: Some(10),
//!     category: Some("harmony".to_string()),
//!     ..Default::default()
//! };
//!
//! let results = backend.search(params).await?;
//! for result in results.items {
//!     println!("{}: {}", result.id, result.title);
//! }
//! ```

#![doc = include_str!("../README.md")]

// Core modules (always available)
pub mod backend;
pub mod document;
pub mod types;

// Feature-gated Tantivy modules
#[cfg(feature = "fts-tantivy")]
pub mod schema;

#[cfg(feature = "fts-tantivy")]
pub mod query;

#[cfg(feature = "fts-tantivy")]
pub mod indexer;

#[cfg(feature = "fts-tantivy")]
pub mod builder;

#[cfg(feature = "fts-tantivy")]
pub mod freshness;

#[cfg(feature = "fts-tantivy")]
pub mod stopwords;

#[cfg(feature = "fts-tantivy")]
pub mod tantivy_search;

// Re-exports
pub use backend::{SearchBackend, SearchParams, SearchResult, SearchResults};
pub use document::SearchDocument;
pub use types::{QueryMode, SearchConfig};

#[cfg(feature = "fts-tantivy")]
pub use schema::SearchSchema;

#[cfg(feature = "fts-tantivy")]
pub use query::QueryBuilder;

#[cfg(feature = "fts-tantivy")]
pub use indexer::Indexer;

#[cfg(feature = "fts-tantivy")]
pub use builder::{IndexBuilder, IndexStats};

#[cfg(feature = "fts-tantivy")]
pub use freshness::IndexMetadata;

#[cfg(feature = "fts-tantivy")]
pub use stopwords::StopwordFilter;

#[cfg(feature = "fts-tantivy")]
pub use tantivy_search::TantivySearch;

/// Create a search backend based on configuration.
///
/// Returns `TantivySearch` if:
/// - The `fts-tantivy` feature is enabled
/// - An index exists at the configured path
///
/// Otherwise returns `SimpleSearch` as fallback.
pub async fn create_search_backend(
    config: &SearchConfig,
) -> fabryk_core::Result<Box<dyn SearchBackend>> {
    backend::create_search_backend(config).await
}
```

### Step 4: Create `fabryk-fts/src/types.rs`

```rust
//! Common types for the FTS module.
//!
//! These types are used across all search backends and are always available
//! regardless of feature flags.

use serde::{Deserialize, Serialize};

/// Search query mode.
///
/// Controls how multiple search terms are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMode {
    /// Smart mode: AND for 1-2 terms, OR with minimum match for 3+.
    #[default]
    Smart,
    /// All terms must match (AND).
    And,
    /// Any term can match (OR).
    Or,
    /// At least N terms must match (configured separately).
    MinimumMatch,
}

/// Search configuration.
///
/// Domain implementations provide this to configure search behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Backend type: "tantivy" or "simple".
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Path to the search index directory.
    pub index_path: Option<String>,

    /// Path to content for indexing.
    pub content_path: Option<String>,

    /// Default query mode.
    #[serde(default)]
    pub query_mode: QueryMode,

    /// Enable fuzzy matching.
    #[serde(default)]
    pub fuzzy_enabled: bool,

    /// Fuzzy edit distance (1 or 2).
    #[serde(default = "default_fuzzy_distance")]
    pub fuzzy_distance: u8,

    /// Enable stopword filtering.
    #[serde(default = "default_true")]
    pub stopwords_enabled: bool,

    /// Custom stopwords to add.
    #[serde(default)]
    pub custom_stopwords: Vec<String>,

    /// Words to preserve (not filter as stopwords).
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Default result limit.
    #[serde(default = "default_limit")]
    pub default_limit: usize,

    /// Snippet length in characters.
    #[serde(default = "default_snippet_length")]
    pub snippet_length: usize,
}

fn default_backend() -> String {
    "tantivy".to_string()
}

fn default_fuzzy_distance() -> u8 {
    1
}

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    10
}

fn default_snippet_length() -> usize {
    200
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            index_path: None,
            content_path: None,
            query_mode: QueryMode::default(),
            fuzzy_enabled: false,
            fuzzy_distance: default_fuzzy_distance(),
            stopwords_enabled: default_true(),
            custom_stopwords: Vec::new(),
            allowlist: Vec::new(),
            default_limit: default_limit(),
            snippet_length: default_snippet_length(),
        }
    }
}
```

### Step 5: Create stub modules

Create placeholder files for each module to be filled in subsequent milestones:

**`fabryk-fts/src/backend.rs`** (stub):
```rust
//! Search backend trait and factory.

use async_trait::async_trait;
use fabryk_core::Result;
use serde::{Deserialize, Serialize};

use crate::types::SearchConfig;

/// Parameters for a search request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchParams {
    /// Search query string.
    pub query: String,
    /// Maximum results to return.
    pub limit: Option<usize>,
    /// Filter by category.
    pub category: Option<String>,
    /// Filter by source.
    pub source: Option<String>,
    /// Filter by content types.
    pub content_types: Option<Vec<String>>,
    /// Query mode override.
    pub query_mode: Option<crate::types::QueryMode>,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub source: Option<String>,
    pub snippet: Option<String>,
    pub relevance: f32,
    pub content_type: Option<String>,
    pub path: Option<String>,
}

/// Collection of search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    pub items: Vec<SearchResult>,
    pub total: usize,
    pub backend: String,
}

/// Abstract search backend trait.
///
/// Implementations provide different search strategies:
/// - `TantivySearch`: Full-text search with Tantivy
/// - `SimpleSearch`: Linear scan fallback
#[async_trait]
pub trait SearchBackend: Send + Sync {
    /// Execute a search query.
    async fn search(&self, params: SearchParams) -> Result<SearchResults>;

    /// Get the backend name for diagnostics.
    fn name(&self) -> &str;
}

/// Create a search backend based on configuration.
pub async fn create_search_backend(
    _config: &SearchConfig,
) -> Result<Box<dyn SearchBackend>> {
    // Stub - will be implemented in milestone 3.3
    todo!("Implement backend factory")
}
```

**`fabryk-fts/src/document.rs`** (stub):
```rust
//! Search document representation.

use serde::{Deserialize, Serialize};

/// A document to be indexed and searched.
///
/// This struct holds all fields that can be indexed and searched.
/// It maps to the default schema defined in `schema.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchDocument {
    // Identity
    pub id: String,
    pub path: String,

    // Full-text fields
    pub title: String,
    pub description: Option<String>,
    pub content: String,

    // Facet fields
    pub category: String,
    pub source: Option<String>,
    pub tags: Vec<String>,

    // Metadata fields
    pub chapter: Option<String>,
    pub part: Option<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub content_type: Option<String>,
    pub section: Option<String>,
}

// Implementation to be added in milestone 3.3
```

### Step 6: Update README

**`fabryk-fts/README.md`**:
```markdown
# fabryk-fts

Full-text search infrastructure for the Fabryk knowledge fabric.

## Status

Under construction — being extracted from the music-theory MCP server.

## Features

- `fts-tantivy`: Enable Tantivy-based full-text search (recommended)

## Default Schema

Fabryk ships with a sensible default schema suitable for any knowledge domain:

| Field | Type | Purpose |
|-------|------|---------|
| `id` | STRING | Unique identifier |
| `path` | STORED | File path |
| `title` | TEXT | Full-text, boosted 3.0x |
| `description` | TEXT | Full-text, boosted 2.0x |
| `content` | TEXT | Full-text, boosted 1.0x |
| `category` | STRING | Facet filtering |
| `source` | STRING | Facet filtering |
| `tags` | STRING | Facet filtering |
| `chapter` | STORED | Metadata |
| `part` | STORED | Metadata |
| `author` | STORED | Metadata |
| `date` | STORED | Metadata |
| `content_type` | STRING | Content type classification |
| `section` | STORED | Section reference |

## License

Apache-2.0
```

### Step 7: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-fts
cargo check -p fabryk-fts --features fts-tantivy
cargo clippy -p fabryk-fts -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-fts/Cargo.toml` updated with full dependencies
- [ ] Feature flag `fts-tantivy` configured
- [ ] `fabryk-fts/src/lib.rs` with module structure and re-exports
- [ ] `fabryk-fts/src/types.rs` with `QueryMode` and `SearchConfig`
- [ ] `fabryk-fts/src/backend.rs` stub with `SearchBackend` trait
- [ ] `fabryk-fts/src/document.rs` stub with `SearchDocument`
- [ ] `cargo check -p fabryk-fts` passes
- [ ] `cargo check -p fabryk-fts --features fts-tantivy` passes
- [ ] README updated with feature documentation

## Design Notes

### Feature-gated architecture

The crate uses feature flags to avoid pulling in heavy Tantivy dependencies
when not needed:

```toml
[features]
fts-tantivy = ["dep:tantivy", "dep:stop-words"]
```

Without the feature:
- `SearchBackend` trait available
- `SearchDocument` and `SearchParams` available
- `SimpleSearch` fallback available (linear scan)

With the feature:
- All Tantivy-based modules available
- Full-text search with BM25 scoring
- English stemming tokenizer
- Stopword filtering

### Default schema rationale (Amendment §2d)

The music-theory schema fields are knowledge-domain-general:

> These fields aren't music-theory-specific. A math skill searches by title,
> content, and category too. A Rust skill has the same needs.

For v0.1, the schema is hardcoded. Custom schemas via `SearchSchemaProvider`
are deferred to v0.2 if domains genuinely need different fields.

## Commit Message

```
feat(fts): scaffold fabryk-fts crate for Phase 3 extraction

Set up fabryk-fts crate structure:
- Feature flag fts-tantivy for Tantivy dependencies
- SearchBackend trait and factory function
- SearchDocument for indexed content
- SearchConfig and QueryMode types
- Module structure for extraction targets

Default schema defined per Amendment §2d (14 fields,
knowledge-domain-general).

Ref: Doc 0013 milestone 3.1, Amendment §2d

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
