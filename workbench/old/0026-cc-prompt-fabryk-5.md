---
title: "CC Prompt: Fabryk 5.2 — Content & Source Traits"
milestone: "5.2"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["5.1 MCP core complete"]
governing-docs: [0011-audit, 0012-amendment §2a, 0013-project-plan]
---

# CC Prompt: Fabryk 5.2 — Content & Source Traits

## Context

Per Amendment §2a, the content listing tools (`concepts.rs`, `sources.rs`) were
reclassified from Domain-Specific (D) to Parameterized (P). The pattern of
"list items, get item, filter by category" is universal across domains.

This milestone defines two key traits:

1. `ContentItemProvider` - For listing and retrieving domain content items
2. `SourceProvider` - For accessing source materials (books, papers, etc.)

These traits go in `fabryk-mcp-content`.

## Objective

Create `fabryk-mcp-content` crate with:

1. `ContentItemProvider` trait (Amendment §2a)
2. `SourceProvider` trait (Amendment §2a)
3. Generic MCP tools that delegate to these traits
4. Response types for content operations

## Implementation Steps

### Step 1: Create fabryk-mcp-content crate

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-mcp-content/src
```

Create `fabryk-mcp-content/Cargo.toml`:

```toml
[package]
name = "fabryk-mcp-content"
version = "0.1.0"
edition = "2021"
description = "Content MCP tools for Fabryk domains"
license = "Apache-2.0"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-mcp = { path = "../fabryk-mcp" }

async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
mcp-core = "0.2"
tokio = { version = "1.0", features = ["fs"] }

[dev-dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

### Step 2: Define ContentItemProvider trait

Create `fabryk-mcp-content/src/traits.rs`:

```rust
//! Traits for content item and source material providers.
//!
//! These traits enable domain-agnostic MCP tools for content operations.

use async_trait::async_trait;
use fabryk_core::Result;
use serde::Serialize;
use std::path::PathBuf;

/// Information about a content category.
#[derive(Clone, Debug, Serialize)]
pub struct CategoryInfo {
    /// Category identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Number of items in this category.
    pub count: usize,
    /// Optional description.
    pub description: Option<String>,
}

/// Information about a chapter in a source.
#[derive(Clone, Debug, Serialize)]
pub struct ChapterInfo {
    /// Chapter identifier.
    pub id: String,
    /// Chapter title.
    pub title: String,
    /// Chapter number (if applicable).
    pub number: Option<String>,
    /// Whether content is available.
    pub available: bool,
}

/// Trait for providing domain-specific content item access.
///
/// Each skill implements this to define how its content items
/// are listed, retrieved, and described via MCP tools.
///
/// # Example
///
/// ```rust,ignore
/// struct MusicTheoryContentProvider { /* ... */ }
///
/// #[async_trait]
/// impl ContentItemProvider for MusicTheoryContentProvider {
///     type ItemSummary = ConceptInfo;
///     type ItemDetail = ConceptCard;
///
///     async fn list_items(&self, category: Option<&str>, limit: Option<usize>)
///         -> Result<Vec<Self::ItemSummary>> {
///         // Return concept summaries, optionally filtered
///     }
///
///     async fn get_item(&self, id: &str) -> Result<Self::ItemDetail> {
///         // Return full concept card
///     }
/// }
/// ```
#[async_trait]
pub trait ContentItemProvider: Send + Sync {
    /// Summary type returned when listing items.
    ///
    /// Music theory: ConceptInfo { id, title, category, source }
    /// Math: TheoremInfo { id, name, area, difficulty }
    type ItemSummary: Serialize + Send + Sync;

    /// Detail type returned when getting a single item.
    ///
    /// Music theory: full concept card content
    /// Math: full theorem content with proof sketch
    type ItemDetail: Serialize + Send + Sync;

    /// List all items, optionally filtered by category.
    ///
    /// # Arguments
    ///
    /// * `category` - Optional category filter
    /// * `limit` - Optional maximum number of results
    ///
    /// # Returns
    ///
    /// Vector of item summaries.
    async fn list_items(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Self::ItemSummary>>;

    /// Get a single item by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - Item identifier
    ///
    /// # Returns
    ///
    /// Full item detail, or error if not found.
    async fn get_item(&self, id: &str) -> Result<Self::ItemDetail>;

    /// List available categories with counts.
    ///
    /// # Returns
    ///
    /// Vector of category information.
    async fn list_categories(&self) -> Result<Vec<CategoryInfo>>;

    /// Get total item count.
    async fn count(&self) -> Result<usize> {
        Ok(self.list_items(None, None).await?.len())
    }

    /// Get item count for a specific category.
    async fn count_in_category(&self, category: &str) -> Result<usize> {
        Ok(self.list_items(Some(category), None).await?.len())
    }

    /// Returns the content type name for this provider.
    ///
    /// Used in MCP tool descriptions.
    fn content_type_name(&self) -> &str {
        "item"
    }

    /// Returns the plural content type name.
    fn content_type_name_plural(&self) -> &str {
        "items"
    }
}

/// Trait for providing source material access.
///
/// Sources are reference materials like books, papers, or documentation
/// that the domain knowledge is derived from.
#[async_trait]
pub trait SourceProvider: Send + Sync {
    /// Summary type for source listings.
    type SourceSummary: Serialize + Send + Sync;

    /// List all source materials with availability status.
    async fn list_sources(&self) -> Result<Vec<Self::SourceSummary>>;

    /// Get a specific chapter from a source.
    ///
    /// # Arguments
    ///
    /// * `source_id` - Source identifier
    /// * `chapter` - Chapter identifier
    /// * `section` - Optional section within chapter
    ///
    /// # Returns
    ///
    /// Chapter content as text.
    async fn get_chapter(
        &self,
        source_id: &str,
        chapter: &str,
        section: Option<&str>,
    ) -> Result<String>;

    /// List chapters for a source.
    async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>>;

    /// Get filesystem path to source PDF/EPUB.
    ///
    /// Returns None if source is not available locally.
    async fn get_source_path(&self, source_id: &str) -> Result<Option<PathBuf>>;

    /// Check if a source is available.
    async fn is_available(&self, source_id: &str) -> Result<bool> {
        Ok(self.get_source_path(source_id).await?.is_some())
    }
}
```

### Step 3: Create generic content tools

Create `fabryk-mcp-content/src/tools.rs`:

```rust
//! Generic MCP tools for content operations.

use crate::traits::{ContentItemProvider, SourceProvider};
use fabryk_core::Result;
use fabryk_mcp::{ToolRegistry, ToolResult};
use mcp_core::ToolInfo;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

/// Arguments for list_items tool.
#[derive(Debug, Deserialize)]
pub struct ListItemsArgs {
    pub category: Option<String>,
    pub limit: Option<usize>,
}

/// Arguments for get_item tool.
#[derive(Debug, Deserialize)]
pub struct GetItemArgs {
    pub id: String,
}

/// Arguments for get_chapter tool.
#[derive(Debug, Deserialize)]
pub struct GetChapterArgs {
    pub source_id: String,
    pub chapter: String,
    pub section: Option<String>,
}

/// MCP tools backed by a ContentItemProvider.
pub struct ContentTools<P: ContentItemProvider> {
    provider: Arc<P>,
    tool_prefix: String,
}

impl<P: ContentItemProvider + 'static> ContentTools<P> {
    /// Create new content tools.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            tool_prefix: String::new(),
        }
    }

    /// Set a prefix for tool names.
    ///
    /// E.g., prefix "concepts" gives tools "concepts_list", "concepts_get".
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = prefix.into();
        self
    }

    fn tool_name(&self, base: &str) -> String {
        if self.tool_prefix.is_empty() {
            base.to_string()
        } else {
            format!("{}_{}", self.tool_prefix, base)
        }
    }
}

impl<P: ContentItemProvider + 'static> ToolRegistry for ContentTools<P> {
    fn tools(&self) -> Vec<ToolInfo> {
        let type_name = self.provider.content_type_name();
        let type_plural = self.provider.content_type_name_plural();

        vec![
            ToolInfo {
                name: self.tool_name("list"),
                description: format!("List all {} with optional category filter", type_plural),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Filter by category"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results"
                        }
                    }
                }),
            },
            ToolInfo {
                name: self.tool_name("get"),
                description: format!("Get a specific {} by ID", type_name),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": format!("{} identifier", type_name)
                        }
                    },
                    "required": ["id"]
                }),
            },
            ToolInfo {
                name: self.tool_name("categories"),
                description: format!("List available {} categories", type_name),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name("list") {
            return Some(Box::pin(async move {
                let args: ListItemsArgs = serde_json::from_value(args)?;
                let items = provider
                    .list_items(args.category.as_deref(), args.limit)
                    .await?;
                Ok(serde_json::to_value(items)?)
            }));
        }

        if name == self.tool_name("get") {
            return Some(Box::pin(async move {
                let args: GetItemArgs = serde_json::from_value(args)?;
                let item = provider.get_item(&args.id).await?;
                Ok(serde_json::to_value(item)?)
            }));
        }

        if name == self.tool_name("categories") {
            return Some(Box::pin(async move {
                let categories = provider.list_categories().await?;
                Ok(serde_json::to_value(categories)?)
            }));
        }

        None
    }
}

/// MCP tools backed by a SourceProvider.
pub struct SourceTools<P: SourceProvider> {
    provider: Arc<P>,
}

impl<P: SourceProvider + 'static> SourceTools<P> {
    /// Create new source tools.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }
}

impl<P: SourceProvider + 'static> ToolRegistry for SourceTools<P> {
    fn tools(&self) -> Vec<ToolInfo> {
        vec![
            ToolInfo {
                name: "sources_list".to_string(),
                description: "List all source materials".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            ToolInfo {
                name: "sources_chapters".to_string(),
                description: "List chapters in a source".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        }
                    },
                    "required": ["source_id"]
                }),
            },
            ToolInfo {
                name: "sources_get_chapter".to_string(),
                description: "Get content from a source chapter".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "source_id": { "type": "string" },
                        "chapter": { "type": "string" },
                        "section": { "type": "string" }
                    },
                    "required": ["source_id", "chapter"]
                }),
            },
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        match name {
            "sources_list" => Some(Box::pin(async move {
                let sources = provider.list_sources().await?;
                Ok(serde_json::to_value(sources)?)
            })),
            "sources_chapters" => Some(Box::pin(async move {
                let args: serde_json::Value = args;
                let source_id = args["source_id"].as_str().unwrap_or("");
                let chapters = provider.list_chapters(source_id).await?;
                Ok(serde_json::to_value(chapters)?)
            })),
            "sources_get_chapter" => Some(Box::pin(async move {
                let args: GetChapterArgs = serde_json::from_value(args)?;
                let content = provider
                    .get_chapter(&args.source_id, &args.chapter, args.section.as_deref())
                    .await?;
                Ok(json!({ "content": content }))
            })),
            _ => None,
        }
    }
}
```

### Step 4: Create lib.rs

Create `fabryk-mcp-content/src/lib.rs`:

```rust
//! Content MCP tools for Fabryk domains.
//!
//! This crate provides traits and tools for content item operations:
//!
//! - `ContentItemProvider` - List and retrieve domain content items
//! - `SourceProvider` - Access source materials (books, papers)
//! - `ContentTools<P>` - MCP tools backed by a provider
//! - `SourceTools<P>` - MCP tools for source access
//!
//! # Example
//!
//! ```rust,ignore
//! // Implement the trait for your domain
//! struct MyContentProvider { /* ... */ }
//!
//! impl ContentItemProvider for MyContentProvider {
//!     type ItemSummary = MyItemSummary;
//!     type ItemDetail = MyItemDetail;
//!     // ... implement methods
//! }
//!
//! // Create MCP tools
//! let tools = ContentTools::new(provider).with_prefix("concepts");
//! ```

pub mod tools;
pub mod traits;

pub use tools::{ContentTools, SourceTools};
pub use traits::{CategoryInfo, ChapterInfo, ContentItemProvider, SourceProvider};
```

### Step 5: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-mcp-content
cargo test -p fabryk-mcp-content
cargo clippy -p fabryk-mcp-content -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-mcp-content` crate created
- [ ] `ContentItemProvider` trait with `list_items()`, `get_item()`, `list_categories()`
- [ ] `SourceProvider` trait with `list_sources()`, `get_chapter()`, `list_chapters()`
- [ ] `ContentTools<P>` implements ToolRegistry
- [ ] `SourceTools<P>` implements ToolRegistry
- [ ] Tools support optional category filtering and limits
- [ ] All tests pass

## Commit Message

```
feat(mcp): add fabryk-mcp-content with ContentItemProvider and SourceProvider

Add content MCP tools per Amendment §2a:
- ContentItemProvider trait for domain content access
- SourceProvider trait for source material access
- ContentTools<P> generates list/get/categories tools
- SourceTools<P> generates source listing/chapter tools

These traits enable domain-agnostic content MCP tools.
Each domain implements the traits with its specific types.

Phase 5 milestone 5.2 of Fabryk extraction.

Ref: Doc 0012 §2a (content tool reclassification)
Ref: Doc 0013 Phase 5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
