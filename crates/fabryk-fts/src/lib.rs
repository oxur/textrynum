//! Full-text search infrastructure for Fabryk.
//!
//! This crate provides search functionality with a Tantivy backend (feature-gated).
//! It includes a default schema suitable for knowledge domains, query building,
//! indexing, and search execution.
//!
//! # Features
//!
//! - `fts-tantivy`: Enable Tantivy-based full-text search (recommended)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      fabryk-fts                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SearchBackend trait                                        │
//! │  ├── SimpleSearch (linear scan fallback)                    │
//! │  └── TantivySearch (full-text with Tantivy)                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SearchSchema (default 14-field schema)                     │
//! │  SearchDocument (indexed document representation)           │
//! │  QueryBuilder (weighted multi-field queries)                │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Indexer (Tantivy index writer)                            │
//! │  IndexBuilder (batch indexing orchestration)               │
//! │  IndexFreshness (content hash validation)                  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Default Schema
//!
//! Per Amendment §2d, Fabryk ships with a sensible default schema suitable for
//! any knowledge domain:
//!
//! | Field | Type | Purpose |
//! |-------|------|---------|
//! | `id` | STRING | Unique identifier |
//! | `path` | STORED | File path |
//! | `title` | TEXT | Full-text, boosted 3.0x |
//! | `description` | TEXT | Full-text, boosted 2.0x |
//! | `content` | TEXT | Full-text, boosted 1.0x |
//! | `category` | STRING | Facet filtering |
//! | `source` | STRING | Facet filtering |
//! | `tags` | STRING | Facet filtering |
//! | `chapter` | STORED | Metadata |
//! | `part` | STORED | Metadata |
//! | `author` | STORED | Metadata |
//! | `date` | STORED | Metadata |
//! | `content_type` | STRING | Content type classification |
//! | `section` | STORED | Section reference |
//!
//! Custom schemas can be added via `SearchSchemaProvider` trait in future
//! versions (v0.2+).
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::{SearchBackend, SearchParams, create_search_backend};
//!
//! // Create backend (uses config to choose Tantivy or SimpleSearch)
//! let backend = create_search_backend(&config).await?;
//!
//! // Execute search
//! let params = SearchParams {
//!     query: "functional harmony".to_string(),
//!     limit: Some(10),
//!     category: Some("harmony".to_string()),
//!     ..Default::default()
//! };
//!
//! let results = backend.search(params).await?;
//! for result in results.items {
//!     println!("{}: {}", result.id, result.title);
//! }
//! ```

// Core modules (always available)
pub mod backend;
pub mod document;
pub mod types;

// Feature-gated Tantivy modules
#[cfg(feature = "fts-tantivy")]
pub mod schema;

#[cfg(feature = "fts-tantivy")]
pub mod query;

#[cfg(feature = "fts-tantivy")]
pub mod indexer;

#[cfg(feature = "fts-tantivy")]
pub mod builder;

#[cfg(feature = "fts-tantivy")]
pub mod freshness;

#[cfg(feature = "fts-tantivy")]
pub mod stopwords;

#[cfg(feature = "fts-tantivy")]
pub mod tantivy_search;

// Re-exports
pub use backend::{SearchBackend, SearchParams, SearchResult, SearchResults};
pub use document::SearchDocument;
pub use types::{QueryMode, SearchConfig};

#[cfg(feature = "fts-tantivy")]
pub use schema::SearchSchema;

#[cfg(feature = "fts-tantivy")]
pub use query::QueryBuilder;

#[cfg(feature = "fts-tantivy")]
pub use indexer::Indexer;

#[cfg(feature = "fts-tantivy")]
pub use builder::{DocumentExtractor, IndexBuilder, IndexStats};

#[cfg(feature = "fts-tantivy")]
pub use freshness::{is_index_fresh, IndexMetadata};

#[cfg(feature = "fts-tantivy")]
pub use stopwords::StopwordFilter;

#[cfg(feature = "fts-tantivy")]
pub use tantivy_search::TantivySearch;

/// Create a search backend based on configuration.
///
/// Returns `TantivySearch` if:
/// - The `fts-tantivy` feature is enabled
/// - An index exists at the configured path
///
/// Otherwise returns `SimpleSearch` as fallback.
pub async fn create_search_backend(
    config: &SearchConfig,
) -> fabryk_core::Result<Box<dyn SearchBackend>> {
    backend::create_search_backend(config).await
}
