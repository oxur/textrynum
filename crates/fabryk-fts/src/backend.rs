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

use crate::concept_card_extractor::ConceptCardDocumentExtractor;
use crate::document::SearchDocument;
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

    /// Domain-specific extra filters passed through from MCP tool arguments.
    /// Search backends can use these for post-filtering or query refinement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_filters: Option<serde_json::Value>,
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
pub async fn create_search_backend(config: &SearchConfig) -> Result<Box<dyn SearchBackend>> {
    match config.backend.as_str() {
        #[cfg(feature = "fts-tantivy")]
        "tantivy" => {
            if let Some(ref index_path) = config.index_path {
                let path = std::path::Path::new(index_path);
                if path.exists() {
                    match crate::tantivy_search::TantivySearch::new(config) {
                        Ok(backend) => return Ok(Box::new(backend)),
                        Err(e) => {
                            log::warn!(
                                "Failed to open Tantivy index: {e}, falling back to simple search"
                            );
                        }
                    }
                }
            }
            // Fall through to simple search
            Ok(Box::new(SimpleSearch::with_default_extractor(config)))
        }
        _ => Ok(Box::new(SimpleSearch::with_default_extractor(config))),
    }
}

/// Trait for extracting [`SearchDocument`]s from file content.
///
/// This is the non-feature-gated version of document extraction, usable by
/// [`SimpleSearch`] without requiring the `fts-tantivy` feature.
///
/// # Example
///
/// ```rust,ignore
/// struct MyExtractor;
///
/// impl SimpleDocumentExtractor for MyExtractor {
///     fn extract(&self, path: &std::path::Path, content: &str) -> Option<SearchDocument> {
///         Some(SearchDocument::builder()
///             .id("doc")
///             .title("Title")
///             .content(content)
///             .category("general")
///             .build())
///     }
/// }
/// ```
pub trait SimpleDocumentExtractor: Send + Sync {
    /// Extract a [`SearchDocument`] from file content.
    ///
    /// Returns `None` if the content cannot be parsed.
    fn extract(&self, path: &std::path::Path, content: &str) -> Option<SearchDocument>;
}

/// Blanket implementation so [`ConceptCardDocumentExtractor`] satisfies the trait
/// via its inherent `extract()` method.
impl SimpleDocumentExtractor for ConceptCardDocumentExtractor {
    fn extract(&self, path: &std::path::Path, content: &str) -> Option<SearchDocument> {
        ConceptCardDocumentExtractor::extract(self, path, content)
    }
}

/// Simple linear-scan search backend.
///
/// Scans markdown files in the configured `content_path`, extracts documents
/// via a [`SimpleDocumentExtractor`], and performs substring matching with
/// weighted relevance scoring.
///
/// # Limitations
///
/// - O(n) search time per query
/// - No stemming or fuzzy matching
/// - Substring matching only
pub struct SimpleSearch {
    config: SearchConfig,
    extractor: Box<dyn SimpleDocumentExtractor>,
}

impl SimpleSearch {
    /// Create a new simple search backend with a custom extractor.
    pub fn new(config: &SearchConfig, extractor: Box<dyn SimpleDocumentExtractor>) -> Self {
        Self {
            config: config.clone(),
            extractor,
        }
    }

    /// Create a new simple search backend with the default
    /// [`ConceptCardDocumentExtractor`].
    pub fn with_default_extractor(config: &SearchConfig) -> Self {
        Self::new(config, Box::new(ConceptCardDocumentExtractor::new()))
    }

    /// Apply optional filters (category, source, content_types) to a document.
    ///
    /// Returns `true` if the document passes all active filters.
    fn passes_filters(&self, doc: &SearchDocument, params: &SearchParams) -> bool {
        if let Some(ref cat) = params.category {
            if !doc.matches_category(cat) {
                return false;
            }
        }
        if let Some(ref src) = params.source {
            if !doc.matches_source(src) {
                return false;
            }
        }
        if let Some(ref types) = params.content_types {
            if !types.iter().any(|ct| doc.matches_content_type(ct)) {
                return false;
            }
        }
        true
    }
}

#[async_trait]
impl SearchBackend for SimpleSearch {
    async fn search(&self, params: SearchParams) -> Result<SearchResults> {
        let limit = params.limit.unwrap_or(self.config.default_limit);
        let snippet_length = params.snippet_length.unwrap_or(self.config.snippet_length);

        log::debug!(
            "SimpleSearch: query='{}', limit={}, category={:?}",
            params.query,
            limit,
            params.category
        );

        // If no content_path configured, return empty results.
        let content_path = match self.config.content_path {
            Some(ref p) => std::path::PathBuf::from(p),
            None => return Ok(SearchResults::empty(self.name())),
        };

        if !content_path.exists() {
            log::warn!(
                "SimpleSearch: content_path does not exist: {}",
                content_path.display()
            );
            return Ok(SearchResults::empty(self.name()));
        }

        // Discover all markdown files.
        let files = fabryk_core::util::files::find_all_files(
            &content_path,
            fabryk_core::util::files::FindOptions::markdown(),
        )
        .await?;

        let mut results: Vec<SearchResult> = Vec::new();

        for file_info in &files {
            // Read file content.
            let content = match fabryk_core::util::files::read_file(&file_info.path).await {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("SimpleSearch: failed to read {:?}: {}", file_info.path, e);
                    continue;
                }
            };

            // Extract document.
            let doc = match self.extractor.extract(&file_info.path, &content) {
                Some(d) => d,
                None => continue,
            };

            // Check query match.
            if !doc.matches_query(&params.query) {
                continue;
            }

            // Apply filters.
            if !self.passes_filters(&doc, &params) {
                continue;
            }

            // Score and build result.
            let relevance = doc.relevance(&params.query);
            let snippet = doc.extract_snippet(&params.query, snippet_length);

            results.push(SearchResult {
                id: doc.id.clone(),
                title: doc.title.clone(),
                description: doc.description.clone(),
                category: doc.category.clone(),
                source: doc.source.clone(),
                snippet,
                relevance,
                content_type: doc.content_type.clone(),
                path: Some(doc.path.clone()),
                chapter: doc.chapter.clone(),
                section: doc.section.clone(),
            });
        }

        // Sort by relevance descending.
        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = results.len();
        results.truncate(limit);

        Ok(SearchResults {
            items: results,
            total,
            backend: self.name().to_string(),
        })
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
        let backend = SimpleSearch::with_default_extractor(&config);
        assert_eq!(backend.name(), "simple");
        assert!(backend.is_ready());
    }

    #[test]
    fn test_simple_search_creation_custom_extractor() {
        let config = SearchConfig::default();
        let extractor = Box::new(ConceptCardDocumentExtractor::new());
        let backend = SimpleSearch::new(&config, extractor);
        assert_eq!(backend.name(), "simple");
    }

    #[tokio::test]
    async fn test_simple_search_empty_result_no_content_path() {
        let config = SearchConfig::default();
        let backend = SimpleSearch::with_default_extractor(&config);

        let params = SearchParams {
            query: "test".to_string(),
            ..Default::default()
        };

        let results = backend.search(params).await.unwrap();
        assert_eq!(results.backend, "simple");
        assert_eq!(results.total, 0);
    }

    #[tokio::test]
    async fn test_simple_search_with_real_files() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create category directory structure.
        let harmony_dir = root.join("harmony");
        std::fs::create_dir_all(&harmony_dir).unwrap();

        std::fs::write(
            harmony_dir.join("major-triad.md"),
            "---\ntitle: Major Triad\ncategory: harmony\n---\n\nThe major triad consists of a root, major third, and perfect fifth.",
        )
        .unwrap();

        std::fs::write(
            harmony_dir.join("minor-scale.md"),
            "---\ntitle: Minor Scale\ncategory: harmony\n---\n\nThe natural minor scale follows a specific pattern of whole and half steps.",
        )
        .unwrap();

        let rhythm_dir = root.join("rhythm");
        std::fs::create_dir_all(&rhythm_dir).unwrap();

        std::fs::write(
            rhythm_dir.join("time-signatures.md"),
            "---\ntitle: Time Signatures\ncategory: rhythm\n---\n\nTime signatures indicate the meter of a piece.",
        )
        .unwrap();

        let config = SearchConfig {
            backend: "simple".to_string(),
            content_path: Some(root.to_string_lossy().to_string()),
            ..Default::default()
        };
        let backend = SimpleSearch::with_default_extractor(&config);

        // Search for "triad" -- should find major-triad.
        let params = SearchParams {
            query: "triad".to_string(),
            ..Default::default()
        };
        let results = backend.search(params).await.unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].id, "major-triad");
        assert!(results.items[0].relevance > 0.0);

        // Search with category filter.
        let params = SearchParams {
            query: "".to_string(),
            category: Some("rhythm".to_string()),
            ..Default::default()
        };
        let results = backend.search(params).await.unwrap();
        assert_eq!(results.total, 1);
        assert_eq!(results.items[0].id, "time-signatures");

        // Wildcard search returns all 3.
        let params = SearchParams {
            query: "*".to_string(),
            ..Default::default()
        };
        let results = backend.search(params).await.unwrap();
        assert_eq!(results.total, 3);
    }

    #[tokio::test]
    async fn test_simple_search_limit() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path();

        for i in 0..5 {
            std::fs::write(
                root.join(format!("doc-{i}.md")),
                format!("---\ntitle: Doc {i}\ncategory: test\n---\n\nContent about harmony {i}."),
            )
            .unwrap();
        }

        let config = SearchConfig {
            backend: "simple".to_string(),
            content_path: Some(root.to_string_lossy().to_string()),
            ..Default::default()
        };
        let backend = SimpleSearch::with_default_extractor(&config);

        let params = SearchParams {
            query: "*".to_string(),
            limit: Some(2),
            ..Default::default()
        };
        let results = backend.search(params).await.unwrap();
        assert_eq!(results.total, 5);
        assert_eq!(results.items.len(), 2);
    }

    #[tokio::test]
    async fn test_simple_search_relevance_ordering() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let root = temp_dir.path();

        // "triad" appears only in content.
        std::fs::write(
            root.join("content-only.md"),
            "---\ntitle: Something Else\ncategory: test\n---\n\nThe triad is important.",
        )
        .unwrap();

        // "triad" appears in both title and content.
        std::fs::write(
            root.join("title-and-content.md"),
            "---\ntitle: Triad Fundamentals\ncategory: test\n---\n\nA triad has three notes.",
        )
        .unwrap();

        let config = SearchConfig {
            backend: "simple".to_string(),
            content_path: Some(root.to_string_lossy().to_string()),
            ..Default::default()
        };
        let backend = SimpleSearch::with_default_extractor(&config);

        let params = SearchParams {
            query: "triad".to_string(),
            ..Default::default()
        };
        let results = backend.search(params).await.unwrap();
        assert_eq!(results.total, 2);
        // Title + content match should score higher.
        assert_eq!(results.items[0].id, "title-and-content");
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

    #[test]
    fn test_search_params_extra_filters_default_none() {
        let params = SearchParams::default();
        assert!(params.extra_filters.is_none());
    }

    #[test]
    fn test_search_params_with_extra_filters() {
        let params = SearchParams {
            query: "chord voicings".to_string(),
            extra_filters: Some(serde_json::json!({"tier": "advanced", "min_confidence": 0.8})),
            ..Default::default()
        };
        assert!(params.extra_filters.is_some());
        let filters = params.extra_filters.unwrap();
        assert_eq!(filters["tier"], "advanced");
        assert_eq!(filters["min_confidence"], 0.8);
    }

    #[test]
    fn test_search_params_extra_filters_serialization() {
        let params = SearchParams {
            query: "test".to_string(),
            extra_filters: Some(serde_json::json!({"tier": "basic"})),
            ..Default::default()
        };
        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("extra_filters"));
        assert!(json.contains("basic"));

        // Round-trip
        let deserialized: SearchParams = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.extra_filters.unwrap()["tier"], "basic");
    }

    #[test]
    fn test_search_params_extra_filters_absent_in_json() {
        // Deserializing JSON without extra_filters should yield None.
        let json = r#"{"query": "test"}"#;
        let params: SearchParams = serde_json::from_str(json).unwrap();
        assert!(params.extra_filters.is_none());
    }
}
