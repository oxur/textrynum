//! Query response types for graph operations.
//!
//! This module provides structured response types used by MCP tools
//! and other interfaces to return graph query results. All types
//! derive `Serialize`/`Deserialize` for JSON transport.

use crate::{Edge, GraphData, Node};
use fabryk_core::Result;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Node / Edge summaries
// ============================================================================

/// Summary information about a node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeSummary {
    /// Node ID.
    pub id: String,
    /// Node title.
    pub title: String,
    /// Optional category.
    pub category: Option<String>,
    /// Optional description or summary.
    pub description: Option<String>,
}

impl From<&Node> for NodeSummary {
    fn from(node: &Node) -> Self {
        Self {
            id: node.id.clone(),
            title: node.title.clone(),
            category: node.category.clone(),
            description: node
                .metadata
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
        }
    }
}

/// Summary information about an edge.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeInfo {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Relationship type.
    pub relationship: String,
    /// Edge weight.
    pub weight: f32,
}

impl From<&Edge> for EdgeInfo {
    fn from(edge: &Edge) -> Self {
        Self {
            from: edge.from.clone(),
            to: edge.to.clone(),
            relationship: edge.relationship.name().to_string(),
            weight: edge.weight,
        }
    }
}

// ============================================================================
// Related concepts
// ============================================================================

/// Response for related concepts query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelatedConceptsResponse {
    /// The source concept.
    pub source: NodeSummary,
    /// Related concepts grouped by relationship type.
    pub related: Vec<RelatedGroup>,
    /// Total count of related concepts.
    pub total_count: usize,
}

/// A group of related concepts sharing a relationship type.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelatedGroup {
    /// The relationship type.
    pub relationship: String,
    /// Concepts in this group.
    pub concepts: Vec<NodeSummary>,
}

// ============================================================================
// Path
// ============================================================================

/// Response for concept path query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathResponse {
    /// Source node.
    pub from: NodeSummary,
    /// Target node.
    pub to: NodeSummary,
    /// Path nodes in order (including from and to).
    pub path: Vec<PathStep>,
    /// Whether a path was found.
    pub found: bool,
    /// Total path length.
    pub length: usize,
}

/// A step in a path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PathStep {
    /// The node at this step.
    pub node: NodeSummary,
    /// Relationship to the next node (None for last node).
    pub relationship_to_next: Option<String>,
}

// ============================================================================
// Prerequisites
// ============================================================================

/// Response for prerequisites query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrerequisitesResponse {
    /// Target concept.
    pub target: NodeSummary,
    /// Prerequisites in learning order (fundamentals first).
    pub prerequisites: Vec<PrerequisiteInfo>,
    /// Total count.
    pub count: usize,
    /// Whether cycles were detected in prerequisites.
    pub has_cycles: bool,
}

/// Information about a prerequisite.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrerequisiteInfo {
    /// The prerequisite node.
    pub node: NodeSummary,
    /// Depth in the dependency tree (1 = direct prerequisite).
    pub depth: usize,
}

// ============================================================================
// Neighborhood
// ============================================================================

/// Response for neighborhood query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeighborhoodResponse {
    /// Center node.
    pub center: NodeSummary,
    /// Nodes in the neighborhood.
    pub nodes: Vec<NeighborInfo>,
    /// Edges in the neighborhood.
    pub edges: Vec<EdgeInfo>,
    /// Radius used for the query.
    pub radius: usize,
}

/// Information about a neighbor node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeighborInfo {
    /// The neighbor node.
    pub node: NodeSummary,
    /// Distance from center.
    pub distance: usize,
}

// ============================================================================
// Graph info
// ============================================================================

/// Response for graph info query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphInfoResponse {
    /// Total number of nodes.
    pub node_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
    /// Categories with counts.
    pub categories: Vec<CategoryCount>,
    /// Relationship types with counts.
    pub relationships: Vec<RelationshipCount>,
}

/// Category with count.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CategoryCount {
    /// The category name.
    pub category: String,
    /// Number of nodes in this category.
    pub count: usize,
}

/// Relationship type with count.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipCount {
    /// The relationship name.
    pub relationship: String,
    /// Number of edges with this relationship.
    pub count: usize,
}

// ============================================================================
// Node detail
// ============================================================================

/// Detailed information about a single node, including degree counts.
///
/// This is a richer version of `NodeSummary` that includes all node fields
/// plus computed graph metrics (in-degree / out-degree).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeDetail {
    /// Node ID.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Optional category.
    pub category: Option<String>,
    /// Optional source identifier.
    pub source_id: Option<String>,
    /// Whether this is a canonical node.
    pub is_canonical: bool,
    /// If not canonical, the ID of the canonical node.
    pub canonical_id: Option<String>,
    /// Node type as a string (e.g. "domain", "user_query").
    pub node_type: String,
    /// Domain-specific metadata.
    pub metadata: HashMap<String, serde_json::Value>,
    /// Number of incoming edges.
    pub in_degree: usize,
    /// Number of outgoing edges.
    pub out_degree: usize,
}

/// All edges connected to a node, separated by direction.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeEdgesResult {
    /// The node ID these edges connect to.
    pub node_id: String,
    /// Edges pointing into the node.
    pub incoming: Vec<EdgeInfo>,
    /// Edges pointing out of the node.
    pub outgoing: Vec<EdgeInfo>,
}

// ============================================================================
// Query functions
// ============================================================================

/// Get detailed information about a single node including degree counts.
///
/// Returns a `NodeDetail` with all node fields plus in-degree and out-degree
/// computed from the graph structure.
///
/// # Errors
///
/// Returns `Error::NotFound` if the node ID does not exist in the graph.
pub fn get_node_detail(graph: &GraphData, id: &str) -> Result<NodeDetail> {
    let node = graph
        .get_node(id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", id))?;

    let idx = graph
        .get_index(id)
        .ok_or_else(|| fabryk_core::Error::not_found("node index", id))?;

    let in_degree = graph
        .graph
        .edges_directed(idx, Direction::Incoming)
        .count();
    let out_degree = graph
        .graph
        .edges_directed(idx, Direction::Outgoing)
        .count();

    // Serialize NodeType to its snake_case string representation.
    let node_type = serde_json::to_value(&node.node_type)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_else(|| format!("{:?}", node.node_type));

    Ok(NodeDetail {
        id: node.id.clone(),
        title: node.title.clone(),
        category: node.category.clone(),
        source_id: node.source_id.clone(),
        is_canonical: node.is_canonical,
        canonical_id: node.canonical_id.clone(),
        node_type,
        metadata: node.metadata.clone(),
        in_degree,
        out_degree,
    })
}

/// Get all edges connected to a node, optionally filtered by direction.
///
/// The `direction` parameter accepts:
/// - `Some("incoming")` — only edges pointing into the node
/// - `Some("outgoing")` — only edges pointing out of the node
/// - `Some("both")` or `None` — all connected edges (default)
///
/// # Errors
///
/// Returns `Error::NotFound` if the node ID does not exist in the graph.
pub fn get_node_edges(
    graph: &GraphData,
    id: &str,
    direction: Option<&str>,
) -> Result<NodeEdgesResult> {
    let _node = graph
        .get_node(id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", id))?;

    let idx = graph
        .get_index(id)
        .ok_or_else(|| fabryk_core::Error::not_found("node index", id))?;

    let dir = direction.unwrap_or("both");

    let incoming = if dir == "incoming" || dir == "both" {
        graph
            .graph
            .edges_directed(idx, Direction::Incoming)
            .map(|e| EdgeInfo::from(e.weight()))
            .collect()
    } else {
        Vec::new()
    };

    let outgoing = if dir == "outgoing" || dir == "both" {
        graph
            .graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| EdgeInfo::from(e.weight()))
            .collect()
    } else {
        Vec::new()
    };

    Ok(NodeEdgesResult {
        node_id: id.to_string(),
        incoming,
        outgoing,
    })
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_node_summary_from_node() {
        let node = Node::new("test-id", "Test Title")
            .with_category("test-cat")
            .with_metadata("description", "A test concept");

        let summary = NodeSummary::from(&node);

        assert_eq!(summary.id, "test-id");
        assert_eq!(summary.title, "Test Title");
        assert_eq!(summary.category, Some("test-cat".to_string()));
        assert_eq!(summary.description, Some("A test concept".to_string()));
    }

    #[test]
    fn test_node_summary_from_node_no_description() {
        let node = Node::new("x", "X");
        let summary = NodeSummary::from(&node);

        assert_eq!(summary.id, "x");
        assert!(summary.description.is_none());
        assert!(summary.category.is_none());
    }

    #[test]
    fn test_edge_info_from_edge() {
        let edge = Edge::new("a", "b", Relationship::Prerequisite).with_weight(0.8);
        let info = EdgeInfo::from(&edge);

        assert_eq!(info.from, "a");
        assert_eq!(info.to, "b");
        assert_eq!(info.relationship, "prerequisite");
        assert_eq!(info.weight, 0.8);
    }

    #[test]
    fn test_edge_info_custom_relationship() {
        let edge = Edge::new("a", "b", Relationship::Custom("implies".to_string()));
        let info = EdgeInfo::from(&edge);

        assert_eq!(info.relationship, "implies");
    }

    #[test]
    fn test_node_summary_serialization() {
        let summary = NodeSummary {
            id: "test".to_string(),
            title: "Test".to_string(),
            category: Some("cat".to_string()),
            description: None,
        };

        let json = serde_json::to_string(&summary).unwrap();
        let parsed: NodeSummary = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "test");
        assert_eq!(parsed.category, Some("cat".to_string()));
    }

    #[test]
    fn test_edge_info_serialization() {
        let info = EdgeInfo {
            from: "a".to_string(),
            to: "b".to_string(),
            relationship: "prerequisite".to_string(),
            weight: 1.0,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: EdgeInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from, "a");
        assert_eq!(parsed.weight, 1.0);
    }

    #[test]
    fn test_related_concepts_response_serialization() {
        let response = RelatedConceptsResponse {
            source: NodeSummary {
                id: "src".to_string(),
                title: "Source".to_string(),
                category: None,
                description: None,
            },
            related: vec![RelatedGroup {
                relationship: "prerequisite".to_string(),
                concepts: vec![NodeSummary {
                    id: "dep".to_string(),
                    title: "Dependency".to_string(),
                    category: None,
                    description: None,
                }],
            }],
            total_count: 1,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: RelatedConceptsResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.total_count, 1);
        assert_eq!(parsed.related.len(), 1);
        assert_eq!(parsed.related[0].concepts.len(), 1);
    }

    #[test]
    fn test_path_response_serialization() {
        let response = PathResponse {
            from: NodeSummary {
                id: "a".to_string(),
                title: "A".to_string(),
                category: None,
                description: None,
            },
            to: NodeSummary {
                id: "c".to_string(),
                title: "C".to_string(),
                category: None,
                description: None,
            },
            path: vec![
                PathStep {
                    node: NodeSummary {
                        id: "a".to_string(),
                        title: "A".to_string(),
                        category: None,
                        description: None,
                    },
                    relationship_to_next: Some("prerequisite".to_string()),
                },
                PathStep {
                    node: NodeSummary {
                        id: "c".to_string(),
                        title: "C".to_string(),
                        category: None,
                        description: None,
                    },
                    relationship_to_next: None,
                },
            ],
            found: true,
            length: 2,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: PathResponse = serde_json::from_str(&json).unwrap();

        assert!(parsed.found);
        assert_eq!(parsed.path.len(), 2);
    }

    #[test]
    fn test_prerequisites_response_serialization() {
        let response = PrerequisitesResponse {
            target: NodeSummary {
                id: "target".to_string(),
                title: "Target".to_string(),
                category: None,
                description: None,
            },
            prerequisites: vec![PrerequisiteInfo {
                node: NodeSummary {
                    id: "prereq".to_string(),
                    title: "Prereq".to_string(),
                    category: None,
                    description: None,
                },
                depth: 1,
            }],
            count: 1,
            has_cycles: false,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: PrerequisitesResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.count, 1);
        assert!(!parsed.has_cycles);
    }

    #[test]
    fn test_neighborhood_response_serialization() {
        let response = NeighborhoodResponse {
            center: NodeSummary {
                id: "center".to_string(),
                title: "Center".to_string(),
                category: None,
                description: None,
            },
            nodes: vec![NeighborInfo {
                node: NodeSummary {
                    id: "neighbor".to_string(),
                    title: "Neighbor".to_string(),
                    category: None,
                    description: None,
                },
                distance: 1,
            }],
            edges: vec![EdgeInfo {
                from: "center".to_string(),
                to: "neighbor".to_string(),
                relationship: "relates_to".to_string(),
                weight: 0.7,
            }],
            radius: 2,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: NeighborhoodResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.radius, 2);
        assert_eq!(parsed.nodes.len(), 1);
        assert_eq!(parsed.edges.len(), 1);
    }

    #[test]
    fn test_graph_info_response_serialization() {
        let response = GraphInfoResponse {
            node_count: 42,
            edge_count: 100,
            categories: vec![CategoryCount {
                category: "harmony".to_string(),
                count: 15,
            }],
            relationships: vec![RelationshipCount {
                relationship: "prerequisite".to_string(),
                count: 50,
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: GraphInfoResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_count, 42);
        assert_eq!(parsed.edge_count, 100);
        assert_eq!(parsed.categories.len(), 1);
        assert_eq!(parsed.relationships.len(), 1);
    }

    // ========================================================================
    // Helper: build a small test graph
    // ========================================================================

    fn make_query_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        graph.add_node(
            Node::new("a", "Node A")
                .with_category("cat1")
                .with_metadata("description", "First node"),
        );
        graph.add_node(Node::new("b", "Node B").with_category("cat2"));
        graph.add_node(
            Node::new("c", "Node C")
                .with_category("cat1")
                .with_source("src-1")
                .as_variant_of("a"),
        );

        // a -> b (prerequisite), b -> c (leads_to), a -> c (relates_to)
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::LeadsTo))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "c", Relationship::RelatesTo))
            .unwrap();

        graph
    }

    // ========================================================================
    // get_node_detail tests
    // ========================================================================

    #[test]
    fn test_get_node_detail_basic() {
        let graph = make_query_test_graph();
        let detail = get_node_detail(&graph, "a").unwrap();

        assert_eq!(detail.id, "a");
        assert_eq!(detail.title, "Node A");
        assert_eq!(detail.category, Some("cat1".to_string()));
        assert!(detail.is_canonical);
        assert!(detail.canonical_id.is_none());
        assert!(detail.source_id.is_none());
        assert_eq!(detail.node_type, "domain");
        assert!(detail.metadata.contains_key("description"));
    }

    #[test]
    fn test_get_node_detail_degrees() {
        let graph = make_query_test_graph();

        // a: out=2 (a->b, a->c), in=0
        let detail_a = get_node_detail(&graph, "a").unwrap();
        assert_eq!(detail_a.out_degree, 2);
        assert_eq!(detail_a.in_degree, 0);

        // b: out=1 (b->c), in=1 (a->b)
        let detail_b = get_node_detail(&graph, "b").unwrap();
        assert_eq!(detail_b.out_degree, 1);
        assert_eq!(detail_b.in_degree, 1);

        // c: out=0, in=2 (b->c, a->c)
        let detail_c = get_node_detail(&graph, "c").unwrap();
        assert_eq!(detail_c.out_degree, 0);
        assert_eq!(detail_c.in_degree, 2);
    }

    #[test]
    fn test_get_node_detail_variant_node() {
        let graph = make_query_test_graph();
        let detail = get_node_detail(&graph, "c").unwrap();

        assert!(!detail.is_canonical);
        assert_eq!(detail.canonical_id, Some("a".to_string()));
        assert_eq!(detail.source_id, Some("src-1".to_string()));
    }

    #[test]
    fn test_get_node_detail_not_found() {
        let graph = make_query_test_graph();
        let result = get_node_detail(&graph, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_node_detail_serialization() {
        let graph = make_query_test_graph();
        let detail = get_node_detail(&graph, "a").unwrap();

        let json = serde_json::to_string(&detail).unwrap();
        let parsed: NodeDetail = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "a");
        assert_eq!(parsed.out_degree, 2);
        assert_eq!(parsed.in_degree, 0);
        assert_eq!(parsed.node_type, "domain");
    }

    #[test]
    fn test_get_node_detail_user_query_type() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("q1", "Query").with_node_type(NodeType::UserQuery));

        let detail = get_node_detail(&graph, "q1").unwrap();
        assert_eq!(detail.node_type, "user_query");
    }

    #[test]
    fn test_get_node_detail_custom_type() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("x", "X").with_node_type(NodeType::Custom("special".to_string())));

        let detail = get_node_detail(&graph, "x").unwrap();
        // Custom variant serializes differently — just check it's non-empty
        assert!(!detail.node_type.is_empty());
    }

    // ========================================================================
    // get_node_edges tests
    // ========================================================================

    #[test]
    fn test_get_node_edges_both() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "b", None).unwrap();

        assert_eq!(result.node_id, "b");
        // b has 1 incoming (a->b) and 1 outgoing (b->c)
        assert_eq!(result.incoming.len(), 1);
        assert_eq!(result.outgoing.len(), 1);
        assert_eq!(result.incoming[0].from, "a");
        assert_eq!(result.incoming[0].to, "b");
        assert_eq!(result.outgoing[0].from, "b");
        assert_eq!(result.outgoing[0].to, "c");
    }

    #[test]
    fn test_get_node_edges_incoming_only() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "b", Some("incoming")).unwrap();

        assert_eq!(result.incoming.len(), 1);
        assert!(result.outgoing.is_empty());
    }

    #[test]
    fn test_get_node_edges_outgoing_only() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "b", Some("outgoing")).unwrap();

        assert!(result.incoming.is_empty());
        assert_eq!(result.outgoing.len(), 1);
    }

    #[test]
    fn test_get_node_edges_explicit_both() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "b", Some("both")).unwrap();

        assert_eq!(result.incoming.len(), 1);
        assert_eq!(result.outgoing.len(), 1);
    }

    #[test]
    fn test_get_node_edges_no_edges() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("solo", "Solo Node"));

        let result = get_node_edges(&graph, "solo", None).unwrap();
        assert!(result.incoming.is_empty());
        assert!(result.outgoing.is_empty());
    }

    #[test]
    fn test_get_node_edges_not_found() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "nonexistent", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_node_edges_source_node() {
        let graph = make_query_test_graph();
        // a: out=2 (a->b, a->c), in=0
        let result = get_node_edges(&graph, "a", None).unwrap();

        assert!(result.incoming.is_empty());
        assert_eq!(result.outgoing.len(), 2);
    }

    #[test]
    fn test_get_node_edges_sink_node() {
        let graph = make_query_test_graph();
        // c: out=0, in=2 (b->c, a->c)
        let result = get_node_edges(&graph, "c", None).unwrap();

        assert_eq!(result.incoming.len(), 2);
        assert!(result.outgoing.is_empty());
    }

    #[test]
    fn test_get_node_edges_serialization() {
        let graph = make_query_test_graph();
        let result = get_node_edges(&graph, "b", None).unwrap();

        let json = serde_json::to_string(&result).unwrap();
        let parsed: NodeEdgesResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_id, "b");
        assert_eq!(parsed.incoming.len(), 1);
        assert_eq!(parsed.outgoing.len(), 1);
    }
}
