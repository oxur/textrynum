//! Search document representation.
//!
//! This module defines `SearchDocument`, the struct used to represent indexed
//! content. It maps directly to the schema fields defined in `schema.rs`.
//!
//! # Creating Documents
//!
//! Documents can be created from domain-specific metadata types using the
//! builder pattern or direct construction:
//!
//! ```rust
//! use fabryk_fts::SearchDocument;
//!
//! let doc = SearchDocument::builder()
//!     .id("my-concept")
//!     .title("My Concept")
//!     .content("The main content...")
//!     .category("fundamentals")
//!     .build();
//! ```
//!
//! # Relevance Scoring
//!
//! The document provides a `matches_query()` method for simple substring
//! matching and a `relevance()` method for weighted scoring (used by
//! `SimpleSearch`).

use serde::{Deserialize, Serialize};

/// A document to be indexed and searched.
///
/// This struct holds all fields that can be indexed and searched.
/// It maps to the default schema defined in `schema.rs`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchDocument {
    // Identity
    /// Unique document identifier (required).
    pub id: String,
    /// File path for reference.
    pub path: String,

    // Full-text fields
    /// Document title (required, boosted 3.0x in search).
    pub title: String,
    /// Brief description (boosted 2.0x in search).
    pub description: Option<String>,
    /// Main content body (boosted 1.0x in search).
    pub content: String,

    // Facet fields
    /// Content category (e.g., "harmony", "rhythm").
    pub category: String,
    /// Source reference (e.g., "Open Music Theory").
    pub source: Option<String>,
    /// Tags for additional categorization.
    pub tags: Vec<String>,

    // Metadata fields
    /// Chapter reference.
    pub chapter: Option<String>,
    /// Part within source.
    pub part: Option<String>,
    /// Content author.
    pub author: Option<String>,
    /// Publication/creation date.
    pub date: Option<String>,
    /// Content type classification.
    pub content_type: Option<String>,
    /// Section reference.
    pub section: Option<String>,
}

impl SearchDocument {
    /// Create a new document builder.
    pub fn builder() -> SearchDocumentBuilder {
        SearchDocumentBuilder::default()
    }

    /// Check if the document matches a query (case-insensitive substring match).
    ///
    /// Searches in: title, description, content, category, source, tags.
    pub fn matches_query(&self, query: &str) -> bool {
        if query.is_empty() || query == "*" {
            return true;
        }

        let query_lower = query.to_lowercase();

        self.title.to_lowercase().contains(&query_lower)
            || self
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            || self.content.to_lowercase().contains(&query_lower)
            || self.category.to_lowercase().contains(&query_lower)
            || self
                .source
                .as_ref()
                .map(|s| s.to_lowercase().contains(&query_lower))
                .unwrap_or(false)
            || self
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&query_lower))
    }

    /// Calculate relevance score for a query.
    ///
    /// Uses weighted field matching:
    /// - Title match: 3.0
    /// - Description match: 2.0
    /// - Content match: 1.0
    /// - Category/source/tags: 0.5 each
    pub fn relevance(&self, query: &str) -> f32 {
        if query.is_empty() || query == "*" {
            return 1.0;
        }

        let query_lower = query.to_lowercase();
        let mut score = 0.0;

        if self.title.to_lowercase().contains(&query_lower) {
            score += 3.0;
        }

        if self
            .description
            .as_ref()
            .map(|d| d.to_lowercase().contains(&query_lower))
            .unwrap_or(false)
        {
            score += 2.0;
        }

        if self.content.to_lowercase().contains(&query_lower) {
            score += 1.0;
        }

        if self.category.to_lowercase().contains(&query_lower) {
            score += 0.5;
        }

        if self
            .source
            .as_ref()
            .map(|s| s.to_lowercase().contains(&query_lower))
            .unwrap_or(false)
        {
            score += 0.5;
        }

        if self
            .tags
            .iter()
            .any(|t| t.to_lowercase().contains(&query_lower))
        {
            score += 0.5;
        }

        score
    }

    /// Extract a snippet around the query match.
    ///
    /// Returns a portion of the content centered on the first match,
    /// with ellipsis if truncated.
    pub fn extract_snippet(&self, query: &str, max_length: usize) -> Option<String> {
        if query.is_empty() || query == "*" {
            return self.description.clone().or_else(|| {
                if self.content.len() > max_length {
                    Some(format!("{}...", &self.content[..max_length]))
                } else {
                    Some(self.content.clone())
                }
            });
        }

        let query_lower = query.to_lowercase();

        // Try description first
        if let Some(ref desc) = self.description {
            if let Some(snippet) = find_snippet(desc, &query_lower, max_length) {
                return Some(snippet);
            }
        }

        // Try content
        if let Some(snippet) = find_snippet(&self.content, &query_lower, max_length) {
            return Some(snippet);
        }

        // Fallback to description or content start
        self.description.clone().or_else(|| {
            if self.content.len() > max_length {
                Some(format!("{}...", &self.content[..max_length]))
            } else {
                Some(self.content.clone())
            }
        })
    }

    /// Check if document matches category filter.
    pub fn matches_category(&self, category: &str) -> bool {
        self.category.eq_ignore_ascii_case(category)
    }

    /// Check if document matches source filter.
    pub fn matches_source(&self, source: &str) -> bool {
        self.source
            .as_ref()
            .map(|s| s.eq_ignore_ascii_case(source))
            .unwrap_or(false)
    }

    /// Check if document matches content type filter.
    pub fn matches_content_type(&self, content_type: &str) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.eq_ignore_ascii_case(content_type))
            .unwrap_or(false)
    }
}

/// Find a snippet of text centered around a query match.
fn find_snippet(text: &str, query: &str, max_length: usize) -> Option<String> {
    let text_lower = text.to_lowercase();
    let pos = text_lower.find(query)?;

    // Calculate start position (with context)
    let context = max_length / 4;
    let start = pos.saturating_sub(context);

    // Find word boundary near start
    let start = if start > 0 {
        text[..start]
            .rfind(char::is_whitespace)
            .map(|p| p + 1)
            .unwrap_or(start)
    } else {
        0
    };

    // Calculate end position
    let end = (start + max_length).min(text.len());

    // Find word boundary near end
    let end = if end < text.len() {
        text[end..]
            .find(char::is_whitespace)
            .map(|p| end + p)
            .unwrap_or(end)
    } else {
        text.len()
    };

    // Build snippet with ellipsis
    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(text[start..end].trim());
    if end < text.len() {
        snippet.push_str("...");
    }

    // Normalize whitespace
    let snippet = snippet.split_whitespace().collect::<Vec<_>>().join(" ");

    Some(snippet)
}

/// Builder for SearchDocument.
#[derive(Debug, Default)]
pub struct SearchDocumentBuilder {
    doc: SearchDocument,
}

impl SearchDocumentBuilder {
    /// Set the document ID.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.doc.id = id.into();
        self
    }

    /// Set the file path.
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.doc.path = path.into();
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.doc.title = title.into();
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.doc.description = Some(description.into());
        self
    }

    /// Set the content.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.doc.content = content.into();
        self
    }

    /// Set the category.
    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.doc.category = category.into();
        self
    }

    /// Set the source.
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.doc.source = Some(source.into());
        self
    }

    /// Set the tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.doc.tags = tags;
        self
    }

    /// Set the content type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.doc.content_type = Some(content_type.into());
        self
    }

    /// Set the chapter.
    pub fn chapter(mut self, chapter: impl Into<String>) -> Self {
        self.doc.chapter = Some(chapter.into());
        self
    }

    /// Set the part.
    pub fn part(mut self, part: impl Into<String>) -> Self {
        self.doc.part = Some(part.into());
        self
    }

    /// Set the author.
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.doc.author = Some(author.into());
        self
    }

    /// Set the date.
    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.doc.date = Some(date.into());
        self
    }

    /// Set the section.
    pub fn section(mut self, section: impl Into<String>) -> Self {
        self.doc.section = Some(section.into());
        self
    }

    /// Build the document.
    pub fn build(self) -> SearchDocument {
        self.doc
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_document() -> SearchDocument {
        SearchDocument::builder()
            .id("major-triad")
            .path("/concepts/harmony/major-triad.md")
            .title("Major Triad")
            .description("A major triad is a three-note chord...")
            .content("The major triad consists of a root, major third, and perfect fifth.")
            .category("harmony")
            .source("Open Music Theory")
            .tags(vec!["chord".to_string(), "fundamentals".to_string()])
            .content_type("concept")
            .build()
    }

    // ------------------------------------------------------------------------
    // Builder tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_builder() {
        let doc = sample_document();
        assert_eq!(doc.id, "major-triad");
        assert_eq!(doc.title, "Major Triad");
        assert_eq!(doc.category, "harmony");
        assert!(doc.description.is_some());
    }

    #[test]
    fn test_builder_minimal() {
        let doc = SearchDocument::builder()
            .id("test")
            .title("Test")
            .content("Content")
            .category("test")
            .build();

        assert_eq!(doc.id, "test");
        assert!(doc.description.is_none());
        assert!(doc.source.is_none());
    }

    // ------------------------------------------------------------------------
    // Query matching tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_matches_query_title() {
        let doc = sample_document();
        assert!(doc.matches_query("major"));
        assert!(doc.matches_query("MAJOR")); // Case-insensitive
        assert!(doc.matches_query("triad"));
    }

    #[test]
    fn test_matches_query_content() {
        let doc = sample_document();
        assert!(doc.matches_query("perfect fifth"));
        assert!(doc.matches_query("root"));
    }

    #[test]
    fn test_matches_query_category() {
        let doc = sample_document();
        assert!(doc.matches_query("harmony"));
    }

    #[test]
    fn test_matches_query_tags() {
        let doc = sample_document();
        assert!(doc.matches_query("chord"));
        assert!(doc.matches_query("fundamentals"));
    }

    #[test]
    fn test_matches_query_wildcard() {
        let doc = sample_document();
        assert!(doc.matches_query("*"));
        assert!(doc.matches_query(""));
    }

    #[test]
    fn test_matches_query_no_match() {
        let doc = sample_document();
        assert!(!doc.matches_query("nonexistent"));
    }

    // ------------------------------------------------------------------------
    // Relevance scoring tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_relevance_title_highest() {
        let doc = sample_document();
        let title_score = doc.relevance("major"); // In title
        let content_score = doc.relevance("fifth"); // Only in content

        assert!(title_score > content_score);
    }

    #[test]
    fn test_relevance_description_medium() {
        let doc = sample_document();
        let desc_score = doc.relevance("three-note"); // In description
        let content_score = doc.relevance("fifth"); // In content

        assert!(desc_score > content_score);
    }

    #[test]
    fn test_relevance_wildcard() {
        let doc = sample_document();
        assert_eq!(doc.relevance("*"), 1.0);
        assert_eq!(doc.relevance(""), 1.0);
    }

    // ------------------------------------------------------------------------
    // Snippet extraction tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_snippet_from_content() {
        let doc = sample_document();
        let snippet = doc.extract_snippet("perfect fifth", 100);

        assert!(snippet.is_some());
        let snippet = snippet.unwrap();
        assert!(snippet.contains("perfect fifth"));
    }

    #[test]
    fn test_extract_snippet_ellipsis() {
        let doc = SearchDocument::builder()
            .id("test")
            .title("Test")
            .content("Word word word word word MATCH word word word word word".repeat(3))
            .category("test")
            .build();

        let snippet = doc.extract_snippet("MATCH", 50);
        assert!(snippet.is_some());
        let snippet = snippet.unwrap();
        assert!(snippet.contains("..."));
    }

    #[test]
    fn test_extract_snippet_fallback() {
        let doc = sample_document();
        let snippet = doc.extract_snippet("nonexistent", 100);

        // Should fallback to description
        assert!(snippet.is_some());
    }

    // ------------------------------------------------------------------------
    // Filter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_matches_category() {
        let doc = sample_document();
        assert!(doc.matches_category("harmony"));
        assert!(doc.matches_category("HARMONY")); // Case-insensitive
        assert!(!doc.matches_category("rhythm"));
    }

    #[test]
    fn test_matches_source() {
        let doc = sample_document();
        assert!(doc.matches_source("Open Music Theory"));
        assert!(!doc.matches_source("Other Source"));
    }

    #[test]
    fn test_matches_content_type() {
        let doc = sample_document();
        assert!(doc.matches_content_type("concept"));
        assert!(!doc.matches_content_type("chapter"));
    }

    // ------------------------------------------------------------------------
    // Serialization tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_serialization_roundtrip() {
        let doc = sample_document();
        let json = serde_json::to_string(&doc).unwrap();
        let restored: SearchDocument = serde_json::from_str(&json).unwrap();

        assert_eq!(doc.id, restored.id);
        assert_eq!(doc.title, restored.title);
        assert_eq!(doc.tags, restored.tags);
    }
}
