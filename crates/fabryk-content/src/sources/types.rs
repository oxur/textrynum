//! Data types for source reference validation.
//!
//! These types model the entities involved in cross-validating source
//! references across concept cards, configuration, and the filesystem.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A source reference found in concept cards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReference {
    /// The source title as written in the concept card.
    pub title: String,
    /// IDs of concept cards that reference this source.
    pub card_ids: Vec<String>,
}

impl SourceReference {
    /// Create a new source reference with no card IDs.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            card_ids: Vec::new(),
        }
    }

    /// Add a card ID to this reference.
    pub fn add_card(&mut self, card_id: impl Into<String>) {
        self.card_ids.push(card_id.into());
    }

    /// Get the number of cards referencing this source.
    pub fn card_count(&self) -> usize {
        self.card_ids.len()
    }
}

/// How a source reference was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionMethod {
    /// Matched by exact title extracted from filename.
    ExactTitle,
    /// Matched by a configured alias.
    Alias(String),
    /// Matched by direct source ID.
    DirectId,
    /// No match found.
    Unresolved,
}

/// A fuzzy match suggestion for an unresolved source.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceSuggestion {
    /// The source ID that might match.
    pub config_id: String,
    /// The title of the suggested source.
    pub title: String,
    /// Similarity score (0.0 to 1.0).
    pub similarity: f32,
}

impl SourceSuggestion {
    /// Create a new suggestion.
    pub fn new(config_id: impl Into<String>, title: impl Into<String>, similarity: f32) -> Self {
        Self {
            config_id: config_id.into(),
            title: title.into(),
            similarity,
        }
    }
}

/// A source referenced in cards but missing from configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct MissingSource {
    /// The title as written in concept cards.
    pub title: String,
    /// Number of cards referencing this source.
    pub card_count: usize,
    /// Sample card IDs (first few).
    pub sample_card_ids: Vec<String>,
    /// Fuzzy match suggestions.
    pub suggestions: Vec<SourceSuggestion>,
}

impl MissingSource {
    /// Create a new missing source entry.
    ///
    /// Stores at most 5 sample card IDs for display purposes.
    pub fn new(title: impl Into<String>, card_ids: &[String]) -> Self {
        let sample_card_ids = card_ids.iter().take(5).cloned().collect();
        Self {
            title: title.into(),
            card_count: card_ids.len(),
            sample_card_ids,
            suggestions: Vec::new(),
        }
    }

    /// Add a suggestion.
    pub fn add_suggestion(&mut self, suggestion: SourceSuggestion) {
        self.suggestions.push(suggestion);
    }
}

/// A source configured but missing from the filesystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingFile {
    /// The source ID from configuration.
    pub config_id: String,
    /// The expected file path.
    pub expected_path: PathBuf,
    /// The source category.
    pub category: String,
}

impl MissingFile {
    /// Create a new missing file entry.
    pub fn new(
        config_id: impl Into<String>,
        expected_path: impl Into<PathBuf>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            config_id: config_id.into(),
            expected_path: expected_path.into(),
            category: category.into(),
        }
    }
}

/// Statistics from validation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total concept cards scanned.
    pub total_cards_scanned: usize,
    /// Unique source titles found in cards.
    pub unique_sources_found: usize,
    /// Sources successfully resolved to config.
    pub sources_resolved: usize,
    /// Sources in config.
    pub sources_in_config: usize,
    /// Configured sources found on disk.
    pub sources_on_disk: usize,
    /// Sources missing from config.
    pub missing_from_config: usize,
    /// Sources missing from disk.
    pub missing_from_disk: usize,
}

/// Comprehensive validation report.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Sources referenced in cards but not in configuration.
    pub missing_from_config: Vec<MissingSource>,
    /// Sources in configuration but not on filesystem.
    pub missing_from_filesystem: Vec<MissingFile>,
    /// Summary statistics.
    pub stats: ValidationStats,
}

impl ValidationReport {
    /// Create a new empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if validation passed (no issues found).
    pub fn is_valid(&self) -> bool {
        self.missing_from_config.is_empty() && self.missing_from_filesystem.is_empty()
    }

    /// Get exit code for CLI (0 = valid, 1 = issues found).
    pub fn exit_code(&self) -> i32 {
        if self.is_valid() { 0 } else { 1 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_reference_new() {
        let sr = SourceReference::new("Test Source");
        assert_eq!(sr.title, "Test Source");
        assert!(sr.card_ids.is_empty());
        assert_eq!(sr.card_count(), 0);
    }

    #[test]
    fn test_source_reference_add_card() {
        let mut sr = SourceReference::new("Test Source");
        sr.add_card("card-1");
        sr.add_card("card-2");
        assert_eq!(sr.card_count(), 2);
        assert_eq!(sr.card_ids, vec!["card-1", "card-2"]);
    }

    #[test]
    fn test_resolution_method_equality() {
        assert_eq!(ResolutionMethod::ExactTitle, ResolutionMethod::ExactTitle);
        assert_eq!(
            ResolutionMethod::Alias("test".to_string()),
            ResolutionMethod::Alias("test".to_string())
        );
        assert_ne!(ResolutionMethod::ExactTitle, ResolutionMethod::DirectId);
    }

    #[test]
    fn test_source_suggestion_new() {
        let suggestion = SourceSuggestion::new("oxford-test", "Test Source", 0.85);
        assert_eq!(suggestion.config_id, "oxford-test");
        assert_eq!(suggestion.title, "Test Source");
        assert!((suggestion.similarity - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn test_missing_source_new() {
        let card_ids = vec![
            "card-1".to_string(),
            "card-2".to_string(),
            "card-3".to_string(),
        ];
        let ms = MissingSource::new("Missing Title", &card_ids);
        assert_eq!(ms.title, "Missing Title");
        assert_eq!(ms.card_count, 3);
        assert_eq!(ms.sample_card_ids.len(), 3);
        assert!(ms.suggestions.is_empty());
    }

    #[test]
    fn test_missing_source_sample_limit() {
        let card_ids: Vec<String> = (0..10).map(|i| format!("card-{i}")).collect();
        let ms = MissingSource::new("Many Cards", &card_ids);
        assert_eq!(ms.card_count, 10);
        assert_eq!(ms.sample_card_ids.len(), 5);
    }

    #[test]
    fn test_missing_source_add_suggestion() {
        let mut ms = MissingSource::new("Test", &[]);
        ms.add_suggestion(SourceSuggestion::new("id-1", "Title 1", 0.9));
        assert_eq!(ms.suggestions.len(), 1);
        assert_eq!(ms.suggestions[0].config_id, "id-1");
    }

    #[test]
    fn test_missing_file_new() {
        let mf = MissingFile::new("oxford-test", "/path/to/file.pdf", "oxford");
        assert_eq!(mf.config_id, "oxford-test");
        assert_eq!(mf.expected_path, PathBuf::from("/path/to/file.pdf"));
        assert_eq!(mf.category, "oxford");
    }

    #[test]
    fn test_validation_stats_default() {
        let stats = ValidationStats::default();
        assert_eq!(stats.total_cards_scanned, 0);
        assert_eq!(stats.unique_sources_found, 0);
        assert_eq!(stats.sources_resolved, 0);
        assert_eq!(stats.sources_in_config, 0);
        assert_eq!(stats.sources_on_disk, 0);
        assert_eq!(stats.missing_from_config, 0);
        assert_eq!(stats.missing_from_disk, 0);
    }

    #[test]
    fn test_validation_report_is_valid_empty() {
        let report = ValidationReport::new();
        assert!(report.is_valid());
        assert_eq!(report.exit_code(), 0);
    }

    #[test]
    fn test_validation_report_is_valid_with_missing_config() {
        let mut report = ValidationReport::new();
        report
            .missing_from_config
            .push(MissingSource::new("Test", &[]));
        assert!(!report.is_valid());
        assert_eq!(report.exit_code(), 1);
    }

    #[test]
    fn test_validation_report_is_valid_with_missing_file() {
        let mut report = ValidationReport::new();
        report
            .missing_from_filesystem
            .push(MissingFile::new("test", "/path", "oxford"));
        assert!(!report.is_valid());
        assert_eq!(report.exit_code(), 1);
    }
}
