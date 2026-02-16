//! Batch indexing orchestration.
//!
//! This module provides `IndexBuilder` for batch indexing of content directories,
//! and the `DocumentExtractor` trait for domain-specific document extraction.
//! This module is only available with the `fts-tantivy` feature.
//!
//! # Usage
//!
//! ```rust,ignore
//! use fabryk_fts::{IndexBuilder, SearchDocument, SearchSchema};
//!
//! // Define a custom extractor for your domain
//! struct MyExtractor;
//!
//! impl DocumentExtractor for MyExtractor {
//!     fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
//!         // Parse your content format and create a SearchDocument
//!         Some(SearchDocument::builder()
//!             .id(path.to_string_lossy())
//!             .title("My Title")
//!             .content(content)
//!             .category("general")
//!             .build())
//!     }
//!
//!     fn supported_extensions(&self) -> &[&str] {
//!         &["md", "txt"]
//!     }
//! }
//!
//! // Build the index
//! let builder = IndexBuilder::new()
//!     .with_extractor(Box::new(MyExtractor));
//!
//! let stats = builder.build(&content_path, &index_path).await?;
//! println!("Indexed {} documents", stats.documents_indexed);
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use async_walkdir::WalkDir;
use fabryk_core::{Error, Result};
use futures::StreamExt;

use crate::document::SearchDocument;
use crate::freshness::IndexMetadata;
use crate::indexer::Indexer;
use crate::schema::SearchSchema;

/// Statistics about an indexing operation.
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Number of documents successfully indexed.
    pub documents_indexed: usize,
    /// Number of files processed.
    pub files_processed: usize,
    /// Number of files skipped (unsupported or extraction failed).
    pub files_skipped: usize,
    /// Number of errors encountered.
    pub errors: usize,
    /// Total bytes of content processed.
    pub bytes_processed: usize,
    /// Content hash for freshness checking.
    pub content_hash: String,
}

/// Trait for extracting `SearchDocument`s from raw content.
///
/// Implement this trait to customize how your domain-specific content
/// is converted into searchable documents.
///
/// # Example
///
/// ```rust,ignore
/// struct MarkdownExtractor;
///
/// impl DocumentExtractor for MarkdownExtractor {
///     fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
///         // Parse frontmatter, extract title, etc.
///         Some(SearchDocument::builder()
///             .id(path.file_stem()?.to_string_lossy())
///             .title(extract_title(content))
///             .content(content)
///             .category("general")
///             .build())
///     }
///
///     fn supported_extensions(&self) -> &[&str] {
///         &["md", "markdown"]
///     }
/// }
/// ```
pub trait DocumentExtractor: Send + Sync {
    /// Extract a `SearchDocument` from file content.
    ///
    /// Returns `None` if the content cannot be extracted (invalid format,
    /// missing required fields, etc.).
    fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument>;

    /// List of file extensions this extractor supports (without dots).
    ///
    /// Example: `["md", "txt", "rst"]`
    fn supported_extensions(&self) -> &[&str];

    /// Check if this extractor supports the given file extension.
    fn supports_extension(&self, ext: &str) -> bool {
        self.supported_extensions()
            .iter()
            .any(|e| e.eq_ignore_ascii_case(ext))
    }
}

/// Default document extractor that creates minimal documents.
///
/// This extractor:
/// - Uses the file stem as the document ID
/// - Uses the file name as the title
/// - Uses the entire content as the document body
/// - Sets category to "general"
///
/// For production use, implement a custom `DocumentExtractor`.
pub struct DefaultExtractor {
    extensions: Vec<&'static str>,
}

impl Default for DefaultExtractor {
    fn default() -> Self {
        Self {
            extensions: vec!["md", "txt", "rst"],
        }
    }
}

impl DefaultExtractor {
    /// Create a default extractor with custom extensions.
    pub fn with_extensions(extensions: Vec<&'static str>) -> Self {
        Self { extensions }
    }
}

impl DocumentExtractor for DefaultExtractor {
    fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
        let id = path.file_stem()?.to_string_lossy().to_string();
        let title = path.file_name()?.to_string_lossy().to_string();

        Some(
            SearchDocument::builder()
                .id(&id)
                .path(path.to_string_lossy())
                .title(&title)
                .content(content)
                .category("general")
                .build(),
        )
    }

    fn supported_extensions(&self) -> &[&str] {
        &self.extensions
    }
}

/// Batch index builder.
///
/// Orchestrates indexing of content directories, including:
/// - File discovery with extension filtering
/// - Parallel content extraction
/// - Batched index commits
/// - Progress reporting
/// - Freshness tracking
pub struct IndexBuilder {
    extractor: Box<dyn DocumentExtractor>,
    batch_size: usize,
    skip_freshness_check: bool,
}

impl IndexBuilder {
    /// Create a new index builder with default extractor.
    pub fn new() -> Self {
        Self {
            extractor: Box::new(DefaultExtractor::default()),
            batch_size: 100,
            skip_freshness_check: false,
        }
    }

    /// Set the document extractor.
    pub fn with_extractor(mut self, extractor: Box<dyn DocumentExtractor>) -> Self {
        self.extractor = extractor;
        self
    }

    /// Set the batch size for commits.
    ///
    /// Documents are committed in batches to balance memory usage and I/O.
    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }

    /// Skip freshness check and always rebuild.
    pub fn force_rebuild(mut self) -> Self {
        self.skip_freshness_check = true;
        self
    }

    /// Build an index from the given content path.
    ///
    /// This method:
    /// 1. Computes content hash for freshness checking
    /// 2. Checks if index is already fresh (unless forced)
    /// 3. Discovers all supported files
    /// 4. Extracts documents using the configured extractor
    /// 5. Indexes documents in batches
    /// 6. Saves metadata for freshness checking
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The content path doesn't exist
    /// - Index creation fails
    /// - File reading fails (logged, but may not stop indexing)
    pub async fn build(&self, content_path: &Path, index_path: &Path) -> Result<IndexStats> {
        // Validate content path
        if !content_path.exists() {
            return Err(Error::not_found(
                content_path.to_string_lossy(),
                "content directory",
            ));
        }

        // Compute content hash
        let content_hash = IndexMetadata::compute_hash(content_path).await?;

        // Check freshness (unless forced)
        if !self.skip_freshness_check {
            if let Ok(Some(metadata)) = IndexMetadata::load(index_path) {
                if metadata.content_hash == content_hash {
                    log::info!("Index is fresh, skipping rebuild");
                    return Ok(IndexStats {
                        documents_indexed: metadata.document_count,
                        content_hash,
                        ..Default::default()
                    });
                }
            }
        }

        log::info!("Building index from {:?}", content_path);

        // Create schema and indexer
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new(index_path, &schema)?;

        // Clear existing documents
        indexer.clear()?;

        // Collect supported extensions
        let extensions: HashSet<_> = self
            .extractor
            .supported_extensions()
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Find all files
        let files = find_files_with_extensions(content_path, &extensions).await?;

        let mut stats = IndexStats {
            content_hash: content_hash.clone(),
            ..Default::default()
        };

        let mut batch_count = 0;

        // Process files
        for file_path in files {
            stats.files_processed += 1;

            // Read file content
            let content = match tokio::fs::read_to_string(&file_path).await {
                Ok(c) => c,
                Err(e) => {
                    log::warn!("Failed to read {:?}: {}", file_path, e);
                    stats.errors += 1;
                    continue;
                }
            };

            stats.bytes_processed += content.len();

            // Extract document
            let doc = match self.extractor.extract(&file_path, &content) {
                Some(d) => d,
                None => {
                    log::debug!("Skipped {:?} (extraction returned None)", file_path);
                    stats.files_skipped += 1;
                    continue;
                }
            };

            // Add to index
            if let Err(e) = indexer.add_document(&doc) {
                log::warn!("Failed to index {:?}: {}", file_path, e);
                stats.errors += 1;
                continue;
            }

            stats.documents_indexed += 1;
            batch_count += 1;

            // Commit batch
            if batch_count >= self.batch_size {
                indexer.commit()?;
                batch_count = 0;
            }
        }

        // Final commit
        if batch_count > 0 {
            indexer.commit()?;
        }

        // Save metadata
        let metadata = IndexMetadata::new(content_hash.clone(), stats.documents_indexed);
        metadata.save(index_path)?;

        log::info!(
            "Indexed {} documents ({} bytes, {} errors)",
            stats.documents_indexed,
            stats.bytes_processed,
            stats.errors
        );

        Ok(stats)
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for IndexBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IndexBuilder")
            .field("batch_size", &self.batch_size)
            .field("skip_freshness_check", &self.skip_freshness_check)
            .finish()
    }
}

/// Find all files with the given extensions in a directory tree.
///
/// Recursively walks the directory and returns paths to files matching
/// any of the specified extensions (case-insensitive).
///
/// # Arguments
///
/// * `root` - Root directory to search
/// * `extensions` - Set of extensions to match (lowercase, without dots)
///
/// # Returns
///
/// Sorted list of file paths matching the extensions.
pub async fn find_files_with_extensions(
    root: &Path,
    extensions: &HashSet<String>,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut walker = WalkDir::new(root);

    while let Some(entry) = walker.next().await {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                log::warn!("Walk error: {}", e);
                continue;
            }
        };

        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Check extension
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if extensions.contains(&ext_lower) {
                files.push(path);
            }
        }
    }

    // Sort for deterministic ordering
    files.sort();

    Ok(files)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    // ------------------------------------------------------------------------
    // IndexStats tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_index_stats_default() {
        let stats = IndexStats::default();
        assert_eq!(stats.documents_indexed, 0);
        assert_eq!(stats.files_processed, 0);
        assert_eq!(stats.errors, 0);
    }

    // ------------------------------------------------------------------------
    // DefaultExtractor tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_default_extractor_extract() {
        let extractor = DefaultExtractor::default();
        let path = Path::new("/content/test-doc.md");
        let content = "This is test content";

        let doc = extractor.extract(path, content);
        assert!(doc.is_some());

        let doc = doc.unwrap();
        assert_eq!(doc.id, "test-doc");
        assert_eq!(doc.title, "test-doc.md");
        assert_eq!(doc.content, "This is test content");
        assert_eq!(doc.category, "general");
    }

    #[test]
    fn test_default_extractor_extensions() {
        let extractor = DefaultExtractor::default();
        assert!(extractor.supports_extension("md"));
        assert!(extractor.supports_extension("MD"));
        assert!(extractor.supports_extension("txt"));
        assert!(!extractor.supports_extension("pdf"));
    }

    #[test]
    fn test_default_extractor_custom_extensions() {
        let extractor = DefaultExtractor::with_extensions(vec!["json", "yaml"]);
        assert!(extractor.supports_extension("json"));
        assert!(extractor.supports_extension("yaml"));
        assert!(!extractor.supports_extension("md"));
    }

    // ------------------------------------------------------------------------
    // IndexBuilder tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_index_builder_new() {
        let builder = IndexBuilder::new();
        assert_eq!(builder.batch_size, 100);
        assert!(!builder.skip_freshness_check);
    }

    #[test]
    fn test_index_builder_with_batch_size() {
        let builder = IndexBuilder::new().with_batch_size(50);
        assert_eq!(builder.batch_size, 50);
    }

    #[test]
    fn test_index_builder_force_rebuild() {
        let builder = IndexBuilder::new().force_rebuild();
        assert!(builder.skip_freshness_check);
    }

    #[test]
    fn test_index_builder_debug() {
        let builder = IndexBuilder::new();
        let debug = format!("{:?}", builder);
        assert!(debug.contains("IndexBuilder"));
        assert!(debug.contains("batch_size"));
    }

    // ------------------------------------------------------------------------
    // find_files_with_extensions tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_find_files_with_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        create_test_file(root, "doc1.md", "content 1");
        create_test_file(root, "doc2.txt", "content 2");
        create_test_file(root, "doc3.pdf", "content 3");
        create_test_file(root, "subdir/doc4.md", "content 4");

        let extensions: HashSet<_> = ["md", "txt"].iter().map(|s| s.to_string()).collect();
        let files = find_files_with_extensions(root, &extensions).await.unwrap();

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("doc1.md")));
        assert!(files.iter().any(|p| p.ends_with("doc2.txt")));
        assert!(files.iter().any(|p| p.ends_with("doc4.md")));
        assert!(!files.iter().any(|p| p.ends_with("doc3.pdf")));
    }

    #[tokio::test]
    async fn test_find_files_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let extensions: HashSet<_> = ["md"].iter().map(|s| s.to_string()).collect();
        let files = find_files_with_extensions(temp_dir.path(), &extensions)
            .await
            .unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_find_files_case_insensitive() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "doc.MD", "content");

        let extensions: HashSet<_> = ["md"].iter().map(|s| s.to_string()).collect();
        let files = find_files_with_extensions(temp_dir.path(), &extensions)
            .await
            .unwrap();

        assert_eq!(files.len(), 1);
    }

    // ------------------------------------------------------------------------
    // Integration tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_index_builder_build() {
        let content_dir = TempDir::new().unwrap();
        let index_dir = TempDir::new().unwrap();

        // Create test content
        create_test_file(content_dir.path(), "doc1.md", "# Title\n\nContent here");
        create_test_file(content_dir.path(), "doc2.md", "# Another\n\nMore content");

        let builder = IndexBuilder::new().force_rebuild();
        let stats = builder
            .build(content_dir.path(), index_dir.path())
            .await
            .unwrap();

        assert_eq!(stats.documents_indexed, 2);
        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.errors, 0);
        assert!(!stats.content_hash.is_empty());
    }

    #[tokio::test]
    async fn test_index_builder_freshness_skip() {
        let content_dir = TempDir::new().unwrap();
        let index_dir = TempDir::new().unwrap();

        create_test_file(content_dir.path(), "doc.md", "content");

        // First build
        let builder = IndexBuilder::new().force_rebuild();
        let stats1 = builder
            .build(content_dir.path(), index_dir.path())
            .await
            .unwrap();
        assert_eq!(stats1.documents_indexed, 1);

        // Second build (should skip due to freshness)
        let builder2 = IndexBuilder::new();
        let stats2 = builder2
            .build(content_dir.path(), index_dir.path())
            .await
            .unwrap();

        // Should return cached count without reprocessing
        assert_eq!(stats2.documents_indexed, 1);
        assert_eq!(stats2.files_processed, 0); // Skipped
    }

    #[tokio::test]
    async fn test_index_builder_content_not_found() {
        let index_dir = TempDir::new().unwrap();
        let builder = IndexBuilder::new();

        let result = builder
            .build(Path::new("/nonexistent/path"), index_dir.path())
            .await;
        assert!(result.is_err());
    }
}
