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
    Field, Schema, SchemaBuilder, TextFieldIndexing, TextOptions, FAST, STORED, STRING,
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
    /// Unique document identifier.
    pub id: Field,
    /// File path for reference.
    pub path: Field,

    // Full-text fields
    /// Document title (boosted 3.0x in search).
    pub title: Field,
    /// Brief description (boosted 2.0x in search).
    pub description: Field,
    /// Main content body (boosted 1.0x in search).
    pub content: Field,

    // Facet fields
    /// Content category for filtering.
    pub category: Field,
    /// Origin/source reference.
    pub source: Field,
    /// Tags for multi-facet filtering.
    pub tags: Field,

    // Metadata fields
    /// Chapter reference.
    pub chapter: Field,
    /// Part/section within source.
    pub part: Field,
    /// Content author.
    pub author: Field,
    /// Publication/creation date.
    pub date: Field,
    /// Type classification (card, chapter, etc.).
    pub content_type: Field,
    /// Specific section reference.
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
