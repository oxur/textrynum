//! VectorExtractor trait for domain-specific document extraction.
//!
//! This module defines the core abstraction that enables Fabryk to support
//! multiple knowledge domains for vector search. Each domain implements
//! `VectorExtractor` to control how content files are transformed into
//! `VectorDocument` instances for embedding.
//!
//! # Design Philosophy
//!
//! The extractor separates text composition from embedding. Domains control
//! what text gets embedded (title, description, body, etc.) by composing
//! the `VectorDocument.text` field. The embedding provider then handles
//! the actual vector generation.

use crate::types::VectorDocument;
use fabryk_core::Result;
use std::path::Path;

/// Trait for extracting vector documents from domain-specific content.
///
/// Each knowledge domain (music theory, math, etc.) implements this trait
/// to define how its markdown files with frontmatter are transformed into
/// `VectorDocument` instances. The key responsibility is **text composition**:
/// deciding what content should be embedded.
///
/// # Lifecycle
///
/// For each content file, `VectorIndexBuilder` calls:
///
/// 1. `extract_document()` — Parse file and compose text for embedding
///
/// The returned `VectorDocument.text` is what gets embedded by the
/// `EmbeddingProvider`.
pub trait VectorExtractor: Send + Sync {
    /// Extract a vector document from a content file.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for content
    /// * `file_path` - Full path to the file being processed
    /// * `frontmatter` - Parsed YAML frontmatter as generic Value
    /// * `content` - Markdown body (after frontmatter)
    ///
    /// # Text Composition
    ///
    /// The implementation should compose the `text` field with all content
    /// that should influence semantic similarity. A common pattern is:
    ///
    /// ```text
    /// title | description | key terms | body content
    /// ```
    fn extract_document(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &yaml_serde::Value,
        content: &str,
    ) -> Result<VectorDocument>;

    /// Returns the content glob pattern for this domain.
    ///
    /// Used by `VectorIndexBuilder` to discover content files.
    /// Default: `"**/*.md"` (all markdown files recursively).
    fn content_glob(&self) -> &str {
        "**/*.md"
    }

    /// Returns the name of this extractor for logging/debugging.
    fn name(&self) -> &str {
        "unnamed"
    }
}

// ============================================================================
// Mock extractor for testing
// ============================================================================

/// A simple mock extractor for testing.
///
/// Composes text from frontmatter title + body content, separated by ` | `.
#[derive(Clone, Debug, Default)]
pub struct MockVectorExtractor;

impl VectorExtractor for MockVectorExtractor {
    fn extract_document(
        &self,
        _base_path: &Path,
        file_path: &Path,
        frontmatter: &yaml_serde::Value,
        content: &str,
    ) -> Result<VectorDocument> {
        let id = fabryk_core::util::ids::id_from_path(file_path)
            .ok_or_else(|| fabryk_core::Error::parse("no file stem"))?;

        let title = frontmatter
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or(&id);

        let category = frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Compose text: title | content (trimmed)
        let text = format!("{} | {}", title, content.trim());

        let mut doc = VectorDocument::new(id, text);
        if let Some(cat) = category {
            doc = doc.with_category(cat);
        }

        // Extract any additional metadata from frontmatter
        if let Some(tier) = frontmatter.get("tier").and_then(|v| v.as_str()) {
            doc = doc.with_metadata("tier", tier);
        }

        Ok(doc)
    }

    fn name(&self) -> &str {
        "mock"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_frontmatter() -> yaml_serde::Value {
        yaml_serde::from_str(
            r#"
title: "Test Concept"
category: "test-category"
tier: "beginner"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_mock_extractor_extract_document() {
        let extractor = MockVectorExtractor;
        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/test-concept.md");
        let frontmatter = sample_frontmatter();

        let doc = extractor
            .extract_document(&base_path, &file_path, &frontmatter, "Body content here.")
            .unwrap();

        assert_eq!(doc.id, "test-concept");
        assert!(doc.text.contains("Test Concept"));
        assert!(doc.text.contains("Body content here."));
        assert_eq!(doc.category, Some("test-category".to_string()));
        assert_eq!(doc.metadata.get("tier").unwrap(), "beginner");
    }

    #[test]
    fn test_mock_extractor_minimal_frontmatter() {
        let extractor = MockVectorExtractor;
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/simple.md");
        let frontmatter: yaml_serde::Value = yaml_serde::from_str("title: Simple").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &frontmatter, "Content")
            .unwrap();

        assert_eq!(doc.id, "simple");
        assert_eq!(doc.text, "Simple | Content");
        assert!(doc.category.is_none());
        assert!(doc.metadata.is_empty());
    }

    #[test]
    fn test_mock_extractor_no_title() {
        let extractor = MockVectorExtractor;
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/no-title.md");
        let frontmatter: yaml_serde::Value = yaml_serde::from_str("category: test").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &frontmatter, "Content")
            .unwrap();

        // Falls back to file stem as title in text
        assert!(doc.text.contains("no-title"));
    }

    #[test]
    fn test_mock_extractor_defaults() {
        let extractor = MockVectorExtractor;
        assert_eq!(extractor.content_glob(), "**/*.md");
        assert_eq!(extractor.name(), "mock");
    }

    #[test]
    fn test_trait_object_safety() {
        fn _assert_object_safe(_: &dyn VectorExtractor) {}
    }
}
