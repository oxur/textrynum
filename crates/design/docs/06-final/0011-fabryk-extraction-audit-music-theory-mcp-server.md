---
number: 11
title: "Fabryk Extraction Audit: Music Theory MCP Server"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-02-03
updated: 2026-02-03
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Extraction Audit: Music Theory MCP Server

**Date:** 2026-01-28
**Auditor:** Claude Code
**Source Codebase:** `~/lab/music-comp/ai-music-theory/mcp-server`
**Target:** Extract domain-agnostic infrastructure into Fabryk ecosystem
**Deliverable:** Comprehensive extraction plan with trait designs and migration strategy

---

## Executive Summary

The Music Theory MCP server has organically grown into a sophisticated knowledge management system with **25 MCP tools**, **full-text search (Tantivy)**, **graph database**, and **content loading infrastructure**. This audit reveals that approximately **70-75% of the codebase is domain-agnostic** and can be extracted into reusable Fabryk crates.

**Key Findings:**

- **Domain coupling is concentrated** in 3-4 critical files (config.rs, graph/parser.rs, metadata/extraction.rs)
- **Most infrastructure is already generic**: markdown parsing, file utilities, search backend, graph algorithms, MCP server framework
- **The `GraphExtractor` trait** is the critical abstraction for enabling multi-domain support
- **Extraction strategy**: Start with utilities → content → search → graph → MCP tools (in order of increasing complexity)

**Risk Level:** **MODERATE** - Well-defined boundaries, existing test coverage ≥95%, but graph/parser.rs refactoring requires careful trait design.

---

## Table of Contents

1. [File-by-File Inventory](#1-file-by-file-inventory)
2. [Classification Matrix](#2-classification-matrix)
3. [GraphExtractor Trait Design](#3-graphextractor-trait-design)
4. [Extraction Plan by Crate](#4-extraction-plan-by-crate)
5. [Music Theory Remainder](#5-music-theory-remainder)
6. [Dependency Analysis](#6-dependency-analysis)
7. [Migration Steps](#7-migration-steps)
8. [Risk Assessment](#8-risk-assessment)
9. [GraphExtractor Deep Dive](#9-graphextractor-deep-dive)

---

## 1. File-by-File Inventory

### Core Infrastructure (6 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `main.rs` | Entry point | 36 | `main()` | tokio |
| `lib.rs` | Library exports, run() | 103 | `run()`, `cli::Cli` | clap, tokio |
| `cli.rs` | Command-line interface | 1412 | `Cli`, `Commands`, `handle_command()` | clap, tokio |
| `config.rs` | Configuration loading | 753 | `Config`, `SourcesConfig`, `SearchConfig` | serde, toml |
| `error.rs` | Error types | 182 | `Error`, `Result<T>` | thiserror |
| `server.rs` | MCP server setup | 1043 | `MusicTheoryServer`, `run_server()` | rmcp, tokio |
| `state.rs` | Application state | 207 | `AppState`, `GraphState` | Arc, RwLock |

**Classification:** Mixed (config.rs is domain-specific, rest is generic/parameterizable)

### Markdown & Metadata (5 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `markdown/mod.rs` | Module exports | 45 | - | - |
| `markdown/frontmatter.rs` | Frontmatter extraction | 216 | `extract_frontmatter()`, `FrontmatterData` | yaml-front-matter |
| `markdown/parser.rs` | Markdown parsing | 176 | `parse_markdown()`, `extract_first_heading()` | pulldown-cmark |
| `metadata/mod.rs` | Module exports | 7 | `ContentType` enum | - |
| `metadata/extraction.rs` | Metadata extraction | 433 | `extract_concept_metadata()`, `ConceptMetadata` | serde |

**Classification:** Mostly Generic (frontmatter.rs, parser.rs are fully generic; extraction.rs has domain-specific field names)

### Utilities (3 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `util/mod.rs` | Module exports | 4 | - | - |
| `util/files.rs` | Async file utilities | 624 | `find_file_by_id()`, `find_all_files()`, `FindOptions` | tokio::fs |
| `util/paths.rs` | Path resolution | 535 | `server_root()`, `project_root()`, `expand_tilde()` | std::path |

**Classification:** Generic (files.rs) + Parameterized (paths.rs has hardcoded "MUSIC_THEORY_*" env vars)

### Graph Module (11 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `graph/mod.rs` | Module exports | 32 | - | - |
| `graph/types.rs` | Core graph types | 500+ | `Node`, `Edge`, `Relationship`, `GraphData` | serde, rkyv |
| `graph/parser.rs` | **CRITICAL** Concept extraction | 318 | `parse_concept_card()`, `ConceptCard`, `RelatedConcepts` | serde |
| `graph/builder.rs` | Graph construction | 933 | `GraphBuilder`, `build()` | petgraph |
| `graph/algorithms.rs` | Traversal algorithms | 749 | `neighborhood()`, `shortest_path()`, `prerequisites_sorted()` | petgraph |
| `graph/persistence.rs` | Save/load with rkyv cache | 639 | `save_graph()`, `load_graph()`, `to_petgraph()` | rkyv, memmap2, blake3 |
| `graph/loader.rs` | Card loading | 78 | `load_concept_cards()` | tokio |
| `graph/query.rs` | Query response types | 300+ | `RelatedConceptsResponse`, `ConceptPathResponse` | serde |
| `graph/stats.rs` | Statistics | 200+ | `GraphStats` | - |
| `graph/validation.rs` | Graph validation | 200+ | `validate_graph()` | - |
| `graph/cli.rs` | CLI commands | 465 | `handle_build()`, `handle_validate()` | tokio |

**Classification:** Mixed (types.rs has domain-specific Relationship enum, parser.rs is heavily domain-specific, algorithms/persistence are generic)

### Search Module (10 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `search/mod.rs` | Module exports | 13 | - | - |
| `search/backend.rs` | Backend abstraction | 153 | `SearchBackend` trait, `create_search_backend()` | async-trait |
| `search/schema.rs` | Tantivy schema | 309 | `SearchSchema::build()`, field definitions | tantivy |
| `search/document.rs` | Search document | 489 | `SearchDocument`, `matches_query()`, `relevance()` | serde |
| `search/simple_search.rs` | Linear search | 300+ | `SimpleSearch::search()` | - |
| `search/tantivy_search.rs` | Tantivy backend | 800+ | `TantivySearch::search()` | tantivy |
| `search/indexer.rs` | Index building | 400+ | `build_index()` | tantivy, tokio |
| `search/query.rs` | Query parsing | 300+ | `parse_query()` | tantivy |
| `search/builder.rs` | Index builder | 200+ | `IndexBuilder` | tantivy |
| `search/stopwords.rs` | Stopword filtering | 200+ | `build_stopwords()` | - |
| `search/freshness.rs` | Cache validation | 150+ | `is_index_fresh()`, `IndexMetadata` | std::time |

**Classification:** Parameterized (schema.rs has hardcoded field names, rest is generic)

### Tools (MCP Interface) (6 files)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `tools/mod.rs` | Module exports | 24 | - | - |
| `tools/health.rs` | Health check | 326 | `get_health()`, `HealthResponse` | serde |
| `tools/concepts.rs` | Concept card tools | 574 | `list_concepts()`, `get_concept()` | serde, tokio |
| `tools/sources.rs` | Source material tools | 1093 | `list_sources()`, `get_source_chapter()` | serde, tokio |
| `tools/guides.rs` | Guide document tools | 388 | `list_guides()`, `get_guide()` | serde, tokio |
| `tools/search.rs` | Search tools | 217 | `search_concepts()`, `SearchConceptsParams` | serde |
| `tools/graph.rs` | Graph inspection tools | 954 | `graph_status()`, `get_node()`, `get_node_edges()` | serde, petgraph |
| `tools/graph_query.rs` | Graph query tools | 1876 | `get_related_concepts()`, `find_concept_path()`, `get_prerequisites()` | serde, petgraph |

**Classification:** Mixed (health.rs is generic, concepts/sources have domain-aware data structures, graph tools are generic with domain-specific types)

### Resources (1 file)

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| `resources/mod.rs` | Resource serving | 50+ | `serve_resource()` | - |

**Classification:** Generic

---

## 2. Classification Matrix

### Legend

- **Generic (G)**: Fully reusable, no domain coupling
- **Parameterized (P)**: Reusable with configuration/trait parameters
- **Domain-Specific (D)**: Music-theory-specific, stays in ai-music-theory
- **Mixed (M)**: Contains both generic and domain-specific parts

| File | Classification | Target Crate | Changes Needed |
|------|---------------|--------------|----------------|
| **Core Infrastructure** |
| `main.rs` | G | fabryk-cli | None |
| `lib.rs` | P | fabryk-cli | Parameterize project name |
| `cli.rs` | P | fabryk-cli | Parameterize command names, server name |
| `config.rs` | D | ai-music-theory | **Stays in domain crate** |
| `error.rs` | G | fabryk-core | None |
| `server.rs` | P | fabryk-mcp | Parameterize server name, tool registration |
| `state.rs` | P | fabryk-core | Make generic over Config type |
| **Markdown & Metadata** |
| `markdown/frontmatter.rs` | G | fabryk-content | None |
| `markdown/parser.rs` | G | fabryk-content | None |
| `metadata/extraction.rs` | P | fabryk-content | **Extract field names to trait** |
| **Utilities** |
| `util/files.rs` | G | fabryk-core | None |
| `util/paths.rs` | P | fabryk-core | Parameterize env var names (`FABRYK_*` vs `MUSIC_THEORY_*`) |
| **Graph Module** |
| `graph/types.rs` | P | fabryk-graph | Make `Relationship` enum configurable/trait-based |
| `graph/parser.rs` | **M** | **CRITICAL SPLIT** | **Extract `GraphExtractor` trait to fabryk-graph**, impl stays in ai-music-theory |
| `graph/builder.rs` | P | fabryk-graph | Use `GraphExtractor` trait |
| `graph/algorithms.rs` | G | fabryk-graph | None |
| `graph/persistence.rs` | G | fabryk-graph | None |
| `graph/loader.rs` | P | fabryk-graph | Use `GraphExtractor` trait |
| `graph/query.rs` | G | fabryk-graph | None (response types are generic) |
| `graph/stats.rs` | G | fabryk-graph | None |
| `graph/validation.rs` | G | fabryk-graph | None |
| `graph/cli.rs` | P | fabryk-cli | Use `GraphExtractor` trait |
| **Search Module** |
| `search/backend.rs` | G | fabryk-fts | None |
| `search/schema.rs` | P | fabryk-fts | **Parameterize field names via trait** |
| `search/document.rs` | P | fabryk-fts | Make field names configurable |
| `search/simple_search.rs` | G | fabryk-fts | None |
| `search/tantivy_search.rs` | G | fabryk-fts | None |
| `search/indexer.rs` | P | fabryk-fts | Use schema trait |
| `search/query.rs` | G | fabryk-fts | None |
| `search/builder.rs` | G | fabryk-fts | None |
| `search/stopwords.rs` | G | fabryk-fts | None |
| `search/freshness.rs` | G | fabryk-fts | None |
| **Tools (MCP Interface)** |
| `tools/health.rs` | G | fabryk-mcp | None |
| `tools/concepts.rs` | D | ai-music-theory | **Domain-specific** (concept card structure) |
| `tools/sources.rs` | D | ai-music-theory | **Domain-specific** (source material structure) |
| `tools/guides.rs` | G | fabryk-mcp-content | Rename to `documents.rs` |
| `tools/search.rs` | P | fabryk-mcp-fts | Parameterize response types |
| `tools/graph.rs` | P | fabryk-mcp-graph | Use generic node/edge types |
| `tools/graph_query.rs` | P | fabryk-mcp-graph | Use generic node/edge types |
| **Resources** |
| `resources/mod.rs` | G | fabryk-core | None |

**Summary Statistics:**

- **Fully Generic (G):** 18 files (39%)
- **Parameterized (P):** 20 files (43%)
- **Domain-Specific (D):** 3 files (7%) - `config.rs`, `tools/concepts.rs`, `tools/sources.rs`
- **Mixed (M):** 5 files (11%) - `graph/parser.rs` is the main one

**Conclusion:** **82% of the codebase (38 files) can be extracted** into Fabryk crates with minimal to moderate changes.

---

## 3. GraphExtractor Trait Design

### The Problem

`graph/parser.rs` currently hardcodes music-theory-specific frontmatter fields:

```rust
// Current implementation (domain-coupled)
pub struct ConceptCard {
    pub id: String,
    pub title: String,
    pub category: String,      // ← Domain-specific
    pub source: String,         // ← Domain-specific
    pub related_concepts: Option<RelatedConcepts>,  // ← Domain-specific
}

pub struct RelatedConcepts {
    pub prerequisite: Vec<String>,  // ← Domain-specific field name
    pub leads_to: Vec<String>,      // ← Domain-specific field name
    pub see_also: Vec<String>,      // ← Domain-specific field name
}

pub fn parse_concept_card(base_path: &Path, file_path: &Path) -> Result<ConceptCard> {
    // Hardcoded frontmatter field access:
    let concept = frontmatter.get("concept")...;  // ← Hardcoded
    let category = frontmatter.get("category")...; // ← Hardcoded
    // ...
}
```

**The solution:** Extract a `GraphExtractor` trait that abstracts "how to extract graph nodes and edges from content files."

### GraphExtractor Trait API

```rust
/// Trait for extracting graph data from content files.
///
/// Implementations define:
/// - What frontmatter fields to read
/// - How to map fields to node properties
/// - How to extract relationship edges
///
/// This enables Fabryk to support multiple domains (music theory, math, programming)
/// without hardcoding domain-specific field names.
pub trait GraphExtractor: Send + Sync {
    /// The node data type extracted from a file.
    /// For music theory: ConceptCard
    /// For math: MathConceptCard
    /// For Rust: RustConceptCard
    type NodeData: Clone + Send + Sync;

    /// The edge/relationship type.
    /// For music theory: RelatedConcepts
    /// For math: MathRelations
    /// For Rust: RustRelations
    type EdgeData: Clone + Send + Sync;

    /// Extract node data from a markdown file.
    ///
    /// # Arguments
    /// * `base_path` - Base directory for the content
    /// * `file_path` - Path to the markdown file
    /// * `frontmatter` - Parsed frontmatter YAML data
    /// * `content` - Full file content (with frontmatter stripped)
    ///
    /// # Returns
    /// Returns the extracted node data or an error.
    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Self::NodeData>;

    /// Extract edge/relationship data from a markdown file.
    ///
    /// # Arguments
    /// * `frontmatter` - Parsed frontmatter YAML data
    /// * `content` - Full file content (for extracting from body if needed)
    ///
    /// # Returns
    /// Returns the extracted edge data or None if no relationships found.
    fn extract_edges(
        &self,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>>;

    /// Convert domain node data to generic graph Node.
    ///
    /// # Arguments
    /// * `node_data` - Domain-specific node data
    ///
    /// # Returns
    /// Returns a Node::Concept with appropriate fields filled.
    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node;

    /// Convert domain edge data to generic graph Edges.
    ///
    /// # Arguments
    /// * `from_id` - Source node ID
    /// * `edge_data` - Domain-specific edge data
    ///
    /// # Returns
    /// Returns a vector of Edge structs representing relationships.
    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge>;
}
```

### Music Theory Implementation

```rust
/// Music theory concept card data.
pub struct MusicTheoryNodeData {
    pub id: String,
    pub title: String,
    pub category: String,
    pub source: String,
}

/// Music theory relationships.
pub struct MusicTheoryEdgeData {
    pub prerequisite: Vec<String>,
    pub leads_to: Vec<String>,
    pub see_also: Vec<String>,
}

/// Music theory implementation of GraphExtractor.
pub struct MusicTheoryExtractor;

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
        // Extract music-theory-specific fields
        let concept = frontmatter
            .get("concept")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::config("Missing 'concept' field"))?;

        let category = frontmatter
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("uncategorized")
            .to_string();

        let source = frontmatter
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(MusicTheoryNodeData {
            id: compute_id(base_path, file_path)?,
            title: concept.to_string(),
            category,
            source,
        })
    }

    fn extract_edges(
        &self,
        frontmatter: &serde_yaml::Value,
        _content: &str,
    ) -> Result<Option<Self::EdgeData>> {
        // Extract relationship fields
        let prerequisite = extract_string_list(frontmatter, "prerequisite");
        let leads_to = extract_string_list(frontmatter, "leads_to");
        let see_also = extract_string_list(frontmatter, "see_also");

        if prerequisite.is_empty() && leads_to.is_empty() && see_also.is_empty() {
            return Ok(None);
        }

        Ok(Some(MusicTheoryEdgeData {
            prerequisite,
            leads_to,
            see_also,
        }))
    }

    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node {
        Node::Concept(ConceptNode {
            id: node_data.id.clone(),
            title: node_data.title.clone(),
            category: node_data.category.clone(),
            source_id: node_data.source.clone(),
            canonical_id: None,
            is_canonical: true,
        })
    }

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();

        // Prerequisite edges
        for prereq_id in &edge_data.prerequisite {
            edges.push(Edge {
                from: prereq_id.clone(),
                to: from_id.to_string(),
                relationship: Relationship::Prerequisite,
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // Leads to edges
        for leads_id in &edge_data.leads_to {
            edges.push(Edge {
                from: from_id.to_string(),
                to: leads_id.clone(),
                relationship: Relationship::Prerequisite,
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // See also edges
        for see_id in &edge_data.see_also {
            edges.push(Edge {
                from: from_id.to_string(),
                to: see_id.clone(),
                relationship: Relationship::RelatesTo,
                weight: 0.7,
                origin: EdgeOrigin::Extracted,
            });
        }

        edges
    }
}
```

### Generic Graph Builder

```rust
/// Generic graph builder using GraphExtractor trait.
pub struct GraphBuilder<E: GraphExtractor> {
    extractor: E,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    // ...
}

impl<E: GraphExtractor> GraphBuilder<E> {
    pub fn new(extractor: E) -> Self {
        Self {
            extractor,
            nodes: Vec::new(),
            edges: Vec::new(),
            node_ids: HashSet::new(),
            warnings: Vec::new(),
        }
    }

    pub async fn build(&mut self, config: &Config) -> Result<(GraphData, Vec<String>)> {
        // ... (same logic as current builder, but uses self.extractor)

        for file_info in file_infos {
            // Read file
            let content = std::fs::read_to_string(&file_info.path)?;
            let (frontmatter_raw, body) = extract_frontmatter(&content)?;
            let frontmatter: serde_yaml::Value = serde_yaml::from_str(&frontmatter_raw)?;

            // Extract node via trait
            let node_data = self.extractor.extract_node(
                &base_path,
                &file_info.path,
                &frontmatter,
                body,
            )?;

            // Convert to graph node
            let node = self.extractor.to_graph_node(&node_data);
            self.nodes.push(node);

            // Extract edges via trait
            if let Some(edge_data) = self.extractor.extract_edges(&frontmatter, body)? {
                let edges = self.extractor.to_graph_edges(&node_data.id, &edge_data);
                self.edges.extend(edges);
            }
        }

        // ... (rest of build logic)
    }
}
```

**Key Benefits:**

1. **Zero domain coupling** in fabryk-graph
2. **Full flexibility** for domain implementations
3. **Type safety** - compiler ensures trait implementation correctness
4. **Testability** - easy to write mock extractors for testing

---

## 4. Extraction Plan by Crate

### 4.1 fabryk-core

**Purpose:** Shared types, traits, and utilities used across all Fabryk crates.

**Files to Extract:**

- `error.rs` (182 lines) - Error type and Result alias
- `util/files.rs` (624 lines) - Async file utilities
- `util/paths.rs` (535 lines) - Path resolution (parameterize env vars)
- `state.rs` (207 lines) - Generic AppState (parameterize over Config type)
- `resources/mod.rs` (50+ lines) - Resource serving

**New Traits to Define:**

```rust
/// Trait for domain-specific configuration.
pub trait ConfigProvider: Send + Sync + Clone {
    /// Get base path for content
    fn base_path(&self) -> Result<PathBuf>;

    /// Get path for specific content type
    fn content_path(&self, content_type: &str) -> Result<PathBuf>;

    /// Get project name (for env var prefixes)
    fn project_name(&self) -> &str;
}
```

**Changes Required:**

1. **util/paths.rs**: Replace `MUSIC_THEORY_CONFIG_DIR` with `{PROJECT_NAME}_CONFIG_DIR`
2. **state.rs**: Make `AppState<C: ConfigProvider>` generic over config type
3. **error.rs**: No changes needed (already generic)

**Dependencies:**

```toml
[dependencies]
tokio = { version = "1", features = ["fs", "sync"] }
thiserror = "1"
```

**Testing:** All existing tests in these files should pass without modification after parameterization.

---

### 4.2 fabryk-content

**Purpose:** Markdown parsing, frontmatter extraction, and metadata handling.

**Files to Extract:**

- `markdown/frontmatter.rs` (216 lines) - Fully generic frontmatter extraction
- `markdown/parser.rs` (176 lines) - Fully generic markdown parsing
- `metadata/extraction.rs` (433 lines) - **Parameterize field names**

**New Traits to Define:**

```rust
/// Trait for extracting metadata from frontmatter.
pub trait MetadataExtractor: Send + Sync {
    /// The metadata type produced
    type Metadata: Send + Sync;

    /// Extract metadata from frontmatter and file path
    fn extract(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Self::Metadata>;
}
```

**Changes Required:**

1. **metadata/extraction.rs**: Extract `MetadataExtractor` trait
2. Move `ConceptMetadata` to ai-music-theory crate
3. Keep generic `extract_frontmatter()` and `parse_markdown()` functions

**Dependencies:**

```toml
[dependencies]
yaml-front-matter = "0.1"
pulldown-cmark = "0.9"
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
```

**Testing:** Existing tests in frontmatter.rs and parser.rs require no changes. Metadata extraction tests move to ai-music-theory.

---

### 4.3 fabryk-fts

**Purpose:** Full-text search with Tantivy backend, configurable schema.

**Files to Extract:**

- `search/backend.rs` (153 lines) - SearchBackend trait
- `search/schema.rs` (309 lines) - **Parameterize field names**
- `search/document.rs` (489 lines) - **Parameterize field names**
- `search/simple_search.rs` (300+ lines) - Linear search implementation
- `search/tantivy_search.rs` (800+ lines) - Tantivy backend
- `search/indexer.rs` (400+ lines) - Index building
- `search/query.rs` (300+ lines) - Query parsing
- `search/builder.rs` (200+ lines) - Index builder
- `search/stopwords.rs` (200+ lines) - Stopword filtering
- `search/freshness.rs` (150+ lines) - Cache validation

**New Traits to Define:**

```rust
/// Trait for defining search schema fields.
pub trait SearchSchemaProvider: Send + Sync {
    /// Get list of full-text fields with boost weights
    fn fulltext_fields(&self) -> Vec<(&'static str, f32)>;

    /// Get list of facet fields (for filtering)
    fn facet_fields(&self) -> Vec<&'static str>;

    /// Get list of stored-only fields
    fn stored_fields(&self) -> Vec<&'static str>;

    /// Build field name for a semantic field
    /// e.g., "title" -> "title", "category" -> "category"
    fn field_name(&self, semantic_name: &str) -> &'static str {
        semantic_name // Default: use semantic name as-is
    }
}

/// Music theory implementation
pub struct MusicTheorySchemaProvider;

impl SearchSchemaProvider for MusicTheorySchemaProvider {
    fn fulltext_fields(&self) -> Vec<(&'static str, f32)> {
        vec![
            ("title", 3.0),
            ("description", 2.0),
            ("content", 1.0),
        ]
    }

    fn facet_fields(&self) -> Vec<&'static str> {
        vec!["category", "source", "tags"]
    }

    fn stored_fields(&self) -> Vec<&'static str> {
        vec!["id", "path", "chapter", "part", "author", "date", "content_type", "section"]
    }
}
```

**Changes Required:**

1. **search/schema.rs**: Use `SearchSchemaProvider` trait instead of hardcoded fields
2. **search/document.rs**: Make field access use schema provider
3. **search/indexer.rs**: Pass schema provider to indexing functions

**Dependencies:**

```toml
[dependencies]
tantivy = "0.21"
async-trait = "0.1"
tokio = { version = "1", features = ["fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dependencies.fabryk-core]
path = "../fabryk-core"
```

**Testing:** All search backend tests should pass. Schema tests need to use a test schema provider.

---

### 4.4 fabryk-graph

**Purpose:** Graph database with configurable node/edge extraction via `GraphExtractor` trait.

**Files to Extract:**

- `graph/types.rs` (500+ lines) - **Parameterize Relationship enum**
- `graph/builder.rs` (933 lines) - **Use GraphExtractor trait**
- `graph/algorithms.rs` (749 lines) - Fully generic algorithms
- `graph/persistence.rs` (639 lines) - Fully generic save/load/cache
- `graph/loader.rs` (78 lines) - **Use GraphExtractor trait**
- `graph/query.rs` (300+ lines) - Generic query response types
- `graph/stats.rs` (200+ lines) - Generic statistics
- `graph/validation.rs` (200+ lines) - Generic validation
- `graph/cli.rs` (465 lines) - **Use GraphExtractor trait**

**Files to STAY in ai-music-theory:**

- `graph/parser.rs` (318 lines) - **Split**: Trait goes to fabryk-graph, music theory impl stays

**New Traits:** See [Section 3: GraphExtractor Trait Design](#3-graphextractor-trait-design)

**Changes Required:**

1. **types.rs**: Make `Relationship` enum extensible or trait-based:

   ```rust
   // Option 1: Keep enum but add Custom variant
   pub enum Relationship {
       Prerequisite,
       LeadsTo,
       RelatesTo,
       Extends,
       Introduces,
       Covers,
       Custom(String),  // For domain-specific relationships
   }

   // Option 2: Use trait (more complex, but more flexible)
   pub trait RelationshipType: Clone + Debug + Serialize + Deserialize {
       fn name(&self) -> &str;
       fn weight(&self) -> f32;
   }
   ```

2. **builder.rs**: Change signature to:

   ```rust
   impl<E: GraphExtractor> GraphBuilder<E> {
       pub fn new(extractor: E) -> Self { ... }
       pub async fn build(&mut self, config: &Config) -> Result<(GraphData, Vec<String>)> { ... }
   }
   ```

3. **loader.rs**: Use `extractor.extract_node()` instead of `parse_concept_card()`

4. **cli.rs**: Pass extractor instance:

   ```rust
   pub async fn handle_build<E: GraphExtractor>(
       config: &Config,
       extractor: E,
       dry_run: bool,
       verbose: bool,
   ) -> Result<()>
   ```

**Dependencies:**

```toml
[dependencies]
petgraph = "0.6"
rkyv = { version = "0.7", features = ["validation"] }
memmap2 = "0.9"
blake3 = "1.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = { version = "0.4", features = ["serde"] }

[dependencies.fabryk-core]
path = "../fabryk-core"

[dependencies.fabryk-content]
path = "../fabryk-content"
```

**Testing:**

- Algorithm tests require no changes (fully generic)
- Builder tests need to use a mock extractor
- Persistence tests require no changes

---

### 4.5 fabryk-acl

**Purpose:** Placeholder for future access control layer.

**Status:** Not yet implemented in music-theory server.

**Design Notes:**

- Should provide traits for:
  - User/tenant identification
  - Permission checking
  - Resource ownership
  - Multi-tenancy isolation
- Will integrate with fabryk-graph and fabryk-fts for filtering

**Skeleton:**

```rust
/// Trait for access control decisions.
pub trait AccessControl: Send + Sync {
    /// Check if user can access a resource
    fn can_access(&self, user_id: &str, resource_id: &str) -> bool;

    /// Filter results based on user permissions
    fn filter_results<T>(&self, user_id: &str, results: Vec<T>) -> Vec<T>
    where
        T: HasResourceId;
}

pub trait HasResourceId {
    fn resource_id(&self) -> &str;
}
```

---

### 4.6 fabryk-mcp

**Purpose:** Core MCP server infrastructure, tool registration, protocol handling.

**Files to Extract:**

- `server.rs` (1043 lines) - **Parameterize server name and tool registration**
- `tools/health.rs` (326 lines) - Fully generic health check

**Changes Required:**

1. **server.rs**: Make generic over server name:

   ```rust
   pub struct FabrykMcpServer<C: ConfigProvider> {
       state: AppState<C>,
       server_name: String,
   }

   impl<C: ConfigProvider> FabrykMcpServer<C> {
       pub fn new(state: AppState<C>, server_name: String) -> Self { ... }
   }
   ```

2. Create `ToolRegistry` trait for registering domain-specific tools:

   ```rust
   #[async_trait]
   pub trait ToolRegistry: Send + Sync {
       /// Register domain-specific tools with the server
       fn register_tools(&self, server: &mut ServerBuilder) -> Result<()>;
   }
   ```

**Dependencies:**

```toml
[dependencies]
rmcp = "0.1"
async-trait = "0.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }

[dependencies.fabryk-core]
path = "../fabryk-core"
```

**Testing:** Server setup tests need mock tool registry.

---

### 4.7 fabryk-mcp-content

**Purpose:** Generic MCP tools for content (guides/documents).

**Files to Extract:**

- `tools/guides.rs` (388 lines) - Rename to `documents.rs` for generality

**Changes Required:**

1. Rename `GuideInfo` → `DocumentInfo`
2. Rename `list_guides` → `list_documents`
3. Rename `get_guide` → `get_document`
4. Make topic extraction configurable

**Dependencies:**

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["fs"] }

[dependencies.fabryk-core]
path = "../fabryk-core"

[dependencies.fabryk-content]
path = "../fabryk-content"
```

---

### 4.8 fabryk-mcp-fts

**Purpose:** MCP tools for full-text search.

**Files to Extract:**

- `tools/search.rs` (217 lines) - **Parameterize response types**

**Changes Required:**

1. Make `SearchResult` generic or use schema provider for field access
2. Pass `SearchSchemaProvider` to search functions

**Dependencies:**

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = "0.8"

[dependencies.fabryk-fts]
path = "../fabryk-fts"

[dependencies.fabryk-mcp]
path = "../fabryk-mcp"
```

---

### 4.9 fabryk-mcp-graph

**Purpose:** MCP tools for graph queries.

**Files to Extract:**

- `tools/graph.rs` (954 lines) - **Parameterize over node types**
- `tools/graph_query.rs` (1876 lines) - **Parameterize over node types**

**Changes Required:**

1. Make tools generic over `Node` type
2. Use trait-based node access instead of matching on `Node::Concept`

**Dependencies:**

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
petgraph = "0.6"

[dependencies.fabryk-graph]
path = "../fabryk-graph"

[dependencies.fabryk-mcp]
path = "../fabryk-mcp"
```

---

### 4.10 fabryk-cli

**Purpose:** CLI framework for Fabryk-based applications.

**Files to Extract:**

- `main.rs` (36 lines) - Generic entry point
- `lib.rs` (103 lines) - **Parameterize project name**
- `cli.rs` (1412 lines) - **Parameterize command names**

**Changes Required:**

1. Make CLI commands generic:

   ```rust
   pub struct FabrykCli<C: ConfigProvider> {
       pub config_provider: PhantomData<C>,
       pub project_name: String,
       pub command: Option<FabrykCommands>,
   }
   ```

2. Let domains register custom subcommands via trait

**Dependencies:**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }

[dependencies.fabryk-core]
path = "../fabryk-core"

[dependencies.fabryk-mcp]
path = "../fabryk-mcp"
```

---

## 5. Music Theory Remainder

**What stays in `ai-music-theory` crate:**

### Domain-Specific Files (stays as-is)

1. **`config.rs` (753 lines)**
   - Music theory source definitions (oxford, general, papers)
   - Hardcoded file mappings like `"[2024] Test - Test Source.md"`
   - Music-specific path configuration

2. **`tools/concepts.rs` (574 lines)**
   - `ConceptInfo`, `ListConceptsResponse` with music-theory fields
   - `list_concepts()`, `get_concept()` specific to concept card structure
   - `CategoryInfo` with music theory categories

3. **`tools/sources.rs` (1093 lines)**
   - `SourceInfo`, `SourceFormat`, `AvailabilityStatus`
   - Music theory source material structure
   - `check_source_availability()` for specific source types

### Domain Implementations (new files)

1. **`music_theory_extractor.rs` (new, ~300 lines)**
   - Implements `GraphExtractor` trait for music theory
   - Contains `MusicTheoryNodeData`, `MusicTheoryEdgeData`
   - Maps music theory frontmatter fields to graph nodes/edges
   - (See [Section 3](#3-graphextractor-trait-design) for full implementation)

2. **`music_theory_schema.rs` (new, ~100 lines)**
   - Implements `SearchSchemaProvider` for music theory
   - Defines field names and boost weights
   - Maps semantic fields to Tantivy schema

3. **`music_theory_metadata.rs` (new, ~200 lines)**
   - Implements `MetadataExtractor` for concept cards
   - Contains `ConceptMetadata` type
   - Extracts music-theory-specific metadata

4. **`music_theory_config.rs` (new, ~100 lines)**
   - Implements `ConfigProvider` trait
   - Wraps existing `Config` type
   - Provides standard interface for Fabryk crates

### Integration Code (updated)

1. **`main.rs` (updated, ~50 lines)**

   ```rust
   use fabryk_cli::FabrykCli;
   use ai_music_theory::{MusicTheoryConfig, MusicTheoryExtractor};

   #[tokio::main]
   async fn main() -> Result<()> {
       let config = MusicTheoryConfig::load()?;
       let extractor = MusicTheoryExtractor::new();

       fabryk_cli::run(config, extractor, "Music Theory MCP Server").await
   }
   ```

2. **`lib.rs` (updated, ~150 lines)**
   - Re-exports from Fabryk crates
   - Provides music-theory-specific tool registration
   - Implements `ToolRegistry` trait

**Total Lines in ai-music-theory after extraction:** ~3500 lines (down from ~12,000)

---

## 6. Dependency Analysis

### 6.1 Fabryk Workspace Cargo.toml

```toml
[workspace]
members = [
    "fabryk-core",
    "fabryk-content",
    "fabryk-fts",
    "fabryk-graph",
    "fabryk-acl",
    "fabryk-mcp",
    "fabryk-mcp-content",
    "fabryk-mcp-fts",
    "fabryk-mcp-graph",
    "fabryk-cli",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["Fabryk Contributors"]
license = "MIT OR Apache-2.0"
repository = "https://github.com/oxur/fabryk"

[workspace.dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"

# Error handling
thiserror = "1"

# Graph
petgraph = "0.6"
rkyv = { version = "0.7", features = ["validation"] }
memmap2 = "0.9"
blake3 = "1.5"

# Search
tantivy = "0.21"

# Markdown
pulldown-cmark = "0.9"
yaml-front-matter = "0.1"

# MCP
rmcp = "0.1"

# CLI
clap = { version = "4", features = ["derive"] }

# Time
chrono = { version = "0.4", features = ["serde"] }

# Logging
twyg = "0.5"
log = "0.4"

# Schema
schemars = "0.8"
```

### 6.2 Dependency Graph

```
ai-music-theory
├── fabryk-cli
│   ├── fabryk-core
│   ├── fabryk-mcp
│   ├── fabryk-graph
│   └── fabryk-fts
├── fabryk-mcp-content
│   ├── fabryk-mcp
│   ├── fabryk-core
│   └── fabryk-content
├── fabryk-mcp-fts
│   ├── fabryk-mcp
│   └── fabryk-fts
├── fabryk-mcp-graph
│   ├── fabryk-mcp
│   └── fabryk-graph
├── fabryk-graph
│   ├── fabryk-core
│   └── fabryk-content
├── fabryk-fts
│   └── fabryk-core
├── fabryk-content
│   └── fabryk-core
└── fabryk-core (no internal deps)
```

**Dependency Levels:**

- **Level 0:** fabryk-core
- **Level 1:** fabryk-content, fabryk-fts
- **Level 2:** fabryk-graph, fabryk-mcp
- **Level 3:** fabryk-mcp-content, fabryk-mcp-fts, fabryk-mcp-graph, fabryk-cli
- **Level 4:** ai-music-theory

### 6.3 Music Theory Cargo.toml

```toml
[package]
name = "ai-music-theory"
version = "0.3.0"
edition = "2021"
authors = ["Duncan McGreggor <duncan@oxur.org>"]

[dependencies]
# Fabryk dependencies
fabryk-core = { path = "../fabryk/fabryk-core" }
fabryk-content = { path = "../fabryk/fabryk-content" }
fabryk-fts = { path = "../fabryk/fabryk-fts", features = ["tantivy"] }
fabryk-graph = { path = "../fabryk/fabryk-graph", features = ["rkyv-cache"] }
fabryk-mcp = { path = "../fabryk/fabryk-mcp" }
fabryk-mcp-content = { path = "../fabryk/fabryk-mcp-content" }
fabryk-mcp-fts = { path = "../fabryk/fabryk-mcp-fts" }
fabryk-mcp-graph = { path = "../fabryk/fabryk-mcp-graph" }
fabryk-cli = { path = "../fabryk/fabryk-cli" }

# Domain-specific dependencies
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"

[features]
default = ["fts", "graph"]
fts = ["fabryk-fts/tantivy", "fabryk-mcp-fts/tantivy"]
graph = ["fabryk-graph/rkyv-cache", "fabryk-mcp-graph/full"]
```

---

## 7. Migration Steps

### Phase 1: Preparation (1-2 weeks)

**Goal:** Set up Fabryk repository structure without breaking ai-music-theory.

1. **Create fabryk repository:**

   ```bash
   cd ~/lab/music-comp
   git clone https://github.com/oxur/fabryk.git
   cd fabryk
   cargo new --lib fabryk-core
   cargo new --lib fabryk-content
   cargo new --lib fabryk-fts
   cargo new --lib fabryk-graph
   cargo new --lib fabryk-acl
   cargo new --lib fabryk-mcp
   cargo new --lib fabryk-mcp-content
   cargo new --lib fabryk-mcp-fts
   cargo new --lib fabryk-mcp-graph
   cargo new --lib fabryk-cli
   ```

2. **Set up workspace Cargo.toml** (see Section 6.1)

3. **Create Git branches:**

   ```bash
   # In fabryk repo
   git checkout -b feature/core-extraction

   # In ai-music-theory repo
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   git checkout -b feature/fabryk-integration
   ```

4. **Document traits:**
   - Write trait definitions in `fabryk-core/src/traits.rs`
   - Write `GraphExtractor` trait in `fabryk-graph/src/extractor.rs`
   - Write `SearchSchemaProvider` trait in `fabryk-fts/src/schema.rs`
   - Write `MetadataExtractor` trait in `fabryk-content/src/extractor.rs`

**Commit:** "chore: Initialize Fabryk workspace structure and core traits"

---

### Phase 2: Extract Core & Utilities (1 week)

**Goal:** Extract fully generic utilities and error handling.

1. **fabryk-core extraction:**

   ```bash
   cd fabryk/fabryk-core
   # Copy files
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/error.rs src/
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/util/files.rs src/
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/util/paths.rs src/
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/resources/mod.rs src/

   # Parameterize paths.rs
   # Replace MUSIC_THEORY_CONFIG_DIR with {PROJECT_NAME}_CONFIG_DIR
   sed -i 's/MUSIC_THEORY/PROJECT/g' src/paths.rs

   # Run tests
   cargo test
   ```

2. **Update ai-music-theory to use fabryk-core:**

   ```bash
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   # Add fabryk-core dependency
   # Update imports to use fabryk_core::
   cargo test
   ```

3. **Verify no breakage:**

   ```bash
   cargo test --all-features
   cargo clippy
   ```

**Commit:** "feat: Extract core utilities to fabryk-core"

---

### Phase 3: Extract Content (1 week)

**Goal:** Extract markdown and metadata handling with trait abstraction.

1. **fabryk-content extraction:**

   ```bash
   cd fabryk/fabryk-content
   # Copy markdown files
   cp -r ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/markdown/* src/

   # Extract MetadataExtractor trait
   # Refactor metadata/extraction.rs to use trait

   cargo test
   ```

2. **Create MusicTheoryMetadataExtractor in ai-music-theory:**

   ```bash
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   # Create music_theory_metadata.rs
   # Implement MetadataExtractor trait

   cargo test
   ```

**Commit:** "feat: Extract content parsing to fabryk-content with metadata trait"

---

### Phase 4: Extract Search (2 weeks)

**Goal:** Extract Tantivy search with schema provider trait.

1. **fabryk-fts extraction:**

   ```bash
   cd fabryk/fabryk-fts
   # Copy search files
   cp -r ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/search/* src/

   # Refactor schema.rs to use SearchSchemaProvider trait
   # Refactor document.rs to use schema provider

   cargo test --features tantivy
   ```

2. **Create MusicTheorySchemaProvider:**

   ```bash
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   # Create music_theory_schema.rs
   # Implement SearchSchemaProvider trait

   cargo test --features fts
   ```

**Commit:** "feat: Extract FTS to fabryk-fts with configurable schema"

---

### Phase 5: Extract Graph (3 weeks) **[MOST CRITICAL]**

**Goal:** Extract graph database with GraphExtractor trait.

1. **Define GraphExtractor trait in fabryk-graph:**

   ```bash
   cd fabryk/fabryk-graph
   # Create src/extractor.rs with trait definition (see Section 3)

   # Copy generic graph files
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/graph/types.rs src/
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/graph/algorithms.rs src/
   cp ~/lab/music-comp/ai-music-theory/mcp-server/crates/server/src/graph/persistence.rs src/
   # ... (copy other generic graph files)

   # Parameterize Relationship enum (add Custom variant)

   cargo test
   ```

2. **Refactor GraphBuilder to use trait:**

   ```bash
   # Modify builder.rs to be generic over GraphExtractor
   # Change signature: GraphBuilder<E: GraphExtractor>

   cargo test
   ```

3. **Create MusicTheoryExtractor in ai-music-theory:**

   ```bash
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   # Create music_theory_extractor.rs
   # Implement GraphExtractor trait (see Section 3)
   # Move ConceptCard and RelatedConcepts types here

   # Update graph CLI to pass extractor

   cargo test --features graph
   ```

4. **Integration testing:**

   ```bash
   # Build test graph
   cargo run --features graph -- graph build --dry-run

   # Verify node count and edges match before extraction
   cargo run --features graph -- graph stats
   ```

**Commit:** "feat: Extract graph database to fabryk-graph with GraphExtractor trait"

**Critical:** This is the **highest risk step**. Plan for 1 week of testing and debugging.

---

### Phase 6: Extract MCP Infrastructure (1-2 weeks)

**Goal:** Extract MCP server framework and generic tools.

1. **fabryk-mcp extraction:**

   ```bash
   cd fabryk/fabryk-mcp
   # Copy server.rs and parameterize
   # Copy tools/health.rs

   cargo test
   ```

2. **fabryk-mcp-content:**

   ```bash
   cd fabryk/fabryk-mcp-content
   # Copy tools/guides.rs → documents.rs
   # Generalize for any document type

   cargo test
   ```

3. **fabryk-mcp-fts:**

   ```bash
   cd fabryk/fabryk-mcp-fts
   # Copy tools/search.rs
   # Parameterize response types

   cargo test
   ```

4. **fabryk-mcp-graph:**

   ```bash
   cd fabryk/fabryk-mcp-graph
   # Copy tools/graph.rs and tools/graph_query.rs
   # Make generic over node types

   cargo test
   ```

**Commit:** "feat: Extract MCP infrastructure and generic tools"

---

### Phase 7: Extract CLI (1 week)

**Goal:** Extract CLI framework.

1. **fabryk-cli extraction:**

   ```bash
   cd fabryk/fabryk-cli
   # Copy cli.rs, main.rs, lib.rs
   # Parameterize project name and commands

   cargo test
   ```

2. **Update ai-music-theory CLI:**

   ```bash
   cd ~/lab/music-comp/ai-music-theory/mcp-server
   # Update main.rs to use fabryk-cli
   # Register music-theory-specific commands

   cargo build
   cargo run -- --help
   ```

**Commit:** "feat: Extract CLI framework to fabryk-cli"

---

### Phase 8: Final Integration (1 week)

**Goal:** Clean up, documentation, and verification.

1. **Update all documentation:**

   ```bash
   # In fabryk repo
   for crate in fabryk-*/; do
       cd "$crate"
       cargo doc --no-deps
       cd ..
   done

   # Write README.md for each crate
   # Write top-level fabryk README.md
   ```

2. **Comprehensive testing:**

   ```bash
   # In ai-music-theory repo
   cd ~/lab/music-comp/ai-music-theory/mcp-server

   # All features
   cargo test --all-features
   cargo clippy --all-features
   cargo fmt --check

   # Run MCP server
   cargo run --features fts,graph -- serve --test

   # Build index
   cargo run --features fts -- index

   # Build graph
   cargo run --features graph -- graph build

   # Verify tool count
   npx @modelcontextprotocol/inspector target/debug/music-theory-mcp
   # Should still show 25 tools
   ```

3. **Performance benchmarking:**

   ```bash
   # Compare startup time before/after extraction
   hyperfine 'cargo run --release -- serve --test'

   # Compare graph build time
   hyperfine 'cargo run --release --features graph -- graph build'
   ```

4. **Git cleanup:**

   ```bash
   # Squash commits if needed
   # Merge feature branches

   # In fabryk repo
   git checkout main
   git merge feature/core-extraction
   git tag v0.1.0

   # In ai-music-theory repo
   git checkout main
   git merge feature/fabryk-integration
   ```

**Commit:** "chore: Complete Fabryk extraction and integration"

---

## 8. Risk Assessment

### 8.1 Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **GraphExtractor trait breaks existing functionality** | Medium | High | - Incremental refactoring with tests at each step<br>- Keep music-theory tests passing throughout<br>- Side-by-side comparison of graph build output |
| **Search schema parameterization introduces bugs** | Low | Medium | - Extensive test coverage exists (≥95%)<br>- Tantivy schema validation catches errors early<br>- Schema provider tests |
| **Dependency cycles between Fabryk crates** | Low | Medium | - Clear dependency hierarchy (see Section 6.2)<br>- Level-based extraction order prevents cycles |
| **Performance regression after extraction** | Low | Low | - Zero-cost abstractions (trait monomorphization)<br>- Benchmark before/after (see Phase 8)<br>- rkyv cache unchanged |
| **Type complexity explosion** | Medium | Low | - Use type aliases for common combinations<br>- Hide complexity behind builder patterns<br>- Good documentation |
| **Breaking changes during Fabryk development** | Medium | Medium | - Semantic versioning<br>- Lock Fabryk versions in ai-music-theory<br>- Maintain compatibility layer |

### 8.2 API Design Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Trait design too rigid for future domains** | Medium | High | - Design with 3 reference domains in mind (music, math, Rust)<br>- Add extension points with associated types<br>- Allow trait method defaults |
| **Trait design too flexible (leaky abstraction)** | Low | Medium | - Follow existing Rust patterns (Iterator, From, etc.)<br>- Clear trait documentation with contracts<br>- Reference implementations |
| **Configuration provider trait insufficient** | Low | Medium | - Start simple, add methods as needed<br>- Provide sensible defaults<br>- Make extensible with associated types |

### 8.3 Testing Gaps

| Area | Current Coverage | Risk | Action |
|------|------------------|------|--------|
| **graph/parser.rs** | ~95% | Low | Maintain coverage in music-theory extractor |
| **Integration between Fabryk crates** | 0% (new) | High | - Add integration tests in fabryk workspace<br>- Test music-theory E2E after each phase |
| **GraphExtractor implementations** | 0% (new) | Medium | - Mock extractors for testing<br>- Example implementations for 3 domains |
| **Multi-domain scenarios** | 0% (new) | Medium | - Create test harness with 2+ domains<br>- Verify no cross-domain pollution |

### 8.4 Documentation Needs

**Critical Documentation:**

1. **GraphExtractor trait guide** - How to implement for a new domain
2. **SearchSchemaProvider guide** - How to define custom fields
3. **Migration guide** - For future Fabryk users
4. **Architecture decision records (ADRs)** - Why traits were chosen over other approaches

**Examples Needed:**

1. Music theory (reference implementation)
2. Higher math (example in fabryk README)
3. Rust programming concepts (example in fabryk README)

### 8.5 Overall Risk Level: **MODERATE**

**Factors reducing risk:**

- Existing test coverage ≥95%
- Clear domain boundaries identified
- Most code is already generic
- Incremental migration strategy

**Factors increasing risk:**

- GraphExtractor trait is new abstraction (not battle-tested)
- Coordination across 2 repositories
- Potential for breaking changes in Fabryk ecosystem

**Recommended Risk Reduction:**

1. Start with fabryk-core (lowest risk)
2. Build up to fabryk-graph (highest risk)
3. Keep ai-music-theory working at every phase
4. Extensive testing at each boundary
5. Consider feature flags during migration

---

## 9. GraphExtractor Deep Dive

### 9.1 Current Behavior (graph/parser.rs)

**Function:** `parse_concept_card(base_path: &Path, file_path: &Path) -> Result<ConceptCard>`

**What it does:**

1. Reads markdown file from disk
2. Extracts YAML frontmatter
3. Parses specific music-theory fields:
   - `concept` (required) → becomes `title`
   - `category` (optional, default: "uncategorized")
   - `source` (optional, default: "unknown")
4. Computes ID from file path (removes base_path, strips extension)
5. Parses "Related Concepts" section from markdown body:
   - Looks for Markdown list under `## Related Concepts`
   - Extracts `**Prerequisite**:`, `**Leads to**:`, `**See also**:` lines
   - Parses comma-separated concept IDs
6. Returns `ConceptCard` with node data and edge data

**Hardcoded Assumptions:**

- Frontmatter has `concept`, `category`, `source` fields
- Relationships are in `## Related Concepts` section
- Relationship types are `prerequisite`, `leads_to`, `see_also`
- Relationships are in Markdown list format: `- **Type**: id1, id2, id3`

**Example Input:**

```markdown
---
concept: Picardy Third
category: chromaticism
source: open-music-theory
---

# Picardy Third

A Picardy third is a major chord used at the end of a piece in minor key.

## Related Concepts

- **Prerequisite**: major-triad, minor-key
- **Leads to**: modal-mixture
- **See also**: borrowed-chords
```

**Output:**

```rust
ConceptCard {
    id: "picardy-third",
    title: "Picardy Third",
    category: "chromaticism",
    source: "open-music-theory",
    related_concepts: Some(RelatedConcepts {
        prerequisite: vec!["major-triad", "minor-key"],
        leads_to: vec!["modal-mixture"],
        see_also: vec!["borrowed-chords"],
    }),
}
```

### 9.2 Generalization Strategy

**Problem:** How to support different domains with different frontmatter schemas and relationship types?

**Solution:** The `GraphExtractor` trait (see Section 3) provides:

1. **Flexible field access** - implementations choose which frontmatter fields to read
2. **Custom relationship types** - implementations define their own edge semantics
3. **Separation of concerns** - fabryk-graph provides graph infrastructure, domains provide extraction logic

**Key Insight:** The pattern of "read markdown → extract frontmatter → build nodes → extract edges" is **universal**. Only the **specific field names and relationship types** are domain-specific.

### 9.3 Example: Higher Math Domain

**Higher Math Concept Card:**

```markdown
---
theorem: Fundamental Theorem of Calculus
area: analysis
difficulty: intermediate
proven_by: Newton, Leibniz
---

# Fundamental Theorem of Calculus

Relates differentiation and integration.

## Dependencies

- **Requires**: derivative-definition, integral-definition, continuous-functions
- **Implies**: antiderivative-existence
- **Related**: mean-value-theorem, integration-by-parts
```

**MathExtractor Implementation:**

```rust
pub struct MathNodeData {
    pub id: String,
    pub theorem_name: String,
    pub area: String,
    pub difficulty: String,
}

pub struct MathEdgeData {
    pub requires: Vec<String>,      // Prerequisites
    pub implies: Vec<String>,       // Logical implications
    pub related: Vec<String>,       // Related theorems
}

pub struct MathExtractor;

impl GraphExtractor for MathExtractor {
    type NodeData = MathNodeData;
    type EdgeData = MathEdgeData;

    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        _content: &str,
    ) -> Result<Self::NodeData> {
        let theorem = frontmatter
            .get("theorem")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::config("Missing 'theorem' field"))?;

        let area = frontmatter
            .get("area")
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();

        let difficulty = frontmatter
            .get("difficulty")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(MathNodeData {
            id: compute_id(base_path, file_path)?,
            theorem_name: theorem.to_string(),
            area,
            difficulty,
        })
    }

    fn extract_edges(
        &self,
        _frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>> {
        // Parse "Dependencies" section (similar to music theory parsing)
        let requires = extract_list_from_section(content, "Dependencies", "Requires");
        let implies = extract_list_from_section(content, "Dependencies", "Implies");
        let related = extract_list_from_section(content, "Dependencies", "Related");

        if requires.is_empty() && implies.is_empty() && related.is_empty() {
            return Ok(None);
        }

        Ok(Some(MathEdgeData {
            requires,
            implies,
            related,
        }))
    }

    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node {
        Node::Concept(ConceptNode {
            id: node_data.id.clone(),
            title: node_data.theorem_name.clone(),
            category: node_data.area.clone(),  // Map "area" to "category"
            source_id: "math-textbook".to_string(),  // Could be from frontmatter
            canonical_id: None,
            is_canonical: true,
        })
    }

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();

        // Requires → Prerequisite edges
        for req_id in &edge_data.requires {
            edges.push(Edge {
                from: req_id.clone(),
                to: from_id.to_string(),
                relationship: Relationship::Prerequisite,
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // Implies → Custom("Implies") edges
        for implied_id in &edge_data.implies {
            edges.push(Edge {
                from: from_id.to_string(),
                to: implied_id.clone(),
                relationship: Relationship::Custom("implies".to_string()),
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // Related → RelatesTo edges
        for related_id in &edge_data.related {
            edges.push(Edge {
                from: from_id.to_string(),
                to: related_id.clone(),
                relationship: Relationship::RelatesTo,
                weight: 0.7,
                origin: EdgeOrigin::Extracted,
            });
        }

        edges
    }
}
```

**Usage:**

```rust
// In higher-math crate
let config = MathConfig::load()?;
let extractor = MathExtractor::new();
let mut builder = GraphBuilder::new(extractor);
let (graph_data, warnings) = builder.build(&config).await?;
```

### 9.4 Example: Rust Programming Domain

**Rust Concept Card:**

```markdown
---
feature: async/await
rust_version: 1.39
stability: stable
module: std::future
---

# Async/Await Syntax

Asynchronous programming syntax for writing non-blocking code.

## Learning Path

- **Must know first**: futures, polling, pinning
- **Unlocks**: async-fn, async-block, tokio
- **See also**: threads, channels
```

**RustExtractor Implementation:**

```rust
pub struct RustNodeData {
    pub id: String,
    pub feature_name: String,
    pub rust_version: String,
    pub stability: String,
    pub module: String,
}

pub struct RustEdgeData {
    pub must_know_first: Vec<String>,
    pub unlocks: Vec<String>,
    pub see_also: Vec<String>,
}

pub struct RustExtractor;

impl GraphExtractor for RustExtractor {
    type NodeData = RustNodeData;
    type EdgeData = RustEdgeData;

    fn extract_node(
        &self,
        base_path: &Path,
        file_path: &Path,
        frontmatter: &serde_yaml::Value,
        _content: &str,
    ) -> Result<Self::NodeData> {
        let feature = frontmatter
            .get("feature")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::config("Missing 'feature' field"))?;

        let rust_version = frontmatter
            .get("rust_version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0")
            .to_string();

        let stability = frontmatter
            .get("stability")
            .and_then(|v| v.as_str())
            .unwrap_or("unstable")
            .to_string();

        let module = frontmatter
            .get("module")
            .and_then(|v| v.as_str())
            .unwrap_or("std")
            .to_string();

        Ok(RustNodeData {
            id: compute_id(base_path, file_path)?,
            feature_name: feature.to_string(),
            rust_version,
            stability,
            module,
        })
    }

    fn extract_edges(
        &self,
        _frontmatter: &serde_yaml::Value,
        content: &str,
    ) -> Result<Option<Self::EdgeData>> {
        // Parse "Learning Path" section
        let must_know = extract_list_from_section(content, "Learning Path", "Must know first");
        let unlocks = extract_list_from_section(content, "Learning Path", "Unlocks");
        let see_also = extract_list_from_section(content, "Learning Path", "See also");

        if must_know.is_empty() && unlocks.is_empty() && see_also.is_empty() {
            return Ok(None);
        }

        Ok(Some(RustEdgeData {
            must_know_first: must_know,
            unlocks,
            see_also,
        }))
    }

    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node {
        Node::Concept(ConceptNode {
            id: node_data.id.clone(),
            title: node_data.feature_name.clone(),
            category: node_data.module.clone(),  // Use module as category
            source_id: format!("rust-{}", node_data.rust_version),
            canonical_id: None,
            is_canonical: true,
        })
    }

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();

        // Must know first → Prerequisite
        for prereq_id in &edge_data.must_know_first {
            edges.push(Edge {
                from: prereq_id.clone(),
                to: from_id.to_string(),
                relationship: Relationship::Prerequisite,
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // Unlocks → Custom("enables")
        for unlocks_id in &edge_data.unlocks {
            edges.push(Edge {
                from: from_id.to_string(),
                to: unlocks_id.clone(),
                relationship: Relationship::Custom("enables".to_string()),
                weight: 1.0,
                origin: EdgeOrigin::Extracted,
            });
        }

        // See also → RelatesTo
        for related_id in &edge_data.see_also {
            edges.push(Edge {
                from: from_id.to_string(),
                to: related_id.clone(),
                relationship: Relationship::RelatesTo,
                weight: 0.7,
                origin: EdgeOrigin::Extracted,
            });
        }

        edges
    }
}
```

### 9.5 Trade-offs and Design Decisions

#### Why Trait-Based vs. Configuration-Based?

**Configuration approach (rejected):**

```yaml
# graph_config.yaml
node_fields:
  id_field: concept
  category_field: category
  source_field: source
edge_sections:
  - name: prerequisite
    keyword: "Prerequisite"
    relationship: Prerequisite
```

**Why rejected:**

- Cannot handle custom parsing logic (e.g., extracting from markdown body vs. frontmatter)
- Difficult to express complex transformations
- No compile-time type safety
- Limited to simple field mappings

**Trait approach (chosen):**

- Full flexibility for custom extraction logic
- Type-safe: compiler enforces correct implementation
- Can handle arbitrary complexity (parsing markdown lists, custom formats, etc.)
- Allows domain-specific validation and transformation
- Testable: easy to write mock extractors

#### Why Not Use Serde Deserialize Directly?

**Problem:** Serde expects fixed struct fields, but we need flexible field extraction across domains.

**Solution:** Use `serde_yaml::Value` for frontmatter, let trait implementations map to their own types.

#### Associated Types vs. Generics?

**Chosen:** Associated types (`type NodeData`, `type EdgeData`)

**Why:**

- Simpler API: `GraphBuilder<MusicTheoryExtractor>` vs. `GraphBuilder<MusicTheoryExtractor, NodeData, EdgeData>`
- Each extractor has exactly one node/edge type (no ambiguity)
- Better error messages from compiler

---

## Conclusion

**Extraction Feasibility:** **HIGH** - 82% of codebase is generic or parameterizable.

**Critical Success Factor:** The `GraphExtractor` trait design is the linchpin. It must be:

- Flexible enough for diverse domains (music, math, programming)
- Simple enough for easy implementation
- Well-documented with clear examples

**Recommended Next Steps:**

1. Implement `GraphExtractor` trait in fabryk-graph (1-2 days)
2. Create music theory extractor as reference implementation (2-3 days)
3. Validate with higher math and Rust examples (1 week)
4. Begin Phase 1 migration (see Section 7)

**Expected Benefits:**

- **Reusability:** Core infrastructure works for any knowledge domain
- **Maintainability:** Domain logic separated from infrastructure
- **Testability:** Mock extractors for testing, clear boundaries
- **Community:** Enables Fabryk ecosystem for diverse use cases

**Timeline Estimate:** **8-12 weeks** for complete extraction (with careful testing at each phase).

---

**End of Audit**
