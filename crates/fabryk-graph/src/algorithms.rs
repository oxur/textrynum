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

use crate::{Edge, GraphData, Node, NodeSummary, Relationship};
use fabryk_core::Result;
use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
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

/// A single step in a learning path.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningStep {
    /// 1-based step number.
    pub step: usize,
    /// Node ID.
    pub node_id: String,
    /// Node title.
    pub title: String,
    /// Optional category.
    pub category: Option<String>,
}

/// Result of a learning path query.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LearningPathResult {
    /// The target node.
    pub target: NodeSummary,
    /// Ordered learning steps (prerequisites first, target last).
    pub steps: Vec<LearningStep>,
    /// Total number of steps.
    pub total_steps: usize,
    /// Whether cycles were detected in the prerequisite graph.
    pub has_cycles: bool,
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

            if let Some(filter) = relationship_filter
                && !filter.contains(&edge_weight.relationship)
            {
                continue;
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

            if let Some(filter) = relationship_filter
                && !filter.contains(&edge_weight.relationship)
            {
                continue;
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
                if let [from, to] = window
                    && let Some(edge_idx) = graph.graph.find_edge(*from, *to)
                {
                    edges.push(graph.graph[edge_idx].clone());
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

/// Find all nodes that depend on the given node (reverse prerequisite traversal).
///
/// Follows incoming `Prerequisite` edges up to `max_depth` hops via BFS.
/// Returns nodes in BFS order (closest dependents first), excluding the
/// start node itself.
///
/// This is the reverse of `prerequisites_sorted`: while prerequisites answers
/// "what must I learn before X?", dependents answers "what depends on X?".
pub fn dependents(graph: &GraphData, id: &str, max_depth: usize) -> Result<Vec<Node>> {
    // Validate node exists before traversal.
    let start_idx = graph
        .get_index(id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", id))?;

    let max_depth = max_depth.min(MAX_BFS_DEPTH);

    let mut visited: HashSet<NodeIndex> = HashSet::new();
    let mut queue: VecDeque<(NodeIndex, usize)> = VecDeque::new();
    let mut result_nodes: Vec<Node> = Vec::new();

    visited.insert(start_idx);
    queue.push_back((start_idx, 0));

    while let Some((current_idx, current_dist)) = queue.pop_front() {
        if current_dist >= max_depth {
            continue;
        }

        // Follow INCOMING Prerequisite edges: if edge is A -> current (Prerequisite),
        // then A depends on current (A requires current as a prerequisite).
        for edge_ref in graph.graph.edges_directed(current_idx, Direction::Incoming) {
            if edge_ref.weight().relationship == Relationship::Prerequisite {
                let source_idx = edge_ref.source();
                if visited.insert(source_idx) {
                    let node = graph.graph[source_idx].clone();
                    result_nodes.push(node);
                    queue.push_back((source_idx, current_dist + 1));
                }
            }
        }
    }

    Ok(result_nodes)
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

/// Find all source nodes that introduce or cover a given concept.
///
/// Looks for incoming edges of type `Introduces` or `Covers` targeting
/// `concept_id` and returns the source (from) nodes.
pub fn concept_sources(graph: &GraphData, concept_id: &str) -> Result<Vec<Node>> {
    // Validate that the concept node exists.
    let _idx = graph
        .get_index(concept_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", concept_id))?;

    let mut results = Vec::new();
    for edge in graph.iter_edges() {
        if edge.to == concept_id
            && matches!(
                edge.relationship,
                Relationship::Introduces | Relationship::Covers
            )
            && let Some(node) = graph.get_node(&edge.from)
        {
            results.push(node.clone());
        }
    }
    Ok(results)
}

/// Find all source-specific variants of a canonical concept.
///
/// Looks for nodes connected to `canonical_id` via incoming `VariantOf`
/// edges and returns the variant (from) nodes.
pub fn concept_variants(graph: &GraphData, canonical_id: &str) -> Result<Vec<Node>> {
    // Validate that the canonical node exists.
    let _idx = graph
        .get_index(canonical_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", canonical_id))?;

    let mut results = Vec::new();
    for edge in graph.iter_edges() {
        if edge.to == canonical_id
            && edge.relationship == Relationship::VariantOf
            && let Some(node) = graph.get_node(&edge.from)
        {
            results.push(node.clone());
        }
    }
    Ok(results)
}

/// Find all concepts that a given source introduces or covers.
///
/// Looks for outgoing edges of type `Introduces` or `Covers` from
/// `source_id` and returns the target (to) nodes.
pub fn source_coverage(graph: &GraphData, source_id: &str) -> Result<Vec<Node>> {
    // Validate that the source node exists.
    let _idx = graph
        .get_index(source_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", source_id))?;

    let mut results = Vec::new();
    for edge in graph.iter_edges() {
        if edge.from == source_id
            && matches!(
                edge.relationship,
                Relationship::Introduces | Relationship::Covers
            )
            && let Some(node) = graph.get_node(&edge.to)
        {
            results.push(node.clone());
        }
    }
    Ok(results)
}

/// Produce a step-numbered learning path to reach a target concept.
///
/// Wraps `prerequisites_sorted` with step numbering and summary formatting.
/// The target node itself is included as the final step.
pub fn learning_path(graph: &GraphData, target_id: &str) -> Result<LearningPathResult> {
    let result = prerequisites_sorted(graph, target_id)?;

    let target_summary = NodeSummary::from(&result.target);

    let mut steps: Vec<LearningStep> = result
        .ordered
        .iter()
        .enumerate()
        .map(|(i, node)| LearningStep {
            step: i + 1,
            node_id: node.id.clone(),
            title: node.title.clone(),
            category: node.category.clone(),
        })
        .collect();

    // Add the target itself as the final step.
    steps.push(LearningStep {
        step: steps.len() + 1,
        node_id: result.target.id.clone(),
        title: result.target.title.clone(),
        category: result.target.category.clone(),
    });

    let total_steps = steps.len();

    Ok(LearningPathResult {
        target: target_summary,
        steps,
        total_steps,
        has_cycles: result.has_cycles,
    })
}

/// Find nodes that bridge two categories (have neighbors in both).
///
/// A bridge node has at least one neighbor (via any edge direction) in
/// `category_a` and at least one neighbor in `category_b`. The bridge
/// node itself may belong to any category (or none).
pub fn bridge_between_categories(
    graph: &GraphData,
    category_a: &str,
    category_b: &str,
    limit: usize,
) -> Result<Vec<Node>> {
    let mut results = Vec::new();

    for node in graph.iter_nodes() {
        let idx = match graph.get_index(&node.id) {
            Some(idx) => idx,
            None => continue,
        };

        let mut has_cat_a = false;
        let mut has_cat_b = false;

        // Check all neighbors in both directions.
        for neighbor_idx in graph.graph.neighbors_undirected(idx) {
            let neighbor = &graph.graph[neighbor_idx];
            if let Some(ref cat) = neighbor.category {
                if cat == category_a {
                    has_cat_a = true;
                }
                if cat == category_b {
                    has_cat_b = true;
                }
                if has_cat_a && has_cat_b {
                    break;
                }
            }
        }

        if has_cat_a && has_cat_b {
            results.push(node.clone());
            if results.len() >= limit {
                break;
            }
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
    // Dependents tests
    // ------------------------------------------------------------------------

    fn create_chain_graph() -> GraphData {
        // Chain: A -> B -> C (all Prerequisite edges)
        // Edge semantics: from is prerequisite of to.
        // So A is prereq of B, B is prereq of C.
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));
        graph.add_node(Node::new("b", "Node B"));
        graph.add_node(Node::new("c", "Node C"));

        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::Prerequisite))
            .unwrap();

        graph
    }

    #[test]
    fn test_dependents_chain_full_depth() {
        let graph = create_chain_graph();
        // dependents of C: follows incoming prereq edges from C.
        // B->C found, then A->B found. Result: [B, A] in BFS order.
        let result = dependents(&graph, "c", 3).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "b");
        assert_eq!(result[1].id, "a");
    }

    #[test]
    fn test_dependents_chain_depth_1() {
        let graph = create_chain_graph();
        // With depth=1, only immediate incoming prereqs: [B]
        let result = dependents(&graph, "c", 1).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "b");
    }

    #[test]
    fn test_dependents_leaf_node() {
        let graph = create_chain_graph();
        // A has no incoming prereq edges, so dependents = []
        let result = dependents(&graph, "a", 3).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_dependents_max_depth_respected() {
        let graph = create_chain_graph();
        // depth=0 means no traversal at all
        let result = dependents(&graph, "c", 0).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_dependents_node_not_found() {
        let graph = create_chain_graph();
        let result = dependents(&graph, "nonexistent", 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_dependents_only_follows_prerequisite_edges() {
        // Use the test graph which has mixed relationship types
        let graph = create_test_graph();
        // Node D has incoming edges: A->D (RelatesTo), B->D (LeadsTo)
        // Neither is Prerequisite, so dependents should be empty
        let result = dependents(&graph, "d", 3).unwrap();
        assert!(result.is_empty());
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

    // ------------------------------------------------------------------------
    // Source-concept relationship helpers
    // ------------------------------------------------------------------------

    fn create_source_concept_graph() -> GraphData {
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
                .as_variant_of("concept-x")
                .with_node_type(NodeType::Custom("source".to_string())),
        );
        graph.add_node(
            Node::new("concept-x-beta", "Concept X (Beta)")
                .as_variant_of("concept-x")
                .with_node_type(NodeType::Custom("source".to_string())),
        );

        // Edges: book-alpha introduces concept-x, covers concept-y
        graph
            .add_edge(Edge::new(
                "book-alpha",
                "concept-x",
                Relationship::Introduces,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new("book-alpha", "concept-y", Relationship::Covers))
            .unwrap();

        // Edges: book-beta covers concept-x
        graph
            .add_edge(Edge::new("book-beta", "concept-x", Relationship::Covers))
            .unwrap();

        // Variant edges
        graph
            .add_edge(Edge::new(
                "concept-x-alpha",
                "concept-x",
                Relationship::VariantOf,
            ))
            .unwrap();
        graph
            .add_edge(Edge::new(
                "concept-x-beta",
                "concept-x",
                Relationship::VariantOf,
            ))
            .unwrap();

        graph
    }

    // -- concept_sources tests ------------------------------------------------

    #[test]
    fn test_concept_sources_multiple() {
        let graph = create_source_concept_graph();
        let sources = concept_sources(&graph, "concept-x").unwrap();
        // book-alpha (Introduces) and book-beta (Covers)
        assert_eq!(sources.len(), 2);
        let ids: Vec<&str> = sources.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"book-alpha"));
        assert!(ids.contains(&"book-beta"));
    }

    #[test]
    fn test_concept_sources_single() {
        let graph = create_source_concept_graph();
        let sources = concept_sources(&graph, "concept-y").unwrap();
        // Only book-alpha covers concept-y
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].id, "book-alpha");
    }

    #[test]
    fn test_concept_sources_none() {
        let graph = create_source_concept_graph();
        // concept-x-alpha has no incoming Introduces/Covers edges
        let sources = concept_sources(&graph, "concept-x-alpha").unwrap();
        assert!(sources.is_empty());
    }

    #[test]
    fn test_concept_sources_not_found() {
        let graph = create_source_concept_graph();
        let result = concept_sources(&graph, "nonexistent");
        assert!(result.is_err());
    }

    // -- concept_variants tests -----------------------------------------------

    #[test]
    fn test_concept_variants_multiple() {
        let graph = create_source_concept_graph();
        let variants = concept_variants(&graph, "concept-x").unwrap();
        assert_eq!(variants.len(), 2);
        let ids: Vec<&str> = variants.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"concept-x-alpha"));
        assert!(ids.contains(&"concept-x-beta"));
    }

    #[test]
    fn test_concept_variants_none() {
        let graph = create_source_concept_graph();
        // concept-y has no VariantOf edges
        let variants = concept_variants(&graph, "concept-y").unwrap();
        assert!(variants.is_empty());
    }

    #[test]
    fn test_concept_variants_not_found() {
        let graph = create_source_concept_graph();
        let result = concept_variants(&graph, "nonexistent");
        assert!(result.is_err());
    }

    // -- source_coverage tests ------------------------------------------------

    #[test]
    fn test_source_coverage_multiple() {
        let graph = create_source_concept_graph();
        let concepts = source_coverage(&graph, "book-alpha").unwrap();
        // book-alpha introduces concept-x, covers concept-y
        assert_eq!(concepts.len(), 2);
        let ids: Vec<&str> = concepts.iter().map(|n| n.id.as_str()).collect();
        assert!(ids.contains(&"concept-x"));
        assert!(ids.contains(&"concept-y"));
    }

    #[test]
    fn test_source_coverage_single() {
        let graph = create_source_concept_graph();
        let concepts = source_coverage(&graph, "book-beta").unwrap();
        // book-beta only covers concept-x
        assert_eq!(concepts.len(), 1);
        assert_eq!(concepts[0].id, "concept-x");
    }

    #[test]
    fn test_source_coverage_none() {
        let graph = create_source_concept_graph();
        // concept-x is not a source, has no outgoing Introduces/Covers
        let concepts = source_coverage(&graph, "concept-x").unwrap();
        assert!(concepts.is_empty());
    }

    #[test]
    fn test_source_coverage_not_found() {
        let graph = create_source_concept_graph();
        let result = source_coverage(&graph, "nonexistent");
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // Learning path tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_learning_path_basic() {
        let graph = create_test_graph();
        let result = learning_path(&graph, "c").unwrap();

        assert_eq!(result.target.id, "c");
        assert!(!result.has_cycles);
        // Prerequisites a, b plus target c = 3 steps.
        assert_eq!(result.total_steps, 3);
        assert_eq!(result.steps.len(), 3);

        // First steps should be prerequisites in order, last step is target.
        assert_eq!(result.steps.last().unwrap().node_id, "c");
        assert_eq!(result.steps.last().unwrap().step, 3);

        // a should come before b in the learning path.
        let a_step = result.steps.iter().find(|s| s.node_id == "a").unwrap();
        let b_step = result.steps.iter().find(|s| s.node_id == "b").unwrap();
        assert!(a_step.step < b_step.step);
    }

    #[test]
    fn test_learning_path_no_prerequisites() {
        let graph = create_test_graph();
        let result = learning_path(&graph, "a").unwrap();

        assert_eq!(result.target.id, "a");
        // Only the target itself.
        assert_eq!(result.total_steps, 1);
        assert_eq!(result.steps[0].node_id, "a");
        assert_eq!(result.steps[0].step, 1);
    }

    #[test]
    fn test_learning_path_not_found() {
        let graph = create_test_graph();
        let result = learning_path(&graph, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_learning_path_step_numbering() {
        let graph = create_test_graph();
        let result = learning_path(&graph, "c").unwrap();

        // Steps should be 1-based and sequential.
        for (i, step) in result.steps.iter().enumerate() {
            assert_eq!(step.step, i + 1);
        }
    }

    #[test]
    fn test_learning_path_categories_populated() {
        let graph = create_test_graph();
        let result = learning_path(&graph, "c").unwrap();

        // Node C has category "cat2".
        let target_step = result.steps.last().unwrap();
        assert_eq!(target_step.category.as_deref(), Some("cat2"));
    }

    // ------------------------------------------------------------------------
    // Bridge between categories tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_bridge_between_categories_basic() {
        // In create_test_graph:
        //   a (cat1) -> b (cat1) -> c (cat2), a -> d (cat2), b -> d (cat2)
        // Node b has neighbors: a (cat1), c (cat2), d (cat2) — bridges cat1 and cat2.
        // Node a has neighbors: b (cat1), d (cat2) — bridges cat1 and cat2.
        let graph = create_test_graph();
        let bridges = bridge_between_categories(&graph, "cat1", "cat2", 10).unwrap();

        assert!(!bridges.is_empty());
        let ids: Vec<&str> = bridges.iter().map(|n| n.id.as_str()).collect();
        // Both a and b bridge cat1 and cat2.
        assert!(ids.contains(&"a") || ids.contains(&"b"));
    }

    #[test]
    fn test_bridge_between_categories_limit() {
        let graph = create_test_graph();
        let bridges = bridge_between_categories(&graph, "cat1", "cat2", 1).unwrap();

        assert_eq!(bridges.len(), 1);
    }

    #[test]
    fn test_bridge_between_categories_no_match() {
        let graph = create_test_graph();
        let bridges =
            bridge_between_categories(&graph, "cat1", "nonexistent_category", 10).unwrap();

        assert!(bridges.is_empty());
    }

    #[test]
    fn test_bridge_between_categories_empty_graph() {
        let graph = GraphData::new();
        let bridges = bridge_between_categories(&graph, "cat1", "cat2", 10).unwrap();

        assert!(bridges.is_empty());
    }

    #[test]
    fn test_bridge_between_categories_same_category() {
        // When both categories are the same, a bridge needs at least 2 neighbors
        // in that category. In test graph: a has neighbor b (cat1), that's only 1
        // in cat1, but since cat_a == cat_b, having 1 neighbor satisfies both.
        let graph = create_test_graph();
        let bridges = bridge_between_categories(&graph, "cat1", "cat1", 10).unwrap();

        // Any node with at least one cat1 neighbor qualifies.
        let ids: Vec<&str> = bridges.iter().map(|n| n.id.as_str()).collect();
        // b has neighbor a (cat1), c has neighbor b (cat1) via incoming.
        assert!(!bridges.is_empty());
        // a has neighbor b (cat1).
        assert!(ids.contains(&"a") || ids.contains(&"c"));
    }
}
