//! Vector search infrastructure for Fabryk.
//!
//! This crate provides semantic vector search with pluggable embedding
//! providers and vector backends. It includes LanceDB and fastembed
//! backends (feature-gated), plus in-memory fallbacks for testing.
//!
//! # Features
//!
//! - `vector-lancedb`: Enable LanceDB-based vector storage and ANN search
//! - `vector-fastembed`: Enable local embedding generation via fastembed
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     fabryk-vector                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  EmbeddingProvider trait                                    │
//! │  ├── MockEmbeddingProvider (always available)               │
//! │  └── FastEmbedProvider (feature: vector-fastembed)          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  VectorBackend trait                                        │
//! │  ├── SimpleVectorBackend (in-memory fallback)               │
//! │  └── LancedbBackend (feature: vector-lancedb)              │
//! ├─────────────────────────────────────────────────────────────┤
//! │  VectorExtractor trait (domain text composition)            │
//! │  VectorIndexBuilder (batch embed + index orchestration)     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Hybrid search (RRF merge with FTS results)                │
//! │  Persistence (content hash freshness checking)              │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_vector::{
//!     VectorSearchParams, VectorBackend,
//!     MockEmbeddingProvider, SimpleVectorBackend,
//! };
//! use std::sync::Arc;
//!
//! let provider = Arc::new(MockEmbeddingProvider::new(384));
//! let backend = SimpleVectorBackend::new(provider);
//!
//! let params = VectorSearchParams::new("semantic query")
//!     .with_limit(10)
//!     .with_category("harmony");
//!
//! let results = backend.search(params).await?;
//! for result in results.items {
//!     println!("{}: {:.3}", result.id, result.score);
//! }
//! ```

// Core modules (always available)
pub mod backend;
pub mod concept_card_extractor;
pub mod embedding;
pub mod probe;
pub mod types;

// Builder and extractor modules (always available)
pub mod builder;
pub mod extractor;

// Hybrid search and persistence (always available)
pub mod hybrid;
pub mod persistence;

// Feature-gated backend modules
#[cfg(feature = "vector-fastembed")]
pub mod fastembed;

#[cfg(feature = "vector-lancedb")]
pub mod lancedb;

// Re-exports — core types
pub use types::{
    BuildError, EmbeddedDocument, VectorConfig, VectorDocument, VectorIndexStats,
    VectorSearchParams, VectorSearchResult, VectorSearchResults,
};

// Re-exports — traits
pub use backend::{SimpleVectorBackend, VectorBackend};
pub use concept_card_extractor::ConceptCardVectorExtractor;
pub use embedding::{EmbeddingProvider, MockEmbeddingProvider};
pub use extractor::VectorExtractor;
pub use probe::{VectorProbe, vector_probe};

// Re-exports — builder
pub use builder::VectorIndexBuilder;

// Re-exports — hybrid search
pub use hybrid::{FtsResult, HybridSearchResult, reciprocal_rank_fusion};

// Re-exports — persistence
pub use persistence::is_index_fresh;

// Re-exports — factory
pub use backend::create_vector_backend;

// Feature-gated re-exports
#[cfg(feature = "vector-fastembed")]
pub use fastembed::FastEmbedProvider;

#[cfg(feature = "vector-lancedb")]
pub use lancedb::LancedbBackend;
