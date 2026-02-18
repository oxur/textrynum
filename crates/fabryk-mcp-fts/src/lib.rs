//! Full-text search MCP tools for Fabryk.
//!
//! This crate provides MCP tools that delegate to `fabryk-fts` backends.
//!
//! # Tools
//!
//! - `search` — full-text search with category/source filtering
//! - `search_status` — search backend availability
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::create_search_backend;
//! use fabryk_mcp_fts::FtsTools;
//!
//! let backend = create_search_backend(&config).await?;
//! let fts_tools = FtsTools::from_boxed(backend);
//!
//! // Register with composite registry
//! let registry = CompositeRegistry::new().add(fts_tools);
//! ```

pub mod tools;

// Re-exports
pub use tools::{FtsTools, SearchArgs, SearchResponse, SearchResultResponse, SearchStatusResponse};
