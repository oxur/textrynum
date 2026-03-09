//! MCP tools for semantic (hybrid) search.
//!
//! Provides `SemanticSearchTools` that implements `ToolRegistry` by combining
//! full-text search and vector similarity backends.

use std::collections::HashMap;
use std::sync::Arc;

use fabryk_fts::{SearchBackend, SearchParams};
use fabryk_mcp_core::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp_core::registry::{ToolRegistry, ToolResult};
use fabryk_vector::{
    FtsResult, HybridSearchResult, VectorBackend, VectorSearchParams, reciprocal_rank_fusion,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    Tool::new(name.to_string(), description.to_string(), json_schema(schema))
}

fn serialize_response<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ---------------------------------------------------------------------------
// Argument types
// ---------------------------------------------------------------------------

/// Arguments for the semantic_search tool.
#[derive(Debug, Deserialize)]
pub struct SemanticSearchArgs {
    /// Search query string.
    pub query: String,
    /// Search mode: "vector", "keyword", or "hybrid" (default).
    pub mode: Option<String>,
    /// Optional category filter.
    pub category: Option<String>,
    /// Optional source filter.
    pub source: Option<String>,
    /// Maximum results to return (default 10, max 50).
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// Response types
// ---------------------------------------------------------------------------

/// A merged search result from hybrid search.
///
/// Wraps `HybridSearchResult` for MCP serialization compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    /// Document identifier.
    pub id: String,
    /// Combined RRF score (higher is better).
    pub rrf_score: f32,
    /// Source of the result: "vector", "keyword", or "hybrid".
    pub source: String,
    /// Metadata snapshot.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl From<HybridSearchResult> for HybridResult {
    fn from(r: HybridSearchResult) -> Self {
        Self {
            id: r.id,
            rrf_score: r.score,
            source: r.source,
            metadata: r.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// FTS → FtsResult adapter
// ---------------------------------------------------------------------------

/// Convert `fabryk_fts::SearchResults` items into the `FtsResult` type
/// expected by `fabryk_vector::reciprocal_rank_fusion`.
fn to_fts_results(results: &fabryk_fts::SearchResults) -> Vec<FtsResult> {
    results
        .items
        .iter()
        .map(|item| FtsResult {
            id: item.id.clone(),
            score: item.relevance,
            metadata: HashMap::new(),
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

/// Shared slot for a vector backend that may be populated asynchronously.
pub type VectorSlot = Arc<tokio::sync::RwLock<Option<Arc<dyn VectorBackend>>>>;

// ---------------------------------------------------------------------------
// SemanticSearchTools
// ---------------------------------------------------------------------------

/// MCP tools for semantic (hybrid) search.
///
/// Provides a single `semantic_search` tool that supports three modes:
///
/// - **keyword** — delegates to the FTS backend
/// - **vector** — delegates to the vector backend
/// - **hybrid** (default) — runs both and merges via Reciprocal Rank Fusion
///
/// When no vector backend is available, hybrid mode falls back to keyword-only.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_mcp_semantic::SemanticSearchTools;
///
/// let tools = SemanticSearchTools::new(fts_arc.clone(), Some(vector_arc.clone()));
/// ```
pub struct SemanticSearchTools {
    fts: Arc<dyn SearchBackend>,
    vector: Option<Arc<dyn VectorBackend>>,
    vector_slot: Option<VectorSlot>,
}

impl SemanticSearchTools {
    /// Create semantic search tools with FTS and optional vector backends.
    pub fn new(fts: Arc<dyn SearchBackend>, vector: Option<Arc<dyn VectorBackend>>) -> Self {
        Self {
            fts,
            vector,
            vector_slot: None,
        }
    }

    /// Create semantic search tools from boxed backends.
    pub fn from_boxed(fts: Box<dyn SearchBackend>, vector: Option<Box<dyn VectorBackend>>) -> Self {
        Self {
            fts: Arc::from(fts),
            vector: vector.map(Arc::from),
            vector_slot: None,
        }
    }

    /// Create semantic search tools with shared backend references.
    pub fn with_shared(
        fts: Arc<dyn SearchBackend>,
        vector: Option<Arc<dyn VectorBackend>>,
    ) -> Self {
        Self::new(fts, vector)
    }

    /// Create with a shared slot that will be populated by a background build.
    ///
    /// The slot is checked at call time, so vector search becomes available
    /// as soon as the background builder populates it.
    pub fn with_vector_slot(fts: Arc<dyn SearchBackend>, vector_slot: VectorSlot) -> Self {
        Self {
            fts,
            vector: None,
            vector_slot: Some(vector_slot),
        }
    }

    /// Resolve the vector backend: prefer the direct field, then try the shared slot.
    fn resolve_vector(&self) -> Option<Arc<dyn VectorBackend>> {
        if let Some(ref v) = self.vector {
            return Some(v.clone());
        }
        if let Some(ref slot) = self.vector_slot {
            // try_read avoids blocking the async runtime; returns None if locked
            slot.try_read().ok().and_then(|guard| guard.clone())
        } else {
            None
        }
    }
}

impl ToolRegistry for SemanticSearchTools {
    fn tools(&self) -> Vec<Tool> {
        vec![make_tool(
            "semantic_search",
            "Search concepts using semantic similarity. Supports 'vector' (embedding-based), \
             'keyword' (FTS), or 'hybrid' (both via RRF, default).",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Natural language search query"
                    },
                    "mode": {
                        "type": "string",
                        "description": "Search mode: 'vector', 'keyword', or 'hybrid' (default)",
                        "enum": ["vector", "keyword", "hybrid"]
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
                        "description": "Maximum results (default: 10)"
                    }
                },
                "required": ["query"]
            }),
        )]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        if name != "semantic_search" {
            return None;
        }

        let fts = self.fts.clone();
        let vector = self.resolve_vector();

        Some(Box::pin(async move {
            let args: SemanticSearchArgs = serde_json::from_value(args)
                .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;

            let mode = args.mode.as_deref().unwrap_or("hybrid");
            let limit = args.limit.unwrap_or(10).min(50);

            match mode {
                "vector" => {
                    let backend = vector.as_ref().ok_or_else(|| {
                        ErrorData::internal_error(
                            "Vector search is not available".to_string(),
                            None,
                        )
                    })?;
                    let params = VectorSearchParams::new(&args.query).with_limit(limit);
                    let results = backend
                        .search(params)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    serialize_response(&results)
                }
                "keyword" => {
                    let params = SearchParams {
                        query: args.query,
                        limit: Some(limit),
                        category: args.category,
                        source: args.source,
                        ..Default::default()
                    };
                    let results = fts
                        .search(params)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                    serialize_response(&results)
                }
                _ => {
                    // Hybrid: run both and merge via reciprocal rank fusion
                    let fts_params = SearchParams {
                        query: args.query.clone(),
                        limit: Some(limit * 2),
                        category: args.category.clone(),
                        source: args.source.clone(),
                        ..Default::default()
                    };

                    let fts_results = fts
                        .search(fts_params)
                        .await
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

                    // If vector is available, do hybrid; otherwise fall back to FTS only
                    if let Some(ref backend) = vector {
                        let vector_params =
                            VectorSearchParams::new(&args.query).with_limit(limit * 2);
                        let vector_results = backend
                            .search(vector_params)
                            .await
                            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

                        // Convert FTS results to the adapter type and run RRF
                        let fts_adapted = to_fts_results(&fts_results);
                        let merged =
                            reciprocal_rank_fusion(&vector_results.items, &fts_adapted, limit, 60);
                        let results: Vec<HybridResult> =
                            merged.into_iter().map(HybridResult::from).collect();
                        serialize_response(&results)
                    } else {
                        // No vector backend — return FTS results only
                        serialize_response(&fts_results)
                    }
                }
            }
        }))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fabryk_fts::{SearchResult, SearchResults};

    // -- Test helpers -------------------------------------------------------

    fn make_fts_result(id: &str, relevance: f32) -> SearchResult {
        SearchResult {
            id: id.to_string(),
            title: id.to_string(),
            description: None,
            category: String::new(),
            source: None,
            snippet: None,
            relevance,
            content_type: None,
            path: None,
            chapter: None,
            section: None,
        }
    }

    fn make_fts_results(items: Vec<SearchResult>) -> SearchResults {
        let total = items.len();
        SearchResults {
            items,
            total,
            backend: "test".to_string(),
        }
    }

    // -- Mock backends ------------------------------------------------------

    struct MockFts {
        results: SearchResults,
    }

    impl MockFts {
        fn new(items: Vec<SearchResult>) -> Self {
            Self {
                results: make_fts_results(items),
            }
        }

        fn empty() -> Self {
            Self::new(vec![])
        }
    }

    #[async_trait::async_trait]
    impl SearchBackend for MockFts {
        async fn search(&self, _params: SearchParams) -> fabryk_core::Result<SearchResults> {
            Ok(self.results.clone())
        }

        fn name(&self) -> &str {
            "mock"
        }
    }

    struct MockVector {
        results: fabryk_vector::VectorSearchResults,
    }

    impl MockVector {
        fn new(items: Vec<fabryk_vector::VectorSearchResult>) -> Self {
            Self {
                results: fabryk_vector::VectorSearchResults {
                    items: items.clone(),
                    total: items.len(),
                    backend: "mock-vector".to_string(),
                },
            }
        }

        fn with_ids(ids: &[&str]) -> Self {
            let items = ids
                .iter()
                .enumerate()
                .map(|(i, id)| fabryk_vector::VectorSearchResult {
                    id: id.to_string(),
                    score: 1.0 - i as f32 * 0.1,
                    distance: i as f32 * 0.1,
                    metadata: HashMap::new(),
                })
                .collect();
            Self::new(items)
        }
    }

    #[async_trait::async_trait]
    impl VectorBackend for MockVector {
        async fn search(
            &self,
            _params: VectorSearchParams,
        ) -> fabryk_core::Result<fabryk_vector::VectorSearchResults> {
            Ok(self.results.clone())
        }

        fn name(&self) -> &str {
            "mock-vector"
        }

        fn document_count(&self) -> fabryk_core::Result<usize> {
            Ok(self.results.total)
        }
    }

    fn make_tools_fts_only() -> SemanticSearchTools {
        SemanticSearchTools::new(Arc::new(MockFts::empty()), None)
    }

    fn make_tools_with_vector(
        fts_items: Vec<SearchResult>,
        vector_ids: &[&str],
    ) -> SemanticSearchTools {
        SemanticSearchTools::new(
            Arc::new(MockFts::new(fts_items)),
            Some(Arc::new(MockVector::with_ids(vector_ids))),
        )
    }

    // -- Tool definition tests ----------------------------------------------

    #[test]
    fn test_tools_returns_semantic_search() {
        let tools = make_tools_fts_only();
        let tool_list = tools.tools();
        assert_eq!(tool_list.len(), 1);
        assert_eq!(tool_list[0].name.as_ref(), "semantic_search");
        assert!(tool_list[0].description.is_some());
    }

    #[test]
    fn test_has_tool() {
        let tools = make_tools_fts_only();
        assert!(tools.has_tool("semantic_search"));
        assert!(!tools.has_tool("search"));
    }

    #[test]
    fn test_unknown_tool_returns_none() {
        let tools = make_tools_fts_only();
        assert!(tools.call("nonexistent", Value::Null).is_none());
    }

    // -- Keyword mode tests ------------------------------------------------

    #[tokio::test]
    async fn test_keyword_mode() {
        let fts_items = vec![
            make_fts_result("card-a", 0.9),
            make_fts_result("card-b", 0.7),
        ];
        let tools = SemanticSearchTools::new(Arc::new(MockFts::new(fts_items)), None);

        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test query", "mode": "keyword"}),
            )
            .unwrap()
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_keyword_mode_with_category() {
        let tools = make_tools_fts_only();
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "keyword", "category": "di"}),
            )
            .unwrap()
            .await;
        assert!(result.is_ok());
    }

    // -- Vector mode tests ------------------------------------------------

    #[tokio::test]
    async fn test_vector_mode_no_backend() {
        let tools = make_tools_fts_only();
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "vector"}),
            )
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_vector_mode_with_backend() {
        let tools = make_tools_with_vector(vec![], &["vec-a", "vec-b"]);

        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "vector"}),
            )
            .unwrap()
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
    }

    // -- Hybrid mode tests ------------------------------------------------

    #[tokio::test]
    async fn test_hybrid_mode_no_vector_fallback() {
        let fts_items = vec![make_fts_result("fts-only", 0.9)];
        let tools = SemanticSearchTools::new(Arc::new(MockFts::new(fts_items)), None);

        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "hybrid"}),
            )
            .unwrap()
            .await
            .unwrap();

        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_hybrid_mode_with_vector() {
        let fts_items = vec![
            make_fts_result("both", 0.9),
            make_fts_result("fts-only", 0.5),
        ];
        let tools = make_tools_with_vector(fts_items, &["both", "vec-only"]);

        let result = tools
            .call("semantic_search", serde_json::json!({"query": "test"}))
            .unwrap()
            .await
            .unwrap();

        // Default mode is hybrid
        assert!(!result.is_error.unwrap_or(false));
    }

    // -- Argument validation -----------------------------------------------

    #[tokio::test]
    async fn test_invalid_args() {
        let tools = make_tools_fts_only();
        let result = tools
            .call("semantic_search", serde_json::json!({"bad": "args"}))
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_limit_capped_at_50() {
        let tools = make_tools_fts_only();
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "keyword", "limit": 100}),
            )
            .unwrap()
            .await;
        assert!(result.is_ok());
    }

    // -- Constructor tests -------------------------------------------------

    #[test]
    fn test_from_boxed() {
        let fts: Box<dyn SearchBackend> = Box::new(MockFts::empty());
        let tools = SemanticSearchTools::from_boxed(fts, None);
        assert_eq!(tools.tool_count(), 1);
    }

    #[test]
    fn test_with_shared() {
        let fts: Arc<dyn SearchBackend> = Arc::new(MockFts::empty());
        let tools = SemanticSearchTools::with_shared(fts, None);
        assert_eq!(tools.tool_count(), 1);
    }

    // -- vector_slot tests -------------------------------------------------

    #[tokio::test]
    async fn test_with_vector_slot_empty() {
        let slot: VectorSlot = Arc::new(tokio::sync::RwLock::new(None));
        let tools = SemanticSearchTools::with_vector_slot(Arc::new(MockFts::empty()), slot);

        // Hybrid mode should fall back to FTS-only when slot is empty
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "hybrid"}),
            )
            .unwrap()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_vector_slot_populated() {
        let slot: VectorSlot = Arc::new(tokio::sync::RwLock::new(Some(Arc::new(
            MockVector::with_ids(&["vec-a", "vec-b"]),
        ))));
        let fts_items = vec![make_fts_result("fts-a", 0.9)];
        let tools = SemanticSearchTools::with_vector_slot(Arc::new(MockFts::new(fts_items)), slot);

        // Hybrid mode should use vector backend from the slot
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "hybrid"}),
            )
            .unwrap()
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
        // Should produce hybrid results (HybridResult format with rrf_score) rather than plain FTS
        let json = serde_json::to_string(&result.content).unwrap();
        assert!(
            json.contains("rrf_score"),
            "Expected hybrid result format with rrf_score, got: {json}"
        );
    }

    #[tokio::test]
    async fn test_with_vector_slot_vector_mode() {
        let slot: VectorSlot = Arc::new(tokio::sync::RwLock::new(Some(Arc::new(
            MockVector::with_ids(&["vec-a"]),
        ))));
        let tools = SemanticSearchTools::with_vector_slot(Arc::new(MockFts::empty()), slot);

        // Explicit vector mode should work via slot
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "vector"}),
            )
            .unwrap()
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_vector_slot_vector_mode_empty() {
        let slot: VectorSlot = Arc::new(tokio::sync::RwLock::new(None));
        let tools = SemanticSearchTools::with_vector_slot(Arc::new(MockFts::empty()), slot);

        // Explicit vector mode should fail when slot is empty
        let result = tools
            .call(
                "semantic_search",
                serde_json::json!({"query": "test", "mode": "vector"}),
            )
            .unwrap()
            .await;
        assert!(result.is_err());
    }

    // -- FTS adapter tests -------------------------------------------------

    #[test]
    fn test_to_fts_results() {
        let items = vec![make_fts_result("a", 0.9), make_fts_result("b", 0.7)];
        let results = make_fts_results(items);
        let adapted = to_fts_results(&results);

        assert_eq!(adapted.len(), 2);
        assert_eq!(adapted[0].id, "a");
        assert!((adapted[0].score - 0.9).abs() < f32::EPSILON);
        assert_eq!(adapted[1].id, "b");
    }

    // -- HybridResult tests ------------------------------------------------

    #[test]
    fn test_hybrid_result_from_search_result() {
        let search_result = HybridSearchResult {
            id: "doc-1".to_string(),
            score: 0.5,
            source: "hybrid".to_string(),
            metadata: HashMap::new(),
        };
        let result = HybridResult::from(search_result);
        assert_eq!(result.id, "doc-1");
        assert!((result.rrf_score - 0.5).abs() < f32::EPSILON);
        assert_eq!(result.source, "hybrid");
    }

    #[test]
    fn test_hybrid_result_serialize() {
        let result = HybridResult {
            id: "test-id".to_string(),
            rrf_score: 0.5,
            source: "keyword".to_string(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("rrf_score"));
        // Empty metadata should be omitted
        assert!(!json.contains("metadata"));
    }

    #[test]
    fn test_hybrid_result_clone() {
        let result = HybridResult {
            id: "x".to_string(),
            rrf_score: 1.0,
            source: "vector".to_string(),
            metadata: HashMap::new(),
        };
        let cloned = result.clone();
        assert_eq!(cloned.id, "x");
        assert_eq!(cloned.rrf_score, 1.0);
    }
}
