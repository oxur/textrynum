//! Unified metadata extraction from Markdown files with YAML frontmatter.
//!
//! This module provides a single, generic extraction pipeline that handles all
//! content types (concept cards, source chapters, unified concepts, guides).
//! Metadata is resolved through a three-tier precedence chain:
//!
//! 1. **Frontmatter** -- explicit YAML fields take highest priority.
//! 2. **Markdown structure** -- first heading, etc., used as fallback.
//! 3. **Filesystem** -- filename-derived ID, parent-directory category.
//!
//! # Example
//!
//! ```rust,no_run
//! use std::path::Path;
//! use fabryk_content::metadata::{ContentType, extract_metadata};
//!
//! # async fn example() -> Result<(), fabryk_core::Error> {
//! let base = Path::new("/content/concept-cards");
//! let file = Path::new("/content/concept-cards/harmony/tritone.md");
//! let meta = extract_metadata(base, file, ContentType::ConceptCard).await?;
//! assert_eq!(meta.id, "tritone");
//! assert_eq!(meta.category, "harmony");
//! # Ok(())
//! # }
//! ```

use std::fmt;
use std::path::Path;

use crate::concept_card::ConceptCardFrontmatter;

// ---------------------------------------------------------------------------
// ContentType
// ---------------------------------------------------------------------------

/// The kind of content a Markdown file represents.
///
/// Each variant maps to a directory convention in the content tree and may
/// influence title/section fallback behaviour during extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// A standalone concept card (e.g. `concept-cards/harmony/tritone.md`).
    ConceptCard,
    /// A chapter extracted from a source work (e.g. `sources-md/jazz-theory/ch01.md`).
    SourceChapter,
    /// A unified concept that synthesises multiple sources.
    UnifiedConcept,
    /// An instructional guide or tutorial.
    Guide,
}

impl ContentType {
    /// Return a short, human-readable label for this content type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ConceptCard => "concept_card",
            Self::SourceChapter => "source_chapter",
            Self::UnifiedConcept => "unified_concept",
            Self::Guide => "guide",
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// ContentMetadata
// ---------------------------------------------------------------------------

/// Extracted metadata for a single content file.
///
/// Fields are populated from YAML frontmatter first, with fallbacks to
/// Markdown structure and filesystem-derived values.
#[derive(Debug, Clone)]
pub struct ContentMetadata {
    // -- Core identity -------------------------------------------------------
    /// Machine-friendly identifier derived from the filename stem.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Top-level category (frontmatter or parent directory).
    pub category: String,
    /// The kind of content this file represents.
    pub content_type: ContentType,

    // -- Provenance ----------------------------------------------------------
    pub source: Option<String>,
    pub chapter: Option<String>,
    pub section: Option<String>,
    pub part: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub date: Option<String>,
    pub slug: Option<String>,
    pub subcategory: Option<String>,
    pub tier: Option<String>,
    pub source_slug: Option<String>,
    pub extraction_confidence: Option<String>,

    // -- Relationships -------------------------------------------------------
    pub aliases: Vec<String>,
    pub prerequisites: Vec<String>,
    pub extends: Vec<String>,
    pub related: Vec<String>,
    pub contrasts_with: Vec<String>,
    pub answers_questions: Vec<String>,

    // -- Bibliographic -------------------------------------------------------
    pub chapter_number: Option<i32>,
    pub pdf_page: Option<i32>,
    pub authors: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Extract [`ContentMetadata`] from a Markdown file.
///
/// `base_path` is the root directory of the content tree (used to derive
/// a relative category when frontmatter lacks one).  `file_path` is the
/// absolute or relative path to the Markdown file.  `content_type` governs
/// minor behavioural differences in title/section fallback logic.
pub async fn extract_metadata(
    base_path: &Path,
    file_path: &Path,
    content_type: ContentType,
) -> Result<ContentMetadata, fabryk_core::Error> {
    let content = fabryk_core::util::files::read_file(file_path).await?;
    extract_metadata_from_content(base_path, file_path, content_type, &content)
}

/// Extract metadata from already-loaded content (non-async helper used by
/// tests and callers that already have the file contents in memory).
fn extract_metadata_from_content(
    base_path: &Path,
    file_path: &Path,
    content_type: ContentType,
    content: &str,
) -> Result<ContentMetadata, fabryk_core::Error> {
    // -- Frontmatter ---------------------------------------------------------
    let fm_result = crate::extract_frontmatter(content)?;
    let fm: ConceptCardFrontmatter = fm_result
        .deserialize()?
        .unwrap_or_default();

    // -- ID (always from filename) -------------------------------------------
    let id = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // -- Title (precedence depends on content type) --------------------------
    let title = resolve_title(&fm, content, &id, content_type);

    // -- Category ------------------------------------------------------------
    let category = fm
        .category
        .clone()
        .unwrap_or_else(|| extract_category_from_path(base_path, file_path));

    // -- Section (only for SourceChapter and Guide) --------------------------
    let section = match content_type {
        ContentType::SourceChapter | ContentType::Guide => fm.section.clone(),
        _ => None,
    };

    Ok(ContentMetadata {
        id,
        title,
        category,
        content_type,
        source: fm.source,
        chapter: fm.chapter,
        section,
        part: fm.part,
        description: fm.description,
        tags: fm.tags,
        author: fm.author,
        date: fm.date,
        slug: fm.slug,
        subcategory: fm.subcategory,
        tier: fm.tier,
        source_slug: fm.source_slug,
        extraction_confidence: fm.extraction_confidence,
        aliases: fm.aliases,
        prerequisites: fm.prerequisites,
        extends: fm.extends,
        related: fm.related,
        contrasts_with: fm.contrasts_with,
        answers_questions: fm.answers_questions,
        chapter_number: fm.chapter_number,
        pdf_page: fm.pdf_page,
        authors: fm.authors,
    })
}

/// Detect [`ContentType`] from a file path by inspecting ancestor directory
/// names.
///
/// Returns `None` if no recognised pattern is found.
///
/// Recognised directory patterns:
///
/// | Pattern                          | ContentType     |
/// |----------------------------------|-----------------|
/// | `concept-cards`, `concept_cards` | ConceptCard     |
/// | `sources-md`, `sources_md`       | SourceChapter   |
/// | `unified`, `unified-concepts`    | UnifiedConcept  |
/// | `guides`, `guide`                | Guide           |
pub fn detect_content_type(path: &Path) -> Option<ContentType> {
    for component in path.components() {
        let s = component.as_os_str().to_str()?;
        match s {
            "concept-cards" | "concept_cards" => return Some(ContentType::ConceptCard),
            "sources-md" | "sources_md" => return Some(ContentType::SourceChapter),
            "unified" | "unified-concepts" | "unified_concepts" => {
                return Some(ContentType::UnifiedConcept);
            }
            "guides" | "guide" => return Some(ContentType::Guide),
            _ => {}
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Derive a category from the filesystem path.
///
/// Returns the first directory component after `base`, or `"uncategorized"`
/// if the file sits directly inside `base` (or if path stripping fails).
pub fn extract_category_from_path(base: &Path, file: &Path) -> String {
    file.strip_prefix(base)
        .ok()
        .and_then(|rel| rel.parent())
        .and_then(|parent| parent.components().next())
        .and_then(|c| c.as_os_str().to_str())
        .map(String::from)
        .unwrap_or_else(|| "uncategorized".to_string())
}

/// Resolve a human-readable title using the three-tier fallback chain.
///
/// For [`ContentType::SourceChapter`] the `chapter` frontmatter field is
/// tried before falling back to the first Markdown heading.
fn resolve_title(
    fm: &ConceptCardFrontmatter,
    content: &str,
    id: &str,
    content_type: ContentType,
) -> String {
    if let Some(ref t) = fm.title {
        return t.clone();
    }
    if let Some(ref c) = fm.concept {
        return c.clone();
    }
    // SourceChapter additionally tries `chapter` before heading fallback.
    if content_type == ContentType::SourceChapter {
        if let Some(ref ch) = fm.chapter {
            return ch.clone();
        }
    }
    if let Some((_level, heading)) = crate::extract_first_heading(content) {
        return heading;
    }
    humanize_id(id)
}

/// Convert a kebab-case or snake_case identifier into a title-cased string.
fn humanize_id(id: &str) -> String {
    id.split(|c: char| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // -----------------------------------------------------------------------
    // extract_category_from_path
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_category_from_path_with_subdirectory() {
        let base = Path::new("/content/concept-cards");
        let file = Path::new("/content/concept-cards/harmony/tritone.md");
        assert_eq!(extract_category_from_path(base, file), "harmony");
    }

    #[test]
    fn test_extract_category_from_path_nested() {
        let base = Path::new("/data");
        let file = Path::new("/data/rhythm/subdivisions/triplets.md");
        // First component after base is "rhythm"
        assert_eq!(extract_category_from_path(base, file), "rhythm");
    }

    #[test]
    fn test_extract_category_from_path_file_at_root() {
        let base = Path::new("/content");
        let file = Path::new("/content/standalone.md");
        assert_eq!(extract_category_from_path(base, file), "uncategorized");
    }

    #[test]
    fn test_extract_category_from_path_unrelated() {
        let base = Path::new("/other");
        let file = Path::new("/content/harmony/chord.md");
        assert_eq!(extract_category_from_path(base, file), "uncategorized");
    }

    // -----------------------------------------------------------------------
    // detect_content_type
    // -----------------------------------------------------------------------

    #[test]
    fn test_detect_content_type_concept_cards() {
        let path = PathBuf::from("/data/concept-cards/harmony/tritone.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::ConceptCard));
    }

    #[test]
    fn test_detect_content_type_concept_cards_underscore() {
        let path = PathBuf::from("/data/concept_cards/harmony/tritone.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::ConceptCard));
    }

    #[test]
    fn test_detect_content_type_sources_md() {
        let path = PathBuf::from("/data/sources-md/jazz-theory/ch01.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::SourceChapter));
    }

    #[test]
    fn test_detect_content_type_sources_md_underscore() {
        let path = PathBuf::from("/data/sources_md/book/ch02.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::SourceChapter));
    }

    #[test]
    fn test_detect_content_type_unified() {
        let path = PathBuf::from("/data/unified/concepts/interval.md");
        assert_eq!(
            detect_content_type(&path),
            Some(ContentType::UnifiedConcept)
        );
    }

    #[test]
    fn test_detect_content_type_unified_concepts() {
        let path = PathBuf::from("/data/unified-concepts/interval.md");
        assert_eq!(
            detect_content_type(&path),
            Some(ContentType::UnifiedConcept)
        );
    }

    #[test]
    fn test_detect_content_type_guides() {
        let path = PathBuf::from("/data/guides/getting-started.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::Guide));
    }

    #[test]
    fn test_detect_content_type_guide_singular() {
        let path = PathBuf::from("/data/guide/intro.md");
        assert_eq!(detect_content_type(&path), Some(ContentType::Guide));
    }

    #[test]
    fn test_detect_content_type_unknown() {
        let path = PathBuf::from("/data/random/file.md");
        assert_eq!(detect_content_type(&path), None);
    }

    // -----------------------------------------------------------------------
    // extract_metadata (via extract_metadata_from_content)
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_metadata_concept_card() {
        let content = "\
---
title: Tritone Substitution
category: harmony
tags:
  - jazz
  - reharmonization
description: Replacing a dominant chord with one a tritone away
author: Test Author
chapter: Ch 8
section: Basics
---

# Tritone Substitution

Body text here.
";
        let base = Path::new("/content/concept-cards");
        let file = Path::new("/content/concept-cards/harmony/tritone-substitution.md");

        let meta =
            extract_metadata_from_content(base, file, ContentType::ConceptCard, content).unwrap();

        assert_eq!(meta.id, "tritone-substitution");
        assert_eq!(meta.title, "Tritone Substitution");
        assert_eq!(meta.category, "harmony");
        assert_eq!(meta.content_type, ContentType::ConceptCard);
        assert_eq!(meta.tags, vec!["jazz", "reharmonization"]);
        assert_eq!(meta.description.as_deref(), Some("Replacing a dominant chord with one a tritone away"));
        assert_eq!(meta.author.as_deref(), Some("Test Author"));
        // ConceptCard should NOT include section
        assert!(meta.section.is_none());
    }

    #[test]
    fn test_extract_metadata_source_chapter_title_fallback() {
        // No title, no concept, but has chapter -- SourceChapter should use it.
        let content = "\
---
chapter: The Lydian Chromatic Concept
category: theory
section: Part I
---

# Heading In Body
";
        let base = Path::new("/content/sources-md");
        let file = Path::new("/content/sources-md/theory/lydian-chromatic.md");

        let meta =
            extract_metadata_from_content(base, file, ContentType::SourceChapter, content).unwrap();

        assert_eq!(meta.title, "The Lydian Chromatic Concept");
        assert_eq!(meta.section.as_deref(), Some("Part I"));
    }

    #[test]
    fn test_extract_metadata_guide_includes_section() {
        let content = "\
---
title: Getting Started
section: Introduction
---

Body.
";
        let base = Path::new("/content/guides");
        let file = Path::new("/content/guides/getting-started.md");

        let meta =
            extract_metadata_from_content(base, file, ContentType::Guide, content).unwrap();

        assert_eq!(meta.section.as_deref(), Some("Introduction"));
    }

    #[test]
    fn test_extract_metadata_minimal() {
        // No frontmatter at all -- every field falls back to defaults.
        let content = "# A Bare Document\n\nSome text.";
        let base = Path::new("/content");
        let file = Path::new("/content/misc/bare-document.md");

        let meta = extract_metadata_from_content(
            base,
            file,
            ContentType::UnifiedConcept,
            content,
        )
        .unwrap();

        assert_eq!(meta.id, "bare-document");
        assert_eq!(meta.title, "A Bare Document");
        assert_eq!(meta.category, "misc");
        assert!(meta.tags.is_empty());
        assert!(meta.source.is_none());
    }

    #[test]
    fn test_extract_metadata_no_frontmatter_no_heading() {
        let content = "Just plain text with no structure.";
        let base = Path::new("/content");
        let file = Path::new("/content/some-topic.md");

        let meta = extract_metadata_from_content(
            base,
            file,
            ContentType::ConceptCard,
            content,
        )
        .unwrap();

        assert_eq!(meta.id, "some-topic");
        // Should humanize the id
        assert_eq!(meta.title, "Some Topic");
        assert_eq!(meta.category, "uncategorized");
    }

    // -----------------------------------------------------------------------
    // ContentType::as_str / Display
    // -----------------------------------------------------------------------

    #[test]
    fn test_content_type_as_str() {
        assert_eq!(ContentType::ConceptCard.as_str(), "concept_card");
        assert_eq!(ContentType::SourceChapter.as_str(), "source_chapter");
        assert_eq!(ContentType::UnifiedConcept.as_str(), "unified_concept");
        assert_eq!(ContentType::Guide.as_str(), "guide");
    }

    #[test]
    fn test_content_type_display() {
        assert_eq!(format!("{}", ContentType::ConceptCard), "concept_card");
        assert_eq!(format!("{}", ContentType::Guide), "guide");
    }

    // -----------------------------------------------------------------------
    // humanize_id
    // -----------------------------------------------------------------------

    #[test]
    fn test_humanize_id_kebab() {
        assert_eq!(humanize_id("tritone-substitution"), "Tritone Substitution");
    }

    #[test]
    fn test_humanize_id_snake() {
        assert_eq!(humanize_id("tritone_substitution"), "Tritone Substitution");
    }

    #[test]
    fn test_humanize_id_single_word() {
        assert_eq!(humanize_id("harmony"), "Harmony");
    }
}
