//! Default concept card document extractor for full-text search.
//!
//! [`ConceptCardDocumentExtractor`] implements [`DocumentExtractor`] (when the
//! `fts-tantivy` feature is enabled) and also exposes inherent methods so that
//! it can be used with the simple search backend.
//!
//! The extractor is domain-agnostic: it works for any knowledge domain that
//! uses the [`ConceptCardFrontmatter`] schema.

use std::path::Path;

use fabryk_content::{ConceptCardFrontmatter, extract_first_heading, extract_frontmatter};

use crate::SearchDocument;

/// Document extractor for concept card markdown files.
///
/// Parses YAML frontmatter via [`fabryk_content::extract_frontmatter`],
/// deserialises into [`ConceptCardFrontmatter`], and maps the result to a
/// [`SearchDocument`] suitable for full-text search indexing.
///
/// # Title Resolution
///
/// The title is resolved in priority order:
///
/// 1. `frontmatter.title`
/// 2. `frontmatter.concept`
/// 3. First markdown heading in the body
/// 4. File stem
///
/// # Category Resolution
///
/// 1. `frontmatter.category`
/// 2. Parent directory name
/// 3. `"general"`
#[derive(Debug, Default)]
pub struct ConceptCardDocumentExtractor;

impl ConceptCardDocumentExtractor {
    /// Create a new extractor.
    pub fn new() -> Self {
        Self
    }

    /// Extract a [`SearchDocument`] from a markdown concept card.
    ///
    /// Returns `None` if the file path has no stem or frontmatter parsing
    /// fails irrecoverably.
    pub fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
        let id = fabryk_core::id_from_path(path)?;

        // Parse frontmatter (gracefully handle failures).
        let result = extract_frontmatter(content).ok()?;
        let fm: ConceptCardFrontmatter = result
            .deserialize()
            .ok()
            .flatten()
            .unwrap_or_default();
        let body = result.body();

        // Title: fm.title -> fm.concept -> first heading -> file stem
        let title = fm
            .title
            .clone()
            .or_else(|| fm.concept.clone())
            .or_else(|| extract_first_heading(body).map(|(_, text)| text))
            .unwrap_or_else(|| id.clone());

        // Category: fm.category -> parent dir name -> "general"
        let category = fm.category.clone().unwrap_or_else(|| {
            path.parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("general")
                .to_string()
        });

        let mut builder = SearchDocument::builder()
            .id(&id)
            .path(path.to_string_lossy())
            .title(&title)
            .category(&category);

        if let Some(ref desc) = fm.description {
            builder = builder.description(desc);
        }
        if let Some(ref source) = fm.source {
            builder = builder.source(source);
        }
        if let Some(ref chapter) = fm.chapter {
            builder = builder.chapter(chapter);
        }
        if let Some(ref part) = fm.part {
            builder = builder.part(part);
        }
        if let Some(ref section) = fm.section {
            builder = builder.section(section);
        }

        // Tags: original tags + tier/subcategory/confidence as facets
        let mut tags = fm.tags.clone();
        if let Some(ref tier) = fm.tier {
            tags.push(format!("tier:{tier}"));
        }
        if let Some(ref subcat) = fm.subcategory {
            tags.push(format!("subcategory:{subcat}"));
        }
        if let Some(ref confidence) = fm.extraction_confidence {
            tags.push(format!("confidence:{confidence}"));
        }
        if !tags.is_empty() {
            builder = builder.tags(tags);
        }

        // Content: body + aliases + competency questions
        let mut enriched_content = body.trim().to_string();
        if !fm.aliases.is_empty() {
            enriched_content.push_str("\n\nAliases: ");
            enriched_content.push_str(&fm.aliases.join(", "));
        }
        if !fm.answers_questions.is_empty() {
            enriched_content.push_str("\n\nCompetency Questions: ");
            enriched_content.push_str(&fm.answers_questions.join(" | "));
        }
        builder = builder.content(&enriched_content);

        // Content type from path
        let content_type = Self::detect_content_type(path);
        builder = builder.content_type(&content_type);

        Some(builder.build())
    }

    /// File extensions this extractor supports (without dots).
    pub fn supported_extensions(&self) -> &[&str] {
        &["md"]
    }

    /// Check if this extractor supports the given file extension.
    pub fn supports_extension(&self, ext: &str) -> bool {
        self.supported_extensions()
            .iter()
            .any(|e| e.eq_ignore_ascii_case(ext))
    }

    /// Detect the content type from the file path.
    ///
    /// Walks ancestor path components looking for known directory names.
    fn detect_content_type(path: &Path) -> String {
        for component in path.components().rev() {
            if let std::path::Component::Normal(name) = component {
                let name = name.to_string_lossy();
                if name.starts_with("concept-cards") || name.starts_with("concept_cards") {
                    return "concept_card".to_string();
                }
                if name.starts_with("sources-md") || name.starts_with("sources_md") {
                    return "source_chapter".to_string();
                }
                if name.starts_with("concepts-unified") || name.starts_with("concepts_unified") {
                    return "unified_concept".to_string();
                }
                if name == "guides" {
                    return "guide".to_string();
                }
            }
        }
        "concept_card".to_string()
    }
}

// When the `fts-tantivy` feature is enabled, implement the trait.
#[cfg(feature = "fts-tantivy")]
impl crate::DocumentExtractor for ConceptCardDocumentExtractor {
    fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
        ConceptCardDocumentExtractor::extract(self, path, content)
    }

    fn supported_extensions(&self) -> &[&str] {
        ConceptCardDocumentExtractor::supported_extensions(self)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_supported_extensions() {
        let extractor = ConceptCardDocumentExtractor::new();
        assert_eq!(extractor.supported_extensions(), &["md"]);
    }

    #[test]
    fn test_supports_extension() {
        let extractor = ConceptCardDocumentExtractor::default();
        assert!(extractor.supports_extension("md"));
        assert!(extractor.supports_extension("MD"));
        assert!(!extractor.supports_extension("txt"));
    }

    #[test]
    fn test_full_frontmatter() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/concept-cards/harmony/major-triad.md");
        let content = r#"---
title: "Major Triad"
description: "A three-note chord built on major and minor thirds"
category: "harmony"
source: "Open Music Theory"
chapter: "Triads and Seventh Chords"
part: "II"
section: "pp. 45-52"
tags: ["chord", "fundamentals"]
---

# Major Triad

The major triad consists of a root, major third, and perfect fifth."#;

        let doc = extractor.extract(&path, content).unwrap();

        assert_eq!(doc.id, "major-triad");
        assert_eq!(doc.title, "Major Triad");
        assert_eq!(doc.category, "harmony");
        assert_eq!(
            doc.description,
            Some("A three-note chord built on major and minor thirds".to_string())
        );
        assert_eq!(doc.source, Some("Open Music Theory".to_string()));
        assert_eq!(doc.chapter, Some("Triads and Seventh Chords".to_string()));
        assert_eq!(doc.part, Some("II".to_string()));
        assert_eq!(doc.section, Some("pp. 45-52".to_string()));
        assert_eq!(doc.tags, vec!["chord", "fundamentals"]);
        assert_eq!(doc.content_type, Some("concept_card".to_string()));
        assert!(doc.content.contains("major triad consists of"));
    }

    #[test]
    fn test_minimal_frontmatter() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/harmony/intervals.md");
        let content = "---\ntitle: Intervals\n---\n\n# Intervals\n\nContent here.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.id, "intervals");
        assert_eq!(doc.title, "Intervals");
        assert_eq!(doc.category, "harmony");
        assert!(doc.description.is_none());
        assert!(doc.source.is_none());
    }

    #[test]
    fn test_no_frontmatter_heading_fallback() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/rhythm/meter.md");
        let content = "# Meter\n\nMeter is the pattern of beats.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.id, "meter");
        assert_eq!(doc.title, "Meter");
        assert_eq!(doc.category, "rhythm");
    }

    #[test]
    fn test_no_title_no_heading_file_stem_fallback() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/general/something.md");
        let content = "Just plain text with no heading or frontmatter.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.title, "something");
        assert_eq!(doc.category, "general");
    }

    #[test]
    fn test_concept_field_as_title_fallback() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/harmony/test.md");
        let content = "---\nconcept: My Concept\n---\nBody.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.title, "My Concept");
    }

    #[test]
    fn test_v3_enrichment() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/concept-cards/acoustics/consonance.md");
        let content = r#"---
title: "Acoustic Consonance"
category: "acoustics"
tier: "foundational"
subcategory: "consonance-dissonance"
extraction_confidence: "high"
aliases:
  - "sensory consonance"
  - "tonal consonance"
answers_questions:
  - "What makes two tones sound consonant?"
  - "How does frequency ratio affect consonance?"
tags: ["acoustics"]
---
The phenomenon of stability."#;

        let doc = extractor.extract(&path, content).unwrap();

        // Tags include enriched facets
        assert!(doc.tags.contains(&"tier:foundational".to_string()));
        assert!(doc
            .tags
            .contains(&"subcategory:consonance-dissonance".to_string()));
        assert!(doc.tags.contains(&"confidence:high".to_string()));
        assert!(doc.tags.contains(&"acoustics".to_string()));

        // Content includes aliases and competency questions
        assert!(doc.content.contains("sensory consonance"));
        assert!(doc.content.contains("tonal consonance"));
        assert!(doc
            .content
            .contains("What makes two tones sound consonant?"));
        assert!(doc.content.contains("The phenomenon of stability."));
    }

    #[test]
    fn test_no_v3_enrichment_for_simple_cards() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/concept-cards/harmony/triad.md");
        let content = "---\ntitle: Triad\ncategory: harmony\n---\nA three-note chord.";

        let doc = extractor.extract(&path, content).unwrap();
        assert!(doc.tags.is_empty());
        assert_eq!(doc.content, "A three-note chord.");
    }

    #[test]
    fn test_content_type_detection() {
        let extractor = ConceptCardDocumentExtractor::new();
        let base = "---\ntitle: T\ncategory: c\n---\nBody";

        let doc = extractor
            .extract(Path::new("/data/concept-cards/x.md"), base)
            .unwrap();
        assert_eq!(doc.content_type, Some("concept_card".to_string()));

        let doc = extractor
            .extract(Path::new("/data/sources-md/x.md"), base)
            .unwrap();
        assert_eq!(doc.content_type, Some("source_chapter".to_string()));

        let doc = extractor
            .extract(Path::new("/data/concepts-unified/x.md"), base)
            .unwrap();
        assert_eq!(doc.content_type, Some("unified_concept".to_string()));

        let doc = extractor
            .extract(Path::new("/data/guides/x.md"), base)
            .unwrap();
        assert_eq!(doc.content_type, Some("guide".to_string()));
    }

    #[test]
    fn test_path_provides_id_and_path() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/deep/nested/path/my-concept.md");
        let content = "---\ntitle: My Concept\ncategory: test\n---\nBody.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.id, "my-concept");
        assert!(doc.path.contains("my-concept.md"));
    }

    #[test]
    fn test_category_falls_back_to_parent_dir() {
        let extractor = ConceptCardDocumentExtractor::new();
        let path = PathBuf::from("/content/counterpoint/voice-leading.md");
        let content = "---\ntitle: Voice Leading\n---\nBody.";

        let doc = extractor.extract(&path, content).unwrap();
        assert_eq!(doc.category, "counterpoint");
    }

    #[test]
    fn test_category_falls_back_to_general() {
        let extractor = ConceptCardDocumentExtractor::new();
        // Root path: no parent directory name
        let path = PathBuf::from("/test.md");
        let content = "---\ntitle: T\n---\nBody.";

        let doc = extractor.extract(&path, content).unwrap();
        // Parent of /test.md is /, which has no file_name, so "general"
        assert_eq!(doc.category, "general");
    }
}
