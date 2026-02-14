//! Markdown parsing and frontmatter extraction utilities.
//!
//! This module provides generic utilities for parsing markdown content:
//!
//! - [`frontmatter`]: YAML frontmatter extraction
//! - [`parser`]: Markdown structure parsing (headings, paragraphs)
//! - [`helpers`]: Content extraction helpers (lists, sections)
//!
//! # Design Philosophy
//!
//! These utilities return generic types (`serde_yaml::Value`, `String`) rather
//! than domain-specific structs. Domain crates (music-theory, math, etc.)
//! define their own metadata types and use these utilities to extract raw data.
//!
//! # Example
//!
//! ```rust
//! use fabryk_content::markdown::{extract_frontmatter, FrontmatterResult};
//! use serde::Deserialize;
//!
//! // Domain-specific frontmatter struct
//! #[derive(Deserialize)]
//! struct MyFrontmatter {
//!     title: String,
//!     category: Option<String>,
//! }
//!
//! let content = "---\ntitle: Hello\ncategory: test\n---\n\nBody text";
//! let result = extract_frontmatter(content).unwrap();
//!
//! // Deserialize into domain-specific type
//! let fm: Option<MyFrontmatter> = result.deserialize().unwrap();
//! assert_eq!(fm.unwrap().title, "Hello");
//! ```

pub mod frontmatter;
pub mod helpers;
pub mod parser;

// Re-export key types and functions
pub use frontmatter::{extract_frontmatter, strip_frontmatter, FrontmatterResult};
pub use helpers::{
    extract_all_list_items, extract_list_from_section, extract_section_content, normalize_id,
    parse_comma_list, parse_keyword_list,
};
pub use parser::{extract_first_heading, extract_first_paragraph, extract_text_content};
