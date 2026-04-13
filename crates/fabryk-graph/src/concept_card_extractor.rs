//! Default concept card graph extractor.
//!
//! [`ConceptCardGraphExtractor`] implements [`GraphExtractor`] for content
//! that uses the [`ConceptCardFrontmatter`](fabryk_content::ConceptCardFrontmatter)
//! schema.  It extracts nodes with metadata (tier, subcategory, confidence,
//! aliases) and edges from `prerequisites`, `related_concepts`, `extends`,
//! `contrasts_with`, and `related` frontmatter arrays.
//!
//! This extractor is domain-agnostic: it works for any knowledge domain whose
//! concept cards follow the standard frontmatter schema.

use std::path::Path;

use fabryk_core::Result;

use crate::{Edge, GraphExtractor, Node, Relationship};

// ============================================================================
// Associated data types
// ============================================================================

/// Node data extracted from concept card frontmatter.
#[derive(Clone, Debug)]
pub struct ConceptCardNodeData {
    /// Concept identifier derived from the file path.
    pub id: String,
    /// Human-readable title.
    pub title: String,
    /// Top-level category.
    pub category: Option<String>,
    /// Source publication.
    pub source: Option<String>,
    /// Difficulty or depth tier.
    pub tier: Option<String>,
    /// Finer classification within category.
    pub subcategory: Option<String>,
    /// Extraction quality indicator.
    pub extraction_confidence: Option<String>,
    /// Alternative names or synonyms.
    pub aliases: Vec<String>,
}

/// Edge data extracted from concept card frontmatter.
#[derive(Clone, Debug)]
pub struct ConceptCardEdgeData {
    /// Prerequisite concept IDs.
    pub prerequisites: Vec<String>,
    /// Related concept IDs (v2 `related_concepts` field).
    pub related_concepts: Vec<String>,
    /// Concept IDs this card builds upon.
    pub extends: Vec<String>,
    /// Commonly confused concept IDs.
    pub contrasts_with: Vec<String>,
    /// Associated concept IDs (v3 `related` field).
    pub related: Vec<String>,
}

// ============================================================================
// Extractor
// ============================================================================

/// Graph extractor for concept card markdown files.
///
/// Extracts nodes from YAML frontmatter metadata and edges from relationship
/// arrays (`prerequisites`, `related_concepts`, `extends`, `contrasts_with`,
/// `related`).
///
/// # Relationship Mapping
///
/// | Frontmatter field   | Fabryk `Relationship`      |
/// |---------------------|----------------------------|
/// | `prerequisites`     | `Prerequisite`             |
/// | `related_concepts`  | `RelatesTo`                |
/// | `extends`           | `Extends`                  |
/// | `contrasts_with`    | `ContrastsWith`            |
/// | `related`           | `RelatesTo`                |
#[derive(Debug, Default)]
pub struct ConceptCardGraphExtractor;

impl ConceptCardGraphExtractor {
    /// Create a new graph extractor.
    pub fn new() -> Self {
        Self
    }
}

impl GraphExtractor for ConceptCardGraphExtractor {
    type NodeData = ConceptCardNodeData;
    type EdgeData = ConceptCardEdgeData;

    fn extract_node(
        &self,
        _base_path: &Path,
        file_path: &Path,
        frontmatter: &yaml_serde::Value,
        _content: &str,
    ) -> Result<Self::NodeData> {
        let id = fabryk_core::util::ids::id_from_path(file_path)
            .ok_or_else(|| fabryk_core::Error::parse("cannot derive ID from file path"))?;

        let title = frontmatter
            .get("title")
            .or_else(|| frontmatter.get("concept"))
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();

        let category = frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .map(String::from);

        let source = frontmatter
            .get("source")
            .and_then(|v| v.as_str())
            .map(String::from);

        let tier = frontmatter
            .get("tier")
            .and_then(|v| v.as_str())
            .map(String::from);

        let subcategory = frontmatter
            .get("subcategory")
            .and_then(|v| v.as_str())
            .map(String::from);

        let extraction_confidence = frontmatter
            .get("extraction_confidence")
            .and_then(|v| v.as_str())
            .map(String::from);

        let aliases = extract_string_array(frontmatter, "aliases");

        Ok(ConceptCardNodeData {
            id,
            title,
            category,
            source,
            tier,
            subcategory,
            extraction_confidence,
            aliases,
        })
    }

    fn extract_edges(
        &self,
        frontmatter: &yaml_serde::Value,
        _content: &str,
    ) -> Result<Option<Self::EdgeData>> {
        let prerequisites = extract_string_array(frontmatter, "prerequisites");
        let related_concepts = extract_string_array(frontmatter, "related_concepts");
        let extends = extract_string_array(frontmatter, "extends");
        let contrasts_with = extract_string_array(frontmatter, "contrasts_with");
        let related = extract_string_array(frontmatter, "related");

        let has_edges = !prerequisites.is_empty()
            || !related_concepts.is_empty()
            || !extends.is_empty()
            || !contrasts_with.is_empty()
            || !related.is_empty();

        if has_edges {
            Ok(Some(ConceptCardEdgeData {
                prerequisites,
                related_concepts,
                extends,
                contrasts_with,
                related,
            }))
        } else {
            Ok(None)
        }
    }

    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node {
        let mut node = Node::new(&node_data.id, &node_data.title);
        if let Some(ref cat) = node_data.category {
            node = node.with_category(cat);
        }
        if let Some(ref source) = node_data.source {
            node = node.with_source(source);
        }
        if let Some(ref tier) = node_data.tier {
            node = node.with_metadata("tier", serde_json::Value::String(tier.clone()));
        }
        if let Some(ref confidence) = node_data.extraction_confidence {
            node = node.with_metadata(
                "extraction_confidence",
                serde_json::Value::String(confidence.clone()),
            );
        }
        if let Some(ref subcat) = node_data.subcategory {
            node = node.with_metadata("subcategory", serde_json::Value::String(subcat.clone()));
        }
        if !node_data.aliases.is_empty() {
            node = node.with_metadata("aliases", serde_json::json!(node_data.aliases));
        }
        node
    }

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();

        for prereq in &edge_data.prerequisites {
            edges.push(Edge::new(from_id, prereq, Relationship::Prerequisite));
        }
        for related in &edge_data.related_concepts {
            edges.push(Edge::new(from_id, related, Relationship::RelatesTo));
        }
        for ext in &edge_data.extends {
            edges.push(Edge::new(from_id, ext, Relationship::Extends));
        }
        for contrast in &edge_data.contrasts_with {
            edges.push(Edge::new(from_id, contrast, Relationship::ContrastsWith));
        }
        for rel in &edge_data.related {
            edges.push(Edge::new(from_id, rel, Relationship::RelatesTo));
        }

        edges
    }

    fn content_glob(&self) -> &str {
        "**/*.md"
    }

    fn name(&self) -> &str {
        "concept-card"
    }
}

/// Extract a `Vec<String>` from a YAML frontmatter sequence field.
fn extract_string_array(frontmatter: &yaml_serde::Value, key: &str) -> Vec<String> {
    frontmatter
        .get(key)
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_frontmatter() -> yaml_serde::Value {
        yaml_serde::from_str(
            r#"
title: "Voice Leading"
category: "counterpoint"
source: "Open Music Theory"
prerequisites:
  - intervals
  - scales
related_concepts:
  - chord-voicing
  - part-writing
"#,
        )
        .unwrap()
    }

    #[test]
    fn test_extract_node_full() {
        let extractor = ConceptCardGraphExtractor::new();
        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/counterpoint/voice-leading.md");
        let fm = sample_frontmatter();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &fm, "body")
            .unwrap();

        assert_eq!(node_data.id, "voice-leading");
        assert_eq!(node_data.title, "Voice Leading");
        assert_eq!(node_data.category, Some("counterpoint".to_string()));
        assert_eq!(node_data.source, Some("Open Music Theory".to_string()));
    }

    #[test]
    fn test_extract_node_minimal() {
        let extractor = ConceptCardGraphExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/simple.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("title: Simple").unwrap();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &fm, "")
            .unwrap();

        assert_eq!(node_data.id, "simple");
        assert_eq!(node_data.title, "Simple");
        assert!(node_data.category.is_none());
        assert!(node_data.source.is_none());
    }

    #[test]
    fn test_extract_node_no_title_fallback() {
        let extractor = ConceptCardGraphExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/untitled-concept.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("category: test").unwrap();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &fm, "")
            .unwrap();

        assert_eq!(node_data.title, "untitled-concept");
    }

    #[test]
    fn test_extract_node_concept_as_title() {
        let extractor = ConceptCardGraphExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str("concept: My Concept").unwrap();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &fm, "")
            .unwrap();

        assert_eq!(node_data.title, "My Concept");
    }

    #[test]
    fn test_extract_edges() {
        let extractor = ConceptCardGraphExtractor::new();
        let fm = sample_frontmatter();

        let edge_data = extractor.extract_edges(&fm, "content").unwrap().unwrap();

        assert_eq!(edge_data.prerequisites, vec!["intervals", "scales"]);
        assert_eq!(
            edge_data.related_concepts,
            vec!["chord-voicing", "part-writing"]
        );
    }

    #[test]
    fn test_extract_edges_none() {
        let extractor = ConceptCardGraphExtractor::new();
        let fm: yaml_serde::Value = yaml_serde::from_str("title: Leaf").unwrap();

        let edge_data = extractor.extract_edges(&fm, "content").unwrap();
        assert!(edge_data.is_none());
    }

    #[test]
    fn test_extract_edges_only_prerequisites() {
        let extractor = ConceptCardGraphExtractor::new();
        let fm: yaml_serde::Value = yaml_serde::from_str("prerequisites:\n  - pitch").unwrap();

        let edge_data = extractor.extract_edges(&fm, "").unwrap().unwrap();
        assert_eq!(edge_data.prerequisites, vec!["pitch"]);
        assert!(edge_data.related_concepts.is_empty());
    }

    #[test]
    fn test_to_graph_node() {
        let extractor = ConceptCardGraphExtractor::default();
        let node_data = ConceptCardNodeData {
            id: "test-id".to_string(),
            title: "Test Title".to_string(),
            category: Some("harmony".to_string()),
            source: Some("OMT".to_string()),
            tier: None,
            subcategory: None,
            extraction_confidence: None,
            aliases: vec![],
        };

        let node = extractor.to_graph_node(&node_data);
        assert_eq!(node.id, "test-id");
        assert_eq!(node.title, "Test Title");
        assert_eq!(node.category, Some("harmony".to_string()));
        assert_eq!(node.source_id, Some("OMT".to_string()));
    }

    #[test]
    fn test_to_graph_node_no_optionals() {
        let extractor = ConceptCardGraphExtractor::new();
        let node_data = ConceptCardNodeData {
            id: "x".to_string(),
            title: "X".to_string(),
            category: None,
            source: None,
            tier: None,
            subcategory: None,
            extraction_confidence: None,
            aliases: vec![],
        };

        let node = extractor.to_graph_node(&node_data);
        assert!(node.category.is_none());
        assert!(node.source_id.is_none());
        assert!(node.metadata.is_empty());
    }

    #[test]
    fn test_to_graph_node_with_metadata() {
        let extractor = ConceptCardGraphExtractor::new();
        let base_path = PathBuf::from("/data");
        let file_path = PathBuf::from("/data/test.md");
        let fm: yaml_serde::Value = yaml_serde::from_str(
            r#"
title: "Acoustic Consonance"
category: "acoustics"
tier: "foundational"
subcategory: "consonance-dissonance"
extraction_confidence: "high"
aliases:
  - "sensory consonance"
  - "tonal consonance"
"#,
        )
        .unwrap();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &fm, "")
            .unwrap();

        assert_eq!(node_data.tier, Some("foundational".to_string()));
        assert_eq!(
            node_data.subcategory,
            Some("consonance-dissonance".to_string())
        );
        assert_eq!(node_data.extraction_confidence, Some("high".to_string()));
        assert_eq!(
            node_data.aliases,
            vec!["sensory consonance", "tonal consonance"]
        );

        let node = extractor.to_graph_node(&node_data);
        assert_eq!(
            node.metadata.get("tier").unwrap(),
            &serde_json::Value::String("foundational".to_string())
        );
        assert_eq!(
            node.metadata.get("subcategory").unwrap(),
            &serde_json::Value::String("consonance-dissonance".to_string())
        );
        assert_eq!(
            node.metadata.get("extraction_confidence").unwrap(),
            &serde_json::Value::String("high".to_string())
        );
        assert_eq!(
            node.metadata.get("aliases").unwrap(),
            &serde_json::json!(["sensory consonance", "tonal consonance"])
        );
    }

    #[test]
    fn test_to_graph_edges() {
        let extractor = ConceptCardGraphExtractor::new();
        let edge_data = ConceptCardEdgeData {
            prerequisites: vec!["a".to_string(), "b".to_string()],
            related_concepts: vec!["x".to_string()],
            extends: vec![],
            contrasts_with: vec![],
            related: vec![],
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

        assert!(edges.iter().all(|e| e.from == "from-node"));
    }

    #[test]
    fn test_to_graph_edges_empty() {
        let extractor = ConceptCardGraphExtractor::new();
        let edge_data = ConceptCardEdgeData {
            prerequisites: vec![],
            related_concepts: vec![],
            extends: vec![],
            contrasts_with: vec![],
            related: vec![],
        };

        let edges = extractor.to_graph_edges("node", &edge_data);
        assert!(edges.is_empty());
    }

    #[test]
    fn test_v3_edge_types() {
        let extractor = ConceptCardGraphExtractor::new();
        let fm: yaml_serde::Value = yaml_serde::from_str(
            r#"
prerequisites:
  - harmonic-series
extends:
  - interval-quality
contrasts_with:
  - acoustic-dissonance
related:
  - roughness
"#,
        )
        .unwrap();

        let edge_data = extractor.extract_edges(&fm, "").unwrap().unwrap();
        assert_eq!(edge_data.prerequisites, vec!["harmonic-series"]);
        assert_eq!(edge_data.extends, vec!["interval-quality"]);
        assert_eq!(edge_data.contrasts_with, vec!["acoustic-dissonance"]);
        assert_eq!(edge_data.related, vec!["roughness"]);

        let edges = extractor.to_graph_edges("source-node", &edge_data);
        assert_eq!(edges.len(), 4);
        assert!(
            edges
                .iter()
                .any(|e| e.to == "harmonic-series" && e.relationship == Relationship::Prerequisite)
        );
        assert!(
            edges
                .iter()
                .any(|e| e.to == "interval-quality" && e.relationship == Relationship::Extends)
        );
        assert!(edges.iter().any(
            |e| e.to == "acoustic-dissonance" && e.relationship == Relationship::ContrastsWith
        ));
        assert!(
            edges
                .iter()
                .any(|e| e.to == "roughness" && e.relationship == Relationship::RelatesTo)
        );
    }

    #[test]
    fn test_defaults() {
        let extractor = ConceptCardGraphExtractor::new();
        assert_eq!(extractor.content_glob(), "**/*.md");
        assert_eq!(extractor.name(), "concept-card");
    }
}
