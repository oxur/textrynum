---
title: "CC Prompt: Fabryk 3.2 — Default Schema & Stopwords"
milestone: "3.2"
phase: 3
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["3.1 FTS crate scaffold"]
governing-docs: [0011-audit §4.3, 0012-amendment §2d, 0013-project-plan]
---

# CC Prompt: Fabryk 3.2 — Default Schema & Stopwords

## Context

This milestone extracts the Tantivy schema definition and stopword filtering
from music-theory to `fabryk-fts`. Per Amendment §2d, the schema is hardcoded
with sensible defaults — no `SearchSchemaProvider` trait for v0.1.

**Crate:** `fabryk-fts`
**Feature:** `fts-tantivy` (required)
**Risk:** Low

**Key architectural decision (Amendment §2d):**

> For v0.1, `default_schema()` provides a hardcoded schema. Custom schemas can
> be added via `SearchSchemaProvider` in v0.2 only if a domain actually needs
> custom fields.

## Source Files

| File | Lines | Tests | Classification |
|------|-------|-------|----------------|
| `search/schema.rs` | 309 | 10 | G (Fully Generic) |
| `search/stopwords.rs` | 368 | 16 | G (Fully Generic) |

Both files are fully generic — direct extraction with minimal changes.

## Objective

1. Extract `schema.rs` to `fabryk-fts/src/schema.rs`
2. Extract `stopwords.rs` to `fabryk-fts/src/stopwords.rs`
3. Adapt for Fabryk types (config, errors)
4. Preserve all test coverage
5. Verify: `cargo test -p fabryk-fts --features fts-tantivy` passes

## Implementation Steps

### Step 1: Create `fabryk-fts/src/schema.rs`

```rust
//! Tantivy schema definition for full-text search.
//!
//! This module defines the default schema used by Fabryk for indexing knowledge
//! content. The schema is designed to be domain-agnostic, suitable for any
//! knowledge domain (music theory, mathematics, programming, etc.).
//!
//! # Schema Fields
//!
//! The schema has 14 fields organized into four categories:
//!
//! ## Identity Fields
//! - `id`: Unique document identifier (STRING | STORED)
//! - `path`: File path for reference (STORED only)
//!
//! ## Full-Text Fields (searchable with positions for phrase queries)
//! - `title`: Document title (TEXT | STORED), boost 3.0x
//! - `description`: Brief description (TEXT | STORED), boost 2.0x
//! - `content`: Main content body (TEXT | STORED), boost 1.0x
//!
//! ## Facet Fields (filterable)
//! - `category`: Content category (STRING | FAST | STORED)
//! - `source`: Origin/source reference (STRING | FAST | STORED)
//! - `tags`: Comma-separated tags (STRING | STORED)
//!
//! ## Metadata Fields (stored only)
//! - `chapter`: Chapter reference
//! - `part`: Part/section within source
//! - `author`: Content author
//! - `date`: Publication/creation date
//! - `content_type`: Type classification (STRING | FAST | STORED)
//! - `section`: Specific section reference
//!
//! # Tokenizer
//!
//! Uses English stemming tokenizer (`en_stem`) for full-text fields:
//! - SimpleTokenizer → LowerCaser → Stemmer(English)
//!
//! This means "harmonics" matches "harmony", "running" matches "run", etc.

use tantivy::schema::{
    Field, Schema, SchemaBuilder, FAST, STORED, STRING, TEXT, TextFieldIndexing, TextOptions,
};
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, Stemmer, TextAnalyzer};
use tantivy::Index;

/// Schema version for cache invalidation.
///
/// Increment this when schema fields change to force index rebuilds.
pub const SCHEMA_VERSION: u32 = 3;

/// Search schema holding field references and the Tantivy schema.
///
/// This struct provides typed access to schema fields, avoiding string lookups
/// during indexing and querying.
#[derive(Clone)]
pub struct SearchSchema {
    schema: Schema,

    // Identity fields
    pub id: Field,
    pub path: Field,

    // Full-text fields
    pub title: Field,
    pub description: Field,
    pub content: Field,

    // Facet fields
    pub category: Field,
    pub source: Field,
    pub tags: Field,

    // Metadata fields
    pub chapter: Field,
    pub part: Field,
    pub author: Field,
    pub date: Field,
    pub content_type: Field,
    pub section: Field,
}

impl SearchSchema {
    /// Build the default search schema.
    ///
    /// Creates a 14-field schema suitable for any knowledge domain.
    pub fn build() -> Self {
        let mut builder = SchemaBuilder::new();

        // Text field options with positions (for phrase queries)
        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("en_stem")
                    .set_index_option(tantivy::schema::IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        // Identity fields
        let id = builder.add_text_field("id", STRING | STORED);
        let path = builder.add_text_field("path", STORED);

        // Full-text fields (searchable with stemming)
        let title = builder.add_text_field("title", text_options.clone());
        let description = builder.add_text_field("description", text_options.clone());
        let content = builder.add_text_field("content", text_options);

        // Facet fields (filterable, fast for aggregations)
        let category = builder.add_text_field("category", STRING | FAST | STORED);
        let source = builder.add_text_field("source", STRING | FAST | STORED);
        let tags = builder.add_text_field("tags", STRING | STORED);

        // Metadata fields (stored only)
        let chapter = builder.add_text_field("chapter", STORED);
        let part = builder.add_text_field("part", STORED);
        let author = builder.add_text_field("author", STORED);
        let date = builder.add_text_field("date", STORED);

        // v0.3.0 additions
        let content_type = builder.add_text_field("content_type", STRING | FAST | STORED);
        let section = builder.add_text_field("section", STORED);

        let schema = builder.build();

        Self {
            schema,
            id,
            path,
            title,
            description,
            content,
            category,
            source,
            tags,
            chapter,
            part,
            author,
            date,
            content_type,
            section,
        }
    }

    /// Get the underlying Tantivy schema.
    pub fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Register custom tokenizers with a Tantivy index.
    ///
    /// Must be called after creating/opening an index to enable stemming.
    pub fn register_tokenizers(index: &Index) {
        let en_stem = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(tantivy::tokenizer::Language::English))
            .build();

        index.tokenizers().register("en_stem", en_stem);
    }

    /// Get full-text fields with their boost weights.
    ///
    /// Returns fields in order of importance for query building.
    pub fn full_text_fields(&self) -> Vec<(Field, f32)> {
        vec![
            (self.title, 3.0),
            (self.description, 2.0),
            (self.content, 1.0),
        ]
    }

    /// Get facet fields for filtering.
    pub fn facet_fields(&self) -> Vec<Field> {
        vec![self.category, self.source, self.content_type]
    }

    /// Get all fields.
    pub fn all_fields(&self) -> Vec<Field> {
        vec![
            self.id,
            self.path,
            self.title,
            self.description,
            self.content,
            self.category,
            self.source,
            self.tags,
            self.chapter,
            self.part,
            self.author,
            self.date,
            self.content_type,
            self.section,
        ]
    }
}

impl std::fmt::Debug for SearchSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchSchema")
            .field("field_count", &14)
            .field("schema_version", &SCHEMA_VERSION)
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_build() {
        let schema = SearchSchema::build();
        assert_eq!(schema.all_fields().len(), 14);
    }

    #[test]
    fn test_schema_field_names() {
        let schema = SearchSchema::build();
        let tantivy_schema = schema.schema();

        // Verify all expected fields exist
        assert!(tantivy_schema.get_field("id").is_ok());
        assert!(tantivy_schema.get_field("path").is_ok());
        assert!(tantivy_schema.get_field("title").is_ok());
        assert!(tantivy_schema.get_field("description").is_ok());
        assert!(tantivy_schema.get_field("content").is_ok());
        assert!(tantivy_schema.get_field("category").is_ok());
        assert!(tantivy_schema.get_field("source").is_ok());
        assert!(tantivy_schema.get_field("tags").is_ok());
        assert!(tantivy_schema.get_field("chapter").is_ok());
        assert!(tantivy_schema.get_field("part").is_ok());
        assert!(tantivy_schema.get_field("author").is_ok());
        assert!(tantivy_schema.get_field("date").is_ok());
        assert!(tantivy_schema.get_field("content_type").is_ok());
        assert!(tantivy_schema.get_field("section").is_ok());
    }

    #[test]
    fn test_full_text_fields_boost() {
        let schema = SearchSchema::build();
        let fields = schema.full_text_fields();

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].1, 3.0); // title boost
        assert_eq!(fields[1].1, 2.0); // description boost
        assert_eq!(fields[2].1, 1.0); // content boost
    }

    #[test]
    fn test_facet_fields() {
        let schema = SearchSchema::build();
        let fields = schema.facet_fields();

        assert_eq!(fields.len(), 3);
        // category, source, content_type
    }

    #[test]
    fn test_tokenizer_registration() {
        let schema = SearchSchema::build();
        let index = Index::create_in_ram(schema.schema().clone());

        SearchSchema::register_tokenizers(&index);

        // Verify tokenizer exists
        let tokenizer = index.tokenizers().get("en_stem");
        assert!(tokenizer.is_some());
    }

    #[test]
    fn test_schema_version() {
        assert_eq!(SCHEMA_VERSION, 3);
    }

    #[test]
    fn test_schema_debug() {
        let schema = SearchSchema::build();
        let debug = format!("{:?}", schema);
        assert!(debug.contains("SearchSchema"));
        assert!(debug.contains("field_count"));
    }

    #[test]
    fn test_field_types() {
        let schema = SearchSchema::build();
        let tantivy_schema = schema.schema();

        // Check id is STRING (not TEXT)
        let id_entry = tantivy_schema.get_field_entry(schema.id);
        assert!(id_entry.is_indexed());

        // Check path is STORED only
        let path_entry = tantivy_schema.get_field_entry(schema.path);
        assert!(!path_entry.is_indexed());
        assert!(path_entry.is_stored());

        // Check category is FAST
        let category_entry = tantivy_schema.get_field_entry(schema.category);
        assert!(category_entry.is_fast());
    }
}
```

### Step 2: Create `fabryk-fts/src/stopwords.rs`

```rust
//! Stopword filtering for search queries.
//!
//! This module filters common words (stopwords) from search queries to improve
//! search quality. It uses the `stop-words` crate for a comprehensive English
//! stopword list (~500 words) and supports:
//!
//! - Allowlist: Words to preserve even if they're stopwords
//! - Custom stopwords: Additional words to filter
//! - Graceful fallback: If all terms are filtered, returns original query
//!
//! # Domain-Specific Allowlists
//!
//! Some domains have terms that look like stopwords but are meaningful:
//!
//! - Music theory: Roman numerals (I, ii, IV, V, vi, vii)
//! - Music theory: Solfège syllables (do, re, mi, fa, sol, la, ti)
//!
//! Configure allowlists via `SearchConfig::allowlist`.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::stopwords::StopwordFilter;
//! use fabryk_fts::SearchConfig;
//!
//! let config = SearchConfig {
//!     allowlist: vec!["I".to_string(), "V".to_string()],
//!     ..Default::default()
//! };
//!
//! let filter = StopwordFilter::new(&config);
//!
//! // Common words removed
//! assert_eq!(filter.filter("what is a cadence"), "cadence");
//!
//! // Allowlisted words preserved
//! assert_eq!(filter.filter("I V I progression"), "I V I progression");
//! ```

use std::collections::HashSet;
use stop_words::{get, LANGUAGE};

use crate::SearchConfig;

/// Stopword filter for query preprocessing.
///
/// Removes common words while preserving domain-specific terms.
pub struct StopwordFilter {
    stopwords: HashSet<String>,
    allowlist: HashSet<String>,
    enabled: bool,
}

impl StopwordFilter {
    /// Create a new stopword filter from configuration.
    pub fn new(config: &SearchConfig) -> Self {
        let mut stopwords: HashSet<String> = get(LANGUAGE::English)
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Add custom stopwords
        for word in &config.custom_stopwords {
            stopwords.insert(word.to_lowercase());
        }

        // Build allowlist (case-sensitive for proper nouns, numerals)
        let allowlist: HashSet<String> = config.allowlist.iter().cloned().collect();

        Self {
            stopwords,
            allowlist,
            enabled: config.stopwords_enabled,
        }
    }

    /// Create a disabled filter (passes all words through).
    pub fn disabled() -> Self {
        Self {
            stopwords: HashSet::new(),
            allowlist: HashSet::new(),
            enabled: false,
        }
    }

    /// Filter stopwords from a query string.
    ///
    /// Returns the filtered query. If all words are filtered, returns
    /// the original query to avoid empty searches.
    pub fn filter(&self, query: &str) -> String {
        if !self.enabled {
            return query.to_string();
        }

        let filtered: Vec<&str> = query
            .split_whitespace()
            .filter(|word| !self.is_stopword(word))
            .collect();

        if filtered.is_empty() {
            // Fallback: return original if all words filtered
            query.to_string()
        } else {
            filtered.join(" ")
        }
    }

    /// Check if a word is a stopword.
    ///
    /// Returns `false` if the word is in the allowlist (case-sensitive check).
    /// Otherwise, checks the stopword list (case-insensitive).
    pub fn is_stopword(&self, word: &str) -> bool {
        // Allowlist is case-sensitive (for Roman numerals like "I", "V")
        if self.allowlist.contains(word) {
            return false;
        }

        // Stopword check is case-insensitive
        self.stopwords.contains(&word.to_lowercase())
    }

    /// Get the number of stopwords in the filter.
    pub fn stopword_count(&self) -> usize {
        self.stopwords.len()
    }

    /// Check if filtering is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl std::fmt::Debug for StopwordFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StopwordFilter")
            .field("enabled", &self.enabled)
            .field("stopword_count", &self.stopwords.len())
            .field("allowlist_count", &self.allowlist.len())
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_filter() -> StopwordFilter {
        StopwordFilter::new(&SearchConfig::default())
    }

    fn filter_with_allowlist(allowlist: Vec<&str>) -> StopwordFilter {
        let config = SearchConfig {
            allowlist: allowlist.into_iter().map(String::from).collect(),
            ..Default::default()
        };
        StopwordFilter::new(&config)
    }

    // ------------------------------------------------------------------------
    // Basic filtering tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_filter_common_words() {
        let filter = default_filter();
        assert_eq!(filter.filter("what is a cadence"), "cadence");
    }

    #[test]
    fn test_filter_preserves_content_words() {
        let filter = default_filter();
        assert_eq!(filter.filter("harmonic progression"), "harmonic progression");
    }

    #[test]
    fn test_filter_mixed() {
        let filter = default_filter();
        let result = filter.filter("the theory of harmony");
        assert!(result.contains("theory"));
        assert!(result.contains("harmony"));
        assert!(!result.contains("the"));
        assert!(!result.contains("of"));
    }

    // ------------------------------------------------------------------------
    // Allowlist tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_allowlist_roman_numerals() {
        let filter = filter_with_allowlist(vec!["I", "IV", "V", "vi"]);

        // Roman numerals preserved
        assert_eq!(filter.filter("I IV V progression"), "I IV V progression");
        assert_eq!(filter.filter("vi chord"), "vi chord");
    }

    #[test]
    fn test_allowlist_case_sensitive() {
        let filter = filter_with_allowlist(vec!["I", "V"]);

        // Uppercase "I" preserved, lowercase "i" filtered
        let result = filter.filter("I am a musician");
        assert!(result.contains("I"));
        assert!(result.contains("musician"));
    }

    #[test]
    fn test_allowlist_solfege() {
        let filter = filter_with_allowlist(vec!["do", "re", "mi", "fa", "sol", "la", "ti"]);

        assert_eq!(filter.filter("do re mi fa sol"), "do re mi fa sol");
    }

    // ------------------------------------------------------------------------
    // Edge case tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_filter_all_stopwords_fallback() {
        let filter = default_filter();

        // If all words are stopwords, return original
        let original = "the a an";
        assert_eq!(filter.filter(original), original);
    }

    #[test]
    fn test_filter_empty_query() {
        let filter = default_filter();
        assert_eq!(filter.filter(""), "");
    }

    #[test]
    fn test_filter_single_stopword() {
        let filter = default_filter();
        // Single stopword returns original
        assert_eq!(filter.filter("the"), "the");
    }

    #[test]
    fn test_filter_preserves_word_order() {
        let filter = default_filter();
        let result = filter.filter("understand the theory behind music");
        let words: Vec<&str> = result.split_whitespace().collect();

        // Order should be: understand, theory, behind, music
        assert_eq!(words[0], "understand");
        assert!(words.contains(&"theory"));
        assert!(words.contains(&"music"));
    }

    // ------------------------------------------------------------------------
    // Custom stopwords tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_custom_stopwords() {
        let config = SearchConfig {
            custom_stopwords: vec!["foo".to_string(), "bar".to_string()],
            ..Default::default()
        };
        let filter = StopwordFilter::new(&config);

        let result = filter.filter("foo bar baz");
        assert_eq!(result, "baz");
    }

    // ------------------------------------------------------------------------
    // Disabled filter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_disabled_filter() {
        let filter = StopwordFilter::disabled();

        assert_eq!(filter.filter("the a an"), "the a an");
        assert!(!filter.is_enabled());
    }

    #[test]
    fn test_filter_via_config_disabled() {
        let config = SearchConfig {
            stopwords_enabled: false,
            ..Default::default()
        };
        let filter = StopwordFilter::new(&config);

        assert_eq!(filter.filter("the a an"), "the a an");
    }

    // ------------------------------------------------------------------------
    // Stopword count tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_stopword_count() {
        let filter = default_filter();

        // stop-words crate has 500+ English stopwords
        assert!(filter.stopword_count() >= 500);
    }

    #[test]
    fn test_is_stopword() {
        let filter = default_filter();

        assert!(filter.is_stopword("the"));
        assert!(filter.is_stopword("THE")); // Case-insensitive
        assert!(filter.is_stopword("a"));
        assert!(filter.is_stopword("is"));

        assert!(!filter.is_stopword("harmony"));
        assert!(!filter.is_stopword("music"));
    }

    // ------------------------------------------------------------------------
    // Debug formatting
    // ------------------------------------------------------------------------

    #[test]
    fn test_debug_format() {
        let filter = default_filter();
        let debug = format!("{:?}", filter);

        assert!(debug.contains("StopwordFilter"));
        assert!(debug.contains("enabled"));
        assert!(debug.contains("stopword_count"));
    }
}
```

### Step 3: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-fts --features fts-tantivy
cargo test -p fabryk-fts --features fts-tantivy
cargo clippy -p fabryk-fts --features fts-tantivy -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-fts/src/schema.rs` with `SearchSchema` struct
- [ ] Schema has 14 fields matching Amendment §2d specification
- [ ] `SCHEMA_VERSION` constant for cache invalidation
- [ ] `register_tokenizers()` for English stemming
- [ ] `full_text_fields()` returns fields with boost weights
- [ ] `fabryk-fts/src/stopwords.rs` with `StopwordFilter`
- [ ] Stopword filtering with allowlist support
- [ ] Fallback to original if all terms filtered
- [ ] All tests pass (~26 tests total)
- [ ] `cargo clippy` clean

## Design Notes

### Schema field design (Amendment §2d)

The schema fields are intentionally generic:

| Field | Rationale |
|-------|-----------|
| `title` | Every knowledge item has a title |
| `description` | Optional summary, useful for search snippets |
| `content` | Main body text |
| `category` | Hierarchical classification (e.g., "harmony/chords") |
| `source` | Origin reference (book, article, etc.) |
| `tags` | Flexible categorization |
| `content_type` | Classification (concept, chapter, guide, etc.) |

These work for any knowledge domain without customization.

### Stopword allowlist pattern

The allowlist is case-sensitive to handle:

- Roman numerals: "I" (chord degree) vs "i" (pronoun)
- Proper nouns that might be stopwords in lowercase
- Domain abbreviations

Domains configure their allowlists via `SearchConfig::allowlist`.

## Commit Message

```
feat(fts): add default schema and stopword filtering

Extract schema.rs and stopwords.rs from music-theory:

SearchSchema:
- 14-field default schema per Amendment §2d
- English stemming tokenizer (en_stem)
- Field boost weights: title 3.0x, description 2.0x, content 1.0x
- SCHEMA_VERSION=3 for cache invalidation

StopwordFilter:
- stop-words crate integration (500+ English words)
- Configurable allowlist (case-sensitive)
- Custom stopwords support
- Fallback to original if all terms filtered

Both modules are fully generic with no domain-specific logic.
~26 tests for schema and stopword functionality.

Ref: Doc 0013 milestone 3.2, Amendment §2d

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
