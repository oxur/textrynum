//! MCP tools for full-text search.
//!
//! Provides `FtsTools` that implements `ToolRegistry` by delegating
//! search queries to a `fabryk_fts::SearchBackend`.

use fabryk_mcp_core::error::McpErrorExt;
use fabryk_mcp_core::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp_core::registry::{ToolRegistry, ToolResult};

use fabryk_fts::{SearchBackend, SearchParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn json_schema(value: Value) -> Arc<serde_json::Map<String, Value>> {
    match value {
        Value::Object(map) => Arc::new(map),
        _ => Arc::new(serde_json::Map::new()),
    }
}

fn make_tool(name: &str, description: &str, schema: Value) -> Tool {
    Tool::new(
        name.to_string(),
        description.to_string(),
        json_schema(schema),
    )
}

fn serialize_response<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ---------------------------------------------------------------------------
// Argument types
// ---------------------------------------------------------------------------

/// Arguments for the search tool.
#[derive(Debug, Deserialize)]
pub struct SearchArgs {
    /// Search query string.
    pub query: String,
    /// Optional category filter.
    pub category: Option<String>,
    /// Optional source filter.
    pub source: Option<String>,
    /// Maximum results to return (default 10).
    pub limit: Option<usize>,
    /// Optional content type filter.
    pub content_type: Option<String>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A single search result for MCP responses.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResultResponse {
    /// Item ID.
    pub id: String,
    /// Item title.
    pub title: String,
    /// Item description.
    pub description: Option<String>,
    /// Category.
    pub category: String,
    /// Source reference.
    pub source: Option<String>,
    /// Content snippet.
    pub snippet: Option<String>,
    /// Relevance score.
    pub relevance: f32,
    /// Content type.
    pub content_type: Option<String>,
}

/// Response from search tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Search query that was executed.
    pub query: String,
    /// Number of results found.
    pub total: usize,
    /// Results (may be limited).
    pub results: Vec<SearchResultResponse>,
    /// Search duration in milliseconds.
    pub duration_ms: u64,
    /// Backend used.
    pub backend: String,
}

/// Response from search status tool.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchStatusResponse {
    /// Backend name.
    pub backend: String,
    /// Whether backend is ready.
    pub ready: bool,
}

// ---------------------------------------------------------------------------
// FtsTools
// ---------------------------------------------------------------------------

/// MCP tools for full-text search.
///
/// Generates two tools:
/// - `search` — full-text search with filtering
/// - `search_status` — search backend status
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_fts::create_search_backend;
/// use fabryk_mcp_fts::FtsTools;
///
/// let backend = create_search_backend(&config).await?;
/// let fts_tools = FtsTools::from_boxed(backend);
/// ```
pub struct FtsTools {
    backend: Arc<dyn SearchBackend>,
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
    extra_search_schema: Option<serde_json::Value>,
}

impl FtsTools {
    /// Slot key for the search tool.
    pub const SLOT_SEARCH: &str = "search";
    /// Slot key for the search status tool.
    pub const SLOT_STATUS: &str = "search_status";

    /// Create new FTS tools wrapping a search backend.
    pub fn new<B: SearchBackend + 'static>(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            extra_search_schema: None,
        }
    }

    /// Create FTS tools from a boxed backend.
    pub fn from_boxed(backend: Box<dyn SearchBackend>) -> Self {
        Self {
            backend: Arc::from(backend),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            extra_search_schema: None,
        }
    }

    /// Create FTS tools with a shared backend reference.
    pub fn with_shared(backend: Arc<dyn SearchBackend>) -> Self {
        Self {
            backend,
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            extra_search_schema: None,
        }
    }

    /// Override individual tool names.
    ///
    /// Keys are slot constants (`SLOT_SEARCH`, etc.), values are custom
    /// MCP-visible names. Unspecified slots keep their default names.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override individual tool descriptions.
    ///
    /// Keys are slot constants, values are custom descriptions.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    /// Provide extra JSON Schema properties to merge into the search tool schema.
    ///
    /// The value should be a JSON object whose keys are property names and
    /// values are JSON Schema definitions (e.g., `{"tier": {"type": "string"}}`).
    /// These are merged into the `properties` of the search tool's input schema
    /// and forwarded as `extra_filters` on `SearchParams` at call time.
    pub fn with_extra_search_schema(mut self, schema: serde_json::Value) -> Self {
        self.extra_search_schema = Some(schema);
        self
    }

    fn tool_name(&self, slot: &str) -> String {
        self.custom_names
            .get(slot)
            .cloned()
            .unwrap_or_else(|| slot.to_string())
    }

    fn tool_description(&self, slot: &str, default: &str) -> String {
        self.custom_descriptions
            .get(slot)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl ToolRegistry for FtsTools {
    fn tools(&self) -> Vec<Tool> {
        let mut search_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "category": {
                    "type": "string",
                    "description": "Filter by category"
                },
                "source": {
                    "type": "string",
                    "description": "Filter by source"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results (default 10)"
                },
                "content_type": {
                    "type": "string",
                    "description": "Filter by content type"
                }
            },
            "required": ["query"]
        });

        // Merge extra schema properties into the search tool's properties.
        if let Some(extra) = &self.extra_search_schema
            && let (Some(props), Some(extra_props)) =
                (search_schema.pointer_mut("/properties"), extra.as_object())
            && let Some(map) = props.as_object_mut()
        {
            for (key, value) in extra_props {
                map.insert(key.clone(), value.clone());
            }
        }

        vec![
            make_tool(
                &self.tool_name(Self::SLOT_SEARCH),
                &self.tool_description(Self::SLOT_SEARCH, "Full-text search across all content"),
                search_schema,
            ),
            make_tool(
                &self.tool_name(Self::SLOT_STATUS),
                &self.tool_description(Self::SLOT_STATUS, "Get search backend status"),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let backend = Arc::clone(&self.backend);

        if name == self.tool_name(Self::SLOT_SEARCH) {
            return Some(Box::pin(async move {
                let mut obj = match args {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                // Extract known fields.
                let query = obj
                    .remove("query")
                    .and_then(|v| v.as_str().map(String::from))
                    .ok_or_else(|| {
                        ErrorData::invalid_params(
                            "missing required field `query`".to_string(),
                            None,
                        )
                    })?;
                let category = obj
                    .remove("category")
                    .and_then(|v| v.as_str().map(String::from));
                let source = obj
                    .remove("source")
                    .and_then(|v| v.as_str().map(String::from));
                let limit = obj
                    .remove("limit")
                    .and_then(|v| v.as_u64().map(|n| n as usize));
                let content_type = obj
                    .remove("content_type")
                    .and_then(|v| v.as_str().map(String::from));

                let content_types = content_type.map(|ct| vec![ct]);

                // Remaining fields become extra filters.
                let extra_filters = if obj.is_empty() {
                    None
                } else {
                    Some(Value::Object(obj))
                };

                let start = Instant::now();

                let params = SearchParams {
                    query: query.clone(),
                    limit,
                    category,
                    source,
                    content_types,
                    query_mode: None,
                    snippet_length: None,
                    extra_filters,
                };

                let search_results = backend.search(params).await.map_err(|e| e.to_mcp_error())?;

                let results: Vec<SearchResultResponse> = search_results
                    .items
                    .into_iter()
                    .map(|hit| SearchResultResponse {
                        id: hit.id,
                        title: hit.title,
                        description: hit.description,
                        category: hit.category,
                        source: hit.source,
                        snippet: hit.snippet,
                        relevance: hit.relevance,
                        content_type: hit.content_type,
                    })
                    .collect();

                let response = SearchResponse {
                    query,
                    total: search_results.total,
                    results,
                    duration_ms: start.elapsed().as_millis() as u64,
                    backend: search_results.backend,
                };

                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_STATUS) {
            return Some(Box::pin(async move {
                let response = SearchStatusResponse {
                    backend: backend.name().to_string(),
                    ready: backend.is_ready(),
                };
                serialize_response(&response)
            }));
        }

        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use fabryk_fts::{SearchResult, SearchResults};

    // -- Mock backend -------------------------------------------------------

    struct MockSearchBackend {
        results: Vec<SearchResult>,
    }

    impl MockSearchBackend {
        fn new() -> Self {
            Self {
                results: vec![
                    SearchResult {
                        id: "result-1".to_string(),
                        title: "First Result".to_string(),
                        description: Some("A test result".to_string()),
                        category: "test".to_string(),
                        source: None,
                        snippet: Some("...matching text...".to_string()),
                        relevance: 0.95,
                        content_type: Some("concept".to_string()),
                        path: None,
                        chapter: None,
                        section: None,
                    },
                    SearchResult {
                        id: "result-2".to_string(),
                        title: "Second Result".to_string(),
                        description: None,
                        category: "test".to_string(),
                        source: Some("book-1".to_string()),
                        snippet: None,
                        relevance: 0.8,
                        content_type: None,
                        path: None,
                        chapter: None,
                        section: None,
                    },
                ],
            }
        }
    }

    #[async_trait]
    impl SearchBackend for MockSearchBackend {
        async fn search(&self, params: SearchParams) -> fabryk_core::Result<SearchResults> {
            let mut items = self.results.clone();

            if let Some(ref cat) = params.category {
                items.retain(|r| r.category == *cat);
            }
            if let Some(limit) = params.limit {
                items.truncate(limit);
            }

            let total = items.len();
            Ok(SearchResults {
                items,
                total,
                backend: "mock".to_string(),
            })
        }

        fn name(&self) -> &str {
            "mock"
        }

        fn is_ready(&self) -> bool {
            true
        }
    }

    // -- Tool tests ---------------------------------------------------------

    #[test]
    fn test_fts_tools_creation() {
        let tools = FtsTools::new(MockSearchBackend::new());
        assert_eq!(tools.tool_count(), 2);
    }

    #[test]
    fn test_fts_tools_names() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "search");
        assert_eq!(tool_list[1].name, "search_status");
    }

    #[test]
    fn test_fts_tools_has_tool() {
        let tools = FtsTools::new(MockSearchBackend::new());
        assert!(tools.has_tool("search"));
        assert!(tools.has_tool("search_status"));
        assert!(!tools.has_tool("search_suggest"));
    }

    #[tokio::test]
    async fn test_fts_search() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let future = tools
            .call("search", serde_json::json!({"query": "test"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_fts_search_with_category() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let future = tools
            .call(
                "search",
                serde_json::json!({"query": "test", "category": "test"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_fts_search_with_limit() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let future = tools
            .call("search", serde_json::json!({"query": "test", "limit": 1}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_fts_search_missing_query() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let future = tools.call("search", serde_json::json!({})).unwrap();
        let result = future.await;
        // Should fail because query is required
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fts_search_status() {
        let tools = FtsTools::new(MockSearchBackend::new());
        let future = tools.call("search_status", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_fts_tools_unknown_tool() {
        let tools = FtsTools::new(MockSearchBackend::new());
        assert!(
            tools
                .call("search_suggest", serde_json::json!({}))
                .is_none()
        );
    }

    // -- Response type tests ------------------------------------------------

    #[test]
    fn test_search_response_serialization() {
        let response = SearchResponse {
            query: "test".to_string(),
            total: 1,
            results: vec![SearchResultResponse {
                id: "r1".to_string(),
                title: "Result 1".to_string(),
                description: None,
                category: "test".to_string(),
                source: None,
                snippet: None,
                relevance: 0.9,
                content_type: None,
            }],
            duration_ms: 5,
            backend: "mock".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("0.9"));

        let deserialized: SearchResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total, 1);
    }

    #[test]
    fn test_search_status_response_serialization() {
        let response = SearchStatusResponse {
            backend: "tantivy".to_string(),
            ready: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: SearchStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.backend, "tantivy");
        assert!(deserialized.ready);
    }

    #[test]
    fn test_from_boxed() {
        let backend: Box<dyn SearchBackend> = Box::new(MockSearchBackend::new());
        let tools = FtsTools::from_boxed(backend);
        assert_eq!(tools.tool_count(), 2);
    }

    #[test]
    fn test_fts_tools_with_custom_names() {
        let tools = FtsTools::new(MockSearchBackend::new()).with_names(HashMap::from([
            ("search".to_string(), "search_concepts".to_string()),
            ("search_status".to_string(), "fts_status".to_string()),
        ]));
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "search_concepts");
        assert_eq!(tool_list[1].name, "fts_status");
    }

    #[tokio::test]
    async fn test_fts_tools_custom_names_dispatch() {
        let tools = FtsTools::new(MockSearchBackend::new()).with_names(HashMap::from([(
            "search".to_string(),
            "search_concepts".to_string(),
        )]));
        // Old name should NOT work
        assert!(
            tools
                .call("search", serde_json::json!({"query": "test"}))
                .is_none()
        );
        // Custom name should work
        let future = tools
            .call("search_concepts", serde_json::json!({"query": "test"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_fts_tools_with_custom_descriptions() {
        let tools = FtsTools::new(MockSearchBackend::new()).with_descriptions(HashMap::from([(
            "search".to_string(),
            "Custom search desc".to_string(),
        )]));
        let tool_list = tools.tools();
        assert_eq!(
            tool_list[0].description.as_ref().unwrap(),
            "Custom search desc"
        );
        // Other tool keeps default
        assert_eq!(
            tool_list[1].description.as_ref().unwrap(),
            "Get search backend status"
        );
    }

    // -- Extra search schema / filter tests -----------------------------------

    #[test]
    fn test_fts_tools_with_extra_schema() {
        let tools = FtsTools::new(MockSearchBackend::new())
            .with_extra_search_schema(serde_json::json!({
                "tier": {"type": "string"}
            }));
        let tool_list = tools.tools();
        let search_tool = &tool_list[0];
        assert_eq!(search_tool.name, "search");

        // The search tool schema should contain the extra "tier" property.
        let schema_value = serde_json::to_value(&search_tool.input_schema).unwrap();
        let properties = schema_value
            .get("properties")
            .expect("schema should have properties");
        assert!(
            properties.get("tier").is_some(),
            "extra 'tier' property should be merged into schema"
        );
        // Original properties should still be present.
        assert!(properties.get("query").is_some());
        assert!(properties.get("category").is_some());
    }

    #[test]
    fn test_search_params_has_extra_filters() {
        let params = SearchParams {
            query: "test".to_string(),
            extra_filters: Some(serde_json::json!({"tier": "advanced"})),
            ..Default::default()
        };
        assert!(params.extra_filters.is_some());
        let filters = params.extra_filters.unwrap();
        assert_eq!(filters["tier"], "advanced");
    }

    #[tokio::test]
    async fn test_fts_search_passes_extra_filters() {
        let tools = FtsTools::new(MockSearchBackend::new())
            .with_extra_search_schema(serde_json::json!({
                "tier": {"type": "string"}
            }));

        // Call with an extra field that is not a known search arg.
        let future = tools
            .call(
                "search",
                serde_json::json!({"query": "test", "tier": "advanced"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_fts_search_no_extra_filters_when_only_known_fields() {
        let tools = FtsTools::new(MockSearchBackend::new());

        let future = tools
            .call(
                "search",
                serde_json::json!({"query": "test", "category": "test"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }
}
