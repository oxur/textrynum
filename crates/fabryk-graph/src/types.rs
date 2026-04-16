//! Core graph types for Fabryk domains.
//!
//! This module provides the fundamental types for building and querying
//! knowledge graphs. The types are designed to be domain-agnostic while
//! supporting domain-specific extensions via the `Custom` variant.

use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

// ============================================================================
// Relationship enum
// ============================================================================

/// Relationship types for graph edges.
///
/// Common relationships are first-class variants for pattern matching
/// and exhaustive checks. Domain-specific relationships use `Custom(String)`.
///
/// Per Amendment §2b, this enum-with-Custom approach avoids generic type
/// parameter infection across the entire graph infrastructure.
///
/// # Example
///
/// ```rust
/// use fabryk_graph::Relationship;
///
/// let prereq = Relationship::Prerequisite;
/// assert_eq!(prereq.default_weight(), 1.0);
///
/// let custom = Relationship::Custom("implies".to_string());
/// assert_eq!(custom.default_weight(), 0.5);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Relationship {
    /// A must be understood before B.
    #[serde(alias = "prerequisite")]
    Prerequisite,
    /// Understanding A naturally leads to B.
    #[serde(alias = "leads_to")]
    LeadsTo,
    /// A and B are conceptually related.
    #[default]
    #[serde(alias = "relates_to")]
    RelatesTo,
    /// A extends or generalises B.
    #[serde(alias = "extends")]
    Extends,
    /// Source A introduces concept B.
    #[serde(alias = "introduces")]
    Introduces,
    /// Source A covers concept B.
    #[serde(alias = "covers")]
    Covers,
    /// A is a source-specific variant of canonical concept B.
    #[serde(alias = "variant_of")]
    VariantOf,
    /// A contrasts with or is an alternative to B.
    #[serde(alias = "contrasts_with")]
    ContrastsWith,
    /// A answers or addresses question B.
    #[serde(alias = "answers_question")]
    AnswersQuestion,
    /// A is the same concept as B (identity / equivalence).
    #[serde(alias = "same_as")]
    SameAs,
    /// A cites or references B as a source.
    #[serde(alias = "cites")]
    Cites,
    /// Domain-specific relationship not covered above.
    Custom(String),
}

impl Relationship {
    /// Default weight for this relationship type.
    ///
    /// Used when the extractor doesn't specify an explicit weight.
    /// Weights influence pathfinding algorithms like `shortest_path`.
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::Prerequisite => 1.0,
            Self::LeadsTo => 1.0,
            Self::Extends => 0.9,
            Self::Introduces => 0.8,
            Self::Covers => 0.8,
            Self::VariantOf => 0.9,
            Self::ContrastsWith => 0.7,
            Self::AnswersQuestion => 0.6,
            Self::SameAs => 0.9,
            Self::Cites => 0.7,
            Self::RelatesTo => 0.7,
            Self::Custom(_) => 0.5,
        }
    }

    /// Returns the relationship name as a string.
    pub fn name(&self) -> &str {
        match self {
            Self::Prerequisite => "prerequisite",
            Self::LeadsTo => "leads_to",
            Self::RelatesTo => "relates_to",
            Self::Extends => "extends",
            Self::Introduces => "introduces",
            Self::Covers => "covers",
            Self::VariantOf => "variant_of",
            Self::ContrastsWith => "contrasts_with",
            Self::AnswersQuestion => "answers_question",
            Self::SameAs => "same_as",
            Self::Cites => "cites",
            Self::Custom(name) => name,
        }
    }
}

impl fmt::Display for Relationship {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Prerequisite => write!(f, "Prerequisite"),
            Self::LeadsTo => write!(f, "LeadsTo"),
            Self::RelatesTo => write!(f, "RelatesTo"),
            Self::Extends => write!(f, "Extends"),
            Self::Introduces => write!(f, "Introduces"),
            Self::Covers => write!(f, "Covers"),
            Self::VariantOf => write!(f, "VariantOf"),
            Self::ContrastsWith => write!(f, "ContrastsWith"),
            Self::AnswersQuestion => write!(f, "AnswersQuestion"),
            Self::SameAs => write!(f, "SameAs"),
            Self::Cites => write!(f, "Cites"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

impl FromStr for Relationship {
    type Err = std::convert::Infallible;

    /// Parse a relationship name string.
    ///
    /// Accepts PascalCase (`"Prerequisite"`), snake_case (`"relates_to"`),
    /// and case-insensitive matching. Unrecognized strings become
    /// `Custom(original_string)` rather than an error.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().replace('_', "").as_str() {
            "prerequisite" => Self::Prerequisite,
            "leadsto" => Self::LeadsTo,
            "relatesto" => Self::RelatesTo,
            "extends" => Self::Extends,
            "introduces" => Self::Introduces,
            "covers" => Self::Covers,
            "variantof" => Self::VariantOf,
            "contrastswith" => Self::ContrastsWith,
            "answersquestion" => Self::AnswersQuestion,
            "sameas" | "same_as" => Self::SameAs,
            "cites" => Self::Cites,
            _ => Self::Custom(s.to_string()),
        })
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain => write!(f, "domain"),
            Self::UserQuery => write!(f, "user_query"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

// ============================================================================
// EdgeOrigin enum
// ============================================================================

/// Origin of an edge in the graph.
///
/// Tracks where edges came from for debugging and validation.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeOrigin {
    /// Extracted from content frontmatter.
    #[default]
    #[serde(alias = "extracted", alias = "frontmatter")]
    Frontmatter,
    /// Extracted from content body (markdown sections).
    #[serde(alias = "content_body")]
    ContentBody,
    /// Loaded from manual_edges.json.
    #[serde(alias = "manual")]
    Manual,
    /// Inferred by an algorithm (e.g., transitive closure).
    #[serde(alias = "inferred")]
    Inferred,
}

// ============================================================================
// NodeType enum (adapted from Taproot)
// ============================================================================

/// Type of a graph node.
///
/// Distinguishes between domain knowledge nodes and user-model nodes,
/// enabling user integration features like "unexplored concepts".
///
/// Adapted from Taproot's `NodeType` for the generic Fabryk context.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// A domain concept node (the default).
    #[default]
    Domain,
    /// A user-query or user-interaction node.
    UserQuery,
    /// Domain-specific node type.
    Custom(String),
}

// ============================================================================
// Node struct
// ============================================================================

/// A node in the knowledge graph.
///
/// Nodes represent content items (concepts, theorems, chapters, etc.).
/// The `metadata` field stores domain-specific attributes as key-value pairs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier (e.g., "picardy-third", "group-theory").
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Optional category for grouping (e.g., "harmony", "algebra").
    pub category: Option<String>,
    /// Optional source identifier (e.g., "tymoczko", "dummit-foote").
    pub source_id: Option<String>,
    /// Whether this is a canonical node (vs. source-specific variant).
    #[serde(default = "default_is_canonical")]
    pub is_canonical: bool,
    /// If not canonical, the ID of the canonical node this relates to.
    pub canonical_id: Option<String>,
    /// Type of this node (Domain, UserQuery, Custom).
    #[serde(default)]
    pub node_type: NodeType,
    /// Domain-specific metadata as key-value pairs.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_is_canonical() -> bool {
    true
}

impl Node {
    /// Creates a new canonical domain node with the given ID and title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: None,
            source_id: None,
            is_canonical: true,
            canonical_id: None,
            node_type: NodeType::default(),
            metadata: HashMap::new(),
        }
    }

    /// Sets the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Sets the source ID.
    pub fn with_source(mut self, source_id: impl Into<String>) -> Self {
        self.source_id = Some(source_id.into());
        self
    }

    /// Marks this as a variant of a canonical node.
    pub fn as_variant_of(mut self, canonical_id: impl Into<String>) -> Self {
        self.is_canonical = false;
        self.canonical_id = Some(canonical_id.into());
        self
    }

    /// Sets the node type.
    pub fn with_node_type(mut self, node_type: NodeType) -> Self {
        self.node_type = node_type;
        self
    }

    /// Adds a metadata key-value pair.
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    // ---- Node inspection helpers ----

    /// Check if this is a domain-type node (the default node type).
    pub fn is_domain(&self) -> bool {
        matches!(self.node_type, NodeType::Domain)
    }

    /// Check if this is a custom-type node with the given type name.
    pub fn is_custom_type(&self, type_name: &str) -> bool {
        matches!(&self.node_type, NodeType::Custom(s) if s == type_name)
    }

    /// Get the category, defaulting to `"unknown"` if not set.
    pub fn category_or_default(&self) -> &str {
        self.category.as_deref().unwrap_or("unknown")
    }

    /// Get a string value from metadata.
    pub fn metadata_str(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(|v| v.as_str())
    }

    /// Get a u64 value from metadata.
    pub fn metadata_u64(&self, key: &str) -> Option<u64> {
        self.metadata.get(key).and_then(|v| v.as_u64())
    }

    /// Get a bool value from metadata.
    pub fn metadata_bool(&self, key: &str) -> Option<bool> {
        self.metadata.get(key).and_then(|v| v.as_bool())
    }
}

// ============================================================================
// Edge struct
// ============================================================================

/// An edge connecting two nodes in the graph.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source node ID.
    pub from: String,
    /// Target node ID.
    pub to: String,
    /// Type of relationship.
    pub relationship: Relationship,
    /// Edge weight (influences pathfinding).
    pub weight: f32,
    /// Where this edge originated.
    pub origin: EdgeOrigin,
}

impl Edge {
    /// Creates a new edge with default weight from relationship type.
    pub fn new(from: impl Into<String>, to: impl Into<String>, relationship: Relationship) -> Self {
        let weight = relationship.default_weight();
        Self {
            from: from.into(),
            to: to.into(),
            relationship,
            weight,
            origin: EdgeOrigin::default(),
        }
    }

    /// Sets an explicit weight.
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }

    /// Sets the edge origin.
    pub fn with_origin(mut self, origin: EdgeOrigin) -> Self {
        self.origin = origin;
        self
    }
}

// ============================================================================
// GraphData struct
// ============================================================================

/// Core graph data structure.
///
/// Wraps a petgraph `DiGraph` with lookup tables for efficient access.
/// Supports runtime mutation (add/remove nodes and edges) without full rebuild.
#[derive(Clone, Debug)]
pub struct GraphData {
    /// The underlying directed graph.
    pub graph: DiGraph<Node, Edge>,
    /// Lookup table: node ID → petgraph NodeIndex.
    pub node_indices: HashMap<String, NodeIndex>,
    /// Lookup table: node ID → Node data.
    pub nodes: HashMap<String, Node>,
    /// All edges as a flat list (for serialization).
    pub edges: Vec<Edge>,
}

impl GraphData {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Returns the number of nodes.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the number of edges.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Gets a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Gets the petgraph NodeIndex for a node ID.
    pub fn get_index(&self, id: &str) -> Option<NodeIndex> {
        self.node_indices.get(id).copied()
    }

    /// Checks if a node exists.
    pub fn contains_node(&self, id: &str) -> bool {
        self.nodes.contains_key(id)
    }

    /// Returns an iterator over all node IDs.
    pub fn node_ids(&self) -> impl Iterator<Item = &str> {
        self.nodes.keys().map(String::as_str)
    }

    /// Returns an iterator over all nodes.
    pub fn iter_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    /// Returns an iterator over all edges.
    pub fn iter_edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    // ========================================================================
    // Runtime mutation API (adapted from Taproot)
    // ========================================================================

    /// Add a node to the graph at runtime.
    ///
    /// Returns the `NodeIndex` for the newly added node.
    /// If a node with the same ID already exists, returns its existing index.
    pub fn add_node(&mut self, node: Node) -> NodeIndex {
        if let Some(&existing_idx) = self.node_indices.get(&node.id) {
            return existing_idx;
        }
        let id = node.id.clone();
        let idx = self.graph.add_node(node.clone());
        self.node_indices.insert(id.clone(), idx);
        self.nodes.insert(id, node);
        idx
    }

    /// Add an edge between two nodes identified by ID.
    ///
    /// Both nodes must already exist in the graph.
    /// Returns `Ok(())` on success, or an error if a node is missing.
    pub fn add_edge(&mut self, edge: Edge) -> fabryk_core::Result<()> {
        let from_idx = self
            .node_indices
            .get(&edge.from)
            .copied()
            .ok_or_else(|| fabryk_core::Error::not_found("node", &edge.from))?;
        let to_idx = self
            .node_indices
            .get(&edge.to)
            .copied()
            .ok_or_else(|| fabryk_core::Error::not_found("node", &edge.to))?;

        self.graph.add_edge(from_idx, to_idx, edge.clone());
        self.edges.push(edge);
        Ok(())
    }

    /// Remove a node and all its connected edges.
    ///
    /// Returns the removed node, or `None` if the node didn't exist.
    pub fn remove_node(&mut self, id: &str) -> Option<Node> {
        let idx = self.node_indices.remove(id)?;
        let node = self.nodes.remove(id)?;

        // Remove from petgraph (this also removes connected edges)
        self.graph.remove_node(idx);

        // Remove edges from the flat list
        self.edges.retain(|e| e.from != id && e.to != id);

        // Rebuild node_indices since petgraph may have shifted indices
        // after remove_node (petgraph swaps the last node into the removed slot)
        self.node_indices.clear();
        for ni in self.graph.node_indices() {
            let n = &self.graph[ni];
            self.node_indices.insert(n.id.clone(), ni);
        }

        Some(node)
    }
}

impl Default for GraphData {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// LoadedGraph — graph data with load-time metadata
// ============================================================================

/// A loaded graph with metadata about when it was loaded and summary statistics.
///
/// Used by application state to hold the active graph instance alongside
/// its load timestamp and precomputed statistics.
#[derive(Clone, Debug)]
pub struct LoadedGraph {
    /// The underlying graph data.
    pub data: GraphData,
    /// When the graph was loaded.
    pub loaded_at: chrono::DateTime<chrono::Utc>,
    /// Precomputed statistics.
    pub stats: crate::stats::GraphStats,
}

impl LoadedGraph {
    /// Create a new LoadedGraph with stats computed from the data.
    pub fn new(data: GraphData) -> Self {
        let stats = crate::stats::compute_stats(&data);
        Self {
            data,
            loaded_at: chrono::Utc::now(),
            stats,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------------
    // Relationship tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_relationship_default_weights() {
        assert_eq!(Relationship::Prerequisite.default_weight(), 1.0);
        assert_eq!(Relationship::LeadsTo.default_weight(), 1.0);
        assert_eq!(Relationship::Extends.default_weight(), 0.9);
        assert_eq!(Relationship::Introduces.default_weight(), 0.8);
        assert_eq!(Relationship::Covers.default_weight(), 0.8);
        assert_eq!(Relationship::VariantOf.default_weight(), 0.9);
        assert_eq!(Relationship::RelatesTo.default_weight(), 0.7);
        assert_eq!(
            Relationship::Custom("custom".to_string()).default_weight(),
            0.5
        );
    }

    #[test]
    fn test_relationship_names() {
        assert_eq!(Relationship::Prerequisite.name(), "prerequisite");
        assert_eq!(Relationship::LeadsTo.name(), "leads_to");
        assert_eq!(Relationship::RelatesTo.name(), "relates_to");
        assert_eq!(Relationship::Extends.name(), "extends");
        assert_eq!(Relationship::Introduces.name(), "introduces");
        assert_eq!(Relationship::Covers.name(), "covers");
        assert_eq!(Relationship::VariantOf.name(), "variant_of");
        assert_eq!(
            Relationship::Custom("implies".to_string()).name(),
            "implies"
        );
    }

    #[test]
    fn test_relationship_default() {
        assert_eq!(Relationship::default(), Relationship::RelatesTo);
    }

    #[test]
    fn test_relationship_serialization() {
        let rel = Relationship::Custom("implies".to_string());
        let json = serde_json::to_string(&rel).unwrap();
        assert!(json.contains("implies"));

        let parsed: Relationship = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, rel);
    }

    #[test]
    fn test_relationship_all_variants_serialize() {
        let variants = vec![
            Relationship::Prerequisite,
            Relationship::LeadsTo,
            Relationship::RelatesTo,
            Relationship::Extends,
            Relationship::Introduces,
            Relationship::Covers,
            Relationship::VariantOf,
            Relationship::Custom("test".to_string()),
        ];

        for rel in variants {
            let json = serde_json::to_string(&rel).unwrap();
            let parsed: Relationship = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, rel);
        }
    }

    // ------------------------------------------------------------------------
    // EdgeOrigin tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_edge_origin_default() {
        assert_eq!(EdgeOrigin::default(), EdgeOrigin::Frontmatter);
    }

    #[test]
    fn test_edge_origin_serialization() {
        let origins = vec![
            EdgeOrigin::Frontmatter,
            EdgeOrigin::ContentBody,
            EdgeOrigin::Manual,
            EdgeOrigin::Inferred,
        ];

        for origin in origins {
            let json = serde_json::to_string(&origin).unwrap();
            let parsed: EdgeOrigin = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, origin);
        }
    }

    // ------------------------------------------------------------------------
    // NodeType tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_node_type_default() {
        assert_eq!(NodeType::default(), NodeType::Domain);
    }

    #[test]
    fn test_node_type_serialization() {
        let types = vec![
            NodeType::Domain,
            NodeType::UserQuery,
            NodeType::Custom("special".to_string()),
        ];

        for nt in types {
            let json = serde_json::to_string(&nt).unwrap();
            let parsed: NodeType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, nt);
        }
    }

    #[test]
    fn test_node_type_rename_all() {
        let json = serde_json::to_string(&NodeType::UserQuery).unwrap();
        assert_eq!(json, "\"user_query\"");

        let json = serde_json::to_string(&NodeType::Domain).unwrap();
        assert_eq!(json, "\"domain\"");
    }

    // ------------------------------------------------------------------------
    // Node tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_node_builder() {
        let node = Node::new("test-id", "Test Title")
            .with_category("test-category")
            .with_source("test-source")
            .with_metadata("key", "value");

        assert_eq!(node.id, "test-id");
        assert_eq!(node.title, "Test Title");
        assert_eq!(node.category, Some("test-category".to_string()));
        assert_eq!(node.source_id, Some("test-source".to_string()));
        assert!(node.is_canonical);
        assert!(node.canonical_id.is_none());
        assert_eq!(node.node_type, NodeType::Domain);
        assert!(node.metadata.contains_key("key"));
    }

    #[test]
    fn test_node_variant() {
        let variant =
            Node::new("source-concept", "Source Concept").as_variant_of("canonical-concept");

        assert!(!variant.is_canonical);
        assert_eq!(variant.canonical_id, Some("canonical-concept".to_string()));
    }

    #[test]
    fn test_node_with_node_type() {
        let node = Node::new("query-1", "User Query").with_node_type(NodeType::UserQuery);

        assert_eq!(node.node_type, NodeType::UserQuery);
    }

    #[test]
    fn test_node_serialization() {
        let node = Node::new("test", "Test")
            .with_category("cat")
            .with_node_type(NodeType::UserQuery)
            .with_metadata("foo", "bar");

        let json = serde_json::to_string(&node).unwrap();
        let parsed: Node = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, node.id);
        assert_eq!(parsed.title, node.title);
        assert_eq!(parsed.category, node.category);
        assert_eq!(parsed.node_type, node.node_type);
    }

    // ------------------------------------------------------------------------
    // Edge tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_edge_builder() {
        let edge = Edge::new("a", "b", Relationship::Prerequisite)
            .with_weight(0.8)
            .with_origin(EdgeOrigin::Manual);

        assert_eq!(edge.from, "a");
        assert_eq!(edge.to, "b");
        assert_eq!(edge.weight, 0.8);
        assert_eq!(edge.origin, EdgeOrigin::Manual);
    }

    #[test]
    fn test_edge_default_weight() {
        let edge = Edge::new("a", "b", Relationship::Prerequisite);
        assert_eq!(edge.weight, 1.0);

        let edge2 = Edge::new("a", "b", Relationship::RelatesTo);
        assert_eq!(edge2.weight, 0.7);
    }

    #[test]
    fn test_edge_default_origin() {
        let edge = Edge::new("a", "b", Relationship::Prerequisite);
        assert_eq!(edge.origin, EdgeOrigin::Frontmatter);
    }

    #[test]
    fn test_edge_serialization() {
        let edge = Edge::new("a", "b", Relationship::LeadsTo)
            .with_weight(0.5)
            .with_origin(EdgeOrigin::Manual);

        let json = serde_json::to_string(&edge).unwrap();
        let parsed: Edge = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.from, edge.from);
        assert_eq!(parsed.to, edge.to);
        assert_eq!(parsed.relationship, edge.relationship);
        assert_eq!(parsed.weight, edge.weight);
        assert_eq!(parsed.origin, edge.origin);
    }

    // ------------------------------------------------------------------------
    // GraphData basic tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_graph_data_new() {
        let graph = GraphData::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.contains_node("test"));
    }

    #[test]
    fn test_graph_data_default() {
        let graph = GraphData::default();
        assert_eq!(graph.node_count(), 0);
    }

    #[test]
    fn test_graph_data_iterators_empty() {
        let graph = GraphData::new();
        assert_eq!(graph.node_ids().count(), 0);
        assert_eq!(graph.iter_nodes().count(), 0);
        assert_eq!(graph.iter_edges().count(), 0);
    }

    // ------------------------------------------------------------------------
    // GraphData mutation API tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_graph_data_add_node() {
        let mut graph = GraphData::new();
        let node = Node::new("a", "Node A");
        let idx = graph.add_node(node);

        assert_eq!(graph.node_count(), 1);
        assert!(graph.contains_node("a"));
        assert_eq!(graph.get_index("a"), Some(idx));
        assert_eq!(graph.get_node("a").unwrap().title, "Node A");
    }

    #[test]
    fn test_graph_data_add_node_duplicate() {
        let mut graph = GraphData::new();
        let idx1 = graph.add_node(Node::new("a", "Node A"));
        let idx2 = graph.add_node(Node::new("a", "Node A Again"));

        // Should return existing index, not create a duplicate
        assert_eq!(idx1, idx2);
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_graph_data_add_edge() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));
        graph.add_node(Node::new("b", "Node B"));

        let edge = Edge::new("a", "b", Relationship::Prerequisite);
        graph.add_edge(edge).unwrap();

        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn test_graph_data_add_edge_missing_from() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("b", "Node B"));

        let edge = Edge::new("missing", "b", Relationship::Prerequisite);
        let result = graph.add_edge(edge);

        assert!(result.is_err());
    }

    #[test]
    fn test_graph_data_add_edge_missing_to() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));

        let edge = Edge::new("a", "missing", Relationship::Prerequisite);
        let result = graph.add_edge(edge);

        assert!(result.is_err());
    }

    #[test]
    fn test_graph_data_remove_node() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));
        graph.add_node(Node::new("b", "Node B"));
        graph.add_node(Node::new("c", "Node C"));
        graph
            .add_edge(Edge::new("a", "b", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("b", "c", Relationship::LeadsTo))
            .unwrap();

        let removed = graph.remove_node("b");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().id, "b");

        assert_eq!(graph.node_count(), 2);
        assert!(!graph.contains_node("b"));
        assert!(graph.contains_node("a"));
        assert!(graph.contains_node("c"));
        assert_eq!(graph.edge_count(), 0); // All edges involving b removed
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_graph_data_remove_nonexistent_node() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));

        let removed = graph.remove_node("nonexistent");
        assert!(removed.is_none());
        assert_eq!(graph.node_count(), 1);
    }

    #[test]
    fn test_graph_data_remove_node_preserves_indices() {
        let mut graph = GraphData::new();
        graph.add_node(Node::new("a", "Node A"));
        graph.add_node(Node::new("b", "Node B"));
        graph.add_node(Node::new("c", "Node C"));

        graph.remove_node("a");

        // Remaining nodes should still be accessible
        assert!(graph.contains_node("b"));
        assert!(graph.contains_node("c"));
        assert!(graph.get_index("b").is_some());
        assert!(graph.get_index("c").is_some());
    }

    #[test]
    fn test_graph_data_full_workflow() {
        let mut graph = GraphData::new();

        // Add nodes
        graph.add_node(Node::new("intervals", "Intervals").with_category("basics"));
        graph.add_node(Node::new("scales", "Scales").with_category("basics"));
        graph.add_node(Node::new("chords", "Chords").with_category("harmony"));

        // Add edges
        graph
            .add_edge(Edge::new("intervals", "scales", Relationship::Prerequisite))
            .unwrap();
        graph
            .add_edge(Edge::new("scales", "chords", Relationship::LeadsTo))
            .unwrap();

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        // Query
        let intervals = graph.get_node("intervals").unwrap();
        assert_eq!(intervals.category, Some("basics".to_string()));

        // Mutate: add a new node and edge
        graph.add_node(Node::new("query-1", "User Query").with_node_type(NodeType::UserQuery));
        graph
            .add_edge(Edge::new(
                "query-1",
                "chords",
                Relationship::Custom("queries_about".to_string()),
            ))
            .unwrap();

        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 3);

        // Remove the user query node
        graph.remove_node("query-1");
        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }
}
