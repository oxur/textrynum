//! Content extraction helper utilities.
//!
//! This module provides utilities for extracting structured data from markdown
//! content beyond basic parsing. These helpers are used by `GraphExtractor`
//! implementations to extract relationship lists and other structured content.
//!
//! # Key Functions
//!
//! - [`extract_list_from_section`]: Extract items from a list under a heading
//! - [`extract_section_content`]: Get all content under a heading
//! - [`parse_keyword_list`]: Parse "**Keyword**: item1, item2" format
//!
//! # Example
//!
//! ```rust
//! use fabryk_content::markdown::helpers::extract_list_from_section;
//!
//! let content = "## Related Concepts\n\n- **Prerequisite**: concept-a, concept-b\n- **See also**: concept-c\n";
//!
//! let prereqs = extract_list_from_section(content, "Related Concepts", "Prerequisite");
//! assert_eq!(prereqs, vec!["concept-a", "concept-b"]);
//! ```

use regex::Regex;

/// Extract a list of items from a named section of a markdown document.
///
/// Looks for a heading matching `section_heading`, then within that section
/// finds a list item matching `keyword`, and returns the comma-separated
/// values.
///
/// # Arguments
///
/// * `content` - Markdown content to search
/// * `section_heading` - The heading text to find (without `##` prefix)
/// * `keyword` - The keyword to look for in the list (without `**` markers)
///
/// # Returns
///
/// A vector of trimmed string values. Returns empty vec if section or keyword
/// not found.
///
/// # Format Expected
///
/// ```markdown
/// ## Section Heading
///
/// - **Keyword**: value1, value2, value3
/// - **Other**: something-else
/// ```
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::extract_list_from_section;
///
/// let content = "## Related Concepts\n\n- **Prerequisite**: major-triad, minor-key\n- **See also**: borrowed-chords\n";
///
/// let prereqs = extract_list_from_section(content, "Related Concepts", "Prerequisite");
/// assert_eq!(prereqs, vec!["major-triad", "minor-key"]);
///
/// let see_also = extract_list_from_section(content, "Related Concepts", "See also");
/// assert_eq!(see_also, vec!["borrowed-chords"]);
///
/// // Non-existent section returns empty vec
/// let missing = extract_list_from_section(content, "Missing", "Anything");
/// assert!(missing.is_empty());
/// ```
pub fn extract_list_from_section(
    content: &str,
    section_heading: &str,
    keyword: &str,
) -> Vec<String> {
    // Find the section
    let section_content = match extract_section_content(content, section_heading) {
        Some(s) => s,
        None => return Vec::new(),
    };

    // Parse keyword list items in the section
    parse_keyword_list(&section_content, keyword)
}

/// Extract all content under a section heading until the next heading.
///
/// # Arguments
///
/// * `content` - Markdown content to search
/// * `section_heading` - The heading text to find (without `##` prefix)
///
/// # Returns
///
/// The content under the heading, or `None` if not found.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::extract_section_content;
///
/// let content = "## Introduction\n\nThis is the intro.\n\n## Details\n\nMore details here.\n";
///
/// let intro = extract_section_content(content, "Introduction").unwrap();
/// assert!(intro.contains("This is the intro"));
/// assert!(!intro.contains("More details"));
/// ```
pub fn extract_section_content(content: &str, section_heading: &str) -> Option<String> {
    let section_heading_re =
        Regex::new(r"^#{2,6}\s+(.+)$").expect("Invalid section heading regex");

    let heading_lower = section_heading.to_lowercase();
    let lines: Vec<&str> = content.lines().collect();

    let mut in_section = false;
    let mut section_lines = Vec::new();
    let mut section_level = 0;

    for line in lines {
        if let Some(caps) = section_heading_re.captures(line) {
            let current_heading = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let current_level = line.chars().take_while(|&c| c == '#').count();

            if current_heading.to_lowercase().trim() == heading_lower.trim() {
                // Found the target section
                in_section = true;
                section_level = current_level;
                continue;
            } else if in_section && current_level <= section_level {
                // Hit a same-level or higher heading, end of section
                break;
            }
        }

        if in_section {
            section_lines.push(line);
        }
    }

    if section_lines.is_empty() {
        None
    } else {
        Some(section_lines.join("\n"))
    }
}

/// Parse keyword list items from content.
///
/// Finds lines matching the pattern `- **Keyword**: value1, value2` and
/// returns the comma-separated values for the specified keyword.
///
/// # Arguments
///
/// * `content` - Content to search (typically a section's content)
/// * `keyword` - The keyword to look for (case-insensitive)
///
/// # Returns
///
/// A vector of trimmed string values.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::parse_keyword_list;
///
/// let content = "- **Tags**: rust, programming, async\n- **Authors**: Alice, Bob\n";
///
/// let tags = parse_keyword_list(content, "Tags");
/// assert_eq!(tags, vec!["rust", "programming", "async"]);
///
/// let authors = parse_keyword_list(content, "authors"); // Case-insensitive
/// assert_eq!(authors, vec!["Alice", "Bob"]);
/// ```
pub fn parse_keyword_list(content: &str, keyword: &str) -> Vec<String> {
    let keyword_list_re =
        Regex::new(r"^\s*[-*]\s+\*\*([^*]+)\*\*:\s*(.+)$").expect("Invalid keyword list regex");

    let keyword_lower = keyword.to_lowercase();

    for line in content.lines() {
        if let Some(caps) = keyword_list_re.captures(line) {
            let line_keyword = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let values = caps.get(2).map(|m| m.as_str()).unwrap_or("");

            if line_keyword.to_lowercase().trim() == keyword_lower.trim() {
                return parse_comma_list(values);
            }
        }
    }

    Vec::new()
}

/// Parse a comma-separated list into trimmed strings.
///
/// Handles various separators: comma, semicolon, " and ", " or ".
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::parse_comma_list;
///
/// let items = parse_comma_list("apple, banana, cherry");
/// assert_eq!(items, vec!["apple", "banana", "cherry"]);
///
/// let items = parse_comma_list("one; two; three");
/// assert_eq!(items, vec!["one", "two", "three"]);
/// ```
pub fn parse_comma_list(input: &str) -> Vec<String> {
    // Primary separator is comma, but also handle semicolon
    let separator_re =
        Regex::new(r"[,;]\s*|\s+and\s+|\s+or\s+").expect("Invalid separator regex");

    separator_re
        .split(input)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Extract all list items from a section (regardless of keyword).
///
/// Returns all items from bullet lists in the section, without parsing
/// the keyword format.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::extract_all_list_items;
///
/// let content = "## Topics\n\n- First item\n- Second item\n- Third item\n";
///
/// let items = extract_all_list_items(content, "Topics");
/// assert_eq!(items.len(), 3);
/// assert_eq!(items[0], "First item");
/// ```
pub fn extract_all_list_items(content: &str, section_heading: &str) -> Vec<String> {
    let section = match extract_section_content(content, section_heading) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let list_item_re = Regex::new(r"^\s*[-*]\s+(.+)$").expect("Invalid list item regex");

    section
        .lines()
        .filter_map(|line| {
            list_item_re
                .captures(line)
                .and_then(|caps| caps.get(1))
                .map(|m| m.as_str().trim().to_string())
        })
        .collect()
}

/// Normalize an ID string: lowercase, replace spaces with dashes.
///
/// This is a convenience function for normalizing extracted IDs to a
/// consistent format.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::helpers::normalize_id;
///
/// assert_eq!(normalize_id("Major Triad"), "major-triad");
/// assert_eq!(normalize_id("  Picardy Third  "), "picardy-third");
/// ```
pub fn normalize_id(id: &str) -> String {
    id.trim().to_lowercase().replace([' ', '_'], "-")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // extract_list_from_section tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_list_basic() {
        let content = r#"
# Concept Card

Description here.

## Related Concepts

- **Prerequisite**: major-triad, minor-key
- **Leads to**: modal-mixture
- **See also**: borrowed-chords
"#;

        let prereqs = extract_list_from_section(content, "Related Concepts", "Prerequisite");
        assert_eq!(prereqs, vec!["major-triad", "minor-key"]);

        let leads_to = extract_list_from_section(content, "Related Concepts", "Leads to");
        assert_eq!(leads_to, vec!["modal-mixture"]);

        let see_also = extract_list_from_section(content, "Related Concepts", "See also");
        assert_eq!(see_also, vec!["borrowed-chords"]);
    }

    #[test]
    fn test_extract_list_case_insensitive() {
        let content = r#"
## Dependencies

- **REQUIRES**: concept-a, concept-b
"#;

        let items = extract_list_from_section(content, "dependencies", "requires");
        assert_eq!(items, vec!["concept-a", "concept-b"]);
    }

    #[test]
    fn test_extract_list_missing_section() {
        let content = "# Just a heading\n\nNo related concepts section.";
        let items = extract_list_from_section(content, "Related Concepts", "Prerequisite");
        assert!(items.is_empty());
    }

    #[test]
    fn test_extract_list_missing_keyword() {
        let content = r#"
## Related Concepts

- **See also**: something
"#;

        let items = extract_list_from_section(content, "Related Concepts", "Prerequisite");
        assert!(items.is_empty());
    }

    #[test]
    fn test_extract_list_multiple_sections() {
        let content = r#"
## Section One

- **Items**: a, b

## Section Two

- **Items**: c, d

## Section Three

Content here.
"#;

        let section_one = extract_list_from_section(content, "Section One", "Items");
        assert_eq!(section_one, vec!["a", "b"]);

        let section_two = extract_list_from_section(content, "Section Two", "Items");
        assert_eq!(section_two, vec!["c", "d"]);
    }

    // ------------------------------------------------------------------------
    // extract_section_content tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_section_basic() {
        let content = r#"
## Intro

This is the intro.

## Details

More details.
"#;

        let intro = extract_section_content(content, "Intro").unwrap();
        assert!(intro.contains("This is the intro"));
        assert!(!intro.contains("More details"));
    }

    #[test]
    fn test_extract_section_with_subsections() {
        let content = r#"
## Main Section

Content.

### Subsection

Subsection content.

## Next Section

Different content.
"#;

        let main = extract_section_content(content, "Main Section").unwrap();
        assert!(main.contains("Content"));
        assert!(main.contains("Subsection content")); // Includes subsections
        assert!(!main.contains("Different content"));
    }

    #[test]
    fn test_extract_section_not_found() {
        let content = "## Other Section\n\nContent.";
        assert!(extract_section_content(content, "Missing").is_none());
    }

    // ------------------------------------------------------------------------
    // parse_keyword_list tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_keyword_list_basic() {
        let content = "- **Tags**: rust, programming, async";
        let tags = parse_keyword_list(content, "Tags");
        assert_eq!(tags, vec!["rust", "programming", "async"]);
    }

    #[test]
    fn test_parse_keyword_list_single_item() {
        let content = "- **Author**: Claude";
        let authors = parse_keyword_list(content, "Author");
        assert_eq!(authors, vec!["Claude"]);
    }

    #[test]
    fn test_parse_keyword_list_asterisk_bullets() {
        let content = "* **Items**: one, two, three";
        let items = parse_keyword_list(content, "Items");
        assert_eq!(items, vec!["one", "two", "three"]);
    }

    // ------------------------------------------------------------------------
    // parse_comma_list tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_comma_list_basic() {
        let items = parse_comma_list("apple, banana, cherry");
        assert_eq!(items, vec!["apple", "banana", "cherry"]);
    }

    #[test]
    fn test_parse_comma_list_semicolon() {
        let items = parse_comma_list("one; two; three");
        assert_eq!(items, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_parse_comma_list_and_or() {
        let items = parse_comma_list("red and blue or green");
        assert_eq!(items, vec!["red", "blue", "green"]);
    }

    #[test]
    fn test_parse_comma_list_whitespace() {
        let items = parse_comma_list("  item1  ,  item2  ,  item3  ");
        assert_eq!(items, vec!["item1", "item2", "item3"]);
    }

    #[test]
    fn test_parse_comma_list_empty() {
        let items = parse_comma_list("");
        assert!(items.is_empty());
    }

    // ------------------------------------------------------------------------
    // extract_all_list_items tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_all_list_items_basic() {
        let content = r#"
## Topics

- First item
- Second item
- Third item

## Other
"#;

        let items = extract_all_list_items(content, "Topics");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "First item");
        assert_eq!(items[2], "Third item");
    }

    #[test]
    fn test_extract_all_list_items_with_formatting() {
        let content = r#"
## List

- **Bold item**
- *Italic item*
- `Code item`
"#;

        let items = extract_all_list_items(content, "List");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "**Bold item**");
    }

    // ------------------------------------------------------------------------
    // normalize_id tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_normalize_id_spaces() {
        assert_eq!(normalize_id("Major Triad"), "major-triad");
    }

    #[test]
    fn test_normalize_id_underscores() {
        assert_eq!(normalize_id("some_concept"), "some-concept");
    }

    #[test]
    fn test_normalize_id_mixed() {
        assert_eq!(normalize_id("  Mixed_Case Concept  "), "mixed-case-concept");
    }

    #[test]
    fn test_normalize_id_already_normalized() {
        assert_eq!(normalize_id("already-normalized"), "already-normalized");
    }

    // ------------------------------------------------------------------------
    // Real-world example tests (from GraphExtractor usage)
    // ------------------------------------------------------------------------

    #[test]
    fn test_music_theory_related_concepts() {
        let content = r#"
---
concept: Picardy Third
category: chromaticism
---

# Picardy Third

A Picardy third is a major chord used at the end of a piece in minor key.

## Related Concepts

- **Prerequisite**: major-triad, minor-key
- **Leads to**: modal-mixture
- **See also**: borrowed-chords
"#;

        let prereqs = extract_list_from_section(content, "Related Concepts", "Prerequisite");
        assert_eq!(prereqs, vec!["major-triad", "minor-key"]);
    }

    #[test]
    fn test_math_dependencies() {
        let content = r#"
# Fundamental Theorem of Calculus

## Dependencies

- **Requires**: derivative-definition, integral-definition, continuous-functions
- **Implies**: antiderivative-existence
- **Related**: mean-value-theorem, integration-by-parts
"#;

        let requires = extract_list_from_section(content, "Dependencies", "Requires");
        assert_eq!(
            requires,
            vec![
                "derivative-definition",
                "integral-definition",
                "continuous-functions"
            ]
        );

        let implies = extract_list_from_section(content, "Dependencies", "Implies");
        assert_eq!(implies, vec!["antiderivative-existence"]);
    }

    #[test]
    fn test_rust_learning_path() {
        let content = r#"
# Async/Await Syntax

## Learning Path

- **Must know first**: futures, polling, pinning
- **Unlocks**: async-fn, async-block, tokio
- **See also**: threads, channels
"#;

        let must_know = extract_list_from_section(content, "Learning Path", "Must know first");
        assert_eq!(must_know, vec!["futures", "polling", "pinning"]);

        let unlocks = extract_list_from_section(content, "Learning Path", "Unlocks");
        assert_eq!(unlocks, vec!["async-fn", "async-block", "tokio"]);
    }
}
