//! Traits for content item and source material providers.
//!
//! These traits enable domain-agnostic MCP tools for content operations.
//! Each domain implements these traits with its own types.

use async_trait::async_trait;
use fabryk_core::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A map of extra domain-specific filter key-value pairs.
///
/// Used by [`ContentItemProvider::list_items_filtered`] to pass
/// arbitrary filters (tier, subcategory, confidence, etc.) without
/// making the trait domain-specific.
pub type FilterMap = serde_json::Map<String, serde_json::Value>;

/// Information about a content category.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryInfo {
    /// Category identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Number of items in this category.
    pub count: usize,
    /// Optional description.
    pub description: Option<String>,
}

/// Information about a chapter in a source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChapterInfo {
    /// Chapter identifier.
    pub id: String,
    /// Chapter title.
    pub title: String,
    /// Chapter number (if applicable).
    pub number: Option<String>,
    /// Whether content is available.
    pub available: bool,
}

/// Trait for providing domain-specific content item access.
///
/// Each domain implements this to define how its content items
/// are listed, retrieved, and described via MCP tools.
///
/// # Example
///
/// ```rust,ignore
/// struct MyContentProvider { /* ... */ }
///
/// #[async_trait]
/// impl ContentItemProvider for MyContentProvider {
///     type ItemSummary = MyItemInfo;
///     type ItemDetail = MyItemDetail;
///
///     async fn list_items(&self, category: Option<&str>, limit: Option<usize>)
///         -> Result<Vec<Self::ItemSummary>> {
///         // Return item summaries, optionally filtered
///     }
///
///     async fn get_item(&self, id: &str) -> Result<Self::ItemDetail> {
///         // Return full item detail
///     }
///
///     async fn list_categories(&self) -> Result<Vec<CategoryInfo>> {
///         // Return available categories
///     }
/// }
/// ```
#[async_trait]
pub trait ContentItemProvider: Send + Sync {
    /// Summary type returned when listing items.
    type ItemSummary: Serialize + Send + Sync;

    /// Detail type returned when getting a single item.
    type ItemDetail: Serialize + Send + Sync;

    /// List all items, optionally filtered by category.
    async fn list_items(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<Self::ItemSummary>>;

    /// Get a single item by ID.
    async fn get_item(&self, id: &str) -> Result<Self::ItemDetail>;

    /// List available categories with counts.
    async fn list_categories(&self) -> Result<Vec<CategoryInfo>>;

    /// Get total item count.
    async fn count(&self) -> Result<usize> {
        Ok(self.list_items(None, None).await?.len())
    }

    /// Get item count for a specific category.
    async fn count_in_category(&self, category: &str) -> Result<usize> {
        Ok(self.list_items(Some(category), None).await?.len())
    }

    /// Returns the content type name for this provider (e.g., "concept").
    fn content_type_name(&self) -> &str {
        "item"
    }

    /// Returns the plural content type name (e.g., "concepts").
    fn content_type_name_plural(&self) -> &str {
        "items"
    }

    /// List items with extended domain-specific filters.
    ///
    /// The default implementation ignores extra filters and delegates to
    /// [`list_items`](Self::list_items). Domain implementations override
    /// this to apply filters like tier, subcategory, confidence, etc.
    async fn list_items_filtered(
        &self,
        category: Option<&str>,
        limit: Option<usize>,
        _extra_filters: &FilterMap,
    ) -> Result<Vec<Self::ItemSummary>> {
        self.list_items(category, limit).await
    }
}

/// Trait for providing source material access.
///
/// Sources are reference materials like books, papers, or documentation
/// that the domain knowledge is derived from.
///
/// # Example
///
/// ```rust,ignore
/// struct MySourceProvider { /* ... */ }
///
/// #[async_trait]
/// impl SourceProvider for MySourceProvider {
///     type SourceSummary = MySourceInfo;
///
///     async fn list_sources(&self) -> Result<Vec<Self::SourceSummary>> {
///         // Return source summaries
///     }
///
///     async fn get_chapter(&self, source_id: &str, chapter: &str, section: Option<&str>)
///         -> Result<String> {
///         // Return chapter content
///     }
///
///     async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>> {
///         // Return chapter listing
///     }
///
///     async fn get_source_path(&self, source_id: &str) -> Result<Option<PathBuf>> {
///         // Return filesystem path to source
///     }
/// }
/// ```
#[async_trait]
pub trait SourceProvider: Send + Sync {
    /// Summary type for source listings.
    type SourceSummary: Serialize + Send + Sync;

    /// List all source materials with availability status.
    async fn list_sources(&self) -> Result<Vec<Self::SourceSummary>>;

    /// Get a specific chapter from a source.
    async fn get_chapter(
        &self,
        source_id: &str,
        chapter: &str,
        section: Option<&str>,
    ) -> Result<String>;

    /// List chapters for a source.
    async fn list_chapters(&self, source_id: &str) -> Result<Vec<ChapterInfo>>;

    /// Get filesystem path to source PDF/EPUB.
    ///
    /// Returns None if source is not available locally.
    async fn get_source_path(&self, source_id: &str) -> Result<Option<PathBuf>>;

    /// Check if a source is available.
    async fn is_available(&self, source_id: &str) -> Result<bool> {
        Ok(self.get_source_path(source_id).await?.is_some())
    }
}

/// Trait for providing guide/tutorial/reference document access.
///
/// Guides are standalone markdown documents (topic overviews, tutorials,
/// reference sheets) that complement the primary content items.
///
/// # Example
///
/// ```rust,ignore
/// struct MyGuideProvider { /* ... */ }
///
/// #[async_trait]
/// impl GuideProvider for MyGuideProvider {
///     type GuideSummary = MyGuideSummary;
///
///     async fn list_guides(&self) -> Result<Vec<Self::GuideSummary>> {
///         // Return guide summaries
///     }
///
///     async fn get_guide(&self, id: &str) -> Result<String> {
///         // Return full markdown content
///     }
/// }
/// ```
#[async_trait]
pub trait GuideProvider: Send + Sync {
    /// Summary type returned when listing guides.
    type GuideSummary: Serialize + Send + Sync;

    /// List all available guides.
    async fn list_guides(&self) -> Result<Vec<Self::GuideSummary>>;

    /// Get a guide's full content by ID.
    async fn get_guide(&self, id: &str) -> Result<String>;

    /// Returns the guide type name (e.g., "guide", "tutorial").
    fn guide_type_name(&self) -> &str {
        "guide"
    }

    /// Returns the plural guide type name.
    fn guide_type_name_plural(&self) -> &str {
        "guides"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_category_info_serialization() {
        let cat = CategoryInfo {
            id: "harmony".to_string(),
            name: "Harmony".to_string(),
            count: 42,
            description: Some("Harmonic concepts".to_string()),
        };
        let json = serde_json::to_string(&cat).unwrap();
        assert!(json.contains("harmony"));
        assert!(json.contains("42"));

        let deserialized: CategoryInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "harmony");
        assert_eq!(deserialized.count, 42);
    }

    #[test]
    fn test_category_info_without_description() {
        let cat = CategoryInfo {
            id: "rhythm".to_string(),
            name: "Rhythm".to_string(),
            count: 10,
            description: None,
        };
        let json = serde_json::to_string(&cat).unwrap();
        let deserialized: CategoryInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.description.is_none());
    }

    #[test]
    fn test_chapter_info_serialization() {
        let chapter = ChapterInfo {
            id: "chapter-1".to_string(),
            title: "Introduction".to_string(),
            number: Some("1".to_string()),
            available: true,
        };
        let json = serde_json::to_string(&chapter).unwrap();
        assert!(json.contains("Introduction"));

        let deserialized: ChapterInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "chapter-1");
        assert!(deserialized.available);
    }

    #[test]
    fn test_chapter_info_without_number() {
        let chapter = ChapterInfo {
            id: "preface".to_string(),
            title: "Preface".to_string(),
            number: None,
            available: true,
        };
        let json = serde_json::to_string(&chapter).unwrap();
        let deserialized: ChapterInfo = serde_json::from_str(&json).unwrap();
        assert!(deserialized.number.is_none());
    }

    // -- GuideProvider summary serialization tests ----------------------------

    #[derive(Clone, Debug, Serialize, Deserialize)]
    struct MockGuideSummary {
        id: String,
        title: String,
    }

    #[test]
    fn test_guide_summary_serialization() {
        let summary = MockGuideSummary {
            id: "intro".to_string(),
            title: "Introduction".to_string(),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("intro"));
        assert!(json.contains("Introduction"));

        let deserialized: MockGuideSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "intro");
        assert_eq!(deserialized.title, "Introduction");
    }

    #[test]
    fn test_guide_summary_list_serialization() {
        let summaries = vec![
            MockGuideSummary {
                id: "intro".to_string(),
                title: "Introduction".to_string(),
            },
            MockGuideSummary {
                id: "advanced".to_string(),
                title: "Advanced Topics".to_string(),
            },
        ];
        let json = serde_json::to_string(&summaries).unwrap();
        let deserialized: Vec<MockGuideSummary> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 2);
        assert_eq!(deserialized[0].id, "intro");
        assert_eq!(deserialized[1].id, "advanced");
    }
}
