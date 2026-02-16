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
use tantivy::{Index, IndexWriter, TantivyDocument};

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
            std::fs::create_dir_all(index_path).map_err(|e| Error::io_with_path(e, index_path))?;
        }

        // Create or open index
        let index = if index_path.join("meta.json").exists() {
            Index::open_in_dir(index_path)
                .map_err(|e| Error::operation(format!("Failed to open index: {e}")))?
        } else {
            Index::create_in_dir(index_path, schema.schema().clone())
                .map_err(|e| Error::operation(format!("Failed to create index: {e}")))?
        };

        // Register tokenizers
        SearchSchema::register_tokenizers(&index);

        // Create writer
        let writer = index
            .writer(WRITER_BUFFER_SIZE)
            .map_err(|e| Error::operation(format!("Failed to create index writer: {e}")))?;

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

        let writer = index
            .writer(WRITER_BUFFER_SIZE)
            .map_err(|e| Error::operation(format!("Failed to create index writer: {e}")))?;

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
        self.writer
            .add_document(tantivy_doc)
            .map_err(|e| Error::operation(format!("Failed to add document: {e}")))?;
        Ok(())
    }

    /// Commit staged changes to make them searchable.
    pub fn commit(&mut self) -> Result<()> {
        self.writer
            .commit()
            .map_err(|e| Error::operation(format!("Failed to commit index: {e}")))?;
        Ok(())
    }

    /// Clear all documents from the index.
    pub fn clear(&mut self) -> Result<()> {
        self.writer
            .delete_all_documents()
            .map_err(|e| Error::operation(format!("Failed to clear index: {e}")))?;
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

        {
            let mut indexer = Indexer::new(temp_dir.path(), &schema).unwrap();
            indexer.add_document(&create_test_doc("disk-1")).unwrap();
            indexer.commit().unwrap();
            // indexer is dropped here, releasing the lock
        }

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
