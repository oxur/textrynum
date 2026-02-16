//! Query building utilities.
//!
//! Provides `QueryBuilder` for constructing weighted multi-field queries.
//! This module is only available with the `fts-tantivy` feature.

use tantivy::query::Query;

use crate::types::QueryMode;

/// Builder for constructing search queries.
///
/// Supports weighted multi-field queries with fuzzy matching
/// and various query modes (AND, OR, Smart).
pub struct QueryBuilder {
    #[allow(dead_code)]
    mode: QueryMode,
}

impl QueryBuilder {
    /// Create a new query builder.
    pub fn new(mode: QueryMode) -> Self {
        Self { mode }
    }

    /// Build a query from the search string.
    pub fn build(&self, _query: &str) -> Box<dyn Query> {
        // Stub - will be implemented in milestone 3.3
        todo!("Implement QueryBuilder in milestone 3.3")
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new(QueryMode::default())
    }
}
