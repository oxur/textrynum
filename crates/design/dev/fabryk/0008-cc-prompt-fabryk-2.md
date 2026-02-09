---
title: "CC Prompt: Fabryk 2.1 — Frontmatter Extraction"
milestone: "2.1"
phase: 2
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 1 complete (1.0-1.6)"]
governing-docs: [0011-audit §4.2, 0012-amendment §2c, 0013-project-plan]
---

# CC Prompt: Fabryk 2.1 — Frontmatter Extraction

## Context

Phase 2 extracts markdown parsing, frontmatter extraction, and content helper
utilities into `fabryk-content`. This is the first milestone of Phase 2.

**Crate:** `fabryk-content`
**Dependency level:** 1 (depends on `fabryk-core`)
**Risk:** Low

**Key architectural decision (Amendment §2c):** The `MetadataExtractor` trait is
deferred. `fabryk-content` provides generic parsing utilities that return raw
YAML values. Domain implementations (like music-theory) define their own
`Frontmatter` structs and parse the raw YAML into domain-specific types.

This means:
- `fabryk-content::extract_frontmatter()` returns `serde_yaml::Value`
- Domain crates define their own `Frontmatter` struct (e.g., with `concept`,
  `category`, `source` fields for music-theory)
- Domain crates deserialize the `Value` into their struct

**Music-Theory Migration**: This milestone extracts code to Fabryk only.
Music-theory continues using its local copy until the v0.1-alpha checkpoint
(after Phase 3 completion), when all imports will be updated in a single
coordinated migration.

## Source Files

**Music-theory source** (via symlink):
```
~/lab/oxur/ecl/workbench/music-theory-mcp-server/crates/server/src/markdown/frontmatter.rs
```

Or directly:
```
~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/markdown/frontmatter.rs
```

**From Research:** 269 lines, 11 tests. Key functions: `extract_frontmatter()`,
`strip_frontmatter()`. Dependencies: `serde_yaml`, `log`.

**Classification:** The YAML parsing logic is Generic (G). The `Frontmatter`
struct is Domain-Specific (D) and stays in music-theory.

## Objective

1. Create `fabryk-content/src/markdown/` module structure
2. Extract generic frontmatter parsing that returns `serde_yaml::Value`
3. Provide helper function to deserialize into any `serde::Deserialize` type
4. Comprehensive test suite for generic parsing
5. Verify: `cargo test -p fabryk-content` passes

## Implementation Steps

### Step 1: Create module structure

```bash
cd ~/lab/oxur/ecl/crates/fabryk-content
mkdir -p src/markdown
```

### Step 2: Create `fabryk-content/src/markdown/mod.rs`

```rust
//! Markdown parsing and frontmatter extraction utilities.
//!
//! This module provides generic utilities for parsing markdown content:
//!
//! - [`frontmatter`]: YAML frontmatter extraction
//! - [`parser`]: Markdown structure parsing (headings, paragraphs)
//! - [`helpers`]: Content extraction helpers
//!
//! # Design Philosophy
//!
//! These utilities return generic types (`serde_yaml::Value`, `String`) rather
//! than domain-specific structs. Domain crates (music-theory, math, etc.)
//! define their own metadata types and use these utilities to extract raw data.
//!
//! # Example
//!
//! ```rust,ignore
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
//! let result = extract_frontmatter(content)?;
//!
//! // Deserialize into domain-specific type
//! let fm: MyFrontmatter = result.deserialize()?;
//! println!("Title: {}", fm.title);
//! ```

pub mod frontmatter;

// Re-export key types and functions
pub use frontmatter::{extract_frontmatter, strip_frontmatter, FrontmatterResult};

// Modules to be added in subsequent milestones:
// pub mod parser;   // Milestone 2.2
// pub mod helpers;  // Milestone 2.3
```

### Step 3: Create `fabryk-content/src/markdown/frontmatter.rs`

```rust
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
//! assert_eq!(result.body(), "Body");
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
    // It must be on its own line, so we look for \n---
    let closing_marker = "\n---";
    let closing_pos = match after_first_delimiter.find(closing_marker) {
        Some(pos) => pos,
        None => {
            // No closing delimiter - treat entire content as body
            log::warn!("Frontmatter opening delimiter found but no closing delimiter");
            return Ok(FrontmatterResult::without_frontmatter(content));
        }
    };

    // Extract the YAML content between delimiters
    let yaml_content = &after_first_delimiter[..closing_pos];

    // Find where the body starts (after ---\n or just after --- at end)
    let body_start = closing_pos + closing_marker.len();
    let body = if body_start < after_first_delimiter.len() {
        // Skip the newline after closing ---
        let remaining = &after_first_delimiter[body_start..];
        remaining.strip_prefix('\n').unwrap_or(remaining)
    } else {
        ""
    };

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
        let author = value.get("meta").and_then(|m| m.get("author")).and_then(|a| a.as_str());
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
```

### Step 4: Update `fabryk-content/src/lib.rs`

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
//!
//! # Example
//!
//! ```rust,ignore
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
//! let result = extract_frontmatter(content)?;
//! let meta: Option<MyMeta> = result.deserialize()?;
//! ```

#![doc = include_str!("../README.md")]

pub mod markdown;

// Re-export commonly used types
pub use markdown::{extract_frontmatter, strip_frontmatter, FrontmatterResult};
```

### Step 5: Update `fabryk-content/Cargo.toml`

Ensure dependencies are correct:

```toml
[package]
name = "fabryk-content"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Markdown parsing, frontmatter extraction, and content utilities for Fabryk"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# Markdown parsing (milestone 2.2)
pulldown-cmark = { workspace = true }

# Frontmatter (YAML)
serde_yaml = { workspace = true }

# Serialization
serde = { workspace = true }

# Regex for content extraction (milestone 2.3)
regex = { workspace = true }

# Logging
log = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### Step 6: Verify

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-content
cargo test -p fabryk-content
cargo clippy -p fabryk-content -- -D warnings
cargo doc -p fabryk-content --no-deps
```

## Exit Criteria

- [ ] `fabryk-content/src/markdown/mod.rs` created with module structure
- [ ] `fabryk-content/src/markdown/frontmatter.rs` with generic parsing
- [ ] `FrontmatterResult` type with:
  - `has_frontmatter()` - check if valid frontmatter found
  - `had_delimiters()` - check if delimiters were present
  - `value()` - get raw `serde_yaml::Value`
  - `body()` - get body content after frontmatter
  - `deserialize<T>()` - deserialize into domain-specific type
  - `get_str()` - convenience accessor for string fields
  - `get_string_list()` - convenience accessor for string lists
- [ ] `extract_frontmatter()` function returns `Result<FrontmatterResult>`
- [ ] `strip_frontmatter()` convenience function
- [ ] Test coverage for:
  - Valid frontmatter extraction
  - No frontmatter handling
  - Empty frontmatter
  - Invalid YAML graceful degradation
  - Nested YAML structures
  - Unicode content
  - Edge cases
- [ ] `cargo test -p fabryk-content` passes
- [ ] `cargo clippy -p fabryk-content -- -D warnings` clean

## Design Notes

### Why `serde_yaml::Value` instead of domain-specific struct?

Per Amendment §2c, the `MetadataExtractor` trait is deferred. Each domain has
different frontmatter fields:

| Domain | Frontmatter Fields |
|--------|-------------------|
| Music Theory | `concept`, `category`, `source`, `chapter`, `section`, `part` |
| Higher Math | `theorem`, `area`, `difficulty`, `proven_by` |
| Rust Concepts | `feature`, `rust_version`, `stability`, `module` |

A generic `Frontmatter` struct can't cover all these. Instead:

1. `fabryk-content` provides raw YAML parsing
2. Each domain defines its own `Frontmatter` struct with `#[derive(Deserialize)]`
3. Domains call `result.deserialize::<MyFrontmatter>()` to get typed data

This keeps `fabryk-content` domain-agnostic while still providing useful utilities.

### Comparison with music-theory implementation

| Music Theory | Fabryk |
|--------------|--------|
| `Frontmatter` struct with fixed fields | `FrontmatterResult` with `serde_yaml::Value` |
| `extract_frontmatter() -> (Option<Frontmatter>, &str)` | `extract_frontmatter() -> Result<FrontmatterResult>` |
| Domain fields hardcoded | Domain deserializes via `deserialize::<T>()` |

## Commit Message

```
feat(content): extract frontmatter parsing to fabryk-content

Add generic YAML frontmatter extraction utilities:
- FrontmatterResult type with raw Value access
- extract_frontmatter() returns generic Result
- deserialize<T>() for domain-specific types
- strip_frontmatter() convenience function
- Comprehensive test suite

Per Amendment §2c, returns serde_yaml::Value instead of
domain-specific struct. Domains deserialize into their own types.

Ref: Doc 0013 milestone 2.1, Audit §4.2

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
