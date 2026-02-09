---
title: "CC Prompt: Fabryk 4.7 — MusicTheoryExtractor"
milestone: "4.7"
phase: 4
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["4.1-4.6 complete"]
governing-docs: [0011-audit §3 §9.1, 0013-project-plan]
---

# CC Prompt: Fabryk 4.7 — MusicTheoryExtractor

## Context

This milestone creates the **domain-specific** implementation of `GraphExtractor`
for the music-theory skill. This code **stays in ai-music-theory** — it is not
part of the fabryk-graph crate.

The `MusicTheoryExtractor` knows about:
- Music theory frontmatter fields (concept, category, source, prerequisites, etc.)
- How to parse `RelatedConcepts` from markdown sections
- Music theory relationship semantics

This is the critical piece that bridges the generic graph infrastructure
with the music-theory domain.

## Objective

Create `MusicTheoryExtractor` in ai-music-theory that implements `GraphExtractor`:

1. Define `MusicTheoryNodeData` (domain-specific node info)
2. Define `MusicTheoryEdgeData` (domain-specific relationship info)
3. Implement `extract_node()` for music theory frontmatter
4. Implement `extract_edges()` for prerequisites and related concepts
5. Implement `to_graph_node()` and `to_graph_edges()` conversions

## Implementation Steps

### Step 1: Create the extractor file

Create `crates/music-theory/mcp-server/src/music_theory_extractor.rs`:

```rust
//! MusicTheoryExtractor - GraphExtractor implementation for music theory.
//!
//! This module provides the domain-specific extraction logic for building
//! the music theory knowledge graph from concept card markdown files.

use fabryk_content::markdown::helpers::extract_list_from_section;
use fabryk_core::util::ids::id_from_path;
use fabryk_core::{Error, Result};
use fabryk_graph::{Edge, EdgeOrigin, GraphExtractor, Node, Relationship};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Domain-specific node data for music theory concepts.
///
/// Extracted from concept card frontmatter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicTheoryNodeData {
    /// Concept ID (derived from file path).
    pub id: String,
    /// Concept title.
    pub title: String,
    /// Category (e.g., "harmony", "rhythm", "form").
    pub category: Option<String>,
    /// Source identifier (e.g., "tymoczko", "lewin").
    pub source: Option<String>,
    /// Brief description of the concept.
    pub description: Option<String>,
    /// Whether this is a canonical concept or source-specific variant.
    pub is_canonical: bool,
    /// If variant, the canonical concept ID.
    pub canonical_id: Option<String>,
    /// Additional tags.
    pub tags: Vec<String>,
    /// Difficulty level (1-5).
    pub difficulty: Option<u8>,
}

/// Domain-specific relationship data for music theory.
///
/// Extracted from frontmatter and markdown sections.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MusicTheoryEdgeData {
    /// Prerequisite concepts (must understand before this).
    pub prerequisites: Vec<String>,
    /// Concepts this leads to (natural next steps).
    pub leads_to: Vec<String>,
    /// Related concepts (see also).
    pub see_also: Vec<String>,
    /// Concepts this extends or generalises.
    pub extends: Vec<String>,
}

impl MusicTheoryEdgeData {
    /// Check if there are any relationships.
    pub fn is_empty(&self) -> bool {
        self.prerequisites.is_empty()
            && self.leads_to.is_empty()
            && self.see_also.is_empty()
            && self.extends.is_empty()
    }
}

/// GraphExtractor implementation for music theory concepts.
#[derive(Clone, Debug, Default)]
pub struct MusicTheoryExtractor {
    /// Whether to extract relationships from markdown body sections.
    pub extract_from_body: bool,
}

impl MusicTheoryExtractor {
    /// Create a new extractor.
    pub fn new() -> Self {
        Self {
            extract_from_body: true,
        }
    }

    /// Create an extractor that only uses frontmatter (no body parsing).
    pub fn frontmatter_only() -> Self {
        Self {
            extract_from_body: false,
        }
    }

    /// Parse string list from frontmatter YAML value.
    fn parse_string_list(value: Option<&serde_yaml::Value>) -> Vec<String> {
        value
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl GraphExtractor for MusicTheoryExtractor {
    type NodeData = MusicTheoryNodeData;
    type EdgeData = MusicTheoryEdgeData;

    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        _content: &str,
    ) -> Result<Self::NodeData> {
        // Compute ID from file path
        let id = id_from_path(base_path, file_path)?;

        // Extract frontmatter fields
        let title = frontmatter
            .get("concept")
            .or_else(|| frontmatter.get("title"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::parse(format!("Missing title in {}", file_path.display())))?
            .to_string();

        let category = frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .map(String::from);

        let source = frontmatter
            .get("source")
            .and_then(|v| v.as_str())
            .map(String::from);

        let description = frontmatter
            .get("description")
            .or_else(|| frontmatter.get("summary"))
            .and_then(|v| v.as_str())
            .map(String::from);

        // Check if this is a canonical concept or a source-specific variant
        let canonical_id = frontmatter
            .get("canonical")
            .or_else(|| frontmatter.get("variant_of"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let is_canonical = canonical_id.is_none();

        let tags = Self::parse_string_list(frontmatter.get("tags"));

        let difficulty = frontmatter
            .get("difficulty")
            .and_then(|v| v.as_u64())
            .map(|d| d.min(5) as u8);

        Ok(MusicTheoryNodeData {
            id,
            title,
            category,
            source,
            description,
            is_canonical,
            canonical_id,
            tags,
            difficulty,
        })
    }

    fn extract_edges(
        &self,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>> {
        // Extract from frontmatter
        let mut prerequisites = Self::parse_string_list(frontmatter.get("prerequisites"));
        let mut leads_to = Self::parse_string_list(frontmatter.get("leads_to"));
        let mut see_also = Self::parse_string_list(frontmatter.get("see_also"));
        let extends = Self::parse_string_list(frontmatter.get("extends"));

        // Optionally extract from markdown body
        if self.extract_from_body {
            // Look for "## Related Concepts" section
            if let Some(body_prereqs) =
                extract_list_from_section(content, "Related Concepts", "Prerequisite")
            {
                for prereq in body_prereqs {
                    if !prerequisites.contains(&prereq) {
                        prerequisites.push(prereq);
                    }
                }
            }

            if let Some(body_leads_to) =
                extract_list_from_section(content, "Related Concepts", "Leads to")
            {
                for item in body_leads_to {
                    if !leads_to.contains(&item) {
                        leads_to.push(item);
                    }
                }
            }

            if let Some(body_see_also) =
                extract_list_from_section(content, "Related Concepts", "See also")
            {
                for item in body_see_also {
                    if !see_also.contains(&item) {
                        see_also.push(item);
                    }
                }
            }
        }

        let edge_data = MusicTheoryEdgeData {
            prerequisites,
            leads_to,
            see_also,
            extends,
        };

        if edge_data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(edge_data))
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

        if !node_data.is_canonical {
            if let Some(ref canonical) = node_data.canonical_id {
                node = node.as_variant_of(canonical);
            }
        }

        // Store additional data in metadata
        if let Some(ref desc) = node_data.description {
            node = node.with_metadata("description", desc.clone());
        }

        if !node_data.tags.is_empty() {
            node = node.with_metadata(
                "tags",
                serde_json::Value::Array(
                    node_data
                        .tags
                        .iter()
                        .map(|t| serde_json::Value::String(t.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(diff) = node_data.difficulty {
            node = node.with_metadata("difficulty", diff as i64);
        }

        node
    }

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();

        // Prerequisites
        for prereq in &edge_data.prerequisites {
            edges.push(
                Edge::new(from_id, prereq, Relationship::Prerequisite)
                    .with_origin(EdgeOrigin::Frontmatter),
            );
        }

        // Leads to
        for target in &edge_data.leads_to {
            edges.push(
                Edge::new(from_id, target, Relationship::LeadsTo)
                    .with_origin(EdgeOrigin::Frontmatter),
            );
        }

        // See also / relates to
        for related in &edge_data.see_also {
            edges.push(
                Edge::new(from_id, related, Relationship::RelatesTo)
                    .with_origin(EdgeOrigin::Frontmatter),
            );
        }

        // Extends
        for target in &edge_data.extends {
            edges.push(
                Edge::new(from_id, target, Relationship::Extends)
                    .with_origin(EdgeOrigin::Frontmatter),
            );
        }

        edges
    }

    fn content_glob(&self) -> &str {
        "**/*.md"
    }

    fn name(&self) -> &str {
        "music-theory"
    }
}
```

### Step 2: Add tests

Create or add to `crates/music-theory/mcp-server/src/music_theory_extractor.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample_frontmatter() -> serde_yaml::Value {
        serde_yaml::from_str(
            r#"
            concept: "Picardy Third"
            category: "harmony"
            source: "tymoczko"
            description: "A major tonic chord at the end of a minor-mode passage"
            difficulty: 3
            tags:
              - cadence
              - modal-mixture
            prerequisites:
              - minor-key
              - major-triad
            leads_to:
              - modal-mixture
            see_also:
              - borrowed-chords
            "#,
        )
        .unwrap()
    }

    #[test]
    fn test_extract_node() {
        let extractor = MusicTheoryExtractor::new();
        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/harmony/picardy-third.md");
        let frontmatter = sample_frontmatter();

        let node_data = extractor
            .extract_node(&base_path, &file_path, &frontmatter, "")
            .unwrap();

        assert_eq!(node_data.id, "picardy-third");
        assert_eq!(node_data.title, "Picardy Third");
        assert_eq!(node_data.category, Some("harmony".to_string()));
        assert_eq!(node_data.source, Some("tymoczko".to_string()));
        assert_eq!(node_data.difficulty, Some(3));
        assert!(node_data.is_canonical);
        assert!(node_data.tags.contains(&"cadence".to_string()));
    }

    #[test]
    fn test_extract_edges() {
        let extractor = MusicTheoryExtractor::new();
        let frontmatter = sample_frontmatter();

        let edge_data = extractor.extract_edges(&frontmatter, "").unwrap().unwrap();

        assert_eq!(edge_data.prerequisites, vec!["minor-key", "major-triad"]);
        assert_eq!(edge_data.leads_to, vec!["modal-mixture"]);
        assert_eq!(edge_data.see_also, vec!["borrowed-chords"]);
    }

    #[test]
    fn test_to_graph_node() {
        let extractor = MusicTheoryExtractor::new();
        let node_data = MusicTheoryNodeData {
            id: "test-concept".to_string(),
            title: "Test Concept".to_string(),
            category: Some("harmony".to_string()),
            source: Some("source".to_string()),
            description: Some("A test".to_string()),
            is_canonical: true,
            canonical_id: None,
            tags: vec!["tag1".to_string()],
            difficulty: Some(2),
        };

        let node = extractor.to_graph_node(&node_data);

        assert_eq!(node.id, "test-concept");
        assert_eq!(node.title, "Test Concept");
        assert_eq!(node.category, Some("harmony".to_string()));
        assert!(node.is_canonical);
    }

    #[test]
    fn test_to_graph_edges() {
        let extractor = MusicTheoryExtractor::new();
        let edge_data = MusicTheoryEdgeData {
            prerequisites: vec!["a".to_string(), "b".to_string()],
            leads_to: vec!["c".to_string()],
            see_also: vec!["d".to_string()],
            extends: vec![],
        };

        let edges = extractor.to_graph_edges("source", &edge_data);

        assert_eq!(edges.len(), 4);

        // Check prerequisites
        let prereqs: Vec<_> = edges
            .iter()
            .filter(|e| e.relationship == Relationship::Prerequisite)
            .collect();
        assert_eq!(prereqs.len(), 2);

        // Check leads_to
        let leads: Vec<_> = edges
            .iter()
            .filter(|e| e.relationship == Relationship::LeadsTo)
            .collect();
        assert_eq!(leads.len(), 1);
    }

    #[test]
    fn test_variant_node() {
        let extractor = MusicTheoryExtractor::new();
        let frontmatter = serde_yaml::from_str(
            r#"
            concept: "Picardy Third (Tymoczko)"
            canonical: "picardy-third"
            source: "tymoczko"
            "#,
        )
        .unwrap();

        let base_path = PathBuf::from("/data/concepts");
        let file_path = PathBuf::from("/data/concepts/sources/tymoczko/picardy-third.md");

        let node_data = extractor
            .extract_node(&base_path, &file_path, &frontmatter, "")
            .unwrap();

        assert!(!node_data.is_canonical);
        assert_eq!(node_data.canonical_id, Some("picardy-third".to_string()));
    }

    #[test]
    fn test_empty_edges() {
        let extractor = MusicTheoryExtractor::new();
        let frontmatter = serde_yaml::from_str("concept: Test").unwrap();

        let edge_data = extractor.extract_edges(&frontmatter, "").unwrap();
        assert!(edge_data.is_none());
    }
}
```

### Step 3: Update music-theory lib.rs/mod.rs

Add the new module to the music-theory crate's module tree:

```rust
// In crates/music-theory/mcp-server/src/lib.rs
pub mod music_theory_extractor;

pub use music_theory_extractor::{MusicTheoryExtractor, MusicTheoryNodeData, MusicTheoryEdgeData};
```

### Step 4: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p music-theory-mcp-server
cargo test -p music-theory-mcp-server
```

## Exit Criteria

- [ ] `MusicTheoryNodeData` struct defined with all concept fields
- [ ] `MusicTheoryEdgeData` struct captures all relationship types
- [ ] `extract_node()` parses music theory frontmatter correctly
- [ ] `extract_edges()` handles frontmatter and optional body extraction
- [ ] `to_graph_node()` converts to generic Node with metadata
- [ ] `to_graph_edges()` uses appropriate Relationship variants
- [ ] Variant nodes (source-specific) handled correctly
- [ ] All tests pass
- [ ] Extractor produces same output as current parser.rs

## Verification

Run this comparison to verify the extractor matches current behavior:

```bash
# Build current graph
cargo run -p music-theory-mcp-server -- graph build --output /tmp/old-graph.json

# Build with new extractor (after integration in 4.8)
cargo run -p music-theory-mcp-server -- graph build --output /tmp/new-graph.json

# Compare node and edge counts
jq '.nodes | length' /tmp/old-graph.json /tmp/new-graph.json
jq '.edges | length' /tmp/old-graph.json /tmp/new-graph.json
```

## Commit Message

```
feat(music-theory): add MusicTheoryExtractor implementing GraphExtractor

Create domain-specific GraphExtractor for music theory:
- MusicTheoryNodeData: id, title, category, source, description, tags, difficulty
- MusicTheoryEdgeData: prerequisites, leads_to, see_also, extends
- extract_node(): Parse concept card frontmatter
- extract_edges(): Extract from frontmatter + optional body sections
- to_graph_node(): Convert to generic Node with metadata
- to_graph_edges(): Map to appropriate Relationship variants

Supports both canonical concepts and source-specific variants.

Phase 4 milestone 4.7 of Fabryk extraction.

Ref: Doc 0011 §3, §9.1 (GraphExtractor design)
Ref: Doc 0013 Phase 4

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
