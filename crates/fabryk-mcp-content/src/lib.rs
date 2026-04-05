//! Content and source MCP tools for Fabryk.
//!
//! This crate provides traits and tools for content item operations:
//!
//! - `ContentItemProvider` — list and retrieve domain content items
//! - `SourceProvider` — access source materials (books, papers)
//! - `GuideProvider` — access guides, tutorials, and reference documents
//! - `QuestionSearchProvider` — search items by competency questions
//! - `ContentTools<P>` — MCP tools backed by a content provider
//! - `SourceTools<P>` — MCP tools for source access
//! - `GuideTools<P>` — MCP tools for guide access
//! - `QuestionSearchTools<P>` — MCP tools for question-based search
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

pub mod fs_content_provider;
pub mod fs_guide_provider;
pub mod tools;
pub mod traits;

// Re-exports — traits
pub use traits::{
    CategoryInfo, ChapterInfo, ContentItemProvider, FilterMap, GuideProvider, QuestionMatch,
    QuestionSearchProvider, QuestionSearchResponse, SourceProvider,
};

// Re-exports — filesystem providers
pub use fs_content_provider::{ContentItemDetail, ContentItemSummary, FsContentItemProvider};
pub use fs_guide_provider::{FsGuideProvider, GuideSummary};

// Re-exports — tools
pub use tools::{
    ContentTools, GetChapterArgs, GetGuideArgs, GetItemArgs, GuideTools, ListItemsArgs,
    QuestionSearchTools, SearchByQuestionArgs, SourceTools,
};
