//! Stopword filtering for search queries.
//!
//! This module filters common words (stopwords) from search queries to improve
//! search quality. It uses the `stop-words` crate for a comprehensive English
//! stopword list (~500 words) and supports:
//!
//! - Allowlist: Words to preserve even if they're stopwords
//! - Custom stopwords: Additional words to filter
//! - Graceful fallback: If all terms are filtered, returns original query
//!
//! # Domain-Specific Allowlists
//!
//! Some domains have terms that look like stopwords but are meaningful:
//!
//! - Music theory: Roman numerals (I, ii, IV, V, vi, vii)
//! - Music theory: Solfège syllables (do, re, mi, fa, sol, la, ti)
//!
//! Configure allowlists via `SearchConfig::allowlist`.
//!
//! # Example
//!
//! ```rust
//! use fabryk_fts::stopwords::StopwordFilter;
//! use fabryk_fts::SearchConfig;
//!
//! let config = SearchConfig {
//!     allowlist: vec!["I".to_string(), "V".to_string()],
//!     ..Default::default()
//! };
//!
//! let filter = StopwordFilter::new(&config);
//!
//! // Common words removed
//! assert_eq!(filter.filter("what is a cadence"), "cadence");
//!
//! // Allowlisted words preserved
//! assert_eq!(filter.filter("I V I progression"), "I V I progression");
//! ```

use std::collections::HashSet;
use stop_words::{get, LANGUAGE};

use crate::SearchConfig;

/// Stopword filter for query preprocessing.
///
/// Removes common words while preserving domain-specific terms.
pub struct StopwordFilter {
    stopwords: HashSet<String>,
    allowlist: HashSet<String>,
    enabled: bool,
}

impl StopwordFilter {
    /// Create a new stopword filter from configuration.
    pub fn new(config: &SearchConfig) -> Self {
        let mut stopwords: HashSet<String> = get(LANGUAGE::English)
            .iter()
            .map(|s| s.to_lowercase())
            .collect();

        // Add custom stopwords
        for word in &config.custom_stopwords {
            stopwords.insert(word.to_lowercase());
        }

        // Build allowlist (case-sensitive for proper nouns, numerals)
        let allowlist: HashSet<String> = config.allowlist.iter().cloned().collect();

        Self {
            stopwords,
            allowlist,
            enabled: config.stopwords_enabled,
        }
    }

    /// Create a disabled filter (passes all words through).
    pub fn disabled() -> Self {
        Self {
            stopwords: HashSet::new(),
            allowlist: HashSet::new(),
            enabled: false,
        }
    }

    /// Filter stopwords from a query string.
    ///
    /// Returns the filtered query. If all words are filtered, returns
    /// the original query to avoid empty searches.
    pub fn filter(&self, query: &str) -> String {
        if !self.enabled {
            return query.to_string();
        }

        let filtered: Vec<&str> = query
            .split_whitespace()
            .filter(|word| !self.is_stopword(word))
            .collect();

        if filtered.is_empty() {
            // Fallback: return original if all words filtered
            query.to_string()
        } else {
            filtered.join(" ")
        }
    }

    /// Check if a word is a stopword.
    ///
    /// Returns `false` if the word is in the allowlist (case-sensitive check).
    /// Otherwise, checks the stopword list (case-insensitive).
    pub fn is_stopword(&self, word: &str) -> bool {
        // Allowlist is case-sensitive (for Roman numerals like "I", "V")
        if self.allowlist.contains(word) {
            return false;
        }

        // Stopword check is case-insensitive
        self.stopwords.contains(&word.to_lowercase())
    }

    /// Get the number of stopwords in the filter.
    pub fn stopword_count(&self) -> usize {
        self.stopwords.len()
    }

    /// Check if filtering is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl std::fmt::Debug for StopwordFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StopwordFilter")
            .field("enabled", &self.enabled)
            .field("stopword_count", &self.stopwords.len())
            .field("allowlist_count", &self.allowlist.len())
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn default_filter() -> StopwordFilter {
        StopwordFilter::new(&SearchConfig::default())
    }

    fn filter_with_allowlist(allowlist: Vec<&str>) -> StopwordFilter {
        let config = SearchConfig {
            allowlist: allowlist.into_iter().map(String::from).collect(),
            ..Default::default()
        };
        StopwordFilter::new(&config)
    }

    // ------------------------------------------------------------------------
    // Basic filtering tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_filter_common_words() {
        let filter = default_filter();
        assert_eq!(filter.filter("what is a cadence"), "cadence");
    }

    #[test]
    fn test_filter_preserves_content_words() {
        let filter = default_filter();
        assert_eq!(filter.filter("harmonic progression"), "harmonic progression");
    }

    #[test]
    fn test_filter_mixed() {
        let filter = default_filter();
        let result = filter.filter("the theory of harmony");
        assert!(result.contains("theory"));
        assert!(result.contains("harmony"));

        // Check that "the" and "of" are not present as separate words
        let words: Vec<&str> = result.split_whitespace().collect();
        assert!(!words.contains(&"the"));
        assert!(!words.contains(&"of"));
    }

    // ------------------------------------------------------------------------
    // Allowlist tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_allowlist_roman_numerals() {
        let filter = filter_with_allowlist(vec!["I", "IV", "V", "vi"]);

        // Roman numerals preserved
        assert_eq!(filter.filter("I IV V progression"), "I IV V progression");
        assert_eq!(filter.filter("vi chord"), "vi chord");
    }

    #[test]
    fn test_allowlist_case_sensitive() {
        let filter = filter_with_allowlist(vec!["I", "V"]);

        // Uppercase "I" preserved, lowercase "i" filtered
        let result = filter.filter("I am a musician");
        assert!(result.contains("I"));
        assert!(result.contains("musician"));
    }

    #[test]
    fn test_allowlist_solfege() {
        let filter = filter_with_allowlist(vec!["do", "re", "mi", "fa", "sol", "la", "ti"]);

        assert_eq!(filter.filter("do re mi fa sol"), "do re mi fa sol");
    }

    // ------------------------------------------------------------------------
    // Edge case tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_filter_all_stopwords_fallback() {
        let filter = default_filter();

        // If all words are stopwords, return original
        let original = "the a an";
        assert_eq!(filter.filter(original), original);
    }

    #[test]
    fn test_filter_empty_query() {
        let filter = default_filter();
        assert_eq!(filter.filter(""), "");
    }

    #[test]
    fn test_filter_single_stopword() {
        let filter = default_filter();
        // Single stopword returns original
        assert_eq!(filter.filter("the"), "the");
    }

    #[test]
    fn test_filter_preserves_word_order() {
        let filter = default_filter();
        let result = filter.filter("understand the theory behind music");
        let words: Vec<&str> = result.split_whitespace().collect();

        // Order should be: understand, theory, behind, music
        assert_eq!(words[0], "understand");
        assert!(words.contains(&"theory"));
        assert!(words.contains(&"music"));
    }

    // ------------------------------------------------------------------------
    // Custom stopwords tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_custom_stopwords() {
        let config = SearchConfig {
            custom_stopwords: vec!["foo".to_string(), "bar".to_string()],
            ..Default::default()
        };
        let filter = StopwordFilter::new(&config);

        let result = filter.filter("foo bar baz");
        assert_eq!(result, "baz");
    }

    // ------------------------------------------------------------------------
    // Disabled filter tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_disabled_filter() {
        let filter = StopwordFilter::disabled();

        assert_eq!(filter.filter("the a an"), "the a an");
        assert!(!filter.is_enabled());
    }

    #[test]
    fn test_filter_via_config_disabled() {
        let config = SearchConfig {
            stopwords_enabled: false,
            ..Default::default()
        };
        let filter = StopwordFilter::new(&config);

        assert_eq!(filter.filter("the a an"), "the a an");
    }

    // ------------------------------------------------------------------------
    // Stopword count tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_stopword_count() {
        let filter = default_filter();

        // stop-words crate has 500+ English stopwords
        assert!(filter.stopword_count() >= 500);
    }

    #[test]
    fn test_is_stopword() {
        let filter = default_filter();

        assert!(filter.is_stopword("the"));
        assert!(filter.is_stopword("THE")); // Case-insensitive
        assert!(filter.is_stopword("a"));
        assert!(filter.is_stopword("is"));

        assert!(!filter.is_stopword("harmony"));
        assert!(!filter.is_stopword("music"));
    }

    // ------------------------------------------------------------------------
    // Debug formatting
    // ------------------------------------------------------------------------

    #[test]
    fn test_debug_format() {
        let filter = default_filter();
        let debug = format!("{:?}", filter);

        assert!(debug.contains("StopwordFilter"));
        assert!(debug.contains("enabled"));
        assert!(debug.contains("stopword_count"));
    }
}
