//! Full-text search backend using Tantivy.
//!
//! This module provides `TantivySearch`, the primary search backend for
//! production use. It executes queries against a Tantivy index with:
//! - BM25 scoring
//! - Multi-field weighted search
//! - Category/source/content_type filtering
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

use tantivy::schema::Value;

use crate::backend::{SearchBackend, SearchParams, SearchResult, SearchResults};
use crate::query::QueryBuilder;
use crate::schema::SearchSchema;
use crate::types::SearchConfig;

/// Tantivy-based full-text search backend.
pub struct TantivySearch {
    /// Retained for ownership — dropping the Index would invalidate the reader.
    #[allow(dead_code)]
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

        let index = Index::open_in_dir(path)
            .map_err(|e| Error::operation(format!("Failed to open index: {e}")))?;

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

    /// Execute a query and return scored document addresses.
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
            let doc: tantivy::TantivyDocument = searcher
                .doc(doc_address)
                .map_err(|e| Error::operation(format!("Failed to retrieve document: {e}")))?;

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
        if let Some(desc) = &description
            && let Some(snippet) = find_snippet_in_text(desc, query, max_len)
        {
            return Some(snippet);
        }

        // Try content
        if let Some(snippet) = find_snippet_in_text(content, query, max_len) {
            return Some(snippet);
        }

        // Fallback to description or content start
        description.clone().or_else(|| {
            if content.len() > max_len {
                let trunc = content.floor_char_boundary(max_len);
                Some(format!("{}...", &content[..trunc]))
            } else if !content.is_empty() {
                Some(content.to_string())
            } else {
                None
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
                    .is_some_and(|s| s.eq_ignore_ascii_case(source))
            });
        }
        if let Some(ref content_types) = params.content_types {
            items.retain(|r| {
                r.content_type
                    .as_ref()
                    .is_some_and(|ct| content_types.iter().any(|t| ct.eq_ignore_ascii_case(t)))
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
fn get_text_field(doc: &tantivy::TantivyDocument, field: tantivy::schema::Field) -> Option<String> {
    doc.get_first(field)
        .and_then(|v| v.as_str())
        .map(String::from)
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

    // Calculate bounds (use char boundaries to avoid panics on multi-byte UTF-8)
    let context = max_len / 4;
    let start = text.floor_char_boundary(pos.saturating_sub(context));
    let end = text.ceil_char_boundary((start + max_len).min(text.len()));

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

    /// Create a temp index with test documents and return the config.
    fn create_test_index() -> (tempfile::TempDir, SearchConfig) {
        let temp_dir = tempfile::tempdir().unwrap();
        let index_path = temp_dir.path().join("index");

        let schema = SearchSchema::build();
        let mut indexer = Indexer::new(&index_path, &schema).unwrap();

        indexer
            .add_document(
                &SearchDocument::builder()
                    .id("test-1")
                    .title("Functional Harmony")
                    .description("Introduction to functional harmony")
                    .content(
                        "Functional harmony describes chord progressions based on tonal function",
                    )
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
                    .content("Voice leading connects chords smoothly using minimal motion")
                    .category("voice-leading")
                    .content_type("concept")
                    .build(),
            )
            .unwrap();

        indexer
            .add_document(
                &SearchDocument::builder()
                    .id("test-3")
                    .title("Cadences")
                    .description("Types of cadences in tonal music")
                    .content("A cadence marks the end of a phrase with harmonic resolution")
                    .category("harmony")
                    .source("Other Source")
                    .content_type("chapter")
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

    #[test]
    fn test_tantivy_search_creation() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config);
        assert!(backend.is_ok());
    }

    #[test]
    fn test_tantivy_search_missing_index_path() {
        let config = SearchConfig::default(); // no index_path
        let result = TantivySearch::new(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_tantivy_search_nonexistent_path() {
        let config = SearchConfig {
            index_path: Some("/nonexistent/path/to/index".to_string()),
            ..Default::default()
        };
        let result = TantivySearch::new(&config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tantivy_search_simple_query() {
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
    async fn test_tantivy_search_wildcard_query() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "*".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        // Should return all documents
        assert_eq!(results.items.len(), 3);
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
        assert!(!results.items.is_empty());
    }

    #[tokio::test]
    async fn test_tantivy_search_with_source_filter() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "*".to_string(),
                source: Some("Test Source".to_string()),
                ..Default::default()
            })
            .await
            .unwrap();

        for item in &results.items {
            assert_eq!(item.source.as_deref(), Some("Test Source"));
        }
    }

    #[tokio::test]
    async fn test_tantivy_search_with_content_type_filter() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "*".to_string(),
                content_types: Some(vec!["chapter".to_string()]),
                ..Default::default()
            })
            .await
            .unwrap();

        for item in &results.items {
            assert_eq!(item.content_type.as_deref(), Some("chapter"));
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

    #[tokio::test]
    async fn test_tantivy_search_relevance_ordering() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "functional harmony".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        // "Functional Harmony" doc should rank first (title match)
        assert!(!results.items.is_empty());
        assert_eq!(results.items[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_tantivy_search_no_results() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "xyznonexistent".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert!(results.items.is_empty());
    }

    #[tokio::test]
    async fn test_tantivy_search_result_fields() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();

        let results = backend
            .search(SearchParams {
                query: "voice leading".to_string(),
                ..Default::default()
            })
            .await
            .unwrap();

        let item = results.items.iter().find(|r| r.id == "test-2").unwrap();
        assert_eq!(item.title, "Voice Leading");
        assert_eq!(item.category, "voice-leading");
        assert!(item.description.is_some());
    }

    #[test]
    fn test_find_snippet_in_text_basic() {
        let text = "This is a test of harmony in music theory";
        let snippet = find_snippet_in_text(text, "harmony", 30);
        assert!(snippet.is_some());
        assert!(snippet.unwrap().contains("harmony"));
    }

    #[test]
    fn test_find_snippet_in_text_not_found() {
        let text = "This is a test";
        let snippet = find_snippet_in_text(text, "nonexistent", 30);
        assert!(snippet.is_none());
    }

    #[test]
    fn test_find_snippet_in_text_empty_query() {
        let text = "Some text";
        assert!(find_snippet_in_text(text, "", 30).is_none());
        assert!(find_snippet_in_text(text, "*", 30).is_none());
    }

    #[test]
    fn test_find_snippet_in_text_case_insensitive() {
        let text = "HARMONY is important in music";
        let snippet = find_snippet_in_text(text, "harmony", 50);
        assert!(snippet.is_some());
    }

    #[test]
    fn test_debug_format() {
        let (_temp, config) = create_test_index();
        let backend = TantivySearch::new(&config).unwrap();
        let debug = format!("{:?}", backend);
        assert!(debug.contains("TantivySearch"));
    }
}
