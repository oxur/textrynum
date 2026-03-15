//! Graph validation and integrity checking.
//!
//! Provides functions to validate graph structure and detect issues
//! such as orphan nodes, self-loops, duplicate edges, prerequisite
//! cycles, and invalid canonical references.

use crate::{GraphData, Relationship};
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ============================================================================
// Types
// ============================================================================

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
    /// Create a new empty (valid) result.
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }

    /// Add an error (marks graph as invalid).
    pub fn add_error(&mut self, issue: ValidationIssue) {
        self.valid = false;
        self.errors.push(issue);
    }

    /// Add a warning.
    pub fn add_warning(&mut self, issue: ValidationIssue) {
        self.warnings.push(issue);
    }

    /// Add an informational finding.
    pub fn add_info(&mut self, issue: ValidationIssue) {
        self.info.push(issue);
    }

    /// Total issue count (errors + warnings).
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

    /// Attach affected nodes.
    pub fn with_nodes(mut self, nodes: Vec<String>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Attach affected edges.
    pub fn with_edges(mut self, edges: Vec<String>) -> Self {
        self.edges = edges;
        self
    }
}

// ============================================================================
// Validation functions
// ============================================================================

/// Validate a graph for common issues.
///
/// Checks for:
/// - Orphan nodes (no connections)
/// - Self-loops (edge from node to itself)
/// - Duplicate edges (same from, to, relationship)
/// - Prerequisite cycles
/// - Missing canonical references
pub fn validate_graph(graph: &GraphData) -> ValidationResult {
    let mut result = ValidationResult::new();

    check_orphans(graph, &mut result);
    check_self_loops(graph, &mut result);
    check_duplicate_edges(graph, &mut result);
    check_prerequisite_cycles(graph, &mut result);
    check_canonical_references(graph, &mut result);

    result
}

/// Quick check if graph has any validation errors.
pub fn is_valid(graph: &GraphData) -> bool {
    validate_graph(graph).valid
}

// ============================================================================
// Individual checks
// ============================================================================

/// Check for orphan nodes (no incoming or outgoing edges).
fn check_orphans(graph: &GraphData, result: &mut ValidationResult) {
    let orphans: Vec<String> = graph
        .iter_nodes()
        .filter(|node| {
            if let Some(idx) = graph.get_index(&node.id) {
                let in_count = graph.graph.edges_directed(idx, Direction::Incoming).count();
                let out_count = graph.graph.edges_directed(idx, Direction::Outgoing).count();
                in_count == 0 && out_count == 0
            } else {
                false
            }
        })
        .map(|node| node.id.clone())
        .collect();

    if !orphans.is_empty() {
        result.add_warning(
            ValidationIssue::new(
                "ORPHAN_NODES",
                format!("{} node(s) have no connections", orphans.len()),
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
                format!("{} edge(s) are self-loops", self_loops.len()),
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
        if !seen.insert(key) {
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
                format!("{} duplicate edge(s) found", duplicates.len()),
            )
            .with_edges(duplicates),
        );
    }
}

/// Check for cycles in prerequisite relationships.
fn check_prerequisite_cycles(graph: &GraphData, result: &mut ValidationResult) {
    use petgraph::algo::toposort;
    use petgraph::graph::DiGraph;
    use std::collections::HashMap;

    // Build a subgraph with only prerequisite edges
    let mut prereq_graph: DiGraph<String, ()> = DiGraph::new();
    let mut indices: HashMap<String, petgraph::graph::NodeIndex> = HashMap::new();

    for node in graph.iter_nodes() {
        let idx = prereq_graph.add_node(node.id.clone());
        indices.insert(node.id.clone(), idx);
    }

    for edge in graph.iter_edges() {
        if edge.relationship == Relationship::Prerequisite
            && let (Some(&from_idx), Some(&to_idx)) =
                (indices.get(&edge.from), indices.get(&edge.to))
        {
            prereq_graph.add_edge(from_idx, to_idx, ());
        }
    }

    if toposort(&prereq_graph, None).is_err() {
        result.add_error(ValidationIssue::new(
            "PREREQUISITE_CYCLE",
            "Cycle detected in prerequisite relationships",
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
                format!("{} invalid canonical reference(s)", invalid_refs.len()),
            )
            .with_nodes(invalid_refs),
        );
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn create_valid_graph() -> GraphData {
        let mut graph = GraphData::new();

        graph.add_node(Node::new("a", "A").with_category("basics"));
        graph.add_node(Node::new("b", "B").with_category("basics"));
        graph.add_node(Node::new("c", "C").with_category("advanced"));

        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::LeadsTo))
            .unwrap();

        graph
    }

    // ------------------------------------------------------------------------
    // Full validation
    // ------------------------------------------------------------------------

    #[test]
    fn test_validate_valid_graph() {
        let graph = create_valid_graph();
        let result = validate_graph(&graph);

        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_validate_empty_graph() {
        let graph = GraphData::new();
        let result = validate_graph(&graph);

        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_is_valid_helper() {
        let graph = create_valid_graph();
        assert!(is_valid(&graph));
    }

    // ------------------------------------------------------------------------
    // Orphan detection
    // ------------------------------------------------------------------------

    #[test]
    fn test_orphan_detection() {
        let mut graph = create_valid_graph();
        graph.add_node(Node::new("orphan", "Orphan"));

        let result = validate_graph(&graph);

        assert!(result.valid); // orphans are warnings, not errors
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code, "ORPHAN_NODES");
        assert!(result.warnings[0].nodes.contains(&"orphan".to_string()));
    }

    #[test]
    fn test_no_orphans() {
        let graph = create_valid_graph();
        let result = validate_graph(&graph);

        assert!(result.warnings.iter().all(|w| w.code != "ORPHAN_NODES"));
    }

    // ------------------------------------------------------------------------
    // Self-loop detection
    // ------------------------------------------------------------------------

    #[test]
    fn test_self_loop_detection() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        graph.add_node(Node::new("b", "B"));

        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "a", Relationship::RelatesTo))
            .unwrap();

        let result = validate_graph(&graph);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "SELF_LOOPS"));
        assert_eq!(
            result
                .errors
                .iter()
                .find(|e| e.code == "SELF_LOOPS")
                .unwrap()
                .edges
                .len(),
            1
        );
    }

    #[test]
    fn test_no_self_loops() {
        let graph = create_valid_graph();
        let result = validate_graph(&graph);

        assert!(!result.errors.iter().any(|e| e.code == "SELF_LOOPS"));
    }

    // ------------------------------------------------------------------------
    // Duplicate edge detection
    // ------------------------------------------------------------------------

    #[test]
    fn test_duplicate_edge_detection() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        graph.add_node(Node::new("b", "B"));

        // Add same edge twice
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();

        let result = validate_graph(&graph);

        assert!(result.warnings.iter().any(|w| w.code == "DUPLICATE_EDGES"));
    }

    #[test]
    fn test_different_relationship_not_duplicate() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        graph.add_node(Node::new("b", "B"));

        // Same nodes, different relationships — not duplicates
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("a", "b", Relationship::RelatesTo))
            .unwrap();

        let result = validate_graph(&graph);

        assert!(!result.warnings.iter().any(|w| w.code == "DUPLICATE_EDGES"));
    }

    // ------------------------------------------------------------------------
    // Prerequisite cycle detection
    // ------------------------------------------------------------------------

    #[test]
    fn test_prerequisite_cycle_detection() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        graph.add_node(Node::new("b", "B"));

        // a -> b and b -> a (cycle)
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "a", Relationship::Prerequisite))
            .unwrap();

        let result = validate_graph(&graph);

        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "PREREQUISITE_CYCLE"));
    }

    #[test]
    fn test_non_prerequisite_cycle_ok() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "A"));
        graph.add_node(Node::new("b", "B"));

        // Cycle in RelatesTo is fine (only Prerequisite cycles are errors)
        graph
            .add_edge(Edge::new("a", "b", Relationship::RelatesTo))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "a", Relationship::RelatesTo))
            .unwrap();

        let result = validate_graph(&graph);

        assert!(!result.errors.iter().any(|e| e.code == "PREREQUISITE_CYCLE"));
    }

    #[test]
    fn test_no_prerequisite_cycles() {
        let graph = create_valid_graph();
        let result = validate_graph(&graph);

        assert!(!result.errors.iter().any(|e| e.code == "PREREQUISITE_CYCLE"));
    }

    // ------------------------------------------------------------------------
    // Canonical reference validation
    // ------------------------------------------------------------------------

    #[test]
    fn test_valid_canonical_reference() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("canonical", "Canonical Concept"));
        graph.add_node(Node::new("variant", "Variant").as_variant_of("canonical"));

        let result = validate_graph(&graph);

        assert!(
            !result
                .errors
                .iter()
                .any(|e| e.code == "INVALID_CANONICAL_REF")
        );
    }

    #[test]
    fn test_missing_canonical_reference() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("variant", "Variant").as_variant_of("missing-canonical"));

        let result = validate_graph(&graph);

        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.code == "INVALID_CANONICAL_REF")
        );
    }

    #[test]
    fn test_non_canonical_without_canonical_id() {
        let mut graph = GraphData::new();
        let mut node = Node::new("bad", "Bad Node");
        node.is_canonical = false;
        node.canonical_id = None;
        graph.add_node(node);

        let result = validate_graph(&graph);

        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.code == "INVALID_CANONICAL_REF")
        );
    }

    // ------------------------------------------------------------------------
    // ValidationResult API
    // ------------------------------------------------------------------------

    #[test]
    fn test_validation_result_new() {
        let result = ValidationResult::new();

        assert!(result.valid);
        assert!(result.errors.is_empty());
        assert!(result.warnings.is_empty());
        assert!(result.info.is_empty());
        assert_eq!(result.total_issues(), 0);
    }

    #[test]
    fn test_validation_result_add_error() {
        let mut result = ValidationResult::new();
        result.add_error(ValidationIssue::new("TEST", "test error"));

        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.total_issues(), 1);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut result = ValidationResult::new();
        result.add_warning(ValidationIssue::new("TEST", "test warning"));

        assert!(result.valid); // warnings don't invalidate
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.total_issues(), 1);
    }

    #[test]
    fn test_validation_result_add_info() {
        let mut result = ValidationResult::new();
        result.add_info(ValidationIssue::new("TEST", "test info"));

        assert!(result.valid);
        assert_eq!(result.info.len(), 1);
        assert_eq!(result.total_issues(), 0); // info not counted
    }

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.valid);
    }

    // ------------------------------------------------------------------------
    // ValidationIssue API
    // ------------------------------------------------------------------------

    #[test]
    fn test_validation_issue_builder() {
        let issue = ValidationIssue::new("CODE", "message")
            .with_nodes(vec!["a".to_string(), "b".to_string()])
            .with_edges(vec!["a -> b".to_string()]);

        assert_eq!(issue.code, "CODE");
        assert_eq!(issue.message, "message");
        assert_eq!(issue.nodes.len(), 2);
        assert_eq!(issue.edges.len(), 1);
    }

    #[test]
    fn test_validation_issue_serialization() {
        let issue =
            ValidationIssue::new("TEST", "test message").with_nodes(vec!["node1".to_string()]);

        let json = serde_json::to_string(&issue).unwrap();
        let parsed: ValidationIssue = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.code, "TEST");
        assert_eq!(parsed.nodes.len(), 1);
    }

    #[test]
    fn test_validation_result_serialization() {
        let mut result = ValidationResult::new();
        result.add_error(ValidationIssue::new("ERR", "error"));
        result.add_warning(ValidationIssue::new("WARN", "warning"));

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ValidationResult = serde_json::from_str(&json).unwrap();

        assert!(!parsed.valid);
        assert_eq!(parsed.errors.len(), 1);
        assert_eq!(parsed.warnings.len(), 1);
    }
}
