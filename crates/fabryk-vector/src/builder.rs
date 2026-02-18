//! VectorIndexBuilder for constructing vector search indices.
//!
//! The builder orchestrates content discovery, text extraction, batch
//! embedding, and index population:
//!
//! 1. Discover content files using glob patterns
//! 2. Parse frontmatter and content
//! 3. Call VectorExtractor to produce VectorDocuments
//! 4. Batch embed documents via EmbeddingProvider
//! 5. Insert into VectorBackend
//!
//! # Two-Phase Build
//!
//! - Phase 1: Discover + extract all documents (sync, CPU-bound)
//! - Phase 2: Batch embed + insert (async, may be I/O-bound)

use crate::backend::SimpleVectorBackend;
use crate::embedding::EmbeddingProvider;
use crate::extractor::VectorExtractor;
use crate::types::{BuildError, EmbeddedDocument, VectorDocument, VectorIndexStats};
use fabryk_content::markdown::extract_frontmatter;
use fabryk_core::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

// ============================================================================
// Error handling options
// ============================================================================

/// Options for handling errors during vector index building.
#[derive(Clone, Debug, Default)]
pub enum ErrorHandling {
    /// Stop on first error.
    #[default]
    FailFast,
    /// Continue and collect errors.
    Collect,
    /// Log and skip problematic files.
    Skip,
}

// ============================================================================
// VectorIndexBuilder
// ============================================================================

/// Builder for constructing vector search indices.
///
/// Orchestrates the full pipeline: discover files → extract documents →
/// batch embed → insert into backend.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_vector::{VectorIndexBuilder, MockEmbeddingProvider, MockVectorExtractor};
/// use std::sync::Arc;
///
/// let provider = Arc::new(MockEmbeddingProvider::new(384));
/// let extractor = MockVectorExtractor;
///
/// let (backend, stats) = VectorIndexBuilder::new(extractor)
///     .with_content_path("/data/concepts")
///     .with_embedding_provider(provider)
///     .build()
///     .await?;
/// ```
pub struct VectorIndexBuilder<E: VectorExtractor> {
    extractor: E,
    content_path: Option<PathBuf>,
    provider: Option<Arc<dyn EmbeddingProvider>>,
    error_handling: ErrorHandling,
    batch_size: usize,
}

impl<E: VectorExtractor> VectorIndexBuilder<E> {
    /// Creates a new builder with the given extractor.
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            content_path: None,
            provider: None,
            error_handling: ErrorHandling::default(),
            batch_size: 64,
        }
    }

    /// Sets the content directory path.
    pub fn with_content_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.content_path = Some(path.into());
        self
    }

    /// Sets the embedding provider.
    pub fn with_embedding_provider(mut self, provider: Arc<dyn EmbeddingProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    /// Sets the error handling strategy.
    pub fn with_error_handling(mut self, handling: ErrorHandling) -> Self {
        self.error_handling = handling;
        self
    }

    /// Sets the batch size for embedding operations.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Builds the vector index.
    ///
    /// Returns a `SimpleVectorBackend` populated with embedded documents,
    /// plus build statistics.
    ///
    /// # Phases
    ///
    /// 1. **Discover + Extract**: Find content files, parse frontmatter,
    ///    call extractor to produce `VectorDocument`s.
    /// 2. **Batch Embed + Insert**: Embed documents in batches via the
    ///    provider, then insert into the backend.
    pub async fn build(self) -> Result<(SimpleVectorBackend, VectorIndexStats)> {
        let start = Instant::now();

        let content_path = self
            .content_path
            .as_ref()
            .ok_or_else(|| Error::config("Content path not set. Use with_content_path() first."))?
            .clone();

        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| {
                Error::config("Embedding provider not set. Use with_embedding_provider() first.")
            })?
            .clone();

        // Discover files
        let files = discover_files(&content_path).await?;

        let mut errors: Vec<BuildError> = Vec::new();
        let mut documents: Vec<VectorDocument> = Vec::new();
        let mut files_processed = 0usize;
        let mut files_skipped = 0usize;

        // ================================================================
        // Phase 1: Discover + Extract documents
        // ================================================================
        for file_path in &files {
            match self.extract_file(&content_path, file_path) {
                Ok(doc) => {
                    documents.push(doc);
                }
                Err(e) => {
                    let build_error = BuildError {
                        file: file_path.clone(),
                        message: e.to_string(),
                    };

                    match self.error_handling {
                        ErrorHandling::FailFast => return Err(e),
                        ErrorHandling::Collect => {
                            files_skipped += 1;
                            errors.push(build_error);
                        }
                        ErrorHandling::Skip => {
                            files_skipped += 1;
                            log::warn!("Skipping {}: {}", file_path.display(), build_error.message);
                            errors.push(build_error);
                        }
                    }
                }
            }
            files_processed += 1;
        }

        // ================================================================
        // Phase 2: Batch embed + insert
        // ================================================================
        let mut embedded_documents: Vec<EmbeddedDocument> = Vec::with_capacity(documents.len());

        for chunk in documents.chunks(self.batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|d| d.text.as_str()).collect();
            let embeddings = provider.embed_batch(&texts).await?;

            for (doc, embedding) in chunk.iter().zip(embeddings.into_iter()) {
                embedded_documents.push(EmbeddedDocument::new(doc.clone(), embedding));
            }
        }

        let documents_indexed = embedded_documents.len();
        let embedding_dimension = provider.dimension();

        // Compute content hash
        let content_hash = compute_content_hash(&content_path).await?;

        // Build the backend
        let mut backend = SimpleVectorBackend::new(provider);
        backend.add_documents(embedded_documents);

        let stats = VectorIndexStats {
            documents_indexed,
            files_processed,
            files_skipped,
            embedding_dimension,
            content_hash,
            build_duration_ms: start.elapsed().as_millis() as u64,
            errors,
        };

        Ok((backend, stats))
    }

    /// Extract a single file to a VectorDocument.
    fn extract_file(&self, base_path: &Path, file_path: &Path) -> Result<VectorDocument> {
        let content =
            std::fs::read_to_string(file_path).map_err(|e| Error::io_with_path(e, file_path))?;

        let fm_result = extract_frontmatter(&content)?;

        let frontmatter = fm_result
            .value()
            .cloned()
            .unwrap_or(serde_yaml::Value::Null);
        let body = fm_result.body();

        self.extractor
            .extract_document(base_path, file_path, &frontmatter, body)
    }

    /// Append documents from a content path into an existing backend.
    ///
    /// Unlike `build()`, this does not create a new backend — it adds
    /// embedded documents to the provided one. Use this to index multiple
    /// content directories (potentially with different extractors) into
    /// a single vector search backend.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Build initial index from concept cards
    /// let (mut backend, stats1) = VectorIndexBuilder::new(card_extractor)
    ///     .with_content_path(&cards_path)
    ///     .with_embedding_provider(provider.clone())
    ///     .build()
    ///     .await?;
    ///
    /// // Append source documents with a different extractor
    /// let stats2 = VectorIndexBuilder::new(source_extractor)
    ///     .with_content_path(&sources_path)
    ///     .with_embedding_provider(provider)
    ///     .build_append(&mut backend)
    ///     .await?;
    /// ```
    pub async fn build_append(self, backend: &mut SimpleVectorBackend) -> Result<VectorIndexStats> {
        let start = Instant::now();

        let content_path = self
            .content_path
            .as_ref()
            .ok_or_else(|| Error::config("Content path not set. Use with_content_path() first."))?
            .clone();

        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| {
                Error::config("Embedding provider not set. Use with_embedding_provider() first.")
            })?
            .clone();

        let files = discover_files(&content_path).await?;

        let mut errors: Vec<BuildError> = Vec::new();
        let mut documents: Vec<VectorDocument> = Vec::new();
        let mut files_processed = 0usize;
        let mut files_skipped = 0usize;

        // Phase 1: Discover + Extract
        for file_path in &files {
            match self.extract_file(&content_path, file_path) {
                Ok(doc) => {
                    documents.push(doc);
                }
                Err(e) => {
                    let build_error = BuildError {
                        file: file_path.clone(),
                        message: e.to_string(),
                    };

                    match self.error_handling {
                        ErrorHandling::FailFast => return Err(e),
                        ErrorHandling::Collect => {
                            files_skipped += 1;
                            errors.push(build_error);
                        }
                        ErrorHandling::Skip => {
                            files_skipped += 1;
                            log::warn!("Skipping {}: {}", file_path.display(), build_error.message);
                            errors.push(build_error);
                        }
                    }
                }
            }
            files_processed += 1;
        }

        // Phase 2: Batch embed + insert into existing backend
        let mut embedded_documents: Vec<EmbeddedDocument> = Vec::with_capacity(documents.len());

        for chunk in documents.chunks(self.batch_size) {
            let texts: Vec<&str> = chunk.iter().map(|d| d.text.as_str()).collect();
            let embeddings = provider.embed_batch(&texts).await?;

            for (doc, embedding) in chunk.iter().zip(embeddings.into_iter()) {
                embedded_documents.push(EmbeddedDocument::new(doc.clone(), embedding));
            }
        }

        let documents_indexed = embedded_documents.len();
        let embedding_dimension = provider.dimension();
        let content_hash = compute_content_hash(&content_path).await?;

        backend.add_documents(embedded_documents);

        let stats = VectorIndexStats {
            documents_indexed,
            files_processed,
            files_skipped,
            embedding_dimension,
            content_hash,
            build_duration_ms: start.elapsed().as_millis() as u64,
            errors,
        };

        log::info!(
            "Appended {} vector documents from {} ({} errors)",
            documents_indexed,
            content_path.display(),
            stats.errors.len(),
        );

        Ok(stats)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Discover content files in a directory.
async fn discover_files(base_path: &Path) -> Result<Vec<PathBuf>> {
    use fabryk_core::util::files::{find_all_files, FindOptions};

    let files = find_all_files(base_path, FindOptions::markdown()).await?;
    let paths: Vec<PathBuf> = files.into_iter().map(|f| f.path).collect();

    Ok(paths)
}

/// Compute a content hash for freshness checking.
///
/// Hashes all markdown file contents in the directory using blake3.
pub async fn compute_content_hash(content_path: &Path) -> Result<String> {
    use fabryk_core::util::files::{find_all_files, FindOptions};

    let files = find_all_files(content_path, FindOptions::markdown()).await?;

    let mut hasher = blake3::Hasher::new();
    let mut paths: Vec<PathBuf> = files.into_iter().map(|f| f.path).collect();
    paths.sort(); // Deterministic ordering

    for path in &paths {
        if let Ok(content) = std::fs::read(path) {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(&content);
        }
    }

    Ok(hasher.finalize().to_hex().to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::VectorBackend;
    use crate::embedding::MockEmbeddingProvider;
    use crate::extractor::MockVectorExtractor;
    use tempfile::tempdir;

    async fn setup_test_files() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        let file_a =
            "---\ntitle: \"Concept A\"\ncategory: \"basics\"\n---\n\nContent for concept A.\n";
        let file_b = "---\ntitle: \"Concept B\"\ncategory: \"advanced\"\ntier: \"intermediate\"\n---\n\nContent for concept B.\n";

        std::fs::write(content_dir.join("concept-a.md"), file_a).unwrap();
        std::fs::write(content_dir.join("concept-b.md"), file_b).unwrap();

        (dir, content_dir)
    }

    #[tokio::test]
    async fn test_builder_basic() {
        let (_dir, content_dir) = setup_test_files().await;
        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (backend, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.documents_indexed, 2);
        assert_eq!(stats.embedding_dimension, 8);
        assert!(stats.errors.is_empty());
        assert_eq!(backend.document_count().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_builder_content_hash() {
        let (_dir, content_dir) = setup_test_files().await;
        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (_, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .build()
            .await
            .unwrap();

        assert!(!stats.content_hash.is_empty());
        // Hash should be hex string
        assert!(stats.content_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_builder_content_hash_deterministic() {
        let (_dir, content_dir) = setup_test_files().await;

        let hash1 = compute_content_hash(&content_dir).await.unwrap();
        let hash2 = compute_content_hash(&content_dir).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_builder_content_hash_changes() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        std::fs::write(
            content_dir.join("test.md"),
            "---\ntitle: Test\n---\nOriginal content",
        )
        .unwrap();

        let hash1 = compute_content_hash(&content_dir).await.unwrap();

        std::fs::write(
            content_dir.join("test.md"),
            "---\ntitle: Test\n---\nModified content",
        )
        .unwrap();

        let hash2 = compute_content_hash(&content_dir).await.unwrap();

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_builder_missing_content_path() {
        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let result = VectorIndexBuilder::new(MockVectorExtractor)
            .with_embedding_provider(provider)
            .build()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_missing_provider() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        let result = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .build()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_builder_empty_directory() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("empty");
        std::fs::create_dir(&content_dir).unwrap();
        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (backend, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.documents_indexed, 0);
        assert_eq!(backend.document_count().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_builder_error_handling_collect() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        std::fs::write(
            content_dir.join("valid.md"),
            "---\ntitle: Valid\n---\nContent",
        )
        .unwrap();
        // Create a file that won't parse as valid frontmatter
        std::fs::write(content_dir.join("invalid.md"), "not yaml frontmatter").unwrap();

        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (_, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .with_error_handling(ErrorHandling::Collect)
            .build()
            .await
            .unwrap();

        assert_eq!(stats.files_processed, 2);
        // At least the valid file should be indexed
        assert!(stats.documents_indexed >= 1);
    }

    #[tokio::test]
    async fn test_builder_batch_size() {
        let dir = tempdir().unwrap();
        let content_dir = dir.path().join("content");
        std::fs::create_dir(&content_dir).unwrap();

        // Create more files than batch_size
        for i in 0..5 {
            let content = format!("---\ntitle: \"Doc {i}\"\n---\n\nContent {i}.\n");
            std::fs::write(content_dir.join(format!("doc-{i}.md")), content).unwrap();
        }

        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (backend, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .with_batch_size(2) // Small batches
            .build()
            .await
            .unwrap();

        assert_eq!(stats.documents_indexed, 5);
        assert_eq!(backend.document_count().unwrap(), 5);
    }

    #[tokio::test]
    async fn test_builder_build_duration_tracked() {
        let (_dir, content_dir) = setup_test_files().await;
        let provider = Arc::new(MockEmbeddingProvider::new(8));

        let (_, stats) = VectorIndexBuilder::new(MockVectorExtractor)
            .with_content_path(&content_dir)
            .with_embedding_provider(provider)
            .build()
            .await
            .unwrap();

        // Build should complete in reasonable time
        assert!(stats.build_duration_ms < 10_000);
    }
}
