---
title: "CC Prompt: Fabryk 4.2 — GraphExtractor Trait"
milestone: "4.2"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1 Graph types complete"]
governing-docs: [0011-audit §3 §9, 0012-amendment §2c, 0013-project-plan]
---

# CC Prompt: Fabryk 4.2 — GraphExtractor Trait

## Context

The `GraphExtractor` trait is the **critical abstraction** that enables Fabryk
to support multiple knowledge domains. It defines how domain-specific content
(frontmatter + markdown body) is transformed into generic graph nodes and edges.

Per Amendment §2c, `MetadataExtractor` was deferred — the `GraphExtractor::NodeData`
associated type serves as the metadata carrier for domain-specific fields.

This trait is the linchpin for Phase 4. Getting it right is essential.

## Objective

Define the `GraphExtractor` trait in `fabryk-graph::extractor` with:

1. Associated types for domain-specific node and edge data
2. Methods for extracting node/edge data from content
3. Methods for converting domain data to generic graph types
4. A mock implementation for testing

## Trait Design Rationale

The trait separates concerns:

1. **Extraction** (`extract_node`, `extract_edges`): Domain knows how to parse
   its frontmatter fields (e.g., `category`, `source`, `prerequisites`)
2. **Conversion** (`to_graph_node`, `to_graph_edges`): Domain knows how to map
   its structures to generic `Node` and `Edge` types

This separation allows the `GraphBuilder` to remain generic — it calls the trait
methods without knowing anything about music theory, math, or any other domain.

## Implementation Steps

### Step 1: Create extractor module

Create `fabryk-graph/src/extractor.rs`:

```rust
//! GraphExtractor trait for domain-specific graph extraction.
//!
//! This module defines the core abstraction that enables Fabryk to support
//! multiple knowledge domains. Each domain implements `GraphExtractor` to
//! define how its content is transformed into graph nodes and edges.
//!
//! # Design Philosophy
//!
//! The trait separates extraction (parsing) from conversion (mapping):
//!
//! - `extract_node()` / `extract_edges()`: Parse domain-specific data
//! - `to_graph_node()` / `to_graph_edges()`: Convert to generic types
//!
//! This separation keeps `GraphBuilder` domain-agnostic while allowing
//! full customization of content interpretation.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_graph::{GraphExtractor, Node, Edge, Relationship};
//!
//! struct MyExtractor;
//!
//! impl GraphExtractor for MyExtractor {
//!     type NodeData = MyNodeData;
//!     type EdgeData = MyEdgeData;
//!
//!     fn extract_node(&self, ...) -> Result<Self::NodeData> { ... }
//!     fn extract_edges(&self, ...) -> Result<Option<Self::EdgeData>> { ... }
//!     fn to_graph_node(&self, data: &Self::NodeData) -> Node { ... }
//!     fn to_graph_edges(&self, from_id: &str, data: &Self::EdgeData) -> Vec<Edge> { ... }
//! }
//! ```

use crate::{Edge, Node};
use fabryk_core::Result;
use std::path::Path;

/// Trait for extracting graph data from domain-specific content.
///
/// Each knowledge domain (music theory, math, etc.) implements this trait
/// to define how its markdown files with frontmatter are transformed into
/// graph nodes and edges.
///
/// # Associated Types
///
/// - `NodeData`: Domain-specific node information (e.g., `ConceptCard`)
/// - `EdgeData`: Domain-specific relationship information (e.g., `RelatedConcepts`)
///
/// # Lifecycle
///
/// For each content file, `GraphBuilder` calls:
///
/// 1. `extract_node()` - Parse frontmatter + content into `NodeData`
/// 2. `extract_edges()` - Parse relationship data into `EdgeData`
/// 3. `to_graph_node()` - Convert `NodeData` to generic `Node`
/// 4. `to_graph_edges()` - Convert `EdgeData` to generic `Vec<Edge>`
///
/// # Error Handling
///
/// Extraction methods return `Result` to handle parsing failures gracefully.
/// The builder can be configured to skip invalid files or fail fast.
pub trait GraphExtractor: Send + Sync {
    /// Domain-specific node data extracted from content.
    ///
    /// This type carries all the domain-specific fields needed for:
    /// - Building the graph node
    /// - Populating search documents
    /// - Serving via MCP tools
    ///
    /// Example for music theory: `ConceptCard` with fields like
    /// `id`, `title`, `category`, `source`, `description`.
    type NodeData: Clone + Send + Sync;

    /// Domain-specific edge/relationship data extracted from content.
    ///
    /// This type carries relationship information that will be converted
    /// to graph edges.
    ///
    /// Example for music theory: `RelatedConcepts` with fields like
    /// `prerequisites`, `leads_to`, `see_also`.
    type EdgeData: Clone + Send + Sync;

    /// Extract node data from a content file.
    ///
    /// Called by `GraphBuilder` for each markdown file discovered.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for content (e.g., `/data/concepts`)
    /// * `file_path` - Full path to the file being processed
    /// * `frontmatter` - Parsed YAML frontmatter as generic Value
    /// * `content` - Markdown body (after frontmatter)
    ///
    /// # Returns
    ///
    /// Domain-specific node data, or an error if extraction fails.
    ///
    /// # Implementation Notes
    ///
    /// Use `fabryk_core::util::ids::id_from_path()` to compute the node ID.
    /// Parse frontmatter fields using serde's `from_value()` or manual access.
    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Self::NodeData>;

    /// Extract relationship/edge data from content.
    ///
    /// Called after `extract_node()` succeeds.
    ///
    /// # Arguments
    ///
    /// * `frontmatter` - Parsed YAML frontmatter
    /// * `content` - Markdown body
    ///
    /// # Returns
    ///
    /// - `Ok(Some(data))` - Relationships found
    /// - `Ok(None)` - No relationships in this content (valid)
    /// - `Err(_)` - Extraction failed
    ///
    /// # Implementation Notes
    ///
    /// Relationships may come from:
    /// - Frontmatter fields (e.g., `prerequisites: [a, b]`)
    /// - Markdown sections (e.g., `## Related Concepts` lists)
    ///
    /// Use `fabryk_content::markdown::extract_list_from_section()` for the latter.
    fn extract_edges(
        &self,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>>;

    /// Convert domain node data to a generic graph Node.
    ///
    /// Maps domain-specific fields to the generic `Node` structure.
    /// Domain-specific fields can be stored in `Node::metadata`.
    ///
    /// # Arguments
    ///
    /// * `node_data` - Previously extracted domain data
    ///
    /// # Returns
    ///
    /// A generic `Node` suitable for the graph.
    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node;

    /// Convert domain edge data to generic graph Edges.
    ///
    /// Maps domain-specific relationships to `Edge` structs with
    /// appropriate `Relationship` variants.
    ///
    /// # Arguments
    ///
    /// * `from_id` - Source node ID (from the node this was extracted from)
    /// * `edge_data` - Previously extracted relationship data
    ///
    /// # Returns
    ///
    /// Zero or more edges originating from `from_id`.
    ///
    /// # Implementation Notes
    ///
    /// Use appropriate `Relationship` variants:
    /// - `Relationship::Prerequisite` for prerequisites
    /// - `Relationship::LeadsTo` for leads_to
    /// - `Relationship::RelatesTo` for see_also/related
    /// - `Relationship::Custom(name)` for domain-specific relationships
    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge>;

    /// Returns the content glob pattern for this domain.
    ///
    /// Used by `GraphBuilder` to discover content files.
    /// Default: `"**/*.md"` (all markdown files recursively).
    fn content_glob(&self) -> &str {
        "**/*.md"
    }

    /// Returns the name of this extractor for logging/debugging.
    fn name(&self) -> &str {
        "unnamed"
    }
}
```

### Step 2: Add mock extractor for testing

Continue in `fabryk-graph/src/extractor.rs`:

```rust
/// A simple mock extractor for testing.
///
/// Extracts minimal data from content files with simple frontmatter.
/// Used in unit tests for `GraphBuilder` and other components.
#[cfg(any(test, feature = "test-utils"))]
pub mod mock {
    use super::*;
    use crate::Relationship;

    /// Mock node data for testing.
    #[derive(Clone, Debug)]
    pub struct MockNodeData {
        pub id: String,
        pub title: String,
        pub category: Option<String>,
    }

    /// Mock edge data for testing.
    #[derive(Clone, Debug)]
    pub struct MockEdgeData {
        pub prerequisites: Vec<String>,
        pub related: Vec<String>,
    }

    /// Mock extractor that expects simple frontmatter.
    ///
    /// Expected frontmatter format:
    /// ```yaml
    /// title: "Node Title"
    /// category: "optional-category"
    /// prerequisites:
    ///   - prereq-id-1
    ///   - prereq-id-2
    /// related:
    ///   - related-id-1
    /// ```
    #[derive(Clone, Debug, Default)]
    pub struct MockExtractor;

    impl GraphExtractor for MockExtractor {
        type NodeData = MockNodeData;
        type EdgeData = MockEdgeData;

        fn extract_node(
            &self,
            base_path: &Path,
            file_path: &Path,
            frontmatter: &serde_yaml::Value,
            _content: &str,
        ) -> Result<Self::NodeData> {
            let id = fabryk_core::util::ids::id_from_path(base_path, file_path)?;

            let title = frontmatter
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let category = frontmatter
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(MockNodeData { id, title, category })
        }

        fn extract_edges(
            &self,
            frontmatter: &serde_yaml::Value,
            _content: &str,
        ) -> Result<Option<Self::EdgeData>> {
            let prerequisites = frontmatter
                .get("prerequisites")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            let related = frontmatter
                .get("related")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            if prerequisites.is_empty() && related.is_empty() {
                Ok(None)
            } else {
                Ok(Some(MockEdgeData { prerequisites, related }))
            }
        }

        fn to_graph_node(&self, node_data: &Self::NodeData) -> Node {
            let mut node = Node::new(&node_data.id, &node_data.title);
            if let Some(ref cat) = node_data.category {
                node = node.with_category(cat);
            }
            node
        }

        fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
            let mut edges = Vec::new();

            for prereq in &edge_data.prerequisites {
                edges.push(Edge::new(from_id, prereq, Relationship::Prerequisite));
            }

            for related in &edge_data.related {
                edges.push(Edge::new(from_id, related, Relationship::RelatesTo));
            }

            edges
        }

        fn name(&self) -> &str {
            "mock"
        }
    }
}
```

### Step 3: Update lib.rs

Update `fabryk-graph/src/lib.rs`:

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
//! - `extractor` - `GraphExtractor` trait for domain-specific extraction
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_graph::{GraphExtractor, Node, Edge, Relationship};
//!
//! // Implement GraphExtractor for your domain
//! struct MyExtractor;
//!
//! impl GraphExtractor for MyExtractor {
//!     type NodeData = MyNodeData;
//!     type EdgeData = MyEdgeData;
//!     // ... implement methods
//! }
//! ```

pub mod extractor;
pub mod types;

// Re-exports
pub use extractor::GraphExtractor;
pub use types::{Edge, EdgeOrigin, GraphData, Node, Relationship};

#[cfg(any(test, feature = "test-utils"))]
pub use extractor::mock::{MockEdgeData, MockExtractor, MockNodeData};
```

### Step 4: Update Cargo.toml for test-utils feature

Update `fabryk-graph/Cargo.toml`:

```toml
[features]
default = []
rkyv-cache = ["rkyv", "blake3"]
test-utils = []
```

### Step 5: Add trait tests

Create `fabryk-graph/src/extractor.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::mock::*;
    use super::*;
    use crate::Relationship;
    use std::path::PathBuf;

    fn sample_frontmatter() -> serde_yaml::Value {
        serde_yaml::from_str(
            r#"
            title: "Test Concept"
            category: "test-category"
            prerequisites:
              - prereq-a
              - prereq-b
            related:
              - related-x
            "#,
        )
        .unwrap()
    }

    #[test]
    fn test_mock_extractor_extract_node() {
        let extractor = MockExtractor;
        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/harmony/test-concept.md");
        let frontmatter = sample_frontmatter();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &frontmatter, "content")
            .unwrap();

        assert_eq!(node_data.id, "test-concept");
        assert_eq!(node_data.title, "Test Concept");
        assert_eq!(node_data.category, Some("test-category".to_string()));
    }

    #[test]
    fn test_mock_extractor_extract_edges() {
        let extractor = MockExtractor;
        let frontmatter = sample_frontmatter();

        let edge_data = extractor
            .extract_edges(&frontmatter, "content")
            .unwrap()
            .unwrap();

        assert_eq!(edge_data.prerequisites, vec!["prereq-a", "prereq-b"]);
        assert_eq!(edge_data.related, vec!["related-x"]);
    }

    #[test]
    fn test_mock_extractor_extract_edges_none() {
        let extractor = MockExtractor;
        let frontmatter = serde_yaml::from_str("title: Test").unwrap();

        let edge_data = extractor.extract_edges(&frontmatter, "content").unwrap();
        assert!(edge_data.is_none());
    }

    #[test]
    fn test_mock_extractor_to_graph_node() {
        let extractor = MockExtractor;
        let node_data = MockNodeData {
            id: "test-id".to_string(),
            title: "Test Title".to_string(),
            category: Some("test-cat".to_string()),
        };

        let node = extractor.to_graph_node(&node_data);

        assert_eq!(node.id, "test-id");
        assert_eq!(node.title, "Test Title");
        assert_eq!(node.category, Some("test-cat".to_string()));
    }

    #[test]
    fn test_mock_extractor_to_graph_edges() {
        let extractor = MockExtractor;
        let edge_data = MockEdgeData {
            prerequisites: vec!["a".to_string(), "b".to_string()],
            related: vec!["x".to_string()],
        };

        let edges = extractor.to_graph_edges("from-node", &edge_data);

        assert_eq!(edges.len(), 3);

        // Check prerequisites
        assert!(edges
            .iter()
            .any(|e| e.to == "a" && e.relationship == Relationship::Prerequisite));
        assert!(edges
            .iter()
            .any(|e| e.to == "b" && e.relationship == Relationship::Prerequisite));

        // Check related
        assert!(edges
            .iter()
            .any(|e| e.to == "x" && e.relationship == Relationship::RelatesTo));

        // All edges should have from_id set
        assert!(edges.iter().all(|e| e.from == "from-node"));
    }

    #[test]
    fn test_extractor_default_methods() {
        let extractor = MockExtractor;
        assert_eq!(extractor.content_glob(), "**/*.md");
        assert_eq!(extractor.name(), "mock");
    }
}
```

### Step 6: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-graph
cargo test -p fabryk-graph
cargo clippy -p fabryk-graph -- -D warnings
```

## Exit Criteria

- [ ] `GraphExtractor` trait defined with associated types `NodeData` and `EdgeData`
- [ ] `extract_node()` method defined with correct signature
- [ ] `extract_edges()` method returns `Option<EdgeData>` for files without relationships
- [ ] `to_graph_node()` converts domain data to generic `Node`
- [ ] `to_graph_edges()` converts domain data to `Vec<Edge>`
- [ ] Default `content_glob()` returns `"**/*.md"`
- [ ] `MockExtractor` implemented for testing
- [ ] `test-utils` feature gates mock types
- [ ] All tests pass
- [ ] Documentation includes usage examples

## Commit Message

```
feat(graph): define GraphExtractor trait

Add GraphExtractor trait - the core abstraction for domain-specific
graph extraction:
- Associated types: NodeData, EdgeData
- Extraction methods: extract_node(), extract_edges()
- Conversion methods: to_graph_node(), to_graph_edges()
- Default content_glob() returning "**/*.md"

Add MockExtractor for testing with test-utils feature.

Per Amendment §2c, GraphExtractor::NodeData serves as the metadata
carrier, deferring standalone MetadataExtractor to v0.2+.

Phase 4 milestone 4.2 of Fabryk extraction.

Ref: Doc 0011 §3, §9 (GraphExtractor design)
Ref: Doc 0012 §2c (MetadataExtractor consolidation)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
