//! MCP tools for graph queries.
//!
//! Provides `GraphTools` that implements `ToolRegistry` by delegating
//! queries to `fabryk_graph` algorithms.

use fabryk_mcp::error::McpErrorExt;
use fabryk_mcp::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp::registry::{ToolRegistry, ToolResult};

use fabryk_graph::{
    calculate_centrality, compute_stats, find_bridges, neighborhood, prerequisites_sorted,
    shortest_path, validate_graph, EdgeInfo, GraphData, NeighborInfo, NodeSummary, PathStep,
    Relationship,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

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

fn parse_relationship(s: &str) -> Relationship {
    match s.to_lowercase().as_str() {
        "prerequisite" => Relationship::Prerequisite,
        "leads_to" | "leadsto" => Relationship::LeadsTo,
        "relates_to" | "relatesto" => Relationship::RelatesTo,
        "extends" => Relationship::Extends,
        "introduces" => Relationship::Introduces,
        "covers" => Relationship::Covers,
        "variant_of" | "variantof" => Relationship::VariantOf,
        other => Relationship::Custom(other.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Argument types
// ---------------------------------------------------------------------------

/// Arguments for graph_related tool.
#[derive(Debug, Deserialize)]
pub struct RelatedArgs {
    /// Node ID to find relations for.
    pub id: String,
    /// Optional relationship type filter.
    pub relationship: Option<String>,
    /// Maximum results.
    pub limit: Option<usize>,
}

/// Arguments for graph_path tool.
#[derive(Debug, Deserialize)]
pub struct PathArgs {
    /// Starting node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
}

/// Arguments for graph_prerequisites tool.
#[derive(Debug, Deserialize)]
pub struct PrerequisitesArgs {
    /// Target node ID.
    pub id: String,
}

/// Arguments for graph_neighborhood tool.
#[derive(Debug, Deserialize)]
pub struct NeighborhoodArgs {
    /// Center node ID.
    pub id: String,
    /// Hops from center (default 1).
    pub radius: Option<usize>,
    /// Optional relationship type filter.
    pub relationship: Option<String>,
}

// ---------------------------------------------------------------------------
// GraphTools
// ---------------------------------------------------------------------------

/// MCP tools for graph queries.
///
/// Generates eight tools:
/// - `graph_related` — find related nodes
/// - `graph_path` — shortest path between nodes
/// - `graph_prerequisites` — learning order prerequisites
/// - `graph_neighborhood` — N-hop neighborhood exploration
/// - `graph_info` — graph statistics
/// - `graph_validate` — structure validation
/// - `graph_centrality` — most central/important nodes
/// - `graph_bridges` — bridge nodes connecting different areas
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_graph::GraphData;
/// use fabryk_mcp_graph::GraphTools;
///
/// let graph = fabryk_graph::load_graph("graph.json")?;
/// let graph_tools = GraphTools::new(graph);
/// ```
pub struct GraphTools {
    graph: Arc<RwLock<GraphData>>,
}

impl GraphTools {
    /// Create new graph tools with owned graph data.
    pub fn new(graph: GraphData) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
        }
    }

    /// Create graph tools with a shared graph reference.
    pub fn with_shared(graph: Arc<RwLock<GraphData>>) -> Self {
        Self { graph }
    }

    /// Update the graph data (e.g., after rebuild).
    pub async fn update_graph(&self, graph: GraphData) {
        let mut lock = self.graph.write().await;
        *lock = graph;
    }
}

impl ToolRegistry for GraphTools {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "graph_related",
                "Find nodes related to a given node",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        },
                        "relationship": {
                            "type": "string",
                            "description": "Filter by relationship type (e.g., prerequisite, relates_to)"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results"
                        }
                    },
                    "required": ["id"]
                }),
            ),
            make_tool(
                "graph_path",
                "Find the shortest path between two nodes",
                json!({
                    "type": "object",
                    "properties": {
                        "from": {
                            "type": "string",
                            "description": "Starting node ID"
                        },
                        "to": {
                            "type": "string",
                            "description": "Target node ID"
                        }
                    },
                    "required": ["from", "to"]
                }),
            ),
            make_tool(
                "graph_prerequisites",
                "Get prerequisites for a node in learning order",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        }
                    },
                    "required": ["id"]
                }),
            ),
            make_tool(
                "graph_neighborhood",
                "Explore the neighborhood around a node",
                json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Center node ID"
                        },
                        "radius": {
                            "type": "integer",
                            "description": "Hops from center (default 1)"
                        },
                        "relationship": {
                            "type": "string",
                            "description": "Filter by relationship type"
                        }
                    },
                    "required": ["id"]
                }),
            ),
            make_tool(
                "graph_info",
                "Get graph statistics and overview",
                json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            make_tool(
                "graph_validate",
                "Validate graph structure and report issues",
                json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            make_tool(
                "graph_centrality",
                "Get most central/important nodes",
                json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                }),
            ),
            make_tool(
                "graph_bridges",
                "Find bridge nodes that connect different areas",
                json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let graph = Arc::clone(&self.graph);

        match name {
            "graph_related" => Some(Box::pin(async move {
                let args: RelatedArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let rel_filter = args
                    .relationship
                    .as_deref()
                    .map(|r| vec![parse_relationship(r)]);

                let result = neighborhood(&graph, &args.id, 1, rel_filter.as_deref())
                    .map_err(|e| e.to_mcp_error())?;

                let mut nodes: Vec<NodeSummary> =
                    result.nodes.iter().map(NodeSummary::from).collect();

                if let Some(limit) = args.limit {
                    nodes.truncate(limit);
                }

                let count = nodes.len();
                let response = json!({
                    "source": NodeSummary::from(&result.center),
                    "related": nodes,
                    "count": count
                });
                serialize_response(&response)
            })),

            "graph_path" => Some(Box::pin(async move {
                let args: PathArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let result =
                    shortest_path(&graph, &args.from, &args.to).map_err(|e| e.to_mcp_error())?;

                if result.found {
                    let path: Vec<PathStep> = result
                        .path
                        .iter()
                        .enumerate()
                        .map(|(i, node)| {
                            let rel = result
                                .edges
                                .get(i)
                                .map(|e| e.relationship.name().to_string());
                            PathStep {
                                node: NodeSummary::from(node),
                                relationship_to_next: rel,
                            }
                        })
                        .collect();

                    let response = json!({
                        "found": true,
                        "path": path,
                        "length": path.len(),
                        "total_weight": result.total_weight
                    });
                    serialize_response(&response)
                } else {
                    let response = json!({
                        "found": false,
                        "message": format!("No path found from {} to {}", args.from, args.to)
                    });
                    serialize_response(&response)
                }
            })),

            "graph_prerequisites" => Some(Box::pin(async move {
                let args: PrerequisitesArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let result =
                    prerequisites_sorted(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                let prereqs: Vec<NodeSummary> =
                    result.ordered.iter().map(NodeSummary::from).collect();

                let count = prereqs.len();
                let response = json!({
                    "target": NodeSummary::from(&result.target),
                    "prerequisites": prereqs,
                    "count": count,
                    "has_cycles": result.has_cycles
                });
                serialize_response(&response)
            })),

            "graph_neighborhood" => Some(Box::pin(async move {
                let args: NeighborhoodArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let radius = args.radius.unwrap_or(1);
                let rel_filter = args
                    .relationship
                    .as_deref()
                    .map(|r| vec![parse_relationship(r)]);

                let result = neighborhood(&graph, &args.id, radius, rel_filter.as_deref())
                    .map_err(|e| e.to_mcp_error())?;

                let nodes: Vec<NeighborInfo> = result
                    .nodes
                    .iter()
                    .map(|n| {
                        let distance = result.distances.get(&n.id).copied().unwrap_or(0);
                        NeighborInfo {
                            node: NodeSummary::from(n),
                            distance,
                        }
                    })
                    .collect();

                let edges: Vec<EdgeInfo> = result.edges.iter().map(EdgeInfo::from).collect();

                let response = json!({
                    "center": NodeSummary::from(&result.center),
                    "radius": radius,
                    "nodes": nodes,
                    "edges": edges,
                    "edge_count": edges.len()
                });
                serialize_response(&response)
            })),

            "graph_info" => Some(Box::pin(async move {
                let graph = graph.read().await;
                let stats = compute_stats(&graph);
                serialize_response(&stats)
            })),

            "graph_validate" => Some(Box::pin(async move {
                let graph = graph.read().await;
                let result = validate_graph(&graph);
                serialize_response(&result)
            })),

            "graph_centrality" => Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let scores = calculate_centrality(&graph);

                let top: Vec<_> = scores.into_iter().take(limit).collect();
                serialize_response(&top)
            })),

            "graph_bridges" => Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let bridges = find_bridges(&graph, limit);

                let summaries: Vec<NodeSummary> = bridges.iter().map(NodeSummary::from).collect();
                serialize_response(&summaries)
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
    use fabryk_graph::{Edge, Node};

    fn make_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        // Add nodes
        graph.add_node(Node::new("node-a", "Node A").with_category("alpha"));
        graph.add_node(Node::new("node-b", "Node B").with_category("beta"));
        graph.add_node(Node::new("node-c", "Node C").with_category("alpha"));

        // Add edges: A -> B (prerequisite), B -> C (relates_to)
        let _ = graph.add_edge(Edge::new("node-a", "node-b", Relationship::Prerequisite));
        let _ = graph.add_edge(Edge::new("node-b", "node-c", Relationship::RelatesTo));

        graph
    }

    // -- Tool creation tests ------------------------------------------------

    #[test]
    fn test_graph_tools_creation() {
        let tools = GraphTools::new(GraphData::new());
        assert_eq!(tools.tool_count(), 8);
    }

    #[test]
    fn test_graph_tools_names() {
        let tools = GraphTools::new(GraphData::new());
        let tool_list = tools.tools();
        let names: Vec<&str> = tool_list.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"graph_related"));
        assert!(names.contains(&"graph_path"));
        assert!(names.contains(&"graph_prerequisites"));
        assert!(names.contains(&"graph_neighborhood"));
        assert!(names.contains(&"graph_info"));
        assert!(names.contains(&"graph_validate"));
        assert!(names.contains(&"graph_centrality"));
        assert!(names.contains(&"graph_bridges"));
    }

    #[test]
    fn test_graph_tools_has_tool() {
        let tools = GraphTools::new(GraphData::new());
        assert!(tools.has_tool("graph_related"));
        assert!(tools.has_tool("graph_info"));
        assert!(!tools.has_tool("graph_delete"));
    }

    // -- graph_info tests ---------------------------------------------------

    #[tokio::test]
    async fn test_graph_info_empty() {
        let tools = GraphTools::new(GraphData::new());
        let future = tools.call("graph_info", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_graph_info_with_data() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools.call("graph_info", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_validate tests -----------------------------------------------

    #[tokio::test]
    async fn test_graph_validate_empty() {
        let tools = GraphTools::new(GraphData::new());
        let future = tools.call("graph_validate", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_graph_validate_with_data() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools.call("graph_validate", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_related tests ------------------------------------------------

    #[tokio::test]
    async fn test_graph_related() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_related", json!({"id": "node-a"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_graph_related_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_related", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_related_with_limit() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_related", json!({"id": "node-b", "limit": 1}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_path tests ---------------------------------------------------

    #[tokio::test]
    async fn test_graph_path() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_path", json!({"from": "node-a", "to": "node-c"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_graph_path_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_path", json!({"from": "node-c", "to": "node-a"}))
            .unwrap();
        let result = future.await.unwrap();
        // Should return found: false, not an error
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_prerequisites tests ------------------------------------------

    #[tokio::test]
    async fn test_graph_prerequisites() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_prerequisites", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_neighborhood tests -------------------------------------------

    #[tokio::test]
    async fn test_graph_neighborhood() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_neighborhood", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_graph_neighborhood_with_radius() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_neighborhood", json!({"id": "node-a", "radius": 2}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_centrality tests ---------------------------------------------

    #[tokio::test]
    async fn test_graph_centrality() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools.call("graph_centrality", json!({"limit": 5})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- graph_bridges tests ------------------------------------------------

    #[tokio::test]
    async fn test_graph_bridges() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools.call("graph_bridges", json!({"limit": 5})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- Shared state tests -------------------------------------------------

    #[tokio::test]
    async fn test_graph_update() {
        let tools = GraphTools::new(GraphData::new());

        // Initial: empty graph
        let future = tools.call("graph_info", json!({})).unwrap();
        let _result = future.await.unwrap();

        // Update with populated graph
        tools.update_graph(make_test_graph()).await;

        let future = tools.call("graph_info", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- Unknown tool test --------------------------------------------------

    #[test]
    fn test_graph_tools_unknown_tool() {
        let tools = GraphTools::new(GraphData::new());
        assert!(tools.call("graph_delete", json!({})).is_none());
    }

    // -- parse_relationship tests -------------------------------------------

    #[test]
    fn test_parse_relationship_known() {
        assert!(matches!(
            parse_relationship("prerequisite"),
            Relationship::Prerequisite
        ));
        assert!(matches!(
            parse_relationship("leads_to"),
            Relationship::LeadsTo
        ));
        assert!(matches!(
            parse_relationship("relates_to"),
            Relationship::RelatesTo
        ));
        assert!(matches!(
            parse_relationship("extends"),
            Relationship::Extends
        ));
    }

    #[test]
    fn test_parse_relationship_custom() {
        match parse_relationship("my_custom") {
            Relationship::Custom(s) => assert_eq!(s, "my_custom"),
            _ => panic!("Expected Custom relationship"),
        }
    }
}
