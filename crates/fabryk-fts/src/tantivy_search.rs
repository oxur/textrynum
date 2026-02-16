//! Tantivy search backend implementation.
//!
//! Provides `TantivySearch`, the full-featured search backend using Tantivy.
//! This module is only available with the `fts-tantivy` feature.

use std::path::Path;

use async_trait::async_trait;
use fabryk_core::Result;

use crate::backend::{SearchBackend, SearchParams, SearchResults};
use crate::types::SearchConfig;

/// Tantivy-based search backend.
///
/// Provides full-text search with BM25 scoring, fuzzy matching,
/// and faceted filtering.
pub struct TantivySearch {
    #[allow(dead_code)]
    config: SearchConfig,
}

impl TantivySearch {
    /// Create a new TantivySearch from configuration.
    ///
    /// Opens the index at `config.index_path` if it exists.
    pub fn new(_config: &SearchConfig) -> Result<Self> {
        // Stub - will be fully implemented in milestone 3.4+
        // For now, return error since we don't have a real index
        Err(fabryk_core::Error::config(
            "TantivySearch not yet fully implemented",
        ))
    }

    /// Open an existing Tantivy index.
    pub fn open(_index_path: &Path) -> Result<Self> {
        // Stub - will be implemented in milestone 3.4+
        Err(fabryk_core::Error::config(
            "TantivySearch::open not yet implemented",
        ))
    }

    /// Check if an index exists at the given path.
    pub fn index_exists(index_path: &Path) -> bool {
        index_path.join("meta.json").exists()
    }
}

#[async_trait]
impl SearchBackend for TantivySearch {
    async fn search(&self, _params: SearchParams) -> Result<SearchResults> {
        // Stub - will be implemented in milestone 3.3
        todo!("Implement TantivySearch::search in milestone 3.3")
    }

    fn name(&self) -> &str {
        "tantivy"
    }
}
