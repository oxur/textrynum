//! Filesystem-backed content item provider.
//!
//! [`FsContentItemProvider`] scans a directory of markdown concept cards and
//! implements [`ContentItemProvider`] by reading frontmatter metadata from each
//! file. This is the default provider for domains that store content as
//! individual markdown files organized in category directories.
//!
//! # Directory Layout
//!
//! ```text
//! content/
//! ├── harmony/
//! │   ├── voice-leading.md
//! │   └── chord-substitution.md
//! ├── rhythm/
//! │   └── syncopation.md
//! └── uncategorized.md
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_content::FsContentItemProvider;
//!
//! let provider = FsContentItemProvider::new("/path/to/concepts")
//!     .with_content_type_name("concept", "concepts");
//! let items = provider.list_items(None, Some(10)).await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use fabryk_content::{ConceptCardFrontmatter, extract_first_heading, extract_frontmatter};
use fabryk_core::Result;
use fabryk_core::util::files::{FindOptions, find_all_files, find_file_by_id, read_file};

use crate::traits::{CategoryInfo, ContentItemProvider, FilterMap};

/// Summary representation of a content item.
///
/// Built from markdown frontmatter fields with sensible fallbacks for
/// missing data. Used when listing items without loading full content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItemSummary {
    /// Unique identifier (typically the file stem).
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Top-level category.
    pub category: String,
    /// Narrower classification within the category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subcategory: Option<String>,
    /// Difficulty or depth tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    /// Source publication or reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Chapter title in the source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter: Option<String>,
    /// Extraction confidence score.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_confidence: Option<String>,
    /// Alternative names or synonyms.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Numeric chapter index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<i32>,
    /// Page number in source PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf_page: Option<i32>,
    /// Filesystem path relative to content root.
    pub path: String,
    /// Short preview or description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Full detail of a single content item.
///
/// Contains the complete markdown content for rendering or analysis.
#[derive(Debug, Clone, Serialize)]
pub struct ContentItemDetail {
    /// Unique identifier.
    pub id: String,
    /// Full markdown content of the item.
    pub content: String,
}

/// Filesystem-backed implementation of [`ContentItemProvider`].
///
/// Scans a directory tree of markdown files, extracts frontmatter metadata
/// using [`ConceptCardFrontmatter`], and provides list/get/filter operations.
pub struct FsContentItemProvider {
    content_path: PathBuf,
    content_type_name: String,
    content_type_name_plural: String,
}

impl FsContentItemProvider {
    /// Create a new provider rooted at the given directory.
    pub fn new(content_path: impl Into<PathBuf>) -> Self {
        Self {
            content_path: content_path.into(),
            content_type_name: "item".to_string(),
            content_type_name_plural: "items".to_string(),
        }
    }

    /// Set the content type names used in tool descriptions.
    pub fn with_content_type_name(mut self, name: &str, plural: &str) -> Self {
        self.content_type_name = name.to_string();
        self.content_type_name_plural = plural.to_string();
        self
    }

    /// Build a [`ContentItemSummary`] from a file's content and metadata.
    fn build_summary(
        &self,
        id: &str,
        content: &str,
        relative_path: &str,
        parent_dir: Option<&str>,
    ) -> ContentItemSummary {
        let fm = extract_frontmatter(content)
            .ok()
            .and_then(|r| r.deserialize::<ConceptCardFrontmatter>().ok().flatten());

        let title = fm
            .as_ref()
            .and_then(|f| f.title.clone())
            .or_else(|| fm.as_ref().and_then(|f| f.concept.clone()))
            .or_else(|| extract_first_heading(content).map(|(_, t)| t))
            .unwrap_or_else(|| id.to_string());

        let category = fm
            .as_ref()
            .and_then(|f| f.category.clone())
            .or_else(|| parent_dir.map(|d| d.to_string()))
            .unwrap_or_else(|| "uncategorized".to_string());

        ContentItemSummary {
            id: id.to_string(),
            title,
            category,
            subcategory: fm.as_ref().and_then(|f| f.subcategory.clone()),
            tier: fm.as_ref().and_then(|f| f.tier.clone()),
            source: fm.as_ref().and_then(|f| f.source.clone()),
            chapter: fm.as_ref().and_then(|f| f.chapter.clone()),
            extraction_confidence: fm.as_ref().and_then(|f| f.extraction_confidence.clone()),
            aliases: fm.as_ref().map(|f| f.aliases.clone()).unwrap_or_default(),
            chapter_number: fm.as_ref().and_then(|f| f.chapter_number),
            pdf_page: fm.as_ref().and_then(|f| f.pdf_page),
            path: relative_path.to_string(),
            preview: fm.as_ref().and_then(|f| f.description.clone()),
        }
    }

    /// Load all item summaries from the content directory.
    async fn load_all_summaries(&self) -> Result<Vec<ContentItemSummary>> {
        let files = find_all_files(&self.content_path, FindOptions::markdown()).await?;
        let mut summaries = Vec::with_capacity(files.len());

        for file_info in &files {
            let content = read_file(&file_info.path).await?;
            let relative_path = file_info.relative_path.to_string_lossy().to_string();

            let parent_dir = file_info
                .relative_path
                .parent()
                .and_then(|p| p.to_str())
                .filter(|s| !s.is_empty());

            let summary = self.build_summary(&file_info.stem, &content, &relative_path, parent_dir);
            summaries.push(summary);
        }

        summaries.sort_by(|a, b| {
            a.category
                .cmp(&b.category)
                .then_with(|| a.title.cmp(&b.title))
        });

        Ok(summaries)
    }
}

#[async_trait]
impl ContentItemProvider for FsContentItemProvider {
    type ItemSummary = ContentItemSummary;
    type ItemDetail = ContentItemDetail;

    async fn list_items(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<ContentItemSummary>> {
        let mut summaries = self.load_all_summaries().await?;

        if let Some(cat) = category {
            summaries.retain(|s| s.category.eq_ignore_ascii_case(cat));
        }

        if let Some(limit) = limit {
            summaries.truncate(limit);
        }

        Ok(summaries)
    }

    async fn get_item(&self, id: &str) -> Result<ContentItemDetail> {
        let path = find_file_by_id(
            &self.content_path,
            id,
            FindOptions::markdown().with_patterns(vec!["{id}.md", "{id}/README.md"]),
        )
        .await?;

        let content = read_file(&path).await?;
        Ok(ContentItemDetail {
            id: id.to_string(),
            content,
        })
    }

    async fn list_categories(&self) -> Result<Vec<CategoryInfo>> {
        let summaries = self.load_all_summaries().await?;
        let mut counts: HashMap<String, usize> = HashMap::new();

        for summary in &summaries {
            *counts.entry(summary.category.clone()).or_insert(0) += 1;
        }

        let mut categories: Vec<CategoryInfo> = counts
            .into_iter()
            .map(|(id, count)| {
                let name = id
                    .chars()
                    .next()
                    .map(|c| c.to_uppercase().to_string() + &id[1..])
                    .unwrap_or_else(|| id.clone());
                CategoryInfo {
                    id: id.clone(),
                    name,
                    count,
                    description: None,
                }
            })
            .collect();

        categories.sort_by(|a, b| a.id.cmp(&b.id));

        Ok(categories)
    }

    async fn list_items_filtered(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
        extra_filters: &FilterMap,
    ) -> Result<Vec<ContentItemSummary>> {
        let mut summaries = self.list_items(category, None).await?;

        if let Some(tier) = extra_filters.get("tier").and_then(|v| v.as_str()) {
            summaries.retain(|s| s.tier.as_deref() == Some(tier));
        }

        if let Some(sub) = extra_filters.get("subcategory").and_then(|v| v.as_str()) {
            summaries.retain(|s| s.subcategory.as_deref() == Some(sub));
        }

        if let Some(source) = extra_filters.get("source").and_then(|v| v.as_str()) {
            summaries.retain(|s| s.source.as_deref() == Some(source));
        }

        if let Some(conf) = extra_filters
            .get("extraction_confidence")
            .and_then(|v| v.as_str())
        {
            summaries.retain(|s| s.extraction_confidence.as_deref() == Some(conf));
        }

        if let Some(limit) = limit {
            summaries.truncate(limit);
        }

        Ok(summaries)
    }

    fn content_type_name(&self) -> &str {
        &self.content_type_name
    }

    fn content_type_name_plural(&self) -> &str {
        &self.content_type_name_plural
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

    /// Create a sample markdown file with frontmatter.
    async fn write_concept(dir: &std::path::Path, filename: &str, yaml: &str, body: &str) {
        let content = format!("---\n{yaml}---\n\n{body}");
        fs::write(dir.join(filename), content).await.unwrap();
    }

    /// Build a populated temp directory with concept cards.
    async fn setup_content_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        // Create category directories
        let harmony = temp.path().join("harmony");
        let rhythm = temp.path().join("rhythm");
        fs::create_dir(&harmony).await.unwrap();
        fs::create_dir(&rhythm).await.unwrap();

        write_concept(
            &harmony,
            "voice-leading.md",
            "title: Voice Leading\ncategory: harmony\ntier: foundational\nsubcategory: counterpoint\nsource: Berklee Harmony\nextraction_confidence: high\naliases:\n  - part-writing\nchapter: Voice Leading Basics\nchapter_number: 3\npdf_page: 42\ndescription: Movement of individual voices\n",
            "Voice leading is the art of moving individual voices.",
        )
        .await;

        write_concept(
            &harmony,
            "chord-substitution.md",
            "title: Chord Substitution\ncategory: harmony\ntier: advanced\nsubcategory: reharmonization\nsource: Jazz Theory\nextraction_confidence: medium\ndescription: Replacing chords\n",
            "Chord substitution replaces one chord with another.",
        )
        .await;

        write_concept(
            &rhythm,
            "syncopation.md",
            "title: Syncopation\ncategory: rhythm\ntier: foundational\ndescription: Off-beat emphasis\n",
            "Syncopation emphasizes weak beats.",
        )
        .await;

        temp
    }

    #[tokio::test]
    async fn test_list_items_all() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(None, None).await.unwrap();

        assert_eq!(items.len(), 3);
        // Sorted by category then title
        assert_eq!(items[0].category, "harmony");
        assert_eq!(items[0].title, "Chord Substitution");
        assert_eq!(items[1].category, "harmony");
        assert_eq!(items[1].title, "Voice Leading");
        assert_eq!(items[2].category, "rhythm");
        assert_eq!(items[2].title, "Syncopation");
    }

    #[tokio::test]
    async fn test_list_items_with_category_filter() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(Some("harmony"), None).await.unwrap();

        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.category == "harmony"));
    }

    #[tokio::test]
    async fn test_list_items_with_limit() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(None, Some(2)).await.unwrap();

        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_get_item_by_id() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let detail = provider.get_item("syncopation").await.unwrap();

        assert_eq!(detail.id, "syncopation");
        assert!(detail.content.contains("Syncopation"));
        assert!(detail.content.contains("emphasizes weak beats"));
    }

    #[tokio::test]
    async fn test_get_item_not_found() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let result = provider.get_item("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_categories() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let categories = provider.list_categories().await.unwrap();

        assert_eq!(categories.len(), 2);
        assert_eq!(categories[0].id, "harmony");
        assert_eq!(categories[0].count, 2);
        assert_eq!(categories[1].id, "rhythm");
        assert_eq!(categories[1].count, 1);
    }

    #[tokio::test]
    async fn test_list_items_filtered_by_tier() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let mut filters = FilterMap::new();
        filters.insert(
            "tier".to_string(),
            serde_json::Value::String("foundational".to_string()),
        );

        let items = provider
            .list_items_filtered(None, None, &filters)
            .await
            .unwrap();

        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|i| i.tier.as_deref() == Some("foundational"))
        );
    }

    #[tokio::test]
    async fn test_list_items_filtered_by_subcategory() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let mut filters = FilterMap::new();
        filters.insert(
            "subcategory".to_string(),
            serde_json::Value::String("counterpoint".to_string()),
        );

        let items = provider
            .list_items_filtered(None, None, &filters)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "voice-leading");
    }

    #[tokio::test]
    async fn test_list_items_filtered_by_source() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let mut filters = FilterMap::new();
        filters.insert(
            "source".to_string(),
            serde_json::Value::String("Berklee Harmony".to_string()),
        );

        let items = provider
            .list_items_filtered(None, None, &filters)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Voice Leading");
    }

    #[tokio::test]
    async fn test_list_items_filtered_by_extraction_confidence() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let mut filters = FilterMap::new();
        filters.insert(
            "extraction_confidence".to_string(),
            serde_json::Value::String("high".to_string()),
        );

        let items = provider
            .list_items_filtered(None, None, &filters)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "voice-leading");
    }

    #[tokio::test]
    async fn test_list_items_filtered_with_category_and_limit() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let mut filters = FilterMap::new();
        filters.insert(
            "tier".to_string(),
            serde_json::Value::String("foundational".to_string()),
        );

        let items = provider
            .list_items_filtered(Some("harmony"), Some(1), &filters)
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].category, "harmony");
    }

    #[tokio::test]
    async fn test_empty_directory() {
        let temp = TempDir::new().unwrap();
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(None, None).await.unwrap();
        assert!(items.is_empty());

        let categories = provider.list_categories().await.unwrap();
        assert!(categories.is_empty());
    }

    #[tokio::test]
    async fn test_summary_fields() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(None, None).await.unwrap();
        let vl = items.iter().find(|i| i.id == "voice-leading").unwrap();

        assert_eq!(vl.title, "Voice Leading");
        assert_eq!(vl.category, "harmony");
        assert_eq!(vl.subcategory.as_deref(), Some("counterpoint"));
        assert_eq!(vl.tier.as_deref(), Some("foundational"));
        assert_eq!(vl.source.as_deref(), Some("Berklee Harmony"));
        assert_eq!(vl.chapter.as_deref(), Some("Voice Leading Basics"));
        assert_eq!(vl.extraction_confidence.as_deref(), Some("high"));
        assert_eq!(vl.aliases, vec!["part-writing"]);
        assert_eq!(vl.chapter_number, Some(3));
        assert_eq!(vl.pdf_page, Some(42));
        assert_eq!(vl.preview.as_deref(), Some("Movement of individual voices"));
        assert!(vl.path.contains("voice-leading.md"));
    }

    #[tokio::test]
    async fn test_title_fallback_to_heading() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("test-item.md"),
            "# My Heading\n\nSome content.",
        )
        .await
        .unwrap();

        let provider = FsContentItemProvider::new(temp.path());
        let items = provider.list_items(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "My Heading");
    }

    #[tokio::test]
    async fn test_title_fallback_to_file_stem() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("plain-item.md"), "Just some text.")
            .await
            .unwrap();

        let provider = FsContentItemProvider::new(temp.path());
        let items = provider.list_items(None, None).await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "plain-item");
    }

    #[tokio::test]
    async fn test_category_fallback_to_parent_dir() {
        let temp = TempDir::new().unwrap();
        let subdir = temp.path().join("my-category");
        fs::create_dir(&subdir).await.unwrap();
        fs::write(subdir.join("item.md"), "---\ntitle: Item\n---\n\nContent.")
            .await
            .unwrap();

        let provider = FsContentItemProvider::new(temp.path());
        let items = provider.list_items(None, None).await.unwrap();

        assert_eq!(items[0].category, "my-category");
    }

    #[tokio::test]
    async fn test_category_fallback_to_uncategorized() {
        let temp = TempDir::new().unwrap();
        fs::write(
            temp.path().join("orphan.md"),
            "---\ntitle: Orphan\n---\n\nNo category.",
        )
        .await
        .unwrap();

        let provider = FsContentItemProvider::new(temp.path());
        let items = provider.list_items(None, None).await.unwrap();

        assert_eq!(items[0].category, "uncategorized");
    }

    #[tokio::test]
    async fn test_content_type_names() {
        let provider =
            FsContentItemProvider::new("/tmp/test").with_content_type_name("concept", "concepts");

        assert_eq!(provider.content_type_name(), "concept");
        assert_eq!(provider.content_type_name_plural(), "concepts");
    }

    #[tokio::test]
    async fn test_content_type_names_default() {
        let provider = FsContentItemProvider::new("/tmp/test");

        assert_eq!(provider.content_type_name(), "item");
        assert_eq!(provider.content_type_name_plural(), "items");
    }

    #[tokio::test]
    async fn test_count() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        assert_eq!(provider.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_count_in_category() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        assert_eq!(provider.count_in_category("harmony").await.unwrap(), 2);
        assert_eq!(provider.count_in_category("rhythm").await.unwrap(), 1);
        assert_eq!(provider.count_in_category("missing").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_case_insensitive_category_filter() {
        let temp = setup_content_dir().await;
        let provider = FsContentItemProvider::new(temp.path());

        let items = provider.list_items(Some("HARMONY"), None).await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_get_item_readme_pattern() {
        let temp = TempDir::new().unwrap();
        let item_dir = temp.path().join("my-concept");
        fs::create_dir(&item_dir).await.unwrap();
        fs::write(item_dir.join("README.md"), "# My Concept\n\nDetails.")
            .await
            .unwrap();

        let provider = FsContentItemProvider::new(temp.path());
        let detail = provider.get_item("my-concept").await.unwrap();

        assert_eq!(detail.id, "my-concept");
        assert!(detail.content.contains("My Concept"));
    }

    #[tokio::test]
    async fn test_summary_serialization_skips_none_fields() {
        let summary = ContentItemSummary {
            id: "test".to_string(),
            title: "Test".to_string(),
            category: "general".to_string(),
            subcategory: None,
            tier: None,
            source: None,
            chapter: None,
            extraction_confidence: None,
            aliases: vec![],
            chapter_number: None,
            pdf_page: None,
            path: "test.md".to_string(),
            preview: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        assert!(!json.contains("subcategory"));
        assert!(!json.contains("tier"));
        assert!(!json.contains("source"));
        assert!(!json.contains("aliases"));
        assert!(!json.contains("preview"));
    }
}
