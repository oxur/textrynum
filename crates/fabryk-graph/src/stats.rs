//! Graph statistics and analysis.
//!
//! Provides functions for analysing graph structure and composition,
//! including degree distribution, category breakdowns, and top-node
//! rankings.

use crate::GraphData;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Types
// ============================================================================

/// Comprehensive statistics about a graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphStats {
    /// Total number of nodes.
    pub node_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
    /// Number of canonical nodes.
    pub canonical_count: usize,
    /// Number of variant nodes.
    pub variant_count: usize,
    /// Nodes per category.
    pub category_distribution: HashMap<String, usize>,
    /// Edges per relationship type.
    pub relationship_distribution: HashMap<String, usize>,
    /// Nodes without any edges (orphans).
    pub orphan_count: usize,
    /// Average edges per node (in + out).
    pub avg_degree: f32,
    /// Maximum in-degree (most dependencies).
    pub max_in_degree: usize,
    /// Maximum out-degree (most dependents).
    pub max_out_degree: usize,
    /// Node with highest in-degree.
    pub most_depended_on: Option<String>,
    /// Node with highest out-degree.
    pub most_dependencies: Option<String>,
    /// Nodes per node type (e.g., "domain" -> 150, "source" -> 12).
    #[serde(default)]
    pub type_counts: HashMap<String, usize>,
}

/// Direction for degree calculation.
#[derive(Clone, Copy, Debug)]
pub enum DegreeDirection {
    /// Incoming edges only.
    In,
    /// Outgoing edges only.
    Out,
    /// Both directions.
    Both,
}

// ============================================================================
// Functions
// ============================================================================

/// Compute comprehensive statistics for a graph.
pub fn compute_stats(graph: &GraphData) -> GraphStats {
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();

    // Count canonical vs variant, and node types
    let mut canonical_count = 0;
    let mut variant_count = 0;
    let mut type_counts: HashMap<String, usize> = HashMap::new();
    for node in graph.iter_nodes() {
        if node.is_canonical {
            canonical_count += 1;
        } else {
            variant_count += 1;
        }
        *type_counts.entry(node.node_type.to_string()).or_insert(0) += 1;
    }

    // Category distribution
    let mut category_distribution: HashMap<String, usize> = HashMap::new();
    for node in graph.iter_nodes() {
        let cat = node
            .category
            .clone()
            .unwrap_or_else(|| "uncategorized".to_string());
        *category_distribution.entry(cat).or_insert(0) += 1;
    }

    // Relationship distribution
    let mut relationship_distribution: HashMap<String, usize> = HashMap::new();
    for edge in graph.iter_edges() {
        let rel = edge.relationship.name().to_string();
        *relationship_distribution.entry(rel).or_insert(0) += 1;
    }

    // Degree analysis
    let mut in_degrees: HashMap<String, usize> = HashMap::new();
    let mut out_degrees: HashMap<String, usize> = HashMap::new();

    for edge in graph.iter_edges() {
        *in_degrees.entry(edge.to.clone()).or_insert(0) += 1;
        *out_degrees.entry(edge.from.clone()).or_insert(0) += 1;
    }

    // Initialise all nodes with 0 degree
    for node_id in graph.node_ids() {
        in_degrees.entry(node_id.to_string()).or_insert(0);
        out_degrees.entry(node_id.to_string()).or_insert(0);
    }

    let orphan_count = graph
        .node_ids()
        .filter(|id| {
            in_degrees.get(*id).copied().unwrap_or(0) == 0
                && out_degrees.get(*id).copied().unwrap_or(0) == 0
        })
        .count();

    let total_degree: usize =
        in_degrees.values().sum::<usize>() + out_degrees.values().sum::<usize>();
    let avg_degree = if node_count > 0 {
        total_degree as f32 / node_count as f32
    } else {
        0.0
    };

    let (most_depended_on, max_in_degree) = in_degrees
        .iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, v)| (Some(k.clone()), *v))
        .unwrap_or((None, 0));

    let (most_dependencies, max_out_degree) = out_degrees
        .iter()
        .max_by_key(|(_, v)| *v)
        .map(|(k, v)| (Some(k.clone()), *v))
        .unwrap_or((None, 0));

    GraphStats {
        node_count,
        edge_count,
        canonical_count,
        variant_count,
        category_distribution,
        relationship_distribution,
        orphan_count,
        avg_degree,
        max_in_degree,
        max_out_degree,
        most_depended_on,
        most_dependencies,
        type_counts,
    }
}

/// Get a quick summary of graph size.
pub fn quick_summary(graph: &GraphData) -> String {
    format!("{} nodes, {} edges", graph.node_count(), graph.edge_count())
}

/// Get top N nodes by degree.
pub fn top_nodes_by_degree(
    graph: &GraphData,
    limit: usize,
    direction: DegreeDirection,
) -> Vec<(String, usize)> {
    let mut scores: Vec<(String, usize)> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();
            let degree = match direction {
                DegreeDirection::In => graph.graph.edges_directed(idx, Direction::Incoming).count(),
                DegreeDirection::Out => {
                    graph.graph.edges_directed(idx, Direction::Outgoing).count()
                }
                DegreeDirection::Both => {
                    graph.graph.edges_directed(idx, Direction::Incoming).count()
                        + graph.graph.edges_directed(idx, Direction::Outgoing).count()
                }
            };
            (node.id.clone(), degree)
        })
        .collect();

    scores.sort_by(|a, b| b.1.cmp(&a.1));
    scores.truncate(limit);
    scores
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn create_test_graph() -> GraphData {
        let mut graph = GraphData::new();

        graph.add_node(Node::new("a", "A").with_category("basics"));
        graph.add_node(Node::new("b", "B").with_category("basics"));
        graph.add_node(Node::new("c", "C").with_category("advanced"));
        graph.add_node(Node::new("d", "D")); // no category
        graph.add_node(Node::new("orphan", "Orphan").with_category("basics"));

        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "c", Relationship::RelatesTo))
            .unwrap();
        graph
            .add_edge(Edge::new("c", "d", Relationship::LeadsTo))
            .unwrap();

        graph
    }

    #[test]
    fn test_compute_stats_basic_counts() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        assert_eq!(stats.node_count, 5);
        assert_eq!(stats.edge_count, 4);
    }

    #[test]
    fn test_compute_stats_canonical_vs_variant() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A")); // canonical
        graph.add_node(Node::new("b", "B").as_variant_of("a")); // variant

        let stats = compute_stats(&graph);

        assert_eq!(stats.canonical_count, 1);
        assert_eq!(stats.variant_count, 1);
    }

    #[test]
    fn test_compute_stats_category_distribution() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        assert_eq!(stats.category_distribution["basics"], 3); // a, b, orphan
        assert_eq!(stats.category_distribution["advanced"], 1); // c
        assert_eq!(stats.category_distribution["uncategorized"], 1); // d
    }

    #[test]
    fn test_compute_stats_relationship_distribution() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        assert_eq!(stats.relationship_distribution["prerequisite"], 2);
        assert_eq!(stats.relationship_distribution["relates_to"], 1);
        assert_eq!(stats.relationship_distribution["leads_to"], 1);
    }

    #[test]
    fn test_compute_stats_orphans() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        assert_eq!(stats.orphan_count, 1); // "orphan" node
    }

    #[test]
    fn test_compute_stats_avg_degree() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        // 4 edges: each contributes 1 in-degree and 1 out-degree = 8 total degree
        // 5 nodes: avg = 8/5 = 1.6
        assert!((stats.avg_degree - 1.6).abs() < 0.01);
    }

    #[test]
    fn test_compute_stats_max_degrees() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        // a has out-degree 2 (a->b, a->c)
        assert_eq!(stats.max_out_degree, 2);
        // c has in-degree 2 (b->c, a->c)
        assert_eq!(stats.max_in_degree, 2);
    }

    #[test]
    fn test_compute_stats_empty_graph() {
        let graph = GraphData::new();
        let stats = compute_stats(&graph);

        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
        assert_eq!(stats.orphan_count, 0);
        assert_eq!(stats.avg_degree, 0.0);
        assert!(stats.most_depended_on.is_none());
        assert!(stats.most_dependencies.is_none());
    }

    #[test]
    fn test_quick_summary() {
        let graph = create_test_graph();
        let summary = quick_summary(&graph);

        assert_eq!(summary, "5 nodes, 4 edges");
    }

    #[test]
    fn test_quick_summary_empty() {
        let graph = GraphData::new();
        let summary = quick_summary(&graph);

        assert_eq!(summary, "0 nodes, 0 edges");
    }

    #[test]
    fn test_top_nodes_by_degree_in() {
        let graph = create_test_graph();
        let top = top_nodes_by_degree(&graph, 2, DegreeDirection::In);

        assert_eq!(top.len(), 2);
        // c has in-degree 2 (highest)
        assert_eq!(top[0].0, "c");
        assert_eq!(top[0].1, 2);
    }

    #[test]
    fn test_top_nodes_by_degree_out() {
        let graph = create_test_graph();
        let top = top_nodes_by_degree(&graph, 2, DegreeDirection::Out);

        assert_eq!(top.len(), 2);
        // a has out-degree 2 (highest)
        assert_eq!(top[0].0, "a");
        assert_eq!(top[0].1, 2);
    }

    #[test]
    fn test_top_nodes_by_degree_both() {
        let graph = create_test_graph();
        let top = top_nodes_by_degree(&graph, 3, DegreeDirection::Both);

        assert_eq!(top.len(), 3);
        // All non-orphan nodes should have degree > 0
        assert!(top[0].1 > 0);
    }

    #[test]
    fn test_top_nodes_by_degree_empty_graph() {
        let graph = GraphData::new();
        let top = top_nodes_by_degree(&graph, 5, DegreeDirection::Both);

        assert!(top.is_empty());
    }

    #[test]
    fn test_top_nodes_limit() {
        let graph = create_test_graph();
        let top = top_nodes_by_degree(&graph, 1, DegreeDirection::Both);

        assert_eq!(top.len(), 1);
    }

    #[test]
    fn test_graph_stats_serialization() {
        let graph = create_test_graph();
        let stats = compute_stats(&graph);

        let json = serde_json::to_string(&stats).unwrap();
        let parsed: GraphStats = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.node_count, stats.node_count);
        assert_eq!(parsed.edge_count, stats.edge_count);
        assert_eq!(parsed.orphan_count, stats.orphan_count);
    }
}
