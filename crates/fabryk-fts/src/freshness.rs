//! Index freshness and content hashing.
//!
//! This module provides `IndexMetadata` for tracking content changes and determining
//! when re-indexing is needed. The freshness check is based on file paths and
//! modification times (not content hashing) for efficiency.
//!
//! This module is only available with the `fts-tantivy` feature.
//!
//! # Freshness Strategy
//!
//! The freshness check uses a hash of:
//! - File paths (sorted for determinism)
//! - File modification times
//! - Schema version
//!
//! This approach is fast (no content reading) and catches:
//! - New files added
//! - Files deleted
//! - Files modified (mtime change)
//! - Schema version changes
//!
//! # Usage
//!
//! ```rust,ignore
//! use fabryk_fts::IndexMetadata;
//!
//! // Check if index needs rebuilding
//! if let Some(metadata) = IndexMetadata::load(&index_path)? {
//!     if metadata.is_fresh(&content_path).await? {
//!         println!("Index is up to date");
//!     } else {
//!         println!("Index needs rebuilding");
//!     }
//! }
//!
//! // After indexing, save metadata
//! let metadata = IndexMetadata::new(content_hash, document_count);
//! metadata.save(&index_path)?;
//! ```

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use async_walkdir::WalkDir;
use chrono::{DateTime, Utc};
use fabryk_core::{Error, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::schema::SCHEMA_VERSION;

/// Metadata filename stored in the index directory.
const METADATA_FILE: &str = "fabryk-fts-metadata.json";

/// Metadata about an index, including content hash for freshness checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Hash of the indexed content (paths + mtimes + schema version).
    pub content_hash: String,

    /// Timestamp of last indexing (ISO 8601 format).
    pub indexed_at: String,

    /// Number of documents in the index.
    pub document_count: usize,

    /// Schema version used for this index.
    pub schema_version: u32,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        Self {
            content_hash: String::new(),
            indexed_at: Utc::now().to_rfc3339(),
            document_count: 0,
            schema_version: SCHEMA_VERSION,
        }
    }
}

impl IndexMetadata {
    /// Create new metadata with the given hash and document count.
    pub fn new(content_hash: String, document_count: usize) -> Self {
        Self {
            content_hash,
            indexed_at: Utc::now().to_rfc3339(),
            document_count,
            schema_version: SCHEMA_VERSION,
        }
    }

    /// Load metadata from the index directory.
    ///
    /// Returns `Ok(None)` if the metadata file doesn't exist.
    /// Returns `Err` if the file exists but cannot be parsed.
    pub fn load(index_path: &Path) -> Result<Option<Self>> {
        let metadata_path = index_path.join(METADATA_FILE);

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&metadata_path)
            .map_err(|e| Error::io_with_path(e, &metadata_path))?;

        let metadata: Self = serde_json::from_str(&content)
            .map_err(|e| Error::parse(format!("Invalid metadata JSON: {e}")))?;

        Ok(Some(metadata))
    }

    /// Save metadata to the index directory.
    pub fn save(&self, index_path: &Path) -> Result<()> {
        // Ensure directory exists
        if !index_path.exists() {
            std::fs::create_dir_all(index_path).map_err(|e| Error::io_with_path(e, index_path))?;
        }

        let metadata_path = index_path.join(METADATA_FILE);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::operation(format!("Failed to serialize metadata: {e}")))?;

        std::fs::write(&metadata_path, content)
            .map_err(|e| Error::io_with_path(e, &metadata_path))?;

        Ok(())
    }

    /// Compute hash of content directory.
    ///
    /// The hash is based on:
    /// - Sorted list of file paths
    /// - File modification times
    /// - Current schema version
    ///
    /// This is efficient (no content reading) and deterministic.
    pub async fn compute_hash(content_path: &Path) -> Result<String> {
        let mut hasher = DefaultHasher::new();

        // Include schema version in hash
        SCHEMA_VERSION.hash(&mut hasher);

        // Collect file info (path, mtime)
        let mut file_info: Vec<(String, u64)> = Vec::new();
        let mut walker = WalkDir::new(content_path);

        while let Some(entry) = walker.next().await {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("Walk error during hash computation: {}", e);
                    continue;
                }
            };

            let path = entry.path();

            // Skip directories
            if path.is_dir() {
                continue;
            }

            // Get relative path for consistency
            let relative = path
                .strip_prefix(content_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            // Get modification time as seconds since epoch
            let mtime = match std::fs::metadata(&path) {
                Ok(meta) => meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                Err(_) => 0,
            };

            file_info.push((relative, mtime));
        }

        // Sort for deterministic ordering
        file_info.sort_by(|a, b| a.0.cmp(&b.0));

        // Hash all file info
        for (path, mtime) in file_info {
            path.hash(&mut hasher);
            mtime.hash(&mut hasher);
        }

        // Format as hex string
        let hash_value = hasher.finish();
        Ok(format!("{:016x}", hash_value))
    }

    /// Check if the index is fresh (matches current content).
    ///
    /// Returns `true` if:
    /// - The stored content hash matches the current content hash
    /// - The schema version matches
    pub async fn is_fresh(&self, content_path: &Path) -> Result<bool> {
        // Check schema version first
        if self.schema_version != SCHEMA_VERSION {
            log::info!(
                "Schema version mismatch: stored={}, current={}",
                self.schema_version,
                SCHEMA_VERSION
            );
            return Ok(false);
        }

        // Compute current hash
        let current_hash = Self::compute_hash(content_path).await?;

        if self.content_hash != current_hash {
            log::debug!(
                "Content hash mismatch: stored={}, current={}",
                self.content_hash,
                current_hash
            );
            return Ok(false);
        }

        Ok(true)
    }

    /// Get the indexed timestamp as a DateTime.
    pub fn indexed_at_datetime(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.indexed_at)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }
}

// ============================================================================
// Append metadata (per-source freshness tracking)
// ============================================================================

/// Filename for append metadata stored alongside the index.
const APPEND_METADATA_FILE: &str = "fabryk-fts-appends.json";

/// Per-source metadata entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendSourceEntry {
    /// Content hash when this source was last appended.
    pub content_hash: String,
    /// Number of documents appended from this source.
    pub document_count: usize,
    /// Timestamp of last append.
    pub appended_at: String,
}

/// Metadata tracking per-source freshness for `build_append()` operations.
///
/// Stored as a companion JSON file (`fabryk-fts-appends.json`) alongside the index.
/// Each entry tracks a content source path and its hash, so that unchanged sources
/// can be skipped on subsequent appends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppendMetadata {
    /// Map from source key to source entry.
    pub sources: std::collections::HashMap<String, AppendSourceEntry>,
}

impl AppendMetadata {
    /// Load append metadata from the index directory.
    pub fn load(index_path: &Path) -> Result<Option<Self>> {
        let path = index_path.join(APPEND_METADATA_FILE);
        if !path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&path).map_err(|e| Error::io_with_path(e, &path))?;

        let metadata: Self = serde_json::from_str(&content)
            .map_err(|e| Error::parse(format!("Invalid append metadata JSON: {e}")))?;

        Ok(Some(metadata))
    }

    /// Save append metadata to the index directory.
    pub fn save(&self, index_path: &Path) -> Result<()> {
        if !index_path.exists() {
            std::fs::create_dir_all(index_path).map_err(|e| Error::io_with_path(e, index_path))?;
        }

        let path = index_path.join(APPEND_METADATA_FILE);
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::operation(format!("Failed to serialize append metadata: {e}")))?;

        std::fs::write(&path, content).map_err(|e| Error::io_with_path(e, &path))?;

        Ok(())
    }

    /// Check if a source is fresh (content hasn't changed since last append).
    pub fn is_source_fresh(&self, key: &str, content_hash: &str) -> bool {
        self.sources
            .get(key)
            .map(|entry| entry.content_hash == content_hash)
            .unwrap_or(false)
    }

    /// Get the document count for a source (0 if not found).
    pub fn source_doc_count(&self, key: &str) -> usize {
        self.sources
            .get(key)
            .map(|entry| entry.document_count)
            .unwrap_or(0)
    }

    /// Set or update a source entry.
    pub fn set_source(&mut self, key: &str, content_hash: String, document_count: usize) {
        self.sources.insert(
            key.to_string(),
            AppendSourceEntry {
                content_hash,
                document_count,
                appended_at: Utc::now().to_rfc3339(),
            },
        );
    }
}

/// Check if an index exists and is fresh for the given content.
///
/// Convenience function that combines loading and freshness check.
///
/// Returns:
/// - `Ok(true)` if index exists and is fresh
/// - `Ok(false)` if index doesn't exist or is stale
/// - `Err` if there's an error checking
pub async fn is_index_fresh(index_path: &Path, content_path: &Path) -> Result<bool> {
    match IndexMetadata::load(index_path)? {
        Some(metadata) => metadata.is_fresh(content_path).await,
        None => Ok(false),
    }
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
    // IndexMetadata basic tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_metadata_default() {
        let metadata = IndexMetadata::default();
        assert!(metadata.content_hash.is_empty());
        assert_eq!(metadata.document_count, 0);
        assert_eq!(metadata.schema_version, SCHEMA_VERSION);
        assert!(!metadata.indexed_at.is_empty());
    }

    #[test]
    fn test_metadata_new() {
        let metadata = IndexMetadata::new("abc123".to_string(), 42);
        assert_eq!(metadata.content_hash, "abc123");
        assert_eq!(metadata.document_count, 42);
        assert_eq!(metadata.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn test_metadata_indexed_at_datetime() {
        let metadata = IndexMetadata::default();
        let dt = metadata.indexed_at_datetime();
        assert!(dt.is_some());
    }

    // ------------------------------------------------------------------------
    // Save/Load tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_metadata_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let metadata = IndexMetadata::new("test-hash".to_string(), 10);

        // Save
        metadata.save(temp_dir.path()).unwrap();

        // Verify file exists
        let metadata_path = temp_dir.path().join(METADATA_FILE);
        assert!(metadata_path.exists());

        // Load
        let loaded = IndexMetadata::load(temp_dir.path()).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.content_hash, "test-hash");
        assert_eq!(loaded.document_count, 10);
        assert_eq!(loaded.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn test_metadata_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let result = IndexMetadata::load(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_metadata_save_creates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let nested_path = temp_dir.path().join("nested/index");

        let metadata = IndexMetadata::new("hash".to_string(), 5);
        metadata.save(&nested_path).unwrap();

        assert!(nested_path.join(METADATA_FILE).exists());
    }

    // ------------------------------------------------------------------------
    // Hash computation tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_compute_hash_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let hash = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 16); // 64-bit hash as hex
    }

    #[tokio::test]
    async fn test_compute_hash_with_files() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "file1.txt", "content 1");
        create_test_file(temp_dir.path(), "file2.txt", "content 2");

        let hash = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();
        assert!(!hash.is_empty());
    }

    #[tokio::test]
    async fn test_compute_hash_deterministic() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "a.txt", "aaa");
        create_test_file(temp_dir.path(), "b.txt", "bbb");

        let hash1 = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();
        let hash2 = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_compute_hash_changes_with_new_file() {
        let temp_dir = TempDir::new().unwrap();
        create_test_file(temp_dir.path(), "file1.txt", "content");

        let hash1 = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();

        // Add new file
        create_test_file(temp_dir.path(), "file2.txt", "more content");

        let hash2 = IndexMetadata::compute_hash(temp_dir.path()).await.unwrap();

        assert_ne!(hash1, hash2);
    }

    // ------------------------------------------------------------------------
    // Freshness tests
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_is_fresh_true() {
        let content_dir = TempDir::new().unwrap();
        create_test_file(content_dir.path(), "doc.md", "content");

        let hash = IndexMetadata::compute_hash(content_dir.path())
            .await
            .unwrap();
        let metadata = IndexMetadata::new(hash, 1);

        assert!(metadata.is_fresh(content_dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_fresh_false_content_changed() {
        let content_dir = TempDir::new().unwrap();
        create_test_file(content_dir.path(), "doc.md", "content");

        let hash = IndexMetadata::compute_hash(content_dir.path())
            .await
            .unwrap();
        let metadata = IndexMetadata::new(hash, 1);

        // Add new file
        create_test_file(content_dir.path(), "doc2.md", "more content");

        assert!(!metadata.is_fresh(content_dir.path()).await.unwrap());
    }

    #[tokio::test]
    async fn test_is_fresh_false_schema_version_mismatch() {
        let content_dir = TempDir::new().unwrap();
        create_test_file(content_dir.path(), "doc.md", "content");

        let hash = IndexMetadata::compute_hash(content_dir.path())
            .await
            .unwrap();

        let mut metadata = IndexMetadata::new(hash, 1);
        metadata.schema_version = SCHEMA_VERSION + 1; // Wrong version

        assert!(!metadata.is_fresh(content_dir.path()).await.unwrap());
    }

    // ------------------------------------------------------------------------
    // is_index_fresh convenience function
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_is_index_fresh_no_metadata() {
        let content_dir = TempDir::new().unwrap();
        let index_dir = TempDir::new().unwrap();

        let result = is_index_fresh(index_dir.path(), content_dir.path())
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_is_index_fresh_with_metadata() {
        let content_dir = TempDir::new().unwrap();
        let index_dir = TempDir::new().unwrap();
        create_test_file(content_dir.path(), "doc.md", "content");

        // Save metadata
        let hash = IndexMetadata::compute_hash(content_dir.path())
            .await
            .unwrap();
        let metadata = IndexMetadata::new(hash, 1);
        metadata.save(index_dir.path()).unwrap();

        let result = is_index_fresh(index_dir.path(), content_dir.path())
            .await
            .unwrap();
        assert!(result);
    }

    // ------------------------------------------------------------------------
    // Serialization tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_metadata_serialization_roundtrip() {
        let metadata = IndexMetadata::new("hash123".to_string(), 50);
        let json = serde_json::to_string(&metadata).unwrap();

        let restored: IndexMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.content_hash, "hash123");
        assert_eq!(restored.document_count, 50);
    }

    #[test]
    fn test_metadata_json_format() {
        let metadata = IndexMetadata::new("abc".to_string(), 1);
        let json = serde_json::to_string_pretty(&metadata).unwrap();

        assert!(json.contains("content_hash"));
        assert!(json.contains("indexed_at"));
        assert!(json.contains("document_count"));
        assert!(json.contains("schema_version"));
    }

    // ------------------------------------------------------------------------
    // AppendMetadata tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_append_metadata_default() {
        let metadata = AppendMetadata::default();
        assert!(metadata.sources.is_empty());
    }

    #[test]
    fn test_append_metadata_set_and_check() {
        let mut metadata = AppendMetadata::default();
        metadata.set_source("append:/data/sources", "hash123".to_string(), 42);

        assert!(metadata.is_source_fresh("append:/data/sources", "hash123"));
        assert!(!metadata.is_source_fresh("append:/data/sources", "different"));
        assert!(!metadata.is_source_fresh("append:/other", "hash123"));
        assert_eq!(metadata.source_doc_count("append:/data/sources"), 42);
        assert_eq!(metadata.source_doc_count("missing"), 0);
    }

    #[test]
    fn test_append_metadata_save_and_load() {
        let temp_dir = TempDir::new().unwrap();

        let mut metadata = AppendMetadata::default();
        metadata.set_source("key1", "hash1".to_string(), 10);
        metadata.set_source("key2", "hash2".to_string(), 20);
        metadata.save(temp_dir.path()).unwrap();

        let loaded = AppendMetadata::load(temp_dir.path()).unwrap().unwrap();
        assert!(loaded.is_source_fresh("key1", "hash1"));
        assert!(loaded.is_source_fresh("key2", "hash2"));
        assert_eq!(loaded.source_doc_count("key1"), 10);
        assert_eq!(loaded.source_doc_count("key2"), 20);
    }

    #[test]
    fn test_append_metadata_load_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let result = AppendMetadata::load(temp_dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_append_metadata_update_source() {
        let mut metadata = AppendMetadata::default();
        metadata.set_source("key", "hash1".to_string(), 5);
        assert!(metadata.is_source_fresh("key", "hash1"));

        // Update with new hash
        metadata.set_source("key", "hash2".to_string(), 10);
        assert!(!metadata.is_source_fresh("key", "hash1"));
        assert!(metadata.is_source_fresh("key", "hash2"));
        assert_eq!(metadata.source_doc_count("key"), 10);
    }
}
