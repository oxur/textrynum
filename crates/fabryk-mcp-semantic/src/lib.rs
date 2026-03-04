//! Semantic (hybrid) search MCP tools for Fabryk.
//!
//! This crate provides MCP tools that combine `fabryk-fts` and `fabryk-vector`
//! backends for keyword, vector, and hybrid search.
//!
//! # Tools
//!
//! - `semantic_search` — search using keyword, vector, or hybrid (RRF) mode
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_semantic::SemanticSearchTools;
//!
//! let tools = SemanticSearchTools::new(fts_arc.clone(), Some(vector_arc.clone()));
//!
//! // Register with composite registry
//! let registry = CompositeRegistry::new().add(tools);
//! ```

pub mod tools;

// Re-exports
pub use tools::{HybridResult, SemanticSearchArgs, SemanticSearchTools};
