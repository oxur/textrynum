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
pub trait GraphExtractor: Send + Sync {
    /// Domain-specific node data extracted from content.
    type NodeData: Clone + Send + Sync;

    /// Domain-specific edge/relationship data extracted from content.
    type EdgeData: Clone + Send + Sync;

    /// Extract node data from a content file.
    ///
    /// # Arguments
    ///
    /// * `base_path` - Root directory for content
    /// * `file_path` - Full path to the file being processed
    /// * `frontmatter` - Parsed YAML frontmatter as generic Value
    /// * `content` - Markdown body (after frontmatter)
    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Self::NodeData>;

    /// Extract relationship/edge data from content.
    ///
    /// Returns `Ok(None)` if no relationships found (valid for leaf nodes).
    fn extract_edges(
        &self,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>>;

    /// Convert domain node data to a generic graph Node.
    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node;

    /// Convert domain edge data to generic graph Edges.
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

// ============================================================================
// Mock extractor for testing
// ============================================================================

/// A simple mock extractor for testing.
///
/// Extracts minimal data from content files with simple frontmatter.
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
            _base_path: &Path,
            file_path: &Path,
            frontmatter: &serde_yaml::Value,
            _content: &str,
        ) -> Result<Self::NodeData> {
            let id = fabryk_core::util::ids::id_from_path(file_path)
                .ok_or_else(|| fabryk_core::Error::parse("no file stem"))?;

            let title = frontmatter
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let category = frontmatter
                .get("category")
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(MockNodeData {
                id,
                title,
                category,
            })
        }

        fn extract_edges(
            &self,
            frontmatter: &serde_yaml::Value,
            _content: &str,
        ) -> Result<Option<Self::EdgeData>> {
            let prerequisites: Vec<String> = frontmatter
                .get("prerequisites")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default();

            let related: Vec<String> = frontmatter
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
                Ok(Some(MockEdgeData {
                    prerequisites,
                    related,
                }))
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

// ============================================================================
// Tests
// ============================================================================

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
    fn test_mock_extractor_to_graph_node_no_category() {
        let extractor = MockExtractor;
        let node_data = MockNodeData {
            id: "x".to_string(),
            title: "X".to_string(),
            category: None,
        };

        let node = extractor.to_graph_node(&node_data);
        assert!(node.category.is_none());
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

        assert!(
            edges
                .iter()
                .any(|e| e.to == "a" && e.relationship == Relationship::Prerequisite)
        );
        assert!(
            edges
                .iter()
                .any(|e| e.to == "b" && e.relationship == Relationship::Prerequisite)
        );
        assert!(
            edges
                .iter()
                .any(|e| e.to == "x" && e.relationship == Relationship::RelatesTo)
        );

        // All edges should have from_id set
        assert!(edges.iter().all(|e| e.from == "from-node"));
    }

    #[test]
    fn test_mock_extractor_to_graph_edges_empty() {
        let extractor = MockExtractor;
        let edge_data = MockEdgeData {
            prerequisites: vec![],
            related: vec![],
        };

        let edges = extractor.to_graph_edges("from-node", &edge_data);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_extractor_default_methods() {
        let extractor = MockExtractor;
        assert_eq!(extractor.content_glob(), "**/*.md");
        assert_eq!(extractor.name(), "mock");
    }
}
