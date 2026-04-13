//! Universal concept card frontmatter schema.
//!
//! Defines [`ConceptCardFrontmatter`], the canonical representation of YAML
//! frontmatter found in concept-card Markdown files. The schema is deliberately
//! permissive: unknown fields are silently ignored so that older consumers
//! remain forward-compatible as new fields are introduced.
//!
//! # Usage
//!
//! ```rust
//! use fabryk_content::{extract_frontmatter, ConceptCardFrontmatter};
//!
//! let md = "---\ntitle: Tritone\ncategory: intervals\ntags:\n  - harmony\n---\nBody text";
//! let result = extract_frontmatter(md).unwrap();
//! let meta: ConceptCardFrontmatter = result.deserialize().unwrap().unwrap();
//! assert_eq!(meta.title.as_deref(), Some("Tritone"));
//! ```

use serde::{Deserialize, Serialize};

/// Frontmatter schema for concept-card Markdown files.
///
/// Fields are grouped into logical sections.  All scalar fields are
/// `Option<_>` so that partially-filled cards are still valid.  All
/// collection fields default to empty via `#[serde(default)]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ConceptCardFrontmatter {
    // -- Core identity ----------------------------------------------------------
    /// Human-readable title of the concept card.
    pub title: Option<String>,
    /// Machine-friendly URL slug.
    pub slug: Option<String>,
    /// The concept this card defines or explains.
    pub concept: Option<String>,

    // -- Classification ---------------------------------------------------------
    /// Top-level category (e.g. "harmony", "rhythm").
    pub category: Option<String>,
    /// Narrower classification within the category.
    pub subcategory: Option<String>,
    /// Difficulty or depth tier (e.g. "beginner", "advanced").
    pub tier: Option<String>,

    // -- Content ----------------------------------------------------------------
    /// Short description or summary of the concept.
    pub description: Option<String>,
    /// Free-form tags for search and filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Alternative names or synonyms for the concept.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Questions this concept card is designed to answer.
    #[serde(default)]
    pub answers_questions: Vec<String>,

    // -- Provenance -------------------------------------------------------------
    /// Source publication or reference work.
    pub source: Option<String>,
    /// URL-friendly slug identifying the source.
    pub source_slug: Option<String>,
    /// Confidence score for automated extraction (e.g. "high", "0.95").
    pub extraction_confidence: Option<String>,
    /// Primary author of this card.
    pub author: Option<String>,
    /// Multiple authors, comma-separated.
    pub authors: Option<String>,
    /// Date the card was created or last updated (ISO-8601 preferred).
    pub date: Option<String>,

    // -- Relationships ----------------------------------------------------------
    /// Concepts that should be understood before this one.
    #[serde(default)]
    pub prerequisites: Vec<String>,
    /// Concepts that this card builds upon or extends.
    #[serde(default)]
    pub extends: Vec<String>,
    /// Related concepts for cross-referencing.
    #[serde(default)]
    pub related: Vec<String>,
    /// Concepts that contrast with or are commonly confused with this one.
    #[serde(default)]
    pub contrasts_with: Vec<String>,

    // -- Bibliographic ----------------------------------------------------------
    /// Chapter title in the source work.
    pub chapter: Option<String>,
    /// Section title or identifier.
    pub section: Option<String>,
    /// Part title or identifier.
    pub part: Option<String>,
    /// Numeric chapter index (1-based).
    pub chapter_number: Option<i32>,
    /// Page number in the source PDF.
    pub pdf_page: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extract_frontmatter;

    fn from_yaml(yaml: &str) -> ConceptCardFrontmatter {
        yaml_serde::from_str(yaml).expect("YAML should deserialize")
    }

    #[test]
    fn test_deserialize_full_yaml() {
        let yaml = r#"
title: Tritone Substitution
slug: tritone-substitution
concept: tritone-sub
category: harmony
subcategory: chord-substitution
tier: advanced
description: Replacing a dominant chord with another dominant a tritone away
tags:
  - jazz
  - reharmonization
aliases:
  - tri-tone sub
  - b5 substitution
answers_questions:
  - What is a tritone substitution?
source: The Jazz Theory Book
source_slug: jazz-theory-book
extraction_confidence: "0.95"
author: Mark Levine
authors: Mark Levine
date: "2025-01-15"
prerequisites:
  - dominant-seventh
  - tritone
extends:
  - chord-substitution
related:
  - secondary-dominant
contrasts_with:
  - diatonic-substitution
chapter: Tritone Substitution
section: Basic Concepts
part: Part II
chapter_number: 8
pdf_page: 127
"#;
        let meta = from_yaml(yaml);

        assert_eq!(meta.title.as_deref(), Some("Tritone Substitution"));
        assert_eq!(meta.slug.as_deref(), Some("tritone-substitution"));
        assert_eq!(meta.concept.as_deref(), Some("tritone-sub"));
        assert_eq!(meta.category.as_deref(), Some("harmony"));
        assert_eq!(meta.subcategory.as_deref(), Some("chord-substitution"));
        assert_eq!(meta.tier.as_deref(), Some("advanced"));
        assert_eq!(meta.tags, vec!["jazz", "reharmonization"]);
        assert_eq!(meta.aliases, vec!["tri-tone sub", "b5 substitution"]);
        assert_eq!(
            meta.answers_questions,
            vec!["What is a tritone substitution?"]
        );
        assert_eq!(meta.source.as_deref(), Some("The Jazz Theory Book"));
        assert_eq!(meta.source_slug.as_deref(), Some("jazz-theory-book"));
        assert_eq!(meta.extraction_confidence.as_deref(), Some("0.95"));
        assert_eq!(meta.author.as_deref(), Some("Mark Levine"));
        assert_eq!(meta.authors.as_deref(), Some("Mark Levine"));
        assert_eq!(meta.date.as_deref(), Some("2025-01-15"));
        assert_eq!(meta.prerequisites, vec!["dominant-seventh", "tritone"]);
        assert_eq!(meta.extends, vec!["chord-substitution"]);
        assert_eq!(meta.related, vec!["secondary-dominant"]);
        assert_eq!(meta.contrasts_with, vec!["diatonic-substitution"]);
        assert_eq!(meta.chapter.as_deref(), Some("Tritone Substitution"));
        assert_eq!(meta.section.as_deref(), Some("Basic Concepts"));
        assert_eq!(meta.part.as_deref(), Some("Part II"));
        assert_eq!(meta.chapter_number, Some(8));
        assert_eq!(meta.pdf_page, Some(127));
    }

    #[test]
    fn test_deserialize_minimal_yaml() {
        let yaml = "title: Aeolian Mode\n";
        let meta = from_yaml(yaml);

        assert_eq!(meta.title.as_deref(), Some("Aeolian Mode"));
        assert_eq!(meta.slug, None);
        assert_eq!(meta.concept, None);
        assert_eq!(meta.category, None);
    }

    #[test]
    fn test_default_vec_fields() {
        let meta = ConceptCardFrontmatter::default();

        assert!(meta.tags.is_empty());
        assert!(meta.aliases.is_empty());
        assert!(meta.answers_questions.is_empty());
        assert!(meta.prerequisites.is_empty());
        assert!(meta.extends.is_empty());
        assert!(meta.related.is_empty());
        assert!(meta.contrasts_with.is_empty());
    }

    #[test]
    fn test_unknown_fields_are_ignored() {
        let yaml = "title: Test\nunknown_field: should be ignored\n";
        let meta = from_yaml(yaml);

        assert_eq!(meta.title.as_deref(), Some("Test"));
    }

    #[test]
    fn test_interop_with_extract_frontmatter() {
        let markdown = "\
---
title: Circle of Fifths
category: harmony
tags:
  - key-signatures
  - scales
prerequisites:
  - major-scale
---

The circle of fifths is a visual representation of key relationships.
";
        let result = extract_frontmatter(markdown).expect("frontmatter extraction should succeed");
        assert!(result.has_frontmatter());

        let meta: ConceptCardFrontmatter = result
            .deserialize()
            .expect("deserialization should succeed")
            .expect("frontmatter should be present");

        assert_eq!(meta.title.as_deref(), Some("Circle of Fifths"));
        assert_eq!(meta.category.as_deref(), Some("harmony"));
        assert_eq!(meta.tags, vec!["key-signatures", "scales"]);
        assert_eq!(meta.prerequisites, vec!["major-scale"]);
        assert!(result.body().contains("circle of fifths"));
    }
}
