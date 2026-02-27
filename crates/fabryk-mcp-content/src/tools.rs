//! Generic MCP tools for content operations.
//!
//! Provides `ContentTools<P>` and `SourceTools<P>` that implement
//! `ToolRegistry` by delegating to domain-specific providers.

use crate::traits::{ContentItemProvider, SourceProvider};
use fabryk_mcp::error::McpErrorExt;
use fabryk_mcp::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp::registry::{ToolRegistry, ToolResult};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value::Object` to an `Arc<serde_json::Map>`.
fn json_schema(value: Value) -> Arc<serde_json::Map<String, Value>> {
    match value {
        Value::Object(map) => Arc::new(map),
        _ => Arc::new(serde_json::Map::new()),
    }
}

/// Serialize a value to a successful `CallToolResult`.
fn serialize_response<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Build a `Tool` with a JSON schema.
fn make_tool(name: &str, description: &str, schema: Value) -> Tool {
    Tool {
        name: name.to_string().into(),
        description: Some(description.to_string().into()),
        input_schema: json_schema(schema),
        title: None,
        output_schema: None,
        annotations: None,
        icons: None,
        execution: None,
        meta: None,
    }
}

// ---------------------------------------------------------------------------
// Argument types
// ---------------------------------------------------------------------------

/// Arguments for list_items tool.
#[derive(Debug, Deserialize)]
pub struct ListItemsArgs {
    /// Optional category filter.
    pub category: Option<String>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Arguments for get_item tool.
#[derive(Debug, Deserialize)]
pub struct GetItemArgs {
    /// Item identifier.
    pub id: String,
}

/// Arguments for get_chapter tool.
#[derive(Debug, Deserialize)]
pub struct GetChapterArgs {
    /// Source identifier.
    pub source_id: String,
    /// Chapter identifier.
    pub chapter: String,
    /// Optional section within chapter.
    pub section: Option<String>,
}

/// Arguments for list_chapters tool.
#[derive(Debug, Deserialize)]
pub struct ListChaptersArgs {
    /// Source identifier.
    pub source_id: String,
}

// ---------------------------------------------------------------------------
// ContentTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `ContentItemProvider`.
///
/// Generates three tools:
/// - `{prefix}_list` — list items with optional category filter
/// - `{prefix}_get` — get a specific item by ID
/// - `{prefix}_categories` — list available categories
///
/// # Example
///
/// ```rust,ignore
/// let tools = ContentTools::new(my_provider).with_prefix("concepts");
/// // Generates: concepts_list, concepts_get, concepts_categories
/// ```
pub struct ContentTools<P: ContentItemProvider> {
    provider: Arc<P>,
    tool_prefix: String,
}

impl<P: ContentItemProvider + 'static> ContentTools<P> {
    /// Create new content tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            tool_prefix: String::new(),
        }
    }

    /// Create content tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self {
            provider,
            tool_prefix: String::new(),
        }
    }

    /// Set a prefix for tool names (e.g., "concepts" → "concepts_list").
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = prefix.into();
        self
    }

    fn tool_name(&self, base: &str) -> String {
        if self.tool_prefix.is_empty() {
            base.to_string()
        } else {
            format!("{}_{}", self.tool_prefix, base)
        }
    }
}

impl<P: ContentItemProvider + 'static> ToolRegistry for ContentTools<P> {
    fn tools(&self) -> Vec<Tool> {
        let type_name = self.provider.content_type_name();
        let type_plural = self.provider.content_type_name_plural();

        vec![
            make_tool(
                &self.tool_name("list"),
                &format!("List all {type_plural} with optional category filter"),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "category": {
                            "type": "string",
                            "description": "Filter by category"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results"
                        }
                    }
                }),
            ),
            make_tool(
                &self.tool_name("get"),
                &format!("Get a specific {type_name} by ID"),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": format!("{type_name} identifier")
                        }
                    },
                    "required": ["id"]
                }),
            ),
            make_tool(
                &self.tool_name("categories"),
                &format!("List available {type_name} categories"),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name("list") {
            return Some(Box::pin(async move {
                let args: ListItemsArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let items = provider
                    .list_items(args.category.as_deref(), args.limit)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&items)
            }));
        }

        if name == self.tool_name("get") {
            return Some(Box::pin(async move {
                let args: GetItemArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let item = provider
                    .get_item(&args.id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&item)
            }));
        }

        if name == self.tool_name("categories") {
            return Some(Box::pin(async move {
                let categories = provider
                    .list_categories()
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&categories)
            }));
        }

        None
    }
}

// ---------------------------------------------------------------------------
// SourceTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `SourceProvider`.
///
/// Generates three tools:
/// - `sources_list` — list all source materials
/// - `sources_chapters` — list chapters in a source
/// - `sources_get_chapter` — get content from a source chapter
pub struct SourceTools<P: SourceProvider> {
    provider: Arc<P>,
}

impl<P: SourceProvider + 'static> SourceTools<P> {
    /// Create new source tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
        }
    }

    /// Create source tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self { provider }
    }
}

impl<P: SourceProvider + 'static> ToolRegistry for SourceTools<P> {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "sources_list",
                "List all source materials",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            make_tool(
                "sources_chapters",
                "List chapters in a source",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        }
                    },
                    "required": ["source_id"]
                }),
            ),
            make_tool(
                "sources_get_chapter",
                "Get content from a source chapter",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        },
                        "chapter": {
                            "type": "string",
                            "description": "Chapter identifier"
                        },
                        "section": {
                            "type": "string",
                            "description": "Optional section within chapter"
                        }
                    },
                    "required": ["source_id", "chapter"]
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        match name {
            "sources_list" => Some(Box::pin(async move {
                let sources = provider
                    .list_sources()
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&sources)
            })),

            "sources_chapters" => Some(Box::pin(async move {
                let args: ListChaptersArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let chapters = provider
                    .list_chapters(&args.source_id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&chapters)
            })),

            "sources_get_chapter" => Some(Box::pin(async move {
                let args: GetChapterArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let content = provider
                    .get_chapter(&args.source_id, &args.chapter, args.section.as_deref())
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                Ok(CallToolResult::success(vec![Content::text(content)]))
            })),

            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{CategoryInfo, ChapterInfo};
    use async_trait::async_trait;
    use serde::Serialize;
    use std::path::PathBuf;

    // -- Mock content types -------------------------------------------------

    #[derive(Clone, Debug, Serialize)]
    struct MockItemSummary {
        id: String,
        title: String,
        category: Option<String>,
    }

    #[derive(Clone, Debug, Serialize)]
    struct MockItemDetail {
        id: String,
        title: String,
        content: String,
    }

    // -- Mock content provider ----------------------------------------------

    struct MockContentProvider;

    #[async_trait]
    impl ContentItemProvider for MockContentProvider {
        type ItemSummary = MockItemSummary;
        type ItemDetail = MockItemDetail;

        async fn list_items(
            &self,
            category: Option<&str>,
            limit: Option<usize>,
        ) -> fabryk_core::Result<Vec<Self::ItemSummary>> {
            let mut items = vec![
                MockItemSummary {
                    id: "item-1".to_string(),
                    title: "First Item".to_string(),
                    category: Some("alpha".to_string()),
                },
                MockItemSummary {
                    id: "item-2".to_string(),
                    title: "Second Item".to_string(),
                    category: Some("beta".to_string()),
                },
                MockItemSummary {
                    id: "item-3".to_string(),
                    title: "Third Item".to_string(),
                    category: Some("alpha".to_string()),
                },
            ];

            if let Some(cat) = category {
                items.retain(|i| i.category.as_deref() == Some(cat));
            }
            if let Some(max) = limit {
                items.truncate(max);
            }
            Ok(items)
        }

        async fn get_item(&self, id: &str) -> fabryk_core::Result<Self::ItemDetail> {
            if id == "item-1" {
                Ok(MockItemDetail {
                    id: "item-1".to_string(),
                    title: "First Item".to_string(),
                    content: "Detailed content here.".to_string(),
                })
            } else {
                Err(fabryk_core::Error::not_found("item", id))
            }
        }

        async fn list_categories(&self) -> fabryk_core::Result<Vec<CategoryInfo>> {
            Ok(vec![
                CategoryInfo {
                    id: "alpha".to_string(),
                    name: "Alpha".to_string(),
                    count: 2,
                    description: None,
                },
                CategoryInfo {
                    id: "beta".to_string(),
                    name: "Beta".to_string(),
                    count: 1,
                    description: None,
                },
            ])
        }

        fn content_type_name(&self) -> &str {
            "concept"
        }

        fn content_type_name_plural(&self) -> &str {
            "concepts"
        }
    }

    // -- Mock source types --------------------------------------------------

    #[derive(Clone, Debug, Serialize)]
    struct MockSourceSummary {
        id: String,
        title: String,
        available: bool,
    }

    // -- Mock source provider -----------------------------------------------

    struct MockSourceProvider;

    #[async_trait]
    impl SourceProvider for MockSourceProvider {
        type SourceSummary = MockSourceSummary;

        async fn list_sources(&self) -> fabryk_core::Result<Vec<Self::SourceSummary>> {
            Ok(vec![MockSourceSummary {
                id: "book-1".to_string(),
                title: "Test Book".to_string(),
                available: true,
            }])
        }

        async fn get_chapter(
            &self,
            source_id: &str,
            chapter: &str,
            _section: Option<&str>,
        ) -> fabryk_core::Result<String> {
            if source_id == "book-1" && chapter == "ch1" {
                Ok("Chapter 1 content.".to_string())
            } else {
                Err(fabryk_core::Error::not_found(
                    "chapter",
                    &format!("{source_id}:{chapter}"),
                ))
            }
        }

        async fn list_chapters(&self, source_id: &str) -> fabryk_core::Result<Vec<ChapterInfo>> {
            if source_id == "book-1" {
                Ok(vec![ChapterInfo {
                    id: "ch1".to_string(),
                    title: "Chapter 1".to_string(),
                    number: Some("1".to_string()),
                    available: true,
                }])
            } else {
                Err(fabryk_core::Error::not_found("source", source_id))
            }
        }

        async fn get_source_path(&self, _source_id: &str) -> fabryk_core::Result<Option<PathBuf>> {
            Ok(None)
        }
    }

    // -- ContentTools tests -------------------------------------------------

    #[test]
    fn test_content_tools_creation() {
        let tools = ContentTools::new(MockContentProvider);
        assert_eq!(tools.tool_count(), 3);
    }

    #[test]
    fn test_content_tools_with_prefix() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let tool_list = tools.tools();
        assert_eq!(tool_list.len(), 3);
        assert_eq!(tool_list[0].name, "concepts_list");
        assert_eq!(tool_list[1].name, "concepts_get");
        assert_eq!(tool_list[2].name, "concepts_categories");
    }

    #[test]
    fn test_content_tools_without_prefix() {
        let tools = ContentTools::new(MockContentProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "list");
        assert_eq!(tool_list[1].name, "get");
        assert_eq!(tool_list[2].name, "categories");
    }

    #[test]
    fn test_content_tools_descriptions() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let tool_list = tools.tools();
        assert!(tool_list[0]
            .description
            .as_ref()
            .unwrap()
            .contains("concepts"));
        assert!(tool_list[1]
            .description
            .as_ref()
            .unwrap()
            .contains("concept"));
    }

    #[test]
    fn test_content_tools_has_tool() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        assert!(tools.has_tool("concepts_list"));
        assert!(tools.has_tool("concepts_get"));
        assert!(tools.has_tool("concepts_categories"));
        assert!(!tools.has_tool("concepts_delete"));
    }

    #[tokio::test]
    async fn test_content_tools_list() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools.call("concepts_list", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_content_tools_list_with_category() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_list", serde_json::json!({"category": "alpha"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_list_with_limit() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_list", serde_json::json!({"limit": 1}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_get() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_get", serde_json::json!({"id": "item-1"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_get_not_found() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_get", serde_json::json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_content_tools_categories() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_categories", serde_json::json!({}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_content_tools_unknown_tool() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        assert!(tools
            .call("concepts_delete", serde_json::json!({}))
            .is_none());
    }

    // -- SourceTools tests --------------------------------------------------

    #[test]
    fn test_source_tools_creation() {
        let tools = SourceTools::new(MockSourceProvider);
        assert_eq!(tools.tool_count(), 3);
    }

    #[test]
    fn test_source_tools_names() {
        let tools = SourceTools::new(MockSourceProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "sources_list");
        assert_eq!(tool_list[1].name, "sources_chapters");
        assert_eq!(tool_list[2].name, "sources_get_chapter");
    }

    #[test]
    fn test_source_tools_has_tool() {
        let tools = SourceTools::new(MockSourceProvider);
        assert!(tools.has_tool("sources_list"));
        assert!(tools.has_tool("sources_chapters"));
        assert!(tools.has_tool("sources_get_chapter"));
        assert!(!tools.has_tool("sources_delete"));
    }

    #[tokio::test]
    async fn test_source_tools_list() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools.call("sources_list", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_chapters() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_chapters",
                serde_json::json!({"source_id": "book-1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_get_chapter() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_get_chapter",
                serde_json::json!({"source_id": "book-1", "chapter": "ch1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_get_chapter_not_found() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_get_chapter",
                serde_json::json!({"source_id": "book-1", "chapter": "missing"}),
            )
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[test]
    fn test_source_tools_unknown_tool() {
        let tools = SourceTools::new(MockSourceProvider);
        assert!(tools
            .call("sources_delete", serde_json::json!({}))
            .is_none());
    }
}
