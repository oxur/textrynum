//! Default concept card vector extractor.
//!
//! [`ConceptCardVectorExtractor`] implements [`VectorExtractor`] for content
//! that uses the [`ConceptCardFrontmatter`](fabryk_content::ConceptCardFrontmatter)
//! schema.  It composes embedding text from frontmatter metadata and body
//! content for optimal semantic search.
//!
//! This extractor is domain-agnostic: it works for any knowledge domain whose
//! concept cards follow the standard frontmatter schema.

use std::path::Path;

use fabryk_core::Result;

use crate::{VectorDocument, VectorExtractor};

/// Vector extractor for concept card markdown files.
///
/// Composes embedding text from concept card metadata in the format:
///
/// ```text
/// title | category | subcategory | description | aliases | answers_questions | body
/// ```
///
/// This ordering places the most semantically significant tokens (title,
/// category) at the beginning, giving them slightly higher influence in
/// typical embedding models.
#[derive(Debug, Default)]
pub struct ConceptCardVectorExtractor;

impl ConceptCardVectorExtractor {
    /// Create a new vector extractor.
    pub fn new() -> Self {
        Self
    }
}

impl VectorExtractor for ConceptCardVectorExtractor {
    fn extract_document(
        &self,
        _base_path: &Path,
        file_path: &Path,
        frontmatter: &yaml_serde::Value,
        content: &str,
    ) -> Result<VectorDocument> {
        let id = fabryk_core::util::ids::id_from_path(file_path)
            .ok_or_else(|| fabryk_core::Error::parse("cannot derive ID from file path"))?;

        let title = frontmatter
            .get("title")
            .or_else(|| frontmatter.get("concept"))
            .and_then(|v| v.as_str())
            .unwrap_or(&id);

        let category = frontmatter.get("category").and_then(|v| v.as_str());
        let description = frontmatter.get("description").and_then(|v| v.as_str());
        let subcategory = frontmatter.get("subcategory").and_then(|v| v.as_str());

        let aliases = extract_string_array(frontmatter, "aliases");
        let answers_questions = extract_string_array(frontmatter, "answers_questions");

        // Compose embedding text: title | category | subcategory | description |
        // aliases | answers_questions | body
        let mut parts = vec![title.to_string()];
        if let Some(cat) = category {
            parts.push(cat.to_string());
        }
        if let Some(subcat) = subcategory {
            parts.push(subcat.to_string());
        }
        if let Some(desc) = description {
            parts.push(desc.to_string());
        }
        if !aliases.is_empty() {
            parts.push(aliases.join(", "));
        }
        if !answers_questions.is_empty() {
            parts.push(answers_questions.join(" | "));
        }
        parts.push(content.trim().to_string());

        let text = parts.join(" | ");

        let mut doc = VectorDocument::new(id, text);
        if let Some(cat) = category {
            doc = doc.with_category(cat);
        }
        if let Some(source) = frontmatter.get("source").and_then(|v| v.as_str()) {
            doc = doc.with_metadata("source", source);
        }
        if let Some(chapter) = frontmatter.get("chapter").and_then(|v| v.as_str()) {
            doc = doc.with_metadata("chapter", chapter);
        }

        Ok(doc)
    }

    fn content_glob(&self) -> &str {
        "**/*.md"
    }

    fn name(&self) -> &str {
        "concept-card"
    }
}

/// Extract a `Vec<String>` from a YAML frontmatter sequence field.
fn extract_string_array(frontmatter: &yaml_serde::Value, key: &str) -> Vec<String> {
    frontmatter
        .get(key)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
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
title: "Major Scale"
category: "fundamentals"
description: "The most common scale in Western music"
source: "Open Music Theory"
chapter: "Scales and Scale Degrees"
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_full_metadata() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/fundamentals/major-scale.md");
        let fm = sample_frontmatter();
        let body = "The major scale is a diatonic scale with seven notes.";

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, body)
            .unwrap();

        assert_eq!(doc.id, "major-scale");
        assert!(doc.text.contains("Major Scale"));
        assert!(doc.text.contains("fundamentals"));
        assert!(doc.text.contains("The most common scale"));
        assert!(doc.text.contains("diatonic scale"));
        assert_eq!(doc.category, Some("fundamentals".to_string()));
        assert_eq!(doc.metadata.get("source").unwrap(), "Open Music Theory");
        assert_eq!(
            doc.metadata.get("chapter").unwrap(),
            "Scales and Scale Degrees"
        );
    }

    #[test]
    fn test_minimal_metadata() {
        let extractor = ConceptCardVectorExtractor::default();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/simple.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("title: Simple").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "Content")
            .unwrap();

        assert_eq!(doc.id, "simple");
        assert_eq!(doc.text, "Simple | Content");
        assert!(doc.category.is_none());
        assert!(doc.metadata.is_empty());
    }

    #[test]
    fn test_no_title_fallback() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/no-title.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("category: test").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "Body text")
            .unwrap();

        assert!(doc.text.contains("no-title"));
        assert!(doc.text.contains("Body text"));
        assert_eq!(doc.category, Some("test".to_string()));
    }

    #[test]
    fn test_concept_as_title_fallback() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("concept: My Concept").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "Body")
            .unwrap();

        assert!(doc.text.starts_with("My Concept"));
    }

    #[test]
    fn test_text_composition_order() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str(
            r#"
title: "Title"
category: "Category"
description: "Description"
"#,
        )
        .unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "Body")
            .unwrap();

        assert_eq!(doc.text, "Title | Category | Description | Body");
    }

    #[test]
    fn test_text_composition_with_subcategory_aliases_questions() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str(
            r#"
title: "T"
category: "C"
subcategory: "SC"
description: "D"
aliases:
  - "A1"
  - "A2"
answers_questions:
  - "Q1"
  - "Q2"
"#,
        )
        .unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "Body")
            .unwrap();

        assert_eq!(doc.text, "T | C | SC | D | A1, A2 | Q1 | Q2 | Body");
    }

    #[test]
    fn test_trims_body() {
        let extractor = ConceptCardVectorExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("title: T").unwrap();

        let doc = extractor
            .extract_document(&base_path, &file_path, &fm, "  \n  Body  \n  ")
            .unwrap();

        assert_eq!(doc.text, "T | Body");
    }

    #[test]
    fn test_defaults() {
        let extractor = ConceptCardVectorExtractor::new();
        assert_eq!(extractor.content_glob(), "**/*.md");
        assert_eq!(extractor.name(), "concept-card");
    }

    #[test]
    fn test_trait_object_safety() {
        fn _assert_object_safe(_: &dyn VectorExtractor) {}
    }
}
