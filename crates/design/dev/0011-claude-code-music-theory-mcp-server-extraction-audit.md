# Claude Code: Music Theory MCP Server Extraction Audit

## Purpose

This prompt guides Claude Code through a comprehensive audit of the `ai-music-theory/mcp-server` codebase. The goal is to plan extraction of domain-agnostic code into **Fabryk** — a general-purpose knowledge fabric with full-text search and graph database capabilities.

## Context

The music-theory MCP server has grown into a sophisticated system with:

- **Content loading**: Markdown parsing, frontmatter extraction, metadata handling
- **Full-text search**: Tantivy-based indexing with BM25 ranking (10 tools)
- **Graph database**: Concept relationships, algorithms, traversal (15 tools)
- **MCP interface**: 25 tools total via rmcp

In the course of the project's development, we've realized this is actually a **prototype of the ECL Fabryk project**. Most code is domain-agnostic and should be extracted into reusable crates, leaving only domain-specific configuration in the music theory project.

## Target Fabryk Crate Structure

After extraction:

```
fabryk/
├── fabryk-core/           # Shared types and traits
│   ├── types.rs           # Item, Partition, Tag, Metadata, ContentItem
│   ├── traits.rs          # ContentLoader, GraphExtractor, SearchBackend
│   └── error.rs           # Common error types
│
├── fabryk-content/        # Content loading and parsing
│   ├── loader.rs          # Filesystem traversal, item loading
│   ├── markdown.rs        # Markdown + frontmatter parsing
│   └── metadata.rs        # Structured metadata extraction
│
├── fabryk-fts/            # Full-text search (Tantivy)
│   ├── backend.rs         # SearchBackend implementation
│   ├── schema.rs          # Tantivy schema definition
│   ├── indexer.rs         # Index creation and updates
│   ├── query.rs           # Query parsing and execution
│   └── document.rs        # Document types
│
├── fabryk-graph/          # Knowledge graph
│   ├── types.rs           # Node, Edge, Graph, RelationshipType
│   ├── builder.rs         # Graph construction (uses GraphExtractor trait)
│   ├── algorithms.rs      # Path finding, centrality, neighborhoods
│   ├── query.rs           # Graph traversal queries
│   ├── persistence.rs     # Serialization
│   └── validation.rs      # Integrity checks
│
├── fabryk-acl/            # Access control (placeholder for v0.2/v0.3)
│   └── lib.rs             # Placeholder with TODO
│
├── fabryk-mcp/            # Core MCP infrastructure
│   ├── server.rs          # rmcp setup, transport config
│   ├── tools.rs           # Tool registration helpers
│   ├── auth.rs            # ACL hooks (placeholder)
│   └── health.rs          # Health check tool
│
├── fabryk-mcp-content/    # Content MCP tools
│   ├── sources.rs         # list_sources, get_source_chapter, etc.
│   ├── concepts.rs        # list_concepts, get_concept, etc.
│   └── guides.rs          # list_guides, get_guide
│
├── fabryk-mcp-fts/        # FTS MCP tools
│   └── search.rs          # search_concepts
│
├── fabryk-mcp-graph/      # Graph MCP tools
│   ├── inspection.rs      # graph_status, graph_stats, graph_validate
│   ├── nodes.rs           # get_node, get_node_edges
│   ├── traversal.rs       # get_related, find_path, get_prerequisites
│   ├── analysis.rs        # get_central_concepts, find_bridge_concepts
│   └── sources.rs         # get_concept_sources, get_source_coverage
│
└── fabryk-cli/            # Admin CLI
    ├── index.rs           # Reindex commands
    ├── graph.rs           # Graph commands
    └── validate.rs        # Validation commands
```

**Post-extraction music-theory structure:**

This represents one possibility:

```
ai-music-theory/mcp-server/crates/server
├── src/
│   ├── main.rs            # Wires Fabryk components with music-theory config
│   ├── extractor.rs       # impl GraphExtractor for MusicTheoryExtractor
│   └── prompts.rs         # Domain-specific MCP prompts (if any)
├── config/
│   └── default.toml       # Paths, feature flags
├── skill.toml             # Skill metadata
├── validation/            # Test queries
└── Cargo.toml             # Depends on fabryk-* crates
```

Final form should be based upon files extracted, files that remain, and the best engineering decicions given the full context of having performed the audit and planned the migration.

---

## The Prompt

Copy everything below this line into Claude Code:

```
I need you to perform a comprehensive audit of the ai-music-theory MCP server codebase (`~/lab/music-comp/ai-music-theory/mcp-server`) to plan its refactoring into the Fabryk ecosystem.

## Your Mission

Analyze every file in the codebase and produce a detailed extraction plan that:
1. Identifies what code moves to which Fabryk crate
2. Identifies what code stays in ai-music-theory (domain-specific)
3. Proposes the GraphExtractor trait design
4. Provides a safe migration path

## Current Codebase Structure

Based on the git-tracked files, the key source directories are:

```

mcp-server/crates/server/src/
├── cli.rs
├── config.rs
├── error.rs
├── lib.rs
├── main.rs
├── server.rs
├── state.rs
├── graph/
│   ├── algorithms.rs
│   ├── builder.rs
│   ├── cli.rs
│   ├── loader.rs
│   ├── mod.rs
│   ├── parser.rs
│   ├── persistence.rs
│   ├── query.rs
│   ├── stats.rs
│   ├── types.rs
│   └── validation.rs
├── markdown/
│   ├── frontmatter.rs
│   ├── mod.rs
│   └── parser.rs
├── metadata/
│   ├── extraction.rs
│   └── mod.rs
├── resources/
│   └── mod.rs
├── search/
│   ├── backend.rs
│   ├── builder.rs
│   ├── document.rs
│   ├── freshness.rs
│   ├── indexer.rs
│   ├── mod.rs
│   ├── query.rs
│   ├── schema.rs
│   ├── simple_search.rs
│   ├── stopwords.rs
│   └── tantivy_search.rs
├── tools/
│   ├── concepts.rs
│   ├── graph.rs
│   ├── graph_query.rs
│   ├── guides.rs
│   ├── health.rs
│   ├── mod.rs
│   ├── search.rs
│   └── sources.rs
└── util/
    ├── files.rs
    ├── mod.rs
    └── paths.rs

```

## Task 1: File-by-File Inventory

Read each source file and create an inventory:

| File | Purpose | Lines | Key Types/Functions | External Deps |
|------|---------|-------|---------------------|---------------|
| ... | ... | ... | ... | ... |

## Task 2: Classification Matrix

For each file, classify as:

| Classification | Meaning | Destination |
|----------------|---------|-------------|
| **Generic** | No music-theory knowledge | fabryk-* |
| **Parameterized** | Generic but needs config | fabryk-* with trait/config |
| **Domain-Specific** | Music theory hardcoded | Stays in music-theory |
| **Mixed** | Contains both | Split required |

Create a detailed table:

| File | Classification | Target Crate | Changes Needed |
|------|----------------|--------------|----------------|
| ... | ... | ... | ... |

## Task 3: GraphExtractor Trait Design

The `graph/parser.rs` file extracts nodes and edges from concept cards. This is domain-aware.

Analyze this file and propose:

1. **What's generic**: Graph structure, node/edge types, builder pattern
2. **What's domain-specific**: Frontmatter field names, relationship types
3. **Trait design**:

```rust
pub trait GraphExtractor: Send + Sync {
    // What methods should this trait have?
    // What associated types?
    // What default implementations?
}
```

1. **Music theory implementation**: How would MusicTheoryExtractor implement this?

## Task 4: Extraction Plan by Crate

For each target Fabryk crate, specify:

### fabryk-core

- Types to extract (with any renames/generalizations)
- Traits to define
- Error types

### fabryk-content

- Files that move here
- Changes needed for generalization
- Configuration surface area

### fabryk-fts

- Files that move here
- Schema parameterization needed
- Query abstraction

### fabryk-graph

- Files that move here
- GraphExtractor integration points
- Algorithm generalization

### fabryk-mcp

- Server setup code
- Tool registration pattern
- Health tool

### fabryk-mcp-content

- Which tools move here
- State/context requirements

### fabryk-mcp-fts

- Which tools move here
- Backend abstraction

### fabryk-mcp-graph

- Which tools move here
- Graph dependency injection

### fabryk-acl (placeholder)

- What placeholder code to include
- Integration hooks to prepare

### fabryk-cli

- Which CLI commands move here
- How to make domain-agnostic

## Task 5: Music Theory Remainder

After extraction, what stays:

1. **main.rs**: How does it wire up Fabryk?
2. **extractor.rs**: Full MusicTheoryExtractor implementation
3. **config/**: What configuration is domain-specific?
4. **prompts.rs**: Any domain-specific MCP prompts?

Provide skeleton code for the post-extraction music-theory crate.

## Task 6: Dependency Analysis

Create two Cargo.toml sketches:

1. **Fabryk workspace Cargo.toml** with all crate dependencies
2. **Music-theory Cargo.toml** showing Fabryk dependencies

## Task 7: Migration Steps

Provide a safe, incremental migration plan:

1. **Phase 1**: What to extract first (lowest risk)
2. **Phase 2**: Next extraction batch
3. **Phase 3**: Final extraction
4. **Verification**: How to confirm nothing broke at each phase

Include specific git commit boundaries.

## Task 8: Risk Assessment

Identify:

- **Technical risks**: What could go wrong?
- **API design risks**: What might need to change later?
- **Testing gaps**: What tests are missing?
- **Documentation needs**: What must be documented?

## Task 9: The GraphExtractor Deep Dive

This is critical for reusability. Analyze `graph/parser.rs` in detail:

1. **Current behavior**: What does it extract? How?
2. **Hardcoded assumptions**: What's music-theory-specific?
3. **Generalization strategy**: How to make it work for any domain?
4. **Example implementations**: Sketch extractors for:
   - Music theory (current)
   - Higher math (hypothetical)
   - Rust programming (hypothetical)

## Output Format

Structure your audit as a markdown document with these sections:

1. **Executive Summary** (1 paragraph)
2. **File Inventory** (Task 1)
3. **Classification Matrix** (Task 2)
4. **GraphExtractor Design** (Task 3)
5. **Crate-by-Crate Extraction Plan** (Task 4)
6. **Music Theory Remainder** (Task 5)
7. **Dependency Configuration** (Task 6)
8. **Migration Plan** (Task 7)
9. **Risk Assessment** (Task 8)
10. **GraphExtractor Deep Dive** (Task 9)
11. **Appendix: File-by-File Recommendations**

Save as: `~/lab/music-comp/ai-music-theory/workbench/fabryk-extraction-audit.md`

## Critical Success Factors

- **Be thorough**: Read every file completely
- **Be specific**: Reference file paths, line numbers, function names
- **Be practical**: Suggest the simplest extraction that works
- **Preserve functionality**: Music theory must work identically after
- **Think ahead**: Consider Higher Math and Advanced Rust skills

## Notes on Specific Files

Pay special attention to:

1. **graph/parser.rs**: Core of the GraphExtractor design
2. **search/schema.rs**: Field names may be hardcoded
3. **tools/*.rs**: MCP tool implementations, check for domain assumptions
4. **state.rs**: Shared state management, may need abstraction
5. **config.rs**: Configuration structure, what's domain-specific?

Take your time. This audit is foundational for the entire Fabryk project.

```

---

## After the Audit

The audit will produce:

1. **Clear inventory** of all code with classification
2. **GraphExtractor trait design** enabling domain customization
3. **Crate-by-crate extraction plan** with specific files
4. **Migration steps** for safe, incremental extraction
5. **Risk assessment** for decision-making

Use this audit to:
1. Create detailed implementation tickets
2. Begin extraction in priority order
3. Validate with music-theory tests at each step

---

## Tips for Running This Prompt

1. **Fresh session**: Start a new Claude Code session
2. **Full codebase access**: Ensure CC can read all source files
3. **Patience**: This is a thorough audit, let it complete
4. **Follow-up ready**: Prepare clarifying questions
5. **Iterate**: First pass may miss things; second pass fills gaps

---

## Expected Outputs

After the audit, you should have:

| Artifact | Purpose |
|----------|---------|
| `~/lab/music-comp/ai-music-theory/workbench/fabryk-extraction-audit.md` | Complete audit document |
| Classification of all ~30 source files | Know what goes where |
| GraphExtractor trait design | Key abstraction for reuse |
| Migration plan with phases | Safe extraction path |
| Risk assessment | Informed decision-making |

This audit will drive the entire Fabryk v0.1.0 implementation.
