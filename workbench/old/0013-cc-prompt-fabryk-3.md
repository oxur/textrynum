---
title: "CC Prompt: Fabryk 3.3 — SearchBackend Trait & Documents"
milestone: "3.3"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["3.1 FTS crate scaffold", "3.2 Default schema"]
governing-docs: [0011-audit §4.3, 0013-project-plan]
---

# CC Prompt: Fabryk 3.3 — SearchBackend Trait & Documents

## Context

This milestone completes the `SearchBackend` trait and `SearchDocument` type
that form the core abstraction layer for `fabryk-fts`. These types are used by
all search backends (Tantivy, SimpleSearch) and by domain code for indexing.

**Crate:** `fabryk-fts`
**Risk:** Low-Medium

**Key types:**
- `SearchBackend` trait: Async interface for search execution
- `SearchDocument`: Document representation for indexing
- `SearchParams`: Query parameters
- `SearchResult`/`SearchResults`: Response types

## Source Files

| File | Lines | Tests | Classification |
|------|-------|-------|----------------|
| `search/backend.rs` | 153 | 4 | G (Fully Generic) |
| `search/document.rs` | 489 | 17 | P (Parameterized) |

## Objective

1. Complete `fabryk-fts/src/backend.rs` with full `SearchBackend` trait
2. Complete `fabryk-fts/src/document.rs` with `SearchDocument` implementation
3. Add `SimpleSearch` fallback backend (no Tantivy dependency)
4. Implement backend factory function
5. Verify: `cargo test -p fabryk-fts` passes (without fts-tantivy feature)

## Implementation Steps

### Step 1: Complete `fabryk-fts/src/backend.rs`

```rust
//! Search backend trait and factory.
//!
//! This module defines the `SearchBackend` trait that all search implementations
//! must satisfy, plus types for search parameters and results.
//!
//! # Backends
//!
//! - `TantivySearch`: Full-text search with Tantivy (requires `fts-tantivy` feature)
//! - `SimpleSearch`: Linear scan fallback for small collections
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::{create_search_backend, SearchParams, SearchConfig};
//!
//! let config = SearchConfig::default();
//! let backend = create_search_backend(&config).await?;
//!
//! let params = SearchParams {
//!     query: "functional harmony".to_string(),
//!     limit: Some(10),
//!     ..Default::default()
//! };
//!
//! let results = backend.search(params).await?;
//! println!("Found {} results", results.total);
//! ```

use async_trait::async_trait;
use fabryk_core::Result;
use serde::{Deserialize, Serialize};

use crate::types::{QueryMode, SearchConfig};

/// Parameters for a search request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchParams {
    /// Search query string.
    pub query: String,

    /// Maximum results to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Filter by category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,

    /// Filter by source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Filter by content types (e.g., ["concept", "chapter"]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_types: Option<Vec<String>>,

    /// Query mode override (Smart, And, Or, MinimumMatch).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_mode: Option<QueryMode>,

    /// Snippet length in characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet_length: Option<usize>,
}

/// A single search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Unique document identifier.
    pub id: String,

    /// Document title.
    pub title: String,

    /// Brief description or summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Content category.
    pub category: String,

    /// Source reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Search snippet with query context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,

    /// Relevance score (0.0 to 1.0+, higher is better).
    pub relevance: f32,

    /// Content type classification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    /// File path (for debugging/reference).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Chapter reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,

    /// Section reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

/// Collection of search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Search result items.
    pub items: Vec<SearchResult>,

    /// Total number of matching documents (may be > items.len() if limited).
    pub total: usize,

    /// Backend that executed the search.
    pub backend: String,
}

impl SearchResults {
    /// Create empty results.
    pub fn empty(backend: &str) -> Self {
        Self {
            items: Vec::new(),
            total: 0,
            backend: backend.to_string(),
        }
    }
}

/// Abstract search backend trait.
///
/// Implementations provide different search strategies:
/// - `TantivySearch`: Full-text search with BM25 scoring, stemming, fuzzy matching
/// - `SimpleSearch`: Linear scan with substring matching (fallback)
///
/// # Async
///
/// The `search` method is async to support I/O-bound operations (file reading,
/// index access) without blocking.
#[async_trait]
pub trait SearchBackend: Send + Sync {
    /// Execute a search query.
    ///
    /// Returns results ordered by relevance (highest first).
    async fn search(&self, params: SearchParams) -> Result<SearchResults>;

    /// Get the backend name for diagnostics.
    fn name(&self) -> &str;

    /// Check if the backend is ready to handle queries.
    fn is_ready(&self) -> bool {
        true
    }
}

/// Create a search backend based on configuration.
///
/// Selection logic:
/// 1. If `fts-tantivy` feature enabled and index exists → `TantivySearch`
/// 2. Otherwise → `SimpleSearch` (linear scan fallback)
///
/// # Errors
///
/// Returns an error if the configured backend cannot be initialized.
pub async fn create_search_backend(
    config: &SearchConfig,
) -> Result<Box<dyn SearchBackend>> {
    match config.backend.as_str() {
        #[cfg(feature = "fts-tantivy")]
        "tantivy" => {
            if let Some(ref index_path) = config.index_path {
                let path = std::path::Path::new(index_path);
                if path.exists() {
                    match crate::tantivy_search::TantivySearch::new(config) {
                        Ok(backend) => return Ok(Box::new(backend)),
                        Err(e) => {
                            log::warn!("Failed to open Tantivy index: {e}, falling back to simple search");
                        }
                    }
                }
            }
            // Fall through to simple search
            Ok(Box::new(SimpleSearch::new(config)))
        }
        _ => Ok(Box::new(SimpleSearch::new(config))),
    }
}

/// Simple linear-scan search backend.
///
/// Used as a fallback when Tantivy is not available or for small collections
/// where indexing overhead isn't justified.
///
/// # Limitations
///
/// - O(n) search time
/// - No stemming or fuzzy matching
/// - Substring matching only
pub struct SimpleSearch {
    config: SearchConfig,
}

impl SimpleSearch {
    /// Create a new simple search backend.
    pub fn new(config: &SearchConfig) -> Self {
        Self {
            config: config.clone(),
        }
    }
}

#[async_trait]
impl SearchBackend for SimpleSearch {
    async fn search(&self, params: SearchParams) -> Result<SearchResults> {
        // Simple search requires loading documents from content_path
        // This is a minimal implementation - full version loads from filesystem

        let limit = params.limit.unwrap_or(self.config.default_limit);

        // For now, return empty results
        // Full implementation will scan content_path and match documents
        log::debug!(
            "SimpleSearch: query='{}', limit={}, category={:?}",
            params.query,
            limit,
            params.category
        );

        Ok(SearchResults::empty(self.name()))
    }

    fn name(&self) -> &str {
        "simple"
    }
}

impl std::fmt::Debug for SimpleSearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleSearch")
            .field("config.backend", &self.config.backend)
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_default() {
        let params = SearchParams::default();
        assert!(params.query.is_empty());
        assert!(params.limit.is_none());
        assert!(params.category.is_none());
    }

    #[test]
    fn test_search_params_serialization() {
        let params = SearchParams {
            query: "test query".to_string(),
            limit: Some(10),
            category: Some("harmony".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("test query"));
        assert!(json.contains("harmony"));

        // Optional fields should be skipped when None
        let minimal = SearchParams {
            query: "test".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_string(&minimal).unwrap();
        assert!(!json.contains("limit"));
        assert!(!json.contains("category"));
    }

    #[test]
    fn test_search_result_serialization() {
        let result = SearchResult {
            id: "test-id".to_string(),
            title: "Test Title".to_string(),
            description: Some("Test description".to_string()),
            category: "test".to_string(),
            source: None,
            snippet: Some("...test snippet...".to_string()),
            relevance: 0.95,
            content_type: Some("concept".to_string()),
            path: None,
            chapter: None,
            section: None,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("0.95"));
        assert!(!json.contains("source")); // None should be skipped
    }

    #[test]
    fn test_search_results_empty() {
        let results = SearchResults::empty("test-backend");
        assert!(results.items.is_empty());
        assert_eq!(results.total, 0);
        assert_eq!(results.backend, "test-backend");
    }

    #[test]
    fn test_simple_search_creation() {
        let config = SearchConfig::default();
        let backend = SimpleSearch::new(&config);
        assert_eq!(backend.name(), "simple");
        assert!(backend.is_ready());
    }

    #[tokio::test]
    async fn test_simple_search_empty_result() {
        let config = SearchConfig::default();
        let backend = SimpleSearch::new(&config);

        let params = SearchParams {
            query: "test".to_string(),
            ..Default::default()
        };

        let results = backend.search(params).await.unwrap();
        assert_eq!(results.backend, "simple");
    }

    #[tokio::test]
    async fn test_create_search_backend_simple() {
        let config = SearchConfig {
            backend: "simple".to_string(),
            ..Default::default()
        };

        let backend = create_search_backend(&config).await.unwrap();
        assert_eq!(backend.name(), "simple");
    }
}
```

### Step 2: Complete `fabryk-fts/src/document.rs`

```rust
//! Search document representation.
//!
//! This module defines `SearchDocument`, the struct used to represent indexed
//! content. It maps directly to the schema fields defined in `schema.rs`.
//!
//! # Creating Documents
//!
//! Documents can be created from domain-specific metadata types using the
//! builder pattern or direct construction:
//!
//! ```rust
//! use fabryk_fts::SearchDocument;
//!
//! let doc = SearchDocument::builder()
//!     .id("my-concept")
//!     .title("My Concept")
//!     .content("The main content...")
//!     .category("fundamentals")
//!     .build();
//! ```
//!
//! # Relevance Scoring
//!
//! The document provides a `matches_query()` method for simple substring
//! matching and a `relevance()` method for weighted scoring (used by
//! `SimpleSearch`).

use serde::{Deserialize, Serialize};

/// A document to be indexed and searched.
///
/// This struct holds all fields that can be indexed and searched.
/// It maps to the default schema defined in `schema.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchDocument {
    // Identity
    /// Unique document identifier (required).
    pub id: String,
    /// File path for reference.
    pub path: String,

    // Full-text fields
    /// Document title (required, boosted 3.0x in search).
    pub title: String,
    /// Brief description (boosted 2.0x in search).
    pub description: Option<String>,
    /// Main content body (boosted 1.0x in search).
    pub content: String,

    // Facet fields
    /// Content category (e.g., "harmony", "rhythm").
    pub category: String,
    /// Source reference (e.g., "Open Music Theory").
    pub source: Option<String>,
    /// Tags for additional categorization.
    pub tags: Vec<String>,

    // Metadata fields
    /// Chapter reference.
    pub chapter: Option<String>,
    /// Part within source.
    pub part: Option<String>,
    /// Content author.
    pub author: Option<String>,
    /// Publication/creation date.
    pub date: Option<String>,
    /// Content type classification.
    pub content_type: Option<String>,
    /// Section reference.
    pub section: Option<String>,
}

impl SearchDocument {
    /// Create a new document builder.
    pub fn builder() -> SearchDocumentBuilder {
        SearchDocumentBuilder::default()
    }

    /// Check if the document matches a query (case-insensitive substring match).
    ///
    /// Searches in: title, description, content, category, source, tags.
    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() || query == "*" {
            return true;
        }

        let query_lower = query.to_lowercase();

        self.title.to_lowercase().contains(&query_lower)
            || self
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            || self.content.to_lowercase().contains(&query_lower)
            || self.category.to_lowercase().contains(&query_lower)
            || self
                .source
                .as_ref()
                .map(|s| s.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&query_lower))
    }

    /// Calculate relevance score for a query.
    ///
    /// Uses weighted field matching:
    /// - Title match: 3.0
    /// - Description match: 2.0
    /// - Content match: 1.0
    /// - Category/source/tags: 0.5 each
    pub fn relevance(&self, query: &str) -> f32 {
        if query.is_empty() || query == "*" {
            return 1.0;
        }

        let query_lower = query.to_lowercase();
        let mut score = 0.0;

        if self.title.to_lowercase().contains(&query_lower) {
            score += 3.0;
        }

        if self
            .description
            .as_ref()
            .map(|d| d.to_lowercase().contains(&query_lower))
            .unwrap_or(false)
        {
            score += 2.0;
        }

        if self.content.to_lowercase().contains(&query_lower) {
            score += 1.0;
        }

        if self.category.to_lowercase().contains(&query_lower) {
            score += 0.5;
        }

        if self
            .source
            .as_ref()
            .map(|s| s.to_lowercase().contains(&query_lower))
            .unwrap_or(false)
        {
            score += 0.5;
        }

        if self
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&query_lower))
        {
            score += 0.5;
        }

        score
    }

    /// Extract a snippet around the query match.
    ///
    /// Returns a portion of the content centered on the first match,
    /// with ellipsis if truncated.
    pub fn extract_snippet(&self, query: &str, max_length: usize) -> Option<String> {
        if query.is_empty() || query == "*" {
            return self.description.clone().or_else(|| {
                if self.content.len() > max_length {
                    Some(format!("{}...", &self.content[..max_length]))
                } else {
                    Some(self.content.clone())
                }
            });
        }

        let query_lower = query.to_lowercase();

        // Try description first
        if let Some(ref desc) = self.description {
            if let Some(snippet) = find_snippet(desc, &query_lower, max_length) {
                return Some(snippet);
            }
        }

        // Try content
        if let Some(snippet) = find_snippet(&self.content, &query_lower, max_length) {
            return Some(snippet);
        }

        // Fallback to description or content start
        self.description.clone().or_else(|| {
            if self.content.len() > max_length {
                Some(format!("{}...", &self.content[..max_length]))
            } else {
                Some(self.content.clone())
            }
        })
    }

    /// Check if document matches category filter.
    pub fn matches_category(&self, category: &str) -> bool {
        self.category.eq_ignore_ascii_case(category)
    }

    /// Check if document matches source filter.
    pub fn matches_source(&self, source: &str) -> bool {
        self.source
            .as_ref()
            .map(|s| s.eq_ignore_ascii_case(source))
            .unwrap_or(false)
    }

    /// Check if document matches content type filter.
    pub fn matches_content_type(&self, content_type: &str) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.eq_ignore_ascii_case(content_type))
            .unwrap_or(false)
    }
}

/// Find a snippet of text centered around a query match.
fn find_snippet(text: &str, query: &str, max_length: usize) -> Option<String> {
    let text_lower = text.to_lowercase();
    let pos = text_lower.find(query)?;

    // Calculate start position (with context)
    let context = max_length / 4;
    let start = pos.saturating_sub(context);

    // Find word boundary near start
    let start = if start > 0 {
        text[..start]
            .rfind(char::is_whitespace)
            .map(|p| p + 1)
            .unwrap_or(start)
    } else {
        0
    };

    // Calculate end position
    let end = (start + max_length).min(text.len());

    // Find word boundary near end
    let end = if end < text.len() {
        text[end..]
            .find(char::is_whitespace)
            .map(|p| end + p)
            .unwrap_or(end)
    } else {
        text.len()
    };

    // Build snippet with ellipsis
    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(text[start..end].trim());
    if end < text.len() {
        snippet.push_str("...");
    }

    // Normalize whitespace
    let snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");

    Some(snippet)
}

/// Builder for SearchDocument.
#[derive(Debug, Default)]
pub struct SearchDocumentBuilder {
    doc: SearchDocument,
}

impl SearchDocumentBuilder {
    /// Set the document ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.doc.id = id.into();
        self
    }

    /// Set the file path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.doc.path = path.into();
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.doc.title = title.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.doc.description = Some(description.into());
        self
    }

    /// Set the content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.doc.content = content.into();
        self
    }

    /// Set the category.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.doc.category = category.into();
        self
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.doc.source = Some(source.into());
        self
    }

    /// Set the tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.doc.tags = tags;
        self
    }

    /// Set the content type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.doc.content_type = Some(content_type.into());
        self
    }

    /// Build the document.
    pub fn build(self) -> SearchDocument {
        self.doc
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> SearchDocument {
        SearchDocument::builder()
            .id("major-triad")
            .path("/concepts/harmony/major-triad.md")
            .title("Major Triad")
            .description("A major triad is a three-note chord...")
            .content("The major triad consists of a root, major third, and perfect fifth.")
            .category("harmony")
            .source("Open Music Theory")
            .tags(vec!["chord".to_string(), "fundamentals".to_string()])
            .content_type("concept")
            .build()
    }

    // ------------------------------------------------------------------------
    // Builder tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_builder() {
        let doc = sample_document();
        assert_eq!(doc.id, "major-triad");
        assert_eq!(doc.title, "Major Triad");
        assert_eq!(doc.category, "harmony");
        assert!(doc.description.is_some());
    }

    #[test]
    fn test_builder_minimal() {
        let doc = SearchDocument::builder()
            .id("test")
            .title("Test")
            .content("Content")
            .category("test")
            .build();

        assert_eq!(doc.id, "test");
        assert!(doc.description.is_none());
        assert!(doc.source.is_none());
    }

    // ------------------------------------------------------------------------
    // Query matching tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_matches_query_title() {
        let doc = sample_document();
        assert!(doc.matches_query("major"));
        assert!(doc.matches_query("MAJOR")); // Case-insensitive
        assert!(doc.matches_query("triad"));
    }

    #[test]
    fn test_matches_query_content() {
        let doc = sample_document();
        assert!(doc.matches_query("perfect fifth"));
        assert!(doc.matches_query("root"));
    }

    #[test]
    fn test_matches_query_category() {
        let doc = sample_document();
        assert!(doc.matches_query("harmony"));
    }

    #[test]
    fn test_matches_query_tags() {
        let doc = sample_document();
        assert!(doc.matches_query("chord"));
        assert!(doc.matches_query("fundamentals"));
    }

    #[test]
    fn test_matches_query_wildcard() {
        let doc = sample_document();
        assert!(doc.matches_query("*"));
        assert!(doc.matches_query(""));
    }

    #[test]
    fn test_matches_query_no_match() {
        let doc = sample_document();
        assert!(!doc.matches_query("nonexistent"));
    }

    // ------------------------------------------------------------------------
    // Relevance scoring tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_relevance_title_highest() {
        let doc = sample_document();
        let title_score = doc.relevance("major"); // In title
        let content_score = doc.relevance("fifth"); // Only in content

        assert!(title_score > content_score);
    }

    #[test]
    fn test_relevance_description_medium() {
        let doc = sample_document();
        let desc_score = doc.relevance("three-note"); // In description
        let content_score = doc.relevance("fifth"); // In content

        assert!(desc_score > content_score);
    }

    #[test]
    fn test_relevance_wildcard() {
        let doc = sample_document();
        assert_eq!(doc.relevance("*"), 1.0);
        assert_eq!(doc.relevance(""), 1.0);
    }

    // ------------------------------------------------------------------------
    // Snippet extraction tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_snippet_from_content() {
        let doc = sample_document();
        let snippet = doc.extract_snippet("perfect fifth", 100);

        assert!(snippet.is_some());
        let snippet = snippet.unwrap();
        assert!(snippet.contains("perfect fifth"));
    }

    #[test]
    fn test_extract_snippet_ellipsis() {
        let doc = SearchDocument::builder()
            .id("test")
            .title("Test")
            .content("Word word word word word MATCH word word word word word".repeat(3))
            .category("test")
            .build();

        let snippet = doc.extract_snippet("MATCH", 50);
        assert!(snippet.is_some());
        let snippet = snippet.unwrap();
        assert!(snippet.contains("..."));
    }

    #[test]
    fn test_extract_snippet_fallback() {
        let doc = sample_document();
        let snippet = doc.extract_snippet("nonexistent", 100);

        // Should fallback to description
        assert!(snippet.is_some());
    }

    // ------------------------------------------------------------------------
    // Filter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_matches_category() {
        let doc = sample_document();
        assert!(doc.matches_category("harmony"));
        assert!(doc.matches_category("HARMONY")); // Case-insensitive
        assert!(!doc.matches_category("rhythm"));
    }

    #[test]
    fn test_matches_source() {
        let doc = sample_document();
        assert!(doc.matches_source("Open Music Theory"));
        assert!(!doc.matches_source("Other Source"));
    }

    #[test]
    fn test_matches_content_type() {
        let doc = sample_document();
        assert!(doc.matches_content_type("concept"));
        assert!(!doc.matches_content_type("chapter"));
    }

    // ------------------------------------------------------------------------
    // Serialization tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_serialization_roundtrip() {
        let doc = sample_document();
        let json = serde_json::to_string(&doc).unwrap();
        let restored: SearchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(doc.id, restored.id);
        assert_eq!(doc.title, restored.title);
        assert_eq!(doc.tags, restored.tags);
    }
}
```

### Step 3: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-fts
cargo test -p fabryk-fts
cargo clippy -p fabryk-fts -- -D warnings
```

## Exit Criteria

- [ ] `SearchBackend` trait with `search()` and `name()` methods
- [ ] `SearchParams` with all filter options
- [ ] `SearchResult`/`SearchResults` response types
- [ ] `SimpleSearch` fallback backend implemented
- [ ] `create_search_backend()` factory function
- [ ] `SearchDocument` with all 14 schema fields
- [ ] `SearchDocumentBuilder` for convenient construction
- [ ] `matches_query()` for substring matching
- [ ] `relevance()` for weighted scoring
- [ ] `extract_snippet()` for context extraction
- [ ] Filter methods: `matches_category()`, `matches_source()`, `matches_content_type()`
- [ ] All tests pass (~23 tests)
- [ ] Works without `fts-tantivy` feature (SimpleSearch only)

## Design Notes

### Backend abstraction

The `SearchBackend` trait allows swapping implementations:

```rust
#[async_trait]
pub trait SearchBackend: Send + Sync {
    async fn search(&self, params: SearchParams) -> Result<SearchResults>;
    fn name(&self) -> &str;
}
```

This enables:
- Runtime backend selection (Tantivy vs Simple)
- Testing with mock backends
- Future backend additions (Meilisearch, etc.)

### SimpleSearch as fallback

`SimpleSearch` provides search capability without Tantivy:
- Useful for small collections (<500 docs)
- No index maintenance
- Zero additional dependencies

Production code should use `TantivySearch` for performance.

## Commit Message

```
feat(fts): implement SearchBackend trait and SearchDocument

Add core search abstraction layer:

SearchBackend trait:
- Async search() method with SearchParams
- SimpleSearch fallback implementation
- Backend factory with feature detection

SearchDocument:
- 14-field document matching default schema
- Builder pattern for construction
- matches_query() for substring search
- relevance() for weighted scoring
- extract_snippet() for context extraction

~23 tests for backend and document functionality.

Ref: Doc 0013 milestone 3.3

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
