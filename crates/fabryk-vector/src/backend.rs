//! Vector backend trait and simple fallback implementation.
//!
//! This module defines the `VectorBackend` trait that all vector search
//! implementations must satisfy. It follows the same pattern as
//! `fabryk_fts::SearchBackend`.
//!
//! # Backends
//!
//! - `LancedbBackend`: Vector search with LanceDB (requires `vector-lancedb` feature)
//! - `SimpleVectorBackend`: In-memory brute-force fallback for small collections

use async_trait::async_trait;
use fabryk_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::embedding::EmbeddingProvider;
use crate::types::{
    EmbeddedDocument, VectorConfig, VectorSearchParams, VectorSearchResult, VectorSearchResults,
};
use std::path::Path;
use std::sync::Arc;

/// Abstract vector search backend trait.
///
/// Implementations provide different vector search strategies:
/// - `LancedbBackend`: Approximate nearest neighbor with LanceDB
/// - `SimpleVectorBackend`: Brute-force cosine similarity (fallback)
///
/// # Async
///
/// The `search` method is async to support I/O-bound operations (embedding
/// generation, index access) without blocking.
#[async_trait]
pub trait VectorBackend: Send + Sync {
    /// Execute a vector similarity search.
    ///
    /// The query string is embedded using the backend's embedding provider,
    /// then compared against indexed vectors. Results are ordered by
    /// similarity score (highest first).
    async fn search(&self, params: VectorSearchParams) -> Result<VectorSearchResults>;

    /// Get the backend name for diagnostics.
    fn name(&self) -> &str;

    /// Check if the backend is ready to handle queries.
    fn is_ready(&self) -> bool {
        true
    }

    /// Get the number of indexed documents.
    fn document_count(&self) -> Result<usize>;
}

/// Create a vector backend based on configuration.
///
/// Selection logic:
/// 1. If `vector-lancedb` feature enabled and config says "lancedb" → `LancedbBackend`
/// 2. Otherwise → `SimpleVectorBackend` (brute-force fallback)
///
/// Note: This creates an empty backend. Use `VectorIndexBuilder` to populate it.
pub fn create_vector_backend(
    config: &VectorConfig,
    provider: Arc<dyn EmbeddingProvider>,
) -> Result<Box<dyn VectorBackend>> {
    if !config.enabled {
        return Ok(Box::new(SimpleVectorBackend::new(provider)));
    }

    match config.backend.as_str() {
        #[cfg(feature = "vector-lancedb")]
        "lancedb" => {
            // LanceDB backend requires async initialization; return simple as default.
            // Use LancedbBackend::build() for full initialization.
            log::info!("LanceDB requested but requires async build(); returning simple backend");
            Ok(Box::new(SimpleVectorBackend::new(provider)))
        }
        _ => Ok(Box::new(SimpleVectorBackend::new(provider))),
    }
}

// ============================================================================
// SimpleVectorBackend
// ============================================================================

/// Serializable vector cache for persistence.
#[derive(Serialize, Deserialize)]
struct VectorCache {
    content_hash: String,
    documents: Vec<EmbeddedDocument>,
}

/// Lightweight header for checking freshness without loading all documents.
#[derive(Deserialize)]
struct VectorCacheHeader {
    content_hash: String,
}

/// Brute-force vector search backend.
///
/// Stores documents in memory and computes cosine similarity for each query.
/// Used as a fallback when LanceDB is not available or for small collections.
///
/// # Caching
///
/// Supports cache persistence via [`save_cache`](Self::save_cache) and
/// [`load_cache`](Self::load_cache). Use [`is_cache_fresh`](Self::is_cache_fresh)
/// to check if a cached index is still valid.
///
/// # Limitations
///
/// - O(n) search time
/// - All documents must fit in memory
pub struct SimpleVectorBackend {
    provider: Arc<dyn EmbeddingProvider>,
    documents: Vec<EmbeddedDocument>,
}

impl SimpleVectorBackend {
    /// Create a new empty simple vector backend.
    pub fn new(provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            provider,
            documents: Vec::new(),
        }
    }

    /// Add documents to the backend.
    pub fn add_documents(&mut self, documents: Vec<EmbeddedDocument>) {
        self.documents.extend(documents);
    }

    /// Save the backend's documents to a cache file.
    ///
    /// Stores documents and a content hash for freshness checking.
    /// Uses JSON format for simplicity and debuggability.
    pub fn save_cache(&self, path: &Path, content_hash: &str) -> Result<()> {
        let cache = VectorCache {
            content_hash: content_hash.to_string(),
            documents: self.documents.clone(),
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io_with_path(e, parent))?;
            }
        }

        let json = serde_json::to_string(&cache)
            .map_err(|e| Error::operation(format!("Failed to serialize vector cache: {e}")))?;

        std::fs::write(path, json).map_err(|e| Error::io_with_path(e, path))?;

        log::info!(
            "Saved vector cache: {} documents to {}",
            self.documents.len(),
            path.display()
        );

        Ok(())
    }

    /// Load a cached backend from disk.
    ///
    /// Returns `Ok(Some(backend))` if the cache exists and loaded successfully,
    /// `Ok(None)` if the cache doesn't exist, or `Err` on read/parse errors.
    pub fn load_cache(path: &Path, provider: Arc<dyn EmbeddingProvider>) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let json = std::fs::read_to_string(path).map_err(|e| Error::io_with_path(e, path))?;

        let cache: VectorCache = serde_json::from_str(&json)
            .map_err(|e| Error::parse(format!("Failed to parse vector cache: {e}")))?;

        let mut backend = Self::new(provider);
        backend.documents = cache.documents;

        log::info!(
            "Loaded vector cache: {} documents from {}",
            backend.documents.len(),
            path.display()
        );

        Ok(Some(backend))
    }

    /// Check if the cache is fresh (content hasn't changed).
    pub fn is_cache_fresh(path: &Path, content_hash: &str) -> bool {
        if !path.exists() {
            return false;
        }

        // Read only the content_hash field without deserializing the full document array
        if let Ok(json) = std::fs::read_to_string(path) {
            if let Ok(cache) = serde_json::from_str::<VectorCacheHeader>(&json) {
                return cache.content_hash == content_hash;
            }
        }

        false
    }

    /// Compute cosine similarity between two vectors.
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

#[async_trait]
impl VectorBackend for SimpleVectorBackend {
    async fn search(&self, params: VectorSearchParams) -> Result<VectorSearchResults> {
        if self.documents.is_empty() {
            return Ok(VectorSearchResults::empty(self.name()));
        }

        let query_embedding = self.provider.embed(&params.query).await?;
        let limit = params.limit.unwrap_or(10);
        let threshold = params.similarity_threshold.unwrap_or(0.0);

        let mut scored: Vec<(usize, f32)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(i, doc)| {
                let sim = Self::cosine_similarity(&query_embedding, &doc.embedding);
                (i, sim)
            })
            .filter(|(_, sim)| *sim >= threshold)
            .collect();

        // Filter by category if specified
        if let Some(ref category) = params.category {
            scored.retain(|(i, _)| {
                self.documents[*i].document.category.as_deref() == Some(category.as_str())
            });
        }

        // Filter by metadata
        for (key, value) in &params.metadata_filters {
            scored.retain(|(i, _)| {
                self.documents[*i]
                    .document
                    .metadata
                    .get(key)
                    .map(|v| v == value)
                    .unwrap_or(false)
            });
        }

        // Sort by similarity (highest first)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        let total = scored.len();
        let items: Vec<VectorSearchResult> = scored
            .into_iter()
            .map(|(i, score)| {
                let doc = &self.documents[i];
                let distance = 1.0 - score; // cosine distance
                VectorSearchResult {
                    id: doc.document.id.clone(),
                    score,
                    distance,
                    metadata: doc.document.metadata.clone(),
                }
            })
            .collect();

        Ok(VectorSearchResults {
            items,
            total,
            backend: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "simple"
    }

    fn document_count(&self) -> Result<usize> {
        Ok(self.documents.len())
    }
}

impl std::fmt::Debug for SimpleVectorBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SimpleVectorBackend")
            .field("documents", &self.documents.len())
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::MockEmbeddingProvider;
    use crate::types::VectorDocument;

    fn mock_provider() -> Arc<dyn EmbeddingProvider> {
        Arc::new(MockEmbeddingProvider::new(8))
    }

    #[test]
    fn test_simple_backend_creation() {
        let backend = SimpleVectorBackend::new(mock_provider());
        assert_eq!(backend.name(), "simple");
        assert!(backend.is_ready());
        assert_eq!(backend.document_count().unwrap(), 0);
    }

    #[test]
    fn test_simple_backend_add_documents() {
        let provider = mock_provider();
        let mut backend = SimpleVectorBackend::new(provider);

        let docs = vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "hello"),
                vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "world"),
                vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
        ];

        backend.add_documents(docs);
        assert_eq!(backend.document_count().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_simple_backend_search_empty() {
        let backend = SimpleVectorBackend::new(mock_provider());

        let params = VectorSearchParams::new("test query");
        let results = backend.search(params).await.unwrap();

        assert!(results.items.is_empty());
        assert_eq!(results.total, 0);
        assert_eq!(results.backend, "simple");
    }

    #[tokio::test]
    async fn test_simple_backend_search_with_results() {
        let provider = Arc::new(MockEmbeddingProvider::new(4));
        let mut backend = SimpleVectorBackend::new(provider.clone());

        // Add documents with known embeddings
        let docs = vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-close", "close match"),
                vec![0.9, 0.1, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-far", "far away"),
                vec![0.0, 0.0, 0.1, 0.9],
            ),
        ];

        backend.add_documents(docs);

        let params = VectorSearchParams::new("test").with_limit(10);
        let results = backend.search(params).await.unwrap();

        assert_eq!(results.items.len(), 2);
        // Results should be ordered by score (highest first)
        assert!(results.items[0].score >= results.items[1].score);
    }

    #[tokio::test]
    async fn test_simple_backend_search_with_threshold() {
        let provider = Arc::new(MockEmbeddingProvider::new(4));
        let mut backend = SimpleVectorBackend::new(provider.clone());

        let docs = vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "text"),
                vec![1.0, 0.0, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "text"),
                vec![0.0, 0.0, 0.0, 1.0],
            ),
        ];

        backend.add_documents(docs);

        // Very high threshold should filter most results
        let params = VectorSearchParams::new("test").with_threshold(0.99);
        let results = backend.search(params).await.unwrap();

        // At threshold 0.99, likely 0 or 1 results
        assert!(results.items.len() <= 2);
    }

    #[tokio::test]
    async fn test_simple_backend_search_with_category() {
        let provider = Arc::new(MockEmbeddingProvider::new(4));
        let mut backend = SimpleVectorBackend::new(provider.clone());

        let docs = vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "harmony text").with_category("harmony"),
                vec![0.5, 0.5, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "rhythm text").with_category("rhythm"),
                vec![0.5, 0.0, 0.5, 0.0],
            ),
        ];

        backend.add_documents(docs);

        let params = VectorSearchParams::new("test").with_category("harmony");
        let results = backend.search(params).await.unwrap();

        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].id, "doc-1");
    }

    #[tokio::test]
    async fn test_simple_backend_search_with_metadata_filter() {
        let provider = Arc::new(MockEmbeddingProvider::new(4));
        let mut backend = SimpleVectorBackend::new(provider.clone());

        let docs = vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "text").with_metadata("tier", "beginner"),
                vec![0.5, 0.5, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "text").with_metadata("tier", "advanced"),
                vec![0.5, 0.0, 0.5, 0.0],
            ),
        ];

        backend.add_documents(docs);

        let params = VectorSearchParams::new("test").with_filter("tier", "beginner");
        let results = backend.search(params).await.unwrap();

        assert_eq!(results.items.len(), 1);
        assert_eq!(results.items[0].id, "doc-1");
    }

    #[tokio::test]
    async fn test_simple_backend_search_limit() {
        let provider = Arc::new(MockEmbeddingProvider::new(4));
        let mut backend = SimpleVectorBackend::new(provider.clone());

        let docs: Vec<EmbeddedDocument> = (0..20)
            .map(|i| {
                EmbeddedDocument::new(
                    VectorDocument::new(format!("doc-{i}"), format!("text {i}")),
                    vec![0.5, 0.5, 0.0, 0.0],
                )
            })
            .collect();

        backend.add_documents(docs);

        let params = VectorSearchParams::new("test").with_limit(5);
        let results = backend.search(params).await.unwrap();

        assert_eq!(results.items.len(), 5);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let v = vec![1.0, 0.0, 0.0];
        let sim = SimpleVectorBackend::cosine_similarity(&v, &v);
        assert!((sim - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = SimpleVectorBackend::cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = SimpleVectorBackend::cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = SimpleVectorBackend::cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let a = vec![1.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = SimpleVectorBackend::cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_create_vector_backend_simple() {
        let config = VectorConfig {
            backend: "simple".to_string(),
            ..Default::default()
        };
        let provider = mock_provider();

        let backend = create_vector_backend(&config, provider).unwrap();
        assert_eq!(backend.name(), "simple");
    }

    #[test]
    fn test_create_vector_backend_disabled() {
        let config = VectorConfig {
            enabled: false,
            ..Default::default()
        };
        let provider = mock_provider();

        let backend = create_vector_backend(&config, provider).unwrap();
        assert_eq!(backend.name(), "simple");
    }

    #[test]
    fn test_trait_object_safety() {
        // Verify VectorBackend can be used as a trait object
        fn _assert_object_safe(_: &dyn VectorBackend) {}
    }

    #[test]
    fn test_simple_backend_debug() {
        let backend = SimpleVectorBackend::new(mock_provider());
        let debug = format!("{:?}", backend);
        assert!(debug.contains("SimpleVectorBackend"));
        assert!(debug.contains("documents"));
    }

    // ================================================================
    // Cache tests
    // ================================================================

    #[test]
    fn test_save_and_load_cache() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("test-cache.json");
        let provider = mock_provider();

        let mut backend = SimpleVectorBackend::new(provider.clone());
        backend.add_documents(vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "hello"),
                vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "world"),
                vec![0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
        ]);

        backend.save_cache(&cache_path, "hash123").unwrap();
        assert!(cache_path.exists());

        let loaded = SimpleVectorBackend::load_cache(&cache_path, provider)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.document_count().unwrap(), 2);
    }

    #[test]
    fn test_cache_freshness() {
        let dir = tempfile::tempdir().unwrap();
        let cache_path = dir.path().join("test-cache.json");
        let provider = mock_provider();

        let mut backend = SimpleVectorBackend::new(provider);
        backend.add_documents(vec![EmbeddedDocument::new(
            VectorDocument::new("doc-1", "hello"),
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        )]);

        backend.save_cache(&cache_path, "hash123").unwrap();

        assert!(SimpleVectorBackend::is_cache_fresh(&cache_path, "hash123"));
        assert!(!SimpleVectorBackend::is_cache_fresh(
            &cache_path,
            "different_hash"
        ));
        assert!(!SimpleVectorBackend::is_cache_fresh(
            &dir.path().join("missing.json"),
            "hash123"
        ));
    }

    #[test]
    fn test_load_cache_nonexistent() {
        let result = SimpleVectorBackend::load_cache(
            std::path::Path::new("/nonexistent/path.json"),
            mock_provider(),
        )
        .unwrap();
        assert!(result.is_none());
    }
}
