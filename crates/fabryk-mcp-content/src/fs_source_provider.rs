//! Filesystem-backed source provider.
//!
//! [`FsSourceProvider`] scans a directory of converted markdown sources and
//! implements [`SourceProvider`]. It supports both converted sources (markdown
//! chapter directories) and unconverted sources registered via configuration.
//!
//! # Directory Layout
//!
//! ```text
//! sources-md/
//! +-- open-music-theory/
//! |   +-- 01-introduction.md
//! |   +-- 02-intervals.md
//! |   +-- 03-chords.md
//! +-- jazz-theory-book/
//!     +-- 01-basics.md
//!     +-- 02-scales.md
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_content::{FsSourceProvider, UnconvertedSource, SourceFormat};
//!
//! let provider = FsSourceProvider::new("/path/to/sources-md")
//!     .with_unconverted_source(UnconvertedSource {
//!         id: "berklee-harmony".to_string(),
//!         title: "Berklee Harmony".to_string(),
//!         path: "/path/to/berklee-harmony.pdf".into(),
//!         format: SourceFormat::Pdf,
//!     });
//!
//! let sources = provider.list_sources().await?;
//! let chapter = provider.get_chapter("open-music-theory", "02-intervals", None).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use fabryk_content::extract_section_content;
use fabryk_core::Result;

use crate::traits::{ChapterInfo, SourceProvider};

// ============================================================================
// Types
// ============================================================================

/// Summary representation of a source material.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSummary {
    /// Unique identifier (typically the directory name).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// File format of the original source.
    pub format: SourceFormat,
    /// Filesystem path (directory for converted, file for unconverted).
    pub path: String,
    /// Number of chapters available (for converted sources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapters: Option<usize>,
    /// Conversion status.
    pub status: SourceStatus,
}

/// Format of a source material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    /// Markdown (converted).
    Markdown,
    /// PDF document.
    Pdf,
    /// EPUB ebook.
    Epub,
    /// XML document.
    Xml,
}

/// Conversion status of a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceStatus {
    /// Source has been converted to markdown chapters.
    Converted,
    /// Source is registered but not yet converted.
    NotConverted,
}

/// An unconverted source registered from configuration.
#[derive(Debug, Clone)]
pub struct UnconvertedSource {
    /// Unique identifier.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Path to the original file.
    pub path: PathBuf,
    /// Format of the original file.
    pub format: SourceFormat,
}

// ============================================================================
// Provider
// ============================================================================

/// Filesystem-backed implementation of [`SourceProvider`].
///
/// Scans a directory tree where each subdirectory represents a converted source
/// containing markdown chapter files. Unconverted sources can be registered via
/// the builder methods.
pub struct FsSourceProvider {
    /// Path to converted sources (markdown chapters).
    sources_md_path: PathBuf,
    /// Unconverted sources registered from config.
    unconverted_sources: Vec<UnconvertedSource>,
}

impl FsSourceProvider {
    /// Create a new provider rooted at the given sources directory.
    pub fn new(sources_md_path: impl Into<PathBuf>) -> Self {
        Self {
            sources_md_path: sources_md_path.into(),
            unconverted_sources: Vec::new(),
        }
    }

    /// Register an unconverted source.
    pub fn with_unconverted_source(mut self, source: UnconvertedSource) -> Self {
        self.unconverted_sources.push(source);
        self
    }

    /// Register multiple unconverted sources.
    pub fn with_unconverted_sources(mut self, sources: Vec<UnconvertedSource>) -> Self {
        self.unconverted_sources.extend(sources);
        self
    }

    /// Count markdown files in a directory.
    async fn count_md_files(dir: &Path) -> Result<usize> {
        let mut count = 0;
        let mut entries = tokio::fs::read_dir(dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Extract chapter number from a filename like "01-introduction.md".
    fn extract_chapter_number(filename: &str) -> Option<String> {
        let stem = filename.strip_suffix(".md").unwrap_or(filename);
        let prefix: String = stem.chars().take_while(|c| c.is_ascii_digit()).collect();
        if prefix.is_empty() {
            None
        } else {
            Some(prefix)
        }
    }

    /// Extract title from chapter content or derive from filename.
    fn extract_chapter_title(content: &str, filename: &str) -> String {
        // Try first heading
        if let Some((_, heading)) = fabryk_content::extract_first_heading(content) {
            return heading;
        }

        // Fall back to humanizing the filename stem
        let stem = filename.strip_suffix(".md").unwrap_or(filename);
        // Strip leading digits and separators
        let title_part = stem.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '_');
        if title_part.is_empty() {
            humanize_source_id(stem)
        } else {
            humanize_source_id(title_part)
        }
    }
}

#[async_trait]
impl SourceProvider for FsSourceProvider {
    type SourceSummary = SourceSummary;

    async fn list_sources(&self) -> Result<Vec<SourceSummary>> {
        let mut summaries = Vec::new();

        // Scan converted source directories
        if self.sources_md_path.exists() {
            let mut entries = tokio::fs::read_dir(&self.sources_md_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    let id = entry.file_name().to_string_lossy().to_string();
                    let chapter_count = Self::count_md_files(&path).await?;
                    summaries.push(SourceSummary {
                        title: humanize_source_id(&id),
                        id,
                        format: SourceFormat::Markdown,
                        path: path.to_string_lossy().to_string(),
                        chapters: Some(chapter_count),
                        status: SourceStatus::Converted,
                    });
                }
            }
        }

        // Add unconverted sources
        for source in &self.unconverted_sources {
            summaries.push(SourceSummary {
                id: source.id.clone(),
                title: source.title.clone(),
                format: source.format.clone(),
                path: source.path.to_string_lossy().to_string(),
                chapters: None,
                status: SourceStatus::NotConverted,
            });
        }

        summaries.sort_by(|a, b| a.title.cmp(&b.title));

        Ok(summaries)
    }

    async fn get_chapter(
        &self,
        source_id: &str,
        chapter: &str,
        section: Option<&str>,
    ) -> Result<String> {
        let chapter_path = self.sources_md_path.join(source_id).join(format!("{chapter}.md"));

        if !chapter_path.exists() {
            return Err(fabryk_core::Error::not_found(
                "chapter",
                format!("{source_id}/{chapter}"),
            ));
        }

        let content = tokio::fs::read_to_string(&chapter_path).await?;

        if let Some(heading) = section {
            match extract_section_content(&content, heading) {
                Some(section_content) => Ok(section_content),
                None => Err(fabryk_core::Error::not_found(
                    "section",
                    format!("{source_id}/{chapter}#{heading}"),
                )),
            }
        } else {
            Ok(content)
        }
    }

    async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>> {
        let source_dir = self.sources_md_path.join(source_id);

        if !source_dir.exists() || !source_dir.is_dir() {
            return Err(fabryk_core::Error::not_found("source", source_id));
        }

        let mut chapters = Vec::new();
        let mut entries = tokio::fs::read_dir(&source_dir).await?;
        let mut files = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }

        // Sort by filename for natural ordering
        files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        for path in &files {
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let content = tokio::fs::read_to_string(path).await?;

            let title = Self::extract_chapter_title(&content, &filename);
            let number = Self::extract_chapter_number(&filename);

            chapters.push(ChapterInfo {
                id: stem,
                title,
                number,
                available: true,
            });
        }

        Ok(chapters)
    }

    async fn get_source_path(&self, source_id: &str) -> Result<Option<PathBuf>> {
        // Check unconverted sources first
        if let Some(source) = self.unconverted_sources.iter().find(|s| s.id == source_id) {
            return Ok(Some(source.path.clone()));
        }

        // Check converted source directory
        let source_dir = self.sources_md_path.join(source_id);
        if source_dir.exists() && source_dir.is_dir() {
            return Ok(Some(source_dir));
        }

        Ok(None)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert a source directory name to a human-readable title.
///
/// Replaces hyphens and underscores with spaces, then title-cases each word.
///
/// # Examples
///
/// ```
/// use fabryk_mcp_content::humanize_source_id;
///
/// assert_eq!(humanize_source_id("open-music-theory"), "Open Music Theory");
/// assert_eq!(humanize_source_id("jazz_theory_book"), "Jazz Theory Book");
/// ```
pub fn humanize_source_id(id: &str) -> String {
    id.split(|c: char| c == '-' || c == '_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let upper: String = first.to_uppercase().collect();
                    upper + chars.as_str()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Build a temp directory with converted source chapters.
    async fn setup_sources_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Converted source: open-music-theory
        let omt = temp.path().join("open-music-theory");
        fs::create_dir(&omt).await.unwrap();
        fs::write(
            omt.join("01-introduction.md"),
            "# Introduction\n\nWelcome to Open Music Theory.",
        )
        .await
        .unwrap();
        fs::write(
            omt.join("02-intervals.md"),
            "# Intervals\n\n## Simple Intervals\n\nSimple interval content.\n\n## Compound Intervals\n\nCompound interval content.",
        )
        .await
        .unwrap();

        // Converted source: jazz-theory
        let jt = temp.path().join("jazz-theory");
        fs::create_dir(&jt).await.unwrap();
        fs::write(jt.join("01-basics.md"), "# Basics\n\nJazz basics.")
            .await
            .unwrap();

        temp
    }

    #[tokio::test]
    async fn test_list_sources_converted_and_unconverted() {
        let temp = setup_sources_dir().await;

        let provider = FsSourceProvider::new(temp.path()).with_unconverted_source(
            UnconvertedSource {
                id: "berklee-harmony".to_string(),
                title: "Berklee Harmony".to_string(),
                path: PathBuf::from("/tmp/berklee-harmony.pdf"),
                format: SourceFormat::Pdf,
            },
        );

        let sources = provider.list_sources().await.unwrap();

        // 2 converted + 1 unconverted = 3
        assert_eq!(sources.len(), 3);

        // Sorted by title
        assert_eq!(sources[0].title, "Berklee Harmony");
        assert_eq!(sources[1].title, "Jazz Theory");
        assert_eq!(sources[2].title, "Open Music Theory");
    }

    #[tokio::test]
    async fn test_list_sources_chapter_counts() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let sources = provider.list_sources().await.unwrap();
        let omt = sources.iter().find(|s| s.id == "open-music-theory").unwrap();
        let jt = sources.iter().find(|s| s.id == "jazz-theory").unwrap();

        assert_eq!(omt.chapters, Some(2));
        assert_eq!(jt.chapters, Some(1));
    }

    #[tokio::test]
    async fn test_list_sources_unconverted_has_no_chapters() {
        let temp = TempDir::new().unwrap();

        let provider = FsSourceProvider::new(temp.path()).with_unconverted_source(
            UnconvertedSource {
                id: "some-book".to_string(),
                title: "Some Book".to_string(),
                path: PathBuf::from("/tmp/some-book.epub"),
                format: SourceFormat::Epub,
            },
        );

        let sources = provider.list_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert!(sources[0].chapters.is_none());
    }

    #[tokio::test]
    async fn test_get_chapter_content() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let content = provider
            .get_chapter("open-music-theory", "01-introduction", None)
            .await
            .unwrap();

        assert!(content.contains("Introduction"));
        assert!(content.contains("Welcome to Open Music Theory"));
    }

    #[tokio::test]
    async fn test_get_chapter_with_section() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let content = provider
            .get_chapter("open-music-theory", "02-intervals", Some("Simple Intervals"))
            .await
            .unwrap();

        assert!(content.contains("Simple interval content"));
        assert!(!content.contains("Compound interval content"));
    }

    #[tokio::test]
    async fn test_get_chapter_missing_section() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let result = provider
            .get_chapter("open-music-theory", "02-intervals", Some("Nonexistent"))
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_chapter_missing_source() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let result = provider
            .get_chapter("nonexistent-source", "01-intro", None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_chapter_missing_chapter() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let result = provider
            .get_chapter("open-music-theory", "99-missing", None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_chapters() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let chapters = provider.list_chapters("open-music-theory").await.unwrap();

        assert_eq!(chapters.len(), 2);
        assert_eq!(chapters[0].id, "01-introduction");
        assert_eq!(chapters[0].title, "Introduction");
        assert_eq!(chapters[0].number, Some("01".to_string()));
        assert!(chapters[0].available);

        assert_eq!(chapters[1].id, "02-intervals");
        assert_eq!(chapters[1].title, "Intervals");
        assert_eq!(chapters[1].number, Some("02".to_string()));
    }

    #[tokio::test]
    async fn test_list_chapters_missing_source() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let result = provider.list_chapters("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_source_path_converted() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        let path = provider
            .get_source_path("open-music-theory")
            .await
            .unwrap();

        assert!(path.is_some());
        assert!(path.unwrap().ends_with("open-music-theory"));
    }

    #[tokio::test]
    async fn test_get_source_path_unconverted() {
        let temp = TempDir::new().unwrap();
        let pdf_path = PathBuf::from("/tmp/berklee.pdf");

        let provider = FsSourceProvider::new(temp.path()).with_unconverted_source(
            UnconvertedSource {
                id: "berklee".to_string(),
                title: "Berklee".to_string(),
                path: pdf_path.clone(),
                format: SourceFormat::Pdf,
            },
        );

        let path = provider.get_source_path("berklee").await.unwrap();
        assert_eq!(path, Some(pdf_path));
    }

    #[tokio::test]
    async fn test_get_source_path_not_found() {
        let temp = TempDir::new().unwrap();
        let provider = FsSourceProvider::new(temp.path());

        let path = provider.get_source_path("nonexistent").await.unwrap();
        assert!(path.is_none());
    }

    #[tokio::test]
    async fn test_humanize_source_id() {
        assert_eq!(humanize_source_id("open-music-theory"), "Open Music Theory");
        assert_eq!(humanize_source_id("jazz_theory_book"), "Jazz Theory Book");
        assert_eq!(
            humanize_source_id("some--double-dash"),
            "Some Double Dash"
        );
        assert_eq!(humanize_source_id("single"), "Single");
        assert_eq!(humanize_source_id(""), "");
    }

    #[tokio::test]
    async fn test_with_unconverted_sources_batch() {
        let temp = TempDir::new().unwrap();

        let sources = vec![
            UnconvertedSource {
                id: "a".to_string(),
                title: "Alpha".to_string(),
                path: PathBuf::from("/tmp/a.pdf"),
                format: SourceFormat::Pdf,
            },
            UnconvertedSource {
                id: "b".to_string(),
                title: "Beta".to_string(),
                path: PathBuf::from("/tmp/b.epub"),
                format: SourceFormat::Epub,
            },
        ];

        let provider = FsSourceProvider::new(temp.path()).with_unconverted_sources(sources);
        let listed = provider.list_sources().await.unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].title, "Alpha");
        assert_eq!(listed[1].title, "Beta");
    }

    #[tokio::test]
    async fn test_source_summary_serialization() {
        let summary = SourceSummary {
            id: "test".to_string(),
            title: "Test".to_string(),
            format: SourceFormat::Pdf,
            path: "/tmp/test.pdf".to_string(),
            chapters: None,
            status: SourceStatus::NotConverted,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("chapters"));
        assert!(json.contains("not_converted"));
        assert!(json.contains("pdf"));

        let deserialized: SourceSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "test");
    }

    #[tokio::test]
    async fn test_source_format_serialization() {
        let formats = vec![
            (SourceFormat::Markdown, "\"markdown\""),
            (SourceFormat::Pdf, "\"pdf\""),
            (SourceFormat::Epub, "\"epub\""),
            (SourceFormat::Xml, "\"xml\""),
        ];

        for (format, expected) in formats {
            let json = serde_json::to_string(&format).unwrap();
            assert_eq!(json, expected);
        }
    }

    #[tokio::test]
    async fn test_chapter_number_extraction() {
        assert_eq!(
            FsSourceProvider::extract_chapter_number("01-introduction.md"),
            Some("01".to_string())
        );
        assert_eq!(
            FsSourceProvider::extract_chapter_number("12-advanced.md"),
            Some("12".to_string())
        );
        assert_eq!(
            FsSourceProvider::extract_chapter_number("preface.md"),
            None
        );
    }

    #[tokio::test]
    async fn test_is_available() {
        let temp = setup_sources_dir().await;
        let provider = FsSourceProvider::new(temp.path());

        assert!(provider.is_available("open-music-theory").await.unwrap());
        assert!(!provider.is_available("nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_empty_sources_directory() {
        let temp = TempDir::new().unwrap();
        let provider = FsSourceProvider::new(temp.path());

        let sources = provider.list_sources().await.unwrap();
        assert!(sources.is_empty());
    }
}
