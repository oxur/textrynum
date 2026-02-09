---
title: "CC Prompt: Fabryk 4.6 — Query, Stats, Validation"
milestone: "4.6"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1-4.5 complete"]
governing-docs: [0011-audit §4.4, 0013-project-plan]
---

# CC Prompt: Fabryk 4.6 — Query, Stats, Validation

## Context

This milestone extracts the supporting modules that provide:

1. **Query** - Response types for graph queries (MCP tool responses)
2. **Stats** - Graph statistics and analysis
3. **Validation** - Structural validation (orphans, cycles, integrity)

These are all generic modules that operate on `GraphData` without
domain-specific knowledge.

## Objective

Extract supporting modules to `fabryk-graph`:

1. `query.rs` - Response types for graph queries
2. `stats.rs` - Graph statistics computation
3. `validation.rs` - Structural validation

## Implementation Steps

### Step 1: Create query module

Create `fabryk-graph/src/query.rs`:

```rust
//! Query response types for graph operations.
//!
//! This module provides structured response types used by MCP tools
//! and other interfaces to return graph query results.

use crate::{Edge, Node, Relationship};
use serde::{Deserialize, Serialize};

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
    pub category: String,
    pub count: usize,
}

/// Relationship type with count.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelationshipCount {
    pub relationship: String,
    pub count: usize,
}
```

### Step 2: Create stats module

Create `fabryk-graph/src/stats.rs`:

```rust
//! Graph statistics and analysis.
//!
//! Provides functions for analyzing graph structure and composition.

use crate::{GraphData, Relationship};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Average edges per node.
    pub avg_degree: f32,
    /// Maximum in-degree (most dependencies).
    pub max_in_degree: usize,
    /// Maximum out-degree (most dependents).
    pub max_out_degree: usize,
    /// Node with highest in-degree.
    pub most_depended_on: Option<String>,
    /// Node with highest out-degree.
    pub most_dependencies: Option<String>,
}

/// Compute comprehensive statistics for a graph.
///
/// # Example
///
/// ```rust,ignore
/// let stats = compute_stats(&graph);
/// println!("Nodes: {}, Edges: {}", stats.node_count, stats.edge_count);
/// ```
pub fn compute_stats(graph: &GraphData) -> GraphStats {
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();

    // Count canonical vs variant
    let mut canonical_count = 0;
    let mut variant_count = 0;
    for node in graph.iter_nodes() {
        if node.is_canonical {
            canonical_count += 1;
        } else {
            variant_count += 1;
        }
    }

    // Category distribution
    let mut category_distribution: HashMap<String, usize> = HashMap::new();
    for node in graph.iter_nodes() {
        let cat = node.category.clone().unwrap_or_else(|| "uncategorized".to_string());
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

    // Initialize all nodes with 0 degree
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

    let total_degree: usize = in_degrees.values().sum::<usize>() + out_degrees.values().sum::<usize>();
    let avg_degree = if node_count > 0 {
        total_degree as f32 / node_count as f32
    } else {
        0.0
    };

    let (most_depended_on, max_in_degree) = in_degrees
        .iter()
        .max_by_key(|(_, &v)| v)
        .map(|(k, &v)| (Some(k.clone()), v))
        .unwrap_or((None, 0));

    let (most_dependencies, max_out_degree) = out_degrees
        .iter()
        .max_by_key(|(_, &v)| v)
        .map(|(k, &v)| (Some(k.clone()), v))
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
    }
}

/// Get a quick summary of graph size.
pub fn quick_summary(graph: &GraphData) -> String {
    format!(
        "{} nodes, {} edges",
        graph.node_count(),
        graph.edge_count()
    )
}

/// Get top N nodes by a specific metric.
pub fn top_nodes_by_degree(graph: &GraphData, limit: usize, direction: DegreeDirection) -> Vec<(String, usize)> {
    use petgraph::Direction;

    let mut scores: Vec<(String, usize)> = graph
        .iter_nodes()
        .map(|node| {
            let idx = graph.get_index(&node.id).unwrap();
            let degree = match direction {
                DegreeDirection::In => graph
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .count(),
                DegreeDirection::Out => graph
                    .graph
                    .edges_directed(idx, Direction::Outgoing)
                    .count(),
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

/// Direction for degree calculation.
#[derive(Clone, Copy, Debug)]
pub enum DegreeDirection {
    In,
    Out,
    Both,
}
```

### Step 3: Create validation module

Create `fabryk-graph/src/validation.rs`:

```rust
//! Graph validation and integrity checking.
//!
//! Provides functions to validate graph structure and detect issues.

use crate::{GraphData, Relationship};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Result of graph validation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the graph is valid (no critical issues).
    pub valid: bool,
    /// Critical issues that should be fixed.
    pub errors: Vec<ValidationIssue>,
    /// Non-critical issues (warnings).
    pub warnings: Vec<ValidationIssue>,
    /// Informational findings.
    pub info: Vec<ValidationIssue>,
}

impl ValidationResult {
    /// Create a new empty validation result.
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Add an error.
    pub fn add_error(&mut self, issue: ValidationIssue) {
        self.valid = false;
        self.errors.push(issue);
    }

    /// Add a warning.
    pub fn add_warning(&mut self, issue: ValidationIssue) {
        self.warnings.push(issue);
    }

    /// Add info.
    pub fn add_info(&mut self, issue: ValidationIssue) {
        self.info.push(issue);
    }

    /// Total issue count.
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// A validation issue found in the graph.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Issue type/code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Affected node IDs (if applicable).
    pub nodes: Vec<String>,
    /// Affected edge descriptions (if applicable).
    pub edges: Vec<String>,
}

impl ValidationIssue {
    /// Create a new issue.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    /// Add affected nodes.
    pub fn with_nodes(mut self, nodes: Vec<String>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Add affected edges.
    pub fn with_edges(mut self, edges: Vec<String>) -> Self {
        self.edges = edges;
        self
    }
}

/// Validate a graph for common issues.
///
/// Checks for:
/// - Orphan nodes (no connections)
/// - Self-loops (edge from node to itself)
/// - Duplicate edges
/// - Prerequisite cycles
/// - Missing canonical references
///
/// # Example
///
/// ```rust,ignore
/// let result = validate_graph(&graph);
/// if !result.valid {
///     for error in &result.errors {
///         println!("Error: {}", error.message);
///     }
/// }
/// ```
pub fn validate_graph(graph: &GraphData) -> ValidationResult {
    let mut result = ValidationResult::new();

    // Check for orphan nodes
    check_orphans(graph, &mut result);

    // Check for self-loops
    check_self_loops(graph, &mut result);

    // Check for duplicate edges
    check_duplicate_edges(graph, &mut result);

    // Check for prerequisite cycles
    check_prerequisite_cycles(graph, &mut result);

    // Check canonical references
    check_canonical_references(graph, &mut result);

    result
}

/// Check for orphan nodes (no incoming or outgoing edges).
fn check_orphans(graph: &GraphData, result: &mut ValidationResult) {
    use petgraph::Direction;

    let orphans: Vec<String> = graph
        .iter_nodes()
        .filter(|node| {
            let idx = graph.get_index(&node.id).unwrap();
            let in_count = graph.graph.edges_directed(idx, Direction::Incoming).count();
            let out_count = graph.graph.edges_directed(idx, Direction::Outgoing).count();
            in_count == 0 && out_count == 0
        })
        .map(|node| node.id.clone())
        .collect();

    if !orphans.is_empty() {
        result.add_warning(
            ValidationIssue::new(
                "ORPHAN_NODES",
                format!("{} nodes have no connections", orphans.len()),
            )
            .with_nodes(orphans),
        );
    }
}

/// Check for self-loops (edges from a node to itself).
fn check_self_loops(graph: &GraphData, result: &mut ValidationResult) {
    let self_loops: Vec<String> = graph
        .iter_edges()
        .filter(|edge| edge.from == edge.to)
        .map(|edge| format!("{} -> {}", edge.from, edge.to))
        .collect();

    if !self_loops.is_empty() {
        result.add_error(
            ValidationIssue::new(
                "SELF_LOOPS",
                format!("{} edges are self-loops", self_loops.len()),
            )
            .with_edges(self_loops),
        );
    }
}

/// Check for duplicate edges (same from, to, and relationship).
fn check_duplicate_edges(graph: &GraphData, result: &mut ValidationResult) {
    let mut seen: HashSet<(String, String, String)> = HashSet::new();
    let mut duplicates: Vec<String> = Vec::new();

    for edge in graph.iter_edges() {
        let key = (
            edge.from.clone(),
            edge.to.clone(),
            edge.relationship.name().to_string(),
        );
        if !seen.insert(key.clone()) {
            duplicates.push(format!(
                "{} -[{}]-> {}",
                edge.from,
                edge.relationship.name(),
                edge.to
            ));
        }
    }

    if !duplicates.is_empty() {
        result.add_warning(
            ValidationIssue::new(
                "DUPLICATE_EDGES",
                format!("{} duplicate edges found", duplicates.len()),
            )
            .with_edges(duplicates),
        );
    }
}

/// Check for cycles in prerequisite relationships.
fn check_prerequisite_cycles(graph: &GraphData, result: &mut ValidationResult) {
    use petgraph::algo::toposort;
    use petgraph::graph::DiGraph;

    // Build a subgraph with only prerequisite edges
    let mut prereq_graph: DiGraph<String, ()> = DiGraph::new();
    let mut indices: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node in graph.iter_nodes() {
        let idx = prereq_graph.add_node(node.id.clone());
        indices.insert(node.id.clone(), idx);
    }

    for edge in graph.iter_edges() {
        if edge.relationship == Relationship::Prerequisite {
            if let (Some(&from_idx), Some(&to_idx)) = (indices.get(&edge.from), indices.get(&edge.to))
            {
                prereq_graph.add_edge(from_idx, to_idx, ());
            }
        }
    }

    // Check for cycles
    if toposort(&prereq_graph, None).is_err() {
        // Find nodes involved in cycles (simplified)
        result.add_error(ValidationIssue::new(
            "PREREQUISITE_CYCLE",
            "Cycle detected in prerequisite relationships. Some concepts have circular dependencies.",
        ));
    }
}

/// Check that variant nodes reference valid canonical nodes.
fn check_canonical_references(graph: &GraphData, result: &mut ValidationResult) {
    let mut invalid_refs: Vec<String> = Vec::new();

    for node in graph.iter_nodes() {
        if !node.is_canonical {
            if let Some(ref canonical_id) = node.canonical_id {
                if !graph.contains_node(canonical_id) {
                    invalid_refs.push(format!(
                        "{} references missing canonical {}",
                        node.id, canonical_id
                    ));
                }
            } else {
                invalid_refs.push(format!(
                    "{} is non-canonical but has no canonical_id",
                    node.id
                ));
            }
        }
    }

    if !invalid_refs.is_empty() {
        result.add_error(
            ValidationIssue::new(
                "INVALID_CANONICAL_REF",
                format!("{} invalid canonical references", invalid_refs.len()),
            )
            .with_nodes(invalid_refs),
        );
    }
}

/// Quick check if graph has any validation errors.
pub fn is_valid(graph: &GraphData) -> bool {
    validate_graph(graph).valid
}
```

### Step 4: Update lib.rs

Update `fabryk-graph/src/lib.rs`:

```rust
pub mod algorithms;
pub mod builder;
pub mod extractor;
pub mod persistence;
pub mod query;
pub mod stats;
pub mod types;
pub mod validation;

// Re-exports - algorithms
pub use algorithms::{
    calculate_centrality, find_bridges, get_related, neighborhood, prerequisites_sorted,
    shortest_path, CentralityScore, NeighborhoodResult, PathResult, PrerequisitesResult,
};

// Re-exports - builder
pub use builder::{BuildError, BuildProgress, BuildResult, ErrorHandling, GraphBuilder, ManualEdge};

// Re-exports - extractor
pub use extractor::GraphExtractor;

// Re-exports - persistence
pub use persistence::{
    is_cache_fresh, load_graph, load_graph_from_str, save_graph, GraphMetadata, SerializableGraph,
};

// Re-exports - query
pub use query::{
    CategoryCount, EdgeInfo, GraphInfoResponse, NeighborInfo, NeighborhoodResponse, NodeSummary,
    PathResponse, PathStep, PrerequisiteInfo, PrerequisitesResponse, RelatedConceptsResponse,
    RelatedGroup, RelationshipCount,
};

// Re-exports - stats
pub use stats::{compute_stats, quick_summary, top_nodes_by_degree, DegreeDirection, GraphStats};

// Re-exports - types
pub use types::{Edge, EdgeOrigin, GraphData, Node, Relationship};

// Re-exports - validation
pub use validation::{is_valid, validate_graph, ValidationIssue, ValidationResult};

#[cfg(any(test, feature = "test-utils"))]
pub use extractor::mock::{MockEdgeData, MockExtractor, MockNodeData};

#[cfg(feature = "rkyv-cache")]
pub use persistence::rkyv_cache;
```

### Step 5: Add tests

Add tests to the respective modules (abbreviated - full tests in the actual files).

### Step 6: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings
```

## Exit Criteria

- [ ] Query response types defined (RelatedConceptsResponse, PathResponse, etc.)
- [ ] `compute_stats()` returns comprehensive GraphStats
- [ ] `validate_graph()` checks for orphans, self-loops, cycles, etc.
- [ ] `ValidationResult` categorizes errors, warnings, and info
- [ ] All types derive Serialize/Deserialize for MCP responses
- [ ] All tests pass

## Commit Message

```
feat(graph): add query, stats, and validation modules

Query module:
- Response types for MCP graph tools
- NodeSummary, PathResponse, NeighborhoodResponse, etc.

Stats module:
- compute_stats() for comprehensive graph analysis
- Category/relationship distribution
- Degree analysis (most depended, most dependencies)

Validation module:
- validate_graph() for structural checks
- Orphan detection, self-loop detection, cycle detection
- Canonical reference validation

Phase 4 milestone 4.6 of Fabryk extraction.

Ref: Doc 0011 §4.4 (supporting modules)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
