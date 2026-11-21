---
title: "CC Prompt: Fabryk 3.5 — Query Builder & TantivySearch"
milestone: "3.5"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["3.1-3.4 complete"]
governing-docs: [0011-audit §4.3, 0013-project-plan]
---

# CC Prompt: Fabryk 3.5 — Query Builder & TantivySearch

## Context

This milestone extracts the query building and search execution components from
music-theory. These are the highest-complexity modules in the FTS system,
providing weighted multi-field search with Tantivy.

**Crate:** `fabryk-fts`
**Feature:** `fts-tantivy` (required)
**Risk:** Medium-High (Tantivy query API complexity)

## Source Files

| File | Lines | Tests | Classification |
|------|-------|-------|----------------|
| `search/query.rs` | 729 | 24 | P (Parameterized) |
| `search/tantivy_search.rs` | 767 | 9 | P (Parameterized) |

Both modules are parameterized over the schema — they work with any
`SearchSchema` instance.

## Objective

1. Extract `query.rs` to `fabryk-fts/src/query.rs`
2. Extract `tantivy_search.rs` to `fabryk-fts/src/tantivy_search.rs`
3. Integrate with previously extracted modules
4. Update backend factory to return `TantivySearch`
5. Verify: Full search round-trip works

## Implementation Steps

### Step 1: Create `fabryk-fts/src/query.rs`

```rust
//! Query building with weighted multi-field search.
//!
//! This module provides `QueryBuilder` for constructing Tantivy queries with:
//! - Field-specific boost weights
//! - Phrase query support
//! - Query mode selection (AND, OR, Smart)
//! - Optional fuzzy matching
//! - Stopword filtering
//!
//! # Query Modes
//!
//! - **Smart** (default): AND for 1-2 terms, OR with minimum match for 3+
//! - **And**: All terms must match
//! - **Or**: Any term can match
//! - **MinimumMatch**: At least N terms must match
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::{QueryBuilder, SearchSchema, SearchConfig};
//!
//! let schema = SearchSchema::build();
//! let config = SearchConfig::default();
//! let builder = QueryBuilder::new(&schema, &config);
//!
//! let query = builder.build_query("functional harmony")?;
//! ```

use fabryk_core::{Error, Result};
use tantivy::query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::tokenizer::TextAnalyzer;
use tantivy::Term;

use crate::schema::SearchSchema;
use crate::stopwords::StopwordFilter;
use crate::types::{QueryMode, SearchConfig};

/// Query builder for constructing Tantivy queries.
pub struct QueryBuilder<'a> {
    schema: &'a SearchSchema,
    config: &'a SearchConfig,
    stopword_filter: StopwordFilter,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder.
    pub fn new(schema: &'a SearchSchema, config: &'a SearchConfig) -> Self {
        let stopword_filter = StopwordFilter::new(config);
        Self {
            schema,
            config,
            stopword_filter,
        }
    }

    /// Build a query from a search string.
    ///
    /// Handles:
    /// - Quoted phrases ("exact phrase")
    /// - Multiple terms with configurable AND/OR logic
    /// - Field-specific boost weights
    /// - Optional fuzzy matching
    pub fn build_query(&self, query_str: &str) -> Result<Box<dyn Query>> {
        let query_str = query_str.trim();

        // Handle empty/wildcard queries
        if query_str.is_empty() || query_str == "*" {
            return Ok(Box::new(tantivy::query::AllQuery));
        }

        // Filter stopwords
        let filtered = self.stopword_filter.filter(query_str);

        // Extract phrases
        let (phrases, remaining) = parse_phrases(&filtered);

        // Parse remaining terms
        let terms: Vec<&str> = remaining.split_whitespace().collect();

        // Build subqueries for each field with boost
        let mut field_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for (field, boost) in self.schema.full_text_fields() {
            let mut term_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

            // Add phrase queries
            for phrase in &phrases {
                if let Some(pq) = self.create_phrase_query(field, phrase) {
                    term_queries.push((Occur::Should, Box::new(BoostQuery::new(pq, boost))));
                }
            }

            // Add term queries
            let occur = self.determine_occur_mode(&terms);
            for term in &terms {
                let tq = self.create_term_query(field, term);
                term_queries.push((occur, Box::new(BoostQuery::new(tq, boost))));
            }

            if !term_queries.is_empty() {
                let field_query = BooleanQuery::new(term_queries);
                field_queries.push((Occur::Should, Box::new(field_query)));
            }
        }

        if field_queries.is_empty() {
            return Ok(Box::new(tantivy::query::AllQuery));
        }

        Ok(Box::new(BooleanQuery::new(field_queries)))
    }

    /// Determine the occur mode based on config and term count.
    fn determine_occur_mode(&self, terms: &[&str]) -> Occur {
        match self.config.query_mode {
            QueryMode::And => Occur::Must,
            QueryMode::Or => Occur::Should,
            QueryMode::Smart => {
                if terms.len() <= 2 {
                    Occur::Must // AND for short queries
                } else {
                    Occur::Should // OR for longer queries
                }
            }
            QueryMode::MinimumMatch => Occur::Should,
        }
    }

    /// Create a phrase query for exact matching.
    fn create_phrase_query(
        &self,
        field: tantivy::schema::Field,
        phrase: &str,
    ) -> Option<Box<dyn Query>> {
        let terms: Vec<Term> = phrase
            .split_whitespace()
            .map(|word| Term::from_field_text(field, &word.to_lowercase()))
            .collect();

        if terms.is_empty() {
            return None;
        }

        if terms.len() == 1 {
            return Some(Box::new(TermQuery::new(
                terms[0].clone(),
                IndexRecordOption::WithFreqs,
            )));
        }

        Some(Box::new(tantivy::query::PhraseQuery::new(terms)))
    }

    /// Create a term query (optionally fuzzy).
    fn create_term_query(&self, field: tantivy::schema::Field, term: &str) -> Box<dyn Query> {
        let term_obj = Term::from_field_text(field, &term.to_lowercase());

        if self.config.fuzzy_enabled && term.len() >= 4 {
            Box::new(tantivy::query::FuzzyTermQuery::new(
                term_obj,
                self.config.fuzzy_distance,
                true, // transposition
            ))
        } else {
            Box::new(TermQuery::new(term_obj, IndexRecordOption::WithFreqs))
        }
    }
}

/// Parse quoted phrases from a query string.
///
/// Returns (phrases, remaining text without quotes).
fn parse_phrases(query: &str) -> (Vec<String>, String) {
    let mut phrases = Vec::new();
    let mut remaining = query.to_string();

    // Simple regex-free phrase extraction
    while let Some(start) = remaining.find('"') {
        if let Some(end) = remaining[start + 1..].find('"') {
            let phrase = remaining[start + 1..start + 1 + end].trim().to_string();
            if !phrase.is_empty() {
                phrases.push(phrase);
            }
            remaining = format!(
                "{}{}",
                &remaining[..start],
                &remaining[start + end + 2..]
            );
        } else {
            break;
        }
    }

    (phrases, remaining)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_builder() -> QueryBuilder<'static> {
        static SCHEMA: std::sync::OnceLock<SearchSchema> = std::sync::OnceLock::new();
        static CONFIG: std::sync::OnceLock<SearchConfig> = std::sync::OnceLock::new();

        let schema = SCHEMA.get_or_init(SearchSchema::build);
        let config = CONFIG.get_or_init(SearchConfig::default);

        QueryBuilder::new(schema, config)
    }

    #[test]
    fn test_build_simple_query() {
        let builder = test_builder();
        let query = builder.build_query("harmony");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_multi_term_query() {
        let builder = test_builder();
        let query = builder.build_query("functional harmony");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_phrase_query() {
        let builder = test_builder();
        let query = builder.build_query("\"functional harmony\"");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_empty_query() {
        let builder = test_builder();
        let query = builder.build_query("");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_wildcard_query() {
        let builder = test_builder();
        let query = builder.build_query("*");
        assert!(query.is_ok());
    }

    #[test]
    fn test_parse_phrases_single() {
        let (phrases, remaining) = parse_phrases("\"exact phrase\" other");
        assert_eq!(phrases, vec!["exact phrase"]);
        assert!(remaining.contains("other"));
    }

    #[test]
    fn test_parse_phrases_multiple() {
        let (phrases, remaining) = parse_phrases("\"one\" word \"two\"");
        assert_eq!(phrases.len(), 2);
        assert!(remaining.contains("word"));
    }

    #[test]
    fn test_parse_phrases_none() {
        let (phrases, remaining) = parse_phrases("no phrases here");
        assert!(phrases.is_empty());
        assert_eq!(remaining.trim(), "no phrases here");
    }

    #[test]
    fn test_determine_occur_mode_smart() {
        let builder = test_builder();

        // Short query: AND
        let occur = builder.determine_occur_mode(&["one", "two"]);
        assert_eq!(occur, Occur::Must);

        // Long query: OR
        let occur = builder.determine_occur_mode(&["one", "two", "three"]);
        assert_eq!(occur, Occur::Should);
    }
}
```

### Step 2: Create `fabryk-fts/src/tantivy_search.rs`

```rust
//! Full-text search backend using Tantivy.
//!
//! This module provides `TantivySearch`, the primary search backend for
//! production use. It executes queries against a Tantivy index with:
//! - BM25 scoring
//! - Multi-field weighted search
//! - Facet filtering
//! - Snippet generation
//!
//! # Usage
//!
//! ```rust,ignore
//! use fabryk_fts::{TantivySearch, SearchConfig, SearchParams};
//!
//! let config = SearchConfig {
//!     index_path: Some("/path/to/index".to_string()),
//!     ..Default::default()
//! };
//!
//! let backend = TantivySearch::new(&config)?;
//! let results = backend.search(SearchParams {
//!     query: "functional harmony".to_string(),
//!     limit: Some(10),
//!     ..Default::default()
//! }).await?;
//! ```

use std::path::Path;

use async_trait::async_trait;
use fabryk_core::{Error, Result};
use tantivy::collector::TopDocs;
use tantivy::query::Query;
use tantivy::{Index, IndexReader, ReloadPolicy};

use crate::backend::{SearchBackend, SearchParams, SearchResult, SearchResults};
use crate::query::QueryBuilder;
use crate::schema::SearchSchema;
use crate::types::SearchConfig;

/// Tantivy-based full-text search backend.
pub struct TantivySearch {
    index: Index,
    reader: IndexReader,
    schema: SearchSchema,
    config: SearchConfig,
}

impl TantivySearch {
    /// Create a new Tantivy search backend.
    ///
    /// Opens an existing index at the configured path.
    pub fn new(config: &SearchConfig) -> Result<Self> {
        let index_path = config
            .index_path
            .as_ref()
            .ok_or_else(|| Error::config("index_path is required for TantivySearch"))?;

        let path = Path::new(index_path);
        if !path.exists() {
            return Err(Error::not_found("Index", index_path));
        }

        let index = Index::open_in_dir(path).map_err(|e| {
            Error::operation(format!("Failed to open index: {e}"))
        })?;

        let schema = SearchSchema::build();
        SearchSchema::register_tokenizers(&index);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .map_err(|e| Error::operation(format!("Failed to create reader: {e}")))?;

        Ok(Self {
            index,
            reader,
            schema,
            config: config.clone(),
        })
    }

    /// Execute a query and return results.
    fn execute_query(
        &self,
        query: &dyn Query,
        limit: usize,
    ) -> Result<Vec<(f32, tantivy::DocAddress)>> {
        let searcher = self.reader.searcher();
        let top_docs = searcher
            .search(query, &TopDocs::with_limit(limit))
            .map_err(|e| Error::operation(format!("Search failed: {e}")))?;

        Ok(top_docs)
    }

    /// Convert Tantivy documents to SearchResults.
    fn convert_results(
        &self,
        docs: Vec<(f32, tantivy::DocAddress)>,
        query_str: &str,
    ) -> Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let mut results = Vec::with_capacity(docs.len());

        for (score, doc_address) in docs {
            let doc = searcher.doc(doc_address).map_err(|e| {
                Error::operation(format!("Failed to retrieve document: {e}"))
            })?;

            let id = get_text_field(&doc, self.schema.id).unwrap_or_default();
            let title = get_text_field(&doc, self.schema.title).unwrap_or_default();
            let description = get_text_field(&doc, self.schema.description);
            let content = get_text_field(&doc, self.schema.content).unwrap_or_default();
            let category = get_text_field(&doc, self.schema.category).unwrap_or_default();
            let source = get_text_field(&doc, self.schema.source);
            let path = get_text_field(&doc, self.schema.path);
            let content_type = get_text_field(&doc, self.schema.content_type);
            let chapter = get_text_field(&doc, self.schema.chapter);
            let section = get_text_field(&doc, self.schema.section);

            // Generate snippet
            let snippet = self.generate_snippet(query_str, &description, &content);

            results.push(SearchResult {
                id,
                title,
                description,
                category,
                source,
                snippet,
                relevance: score,
                content_type,
                path,
                chapter,
                section,
            });
        }

        Ok(results)
    }

    /// Generate a search snippet from description or content.
    fn generate_snippet(
        &self,
        query: &str,
        description: &Option<String>,
        content: &str,
    ) -> Option<String> {
        let max_len = self.config.snippet_length;

        // Try description first
        if let Some(ref desc) = description {
            if let Some(snippet) = find_snippet_in_text(desc, query, max_len) {
                return Some(snippet);
            }
        }

        // Try content
        if let Some(snippet) = find_snippet_in_text(content, query, max_len) {
            return Some(snippet);
        }

        // Fallback to description or content start
        description.clone().or_else(|| {
            if content.len() > max_len {
                Some(format!("{}...", &content[..max_len]))
            } else {
                Some(content.to_string())
            }
        })
    }
}

#[async_trait]
impl SearchBackend for TantivySearch {
    async fn search(&self, params: SearchParams) -> Result<SearchResults> {
        let limit = params.limit.unwrap_or(self.config.default_limit);

        // Build query
        let builder = QueryBuilder::new(&self.schema, &self.config);
        let query = builder.build_query(&params.query)?;

        // Execute
        let docs = self.execute_query(query.as_ref(), limit)?;
        let total = docs.len();

        // Convert to results
        let mut items = self.convert_results(docs, &params.query)?;

        // Apply filters
        if let Some(ref category) = params.category {
            items.retain(|r| r.category.eq_ignore_ascii_case(category));
        }
        if let Some(ref source) = params.source {
            items.retain(|r| {
                r.source
                    .as_ref()
                    .map(|s| s.eq_ignore_ascii_case(source))
                    .unwrap_or(false)
            });
        }
        if let Some(ref content_types) = params.content_types {
            items.retain(|r| {
                r.content_type
                    .as_ref()
                    .map(|ct| content_types.iter().any(|t| ct.eq_ignore_ascii_case(t)))
                    .unwrap_or(false)
            });
        }

        Ok(SearchResults {
            items,
            total,
            backend: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "tantivy"
    }
}

/// Get text field value from a Tantivy document.
fn get_text_field(
    doc: &tantivy::TantivyDocument,
    field: tantivy::schema::Field,
) -> Option<String> {
    doc.get_first(field).and_then(|v| v.as_str()).map(String::from)
}

/// Find a snippet of text containing the query.
fn find_snippet_in_text(text: &str, query: &str, max_len: usize) -> Option<String> {
    if query.is_empty() || query == "*" {
        return None;
    }

    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();

    // Try to find query in text
    let pos = text_lower.find(&query_lower)?;

    // Calculate bounds
    let context = max_len / 4;
    let start = pos.saturating_sub(context);
    let end = (start + max_len).min(text.len());

    // Find word boundaries
    let start = if start > 0 {
        text[..start]
            .rfind(char::is_whitespace)
            .map(|p| p + 1)
            .unwrap_or(start)
    } else {
        0
    };

    let end = if end < text.len() {
        text[end..]
            .find(char::is_whitespace)
            .map(|p| end + p)
            .unwrap_or(end)
    } else {
        text.len()
    };

    // Build snippet
    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(text[start..end].trim());
    if end < text.len() {
        snippet.push_str("...");
    }

    Some(snippet)
}

impl std::fmt::Debug for TantivySearch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TantivySearch")
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
    use crate::document::SearchDocument;
    use crate::indexer::Indexer;

    fn create_test_index() -> (tempfile::TempDir, SearchConfig) {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("index");

        // Create index with test documents
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new(&index_path, &schema).unwrap();

        indexer
            .add_document(
                &SearchDocument::builder()
                    .id("test-1")
                    .title("Functional Harmony")
                    .description("Introduction to functional harmony")
                    .content("Functional harmony describes chord progressions")
                    .category("harmony")
                    .source("Test Source")
                    .content_type("concept")
                    .build(),
            )
            .unwrap();

        indexer
            .add_document(
                &SearchDocument::builder()
                    .id("test-2")
                    .title("Voice Leading")
                    .description("Voice leading techniques")
                    .content("Voice leading connects chords smoothly")
                    .category("voice-leading")
                    .build(),
            )
            .unwrap();

        indexer.commit().unwrap();

        let config = SearchConfig {
            index_path: Some(index_path.to_string_lossy().to_string()),
            ..Default::default()
        };

        (temp_dir, config)
    }

    #[tokio::test]
    async fn test_tantivy_search_creation() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config);
        assert!(backend.is_ok());
    }

    #[tokio::test]
    async fn test_tantivy_search_query() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "harmony".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(!results.items.is_empty());
        assert_eq!(results.backend, "tantivy");
    }

    #[tokio::test]
    async fn test_tantivy_search_with_category_filter() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "*".to_string(),
                category: Some("harmony".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        for item in &results.items {
            assert_eq!(item.category.to_lowercase(), "harmony");
        }
    }

    #[tokio::test]
    async fn test_tantivy_search_with_limit() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "*".to_string(),
                limit: Some(1),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(results.items.len() <= 1);
    }

    #[test]
    fn test_find_snippet() {
        let text = "This is a test of harmony in music theory";
        let snippet = find_snippet_in_text(text, "harmony", 30);
        assert!(snippet.is_some());
        assert!(snippet.unwrap().contains("harmony"));
    }
}
```

### Step 3: Update `fabryk-fts/src/backend.rs`

Update the factory to use `TantivySearch`:

```rust
// In create_search_backend():
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
    Ok(Box::new(SimpleSearch::new(config)))
}
```

### Step 4: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-fts --features fts-tantivy
cargo test -p fabryk-fts --features fts-tantivy
cargo clippy -p fabryk-fts --features fts-tantivy -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-fts/src/query.rs` with `QueryBuilder`
- [ ] `fabryk-fts/src/tantivy_search.rs` with `TantivySearch`
- [ ] Query building with weighted fields
- [ ] Phrase query support
- [ ] Query mode selection (Smart, And, Or)
- [ ] Snippet generation
- [ ] Category/source/content_type filtering
- [ ] Backend factory returns `TantivySearch` when available
- [ ] Full search round-trip works
- [ ] All tests pass (~40+ tests total)

## Design Notes

### Query building strategy

The query builder creates a nested boolean query:

```
BooleanQuery(Should) [
    FieldQuery(title, boost=3.0) [
        TermQuery("harmony")
        TermQuery("functional")
    ],
    FieldQuery(description, boost=2.0) [...],
    FieldQuery(content, boost=1.0) [...],
]
```

This ensures matches in title rank higher than matches in content.

### Snippet generation phases

1. Try to find query in description
2. Try to find query in content
3. Fallback to description start
4. Fallback to content start

This prioritizes contextual snippets over generic ones.

## Commit Message

```
feat(fts): add query builder and TantivySearch backend

QueryBuilder:
- Weighted multi-field queries (title 3x, desc 2x, content 1x)
- Phrase query support ("exact match")
- Query mode selection (Smart/And/Or)
- Stopword filtering integration

TantivySearch:
- Full Tantivy search backend implementation
- BM25 scoring with reader refresh
- Category/source/content_type filtering
- Multi-phase snippet generation

Completes Phase 3 extraction of FTS infrastructure.
~40+ tests for query and search functionality.

Ref: Doc 0013 milestone 3.5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
