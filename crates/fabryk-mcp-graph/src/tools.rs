//! MCP tools for graph queries.
//!
//! Provides `GraphTools` that implements `ToolRegistry` by delegating
//! queries to `fabryk_graph` algorithms.

use fabryk_mcp_core::error::McpErrorExt;
use fabryk_mcp_core::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp_core::registry::{ToolRegistry, ToolResult};

use fabryk_graph::{
    EdgeInfo, GraphData, NeighborInfo, Node, NodeSummary, PathStep, Relationship,
    bridge_between_categories, calculate_centrality, compute_stats, concept_sources,
    concept_variants, dependents, find_bridges, get_node_detail, get_node_edges, learning_path,
    neighborhood, prerequisites_sorted, shortest_path, source_coverage, validate_graph,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// ---------------------------------------------------------------------------
// GraphNodeFilter trait
// ---------------------------------------------------------------------------

/// Domain-specific post-filter for graph query results.
///
/// Implement this trait to filter graph nodes based on domain-specific
/// criteria extracted from MCP tool arguments (e.g., tier, confidence level).
///
/// # Example
///
/// ```rust,ignore
/// struct MusicTheoryFilter;
///
/// impl GraphNodeFilter for MusicTheoryFilter {
///     fn matches(&self, node: &Node, extra_args: &serde_json::Value) -> bool {
///         if let Some(tier) = extra_args.get("tier").and_then(|v| v.as_str()) {
///             if node.metadata.get("tier").and_then(|v| v.as_str()) != Some(tier) {
///                 return false;
///             }
///         }
///         true
///     }
/// }
/// ```
pub trait GraphNodeFilter: Send + Sync {
    /// Test whether a node passes the domain-specific filter.
    ///
    /// `extra_args` contains the full tool call arguments as JSON,
    /// allowing the filter to extract domain-specific parameters.
    fn matches(&self, node: &Node, extra_args: &serde_json::Value) -> bool;
}

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

fn apply_node_filter(
    nodes: Vec<Node>,
    args: &Value,
    filter: &Option<Arc<dyn GraphNodeFilter>>,
) -> Vec<Node> {
    match filter {
        Some(f) => nodes.into_iter().filter(|n| f.matches(n, args)).collect(),
        None => nodes,
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

/// Arguments for graph_get_node tool.
#[derive(Debug, Deserialize)]
pub struct GetNodeArgs {
    /// Node ID to look up.
    pub id: String,
}

/// Arguments for graph_get_node_edges tool.
#[derive(Debug, Deserialize)]
pub struct GetNodeEdgesArgs {
    /// Node ID to get edges for.
    pub id: String,
    /// Direction filter: "incoming", "outgoing", or "both" (default).
    #[serde(default = "default_direction")]
    pub direction: Option<String>,
}

fn default_direction() -> Option<String> {
    None
}

/// Arguments for graph_dependents tool.
#[derive(Debug, Deserialize)]
pub struct DependentsArgs {
    /// Node ID to find dependents for.
    pub id: String,
    /// Maximum traversal depth (default 3).
    #[serde(default = "default_depth")]
    pub depth: Option<usize>,
}

fn default_depth() -> Option<usize> {
    None
}

/// Arguments for graph_learning_path tool.
#[derive(Debug, Deserialize)]
pub struct LearningPathArgs {
    /// Target node ID.
    pub id: String,
}

/// Arguments for graph_bridge_categories tool.
#[derive(Debug, Deserialize)]
pub struct BridgeCategoriesArgs {
    /// First category name.
    pub category_a: String,
    /// Second category name.
    pub category_b: String,
    /// Maximum results (default 10).
    #[serde(default)]
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// GraphTools
// ---------------------------------------------------------------------------

/// MCP tools for graph queries.
///
/// Generates seventeen tools:
/// - `graph_related` — find related nodes
/// - `graph_path` — shortest path between nodes
/// - `graph_prerequisites` — learning order prerequisites
/// - `graph_neighborhood` — N-hop neighborhood exploration
/// - `graph_info` — graph statistics
/// - `graph_validate` — structure validation
/// - `graph_centrality` — most central/important nodes
/// - `graph_bridges` — bridge nodes connecting different areas
/// - `graph_get_node` — detailed information about a single node
/// - `graph_get_node_edges` — edges connected to a node
/// - `graph_dependents` — nodes that depend on a given node (reverse prerequisites)
/// - `graph_status` — whether graph is loaded with basic stats
/// - `graph_concept_sources` — find sources that introduce or cover a concept
/// - `graph_concept_variants` — find source-specific variants of a canonical concept
/// - `graph_source_coverage` — find concepts that a source introduces or covers
/// - `graph_learning_path` — step-numbered learning path to a target concept
/// - `graph_bridge_categories` — find nodes connecting two specific categories
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
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
    node_filter: Option<Arc<dyn GraphNodeFilter>>,
    extra_schemas: HashMap<String, serde_json::Value>,
}

impl GraphTools {
    /// Slot key for the related nodes tool.
    pub const SLOT_RELATED: &str = "graph_related";
    /// Slot key for the shortest path tool.
    pub const SLOT_PATH: &str = "graph_path";
    /// Slot key for the prerequisites tool.
    pub const SLOT_PREREQUISITES: &str = "graph_prerequisites";
    /// Slot key for the neighborhood tool.
    pub const SLOT_NEIGHBORHOOD: &str = "graph_neighborhood";
    /// Slot key for the graph info/stats tool.
    pub const SLOT_INFO: &str = "graph_info";
    /// Slot key for the validation tool.
    pub const SLOT_VALIDATE: &str = "graph_validate";
    /// Slot key for the centrality tool.
    pub const SLOT_CENTRALITY: &str = "graph_centrality";
    /// Slot key for the bridges tool.
    pub const SLOT_BRIDGES: &str = "graph_bridges";
    /// Slot key for the single-node detail tool.
    pub const SLOT_GET_NODE: &str = "graph_get_node";
    /// Slot key for the node edges tool.
    pub const SLOT_GET_NODE_EDGES: &str = "graph_get_node_edges";
    /// Slot key for the dependents tool.
    pub const SLOT_DEPENDENTS: &str = "graph_dependents";
    /// Slot key for the graph status tool.
    pub const SLOT_STATUS: &str = "graph_status";
    /// Slot key for the concept sources tool.
    pub const SLOT_CONCEPT_SOURCES: &str = "graph_concept_sources";
    /// Slot key for the concept variants tool.
    pub const SLOT_CONCEPT_VARIANTS: &str = "graph_concept_variants";
    /// Slot key for the source coverage tool.
    pub const SLOT_SOURCE_COVERAGE: &str = "graph_source_coverage";
    /// Slot key for the learning path tool.
    pub const SLOT_LEARNING_PATH: &str = "graph_learning_path";
    /// Slot key for the bridge categories tool.
    pub const SLOT_BRIDGE_CATEGORIES: &str = "graph_bridge_categories";

    /// Create new graph tools with owned graph data.
    pub fn new(graph: GraphData) -> Self {
        Self {
            graph: Arc::new(RwLock::new(graph)),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            node_filter: None,
            extra_schemas: HashMap::new(),
        }
    }

    /// Create graph tools with a shared graph reference.
    pub fn with_shared(graph: Arc<RwLock<GraphData>>) -> Self {
        Self {
            graph,
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            node_filter: None,
            extra_schemas: HashMap::new(),
        }
    }

    /// Override tool names by slot key.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override tool descriptions by slot key.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    /// Update the graph data (e.g., after rebuild).
    pub async fn update_graph(&self, graph: GraphData) {
        let mut lock = self.graph.write().await;
        *lock = graph;
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

    /// Set a domain-specific node filter applied to graph query results.
    ///
    /// The filter is applied as a post-processing step to tools that return
    /// lists of nodes. Tools that return a single node or structured results
    /// (e.g., `graph_get_node`, `graph_path`, `graph_info`) are not affected.
    pub fn with_node_filter(mut self, filter: Arc<dyn GraphNodeFilter>) -> Self {
        self.node_filter = Some(filter);
        self
    }

    /// Add extra JSON schema properties to a specific tool slot.
    ///
    /// The extra properties are merged into the tool's `properties` object,
    /// making domain-specific parameters visible to MCP clients.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use serde_json::json;
    ///
    /// let tools = GraphTools::new(graph)
    ///     .with_extra_schema("graph_dependents", json!({
    ///         "tier": {
    ///             "type": "string",
    ///             "description": "Filter by tier (e.g., foundational, advanced)"
    ///         }
    ///     }));
    /// ```
    pub fn with_extra_schema(mut self, slot: impl Into<String>, schema: serde_json::Value) -> Self {
        self.extra_schemas.insert(slot.into(), schema);
        self
    }

    /// Merge extra schema properties into a tool's JSON schema for a given slot.
    fn merge_extra_schema(&self, slot: &str, mut schema: Value) -> Value {
        if let Some(extra) = self.extra_schemas.get(slot)
            && let (Some(props), Some(extra_props)) =
                (schema.pointer_mut("/properties"), extra.as_object())
            && let Some(props_map) = props.as_object_mut()
        {
            for (key, value) in extra_props {
                props_map.insert(key.clone(), value.clone());
            }
        }
        schema
    }
}

impl ToolRegistry for GraphTools {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                &self.tool_name(Self::SLOT_RELATED),
                &self.tool_description(Self::SLOT_RELATED, "Find nodes related to a given node"),
                self.merge_extra_schema(Self::SLOT_RELATED, json!({
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
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_PATH),
                &self.tool_description(Self::SLOT_PATH, "Find the shortest path between two nodes"),
                self.merge_extra_schema(Self::SLOT_PATH, json!({
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
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_PREREQUISITES),
                &self.tool_description(
                    Self::SLOT_PREREQUISITES,
                    "Get prerequisites for a node in learning order",
                ),
                self.merge_extra_schema(Self::SLOT_PREREQUISITES, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_NEIGHBORHOOD),
                &self.tool_description(
                    Self::SLOT_NEIGHBORHOOD,
                    "Explore the neighborhood around a node",
                ),
                self.merge_extra_schema(Self::SLOT_NEIGHBORHOOD, json!({
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
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_INFO),
                &self.tool_description(Self::SLOT_INFO, "Get graph statistics and overview"),
                self.merge_extra_schema(Self::SLOT_INFO, json!({
                    "type": "object",
                    "properties": {}
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_VALIDATE),
                &self.tool_description(
                    Self::SLOT_VALIDATE,
                    "Validate graph structure and report issues",
                ),
                self.merge_extra_schema(Self::SLOT_VALIDATE, json!({
                    "type": "object",
                    "properties": {}
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_CENTRALITY),
                &self.tool_description(Self::SLOT_CENTRALITY, "Get most central/important nodes"),
                self.merge_extra_schema(Self::SLOT_CENTRALITY, json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_BRIDGES),
                &self.tool_description(
                    Self::SLOT_BRIDGES,
                    "Find bridge nodes that connect different areas",
                ),
                self.merge_extra_schema(Self::SLOT_BRIDGES, json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "integer",
                            "description": "Number of results (default 10)"
                        }
                    }
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_GET_NODE),
                &self.tool_description(
                    Self::SLOT_GET_NODE,
                    "Get detailed information about a single node including degree counts",
                ),
                self.merge_extra_schema(Self::SLOT_GET_NODE, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_GET_NODE_EDGES),
                &self.tool_description(
                    Self::SLOT_GET_NODE_EDGES,
                    "Get all edges connected to a node, optionally filtered by direction",
                ),
                self.merge_extra_schema(Self::SLOT_GET_NODE_EDGES, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        },
                        "direction": {
                            "type": "string",
                            "description": "Direction filter: incoming, outgoing, or both (default: both)",
                            "enum": ["incoming", "outgoing", "both"]
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_DEPENDENTS),
                &self.tool_description(
                    Self::SLOT_DEPENDENTS,
                    "Find nodes that depend on a given node (reverse prerequisite traversal)",
                ),
                self.merge_extra_schema(Self::SLOT_DEPENDENTS, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Node ID"
                        },
                        "depth": {
                            "type": "integer",
                            "description": "Maximum traversal depth (default 3)"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_STATUS),
                &self.tool_description(
                    Self::SLOT_STATUS,
                    "Report whether the graph is loaded with basic statistics",
                ),
                self.merge_extra_schema(Self::SLOT_STATUS, json!({
                    "type": "object",
                    "properties": {}
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_CONCEPT_SOURCES),
                &self.tool_description(
                    Self::SLOT_CONCEPT_SOURCES,
                    "Find all sources (books/papers) that introduce or cover a given concept",
                ),
                self.merge_extra_schema(Self::SLOT_CONCEPT_SOURCES, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Concept node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_CONCEPT_VARIANTS),
                &self.tool_description(
                    Self::SLOT_CONCEPT_VARIANTS,
                    "Find source-specific variants of a canonical concept",
                ),
                self.merge_extra_schema(Self::SLOT_CONCEPT_VARIANTS, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Canonical concept node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_SOURCE_COVERAGE),
                &self.tool_description(
                    Self::SLOT_SOURCE_COVERAGE,
                    "Find all concepts that a given source introduces or covers",
                ),
                self.merge_extra_schema(Self::SLOT_SOURCE_COVERAGE, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Source node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_LEARNING_PATH),
                &self.tool_description(
                    Self::SLOT_LEARNING_PATH,
                    "Get a step-numbered learning path to reach a target concept",
                ),
                self.merge_extra_schema(Self::SLOT_LEARNING_PATH, json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "Target node ID"
                        }
                    },
                    "required": ["id"]
                })),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_BRIDGE_CATEGORIES),
                &self.tool_description(
                    Self::SLOT_BRIDGE_CATEGORIES,
                    "Find nodes that bridge two categories (have neighbors in both)",
                ),
                self.merge_extra_schema(Self::SLOT_BRIDGE_CATEGORIES, json!({
                    "type": "object",
                    "properties": {
                        "category_a": {
                            "type": "string",
                            "description": "First category name"
                        },
                        "category_b": {
                            "type": "string",
                            "description": "Second category name"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum results (default 10)"
                        }
                    },
                    "required": ["category_a", "category_b"]
                })),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let graph = Arc::clone(&self.graph);
        let node_filter = self.node_filter.clone();

        if name == self.tool_name(Self::SLOT_RELATED) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: RelatedArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let rel_filter = args
                    .relationship
                    .as_deref()
                    .map(|r| vec![parse_relationship(r)]);

                let result = neighborhood(&graph, &args.id, 1, rel_filter.as_deref())
                    .map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(result.nodes, &raw_args, &node_filter);
                let mut nodes: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();

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
            }));
        }

        if name == self.tool_name(Self::SLOT_PATH) {
            return Some(Box::pin(async move {
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
            }));
        }

        if name == self.tool_name(Self::SLOT_PREREQUISITES) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: PrerequisitesArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let result =
                    prerequisites_sorted(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(result.ordered, &raw_args, &node_filter);
                let prereqs: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();

                let count = prereqs.len();
                let response = json!({
                    "target": NodeSummary::from(&result.target),
                    "prerequisites": prereqs,
                    "count": count,
                    "has_cycles": result.has_cycles
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_NEIGHBORHOOD) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
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

                let filtered = apply_node_filter(result.nodes, &raw_args, &node_filter);
                let nodes: Vec<NeighborInfo> = filtered
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
            }));
        }

        if name == self.tool_name(Self::SLOT_INFO) {
            return Some(Box::pin(async move {
                let graph = graph.read().await;
                let stats = compute_stats(&graph);
                serialize_response(&stats)
            }));
        }

        if name == self.tool_name(Self::SLOT_VALIDATE) {
            return Some(Box::pin(async move {
                let graph = graph.read().await;
                let result = validate_graph(&graph);
                serialize_response(&result)
            }));
        }

        if name == self.tool_name(Self::SLOT_CENTRALITY) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let scores = calculate_centrality(&graph);

                let filtered: Vec<_> = match &node_filter {
                    Some(filter) => scores
                        .into_iter()
                        .filter(|s| {
                            graph
                                .get_node(&s.node_id)
                                .map(|n| filter.matches(n, &raw_args))
                                .unwrap_or(false)
                        })
                        .take(limit)
                        .collect(),
                    None => scores.into_iter().take(limit).collect(),
                };
                serialize_response(&filtered)
            }));
        }

        if name == self.tool_name(Self::SLOT_BRIDGES) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let limit = args
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as usize)
                    .unwrap_or(10);

                let graph = graph.read().await;
                let bridges = find_bridges(&graph, limit);

                let filtered = apply_node_filter(bridges, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                serialize_response(&summaries)
            }));
        }

        if name == self.tool_name(Self::SLOT_GET_NODE) {
            return Some(Box::pin(async move {
                let args: GetNodeArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let detail = get_node_detail(&graph, &args.id).map_err(|e| e.to_mcp_error())?;
                serialize_response(&detail)
            }));
        }

        if name == self.tool_name(Self::SLOT_GET_NODE_EDGES) {
            return Some(Box::pin(async move {
                let args: GetNodeEdgesArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let result = get_node_edges(&graph, &args.id, args.direction.as_deref())
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&result)
            }));
        }

        if name == self.tool_name(Self::SLOT_DEPENDENTS) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: DependentsArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let max_depth = args.depth.unwrap_or(3);
                let nodes =
                    dependents(&graph, &args.id, max_depth).map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(nodes, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                let count = summaries.len();
                let response = json!({
                    "source": args.id,
                    "dependents": summaries,
                    "count": count,
                    "max_depth": max_depth
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_STATUS) {
            return Some(Box::pin(async move {
                let graph = graph.read().await;
                let node_count = graph.node_count();
                let edge_count = graph.edge_count();
                let loaded = node_count > 0 || edge_count > 0;
                let response = json!({
                    "loaded": loaded,
                    "node_count": node_count,
                    "edge_count": edge_count
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_CONCEPT_SOURCES) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: GetNodeArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let nodes = concept_sources(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(nodes, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                let count = summaries.len();
                let response = json!({
                    "concept": args.id,
                    "sources": summaries,
                    "count": count
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_CONCEPT_VARIANTS) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: GetNodeArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let nodes = concept_variants(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(nodes, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                let count = summaries.len();
                let response = json!({
                    "canonical": args.id,
                    "variants": summaries,
                    "count": count
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_SOURCE_COVERAGE) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: GetNodeArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let nodes = source_coverage(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(nodes, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                let count = summaries.len();
                let response = json!({
                    "source": args.id,
                    "concepts": summaries,
                    "count": count
                });
                serialize_response(&response)
            }));
        }

        if name == self.tool_name(Self::SLOT_LEARNING_PATH) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: LearningPathArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let mut result = learning_path(&graph, &args.id).map_err(|e| e.to_mcp_error())?;

                if let Some(filter) = &node_filter {
                    result.steps.retain(|step| {
                        graph
                            .get_node(&step.node_id)
                            .map(|n| filter.matches(n, &raw_args))
                            .unwrap_or(true)
                    });
                    result.total_steps = result.steps.len();
                }
                serialize_response(&result)
            }));
        }

        if name == self.tool_name(Self::SLOT_BRIDGE_CATEGORIES) {
            let raw_args = args.clone();
            let node_filter = node_filter.clone();
            return Some(Box::pin(async move {
                let args: BridgeCategoriesArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let graph = graph.read().await;

                let limit = args.limit.unwrap_or(10);
                let nodes =
                    bridge_between_categories(&graph, &args.category_a, &args.category_b, limit)
                        .map_err(|e| e.to_mcp_error())?;

                let filtered = apply_node_filter(nodes, &raw_args, &node_filter);
                let summaries: Vec<NodeSummary> = filtered.iter().map(NodeSummary::from).collect();
                let count = summaries.len();
                let response = json!({
                    "category_a": args.category_a,
                    "category_b": args.category_b,
                    "bridges": summaries,
                    "count": count
                });
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
    use fabryk_graph::{Edge, Node};

    fn make_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        // Add nodes with tier metadata for filter testing
        graph.add_node(
            Node::new("node-a", "Node A")
                .with_category("alpha")
                .with_metadata("tier", "foundational"),
        );
        graph.add_node(
            Node::new("node-b", "Node B")
                .with_category("beta")
                .with_metadata("tier", "advanced"),
        );
        graph.add_node(
            Node::new("node-c", "Node C")
                .with_category("alpha")
                .with_metadata("tier", "foundational"),
        );

        // Add edges: A -> B (prerequisite), B -> C (relates_to)
        let _ = graph.add_edge(Edge::new("node-a", "node-b", Relationship::Prerequisite));
        let _ = graph.add_edge(Edge::new("node-b", "node-c", Relationship::RelatesTo));

        graph
    }

    /// A test filter that filters nodes by tier metadata.
    struct TierFilter;

    impl GraphNodeFilter for TierFilter {
        fn matches(&self, node: &Node, extra_args: &serde_json::Value) -> bool {
            if let Some(tier) = extra_args.get("tier").and_then(|v| v.as_str()) {
                return node.metadata.get("tier").and_then(|v| v.as_str()) == Some(tier);
            }
            true // no tier filter specified, pass everything
        }
    }

    // -- Tool creation tests ------------------------------------------------

    #[test]
    fn test_graph_tools_creation() {
        let tools = GraphTools::new(GraphData::new());
        assert_eq!(tools.tool_count(), 17);
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
        assert!(names.contains(&"graph_get_node"));
        assert!(names.contains(&"graph_get_node_edges"));
        assert!(names.contains(&"graph_dependents"));
        assert!(names.contains(&"graph_status"));
        assert!(names.contains(&"graph_concept_sources"));
        assert!(names.contains(&"graph_concept_variants"));
        assert!(names.contains(&"graph_source_coverage"));
        assert!(names.contains(&"graph_learning_path"));
        assert!(names.contains(&"graph_bridge_categories"));
    }

    #[test]
    fn test_graph_tools_has_tool() {
        let tools = GraphTools::new(GraphData::new());
        assert!(tools.has_tool("graph_related"));
        assert!(tools.has_tool("graph_info"));
        assert!(tools.has_tool("graph_get_node"));
        assert!(tools.has_tool("graph_get_node_edges"));
        assert!(tools.has_tool("graph_dependents"));
        assert!(tools.has_tool("graph_status"));
        assert!(tools.has_tool("graph_concept_sources"));
        assert!(tools.has_tool("graph_concept_variants"));
        assert!(tools.has_tool("graph_source_coverage"));
        assert!(tools.has_tool("graph_learning_path"));
        assert!(tools.has_tool("graph_bridge_categories"));
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

    // -- Custom name/description tests -------------------------------------

    #[test]
    fn test_graph_tools_with_custom_names() {
        let tools = GraphTools::new(GraphData::new()).with_names(HashMap::from([
            (
                "graph_related".to_string(),
                "get_related_concepts".to_string(),
            ),
            ("graph_path".to_string(), "find_concept_path".to_string()),
            ("graph_info".to_string(), "graph_stats".to_string()),
        ]));
        let tool_list = tools.tools();
        let names: Vec<&str> = tool_list.iter().map(|t| t.name.as_ref()).collect();
        assert!(names.contains(&"get_related_concepts"));
        assert!(names.contains(&"find_concept_path"));
        assert!(names.contains(&"graph_stats"));
        // Unrenamed tools keep defaults
        assert!(names.contains(&"graph_prerequisites"));
    }

    #[tokio::test]
    async fn test_graph_tools_custom_names_dispatch() {
        let tools = GraphTools::new(make_test_graph()).with_names(HashMap::from([(
            "graph_info".to_string(),
            "graph_stats".to_string(),
        )]));
        // Old name should NOT work
        assert!(tools.call("graph_info", json!({})).is_none());
        // Custom name should work
        let future = tools.call("graph_stats", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_parse_relationship_custom() {
        match parse_relationship("my_custom") {
            Relationship::Custom(s) => assert_eq!(s, "my_custom"),
            _ => panic!("Expected Custom relationship"),
        }
    }

    // -- graph_get_node tests -------------------------------------------------

    #[tokio::test]
    async fn test_graph_get_node() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_get_node", json!({"id": "node-a"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        // Verify the response contains expected fields
        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["id"], "node-a");
        assert_eq!(parsed["title"], "Node A");
        assert_eq!(parsed["node_type"], "domain");
        assert_eq!(parsed["is_canonical"], true);
    }

    #[tokio::test]
    async fn test_graph_get_node_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_get_node", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_get_node_degrees() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_get_node", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        // node-b: in=1 (a->b), out=1 (b->c)
        assert_eq!(parsed["in_degree"], 1);
        assert_eq!(parsed["out_degree"], 1);
    }

    // -- graph_get_node_edges tests -------------------------------------------

    #[tokio::test]
    async fn test_graph_get_node_edges_both() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_get_node_edges", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["node_id"], "node-b");
        assert_eq!(parsed["incoming"].as_array().unwrap().len(), 1);
        assert_eq!(parsed["outgoing"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_graph_get_node_edges_incoming() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call(
                "graph_get_node_edges",
                json!({"id": "node-b", "direction": "incoming"}),
            )
            .unwrap();
        let result = future.await.unwrap();

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["incoming"].as_array().unwrap().len(), 1);
        assert!(parsed["outgoing"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_graph_get_node_edges_outgoing() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call(
                "graph_get_node_edges",
                json!({"id": "node-b", "direction": "outgoing"}),
            )
            .unwrap();
        let result = future.await.unwrap();

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["incoming"].as_array().unwrap().is_empty());
        assert_eq!(parsed["outgoing"].as_array().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_graph_get_node_edges_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_get_node_edges", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    // -- graph_dependents tests -----------------------------------------------

    #[tokio::test]
    async fn test_graph_dependents() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_dependents", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["source"], "node-b");
        // node-b has incoming Prerequisite from node-a
        let deps = parsed["dependents"].as_array().unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0]["id"], "node-a");
    }

    #[tokio::test]
    async fn test_graph_dependents_with_depth() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_dependents", json!({"id": "node-b", "depth": 1}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["max_depth"], 1);
    }

    #[tokio::test]
    async fn test_graph_dependents_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_dependents", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_dependents_no_deps() {
        let tools = GraphTools::new(make_test_graph());
        // node-a has no incoming prerequisite edges
        let future = tools
            .call("graph_dependents", json!({"id": "node-a"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 0);
    }

    // -- graph_status tests ---------------------------------------------------

    #[tokio::test]
    async fn test_graph_status_empty() {
        let tools = GraphTools::new(GraphData::new());
        let future = tools.call("graph_status", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["loaded"], false);
        assert_eq!(parsed["node_count"], 0);
        assert_eq!(parsed["edge_count"], 0);
    }

    #[tokio::test]
    async fn test_graph_status_with_data() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools.call("graph_status", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["loaded"], true);
        assert_eq!(parsed["node_count"], 3);
        assert_eq!(parsed["edge_count"], 2);
    }

    // -- Source-concept relationship tool tests --------------------------------

    fn make_source_concept_graph() -> GraphData {
        use fabryk_graph::NodeType;

        let mut graph = GraphData::new();

        // Source nodes
        graph.add_node(
            Node::new("book-alpha", "Book Alpha")
                .with_node_type(NodeType::Custom("source".to_string())),
        );
        graph.add_node(
            Node::new("book-beta", "Book Beta")
                .with_node_type(NodeType::Custom("source".to_string())),
        );

        // Canonical concept nodes
        graph.add_node(Node::new("concept-x", "Concept X").with_category("math"));
        graph.add_node(Node::new("concept-y", "Concept Y").with_category("math"));

        // Variant nodes
        graph.add_node(
            Node::new("concept-x-alpha", "Concept X (Alpha)")
                .with_node_type(NodeType::Custom("source".to_string())),
        );
        graph.add_node(
            Node::new("concept-x-beta", "Concept X (Beta)")
                .with_node_type(NodeType::Custom("source".to_string())),
        );

        // Source -> concept edges
        let _ = graph.add_edge(Edge::new(
            "book-alpha",
            "concept-x",
            Relationship::Introduces,
        ));
        let _ = graph.add_edge(Edge::new("book-alpha", "concept-y", Relationship::Covers));
        let _ = graph.add_edge(Edge::new("book-beta", "concept-x", Relationship::Covers));

        // Variant edges
        let _ = graph.add_edge(Edge::new(
            "concept-x-alpha",
            "concept-x",
            Relationship::VariantOf,
        ));
        let _ = graph.add_edge(Edge::new(
            "concept-x-beta",
            "concept-x",
            Relationship::VariantOf,
        ));

        graph
    }

    // -- graph_concept_sources tests ------------------------------------------

    #[tokio::test]
    async fn test_graph_concept_sources() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_sources", json!({"id": "concept-x"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["concept"], "concept-x");
        assert_eq!(parsed["count"], 2);
        let sources = parsed["sources"].as_array().unwrap();
        let source_ids: Vec<&str> = sources.iter().map(|s| s["id"].as_str().unwrap()).collect();
        assert!(source_ids.contains(&"book-alpha"));
        assert!(source_ids.contains(&"book-beta"));
    }

    #[tokio::test]
    async fn test_graph_concept_sources_not_found() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_sources", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_concept_sources_none() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_sources", json!({"id": "concept-x-alpha"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 0);
    }

    // -- graph_concept_variants tests -----------------------------------------

    #[tokio::test]
    async fn test_graph_concept_variants() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_variants", json!({"id": "concept-x"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["canonical"], "concept-x");
        assert_eq!(parsed["count"], 2);
        let variants = parsed["variants"].as_array().unwrap();
        let variant_ids: Vec<&str> = variants.iter().map(|v| v["id"].as_str().unwrap()).collect();
        assert!(variant_ids.contains(&"concept-x-alpha"));
        assert!(variant_ids.contains(&"concept-x-beta"));
    }

    #[tokio::test]
    async fn test_graph_concept_variants_not_found() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_variants", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_concept_variants_none() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_concept_variants", json!({"id": "concept-y"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 0);
    }

    // -- graph_source_coverage tests ------------------------------------------

    #[tokio::test]
    async fn test_graph_source_coverage() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_source_coverage", json!({"id": "book-alpha"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["source"], "book-alpha");
        assert_eq!(parsed["count"], 2);
        let concepts = parsed["concepts"].as_array().unwrap();
        let concept_ids: Vec<&str> = concepts.iter().map(|c| c["id"].as_str().unwrap()).collect();
        assert!(concept_ids.contains(&"concept-x"));
        assert!(concept_ids.contains(&"concept-y"));
    }

    #[tokio::test]
    async fn test_graph_source_coverage_not_found() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_source_coverage", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_graph_source_coverage_single() {
        let tools = GraphTools::new(make_source_concept_graph());
        let future = tools
            .call("graph_source_coverage", json!({"id": "book-beta"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 1);
        let concepts = parsed["concepts"].as_array().unwrap();
        assert_eq!(concepts[0]["id"], "concept-x");
    }

    // -- graph_learning_path tests --------------------------------------------

    #[tokio::test]
    async fn test_graph_learning_path() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_learning_path", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["target"]["id"], "node-b");
        // node-a is prerequisite of node-b, so steps: [node-a, node-b]
        assert_eq!(parsed["total_steps"], 2);
        let steps = parsed["steps"].as_array().unwrap();
        assert_eq!(steps[0]["node_id"], "node-a");
        assert_eq!(steps[0]["step"], 1);
        assert_eq!(steps[1]["node_id"], "node-b");
        assert_eq!(steps[1]["step"], 2);
    }

    #[tokio::test]
    async fn test_graph_learning_path_no_prerequisites() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_learning_path", json!({"id": "node-a"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["total_steps"], 1);
        let steps = parsed["steps"].as_array().unwrap();
        assert_eq!(steps[0]["node_id"], "node-a");
    }

    #[tokio::test]
    async fn test_graph_learning_path_not_found() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call("graph_learning_path", json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    // -- graph_bridge_categories tests ----------------------------------------

    #[tokio::test]
    async fn test_graph_bridge_categories() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call(
                "graph_bridge_categories",
                json!({"category_a": "alpha", "category_b": "beta"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["category_a"], "alpha");
        assert_eq!(parsed["category_b"], "beta");
    }

    #[tokio::test]
    async fn test_graph_bridge_categories_no_match() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call(
                "graph_bridge_categories",
                json!({"category_a": "alpha", "category_b": "nonexistent"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 0);
    }

    #[tokio::test]
    async fn test_graph_bridge_categories_with_limit() {
        let tools = GraphTools::new(make_test_graph());
        let future = tools
            .call(
                "graph_bridge_categories",
                json!({"category_a": "alpha", "category_b": "beta", "limit": 1}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        let bridges = parsed["bridges"].as_array().unwrap();
        assert!(bridges.len() <= 1);
    }

    // -- GraphNodeFilter tests ------------------------------------------------

    #[test]
    fn test_graph_tools_no_filter() {
        // Default GraphTools has no filter — all nodes pass through.
        let tools = GraphTools::new(make_test_graph());
        assert!(tools.node_filter.is_none());
        assert!(tools.extra_schemas.is_empty());
        // Still has the correct tool count.
        assert_eq!(tools.tool_count(), 17);
    }

    #[tokio::test]
    async fn test_graph_tools_with_node_filter() {
        let tools = GraphTools::new(make_test_graph()).with_node_filter(Arc::new(TierFilter));

        // Call graph_dependents for node-b. node-a is the only dependent
        // and has tier "foundational". Filter for "advanced" should exclude it.
        let future = tools
            .call(
                "graph_dependents",
                json!({"id": "node-b", "tier": "advanced"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        // node-a is "foundational", so filtering by "advanced" should yield 0
        assert_eq!(parsed["count"], 0);
        assert!(parsed["dependents"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_graph_tools_with_node_filter_matching() {
        let tools = GraphTools::new(make_test_graph()).with_node_filter(Arc::new(TierFilter));

        // Call graph_dependents for node-b. Filter for "foundational" should
        // include node-a.
        let future = tools
            .call(
                "graph_dependents",
                json!({"id": "node-b", "tier": "foundational"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 1);
        let deps = parsed["dependents"].as_array().unwrap();
        assert_eq!(deps[0]["id"], "node-a");
    }

    #[tokio::test]
    async fn test_graph_tools_with_node_filter_no_tier_arg() {
        let tools = GraphTools::new(make_test_graph()).with_node_filter(Arc::new(TierFilter));

        // Without a tier arg the filter passes everything through.
        let future = tools
            .call("graph_dependents", json!({"id": "node-b"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert_eq!(parsed["count"], 1);
    }

    #[test]
    fn test_graph_tools_extra_schema() {
        let tools = GraphTools::new(make_test_graph()).with_extra_schema(
            "graph_dependents",
            json!({
                "tier": {
                    "type": "string",
                    "description": "Filter by tier (e.g., foundational, advanced)"
                }
            }),
        );

        let tool_list = tools.tools();
        let dep_tool = tool_list
            .iter()
            .find(|t| t.name == "graph_dependents")
            .expect("graph_dependents tool should exist");

        // The schema should now contain a "tier" property.
        let schema_value: serde_json::Value =
            serde_json::to_value(&*dep_tool.input_schema).unwrap();
        let props = schema_value["properties"].as_object().unwrap();
        assert!(
            props.contains_key("tier"),
            "Schema should contain 'tier' property"
        );
        assert_eq!(props["tier"]["type"], "string");
        assert!(
            props.contains_key("id"),
            "Original 'id' property should still be present"
        );
    }

    #[test]
    fn test_graph_tools_extra_schema_no_override() {
        // Extra schemas for a tool that doesn't exist should be harmless.
        let tools = GraphTools::new(make_test_graph()).with_extra_schema(
            "graph_nonexistent",
            json!({
                "foo": { "type": "string" }
            }),
        );

        let tool_list = tools.tools();
        // All existing tools should be unaffected.
        assert_eq!(tool_list.len(), 17);
    }

    #[tokio::test]
    async fn test_graph_tools_filter_does_not_affect_single_node() {
        let tools = GraphTools::new(make_test_graph()).with_node_filter(Arc::new(TierFilter));

        // graph_get_node returns a single node detail, not a list.
        // The filter should NOT prevent it from returning node-a
        // even with a tier that doesn't match.
        let future = tools
            .call(
                "graph_get_node",
                json!({"id": "node-a", "tier": "advanced"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        // node-a is "foundational" but get_node ignores the filter.
        assert_eq!(parsed["id"], "node-a");
        assert_eq!(parsed["title"], "Node A");
    }

    #[tokio::test]
    async fn test_graph_tools_filter_on_bridges() {
        let tools = GraphTools::new(make_test_graph()).with_node_filter(Arc::new(TierFilter));

        let future = tools
            .call(
                "graph_bridges",
                json!({"limit": 10, "tier": "foundational"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        // All returned bridges should have tier "foundational".
        let content_json = serde_json::to_string(&result.content[0]).unwrap();
        let content_obj: serde_json::Value = serde_json::from_str(&content_json).unwrap();
        let text = content_obj["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        let bridges = parsed.as_array().unwrap();
        for bridge in bridges {
            let id = bridge["id"].as_str().unwrap();
            // Only "node-a" and "node-c" have tier "foundational"
            assert!(
                id == "node-a" || id == "node-c",
                "Unexpected bridge node: {id}"
            );
        }
    }
}
