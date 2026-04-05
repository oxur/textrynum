//! Filesystem-backed guide provider.
//!
//! [`FsGuideProvider`] scans a directory of markdown guide files and implements
//! [`GuideProvider`]. Guides are standalone documents (topic overviews,
//! tutorials, reference sheets) that complement the primary content items.
//!
//! # Directory Layout
//!
//! ```text
//! guides/
//! ├── getting-started/
//! │   ├── README.md
//! │   └── first-steps.md
//! ├── harmony/
//! │   └── chord-voicings.md
//! └── overview.md
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_content::FsGuideProvider;
//!
//! let provider = FsGuideProvider::new("/path/to/guides");
//! let guides = provider.list_guides().await?;
//! let content = provider.get_guide("chord-voicings").await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use fabryk_content::{extract_first_heading, extract_first_paragraph, extract_frontmatter};
use fabryk_core::util::files::{find_all_files, find_file_by_id, read_file, FindOptions};
use fabryk_core::Result;

use crate::traits::GuideProvider;

/// Summary representation of a guide document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuideSummary {
    /// Unique identifier (typically the file stem).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Topic area derived from parent directory.
    pub topic: String,
    /// Relative filesystem path.
    pub path: String,
    /// Short description of the guide.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Filesystem-backed implementation of [`GuideProvider`].
///
/// Scans a directory tree for markdown guide files. Topic is derived from the
/// parent directory relative to the guides root. Titles and descriptions are
/// extracted from frontmatter or markdown content.
pub struct FsGuideProvider {
    guides_path: PathBuf,
    patterns: Vec<String>,
}

impl FsGuideProvider {
    /// Create a new provider rooted at the given guides directory.
    pub fn new(guides_path: impl Into<PathBuf>) -> Self {
        Self {
            guides_path: guides_path.into(),
            patterns: vec![
                "{id}.md".to_string(),
                "{id}/README.md".to_string(),
                "{id}/index.md".to_string(),
            ],
        }
    }

    /// Override the default file-resolution patterns.
    ///
    /// Patterns use `{id}` as a placeholder for the guide identifier.
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Extract title from markdown content with fallbacks.
    ///
    /// Priority: frontmatter title -> first heading -> filename stem.
    fn extract_title(content: &str, fallback: &str) -> String {
        // Try frontmatter title
        if let Ok(result) = extract_frontmatter(content) {
            if let Ok(Some(fm)) = result.deserialize::<GuideFrontmatter>() {
                if let Some(title) = fm.title {
                    return title;
                }
            }
        }

        // Try first heading
        if let Some((_, heading)) = extract_first_heading(content) {
            return heading;
        }

        fallback.to_string()
    }

    /// Extract description from markdown content with fallbacks.
    ///
    /// Priority: frontmatter description -> first paragraph.
    fn extract_description(content: &str) -> Option<String> {
        // Try frontmatter description
        if let Ok(result) = extract_frontmatter(content) {
            if let Ok(Some(fm)) = result.deserialize::<GuideFrontmatter>() {
                if fm.description.is_some() {
                    return fm.description;
                }
            }
        }

        // Try first paragraph (limited to 200 chars)
        extract_first_paragraph(content, 200)
    }
}

/// Minimal frontmatter schema for guides.
#[derive(Debug, Deserialize)]
struct GuideFrontmatter {
    title: Option<String>,
    description: Option<String>,
}

#[async_trait]
impl GuideProvider for FsGuideProvider {
    type GuideSummary = GuideSummary;

    async fn list_guides(&self) -> Result<Vec<GuideSummary>> {
        let files = find_all_files(&self.guides_path, FindOptions::markdown()).await?;
        let mut summaries = Vec::with_capacity(files.len());

        for file_info in &files {
            let content = read_file(&file_info.path).await?;
            let relative_path = file_info.relative_path.to_string_lossy().to_string();

            let topic = file_info
                .relative_path
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("general")
                .to_string();

            let title = Self::extract_title(&content, &file_info.stem);
            let description = Self::extract_description(&content);

            summaries.push(GuideSummary {
                id: file_info.stem.clone(),
                title,
                topic,
                path: relative_path,
                description,
            });
        }

        summaries.sort_by(|a, b| a.topic.cmp(&b.topic).then_with(|| a.title.cmp(&b.title)));

        Ok(summaries)
    }

    async fn get_guide(&self, id: &str) -> Result<String> {
        let pattern_refs: Vec<&str> = self.patterns.iter().map(|s| s.as_str()).collect();
        let path = find_file_by_id(
            &self.guides_path,
            id,
            FindOptions::markdown().with_patterns(pattern_refs),
        )
        .await?;

        read_file(&path).await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Create a markdown guide file.
    async fn write_guide(dir: &std::path::Path, filename: &str, content: &str) {
        fs::write(dir.join(filename), content).await.unwrap();
    }

    /// Build a populated temp directory with guide files.
    async fn setup_guides_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Root-level guide
        write_guide(
            temp.path(),
            "overview.md",
            "---\ntitle: Overview\ndescription: A broad overview of the system.\n---\n\nThis is the overview.",
        )
        .await;

        // Topic subdirectories
        let harmony = temp.path().join("harmony");
        let rhythm = temp.path().join("rhythm");
        fs::create_dir(&harmony).await.unwrap();
        fs::create_dir(&rhythm).await.unwrap();

        write_guide(
            &harmony,
            "chord-voicings.md",
            "---\ntitle: Chord Voicings\ndescription: How to voice chords.\n---\n\nVoicing details.",
        )
        .await;

        write_guide(
            &harmony,
            "progressions.md",
            "# Common Progressions\n\nA guide to common chord progressions.",
        )
        .await;

        write_guide(
            &rhythm,
            "time-signatures.md",
            "---\ntitle: Time Signatures\n---\n\nExplaining time signatures in depth.",
        )
        .await;

        temp
    }

    #[tokio::test]
    async fn test_list_guides() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();

        assert_eq!(guides.len(), 4);
        // Sorted by topic then title
        assert_eq!(guides[0].topic, "general");
        assert_eq!(guides[0].title, "Overview");
    }

    #[tokio::test]
    async fn test_list_guides_sorted_by_topic_then_title() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();

        let topics: Vec<&str> = guides.iter().map(|g| g.topic.as_str()).collect();
        // "general" < "harmony" < "rhythm"
        assert!(topics.windows(2).all(|w| w[0] <= w[1]));
    }

    #[tokio::test]
    async fn test_get_guide_by_id() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let content = provider.get_guide("chord-voicings").await.unwrap();

        assert!(content.contains("Chord Voicings"));
        assert!(content.contains("Voicing details"));
    }

    #[tokio::test]
    async fn test_get_guide_not_found() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let result = provider.get_guide("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_guide_title_from_frontmatter() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let voicings = guides.iter().find(|g| g.id == "chord-voicings").unwrap();

        assert_eq!(voicings.title, "Chord Voicings");
    }

    #[tokio::test]
    async fn test_guide_title_from_heading() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let progs = guides.iter().find(|g| g.id == "progressions").unwrap();

        assert_eq!(progs.title, "Common Progressions");
    }

    #[tokio::test]
    async fn test_guide_title_fallback_to_stem() {
        let temp = TempDir::new().unwrap();
        write_guide(temp.path(), "plain-guide.md", "Just plain text.").await;

        let provider = FsGuideProvider::new(temp.path());
        let guides = provider.list_guides().await.unwrap();

        assert_eq!(guides[0].title, "plain-guide");
    }

    #[tokio::test]
    async fn test_guide_description_from_frontmatter() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let overview = guides.iter().find(|g| g.id == "overview").unwrap();

        assert_eq!(
            overview.description.as_deref(),
            Some("A broad overview of the system.")
        );
    }

    #[tokio::test]
    async fn test_guide_description_fallback_to_paragraph() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let progs = guides.iter().find(|g| g.id == "progressions").unwrap();

        assert!(progs.description.is_some());
        assert!(
            progs
                .description
                .as_deref()
                .unwrap()
                .contains("chord progressions")
        );
    }

    #[tokio::test]
    async fn test_guide_topic_from_parent_dir() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let voicings = guides.iter().find(|g| g.id == "chord-voicings").unwrap();

        assert_eq!(voicings.topic, "harmony");
    }

    #[tokio::test]
    async fn test_guide_topic_defaults_to_general() {
        let temp = setup_guides_dir().await;
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        let overview = guides.iter().find(|g| g.id == "overview").unwrap();

        assert_eq!(overview.topic, "general");
    }

    #[tokio::test]
    async fn test_empty_guides_directory() {
        let temp = TempDir::new().unwrap();
        let provider = FsGuideProvider::new(temp.path());

        let guides = provider.list_guides().await.unwrap();
        assert!(guides.is_empty());
    }

    #[tokio::test]
    async fn test_get_guide_readme_pattern() {
        let temp = TempDir::new().unwrap();
        let guide_dir = temp.path().join("my-guide");
        fs::create_dir(&guide_dir).await.unwrap();
        fs::write(guide_dir.join("README.md"), "# My Guide\n\nGuide content.")
            .await
            .unwrap();

        let provider = FsGuideProvider::new(temp.path());
        let content = provider.get_guide("my-guide").await.unwrap();

        assert!(content.contains("My Guide"));
    }

    #[tokio::test]
    async fn test_get_guide_index_pattern() {
        let temp = TempDir::new().unwrap();
        let guide_dir = temp.path().join("my-topic");
        fs::create_dir(&guide_dir).await.unwrap();
        fs::write(
            guide_dir.join("index.md"),
            "# Topic Index\n\nIndex content.",
        )
        .await
        .unwrap();

        let provider = FsGuideProvider::new(temp.path());
        let content = provider.get_guide("my-topic").await.unwrap();

        assert!(content.contains("Topic Index"));
    }

    #[tokio::test]
    async fn test_custom_patterns() {
        let temp = TempDir::new().unwrap();
        let guide_dir = temp.path().join("custom");
        fs::create_dir(&guide_dir).await.unwrap();
        fs::write(
            guide_dir.join("guide.md"),
            "# Custom Guide\n\nCustom content.",
        )
        .await
        .unwrap();

        let provider = FsGuideProvider::new(temp.path())
            .with_patterns(vec!["{id}/guide.md".to_string()]);

        let content = provider.get_guide("custom").await.unwrap();
        assert!(content.contains("Custom Guide"));
    }

    #[tokio::test]
    async fn test_guide_summary_serialization_skips_none() {
        let summary = GuideSummary {
            id: "test".to_string(),
            title: "Test".to_string(),
            topic: "general".to_string(),
            path: "test.md".to_string(),
            description: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("description"));
    }

    #[tokio::test]
    async fn test_guide_summary_roundtrip() {
        let summary = GuideSummary {
            id: "test".to_string(),
            title: "Test Guide".to_string(),
            topic: "harmony".to_string(),
            path: "harmony/test.md".to_string(),
            description: Some("A test guide.".to_string()),
        };

        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: GuideSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, "test");
        assert_eq!(deserialized.title, "Test Guide");
        assert_eq!(deserialized.topic, "harmony");
        assert_eq!(deserialized.description.as_deref(), Some("A test guide."));
    }
}
