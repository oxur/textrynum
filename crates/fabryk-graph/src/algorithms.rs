//! Graph algorithms for knowledge graph analysis.
//!
//! Provides algorithms for:
//! - Neighborhood exploration (N-hop BFS with configurable max depth)
//! - Pathfinding (shortest path between concepts)
//! - Dependency analysis (prerequisites, topological ordering)
//! - Centrality analysis (identifying important nodes)
//! - Bridge detection (nodes connecting different clusters)
//!
//! All algorithms are generic and operate on `GraphData`.

use crate::{Edge, GraphData, Node, Relationship};
use fabryk_core::Result;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

/// Maximum allowed BFS depth to prevent runaway traversals.
const MAX_BFS_DEPTH: usize = 10;

// ============================================================================
// Result types
// ============================================================================

/// Result of a neighborhood query.
#[derive(Clone, Debug)]
pub struct NeighborhoodResult {
    /// The central node.
    pub center: Node,
    /// Nodes within the specified radius.
    pub nodes: Vec<Node>,
    /// Edges connecting the neighborhood nodes.
    pub edges: Vec<Edge>,
    /// Distance from center to each node (by node ID).
    pub distances: HashMap<String, usize>,
}

/// Result of a shortest path query.
#[derive(Clone, Debug)]
pub struct PathResult {
    /// Nodes along the path, in order.
    pub path: Vec<Node>,
    /// Edges along the path.
    pub edges: Vec<Edge>,
    /// Total path weight.
    pub total_weight: f32,
    /// Whether a path was found.
    pub found: bool,
}

impl PathResult {
    /// Creates an empty result indicating no path found.
    pub fn not_found() -> Self {
        Self {
            path: Vec::new(),
            edges: Vec::new(),
            total_weight: 0.0,
            found: false,
        }
    }
}

/// Result of prerequisites analysis.
#[derive(Clone, Debug)]
pub struct PrerequisitesResult {
    /// Prerequisites in topological order (learn first → learn last).
    pub ordered: Vec<Node>,
    /// The target node.
    pub target: Node,
    /// Whether cycles were detected (if true, ordering is approximate).
    pub has_cycles: bool,
}

/// Centrality scores for a node.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CentralityScore {
    /// Node ID.
    pub node_id: String,
    /// Degree centrality (normalized: 0.0 to 1.0).
    pub degree: f32,
    /// In-degree centrality (how many nodes point to this).
    pub in_degree: f32,
    /// Out-degree centrality (how many nodes this points to).
    pub out_degree: f32,
}

// ============================================================================
// Algorithms
// ============================================================================

/// Get the N-hop neighborhood around a node.
///
/// Performs a breadth-first search from the center node, collecting all
/// nodes and edges within the specified radius. Depth is capped at
/// `MAX_BFS_DEPTH` (10) to prevent runaway traversals.
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `center_id` - ID of the center node
/// * `radius` - Maximum distance from center (hops), capped at 10
/// * `relationship_filter` - Optional filter for edge types to follow
pub fn neighborhood(
    graph: &GraphData,
    center_id: &str,
    radius: usize,
    relationship_filter: Option<&[Relationship]>,
) -> Result<NeighborhoodResult> {
    let center_node = graph
        .get_node(center_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", center_id))?
        .clone();

    let center_idx = graph
        .get_index(center_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node index", center_id))?;

    let radius = radius.min(MAX_BFS_DEPTH);

    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut distances: HashMap<String, usize> = HashMap::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    let mut result_nodes: Vec<Node> = Vec::new();
    let mut result_edges: Vec<Edge> = Vec::new();

    visited.insert(center_idx);
    distances.insert(center_id.to_string(), 0);
    queue.push_back((center_idx, 0));

    while let Some((current_idx, current_dist)) = queue.pop_front() {
        if current_dist >= radius {
            continue;
        }

        // Explore outgoing edges
        for edge_ref in graph.graph.edges_directed(current_idx, Direction::Outgoing) {
            let edge_weight = edge_ref.weight();

            if let Some(filter) = relationship_filter {
                if !filter.contains(&edge_weight.relationship) {
                    continue;
                }
            }

            let neighbor_idx = edge_ref.target();
            if visited.insert(neighbor_idx) {
                let neighbor = &graph.graph[neighbor_idx];
                distances.insert(neighbor.id.clone(), current_dist + 1);
                result_nodes.push(neighbor.clone());
                result_edges.push(edge_weight.clone());
                queue.push_back((neighbor_idx, current_dist + 1));
            }
        }

        // Explore incoming edges
        for edge_ref in graph.graph.edges_directed(current_idx, Direction::Incoming) {
            let edge_weight = edge_ref.weight();

            if let Some(filter) = relationship_filter {
                if !filter.contains(&edge_weight.relationship) {
                    continue;
                }
            }

            let neighbor_idx = edge_ref.source();
            if visited.insert(neighbor_idx) {
                let neighbor = &graph.graph[neighbor_idx];
                distances.insert(neighbor.id.clone(), current_dist + 1);
                result_nodes.push(neighbor.clone());
                result_edges.push(edge_weight.clone());
                queue.push_back((neighbor_idx, current_dist + 1));
            }
        }
    }

    Ok(NeighborhoodResult {
        center: center_node,
        nodes: result_nodes,
        edges: result_edges,
        distances,
    })
}

/// Find the shortest path between two nodes using A* search.
///
/// Uses petgraph's A* algorithm with uniform cost.
pub fn shortest_path(graph: &GraphData, from_id: &str, to_id: &str) -> Result<PathResult> {
    let from_idx = match graph.get_index(from_id) {
        Some(idx) => idx,
        None => return Ok(PathResult::not_found()),
    };

    let to_idx = match graph.get_index(to_id) {
        Some(idx) => idx,
        None => return Ok(PathResult::not_found()),
    };

    if from_idx == to_idx {
        let node = graph.graph[from_idx].clone();
        return Ok(PathResult {
            path: vec![node],
            edges: Vec::new(),
            total_weight: 0.0,
            found: true,
        });
    }

    let result = petgraph::algo::astar(
        &graph.graph,
        from_idx,
        |n| n == to_idx,
        |e| (1.0 / e.weight().weight.max(0.01)) as u32,
        |_| 0,
    );

    match result {
        Some((_cost, path_indices)) => {
            let path: Vec<Node> = path_indices
                .iter()
                .map(|&idx| graph.graph[idx].clone())
                .collect();

            let mut edges = Vec::new();
            for window in path_indices.windows(2) {
                if let [from, to] = window {
                    if let Some(edge_idx) = graph.graph.find_edge(*from, *to) {
                        edges.push(graph.graph[edge_idx].clone());
                    }
                }
            }

            let total_weight = edges.iter().map(|e| e.weight).sum();

            Ok(PathResult {
                path,
                edges,
                total_weight,
                found: true,
            })
        }
        None => Ok(PathResult::not_found()),
    }
}

/// Get prerequisites for a concept in topological order.
///
/// Follows `Prerequisite` edges backwards to find all dependencies,
/// then returns them in learning order (fundamentals first).
pub fn prerequisites_sorted(graph: &GraphData, target_id: &str) -> Result<PrerequisitesResult> {
    let target_node = graph
        .get_node(target_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", target_id))?
        .clone();

    let target_idx = graph.get_index(target_id).unwrap();

    // Collect all prerequisites (recursive BFS backwards through Prerequisite edges)
    let mut prereq_indices: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<NodeIndex> = VecDeque::new();
    queue.push_back(target_idx);

    while let Some(current) = queue.pop_front() {
        for edge_ref in graph.graph.edges_directed(current, Direction::Incoming) {
            if edge_ref.weight().relationship == Relationship::Prerequisite {
                let source = edge_ref.source();
                if prereq_indices.insert(source) {
                    queue.push_back(source);
                }
            }
        }
    }

    if prereq_indices.is_empty() {
        return Ok(PrerequisitesResult {
            ordered: Vec::new(),
            target: target_node,
            has_cycles: false,
        });
    }

    // Try topological sort of the full graph
    let sorted = toposort(&graph.graph, None);

    let (ordered, has_cycles) = match sorted {
        Ok(all_sorted) => {
            // Filter to just our prerequisites, maintaining topo order
            let ordered: Vec<Node> = all_sorted
                .into_iter()
                .filter(|idx| prereq_indices.contains(idx))
                .map(|idx| graph.graph[idx].clone())
                .collect();
            (ordered, false)
        }
        Err(_) => {
            // Cycle detected - return in arbitrary order
            let ordered: Vec<Node> = prereq_indices
                .iter()
                .map(|idx| graph.graph[*idx].clone())
                .collect();
            (ordered, true)
        }
    };

    Ok(PrerequisitesResult {
        ordered,
        target: target_node,
        has_cycles,
    })
}

/// Calculate centrality scores for all nodes.
///
/// Computes degree-based centrality metrics for each node.
/// Returns scores sorted by degree (descending).
pub fn calculate_centrality(graph: &GraphData) -> Vec<CentralityScore> {
    let n = graph.node_count() as f32;
    if n < 2.0 {
        return Vec::new();
    }

    let mut scores: Vec<CentralityScore> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();

            let in_deg = graph.graph.edges_directed(idx, Direction::Incoming).count() as f32;
            let out_deg = graph.graph.edges_directed(idx, Direction::Outgoing).count() as f32;
            let degree = in_deg + out_deg;

            CentralityScore {
                node_id: node.id.clone(),
                degree: degree / (2.0 * (n - 1.0)),
                in_degree: in_deg / (n - 1.0),
                out_degree: out_deg / (n - 1.0),
            }
        })
        .collect();

    scores.sort_by(|a, b| {
        b.degree
            .partial_cmp(&a.degree)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scores
}

/// Find bridge concepts connecting different clusters.
///
/// Identifies nodes that connect otherwise-distant parts of the graph,
/// using connectivity × category diversity as a heuristic score.
pub fn find_bridges(graph: &GraphData, limit: usize) -> Vec<Node> {
    let mut bridge_scores: Vec<(String, f32)> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();

            let in_deg = graph.graph.edges_directed(idx, Direction::Incoming).count() as f32;
            let out_deg = graph.graph.edges_directed(idx, Direction::Outgoing).count() as f32;

            // Collect unique categories of neighbors
            let mut neighbor_categories: HashSet<String> = HashSet::new();
            for edge_ref in graph.graph.edges(idx) {
                let neighbor = &graph.graph[edge_ref.target()];
                if let Some(ref cat) = neighbor.category {
                    neighbor_categories.insert(cat.clone());
                }
            }

            // Bridge score: connectivity × category diversity
            let diversity = neighbor_categories.len() as f32;
            let score = (in_deg.min(out_deg) + 1.0) * (diversity + 1.0);

            (node.id.clone(), score)
        })
        .collect();

    bridge_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    bridge_scores
        .into_iter()
        .take(limit)
        .filter_map(|(id, _)| graph.get_node(&id).cloned())
        .collect()
}

/// Get nodes related to a given node by specific relationship types.
pub fn get_related(
    graph: &GraphData,
    node_id: &str,
    relationships: &[Relationship],
    direction: Direction,
) -> Result<Vec<(Node, Relationship)>> {
    let idx = graph
        .get_index(node_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", node_id))?;

    let mut results = Vec::new();

    let edges: Box<dyn Iterator<Item = _>> = match direction {
        Direction::Outgoing => Box::new(graph.graph.edges_directed(idx, Direction::Outgoing)),
        Direction::Incoming => Box::new(graph.graph.edges_directed(idx, Direction::Incoming)),
    };

    for edge_ref in edges {
        let edge = edge_ref.weight();
        if relationships.contains(&edge.relationship) {
            let neighbor_idx = match direction {
                Direction::Outgoing => edge_ref.target(),
                Direction::Incoming => edge_ref.source(),
            };
            let neighbor = graph.graph[neighbor_idx].clone();
            results.push((neighbor, edge.relationship.clone()));
        }
    }

    Ok(results)
}

// ============================================================================
// Typed convenience methods on GraphData (adapted from Taproot)
// ============================================================================

impl GraphData {
    /// Get direct prerequisites of a node (incoming Prerequisite edges).
    pub fn prerequisites(&self, node_id: &str) -> Result<Vec<Node>> {
        let results = get_related(
            self,
            node_id,
            &[Relationship::Prerequisite],
            Direction::Outgoing,
        )?;
        Ok(results.into_iter().map(|(node, _)| node).collect())
    }

    /// Get nodes that depend on this node (outgoing Prerequisite edges pointing here).
    pub fn dependents(&self, node_id: &str) -> Result<Vec<Node>> {
        let results = get_related(
            self,
            node_id,
            &[Relationship::Prerequisite],
            Direction::Incoming,
        )?;
        Ok(results.into_iter().map(|(node, _)| node).collect())
    }

    /// Get nodes related by a specific relationship type (outgoing direction).
    pub fn related_by(&self, node_id: &str, relationship: &Relationship) -> Result<Vec<Node>> {
        let results = get_related(
            self,
            node_id,
            std::slice::from_ref(relationship),
            Direction::Outgoing,
        )?;
        Ok(results.into_iter().map(|(node, _)| node).collect())
    }
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

        // Create a simple graph: A -> B -> C, A -> D, B -> D
        let nodes = vec![
            Node::new("a", "Node A").with_category("cat1"),
            Node::new("b", "Node B").with_category("cat1"),
            Node::new("c", "Node C").with_category("cat2"),
            Node::new("d", "Node D").with_category("cat2"),
        ];

        for node in &nodes {
            graph.add_node(node.clone());
        }

        let edges = vec![
            Edge::new("a", "b", Relationship::Prerequisite),
            Edge::new("b", "c", Relationship::Prerequisite),
            Edge::new("a", "d", Relationship::RelatesTo),
            Edge::new("b", "d", Relationship::LeadsTo),
        ];

        for edge in edges {
            graph.add_edge(edge).unwrap();
        }

        graph
    }

    // ------------------------------------------------------------------------
    // Neighborhood tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_neighborhood_basic() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "a", 1, None).unwrap();

        assert_eq!(result.center.id, "a");
        assert_eq!(result.nodes.len(), 2); // b and d
        assert!(result.nodes.iter().any(|n| n.id == "b"));
        assert!(result.nodes.iter().any(|n| n.id == "d"));
    }

    #[test]
    fn test_neighborhood_radius_2() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "a", 2, None).unwrap();

        assert!(result.nodes.iter().any(|n| n.id == "c"));
        assert_eq!(result.distances["c"], 2);
    }

    #[test]
    fn test_neighborhood_with_filter() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "a", 2, Some(&[Relationship::Prerequisite])).unwrap();

        assert!(result.nodes.iter().any(|n| n.id == "b"));
        assert!(result.nodes.iter().any(|n| n.id == "c"));
        // d should NOT be included (connected via RelatesTo)
        assert!(!result.nodes.iter().any(|n| n.id == "d"));
    }

    #[test]
    fn test_neighborhood_not_found() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "nonexistent", 1, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_neighborhood_empty_graph() {
        let graph = GraphData::new();
        let result = neighborhood(&graph, "x", 1, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_neighborhood_radius_zero() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "a", 0, None).unwrap();

        assert_eq!(result.center.id, "a");
        assert!(result.nodes.is_empty());
    }

    #[test]
    fn test_neighborhood_depth_capped() {
        let graph = create_test_graph();
        // Request radius 100, should be capped to MAX_BFS_DEPTH
        let result = neighborhood(&graph, "a", 100, None).unwrap();
        // Should find all reachable nodes without infinite loop
        assert!(!result.nodes.is_empty());
    }

    // ------------------------------------------------------------------------
    // Shortest path tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_shortest_path_found() {
        let graph = create_test_graph();
        let result = shortest_path(&graph, "a", "c").unwrap();

        assert!(result.found);
        assert!(!result.path.is_empty());
        assert_eq!(result.path.first().unwrap().id, "a");
        assert_eq!(result.path.last().unwrap().id, "c");
    }

    #[test]
    fn test_shortest_path_not_found() {
        let graph = create_test_graph();
        let result = shortest_path(&graph, "c", "a").unwrap();
        assert!(!result.found);
    }

    #[test]
    fn test_shortest_path_same_node() {
        let graph = create_test_graph();
        let result = shortest_path(&graph, "a", "a").unwrap();
        assert!(result.found);
        assert_eq!(result.path.len(), 1);
        assert_eq!(result.total_weight, 0.0);
    }

    #[test]
    fn test_shortest_path_missing_from() {
        let graph = create_test_graph();
        let result = shortest_path(&graph, "missing", "a").unwrap();
        assert!(!result.found);
    }

    #[test]
    fn test_shortest_path_missing_to() {
        let graph = create_test_graph();
        let result = shortest_path(&graph, "a", "missing").unwrap();
        assert!(!result.found);
    }

    // ------------------------------------------------------------------------
    // Prerequisites tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_prerequisites_sorted() {
        let graph = create_test_graph();
        let result = prerequisites_sorted(&graph, "c").unwrap();

        assert_eq!(result.target.id, "c");
        assert!(!result.has_cycles);
        assert!(result.ordered.iter().any(|n| n.id == "a"));
        assert!(result.ordered.iter().any(|n| n.id == "b"));
        // a should come before b (a is prereq of b, b is prereq of c)
        let a_pos = result.ordered.iter().position(|n| n.id == "a").unwrap();
        let b_pos = result.ordered.iter().position(|n| n.id == "b").unwrap();
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_prerequisites_no_deps() {
        let graph = create_test_graph();
        let result = prerequisites_sorted(&graph, "a").unwrap();

        assert_eq!(result.target.id, "a");
        assert!(result.ordered.is_empty());
    }

    #[test]
    fn test_prerequisites_not_found() {
        let graph = create_test_graph();
        let result = prerequisites_sorted(&graph, "nonexistent");
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Centrality tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_calculate_centrality() {
        let graph = create_test_graph();
        let scores = calculate_centrality(&graph);

        assert_eq!(scores.len(), 4);
        for score in &scores {
            assert!(score.degree >= 0.0 && score.degree <= 1.0);
            assert!(score.in_degree >= 0.0 && score.in_degree <= 1.0);
            assert!(score.out_degree >= 0.0 && score.out_degree <= 1.0);
        }
    }

    #[test]
    fn test_calculate_centrality_empty() {
        let graph = GraphData::new();
        let scores = calculate_centrality(&graph);
        assert!(scores.is_empty());
    }

    #[test]
    fn test_calculate_centrality_single_node() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        let scores = calculate_centrality(&graph);
        // Less than 2 nodes => empty
        assert!(scores.is_empty());
    }

    // ------------------------------------------------------------------------
    // Bridge detection tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_find_bridges() {
        let graph = create_test_graph();
        let bridges = find_bridges(&graph, 2);

        assert!(!bridges.is_empty());
        assert!(bridges.len() <= 2);
    }

    #[test]
    fn test_find_bridges_empty() {
        let graph = GraphData::new();
        let bridges = find_bridges(&graph, 5);
        assert!(bridges.is_empty());
    }

    // ------------------------------------------------------------------------
    // get_related tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_get_related_outgoing() {
        let graph = create_test_graph();
        let related = get_related(
            &graph,
            "a",
            &[Relationship::Prerequisite],
            Direction::Outgoing,
        )
        .unwrap();

        assert_eq!(related.len(), 1);
        assert_eq!(related[0].0.id, "b");
        assert_eq!(related[0].1, Relationship::Prerequisite);
    }

    #[test]
    fn test_get_related_incoming() {
        let graph = create_test_graph();
        let related = get_related(
            &graph,
            "b",
            &[Relationship::Prerequisite],
            Direction::Incoming,
        )
        .unwrap();

        assert_eq!(related.len(), 1);
        assert_eq!(related[0].0.id, "a");
    }

    #[test]
    fn test_get_related_not_found() {
        let graph = create_test_graph();
        let result = get_related(
            &graph,
            "nonexistent",
            &[Relationship::Prerequisite],
            Direction::Outgoing,
        );
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Typed convenience method tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_graph_data_prerequisites() {
        let graph = create_test_graph();
        let prereqs = graph.prerequisites("a").unwrap();
        // a -> b is Prerequisite (outgoing from a, so b is "what a requires")
        assert_eq!(prereqs.len(), 1);
        assert_eq!(prereqs[0].id, "b");
    }

    #[test]
    fn test_graph_data_dependents() {
        let graph = create_test_graph();
        // b has an incoming Prerequisite from a
        let deps = graph.dependents("b").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, "a");
    }

    #[test]
    fn test_graph_data_related_by() {
        let graph = create_test_graph();
        let related = graph.related_by("a", &Relationship::RelatesTo).unwrap();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].id, "d");
    }
}
