//! Source reference scanner for concept cards.
//!
//! Scans concept card Markdown files to extract source references from
//! frontmatter, building a de-duplicated map of source titles to card IDs.

use std::collections::HashMap;
use std::path::Path;

use fabryk_core::Result;
use fabryk_core::util::files::{FindOptions, find_all_files};

use crate::concept_card::ConceptCardFrontmatter;
use crate::sources::types::SourceReference;

/// Scan all concept cards and extract source references.
///
/// Returns a map of source titles to their references (including card IDs).
/// Sources without a title in the frontmatter are excluded.
///
/// # Arguments
///
/// * `content_path` - Path to the concept cards directory
///
/// # Returns
///
/// A `HashMap` where:
/// - Key: Source title (e.g., "Open Music Theory")
/// - Value: [`SourceReference`] containing the title and list of card IDs
///
/// # Errors
///
/// Returns `Err` if file scanning fails.
pub async fn scan_content_for_sources(
    content_path: &Path,
) -> Result<HashMap<String, SourceReference>> {
    let mut sources: HashMap<String, SourceReference> = HashMap::new();

    let files = find_all_files(content_path, FindOptions::markdown()).await?;

    for file_info in files {
        if let Some(source_title) = extract_source_from_file(&file_info.path).await {
            let source_title = source_title.trim().to_string();
            if !source_title.is_empty() {
                sources
                    .entry(source_title.clone())
                    .or_insert_with(|| SourceReference::new(&source_title))
                    .add_card(&file_info.stem);
            }
        }
    }

    Ok(sources)
}

/// Scan statistics from a scan operation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanStats {
    /// Total number of concept cards scanned.
    pub total_cards: usize,
    /// Number of unique sources found.
    pub unique_sources: usize,
    /// Number of cards with source references.
    pub cards_with_sources: usize,
}

impl ScanStats {
    /// Create new scan statistics.
    pub fn new(total_cards: usize, unique_sources: usize, cards_with_sources: usize) -> Self {
        Self {
            total_cards,
            unique_sources,
            cards_with_sources,
        }
    }
}

/// Scan concept cards and return both references and statistics.
///
/// This is an extended version of [`scan_content_for_sources`] that also
/// computes statistics about the scan.
pub async fn scan_content_for_sources_with_stats(
    content_path: &Path,
) -> Result<(HashMap<String, SourceReference>, ScanStats)> {
    let mut sources: HashMap<String, SourceReference> = HashMap::new();
    let mut total_cards = 0;
    let mut cards_with_sources = 0;

    let files = find_all_files(content_path, FindOptions::markdown()).await?;

    for file_info in files {
        total_cards += 1;

        if let Some(source_title) = extract_source_from_file(&file_info.path).await {
            let source_title = source_title.trim().to_string();
            if !source_title.is_empty() {
                cards_with_sources += 1;
                sources
                    .entry(source_title.clone())
                    .or_insert_with(|| SourceReference::new(&source_title))
                    .add_card(&file_info.stem);
            }
        }
    }

    let stats = ScanStats::new(total_cards, sources.len(), cards_with_sources);

    Ok((sources, stats))
}

/// Extract the `source` field from a Markdown file's frontmatter.
///
/// Returns `None` if the file cannot be read, has no frontmatter, or
/// the frontmatter lacks a `source` field.
async fn extract_source_from_file(path: &Path) -> Option<String> {
    let content = tokio::fs::read_to_string(path).await.ok()?;
    let result = crate::extract_frontmatter(&content).ok()?;
    let meta: ConceptCardFrontmatter = result.deserialize().ok()??;
    meta.source
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    async fn create_concept_card(
        base_dir: &Path,
        category: &str,
        filename: &str,
        source: Option<&str>,
    ) {
        let dir = base_dir.join(category);
        fs::create_dir_all(&dir).await.unwrap();

        let content = if let Some(src) = source {
            format!(
                "---\ntitle: \"Test Concept\"\ncategory: \"{category}\"\nsource: \"{src}\"\n---\n# Test\n\nContent"
            )
        } else {
            format!(
                "---\ntitle: \"Test Concept\"\ncategory: \"{category}\"\n---\n# Test\n\nContent"
            )
        };

        fs::write(dir.join(filename), content).await.unwrap();
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_empty() {
        let temp_dir = TempDir::new().unwrap();
        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert!(sources.is_empty());
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_single_source() {
        let temp_dir = TempDir::new().unwrap();
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "test-concept.md",
            Some("Open Music Theory"),
        )
        .await;

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains_key("Open Music Theory"));
        assert_eq!(sources["Open Music Theory"].card_count(), 1);
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_multiple_cards_same_source() {
        let temp_dir = TempDir::new().unwrap();
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-1.md",
            Some("Open Music Theory"),
        )
        .await;
        create_concept_card(
            temp_dir.path(),
            "fundamentals",
            "concept-2.md",
            Some("Open Music Theory"),
        )
        .await;

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources["Open Music Theory"].card_count(), 2);
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_multiple_sources() {
        let temp_dir = TempDir::new().unwrap();
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-1.md",
            Some("Open Music Theory"),
        )
        .await;
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-2.md",
            Some("A Geometry of Music"),
        )
        .await;

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert_eq!(sources.len(), 2);
        assert!(sources.contains_key("Open Music Theory"));
        assert!(sources.contains_key("A Geometry of Music"));
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_no_source_field() {
        let temp_dir = TempDir::new().unwrap();
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "with-source.md",
            Some("Open Music Theory"),
        )
        .await;
        create_concept_card(temp_dir.path(), "harmony", "without-source.md", None).await;

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains_key("Open Music Theory"));
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_with_stats() {
        let temp_dir = TempDir::new().unwrap();
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-1.md",
            Some("Open Music Theory"),
        )
        .await;
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-2.md",
            Some("Open Music Theory"),
        )
        .await;
        create_concept_card(
            temp_dir.path(),
            "harmony",
            "concept-3.md",
            Some("A Geometry of Music"),
        )
        .await;
        create_concept_card(temp_dir.path(), "fundamentals", "no-source.md", None).await;

        let (sources, stats) = scan_content_for_sources_with_stats(temp_dir.path())
            .await
            .unwrap();

        assert_eq!(sources.len(), 2);
        assert_eq!(stats.total_cards, 4);
        assert_eq!(stats.unique_sources, 2);
        assert_eq!(stats.cards_with_sources, 3);
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_trims_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path().join("harmony");
        fs::create_dir_all(&dir).await.unwrap();

        let content = "---\ntitle: \"Test\"\nsource: \"  Open Music Theory  \"\n---\n# Test";
        fs::write(dir.join("test.md"), content).await.unwrap();

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources.contains_key("Open Music Theory"));
    }

    #[tokio::test]
    async fn test_scan_content_for_sources_empty_source_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let dir = temp_dir.path().join("harmony");
        fs::create_dir_all(&dir).await.unwrap();

        let content = "---\ntitle: \"Test\"\nsource: \"\"\n---\n# Test";
        fs::write(dir.join("test.md"), content).await.unwrap();

        let sources = scan_content_for_sources(temp_dir.path()).await.unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_scan_stats_new() {
        let stats = ScanStats::new(100, 10, 80);
        assert_eq!(stats.total_cards, 100);
        assert_eq!(stats.unique_sources, 10);
        assert_eq!(stats.cards_with_sources, 80);
    }

    #[test]
    fn test_scan_stats_default() {
        let stats = ScanStats::default();
        assert_eq!(stats.total_cards, 0);
        assert_eq!(stats.unique_sources, 0);
        assert_eq!(stats.cards_with_sources, 0);
    }
}
