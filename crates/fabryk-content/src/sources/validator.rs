//! Source validation logic.
//!
//! Validates source references across:
//! - Concept cards vs configuration
//! - Configuration vs filesystem

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fabryk_core::Result;

use crate::sources::resolver::{SourceCategory, SourceResolver};
use crate::sources::scanner::scan_content_for_sources_with_stats;
use crate::sources::types::{MissingFile, MissingSource, ValidationReport};

/// Validation mode for the validate command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ValidationMode {
    /// Validate everything.
    #[default]
    All,
    /// Only validate cards against config.
    CardsConfig,
    /// Only validate cards against filesystem (via config).
    CardsFilesystem,
    /// Only validate config against filesystem.
    ConfigFilesystem,
}

/// Validate sources according to the specified mode.
///
/// Performs cross-validation between:
/// - Source references in concept cards
/// - Source configuration (titles extracted from filenames, aliases)
/// - Actual source files on the filesystem
///
/// # Arguments
///
/// * `content_path` - Path to the concept cards directory
/// * `categories` - Map of category names to their configurations
/// * `mode` - What to validate
/// * `suggest_matches` - Whether to compute fuzzy match suggestions for missing sources
/// * `threshold` - Similarity threshold for fuzzy suggestions (0.0-1.0)
///
/// # Returns
///
/// A [`ValidationReport`] containing:
/// - Sources referenced in cards but not found in config
/// - Sources in config but not found on filesystem
/// - Summary statistics
///
/// # Errors
///
/// Returns `Err` if scanning content files fails.
pub async fn validate_sources(
    content_path: &Path,
    categories: &HashMap<String, SourceCategory>,
    mode: ValidationMode,
    suggest_matches: bool,
    threshold: f32,
) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();

    // Scan concept cards for source references
    let (sources_in_cards, scan_stats) = scan_content_for_sources_with_stats(content_path).await?;

    // Create resolver from categories
    let resolver = SourceResolver::from_categories(categories);

    // Initialize stats
    report.stats.total_cards_scanned = scan_stats.total_cards;
    report.stats.unique_sources_found = sources_in_cards.len();
    report.stats.sources_in_config = resolver.known_ids().len();

    // Validate cards against config (unless mode is ConfigFilesystem only)
    if mode == ValidationMode::All
        || mode == ValidationMode::CardsConfig
        || mode == ValidationMode::CardsFilesystem
    {
        let mut resolved_count = 0;

        for (title, reference) in &sources_in_cards {
            let (resolved_id, _method) = resolver.resolve(title);

            if resolved_id.is_some() {
                resolved_count += 1;
            } else {
                let mut missing = MissingSource::new(title, &reference.card_ids);

                if suggest_matches {
                    let suggestions = resolver.suggest_matches(title, threshold);
                    for suggestion in suggestions {
                        missing.add_suggestion(suggestion);
                    }
                }

                report.missing_from_config.push(missing);
            }
        }

        report.stats.sources_resolved = resolved_count;
        report.stats.missing_from_config = report.missing_from_config.len();
    }

    // Validate config against filesystem (unless mode is CardsConfig only)
    if mode == ValidationMode::All
        || mode == ValidationMode::ConfigFilesystem
        || mode == ValidationMode::CardsFilesystem
    {
        let missing_files = check_filesystem(categories).await;
        report.stats.sources_on_disk = report.stats.sources_in_config - missing_files.len();
        report.stats.missing_from_disk = missing_files.len();
        report.missing_from_filesystem = missing_files;
    }

    Ok(report)
}

/// Check which configured sources are missing from the filesystem.
async fn check_filesystem(categories: &HashMap<String, SourceCategory>) -> Vec<MissingFile> {
    let mut missing = Vec::new();

    for (category_name, category) in categories {
        // Skip if no base path configured
        if category.path.is_empty() {
            continue;
        }

        let base_path = PathBuf::from(&category.path);

        for (file_id, filename) in &category.files {
            let file_path = base_path.join(filename);
            let source_id = format!("{category_name}-{file_id}");

            if !tokio::fs::try_exists(&file_path).await.unwrap_or(false) {
                missing.push(MissingFile::new(&source_id, &file_path, category_name));
            }
        }
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_concept_card(base_dir: &Path, filename: &str, source: &str) {
        let dir = base_dir.join("harmony");
        fs::create_dir_all(&dir).await.unwrap();

        let content = format!(
            "---\ntitle: \"Test Concept\"\ncategory: \"harmony\"\nsource: \"{source}\"\n---\n# Test\n\nContent"
        );
        fs::write(dir.join(filename), content).await.unwrap();
    }

    async fn create_source_file(sources_dir: &Path, filename: &str) {
        fs::create_dir_all(sources_dir).await.unwrap();
        fs::write(sources_dir.join(filename), "dummy content")
            .await
            .unwrap();
    }

    fn make_categories(
        sources_path: &Path,
        files: HashMap<String, String>,
    ) -> HashMap<String, SourceCategory> {
        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: sources_path.to_string_lossy().to_string(),
                files,
                aliases: HashMap::new(),
            },
        );
        categories
    }

    #[tokio::test]
    async fn test_validate_sources_all_valid() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "open-music-theory".to_string(),
            "[2022] Gotham - Open Music Theory.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        create_concept_card(&cards_path, "test.md", "Open Music Theory").await;
        create_source_file(&sources_path, "[2022] Gotham - Open Music Theory.pdf").await;

        let report = validate_sources(&cards_path, &categories, ValidationMode::All, false, 0.7)
            .await
            .unwrap();

        assert!(report.is_valid());
        assert!(report.missing_from_config.is_empty());
        assert!(report.missing_from_filesystem.is_empty());
        assert_eq!(report.stats.sources_resolved, 1);
    }

    #[tokio::test]
    async fn test_validate_sources_missing_from_config() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        fs::create_dir_all(&cards_path).await.unwrap();

        let categories = HashMap::new();

        create_concept_card(&cards_path, "test.md", "Unknown Source").await;

        let report = validate_sources(&cards_path, &categories, ValidationMode::All, false, 0.7)
            .await
            .unwrap();

        assert!(!report.is_valid());
        assert_eq!(report.missing_from_config.len(), 1);
        assert_eq!(report.missing_from_config[0].title, "Unknown Source");
        assert_eq!(report.stats.missing_from_config, 1);
    }

    #[tokio::test]
    async fn test_validate_sources_missing_from_filesystem() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "missing-source".to_string(),
            "[2022] Author - Missing Source.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        // Don't create the source file

        let report = validate_sources(&cards_path, &categories, ValidationMode::All, false, 0.7)
            .await
            .unwrap();

        assert!(!report.is_valid());
        assert_eq!(report.missing_from_filesystem.len(), 1);
        assert_eq!(
            report.missing_from_filesystem[0].config_id,
            "general-missing-source"
        );
        assert_eq!(report.stats.missing_from_disk, 1);
    }

    #[tokio::test]
    async fn test_validate_sources_with_suggestions() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "open-music-theory".to_string(),
            "[2022] Gotham - Open Music Theory.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        create_source_file(&sources_path, "[2022] Gotham - Open Music Theory.pdf").await;

        // Typo in source name
        create_concept_card(&cards_path, "test.md", "Open Music Theor").await;

        let report = validate_sources(&cards_path, &categories, ValidationMode::All, true, 0.7)
            .await
            .unwrap();

        assert!(!report.is_valid());
        assert_eq!(report.missing_from_config.len(), 1);

        let missing = &report.missing_from_config[0];
        assert!(!missing.suggestions.is_empty());
        assert!(missing.suggestions[0].similarity > 0.7);
    }

    #[tokio::test]
    async fn test_validate_sources_mode_cards_config_only() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "missing-file".to_string(),
            "[2022] Author - Missing.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        create_concept_card(&cards_path, "test.md", "Missing").await;

        // Don't create file - but with CardsConfig mode, we shouldn't check filesystem
        let report = validate_sources(
            &cards_path,
            &categories,
            ValidationMode::CardsConfig,
            false,
            0.7,
        )
        .await
        .unwrap();

        assert!(report.is_valid());
        assert!(report.missing_from_config.is_empty());
        assert!(report.missing_from_filesystem.is_empty());
    }

    #[tokio::test]
    async fn test_validate_sources_mode_config_filesystem_only() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "missing-file".to_string(),
            "[2022] Author - Missing.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        // Create concept card with unknown source - should be ignored in ConfigFilesystem mode
        create_concept_card(&cards_path, "test.md", "Unknown Source").await;

        let report = validate_sources(
            &cards_path,
            &categories,
            ValidationMode::ConfigFilesystem,
            false,
            0.7,
        )
        .await
        .unwrap();

        assert!(!report.is_valid());
        assert!(report.missing_from_config.is_empty());
        assert_eq!(report.missing_from_filesystem.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_sources_stats() {
        let temp_dir = TempDir::new().unwrap();
        let cards_path = temp_dir.path().join("concept-cards");
        let sources_path = temp_dir.path().join("sources");
        fs::create_dir_all(&cards_path).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "source-1".to_string(),
            "[2022] Author - Source One.pdf".to_string(),
        );
        files.insert(
            "source-2".to_string(),
            "[2022] Author - Source Two.pdf".to_string(),
        );

        let categories = make_categories(&sources_path, files);

        create_source_file(&sources_path, "[2022] Author - Source One.pdf").await;
        create_source_file(&sources_path, "[2022] Author - Source Two.pdf").await;

        create_concept_card(&cards_path, "card-1.md", "Source One").await;
        create_concept_card(&cards_path, "card-2.md", "Source One").await;
        create_concept_card(&cards_path, "card-3.md", "Source Two").await;
        create_concept_card(&cards_path, "card-4.md", "Unknown Source").await;

        let report = validate_sources(&cards_path, &categories, ValidationMode::All, false, 0.7)
            .await
            .unwrap();

        assert_eq!(report.stats.total_cards_scanned, 4);
        assert_eq!(report.stats.unique_sources_found, 3);
        assert_eq!(report.stats.sources_in_config, 2);
        assert_eq!(report.stats.sources_resolved, 2);
        assert_eq!(report.stats.missing_from_config, 1);
        assert_eq!(report.stats.sources_on_disk, 2);
        assert_eq!(report.stats.missing_from_disk, 0);
    }

    #[test]
    fn test_validation_mode_default() {
        let mode = ValidationMode::default();
        assert_eq!(mode, ValidationMode::All);
    }

    #[tokio::test]
    async fn test_check_filesystem_empty_path() {
        let categories = HashMap::new();
        let missing = check_filesystem(&categories).await;
        assert!(missing.is_empty());
    }

    #[tokio::test]
    async fn test_check_filesystem_skips_empty_path_category() {
        let mut categories = HashMap::new();
        let mut files = HashMap::new();
        files.insert("some-file".to_string(), "file.pdf".to_string());
        categories.insert(
            "empty".to_string(),
            SourceCategory {
                path: String::new(),
                files,
                aliases: HashMap::new(),
            },
        );

        let missing = check_filesystem(&categories).await;
        assert!(missing.is_empty());
    }
}
