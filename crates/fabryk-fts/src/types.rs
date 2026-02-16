//! Common types for the FTS module.
//!
//! These types are used across all search backends and are always available
//! regardless of feature flags.

use serde::{Deserialize, Serialize};

/// Search query mode.
///
/// Controls how multiple search terms are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryMode {
    /// Smart mode: AND for 1-2 terms, OR with minimum match for 3+.
    #[default]
    Smart,
    /// All terms must match (AND).
    And,
    /// Any term can match (OR).
    Or,
    /// At least N terms must match (configured separately).
    MinimumMatch,
}

/// Search configuration.
///
/// Domain implementations provide this to configure search behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Backend type: "tantivy" or "simple".
    #[serde(default = "default_backend")]
    pub backend: String,

    /// Path to the search index directory.
    pub index_path: Option<String>,

    /// Path to content for indexing.
    pub content_path: Option<String>,

    /// Default query mode.
    #[serde(default)]
    pub query_mode: QueryMode,

    /// Enable fuzzy matching.
    #[serde(default)]
    pub fuzzy_enabled: bool,

    /// Fuzzy edit distance (1 or 2).
    #[serde(default = "default_fuzzy_distance")]
    pub fuzzy_distance: u8,

    /// Enable stopword filtering.
    #[serde(default = "default_true")]
    pub stopwords_enabled: bool,

    /// Custom stopwords to add.
    #[serde(default)]
    pub custom_stopwords: Vec<String>,

    /// Words to preserve (not filter as stopwords).
    #[serde(default)]
    pub allowlist: Vec<String>,

    /// Default result limit.
    #[serde(default = "default_limit")]
    pub default_limit: usize,

    /// Snippet length in characters.
    #[serde(default = "default_snippet_length")]
    pub snippet_length: usize,
}

fn default_backend() -> String {
    "tantivy".to_string()
}

fn default_fuzzy_distance() -> u8 {
    1
}

fn default_true() -> bool {
    true
}

fn default_limit() -> usize {
    10
}

fn default_snippet_length() -> usize {
    200
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            backend: default_backend(),
            index_path: None,
            content_path: None,
            query_mode: QueryMode::default(),
            fuzzy_enabled: false,
            fuzzy_distance: default_fuzzy_distance(),
            stopwords_enabled: default_true(),
            custom_stopwords: Vec::new(),
            allowlist: Vec::new(),
            default_limit: default_limit(),
            snippet_length: default_snippet_length(),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_mode_default() {
        assert_eq!(QueryMode::default(), QueryMode::Smart);
    }

    #[test]
    fn test_query_mode_serialization() {
        let mode = QueryMode::MinimumMatch;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"minimum_match\"");
    }

    #[test]
    fn test_query_mode_deserialization() {
        let mode: QueryMode = serde_json::from_str("\"and\"").unwrap();
        assert_eq!(mode, QueryMode::And);
    }

    #[test]
    fn test_search_config_default() {
        let config = SearchConfig::default();
        assert_eq!(config.backend, "tantivy");
        assert!(config.index_path.is_none());
        assert_eq!(config.query_mode, QueryMode::Smart);
        assert!(!config.fuzzy_enabled);
        assert_eq!(config.fuzzy_distance, 1);
        assert!(config.stopwords_enabled);
        assert_eq!(config.default_limit, 10);
        assert_eq!(config.snippet_length, 200);
    }

    #[test]
    fn test_search_config_serialization() {
        let config = SearchConfig {
            backend: "simple".to_string(),
            index_path: Some("/tmp/index".to_string()),
            fuzzy_enabled: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"backend\":\"simple\""));
        assert!(json.contains("\"index_path\":\"/tmp/index\""));
        assert!(json.contains("\"fuzzy_enabled\":true"));
    }

    #[test]
    fn test_search_config_deserialization_with_defaults() {
        let json = r#"{"backend": "tantivy"}"#;
        let config: SearchConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.backend, "tantivy");
        assert_eq!(config.default_limit, 10);
        assert!(config.stopwords_enabled);
    }
}
