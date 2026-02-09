---
title: "CC Prompt: Fabryk 5.4 — FTS MCP Tools"
milestone: "5.4"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["5.1-5.3 complete", "Phase 3 fabryk-fts complete"]
governing-docs: [0011-audit §4.8, 0013-project-plan]
---

# CC Prompt: Fabryk 5.4 — FTS MCP Tools

## Context

This milestone extracts the search MCP tools to `fabryk-mcp-fts`. These tools
use the `fabryk-fts` infrastructure created in Phase 3.

The search tools provide:
- Full-text search across content
- Faceted filtering (category, source, tags)
- Relevance scoring and result highlighting

## Objective

Create `fabryk-mcp-fts` crate with:

1. Search tool that delegates to `fabryk-fts::SearchBackend`
2. Suggest tool for autocomplete
3. Index status tool
4. Parameterized response types using default schema fields

## Implementation Steps

### Step 1: Create fabryk-mcp-fts crate

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-mcp-fts/src
```

Create `fabryk-mcp-fts/Cargo.toml`:

```toml
[package]
name = "fabryk-mcp-fts"
version = "0.1.0"
edition = "2021"
description = "Full-text search MCP tools for Fabryk domains"
license = "Apache-2.0"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-fts = { path = "../fabryk-fts" }
fabryk-mcp = { path = "../fabryk-mcp" }

async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
mcp-core = "0.2"
tokio = { version = "1.0", features = ["fs"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

### Step 2: Define search response types

Create `fabryk-mcp-fts/src/responses.rs`:

```rust
//! Search response types for MCP tools.

use serde::{Deserialize, Serialize};

/// A single search result.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    /// Item ID.
    pub id: String,
    /// Item title.
    pub title: String,
    /// Item description (if available).
    pub description: Option<String>,
    /// Category.
    pub category: Option<String>,
    /// Source reference.
    pub source: Option<String>,
    /// Content snippet with highlighting.
    pub snippet: Option<String>,
    /// Relevance score (0.0 to 1.0).
    pub score: f32,
    /// Content type.
    pub content_type: Option<String>,
}

/// Response from search tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search query that was executed.
    pub query: String,
    /// Number of results found.
    pub total_count: usize,
    /// Results (may be limited).
    pub results: Vec<SearchResult>,
    /// Whether more results are available.
    pub has_more: bool,
    /// Search duration in milliseconds.
    pub duration_ms: u64,
}

/// Response from suggest tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SuggestResponse {
    /// Original prefix/query.
    pub query: String,
    /// Suggestions.
    pub suggestions: Vec<Suggestion>,
}

/// A search suggestion.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Suggestion {
    /// Suggested term/phrase.
    pub text: String,
    /// Score/relevance.
    pub score: f32,
    /// Number of documents matching.
    pub doc_count: Option<usize>,
}

/// Response from index status tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexStatusResponse {
    /// Whether index exists and is usable.
    pub available: bool,
    /// Number of documents in index.
    pub document_count: usize,
    /// When index was last updated.
    pub last_updated: Option<String>,
    /// Index size in bytes.
    pub size_bytes: Option<u64>,
    /// Whether index is stale (content changed).
    pub is_stale: bool,
}
```

### Step 3: Create search tools

Create `fabryk-mcp-fts/src/tools.rs`:

```rust
//! MCP tools for full-text search.

use crate::responses::{
    IndexStatusResponse, SearchResponse, SearchResult, SuggestResponse, Suggestion,
};
use fabryk_core::Result;
use fabryk_fts::{QueryBuilder, SearchBackend, SearchOptions};
use fabryk_mcp::{ToolRegistry, ToolResult};
use mcp_core::ToolInfo;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Instant;

/// Arguments for search tool.
#[derive(Debug, Deserialize)]
pub struct SearchArgs {
    /// Search query string.
    pub query: String,
    /// Optional category filter.
    pub category: Option<String>,
    /// Optional source filter.
    pub source: Option<String>,
    /// Maximum results to return.
    pub limit: Option<usize>,
    /// Offset for pagination.
    pub offset: Option<usize>,
}

/// Arguments for suggest tool.
#[derive(Debug, Deserialize)]
pub struct SuggestArgs {
    /// Prefix to suggest from.
    pub prefix: String,
    /// Maximum suggestions.
    pub limit: Option<usize>,
}

/// MCP tools for full-text search.
pub struct FtsTools<B: SearchBackend> {
    backend: Arc<B>,
}

impl<B: SearchBackend + 'static> FtsTools<B> {
    /// Create new FTS tools.
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Create new FTS tools with shared backend.
    pub fn with_shared(backend: Arc<B>) -> Self {
        Self { backend }
    }
}

impl<B: SearchBackend + 'static> ToolRegistry for FtsTools<B> {
    fn tools(&self) -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "search".to_string(),
                description: "Full-text search across all content".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "category": {
                            "type": "string",
                            "description": "Filter by category"
                        },
                        "source": {
                            "type": "string",
                            "description": "Filter by source"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results (default 10)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Offset for pagination"
                        }
                    },
                    "required": ["query"]
                }),
            },
            ToolInfo {
                name: "search_suggest".to_string(),
                description: "Get search suggestions for a prefix".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "prefix": {
                            "type": "string",
                            "description": "Text to get suggestions for"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum suggestions"
                        }
                    },
                    "required": ["prefix"]
                }),
            },
            ToolInfo {
                name: "search_index_status".to_string(),
                description: "Get search index status".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let backend = Arc::clone(&self.backend);

        match name {
            "search" => Some(Box::pin(async move {
                let args: SearchArgs = serde_json::from_value(args)?;
                let start = Instant::now();

                // Build query
                let mut query_builder = QueryBuilder::new(&args.query);

                if let Some(ref cat) = args.category {
                    query_builder = query_builder.filter_category(cat);
                }

                if let Some(ref source) = args.source {
                    query_builder = query_builder.filter_source(source);
                }

                let options = SearchOptions {
                    limit: args.limit.unwrap_or(10),
                    offset: args.offset.unwrap_or(0),
                    ..Default::default()
                };

                // Execute search
                let search_results = backend.search(query_builder.build(), options).await?;

                // Convert to response
                let results: Vec<SearchResult> = search_results
                    .hits
                    .into_iter()
                    .map(|hit| SearchResult {
                        id: hit.id,
                        title: hit.title,
                        description: hit.description,
                        category: hit.category,
                        source: hit.source,
                        snippet: hit.snippet,
                        score: hit.score,
                        content_type: hit.content_type,
                    })
                    .collect();

                let response = SearchResponse {
                    query: args.query,
                    total_count: search_results.total_count,
                    results,
                    has_more: search_results.has_more,
                    duration_ms: start.elapsed().as_millis() as u64,
                };

                Ok(serde_json::to_value(response)?)
            })),

            "search_suggest" => Some(Box::pin(async move {
                let args: SuggestArgs = serde_json::from_value(args)?;
                let limit = args.limit.unwrap_or(5);

                let suggestions = backend.suggest(&args.prefix, limit).await?;

                let response = SuggestResponse {
                    query: args.prefix,
                    suggestions: suggestions
                        .into_iter()
                        .map(|s| Suggestion {
                            text: s.text,
                            score: s.score,
                            doc_count: s.doc_count,
                        })
                        .collect(),
                };

                Ok(serde_json::to_value(response)?)
            })),

            "search_index_status" => Some(Box::pin(async move {
                let status = backend.index_status().await?;

                let response = IndexStatusResponse {
                    available: status.available,
                    document_count: status.document_count,
                    last_updated: status.last_updated,
                    size_bytes: status.size_bytes,
                    is_stale: status.is_stale,
                };

                Ok(serde_json::to_value(response)?)
            })),

            _ => None,
        }
    }
}
```

### Step 4: Create lib.rs

Create `fabryk-mcp-fts/src/lib.rs`:

```rust
//! Full-text search MCP tools for Fabryk domains.
//!
//! This crate provides MCP tools that delegate to `fabryk-fts` backends.
//!
//! # Tools
//!
//! - `search` - Full-text search with filtering
//! - `search_suggest` - Autocomplete suggestions
//! - `search_index_status` - Index health and status
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::TantivySearch;
//! use fabryk_mcp_fts::FtsTools;
//!
//! let search_backend = TantivySearch::open(index_path)?;
//! let fts_tools = FtsTools::new(search_backend);
//!
//! // Register with composite registry
//! let registry = CompositeRegistry::new().add(fts_tools);
//! ```

pub mod responses;
pub mod tools;

pub use responses::{
    IndexStatusResponse, SearchResponse, SearchResult, SuggestResponse, Suggestion,
};
pub use tools::{FtsTools, SearchArgs, SuggestArgs};
```

### Step 5: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-mcp-fts
cargo test -p fabryk-mcp-fts
cargo clippy -p fabryk-mcp-fts -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-mcp-fts` crate created
- [ ] `FtsTools<B: SearchBackend>` implements ToolRegistry
- [ ] `search` tool with query, category, source, limit, offset
- [ ] `search_suggest` tool for autocomplete
- [ ] `search_index_status` tool for index health
- [ ] Response types match existing music-theory responses
- [ ] All tests pass

## Commit Message

```
feat(mcp): add fabryk-mcp-fts with search tools

Add MCP tools for full-text search:
- FtsTools<B: SearchBackend> implements ToolRegistry
- search: Full-text search with category/source filtering
- search_suggest: Autocomplete suggestions
- search_index_status: Index availability and health

Tools delegate to fabryk-fts SearchBackend implementations.

Phase 5 milestone 5.4 of Fabryk extraction.

Ref: Doc 0011 §4.8 (search MCP tools)
Ref: Doc 0013 Phase 5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
