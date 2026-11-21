---
title: "CC Prompt: Fabryk 3.4 — Indexer & Builder"
milestone: "3.4"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["3.1-3.3 complete"]
governing-docs: [0011-audit §4.3, 0013-project-plan]
---

# CC Prompt: Fabryk 3.4 — Indexer & Builder

## Context

This milestone extracts the Tantivy index writer (`Indexer`) and batch indexing
orchestration (`IndexBuilder`) from music-theory. These components handle
document indexing and index lifecycle management.

**Crate:** `fabryk-fts`
**Feature:** `fts-tantivy` (required)
**Risk:** Medium (Tantivy API complexity)

## Source Files

| File | Lines | Tests | Classification |
|------|-------|-------|----------------|
| `search/indexer.rs` | 455 | 10 | P (Parameterized) |
| `search/builder.rs` | 548 | 11 | P (Parameterized) |
| `search/freshness.rs` | 453 | 7 | D → P (needs adaptation) |

**Note:** `builder.rs` and `freshness.rs` have music-theory-specific path
assumptions that need to be generalized.

## Objective

1. Extract `indexer.rs` to `fabryk-fts/src/indexer.rs`
2. Extract `builder.rs` to `fabryk-fts/src/builder.rs` (generalized)
3. Extract `freshness.rs` to `fabryk-fts/src/freshness.rs` (generalized)
4. Provide trait/callback pattern for domain-specific document extraction
5. Verify: `cargo test -p fabryk-fts --features fts-tantivy` passes

## Implementation Steps

### Step 1: Create `fabryk-fts/src/indexer.rs`

```rust
//! Tantivy index writer wrapper.
//!
//! This module provides `Indexer`, a wrapper around Tantivy's `IndexWriter`
//! that handles document conversion and index lifecycle.
//!
//! # Usage
//!
//! ```rust,ignore
//! use fabryk_fts::{Indexer, SearchDocument, SearchSchema};
//!
//! // Create or open index
//! let schema = SearchSchema::build();
//! let mut indexer = Indexer::new(&index_path, &schema)?;
//!
//! // Add documents
//! let doc = SearchDocument::builder()
//!     .id("my-doc")
//!     .title("My Document")
//!     .content("Content here...")
//!     .category("general")
//!     .build();
//!
//! indexer.add_document(&doc)?;
//! indexer.commit()?;
//! ```

use std::path::Path;

use fabryk_core::{Error, Result};
use tantivy::schema::Field;
use tantivy::{doc, Index, IndexWriter, TantivyDocument};

use crate::document::SearchDocument;
use crate::schema::SearchSchema;

/// Index writer buffer size (50MB).
const WRITER_BUFFER_SIZE: usize = 50_000_000;

/// Tantivy index writer wrapper.
///
/// Handles document conversion and index lifecycle.
pub struct Indexer {
    index: Index,
    writer: IndexWriter,
    schema: SearchSchema,
}

impl Indexer {
    /// Create or open a Tantivy index at the given path.
    ///
    /// If the directory doesn't exist, creates a new index.
    /// If the directory exists, opens the existing index.
    pub fn new(index_path: &Path, schema: &SearchSchema) -> Result<Self> {
        // Ensure directory exists
        if !index_path.exists() {
            std::fs::create_dir_all(index_path).map_err(|e| {
                Error::io_with_path(e, index_path)
            })?;
        }

        // Create or open index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path).map_err(|e| {
                Error::operation(format!("Failed to open index: {e}"))
            })?
        } else {
            Index::create_in_dir(index_path, schema.schema().clone()).map_err(|e| {
                Error::operation(format!("Failed to create index: {e}"))
            })?
        };

        // Register tokenizers
        SearchSchema::register_tokenizers(&index);

        // Create writer
        let writer = index.writer(WRITER_BUFFER_SIZE).map_err(|e| {
            Error::operation(format!("Failed to create index writer: {e}"))
        })?;

        Ok(Self {
            index,
            writer,
            schema: schema.clone(),
        })
    }

    /// Create an in-memory index (for testing).
    pub fn new_in_memory(schema: &SearchSchema) -> Result<Self> {
        let index = Index::create_in_ram(schema.schema().clone());
        SearchSchema::register_tokenizers(&index);

        let writer = index.writer(WRITER_BUFFER_SIZE).map_err(|e| {
            Error::operation(format!("Failed to create index writer: {e}"))
        })?;

        Ok(Self {
            index,
            writer,
            schema: schema.clone(),
        })
    }

    /// Add a document to the index.
    ///
    /// The document is staged but not yet searchable until `commit()` is called.
    pub fn add_document(&mut self, doc: &SearchDocument) -> Result<()> {
        let tantivy_doc = self.convert_to_tantivy_doc(doc);
        self.writer.add_document(tantivy_doc).map_err(|e| {
            Error::operation(format!("Failed to add document: {e}"))
        })?;
        Ok(())
    }

    /// Commit staged changes to make them searchable.
    pub fn commit(&mut self) -> Result<()> {
        self.writer.commit().map_err(|e| {
            Error::operation(format!("Failed to commit index: {e}"))
        })?;
        Ok(())
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) -> Result<()> {
        self.writer.delete_all_documents().map_err(|e| {
            Error::operation(format!("Failed to clear index: {e}"))
        })?;
        self.commit()
    }

    /// Get reference to the underlying Tantivy index.
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Get the schema.
    pub fn schema(&self) -> &SearchSchema {
        &self.schema
    }

    /// Convert SearchDocument to Tantivy document.
    fn convert_to_tantivy_doc(&self, doc: &SearchDocument) -> TantivyDocument {
        let s = &self.schema;

        let mut tantivy_doc = TantivyDocument::new();

        // Identity fields
        add_text(&mut tantivy_doc, s.id, &doc.id);
        add_text(&mut tantivy_doc, s.path, &doc.path);

        // Full-text fields
        add_text(&mut tantivy_doc, s.title, &doc.title);
        if let Some(ref desc) = doc.description {
            add_text(&mut tantivy_doc, s.description, desc);
        }
        add_text(&mut tantivy_doc, s.content, &doc.content);

        // Facet fields
        add_text(&mut tantivy_doc, s.category, &doc.category);
        if let Some(ref source) = doc.source {
            add_text(&mut tantivy_doc, s.source, source);
        }
        for tag in &doc.tags {
            add_text(&mut tantivy_doc, s.tags, tag);
        }

        // Metadata fields
        if let Some(ref chapter) = doc.chapter {
            add_text(&mut tantivy_doc, s.chapter, chapter);
        }
        if let Some(ref part) = doc.part {
            add_text(&mut tantivy_doc, s.part, part);
        }
        if let Some(ref author) = doc.author {
            add_text(&mut tantivy_doc, s.author, author);
        }
        if let Some(ref date) = doc.date {
            add_text(&mut tantivy_doc, s.date, date);
        }
        if let Some(ref content_type) = doc.content_type {
            add_text(&mut tantivy_doc, s.content_type, content_type);
        }
        if let Some(ref section) = doc.section {
            add_text(&mut tantivy_doc, s.section, section);
        }

        tantivy_doc
    }
}

fn add_text(doc: &mut TantivyDocument, field: Field, value: &str) {
    doc.add_text(field, value);
}

impl std::fmt::Debug for Indexer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Indexer")
            .field("index", &"<tantivy::Index>")
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_doc(id: &str) -> SearchDocument {
        SearchDocument::builder()
            .id(id)
            .title(format!("Test {id}"))
            .content("Test content")
            .category("test")
            .build()
    }

    #[test]
    fn test_indexer_in_memory() {
        let schema = SearchSchema::build();
        let indexer = Indexer::new_in_memory(&schema);
        assert!(indexer.is_ok());
    }

    #[test]
    fn test_indexer_add_document() {
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new_in_memory(&schema).unwrap();

        let doc = create_test_doc("test-1");
        let result = indexer.add_document(&doc);
        assert!(result.is_ok());
    }

    #[test]
    fn test_indexer_commit() {
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new_in_memory(&schema).unwrap();

        let doc = create_test_doc("test-1");
        indexer.add_document(&doc).unwrap();

        let result = indexer.commit();
        assert!(result.is_ok());
    }

    #[test]
    fn test_indexer_clear() {
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new_in_memory(&schema).unwrap();

        indexer.add_document(&create_test_doc("1")).unwrap();
        indexer.add_document(&create_test_doc("2")).unwrap();
        indexer.commit().unwrap();

        let result = indexer.clear();
        assert!(result.is_ok());
    }

    #[test]
    fn test_indexer_on_disk() {
        let temp_dir = tempfile::tempdir().unwrap();
        let schema = SearchSchema::build();

        let mut indexer = Indexer::new(temp_dir.path(), &schema).unwrap();
        indexer.add_document(&create_test_doc("disk-1")).unwrap();
        indexer.commit().unwrap();

        // Re-open
        let indexer2 = Indexer::new(temp_dir.path(), &schema).unwrap();
        assert!(indexer2.index().reader().is_ok());
    }

    #[test]
    fn test_indexer_document_with_all_fields() {
        let schema = SearchSchema::build();
        let mut indexer = Indexer::new_in_memory(&schema).unwrap();

        let doc = SearchDocument {
            id: "full-doc".to_string(),
            path: "/path/to/doc.md".to_string(),
            title: "Full Document".to_string(),
            description: Some("A complete document".to_string()),
            content: "Full content here".to_string(),
            category: "test".to_string(),
            source: Some("Test Source".to_string()),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            chapter: Some("1".to_string()),
            part: Some("Part I".to_string()),
            author: Some("Test Author".to_string()),
            date: Some("2025-01-01".to_string()),
            content_type: Some("concept".to_string()),
            section: Some("1.1".to_string()),
        };

        let result = indexer.add_document(&doc);
        assert!(result.is_ok());
    }
}
```

### Step 2: Create `fabryk-fts/src/builder.rs`

```rust
//! Index building orchestration.
//!
//! This module provides `IndexBuilder` for batch indexing operations. It
//! coordinates document extraction and indexing with progress tracking.
//!
//! # Domain Integration
//!
//! Domain crates provide a `DocumentExtractor` implementation to convert
//! their content files into `SearchDocument` instances:
//!
//! ```rust,ignore
//! use fabryk_fts::{IndexBuilder, DocumentExtractor, SearchDocument};
//!
//! struct MyExtractor;
//!
//! impl DocumentExtractor for MyExtractor {
//!     async fn extract(&self, path: &Path) -> Result<Option<SearchDocument>> {
//!         // Read file, parse frontmatter, create document
//!     }
//! }
//!
//! let builder = IndexBuilder::new(&index_path)?;
//! let stats = builder.build_index(&content_path, &MyExtractor).await?;
//! ```

use std::path::Path;

use async_trait::async_trait;
use fabryk_core::Result;
use serde::{Deserialize, Serialize};

use crate::document::SearchDocument;
use crate::indexer::Indexer;
use crate::schema::SearchSchema;

/// Trait for domain-specific document extraction.
///
/// Implement this trait to define how your domain's content files are
/// converted to `SearchDocument` instances.
#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    /// Extract a document from a file path.
    ///
    /// Returns:
    /// - `Ok(Some(doc))` - Successfully extracted document
    /// - `Ok(None)` - File should be skipped (not indexable)
    /// - `Err(e)` - Extraction failed (logged, indexing continues)
    async fn extract(&self, path: &Path) -> Result<Option<SearchDocument>>;

    /// Get file extensions to index (e.g., `["md", "txt"]`).
    fn extensions(&self) -> &[&str] {
        &["md"]
    }
}

/// Statistics from an indexing operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStats {
    /// Total files found.
    pub files_found: usize,
    /// Files successfully indexed.
    pub files_indexed: usize,
    /// Files skipped (not indexable).
    pub files_skipped: usize,
    /// Files with extraction errors.
    pub files_errored: usize,
}

impl IndexStats {
    /// Create empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if any documents were indexed.
    pub fn has_documents(&self) -> bool {
        self.files_indexed > 0
    }
}

impl std::ops::Add for IndexStats {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            files_found: self.files_found + other.files_found,
            files_indexed: self.files_indexed + other.files_indexed,
            files_skipped: self.files_skipped + other.files_skipped,
            files_errored: self.files_errored + other.files_errored,
        }
    }
}

impl std::ops::AddAssign for IndexStats {
    fn add_assign(&mut self, other: Self) {
        *self = self.clone() + other;
    }
}

/// Index builder for batch indexing operations.
pub struct IndexBuilder {
    indexer: Indexer,
}

impl IndexBuilder {
    /// Create a new index builder.
    pub fn new(index_path: &Path) -> Result<Self> {
        let schema = SearchSchema::build();
        let indexer = Indexer::new(index_path, &schema)?;
        Ok(Self { indexer })
    }

    /// Create an in-memory index builder (for testing).
    pub fn new_in_memory() -> Result<Self> {
        let schema = SearchSchema::build();
        let indexer = Indexer::new_in_memory(&schema)?;
        Ok(Self { indexer })
    }

    /// Build the index from content files.
    ///
    /// Walks the content directory, extracts documents using the provided
    /// extractor, and indexes them.
    ///
    /// # Arguments
    ///
    /// * `content_path` - Root directory containing content files
    /// * `extractor` - Domain-specific document extractor
    /// * `rebuild` - If true, clears existing index first
    pub async fn build_index<E: DocumentExtractor>(
        &mut self,
        content_path: &Path,
        extractor: &E,
        rebuild: bool,
    ) -> Result<IndexStats> {
        if rebuild {
            self.indexer.clear()?;
        }

        let mut stats = IndexStats::new();

        // Walk content directory
        let extensions = extractor.extensions();
        let files = find_files_with_extensions(content_path, extensions).await?;

        stats.files_found = files.len();
        log::info!("Found {} files to index", stats.files_found);

        for (i, file_path) in files.iter().enumerate() {
            // Progress logging every 50 files
            if i > 0 && i % 50 == 0 {
                log::info!("Indexed {}/{} files", i, stats.files_found);
            }

            match extractor.extract(file_path).await {
                Ok(Some(doc)) => {
                    if let Err(e) = self.indexer.add_document(&doc) {
                        log::warn!("Failed to index {}: {e}", file_path.display());
                        stats.files_errored += 1;
                    } else {
                        stats.files_indexed += 1;
                    }
                }
                Ok(None) => {
                    stats.files_skipped += 1;
                }
                Err(e) => {
                    log::warn!("Failed to extract {}: {e}", file_path.display());
                    stats.files_errored += 1;
                }
            }
        }

        // Commit changes
        self.indexer.commit()?;

        log::info!(
            "Indexing complete: {} indexed, {} skipped, {} errors",
            stats.files_indexed,
            stats.files_skipped,
            stats.files_errored
        );

        Ok(stats)
    }

    /// Get reference to the underlying indexer.
    pub fn indexer(&self) -> &Indexer {
        &self.indexer
    }

    /// Get mutable reference to the underlying indexer.
    pub fn indexer_mut(&mut self) -> &mut Indexer {
        &mut self.indexer
    }
}

/// Find files with given extensions in a directory tree.
async fn find_files_with_extensions(
    root: &Path,
    extensions: &[&str],
) -> Result<Vec<std::path::PathBuf>> {
    use async_walkdir::WalkDir;
    use futures::StreamExt;

    let mut files = Vec::new();
    let mut walker = WalkDir::new(root);

    while let Some(entry) = walker.next().await {
        let entry = entry.map_err(|e| {
            fabryk_core::Error::io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.iter().any(|&e| e.eq_ignore_ascii_case(ext)) {
                    files.push(path);
                }
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

    struct TestExtractor;

    #[async_trait]
    impl DocumentExtractor for TestExtractor {
        async fn extract(&self, path: &Path) -> Result<Option<SearchDocument>> {
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            Ok(Some(
                SearchDocument::builder()
                    .id(id)
                    .path(path.to_string_lossy())
                    .title(format!("Test: {id}"))
                    .content("Test content")
                    .category("test")
                    .build(),
            ))
        }
    }

    fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_index_stats_default() {
        let stats = IndexStats::new();
        assert_eq!(stats.files_found, 0);
        assert!(!stats.has_documents());
    }

    #[test]
    fn test_index_stats_add() {
        let a = IndexStats {
            files_found: 10,
            files_indexed: 8,
            files_skipped: 1,
            files_errored: 1,
        };
        let b = IndexStats {
            files_found: 5,
            files_indexed: 4,
            files_skipped: 1,
            files_errored: 0,
        };

        let combined = a + b;
        assert_eq!(combined.files_found, 15);
        assert_eq!(combined.files_indexed, 12);
    }

    #[tokio::test]
    async fn test_builder_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content_dir = temp_dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();

        let index_dir = temp_dir.path().join("index");
        let mut builder = IndexBuilder::new(&index_dir).unwrap();

        let stats = builder
            .build_index(&content_dir, &TestExtractor, false)
            .await
            .unwrap();

        assert_eq!(stats.files_found, 0);
        assert_eq!(stats.files_indexed, 0);
    }

    #[tokio::test]
    async fn test_builder_with_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content_dir = temp_dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();

        // Create test files
        create_test_file(&content_dir, "doc1.md", "# Doc 1\nContent");
        create_test_file(&content_dir, "doc2.md", "# Doc 2\nContent");
        create_test_file(&content_dir, "ignored.txt", "Should be ignored");

        let index_dir = temp_dir.path().join("index");
        let mut builder = IndexBuilder::new(&index_dir).unwrap();

        let stats = builder
            .build_index(&content_dir, &TestExtractor, false)
            .await
            .unwrap();

        assert_eq!(stats.files_found, 2); // Only .md files
        assert_eq!(stats.files_indexed, 2);
    }

    #[tokio::test]
    async fn test_builder_rebuild() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content_dir = temp_dir.path().join("content");
        std::fs::create_dir_all(&content_dir).unwrap();
        create_test_file(&content_dir, "doc.md", "Content");

        let index_dir = temp_dir.path().join("index");
        let mut builder = IndexBuilder::new(&index_dir).unwrap();

        // First build
        builder
            .build_index(&content_dir, &TestExtractor, false)
            .await
            .unwrap();

        // Rebuild (should clear first)
        let stats = builder
            .build_index(&content_dir, &TestExtractor, true)
            .await
            .unwrap();

        assert_eq!(stats.files_indexed, 1);
    }
}
```

### Step 3: Create `fabryk-fts/src/freshness.rs`

```rust
//! Index freshness tracking via content hashing.
//!
//! This module tracks whether an index needs rebuilding by comparing:
//! - Schema version
//! - Content hash (based on file paths and modification times)
//!
//! # Usage
//!
//! ```rust,ignore
//! use fabryk_fts::freshness::{IndexMetadata, is_index_fresh, compute_content_hash};
//!
//! let content_hash = compute_content_hash(&content_path).await?;
//! let is_fresh = is_index_fresh(&index_path, &content_hash)?;
//!
//! if !is_fresh {
//!     // Rebuild index
//! }
//! ```

use std::path::Path;

use async_walkdir::WalkDir;
use fabryk_core::{Error, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::schema::SCHEMA_VERSION;

/// Metadata about an index for freshness checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Schema version used to build the index.
    pub schema_version: u32,
    /// Hash of content files (paths + mtimes).
    pub content_hash: String,
    /// Number of documents indexed.
    pub document_count: usize,
    /// Timestamp of last index build.
    pub built_at: String,
}

impl IndexMetadata {
    /// Create new metadata.
    pub fn new(content_hash: String, document_count: usize) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            content_hash,
            document_count,
            built_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Load metadata from an index directory.
    pub fn load(index_path: &Path) -> Result<Option<Self>> {
        let meta_path = index_path.join("fabryk_meta.json");
        if !meta_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&meta_path).map_err(|e| {
            Error::io_with_path(e, &meta_path)
        })?;

        let meta: Self = serde_json::from_str(&content).map_err(|e| {
            Error::parse(format!("Failed to parse index metadata: {e}"))
        })?;

        Ok(Some(meta))
    }

    /// Save metadata to an index directory.
    pub fn save(&self, index_path: &Path) -> Result<()> {
        let meta_path = index_path.join("fabryk_meta.json");
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            Error::operation(format!("Failed to serialize index metadata: {e}"))
        })?;

        std::fs::write(&meta_path, content).map_err(|e| {
            Error::io_with_path(e, &meta_path)
        })?;

        Ok(())
    }
}

/// Check if an index is fresh (doesn't need rebuilding).
///
/// Returns `true` if:
/// - Index metadata exists
/// - Schema version matches
/// - Content hash matches
pub fn is_index_fresh(index_path: &Path, current_content_hash: &str) -> Result<bool> {
    let meta = match IndexMetadata::load(index_path)? {
        Some(m) => m,
        None => {
            log::debug!("No index metadata found");
            return Ok(false);
        }
    };

    // Check schema version
    if meta.schema_version != SCHEMA_VERSION {
        log::info!(
            "Schema version mismatch: index={}, current={}",
            meta.schema_version,
            SCHEMA_VERSION
        );
        return Ok(false);
    }

    // Check content hash
    if meta.content_hash != current_content_hash {
        log::info!("Content hash mismatch, index needs rebuild");
        return Ok(false);
    }

    log::debug!("Index is fresh");
    Ok(true)
}

/// Compute a hash of content files for freshness checking.
///
/// Hashes file paths and modification times (not content) for speed.
pub async fn compute_content_hash(content_path: &Path) -> Result<String> {
    use std::collections::BTreeMap;
    use std::hash::{Hash, Hasher};

    // Collect file info (sorted for determinism)
    let mut files: BTreeMap<String, u64> = BTreeMap::new();
    let mut walker = WalkDir::new(content_path);

    while let Some(entry) = walker.next().await {
        let entry = entry.map_err(|e| {
            Error::io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if ext.eq_ignore_ascii_case("md") {
                    let mtime = path
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let rel_path = path
                        .strip_prefix(content_path)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.to_string_lossy().to_string());

                    files.insert(rel_path, mtime);
                }
            }
        }
    }

    // Hash the collected data
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for (path, mtime) in &files {
        path.hash(&mut hasher);
        mtime.hash(&mut hasher);
    }

    let hash = hasher.finish();
    Ok(format!("{:016x}", hash))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file(dir: &Path, name: &str) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();
    }

    #[test]
    fn test_metadata_creation() {
        let meta = IndexMetadata::new("abc123".to_string(), 100);
        assert_eq!(meta.schema_version, SCHEMA_VERSION);
        assert_eq!(meta.content_hash, "abc123");
        assert_eq!(meta.document_count, 100);
    }

    #[test]
    fn test_metadata_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let meta = IndexMetadata::new("test-hash".to_string(), 50);

        meta.save(temp_dir.path()).unwrap();
        let loaded = IndexMetadata::load(temp_dir.path()).unwrap().unwrap();

        assert_eq!(loaded.content_hash, "test-hash");
        assert_eq!(loaded.document_count, 50);
    }

    #[test]
    fn test_metadata_load_missing() {
        let temp_dir = tempfile::tempdir().unwrap();
        let loaded = IndexMetadata::load(temp_dir.path()).unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn test_compute_content_hash_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let hash = compute_content_hash(temp_dir.path()).await.unwrap();
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn test_compute_content_hash_with_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        create_test_file(temp_dir.path(), "file1.md");
        create_test_file(temp_dir.path(), "file2.md");

        let hash1 = compute_content_hash(temp_dir.path()).await.unwrap();

        // Same files should produce same hash
        let hash2 = compute_content_hash(temp_dir.path()).await.unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_is_index_fresh_no_metadata() {
        let temp_dir = tempfile::tempdir().unwrap();
        let result = is_index_fresh(temp_dir.path(), "any-hash").unwrap();
        assert!(!result);
    }

    #[test]
    fn test_is_index_fresh_matching() {
        let temp_dir = tempfile::tempdir().unwrap();
        let meta = IndexMetadata::new("test-hash".to_string(), 10);
        meta.save(temp_dir.path()).unwrap();

        let result = is_index_fresh(temp_dir.path(), "test-hash").unwrap();
        assert!(result);
    }

    #[test]
    fn test_is_index_fresh_hash_mismatch() {
        let temp_dir = tempfile::tempdir().unwrap();
        let meta = IndexMetadata::new("old-hash".to_string(), 10);
        meta.save(temp_dir.path()).unwrap();

        let result = is_index_fresh(temp_dir.path(), "new-hash").unwrap();
        assert!(!result);
    }
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

- [ ] `fabryk-fts/src/indexer.rs` with `Indexer` struct
- [ ] `fabryk-fts/src/builder.rs` with `IndexBuilder` and `DocumentExtractor` trait
- [ ] `fabryk-fts/src/freshness.rs` with `IndexMetadata` and hash functions
- [ ] `DocumentExtractor` trait for domain-specific extraction
- [ ] `IndexStats` for tracking indexing results
- [ ] Content hash computation for freshness checking
- [ ] All tests pass (~28 tests)
- [ ] `cargo clippy` clean

## Design Notes

### DocumentExtractor pattern

Rather than hardcoding music-theory paths, domains provide an extractor:

```rust
#[async_trait]
pub trait DocumentExtractor: Send + Sync {
    async fn extract(&self, path: &Path) -> Result<Option<SearchDocument>>;
    fn extensions(&self) -> &[&str];
}
```

This enables reuse across domains without path coupling.

### Freshness via content hash

Index freshness uses a hash of (path, mtime) pairs rather than content hashes:
- Faster: No file content reading
- Sufficient: Detects additions, deletions, modifications
- Deterministic: Sorted paths ensure consistent hashes

## Commit Message

```
feat(fts): add indexer, builder, and freshness modules

Indexer:
- Tantivy IndexWriter wrapper
- Document conversion to Tantivy format
- In-memory and on-disk modes

IndexBuilder:
- Batch indexing orchestration
- DocumentExtractor trait for domain integration
- Progress logging and statistics

IndexMetadata:
- Content hash freshness checking
- Schema version validation
- Metadata persistence

~28 tests for indexing functionality.

Ref: Doc 0013 milestone 3.4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
