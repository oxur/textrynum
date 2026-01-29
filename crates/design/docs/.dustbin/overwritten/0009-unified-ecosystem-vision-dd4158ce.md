---
number: 9
title: "Unified Ecosystem Vision"
author: "extracting this"
component: All
tags: [change-me]
created: 2026-01-28
updated: 2026-01-28
state: Overwritten
supersedes: null
superseded-by: null
version: 1.0
---

# Unified Ecosystem Vision

## ECL + Fabryk + Skill Framework

**Version**: 2.0
**Date**: January 2026
**Status**: Vision Document (Approved)
**Purpose**: Capture the converged architecture of three complementary projects

---

## Executive Summary

What began as three separate initiatives has revealed itself to be a unified ecosystem:

1. **ECL** ("Extract, Cogitate, Load") — Workflow orchestration with managed serialism
2. **Fabryk** — Persistent knowledge fabric with full-text search, graph database, and access control
3. **Skill Framework** — Pipeline for transforming domain sources into AI-consumable knowledge

The key insight: **the Music Theory Skill project inadvertently prototyped significant portions of Fabryk**. The rmcp + Tantivy + Graph MCP server built for music theory is, in essence, a specialized knowledge fabric. By extracting this into Fabryk proper, we gain:

- A reusable foundation for any domain skill
- A general-purpose knowledge store for ECL workflows
- A standardized MCP interface for AI agent access

This document captures the unified vision and the path to realizing it.

---

## The Convergence

### Three Streams, One River

| Project | Original Scope | Realized Role |
|---------|---------------|---------------|
| **Music Theory Skill** | Domain-specific AI knowledge store | **Prototype** for Fabryk + Skill Framework |
| **ECL** | AI workflow orchestration | **Orchestration layer** for skill building and knowledge production |
| **Fabryk** | General knowledge fabric | **Storage and access layer** for all knowledge, including skills |

### The Music Theory Prototype

The Music Theory Skill project built:

1. **Content loading** — Markdown parsing, frontmatter extraction, metadata handling
2. **Full-text search** — Tantivy-based indexing with BM25 ranking
3. **Graph database** — Concept relationships, prerequisites, path finding, centrality analysis
4. **MCP interface** — 25 tools exposing content, search, and graph capabilities

This infrastructure is **domain-agnostic**. The music theory content is configuration; the engines are reusable.

### Current Tool Inventory (25 tools)

| Category | Count | Examples |
|----------|-------|----------|
| Content Access | 10 | `list_sources`, `get_concept`, `get_guide`, `search_concepts` |
| Graph Database | 15 | `get_prerequisites`, `find_concept_path`, `get_concept_neighborhood` |

---

## Unified Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AI CONSUMERS                                   │
│         Claude Desktop │ Cursor │ Claude Code │ Custom Apps │ ECL           │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       │ MCP Protocol
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              FABRYK MCP LAYER                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  fabryk-mcp  │  │fabryk-mcp-   │  │fabryk-mcp-   │  │fabryk-mcp-   │     │
│  │   (core)     │  │   content    │  │     fts      │  │    graph     │     │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤  ├──────────────┤     │
│  │ • Server     │  │ • list_*     │  │ • search_*   │  │ • get_prereq │     │
│  │ • Transport  │  │ • get_*      │  │              │  │ • find_path  │     │
│  │ • Health     │  │              │  │              │  │ • neighbors  │     │
│  │ • Auth hooks │  │              │  │              │  │ • centrality │     │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       │ Fabryk Internal APIs
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              FABRYK CORE LAYER                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ fabryk-core  │  │   fabryk-    │  │  fabryk-fts  │  │ fabryk-graph │     │
│  │              │  │   content    │  │              │  │              │     │
│  ├──────────────┤  ├──────────────┤  ├──────────────┤  ├──────────────┤     │
│  │ • Types      │  │ • Loader     │  │ • Tantivy    │  │ • Types      │     │
│  │ • Traits     │  │ • Markdown   │  │ • Schema     │  │ • Builder    │     │
│  │ • Errors     │  │ • Metadata   │  │ • Indexer    │  │ • Algorithms │     │
│  │              │  │              │  │ • Query      │  │ • Query      │     │
│  │              │  │              │  │              │  │ • Validation │     │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘     │
│                                                                             │
│  ┌──────────────┐  ┌──────────────┐                                         │
│  │  fabryk-acl  │  │  fabryk-cli  │                                         │
│  │  (v0.2/0.3)  │  │              │                                         │
│  ├──────────────┤  ├──────────────┤                                         │
│  │ • Identities │  │ • Index cmds │                                         │
│  │ • Groups     │  │ • Graph cmds │                                         │
│  │ • Policies   │  │ • Validation │                                         │
│  │ (placeholder)│  │              │                                         │
│  └──────────────┘  └──────────────┘                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
         ▲                              ▲                              ▲
         │                              │                              │
         │ store_knowledge()            │ query_knowledge()            │ direct API
         │                              │                              │
┌────────┴────────┐            ┌────────┴────────┐            ┌────────┴────────┐
│       ECL       │            │  Skill Builder  │            │   Domain Skills │
│   Workflows     │            │   (ECL-based)   │            │                 │
│                 │            │                 │            │ • Music Theory  │
│ Any workflow    │            │ Builds domain   │            │ • Higher Math   │
│ can persist     │            │ skills from     │            │ • Advanced Rust │
│ knowledge       │            │ raw sources     │            │ • (Your work?)  │
└─────────────────┘            └─────────────────┘            └─────────────────┘
```

---

## ECL "Ecosystem" Crate Structure

### ECL Layer

```
ecl/crates/
├── ecl
├── ecl-cli
├── ecl-core
├── ecl-steps
└── ecl-workflows
```

### Fabryk Proper Layer

```
ecl/crates/
├── fabryk/                # Umbrella crate
│   └── ...
│
├── fabryk-core/           # Shared types and traits
│   ├── src/
│   │   ├── types.rs       # Item, Partition, Tag, Metadata, ContentItem
│   │   ├── traits.rs      # ContentLoader, GraphExtractor, SearchBackend
│   │   ├── error.rs       # Common error types (thiserror)
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-content/        # Content loading and parsing
│   ├── src/
│   │   ├── loader.rs      # Filesystem traversal, item loading
│   │   ├── markdown.rs    # Markdown + frontmatter parsing
│   │   ├── metadata.rs    # Structured metadata extraction
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-fts/            # Full-text search (Tantivy-based)
│   ├── src/
│   │   ├── backend.rs     # SearchBackend trait implementation
│   │   ├── schema.rs      # Tantivy schema definition
│   │   ├── indexer.rs     # Index creation and updates
│   │   ├── query.rs       # Query parsing and execution
│   │   ├── document.rs    # Document types for indexing
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-graph/          # Knowledge graph
│   ├── src/
│   │   ├── types.rs       # Node, Edge, Graph, RelationshipType
│   │   ├── builder.rs     # Graph construction (uses GraphExtractor)
│   │   ├── algorithms.rs  # Path finding, centrality, neighborhoods
│   │   ├── query.rs       # Graph traversal queries
│   │   ├── persistence.rs # Serialization/deserialization
│   │   ├── validation.rs  # Integrity checks
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-acl/            # Access control (placeholder for v0.2/v0.3)
│   ├── src/
│   │   └── lib.rs         # Placeholder with TODO comments
│   └── Cargo.toml
│
└── fabryk-cli/            # Admin CLI
    ├── src/
    │   ├── main.rs
    │   ├── index.rs       # Reindex commands
    │   ├── graph.rs       # Graph inspection commands
    │   └── validate.rs    # Validation commands
    └── Cargo.toml
```

### Fabryk MCP Layer

```
ecl/crates/
├── fabryk-mcp/            # Core MCP infrastructure
│   ├── src/
│   │   ├── server.rs      # rmcp server setup, transport config
│   │   ├── tools.rs       # Tool registration helpers/macros
│   │   ├── auth.rs        # ACL integration hooks (placeholder)
│   │   ├── health.rs      # Health check tool
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-mcp-content/    # Content access MCP tools
│   ├── src/
│   │   ├── sources.rs     # list_sources, get_source_chapter, get_source_pdf_path
│   │   ├── concepts.rs    # list_concepts, list_categories, get_concept
│   │   ├── guides.rs      # list_guides, get_guide
│   │   └── lib.rs
│   └── Cargo.toml
│
├── fabryk-mcp-fts/        # Full-text search MCP tools
│   ├── src/
│   │   ├── search.rs      # search_concepts (and future search tools)
│   │   └── lib.rs
│   └── Cargo.toml
│
└── fabryk-mcp-graph/      # Graph database MCP tools
    ├── src/
    │   ├── inspection.rs  # graph_status, graph_stats, graph_validate
    │   ├── nodes.rs       # get_node, get_node_edges
    │   ├── traversal.rs   # get_related, find_path, get_prerequisites
    │   ├── analysis.rs    # get_central_concepts, find_bridge_concepts
    │   ├── sources.rs     # get_concept_sources, get_source_coverage
    │   └── lib.rs
    └── Cargo.toml
```

---

## The GraphExtractor Trait

The key abstraction enabling domain-agnostic graph building:

```rust
// In fabryk-graph/src/traits.rs

/// Trait for extracting graph structure from content items.
///
/// Each domain skill implements this to define how its content
/// maps to nodes and edges in the knowledge graph.
pub trait GraphExtractor: Send + Sync {
    /// Extract a node from a content item.
    /// Returns None if the item shouldn't become a graph node.
    fn extract_node(&self, item: &ContentItem) -> Option<Node>;

    /// Extract edges from a content item.
    /// A single item may produce multiple edges (e.g., multiple prerequisites).
    fn extract_edges(&self, item: &ContentItem) -> Vec<Edge>;

    /// Relationship types this extractor recognizes.
    /// Used for validation and documentation.
    fn relationship_types(&self) -> &[RelationshipType];

    /// Optional: Custom node ID generation.
    /// Default uses item path/filename.
    fn node_id(&self, item: &ContentItem) -> String {
        // Default implementation
    }
}
```

**Domain implementations:**

```rust
// In music-theory-mcp-server/src/extractor.rs

pub struct MusicTheoryExtractor;

impl GraphExtractor for MusicTheoryExtractor {
    fn extract_node(&self, item: &ContentItem) -> Option<Node> {
        // Parse music theory frontmatter
        // Extract: id, title, category, summary
        // Return Node with music-theory-specific metadata
    }

    fn extract_edges(&self, item: &ContentItem) -> Vec<Edge> {
        // Extract from frontmatter:
        // - prerequisites → "prerequisite_for" edges
        // - related_concepts → "related_to" edges
        // - see_also → "see_also" edges
    }

    fn relationship_types(&self) -> &[RelationshipType] {
        &[
            RelationshipType::new("prerequisite_for", Directed, "Must understand A before B"),
            RelationshipType::new("related_to", Undirected, "Conceptually related"),
            RelationshipType::new("see_also", Directed, "Additional reading"),
            RelationshipType::new("variant_of", Directed, "Source-specific variant"),
        ]
    }
}
```

---

## Domain Skill Structure

After Fabryk extraction, domain skills become thin configuration + domain-specific code:

```
ai-music-theory/
├── <content>
└── crates/server
    ├── src/
    │   ├── main.rs            # Server setup, wires Fabryk components
    │   ├── extractor.rs       # impl GraphExtractor for MusicTheoryExtractor
    │   └── prompts.rs         # Domain-specific MCP prompts (optional)
    ├── config/
    │   └── default.toml       # Paths, feature flags, etc.
    ├── skill.toml             # Skill metadata (name, version, sources)
    ├── validation/
    │   ├── queries.toml       # Test queries
    │   └── expected/          # Expected results
    ├── Cargo.toml
    └── README.md

# The main.rs wiring:

use fabryk_mcp::McpServerBuilder;
use fabryk_mcp_content::ContentTools;
use fabryk_mcp_fts::FtsTools;
use fabryk_mcp_graph::GraphTools;
use fabryk_content::ContentLoader;
use fabryk_fts::TantivyBackend;
use fabryk_graph::GraphBuilder;

use crate::extractor::MusicTheoryExtractor;

fn main() -> Result<()> {
    let config = load_config()?;

    // Load content
    let loader = ContentLoader::new(&config.content_paths)?;
    let items = loader.load_all()?;

    // Build FTS index
    let fts = TantivyBackend::new(&config.index_path)?;
    fts.index_items(&items)?;

    // Build graph with domain-specific extractor
    let extractor = MusicTheoryExtractor;
    let graph = GraphBuilder::new()
        .with_extractor(&extractor)
        .build_from_items(&items)?;

    // Create MCP server with all tools
    let server = McpServerBuilder::new()
        .with_content_tools(&items)
        .with_fts_tools(&fts)
        .with_graph_tools(&graph)
        // .with_custom_prompts(music_theory_prompts())  // Optional
        .build()?;

    server.run()
}
```

---

## ECL Integration

ECL becomes the orchestration layer for:

1. **Skill building** — Automated concept extraction, synthesis, guide generation
2. **Knowledge production** — Any workflow that produces knowledge items

### Skill Builder Workflow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        SKILL BUILDER WORKFLOW                               │
│                         (Orchestrated by ECL)                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                    │
│  │   Convert   │────▶│   Extract   │────▶│   Validate  │                    │
│  │   Sources   │     │   Concepts  │     │   Cards     │                    │
│  │             │     │   (LLM)     │     │   (LLM)     │                    │
│  └─────────────┘     └──────┬──────┘     └──────┬──────┘                    │
│        │                    │                   │                           │
│        │              ┌─────▼─────┐       ┌─────▼─────┐                     │
│        │              │  Revision │       │  Revision │                     │
│        │              │   Loop    │       │   Loop    │                     │
│        │              └───────────┘       └───────────┘                     │
│        │                    │                   │                           │
│        ▼                    ▼                   ▼                           │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐                    │
│  │   Build     │────▶│   Build     │────▶│   Run       │                    │
│  │   Graph     │     │   FTS Index │     │ Validation  │                    │
│  │             │     │             │     │   Tests     │                    │
│  └─────────────┘     └─────────────┘     └─────────────┘                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Future phases** (after concept cards are complete for all sources):

- Unified concept synthesis
- Debate detection
- Guide generation

---

## Implementation Priorities

### Priority 1: Extract Fabryk from Music Theory MCP Server (v0.1.0)

**Goal**: Factor out domain-agnostic code into Fabryk crates.

**Deliverables**:

- `fabryk-core` — Types and traits
- `fabryk-content` — Content loading (from `src/markdown/`, `src/metadata/`)
- `fabryk-fts` — Full-text search (from `src/search/`)
- `fabryk-graph` — Knowledge graph (from `src/graph/`)
- `fabryk-mcp` — Core MCP infrastructure
- `fabryk-mcp-content` — Content tools
- `fabryk-mcp-fts` — Search tools
- `fabryk-mcp-graph` — Graph tools
- `fabryk-acl` — Placeholder
- `fabryk-cli` — Basic CLI

**Validation**: Music theory skill works identically after extraction.

### Priority 2: Generalize for Any Skill (v0.1.x)

**Goal**: Make Fabryk usable as a dependency for other domain skills.

**Deliverables**:

- `GraphExtractor` trait finalized
- Configuration system documented
- Second skill (Higher Math or Advanced Rust) using Fabryk

### Priority 3: ECL Integration (v0.2.0)

**Goal**: Orchestrate skill building with ECL.

**Deliverables**:

- Skill builder workflow
- LLM-assisted concept extraction
- Integration tests

### Priority 4: ACL Implementation (v0.2.0 or v0.3.0)

**Goal**: Fine-grained access control.

**Deliverables**:

- `fabryk-acl` implementation
- Integration with `fabryk-mcp` auth hooks

---

## Planned Domain Skills

| Skill | Status | Notes |
|-------|--------|-------|
| **Music Theory** | Active prototype | Neo-Riemannian, Open Music Theory sources |
| **Higher Math** | Planned | Group, Category, Type, Homotopy theories |
| **Advanced Rust** | Planned (rebuild) | Previous work exists, will use new approach |
| **Work projects** | Potential | CEO interest in work-hours usage |

---

## Success Criteria

### For Fabryk v0.1.0

1. **Functionality preserved**: Music theory skill works identically
2. **Clean separation**: No music-theory-specific code in Fabryk crates
3. **GraphExtractor trait**: Enables domain-specific graph building
4. **Documented API**: Other skills can depend on Fabryk
5. **Tests passing**: Unit and integration tests for all crates

### For Second Skill

1. **New extractor**: Domain-specific `GraphExtractor` implementation
2. **No Fabryk changes**: New skill requires no Fabryk modifications
3. **Working tools**: All 25 MCP tools functional for new domain

---

## Open Questions (For Future Resolution)

### Technical

1. **Embedding generation**: Should Fabryk generate embeddings for semantic search?
2. **Vector search**: Add pgvector/Qdrant, or keep Tantivy keyword-only?
3. **Schema evolution**: How do item schemas change without breaking skills?

### Skill Framework

1. **LLM provider**: Lock to Claude, or support multiple providers?
2. **Incremental builds**: Rebuild only changed sources, or full rebuild?
3. **Unified concepts**: Process for cross-source synthesis (post-concept-card completion)

### Integration

1. **Transaction semantics**: Should Fabryk store + ECL step be atomic?
2. **Failure handling**: What if Fabryk operations fail mid-workflow?

---

## Document Roadmap

| Doc # | Name | Status | Description |
|-------|------|--------|-------------|
| 0003 | ECL Project Proposal | ✅ Complete | Original ECL vision |
| 0004 | ECL Project Plan | ✅ Complete | Phased implementation |
| 0005 | ECL Ecosystem Vision Summary | ✅ Complete | Expanded vision |
| 0008 | Fabryk-MCP Project Proposal | ✅ Complete | MCP integration vision |
| **NNNN** | **Unified Ecosystem Vision v2** | **✅ This doc** | **Converged architecture** |
| NNNN | Music Theory Extraction Audit | 📝 Next | Detailed extraction plan |
| NNNN | Fabryk Project Plan | ⏳ Future | Phased implementation |
| NNNN | Skill Framework Proposal | ⏳ Future | Pipeline codification |

---

## Summary

The Music Theory Skill revealed that we were building Fabryk all along — with both full-text search AND graph database capabilities. The refined architecture separates:

**Core engines**:

- `fabryk-content` — Load and parse content
- `fabryk-fts` — Full-text search via Tantivy
- `fabryk-graph` — Knowledge graph with algorithms

**MCP exposure**:

- `fabryk-mcp` — Core infrastructure + health
- `fabryk-mcp-content` — Content access tools
- `fabryk-mcp-fts` — Search tools
- `fabryk-mcp-graph` — Graph traversal tools

**Domain skills** provide:

- `GraphExtractor` implementation
- Configuration
- Optional custom prompts

This creates a clean separation where Fabryk handles the "how" (storage, search, graph, MCP) and skills handle the "what" (domain-specific schemas and relationships).

**Next step**: Audit the music-theory MCP server code to plan the extraction.

---

## Appendix: Key Decisions Log

| Decision | Choice | Date | Rationale |
|----------|--------|------|-----------|
| Rename fabryk-query → fabryk-fts | Yes | 2026-01-28 | Specificity; parallels fabryk-graph |
| Rename fabryk-storage → fabryk-content | Yes | 2026-01-28 | Clearer purpose (load/parse, not persist) |
| Add fabryk-graph as first-class crate | Yes | 2026-01-28 | Graph DB is distinct from FTS |
| Split MCP into 4 crates | Yes | 2026-01-28 | Core + content + fts + graph separation |
| GraphExtractor trait for domain customization | Yes | 2026-01-28 | Enables reuse across skills |
| ACL as placeholder in v0.1 | Yes | 2026-01-28 | Defer to v0.2/v0.3 |
| Health tool in fabryk-mcp (core) | Yes | 2026-01-28 | Fundamental capability |
| Content tools in fabryk-mcp-content | Yes | 2026-01-28 | Separate from core infrastructure |