//! Markdown parsing, frontmatter extraction, and content utilities.
//!
//! This crate provides generic content processing utilities used by all Fabryk
//! domains. It has no domain-specific logic — each domain defines its own
//! metadata types and uses these utilities for parsing.
//!
//! # Modules
//!
//! - [`markdown`]: Markdown parsing and frontmatter extraction
//!   - [`markdown::frontmatter`]: YAML frontmatter extraction
//!   - [`markdown::parser`]: Heading, paragraph, text extraction
//!   - [`markdown::helpers`]: List and section extraction
//!
//! # Design Philosophy
//!
//! **Generic utilities, domain-specific types.** This crate provides:
//!
//! - YAML frontmatter extraction → returns `serde_yaml::Value`
//! - Markdown structure parsing → returns strings and enums
//! - Content helpers → returns strings and lists
//!
//! Domain crates (music-theory, math, etc.) define their own structs and
//! deserialize from the generic types.
//!
//! # Example
//!
//! ```rust
//! use fabryk_content::markdown::{extract_frontmatter, FrontmatterResult};
//! use serde::Deserialize;
//!
//! // Your domain-specific frontmatter
//! #[derive(Deserialize)]
//! struct MyMeta {
//!     title: String,
//!     category: Option<String>,
//! }
//!
//! let content = "---\ntitle: Hello\n---\n\nBody";
//! let result = extract_frontmatter(content).unwrap();
//! let meta: Option<MyMeta> = result.deserialize().unwrap();
//! assert_eq!(meta.unwrap().title, "Hello");
//! ```

pub mod markdown;

// Re-export commonly used types
pub use markdown::{
    extract_all_list_items, extract_first_heading, extract_first_paragraph, extract_frontmatter,
    extract_list_from_section, extract_section_content, extract_text_content, normalize_id,
    parse_comma_list, parse_keyword_list, strip_frontmatter, FrontmatterResult,
};

// Re-export HeadingLevel for convenience
pub use pulldown_cmark::HeadingLevel;
