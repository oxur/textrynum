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
