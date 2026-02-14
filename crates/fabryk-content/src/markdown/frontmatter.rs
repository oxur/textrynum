//! YAML frontmatter extraction from markdown files.
//!
//! Frontmatter is metadata at the start of a markdown file, delimited by `---`:
//!
//! ```markdown
//! ---
//! title: My Document
//! category: example
//! tags:
//!   - rust
//!   - markdown
//! ---
//!
//! # Document Content
//!
//! The body of the document starts here.
//! ```
//!
//! # Usage
//!
//! ```rust
//! use fabryk_content::markdown::extract_frontmatter;
//!
//! let content = "---\ntitle: Test\n---\n\nBody";
//! let result = extract_frontmatter(content).unwrap();
//!
//! assert!(result.has_frontmatter());
//! assert_eq!(result.body().trim(), "Body");
//!
//! // Access raw YAML value
//! let title = result.value()
//!     .and_then(|v| v.get("title"))
//!     .and_then(|v| v.as_str());
//! assert_eq!(title, Some("Test"));
//! ```

use fabryk_core::{Error, Result};
use serde::de::DeserializeOwned;
use serde_yaml::Value;

/// Result of frontmatter extraction.
///
/// Contains the parsed YAML value (if present) and the body content
/// after the frontmatter.
#[derive(Debug, Clone)]
pub struct FrontmatterResult<'a> {
    /// Parsed YAML frontmatter, if present and valid.
    value: Option<Value>,
    /// Body content after the frontmatter delimiter.
    body: &'a str,
    /// Whether frontmatter delimiters were found (even if parsing failed).
    had_delimiters: bool,
}

impl<'a> FrontmatterResult<'a> {
    /// Create a result with frontmatter.
    fn with_frontmatter(value: Value, body: &'a str) -> Self {
        Self {
            value: Some(value),
            body,
            had_delimiters: true,
        }
    }

    /// Create a result without frontmatter.
    fn without_frontmatter(body: &'a str) -> Self {
        Self {
            value: None,
            body,
            had_delimiters: false,
        }
    }

    /// Create a result where delimiters were found but parsing failed.
    fn with_invalid_frontmatter(body: &'a str) -> Self {
        Self {
            value: None,
            body,
            had_delimiters: true,
        }
    }

    /// Check if valid frontmatter was found and parsed.
    pub fn has_frontmatter(&self) -> bool {
        self.value.is_some()
    }

    /// Check if frontmatter delimiters were present (even if parsing failed).
    pub fn had_delimiters(&self) -> bool {
        self.had_delimiters
    }

    /// Get the raw YAML value, if present.
    pub fn value(&self) -> Option<&Value> {
        self.value.as_ref()
    }

    /// Take ownership of the YAML value, if present.
    pub fn into_value(self) -> Option<Value> {
        self.value
    }

    /// Get the body content (everything after frontmatter).
    pub fn body(&self) -> &'a str {
        self.body
    }

    /// Deserialize the frontmatter into a specific type.
    ///
    /// Returns `None` if no frontmatter was found.
    /// Returns `Err` if deserialization fails.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fabryk_content::markdown::extract_frontmatter;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct MyMeta {
    ///     title: String,
    /// }
    ///
    /// let content = "---\ntitle: Hello\n---\n\nBody";
    /// let result = extract_frontmatter(content).unwrap();
    /// let meta: Option<MyMeta> = result.deserialize().unwrap();
    /// assert_eq!(meta.unwrap().title, "Hello");
    /// ```
    pub fn deserialize<T: DeserializeOwned>(&self) -> Result<Option<T>> {
        match &self.value {
            Some(value) => {
                let parsed: T = serde_yaml::from_value(value.clone())
                    .map_err(|e| Error::parse(format!("Failed to deserialize frontmatter: {e}")))?;
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }

    /// Get a string field from the frontmatter.
    ///
    /// Convenience method for accessing common string fields.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.value.as_ref()?.get(key)?.as_str()
    }

    /// Get a string list field from the frontmatter.
    ///
    /// Returns an empty vec if the field is missing or not a sequence.
    pub fn get_string_list(&self, key: &str) -> Vec<String> {
        self.value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|item| item.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Extract YAML frontmatter from markdown content.
///
/// Parses content that starts with `---`, followed by YAML, followed by `---`.
/// Returns the parsed YAML value and the remaining body content.
///
/// # Behavior
///
/// - If no frontmatter delimiters found: returns body as-is, `has_frontmatter() == false`
/// - If delimiters found but YAML is invalid: logs warning, returns body after second `---`
/// - If valid frontmatter: returns parsed YAML and body
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::extract_frontmatter;
///
/// // With frontmatter
/// let content = "---\ntitle: Test\n---\n\n# Heading";
/// let result = extract_frontmatter(content).unwrap();
/// assert!(result.has_frontmatter());
/// assert_eq!(result.get_str("title"), Some("Test"));
/// assert_eq!(result.body().trim(), "# Heading");
///
/// // Without frontmatter
/// let content = "# Just Markdown";
/// let result = extract_frontmatter(content).unwrap();
/// assert!(!result.has_frontmatter());
/// assert_eq!(result.body(), "# Just Markdown");
/// ```
pub fn extract_frontmatter(content: &str) -> Result<FrontmatterResult<'_>> {
    // Must start with frontmatter delimiter
    if !content.starts_with("---") {
        return Ok(FrontmatterResult::without_frontmatter(content));
    }

    // Find the end of the first line (the opening ---)
    let after_first_delimiter = match content[3..].find('\n') {
        Some(pos) => &content[3 + pos + 1..],
        None => return Ok(FrontmatterResult::without_frontmatter(content)),
    };

    // Find the closing delimiter
    // Handle empty frontmatter (--- immediately) or normal case (\n---)
    let (yaml_content, body_after_closing) = if let Some(rest) =
        after_first_delimiter.strip_prefix("---")
    {
        // Empty frontmatter: ---\n---
        ("", rest)
    } else if let Some(closing_pos) = after_first_delimiter.find("\n---") {
        // Normal case: content between delimiters
        (
            &after_first_delimiter[..closing_pos],
            &after_first_delimiter[closing_pos + 4..],
        )
    } else {
        // No closing delimiter - treat entire content as body
        log::warn!("Frontmatter opening delimiter found but no closing delimiter");
        return Ok(FrontmatterResult::without_frontmatter(content));
    };

    // Skip the newline after closing ---
    let body = body_after_closing
        .strip_prefix('\n')
        .unwrap_or(body_after_closing);

    // Parse the YAML
    match serde_yaml::from_str::<Value>(yaml_content) {
        Ok(value) => Ok(FrontmatterResult::with_frontmatter(value, body)),
        Err(e) => {
            log::warn!("Failed to parse frontmatter YAML: {e}");
            Ok(FrontmatterResult::with_invalid_frontmatter(body))
        }
    }
}

/// Strip frontmatter from content, returning only the body.
///
/// This is a convenience function when you only need the body content
/// and don't care about the frontmatter metadata.
///
/// # Example
///
/// ```rust
/// use fabryk_content::markdown::strip_frontmatter;
///
/// let content = "---\ntitle: Test\n---\n\n# Heading";
/// let body = strip_frontmatter(content);
/// assert_eq!(body.trim(), "# Heading");
/// ```
pub fn strip_frontmatter(content: &str) -> &str {
    extract_frontmatter(content)
        .map(|r| r.body())
        .unwrap_or(content)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    // ------------------------------------------------------------------------
    // Basic extraction tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_valid_frontmatter() {
        let content = "---\ntitle: Test Document\nauthor: Claude\n---\n\n# Content";
        let result = extract_frontmatter(content).unwrap();

        assert!(result.has_frontmatter());
        assert!(result.had_delimiters());
        assert_eq!(result.get_str("title"), Some("Test Document"));
        assert_eq!(result.get_str("author"), Some("Claude"));
        assert_eq!(result.body().trim(), "# Content");
    }

    #[test]
    fn test_extract_no_frontmatter() {
        let content = "# Just Markdown\n\nNo frontmatter here.";
        let result = extract_frontmatter(content).unwrap();

        assert!(!result.has_frontmatter());
        assert!(!result.had_delimiters());
        assert_eq!(result.body(), content);
    }

    #[test]
    fn test_extract_empty_frontmatter() {
        let content = "---\n---\n\nBody content";
        let result = extract_frontmatter(content).unwrap();

        // Empty YAML parses as Null
        assert!(result.had_delimiters());
        assert_eq!(result.body().trim(), "Body content");
    }

    #[test]
    fn test_extract_frontmatter_no_closing() {
        let content = "---\ntitle: Incomplete\n\nNo closing delimiter";
        let result = extract_frontmatter(content).unwrap();

        assert!(!result.has_frontmatter());
        assert!(!result.had_delimiters()); // We don't set had_delimiters without both
        assert_eq!(result.body(), content);
    }

    #[test]
    fn test_extract_frontmatter_invalid_yaml() {
        let content = "---\n{{invalid: yaml: here}}\n---\n\nBody";
        let result = extract_frontmatter(content).unwrap();

        assert!(!result.has_frontmatter());
        assert!(result.had_delimiters());
        assert_eq!(result.body().trim(), "Body");
    }

    // ------------------------------------------------------------------------
    // Complex frontmatter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_extract_frontmatter_with_lists() {
        let content = "---\ntitle: Test\ntags:\n  - rust\n  - markdown\n---\n\nBody";
        let result = extract_frontmatter(content).unwrap();

        assert!(result.has_frontmatter());
        let tags = result.get_string_list("tags");
        assert_eq!(tags, vec!["rust", "markdown"]);
    }

    #[test]
    fn test_extract_frontmatter_with_nested() {
        let content = "---\nmeta:\n  author: Claude\n  version: 1.0\n---\n\nBody";
        let result = extract_frontmatter(content).unwrap();

        assert!(result.has_frontmatter());
        let value = result.value().unwrap();
        let author = value
            .get("meta")
            .and_then(|m| m.get("author"))
            .and_then(|a| a.as_str());
        assert_eq!(author, Some("Claude"));
    }

    // ------------------------------------------------------------------------
    // Deserialization tests
    // ------------------------------------------------------------------------

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestFrontmatter {
        title: String,
        #[serde(default)]
        tags: Vec<String>,
        category: Option<String>,
    }

    #[test]
    fn test_deserialize_frontmatter() {
        let content = "---\ntitle: My Doc\ntags:\n  - a\n  - b\ncategory: test\n---\n\nBody";
        let result = extract_frontmatter(content).unwrap();
        let fm: Option<TestFrontmatter> = result.deserialize().unwrap();

        let fm = fm.unwrap();
        assert_eq!(fm.title, "My Doc");
        assert_eq!(fm.tags, vec!["a", "b"]);
        assert_eq!(fm.category, Some("test".to_string()));
    }

    #[test]
    fn test_deserialize_no_frontmatter() {
        let content = "# No frontmatter";
        let result = extract_frontmatter(content).unwrap();
        let fm: Option<TestFrontmatter> = result.deserialize().unwrap();

        assert!(fm.is_none());
    }

    #[test]
    fn test_deserialize_partial_frontmatter() {
        let content = "---\ntitle: Only Title\n---\n\nBody";
        let result = extract_frontmatter(content).unwrap();
        let fm: Option<TestFrontmatter> = result.deserialize().unwrap();

        let fm = fm.unwrap();
        assert_eq!(fm.title, "Only Title");
        assert!(fm.tags.is_empty());
        assert!(fm.category.is_none());
    }

    // ------------------------------------------------------------------------
    // strip_frontmatter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_strip_frontmatter() {
        let content = "---\ntitle: Test\n---\n\n# Heading\n\nParagraph";
        let body = strip_frontmatter(content);
        assert_eq!(body.trim(), "# Heading\n\nParagraph");
    }

    #[test]
    fn test_strip_no_frontmatter() {
        let content = "# Just content";
        let body = strip_frontmatter(content);
        assert_eq!(body, content);
    }

    // ------------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------------

    #[test]
    fn test_frontmatter_with_dashes_in_content() {
        let content = "---\ntitle: Test\n---\n\nContent with --- dashes in it";
        let result = extract_frontmatter(content).unwrap();

        assert!(result.has_frontmatter());
        assert!(result.body().contains("--- dashes"));
    }

    #[test]
    fn test_frontmatter_unicode() {
        let content = "---\ntitle: 音楽理論\nauthor: クロード\n---\n\n本文";
        let result = extract_frontmatter(content).unwrap();

        assert!(result.has_frontmatter());
        assert_eq!(result.get_str("title"), Some("音楽理論"));
        assert_eq!(result.get_str("author"), Some("クロード"));
        assert_eq!(result.body().trim(), "本文");
    }

    #[test]
    fn test_empty_content() {
        let content = "";
        let result = extract_frontmatter(content).unwrap();

        assert!(!result.has_frontmatter());
        assert_eq!(result.body(), "");
    }

    #[test]
    fn test_only_opening_delimiter() {
        let content = "---";
        let result = extract_frontmatter(content).unwrap();

        assert!(!result.has_frontmatter());
        assert_eq!(result.body(), "---");
    }
}
