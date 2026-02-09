---
title: "CC Prompt: Fabryk 2.2 — Markdown Parser"
milestone: "2.2"
phase: 2
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["2.1 Frontmatter extraction"]
governing-docs: [0011-audit §4.2, 0013-project-plan]
---

# CC Prompt: Fabryk 2.2 — Markdown Parser

## Context

This milestone extracts the markdown parsing utilities from music-theory to
`fabryk-content`. The `parser.rs` file is **fully generic** — it contains zero
domain-specific logic and can be extracted as-is with minimal modifications.

**Crate:** `fabryk-content`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Very Low

**Key insight from research:** The `markdown/parser.rs` module (277 lines, 13
tests) is 100% reusable. It provides pure markdown parsing utilities:
- Heading extraction
- Paragraph extraction
- Text content stripping

No music-theory concepts are referenced anywhere in the code.

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion).

## Source Files

**Music-theory source** (via symlink):
```
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/markdown/parser.rs
```

Or directly:
```
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/markdown/parser.rs
```

**From Research:** 277 lines, 13 tests. Key functions: `extract_first_heading()`,
`extract_first_paragraph()`, `extract_text_content()`. Dependencies: `pulldown_cmark`.

**Classification:** Fully Generic (G) — direct extraction.

## Objective

1. Extract `markdown/parser.rs` to `fabryk-content/src/markdown/parser.rs`
2. Update module exports
3. Adapt tests to use `fabryk_content` paths
4. Verify: `cargo test -p fabryk-content` passes

## Implementation Steps

### Step 1: Create `fabryk-content/src/markdown/parser.rs`

```rust
//! Markdown structure parsing utilities.
//!
//! This module provides utilities for extracting structural elements from
//! markdown content using `pulldown-cmark`:
//!
//! - Extract first heading (any level)
//! - Extract first paragraph
//! - Strip formatting to get plain text
//!
//! # Example
//!
//! ```rust
//! use fabryk_content::markdown::parser::{extract_first_heading, extract_first_paragraph};
//! use pulldown_cmark::HeadingLevel;
//!
//! let content = "# My Title\n\nThis is the first paragraph.\n\n## Section";
//!
//! let (level, title) = extract_first_heading(content).unwrap();
//! assert_eq!(level, HeadingLevel::H1);
//! assert_eq!(title, "My Title");
//!
//! let paragraph = extract_first_paragraph(content, 100).unwrap();
//! assert_eq!(paragraph, "This is the first paragraph.");
//! ```

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

/// Extract the first heading from markdown content.
///
/// Returns the heading level and text content. Inline formatting (bold, italic,
/// links) is stripped from the heading text.
///
/// # Arguments
///
/// * `content` - Markdown content to parse
///
/// # Returns
///
/// * `Some((HeadingLevel, String))` - The heading level and text
/// * `None` - If no heading is found
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::parser::extract_first_heading;
/// use pulldown_cmark::HeadingLevel;
///
/// let content = "Some text\n\n## Introduction\n\nMore text";
/// let (level, text) = extract_first_heading(content).unwrap();
/// assert_eq!(level, HeadingLevel::H2);
/// assert_eq!(text, "Introduction");
/// ```
pub fn extract_first_heading(content: &str) -> Option<(HeadingLevel, String)> {
    let parser = Parser::new(content);
    let mut in_heading = false;
    let mut heading_level = HeadingLevel::H1;
    let mut heading_text = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = level;
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                if in_heading && !heading_text.is_empty() {
                    return Some((heading_level, heading_text.trim().to_string()));
                }
                in_heading = false;
            }
            Event::Text(text) | Event::Code(text) if in_heading => {
                heading_text.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak if in_heading => {
                heading_text.push(' ');
            }
            _ => {}
        }
    }

    None
}

/// Extract the first paragraph from markdown content.
///
/// Skips headings and extracts the first actual paragraph content.
/// Inline formatting is stripped. Content is truncated to `max_chars` if needed.
///
/// # Arguments
///
/// * `content` - Markdown content to parse
/// * `max_chars` - Maximum characters to return (truncates with "..." if exceeded)
///
/// # Returns
///
/// * `Some(String)` - The paragraph text
/// * `None` - If no paragraph is found
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::parser::extract_first_paragraph;
///
/// let content = "# Title\n\nThis is a **bold** introduction.\n\nMore content.";
/// let paragraph = extract_first_paragraph(content, 50).unwrap();
/// assert_eq!(paragraph, "This is a bold introduction.");
/// ```
pub fn extract_first_paragraph(content: &str, max_chars: usize) -> Option<String> {
    let parser = Parser::new(content);
    let mut in_paragraph = false;
    let mut paragraph_text = String::new();
    let mut skip_until_heading_end = false;

    for event in parser {
        match event {
            // Skip content inside headings
            Event::Start(Tag::Heading { .. }) => {
                skip_until_heading_end = true;
            }
            Event::End(TagEnd::Heading(_)) => {
                skip_until_heading_end = false;
            }

            // Track paragraph boundaries
            Event::Start(Tag::Paragraph) if !skip_until_heading_end => {
                in_paragraph = true;
                paragraph_text.clear();
            }
            Event::End(TagEnd::Paragraph) if in_paragraph => {
                let trimmed = paragraph_text.trim();
                if !trimmed.is_empty() {
                    return Some(truncate_text(trimmed, max_chars));
                }
                in_paragraph = false;
            }

            // Collect text content
            Event::Text(text) | Event::Code(text) if in_paragraph => {
                paragraph_text.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak if in_paragraph => {
                paragraph_text.push(' ');
            }

            _ => {}
        }
    }

    None
}

/// Extract plain text content from markdown, stripping all formatting.
///
/// Removes headings, bold, italic, links, code blocks, etc. Returns just
/// the text content, suitable for indexing or summarization.
///
/// # Arguments
///
/// * `content` - Markdown content to parse
///
/// # Returns
///
/// Plain text content with formatting removed.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::parser::extract_text_content;
///
/// let content = "# Title\n\nSome **bold** and *italic* text.\n\n```rust\ncode\n```";
/// let text = extract_text_content(content);
/// assert!(text.contains("Title"));
/// assert!(text.contains("Some bold and italic text"));
/// assert!(!text.contains("**"));
/// assert!(!text.contains("*"));
/// ```
pub fn extract_text_content(content: &str) -> String {
    let parser = Parser::new(content);
    let mut text_content = String::new();
    let mut in_code_block = false;

    for event in parser {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Text(text) if !in_code_block => {
                if !text_content.is_empty() && !text_content.ends_with(' ') {
                    text_content.push(' ');
                }
                text_content.push_str(&text);
            }
            Event::Code(text) if !in_code_block => {
                if !text_content.is_empty() && !text_content.ends_with(' ') {
                    text_content.push(' ');
                }
                text_content.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak => {
                if !text_content.is_empty() && !text_content.ends_with(' ') {
                    text_content.push(' ');
                }
            }
            Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Heading(_)) => {
                if !text_content.is_empty() && !text_content.ends_with('\n') {
                    text_content.push('\n');
                }
            }
            _ => {}
        }
    }

    // Clean up multiple spaces and normalize whitespace
    normalize_whitespace(&text_content)
}

/// Truncate text to a maximum length, adding "..." if truncated.
fn truncate_text(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        text.to_string()
    } else {
        // Find a word boundary near max_chars
        let truncate_at = text[..max_chars]
            .rfind(|c: char| c.is_whitespace())
            .unwrap_or(max_chars);

        format!("{}...", text[..truncate_at].trim())
    }
}

/// Normalize whitespace: collapse multiple spaces, trim lines.
fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // extract_first_heading tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_h1_heading() {
        let content = "# Main Title\n\nSome content";
        let (level, text) = extract_first_heading(content).unwrap();
        assert_eq!(level, HeadingLevel::H1);
        assert_eq!(text, "Main Title");
    }

    #[test]
    fn test_extract_h2_heading() {
        let content = "## Section Title\n\nContent here";
        let (level, text) = extract_first_heading(content).unwrap();
        assert_eq!(level, HeadingLevel::H2);
        assert_eq!(text, "Section Title");
    }

    #[test]
    fn test_extract_heading_with_formatting() {
        let content = "# Title with **bold** and *italic*\n\nBody";
        let (level, text) = extract_first_heading(content).unwrap();
        assert_eq!(level, HeadingLevel::H1);
        assert_eq!(text, "Title with bold and italic");
    }

    #[test]
    fn test_extract_heading_with_code() {
        let content = "# Using `Result` in Rust\n\nExplanation";
        let (_, text) = extract_first_heading(content).unwrap();
        assert_eq!(text, "Using Result in Rust");
    }

    #[test]
    fn test_extract_heading_skips_initial_text() {
        let content = "Some introductory text\n\n## First Real Heading\n\nBody";
        let (level, text) = extract_first_heading(content).unwrap();
        assert_eq!(level, HeadingLevel::H2);
        assert_eq!(text, "First Real Heading");
    }

    #[test]
    fn test_extract_heading_no_heading() {
        let content = "Just some paragraph text without any heading.";
        assert!(extract_first_heading(content).is_none());
    }

    // ------------------------------------------------------------------------
    // extract_first_paragraph tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_paragraph_after_heading() {
        let content = "# Title\n\nThis is the first paragraph.\n\nSecond paragraph.";
        let paragraph = extract_first_paragraph(content, 100).unwrap();
        assert_eq!(paragraph, "This is the first paragraph.");
    }

    #[test]
    fn test_extract_paragraph_no_heading() {
        let content = "First paragraph without heading.\n\nSecond paragraph.";
        let paragraph = extract_first_paragraph(content, 100).unwrap();
        assert_eq!(paragraph, "First paragraph without heading.");
    }

    #[test]
    fn test_extract_paragraph_with_formatting() {
        let content = "# Title\n\nThis has **bold** and *italic* formatting.\n\nMore.";
        let paragraph = extract_first_paragraph(content, 100).unwrap();
        assert_eq!(paragraph, "This has bold and italic formatting.");
    }

    #[test]
    fn test_extract_paragraph_truncation() {
        let content = "# Title\n\nThis is a longer paragraph that should be truncated.\n\nMore.";
        let paragraph = extract_first_paragraph(content, 20).unwrap();
        assert!(paragraph.len() <= 23); // 20 + "..."
        assert!(paragraph.ends_with("..."));
    }

    #[test]
    fn test_extract_paragraph_no_paragraph() {
        let content = "# Just a Heading";
        assert!(extract_first_paragraph(content, 100).is_none());
    }

    #[test]
    fn test_extract_paragraph_empty_paragraphs_skipped() {
        let content = "# Title\n\n\n\nActual content.\n\nMore.";
        let paragraph = extract_first_paragraph(content, 100).unwrap();
        assert_eq!(paragraph, "Actual content.");
    }

    // ------------------------------------------------------------------------
    // extract_text_content tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_text_strips_formatting() {
        let content = "# Title\n\nSome **bold** and *italic* text.";
        let text = extract_text_content(content);
        assert!(text.contains("Title"));
        assert!(text.contains("Some bold and italic text"));
        assert!(!text.contains("**"));
        assert!(!text.contains("*"));
    }

    #[test]
    fn test_extract_text_excludes_code_blocks() {
        let content = "# Title\n\nParagraph.\n\n```rust\nfn main() {}\n```\n\nMore text.";
        let text = extract_text_content(content);
        assert!(text.contains("Paragraph"));
        assert!(text.contains("More text"));
        assert!(!text.contains("fn main"));
    }

    #[test]
    fn test_extract_text_preserves_inline_code() {
        let content = "Use the `Result` type for error handling.";
        let text = extract_text_content(content);
        assert!(text.contains("Result"));
    }

    #[test]
    fn test_extract_text_handles_links() {
        let content = "Check out [this link](https://example.com) for more info.";
        let text = extract_text_content(content);
        assert!(text.contains("this link"));
        assert!(!text.contains("https://"));
    }

    #[test]
    fn test_extract_text_normalizes_whitespace() {
        let content = "Multiple   spaces   and\n\n\nnewlines.";
        let text = extract_text_content(content);
        assert!(!text.contains("  ")); // No double spaces
    }

    // ------------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------------

    #[test]
    fn test_empty_content() {
        let content = "";
        assert!(extract_first_heading(content).is_none());
        assert!(extract_first_paragraph(content, 100).is_none());
        assert_eq!(extract_text_content(content), "");
    }

    #[test]
    fn test_only_whitespace() {
        let content = "   \n\n   \n";
        assert!(extract_first_heading(content).is_none());
        assert!(extract_first_paragraph(content, 100).is_none());
    }

    #[test]
    fn test_unicode_content() {
        let content = "# 音楽理論\n\nこれは日本語のテキストです。";
        let (_, title) = extract_first_heading(content).unwrap();
        assert_eq!(title, "音楽理論");

        let para = extract_first_paragraph(content, 100).unwrap();
        assert_eq!(para, "これは日本語のテキストです。");
    }
}
```

### Step 2: Update `fabryk-content/src/markdown/mod.rs`

Add the parser module export:

```rust
//! Markdown parsing and frontmatter extraction utilities.
//!
//! This module provides generic utilities for parsing markdown content:
//!
//! - [`frontmatter`]: YAML frontmatter extraction
//! - [`parser`]: Markdown structure parsing (headings, paragraphs)
//! - [`helpers`]: Content extraction helpers (milestone 2.3)
//!
//! # Design Philosophy
//!
//! These utilities return generic types (`serde_yaml::Value`, `String`) rather
//! than domain-specific structs. Domain crates (music-theory, math, etc.)
//! define their own metadata types and use these utilities to extract raw data.

pub mod frontmatter;
pub mod parser;

// Re-export key types and functions
pub use frontmatter::{extract_frontmatter, strip_frontmatter, FrontmatterResult};
pub use parser::{extract_first_heading, extract_first_paragraph, extract_text_content};

// Modules to be added in subsequent milestones:
// pub mod helpers;  // Milestone 2.3
```

### Step 3: Update `fabryk-content/src/lib.rs`

Add re-exports for parser functions:

```rust
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
//!
//! # Design Philosophy
//!
//! **Generic utilities, domain-specific types.** This crate provides:
//!
//! - YAML frontmatter extraction → returns `serde_yaml::Value`
//! - Markdown structure parsing → returns strings and enums
//! - Content helpers → returns strings
//!
//! Domain crates (music-theory, math, etc.) define their own structs and
//! deserialize from the generic types.

#![doc = include_str!("../README.md")]

pub mod markdown;

// Re-export commonly used types
pub use markdown::{
    extract_first_heading, extract_first_paragraph, extract_frontmatter,
    extract_text_content, strip_frontmatter, FrontmatterResult,
};

// Re-export HeadingLevel for convenience
pub use pulldown_cmark::HeadingLevel;
```

### Step 4: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-content
cargo test -p fabryk-content
cargo clippy -p fabryk-content -- -D warnings
cargo doc -p fabryk-content --no-deps
```

## Exit Criteria

- [ ] `fabryk-content/src/markdown/parser.rs` created
- [ ] Functions exported:
  - `extract_first_heading(content) -> Option<(HeadingLevel, String)>`
  - `extract_first_paragraph(content, max_chars) -> Option<String>`
  - `extract_text_content(content) -> String`
- [ ] Helper functions:
  - `truncate_text()` - truncate with "..."
  - `normalize_whitespace()` - collapse spaces
- [ ] `HeadingLevel` re-exported from `pulldown_cmark` for API convenience
- [ ] Test coverage for:
  - H1, H2 heading extraction
  - Heading with formatting (bold, italic, code)
  - Heading extraction skipping initial text
  - No heading case
  - Paragraph after heading
  - Paragraph with formatting
  - Paragraph truncation
  - Empty paragraph skipping
  - Text content formatting stripping
  - Code block exclusion
  - Inline code preservation
  - Link text extraction
  - Whitespace normalization
  - Unicode content
  - Edge cases (empty, whitespace-only)
- [ ] `cargo test -p fabryk-content` passes (all tests including milestone 2.1)
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean

## Design Notes

### Direct extraction rationale

The music-theory `parser.rs` is **fully generic**:
- No music-theory types referenced
- No domain-specific field names
- Pure markdown parsing using `pulldown_cmark`

This allows direct extraction with only path changes.

### API design decisions

| Decision | Rationale |
|----------|-----------|
| Return `Option` for heading/paragraph | Graceful handling when not found |
| `max_chars` parameter | Allow callers to control truncation |
| Re-export `HeadingLevel` | Avoid forcing users to depend on `pulldown_cmark` |
| Strip formatting in all functions | Consistent, clean text output |

### Comparison with music-theory implementation

| Aspect | Music Theory | Fabryk |
|--------|--------------|--------|
| Functions | Same | Same |
| Implementation | `pulldown_cmark` events | `pulldown_cmark` events |
| Tests | 13 tests | 13+ tests (same coverage) |
| Dependencies | `pulldown_cmark` | `pulldown_cmark` |

The extraction is essentially a copy with updated module paths.

## Commit Message

```
feat(content): extract markdown parser to fabryk-content

Add markdown structure parsing utilities (from music-theory):
- extract_first_heading() - heading level and text
- extract_first_paragraph() - first paragraph with truncation
- extract_text_content() - plain text without formatting
- Re-export HeadingLevel from pulldown_cmark

All functions are fully generic with no domain-specific logic.
Comprehensive test suite (13+ tests).

Ref: Doc 0013 milestone 2.2, Audit §4.2

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
