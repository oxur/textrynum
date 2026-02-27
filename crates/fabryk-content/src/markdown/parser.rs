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
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
