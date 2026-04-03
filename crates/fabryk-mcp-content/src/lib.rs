//! Content and source MCP tools for Fabryk.
//!
//! This crate provides traits and tools for content item operations:
//!
//! - `ContentItemProvider` — list and retrieve domain content items
//! - `SourceProvider` — access source materials (books, papers)
//! - `ContentTools<P>` — MCP tools backed by a content provider
//! - `SourceTools<P>` — MCP tools for source access
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_content::{ContentTools, SourceTools, ContentItemProvider};
//!
//! // Implement the trait for your domain
//! struct MyContentProvider { /* ... */ }
//!
//! impl ContentItemProvider for MyContentProvider {
//!     type ItemSummary = MyItemSummary;
//!     type ItemDetail = MyItemDetail;
//!     // ... implement methods
//! }
//!
//! // Create MCP tools
//! let tools = ContentTools::new(provider).with_prefix("concepts");
//! ```

pub mod tools;
pub mod traits;

// Re-exports — traits
pub use traits::{CategoryInfo, ChapterInfo, ContentItemProvider, FilterMap, SourceProvider};

// Re-exports — tools
pub use tools::{ContentTools, GetChapterArgs, GetItemArgs, ListItemsArgs, SourceTools};
