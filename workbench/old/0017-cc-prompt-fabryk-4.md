---
title: "CC Prompt: Fabryk 4.1 — Graph Types & Relationship Enum"
milestone: "4.1"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 3 complete", "v0.1-alpha checkpoint passed"]
governing-docs: [0011-audit §4.4, 0012-amendment §2b, 0013-project-plan]
---

# CC Prompt: Fabryk 4.1 — Graph Types & Relationship Enum

## Context

Phase 4 begins the **highest-risk, highest-value** phase of the Fabryk extraction.
The graph subsystem is the most complex part of the music-theory MCP server,
containing ~3,700 lines across 11 files.

This milestone extracts the core graph types to `fabryk-graph`. The critical
decision from Amendment §2b is to use an enum with `Custom(String)` variant for
the `Relationship` type, avoiding generic type parameter infection across the
entire graph infrastructure.

**Pre-requisite check:** The v0.1-alpha checkpoint must pass before starting
Phase 4. Verify:
- fabryk-core, fabryk-content, fabryk-fts compile and pass tests
- Music-theory depends on these crates for content + search
- All 10 non-graph MCP tools work identically

## Objective

Create `fabryk-graph` crate with core graph types:

1. `Node` enum with generic structure
2. `Edge` struct with relationship and weight
3. `Relationship` enum with `Custom(String)` variant (Amendment §2b)
4. `GraphData` for graph storage
5. `EdgeOrigin` enum for edge provenance

## Implementation Steps

### Step 1: Create fabryk-graph crate scaffold

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-graph/src
```

Create `fabryk-graph/Cargo.toml`:

```toml
[package]
name = "fabryk-graph"
version = "0.1.0"
edition = "2021"
description = "Graph database infrastructure for Fabryk domains"
license = "Apache-2.0"
repository = "https://github.com/oxur/ecl"

[dependencies]
fabryk-core = { path = "../fabryk-core" }
fabryk-content = { path = "../fabryk-content" }

# Graph dependencies
petgraph = "0.6"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime
tokio = { version = "1.0", features = ["fs"] }

# Optional: rkyv caching
rkyv = { version = "0.7", optional = true }
blake3 = { version = "1.0", optional = true }

[features]
default = []
rkyv-cache = ["rkyv", "blake3"]

[dev-dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
tempfile = "3.0"
```

### Step 2: Define Relationship enum (Amendment §2b)

Create `fabryk-graph/src/types.rs`:

```rust
//! Core graph types for Fabryk domains.
//!
//! This module provides the fundamental types for building and querying
//! knowledge graphs. The types are designed to be domain-agnostic while
//! supporting domain-specific extensions via the `Custom` variant.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Relationship types for graph edges.
///
/// Common relationships are first-class variants for pattern matching
/// and exhaustive checks. Domain-specific relationships use `Custom(String)`.
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Relationship {
    /// A must be understood before B.
    Prerequisite,
    /// Understanding A naturally leads to B.
    LeadsTo,
    /// A and B are conceptually related.
    RelatesTo,
    /// A extends or generalises B.
    Extends,
    /// Source A introduces concept B.
    Introduces,
    /// Source A covers concept B.
    Covers,
    /// A is a source-specific variant of canonical concept B.
    VariantOf,
    /// Domain-specific relationship not covered above.
    Custom(String),
}

impl Relationship {
    /// Default weight for this relationship type.
    ///
    /// Used when the extractor doesn't specify an explicit weight.
    /// Weights influence pathfinding algorithms like shortest_path.
    pub fn default_weight(&self) -> f32 {
        match self {
            Self::Prerequisite => 1.0,
            Self::LeadsTo => 1.0,
            Self::Extends => 0.9,
            Self::Introduces => 0.8,
            Self::Covers => 0.8,
            Self::VariantOf => 0.9,
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
            Self::Custom(name) => name,
        }
    }
}

impl Default for Relationship {
    fn default() -> Self {
        Self::RelatesTo
    }
}
```

### Step 3: Define Node and Edge types

Continue in `fabryk-graph/src/types.rs`:

```rust
/// Origin of an edge in the graph.
///
/// Tracks where edges came from for debugging and validation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeOrigin {
    /// Extracted from content frontmatter.
    Frontmatter,
    /// Extracted from content body (markdown sections).
    ContentBody,
    /// Loaded from manual_edges.json.
    Manual,
    /// Inferred by an algorithm (e.g., transitive closure).
    Inferred,
}

impl Default for EdgeOrigin {
    fn default() -> Self {
        Self::Frontmatter
    }
}

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
    pub is_canonical: bool,
    /// If not canonical, the ID of the canonical node this relates to.
    pub canonical_id: Option<String>,
    /// Domain-specific metadata as key-value pairs.
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Node {
    /// Creates a new canonical node with the given ID and title.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            category: None,
            source_id: None,
            is_canonical: true,
            canonical_id: None,
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

    /// Adds a metadata key-value pair.
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

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
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        relationship: Relationship,
    ) -> Self {
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
```

### Step 4: Define GraphData

Continue in `fabryk-graph/src/types.rs`:

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Core graph data structure.
///
/// Wraps a petgraph `DiGraph` with lookup tables for efficient access.
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
}

impl Default for GraphData {
    fn default() -> Self {
        Self::new()
    }
}
```

### Step 5: Create lib.rs with exports

Create `fabryk-graph/src/lib.rs`:

```rust
//! Graph database infrastructure for Fabryk domains.
//!
//! This crate provides the core types and algorithms for building and
//! querying knowledge graphs. It is designed to be domain-agnostic,
//! with domain-specific behavior provided via the `GraphExtractor` trait.
//!
//! # Crate Structure
//!
//! - `types` - Core types: `Node`, `Edge`, `Relationship`, `GraphData`
//!
//! # Example
//!
//! ```rust
//! use fabryk_graph::{Node, Edge, Relationship, GraphData};
//!
//! let node = Node::new("pythagorean-theorem", "Pythagorean Theorem")
//!     .with_category("geometry");
//!
//! let edge = Edge::new(
//!     "pythagorean-theorem",
//!     "right-triangle",
//!     Relationship::Prerequisite,
//! );
//! ```

pub mod types;

// Re-exports
pub use types::{Edge, EdgeOrigin, GraphData, Node, Relationship};
```

### Step 6: Add tests

Create `fabryk-graph/src/types.rs` tests at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_default_weights() {
        assert_eq!(Relationship::Prerequisite.default_weight(), 1.0);
        assert_eq!(Relationship::LeadsTo.default_weight(), 1.0);
        assert_eq!(Relationship::Extends.default_weight(), 0.9);
        assert_eq!(Relationship::Introduces.default_weight(), 0.8);
        assert_eq!(Relationship::Covers.default_weight(), 0.8);
        assert_eq!(Relationship::VariantOf.default_weight(), 0.9);
        assert_eq!(Relationship::RelatesTo.default_weight(), 0.7);
        assert_eq!(Relationship::Custom("custom".to_string()).default_weight(), 0.5);
    }

    #[test]
    fn test_relationship_names() {
        assert_eq!(Relationship::Prerequisite.name(), "prerequisite");
        assert_eq!(Relationship::Custom("implies".to_string()).name(), "implies");
    }

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
        assert!(node.metadata.contains_key("key"));
    }

    #[test]
    fn test_node_variant() {
        let variant = Node::new("source-concept", "Source Concept")
            .as_variant_of("canonical-concept");

        assert!(!variant.is_canonical);
        assert_eq!(variant.canonical_id, Some("canonical-concept".to_string()));
    }

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
        assert_eq!(edge.weight, 1.0); // Prerequisite default

        let edge2 = Edge::new("a", "b", Relationship::RelatesTo);
        assert_eq!(edge2.weight, 0.7); // RelatesTo default
    }

    #[test]
    fn test_graph_data_basic() {
        let graph = GraphData::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(!graph.contains_node("test"));
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
    fn test_node_serialization() {
        let node = Node::new("test", "Test")
            .with_category("cat")
            .with_metadata("foo", "bar");

        let json = serde_json::to_string(&node).unwrap();
        let parsed: Node = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, node.id);
        assert_eq!(parsed.title, node.title);
        assert_eq!(parsed.category, node.category);
    }
}
```

### Step 7: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-graph` crate created with proper Cargo.toml
- [ ] `Relationship` enum defined with 7 common variants + `Custom(String)`
- [ ] `Relationship::default_weight()` returns appropriate weights
- [ ] `Node` struct with builder pattern (`with_category`, `with_source`, etc.)
- [ ] `Edge` struct with builder pattern (`with_weight`, `with_origin`)
- [ ] `EdgeOrigin` enum (Frontmatter, ContentBody, Manual, Inferred)
- [ ] `GraphData` struct with petgraph integration
- [ ] All types derive appropriate traits (Clone, Debug, Serialize, Deserialize)
- [ ] `cargo test -p fabryk-graph` passes
- [ ] `cargo clippy -p fabryk-graph -- -D warnings` clean

## Commit Message

```
feat(graph): add fabryk-graph crate with core types

Add fabryk-graph crate with fundamental graph types:
- Relationship enum with Custom(String) variant (Amendment §2b)
- Node struct with builder pattern and metadata support
- Edge struct with weight and origin tracking
- EdgeOrigin enum for edge provenance
- GraphData wrapper around petgraph DiGraph

Phase 4 milestone 4.1 of Fabryk extraction.

Ref: Doc 0012 §2b (Relationship enum decision)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
