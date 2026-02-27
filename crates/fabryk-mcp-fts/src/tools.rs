//! MCP tools for full-text search.
//!
//! Provides `FtsTools` that implements `ToolRegistry` by delegating
//! search queries to a `fabryk_fts::SearchBackend`.

use fabryk_mcp::error::McpErrorExt;
use fabryk_mcp::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp::registry::{ToolRegistry, ToolResult};

use fabryk_fts::{SearchBackend, SearchParams};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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
}

impl FtsTools {
    /// Create new FTS tools wrapping a search backend.
    pub fn new<B: SearchBackend + 'static>(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    /// Create FTS tools from a boxed backend.
    pub fn from_boxed(backend: Box<dyn SearchBackend>) -> Self {
        Self {
            backend: Arc::from(backend),
        }
    }

    /// Create FTS tools with a shared backend reference.
    pub fn with_shared(backend: Arc<dyn SearchBackend>) -> Self {
        Self { backend }
    }
}

impl ToolRegistry for FtsTools {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "search",
                "Full-text search across all content",
                serde_json::json!({
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
                }),
            ),
            make_tool(
                "search_status",
                "Get search backend status",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let backend = Arc::clone(&self.backend);

        match name {
            "search" => Some(Box::pin(async move {
                let args: SearchArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;

                let start = Instant::now();

                let content_types = args.content_type.map(|ct| vec![ct]);

                let params = SearchParams {
                    query: args.query.clone(),
                    limit: args.limit,
                    category: args.category,
                    source: args.source,
                    content_types,
                    query_mode: None,
                    snippet_length: None,
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
                    query: args.query,
                    total: search_results.total,
                    results,
                    duration_ms: start.elapsed().as_millis() as u64,
                    backend: search_results.backend,
                };

                serialize_response(&response)
            })),

            "search_status" => Some(Box::pin(async move {
                let response = SearchStatusResponse {
                    backend: backend.name().to_string(),
                    ready: backend.is_ready(),
                };
                serialize_response(&response)
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
        assert!(tools
            .call("search_suggest", serde_json::json!({}))
            .is_none());
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
}
