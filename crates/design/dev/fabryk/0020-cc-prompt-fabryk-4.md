---
title: "CC Prompt: Fabryk 4.3 — Graph Algorithms"
milestone: "4.3"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1 Graph types complete", "4.2 GraphExtractor trait defined"]
governing-docs: [0011-audit §4.4, 0013-project-plan]
---

# CC Prompt: Fabryk 4.3 — Graph Algorithms

## Context

The graph algorithms module provides pathfinding, neighborhood exploration,
topological sorting, and centrality analysis. These algorithms operate on
`GraphData` and are **fully generic** — they don't know about music theory,
math, or any specific domain.

This is a low-risk milestone as the algorithms don't require parameterization.
They work with the generic `Node`, `Edge`, and `Relationship` types.

## Objective

Extract graph algorithms to `fabryk-graph::algorithms`:

1. `neighborhood()` - N-hop neighborhood expansion
2. `shortest_path()` - Dijkstra's pathfinding between nodes
3. `prerequisites_sorted()` - Topologically-ordered dependency chain
4. Centrality analysis functions
5. Bridge concept detection

## Implementation Steps

### Step 1: Create algorithms module

Create `fabryk-graph/src/algorithms.rs`:

```rust
//! Graph algorithms for knowledge graph analysis.
//!
//! This module provides algorithms for:
//! - Neighborhood exploration (N-hop expansion)
//! - Pathfinding (shortest path between concepts)
//! - Dependency analysis (prerequisites, topological ordering)
//! - Centrality analysis (identifying important nodes)
//! - Bridge detection (nodes connecting different clusters)
//!
//! All algorithms are generic and operate on `GraphData`.

use crate::{Edge, GraphData, Node, Relationship};
use fabryk_core::Result;
use petgraph::algo::{dijkstra, toposort};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

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
#[derive(Clone, Debug)]
pub struct CentralityScore {
    /// Node ID.
    pub node_id: String,
    /// Degree centrality (normalized: 0.0 to 1.0).
    pub degree: f32,
    /// In-degree centrality (how many nodes point to this).
    pub in_degree: f32,
    /// Out-degree centrality (how many nodes this points to).
    pub out_degree: f32,
    /// Betweenness centrality (how often on shortest paths).
    pub betweenness: Option<f32>,
}

/// Get the N-hop neighborhood around a node.
///
/// Performs a breadth-first search from the center node, collecting all
/// nodes and edges within the specified radius.
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `center_id` - ID of the center node
/// * `radius` - Maximum distance from center (hops)
/// * `relationship_filter` - Optional filter for edge types to follow
///
/// # Returns
///
/// `NeighborhoodResult` containing nodes, edges, and distances.
///
/// # Example
///
/// ```rust,ignore
/// let result = neighborhood(&graph, "major-triad", 2, None)?;
/// println!("Found {} nodes within 2 hops", result.nodes.len());
/// ```
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

            // Apply relationship filter if specified
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

/// Find the shortest path between two nodes.
///
/// Uses Dijkstra's algorithm with edge weights.
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `from_id` - Starting node ID
/// * `to_id` - Target node ID
///
/// # Returns
///
/// `PathResult` with the path (if found) or `found: false`.
///
/// # Example
///
/// ```rust,ignore
/// let result = shortest_path(&graph, "major-scale", "jazz-harmony")?;
/// if result.found {
///     println!("Path length: {} nodes", result.path.len());
/// }
/// ```
pub fn shortest_path(graph: &GraphData, from_id: &str, to_id: &str) -> Result<PathResult> {
    let from_idx = match graph.get_index(from_id) {
        Some(idx) => idx,
        None => return Ok(PathResult::not_found()),
    };

    let to_idx = match graph.get_index(to_id) {
        Some(idx) => idx,
        None => return Ok(PathResult::not_found()),
    };

    // Use Dijkstra to find shortest path
    let costs = dijkstra(&graph.graph, from_idx, Some(to_idx), |e| {
        // Use edge weight as cost (lower weight = prefer)
        // Invert so higher relationship weight = lower cost
        1.0 / e.weight().weight.max(0.01)
    });

    if !costs.contains_key(&to_idx) {
        return Ok(PathResult::not_found());
    }

    // Reconstruct path by backtracking
    let mut path_indices = vec![to_idx];
    let mut current = to_idx;

    while current != from_idx {
        let mut found_prev = false;
        for edge_ref in graph.graph.edges_directed(current, Direction::Incoming) {
            let source = edge_ref.source();
            if costs.contains_key(&source) {
                let edge_cost = 1.0 / edge_ref.weight().weight.max(0.01);
                let expected_cost = costs[&current] - edge_cost;
                if (costs[&source] - expected_cost).abs() < 0.001 {
                    path_indices.push(source);
                    current = source;
                    found_prev = true;
                    break;
                }
            }
        }
        if !found_prev {
            // Fallback: find any predecessor in costs
            for edge_ref in graph.graph.edges_directed(current, Direction::Incoming) {
                let source = edge_ref.source();
                if costs.contains_key(&source) && !path_indices.contains(&source) {
                    path_indices.push(source);
                    current = source;
                    found_prev = true;
                    break;
                }
            }
        }
        if !found_prev {
            break;
        }
    }

    path_indices.reverse();

    // Build result
    let path: Vec<Node> = path_indices
        .iter()
        .map(|idx| graph.graph[*idx].clone())
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

/// Get prerequisites for a concept in topological order.
///
/// Follows `Prerequisite` edges backwards to find all dependencies,
/// then returns them in learning order (fundamentals first).
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `target_id` - ID of the concept to analyze
///
/// # Returns
///
/// `PrerequisitesResult` with ordered prerequisites.
///
/// # Example
///
/// ```rust,ignore
/// let result = prerequisites_sorted(&graph, "jazz-improvisation")?;
/// for prereq in &result.ordered {
///     println!("Learn: {}", prereq.title);
/// }
/// println!("Then learn: {}", result.target.title);
/// ```
pub fn prerequisites_sorted(graph: &GraphData, target_id: &str) -> Result<PrerequisitesResult> {
    let target_node = graph
        .get_node(target_id)
        .ok_or_else(|| fabryk_core::Error::not_found("node", target_id))?
        .clone();

    let target_idx = graph.get_index(target_id).unwrap();

    // Collect all prerequisites (recursive)
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

    // Build subgraph of prerequisites
    let prereq_vec: Vec<NodeIndex> = prereq_indices.iter().copied().collect();

    // Try topological sort
    let sorted = toposort(&graph.graph, None);

    let (ordered, has_cycles) = match sorted {
        Ok(all_sorted) => {
            // Filter to just our prerequisites, maintaining order
            let ordered: Vec<Node> = all_sorted
                .into_iter()
                .filter(|idx| prereq_indices.contains(idx))
                .map(|idx| graph.graph[idx].clone())
                .collect();
            (ordered, false)
        }
        Err(_) => {
            // Cycle detected - return in arbitrary order
            let ordered: Vec<Node> = prereq_vec
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
///
/// # Arguments
///
/// * `graph` - The graph to analyze
///
/// # Returns
///
/// Vector of `CentralityScore` for each node, sorted by degree (descending).
pub fn calculate_centrality(graph: &GraphData) -> Vec<CentralityScore> {
    let n = graph.node_count() as f32;
    if n < 2.0 {
        return Vec::new();
    }

    let mut scores: Vec<CentralityScore> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();

            let in_degree = graph
                .graph
                .edges_directed(idx, Direction::Incoming)
                .count() as f32;
            let out_degree = graph
                .graph
                .edges_directed(idx, Direction::Outgoing)
                .count() as f32;
            let degree = in_degree + out_degree;

            CentralityScore {
                node_id: node.id.clone(),
                degree: degree / (2.0 * (n - 1.0)),
                in_degree: in_degree / (n - 1.0),
                out_degree: out_degree / (n - 1.0),
                betweenness: None, // Computed separately if needed
            }
        })
        .collect();

    scores.sort_by(|a, b| b.degree.partial_cmp(&a.degree).unwrap());
    scores
}

/// Find bridge concepts connecting different clusters.
///
/// Identifies nodes that, if removed, would disconnect parts of the graph.
/// These are often important "gateway" concepts.
///
/// # Arguments
///
/// * `graph` - The graph to analyze
/// * `limit` - Maximum number of bridges to return
///
/// # Returns
///
/// Nodes identified as bridges, sorted by importance.
pub fn find_bridges(graph: &GraphData, limit: usize) -> Vec<Node> {
    // Simple heuristic: nodes with high betweenness-like scores
    // A node is a bridge if it connects otherwise-distant clusters
    // Approximate by finding nodes with both high in and out degree
    // that connect to diverse categories

    let mut bridge_scores: Vec<(String, f32)> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();

            let in_degree = graph
                .graph
                .edges_directed(idx, Direction::Incoming)
                .count() as f32;
            let out_degree = graph
                .graph
                .edges_directed(idx, Direction::Outgoing)
                .count() as f32;

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
            let score = (in_degree.min(out_degree) + 1.0) * (diversity + 1.0);

            (node.id.clone(), score)
        })
        .collect();

    bridge_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    bridge_scores
        .into_iter()
        .take(limit)
        .filter_map(|(id, _)| graph.get_node(&id).cloned())
        .collect()
}

/// Get nodes related to a given node by specific relationship types.
///
/// # Arguments
///
/// * `graph` - The graph to search
/// * `node_id` - ID of the source node
/// * `relationships` - Relationship types to follow
/// * `direction` - Whether to follow outgoing, incoming, or both edges
///
/// # Returns
///
/// Related nodes with their relationships.
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
```

### Step 2: Update lib.rs

Update `fabryk-graph/src/lib.rs`:

```rust
pub mod algorithms;
pub mod extractor;
pub mod types;

// Re-exports
pub use algorithms::{
    calculate_centrality, find_bridges, get_related, neighborhood, prerequisites_sorted,
    shortest_path, CentralityScore, NeighborhoodResult, PathResult, PrerequisitesResult,
};
pub use extractor::GraphExtractor;
pub use types::{Edge, EdgeOrigin, GraphData, Node, Relationship};

#[cfg(any(test, feature = "test-utils"))]
pub use extractor::mock::{MockEdgeData, MockExtractor, MockNodeData};
```

### Step 3: Add algorithm tests

Add to `fabryk-graph/src/algorithms.rs`:

```rust
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
            let idx = graph.graph.add_node(node.clone());
            graph.node_indices.insert(node.id.clone(), idx);
            graph.nodes.insert(node.id.clone(), node.clone());
        }

        let edges = vec![
            Edge::new("a", "b", Relationship::Prerequisite),
            Edge::new("b", "c", Relationship::Prerequisite),
            Edge::new("a", "d", Relationship::RelatesTo),
            Edge::new("b", "d", Relationship::LeadsTo),
        ];

        for edge in &edges {
            let from_idx = graph.node_indices[&edge.from];
            let to_idx = graph.node_indices[&edge.to];
            graph.graph.add_edge(from_idx, to_idx, edge.clone());
            graph.edges.push(edge.clone());
        }

        graph
    }

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

        // Should find b, d (radius 1) and c (radius 2)
        assert!(result.nodes.iter().any(|n| n.id == "c"));
        assert_eq!(result.distances["c"], 2);
    }

    #[test]
    fn test_neighborhood_with_filter() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "a", 2, Some(&[Relationship::Prerequisite])).unwrap();

        // Should only follow Prerequisite edges: a -> b -> c
        assert!(result.nodes.iter().any(|n| n.id == "b"));
        assert!(result.nodes.iter().any(|n| n.id == "c"));
        // d should not be included (connected via RelatesTo)
        assert!(!result.nodes.iter().any(|n| n.id == "d"));
    }

    #[test]
    fn test_neighborhood_not_found() {
        let graph = create_test_graph();
        let result = neighborhood(&graph, "nonexistent", 1, None);
        assert!(result.is_err());
    }

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
        // c has no outgoing edges to a
        let result = shortest_path(&graph, "c", "a").unwrap();
        assert!(!result.found);
    }

    #[test]
    fn test_prerequisites_sorted() {
        let graph = create_test_graph();
        let result = prerequisites_sorted(&graph, "c").unwrap();

        assert_eq!(result.target.id, "c");
        assert!(!result.has_cycles);
        // Prerequisites of c: a and b (via a -> b -> c)
        assert!(result.ordered.iter().any(|n| n.id == "a"));
        assert!(result.ordered.iter().any(|n| n.id == "b"));
        // a should come before b
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
    fn test_calculate_centrality() {
        let graph = create_test_graph();
        let scores = calculate_centrality(&graph);

        assert_eq!(scores.len(), 4);
        // All nodes should have valid centrality scores
        for score in &scores {
            assert!(score.degree >= 0.0 && score.degree <= 1.0);
        }
    }

    #[test]
    fn test_find_bridges() {
        let graph = create_test_graph();
        let bridges = find_bridges(&graph, 2);

        // b connects cat1 to cat2 (via c), should be a bridge
        assert!(!bridges.is_empty());
    }

    #[test]
    fn test_get_related() {
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
}
```

### Step 4: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings
```

## Exit Criteria

- [ ] `neighborhood()` function with N-hop expansion
- [ ] `shortest_path()` using Dijkstra's algorithm
- [ ] `prerequisites_sorted()` with topological ordering
- [ ] `calculate_centrality()` for degree-based metrics
- [ ] `find_bridges()` for gateway concept detection
- [ ] `get_related()` for filtered relationship queries
- [ ] All functions return appropriate result types
- [ ] Tests for all algorithms pass
- [ ] No warnings from clippy

## Commit Message

```
feat(graph): add graph algorithms module

Add algorithms for knowledge graph analysis:
- neighborhood(): N-hop BFS expansion with relationship filtering
- shortest_path(): Dijkstra's pathfinding
- prerequisites_sorted(): Topological ordering of dependencies
- calculate_centrality(): Degree-based centrality metrics
- find_bridges(): Gateway concept detection
- get_related(): Filtered relationship queries

All algorithms are generic, operating on GraphData types.

Phase 4 milestone 4.3 of Fabryk extraction.

Ref: Doc 0011 §4.4 (graph algorithms)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
