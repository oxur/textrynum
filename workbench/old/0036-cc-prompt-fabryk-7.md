---
title: "CC Prompt: Fabryk 7.3 — Documentation"
milestone: "7.3"
phase: 7
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["7.1-7.2 complete"]
governing-docs: [0013-project-plan]
---

# CC Prompt: Fabryk 7.3 — Documentation

## Context

This milestone creates documentation for the Fabryk ecosystem. Good documentation
is essential for:
- New domain developers implementing their own skills
- Contributors understanding the architecture
- Maintenance and debugging

## Objective

Create comprehensive documentation:

1. README for each Fabryk crate
2. Top-level Fabryk README with architecture overview
3. `GraphExtractor` implementation guide
4. `ContentItemProvider` and `SourceProvider` guides
5. API documentation via rustdoc

## Documentation Steps

### Step 1: Create top-level Fabryk README

Create `crates/fabryk/README.md`:

```markdown
# Fabryk

**Fabryk** is a framework for building knowledge-domain MCP (Model Context Protocol)
servers. It provides reusable infrastructure for:

- **Content management** - Markdown parsing, frontmatter extraction
- **Full-text search** - Tantivy-based search with configurable schema
- **Knowledge graphs** - Directed graphs with relationship semantics
- **MCP tools** - Generic tools that delegate to domain implementations
- **CLI framework** - Extensible command-line interface

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Domain Application                        │
│  (e.g., music-theory, math-proofs, rust-patterns)          │
├─────────────────────────────────────────────────────────────┤
│  fabryk-cli      │  fabryk-mcp-*   │  Domain-specific       │
│  CLI framework   │  MCP tools      │  implementations       │
├──────────────────┴─────────────────┴────────────────────────┤
│  fabryk-graph    │  fabryk-fts     │  fabryk-content        │
│  Knowledge graph │  Full-text      │  Content parsing       │
│  + algorithms    │  search         │                        │
├──────────────────┴─────────────────┴────────────────────────┤
│                      fabryk-core                             │
│  Error types, utilities, ConfigProvider trait                │
└─────────────────────────────────────────────────────────────┘
```

## Crates

| Crate | Description |
|-------|-------------|
| `fabryk-core` | Core types, errors, utilities, ConfigProvider trait |
| `fabryk-content` | Markdown parsing, frontmatter extraction |
| `fabryk-fts` | Full-text search with Tantivy backend |
| `fabryk-graph` | Knowledge graph with algorithms |
| `fabryk-mcp` | MCP server infrastructure |
| `fabryk-mcp-content` | Content listing MCP tools |
| `fabryk-mcp-fts` | Search MCP tools |
| `fabryk-mcp-graph` | Graph query MCP tools |
| `fabryk-cli` | CLI framework |

## Quick Start

### Implementing a New Domain

1. **Implement `ConfigProvider`** for your configuration:
   ```rust
   impl ConfigProvider for MyConfig {
       fn project_name(&self) -> &str { "my-domain" }
       fn base_path(&self) -> Result<PathBuf> { ... }
       fn content_path(&self, content_type: &str) -> Result<PathBuf> { ... }
   }
   ```

2. **Implement `GraphExtractor`** for your content:
   ```rust
   impl GraphExtractor for MyExtractor {
       type NodeData = MyNodeData;
       type EdgeData = MyEdgeData;

       fn extract_node(&self, ...) -> Result<Self::NodeData> { ... }
       fn extract_edges(&self, ...) -> Result<Option<Self::EdgeData>> { ... }
       fn to_graph_node(&self, data: &Self::NodeData) -> Node { ... }
       fn to_graph_edges(&self, from_id: &str, data: &Self::EdgeData) -> Vec<Edge> { ... }
   }
   ```

3. **Implement `ContentItemProvider`** for MCP tools:
   ```rust
   impl ContentItemProvider for MyContentProvider {
       type ItemSummary = MyItemSummary;
       type ItemDetail = MyItemDetail;

       async fn list_items(&self, ...) -> Result<Vec<Self::ItemSummary>> { ... }
       async fn get_item(&self, id: &str) -> Result<Self::ItemDetail> { ... }
       async fn list_categories(&self) -> Result<Vec<CategoryInfo>> { ... }
   }
   ```

4. **Wire up the CLI and MCP server**

See the [Implementation Guide](docs/implementation-guide.md) for detailed instructions.

## Key Abstractions

### GraphExtractor

The `GraphExtractor` trait is the core abstraction for building knowledge graphs.
Each domain implements this to define how its content is transformed into nodes
and edges.

[GraphExtractor Guide](docs/graph-extractor-guide.md)

### ContentItemProvider

The `ContentItemProvider` trait enables generic MCP tools for content operations.
Implement this to expose your content via MCP.

[ContentItemProvider Guide](docs/content-provider-guide.md)

## License

Apache-2.0
```

### Step 2: Create implementation guide

Create `crates/fabryk/docs/implementation-guide.md`:

```markdown
# Fabryk Implementation Guide

This guide walks through implementing a new knowledge domain using Fabryk.

## Prerequisites

- Rust 1.70+
- Understanding of MCP (Model Context Protocol)
- Content in Markdown with YAML frontmatter

## Step 1: Define Your Configuration

[detailed config implementation]

## Step 2: Implement GraphExtractor

[detailed extractor implementation with examples]

## Step 3: Implement Content Providers

[detailed provider implementation]

## Step 4: Set Up CLI

[CLI setup instructions]

## Step 5: Create MCP Server

[MCP server setup]

## Testing Your Implementation

[testing instructions]
```

### Step 3: Create GraphExtractor guide

Create `crates/fabryk/docs/graph-extractor-guide.md`:

```markdown
# GraphExtractor Implementation Guide

The `GraphExtractor` trait is how domains define their knowledge graph structure.

## Trait Definition

```rust
pub trait GraphExtractor: Send + Sync {
    type NodeData: Clone + Send + Sync;
    type EdgeData: Clone + Send + Sync;

    fn extract_node(&self, base_path: &Path, file_path: &Path,
                    frontmatter: &serde_yaml::Value, content: &str)
        -> Result<Self::NodeData>;

    fn extract_edges(&self, frontmatter: &serde_yaml::Value, content: &str)
        -> Result<Option<Self::EdgeData>>;

    fn to_graph_node(&self, node_data: &Self::NodeData) -> Node;

    fn to_graph_edges(&self, from_id: &str, edge_data: &Self::EdgeData) -> Vec<Edge>;
}
```

## Example: Math Theorems

[complete example for a math domain]

## Example: Rust Patterns

[complete example for a rust patterns domain]

## Best Practices

1. Use `id_from_path()` for stable IDs
2. Store extra data in `Node::metadata`
3. Use appropriate `Relationship` variants
4. Handle missing frontmatter gracefully
```

### Step 4: Generate rustdoc

```bash
cd ~/lab/oxur/ecl

# Generate docs for all crates
cargo doc --workspace --no-deps

# Open in browser
cargo doc --workspace --no-deps --open
```

### Step 5: Add doc comments to public APIs

Ensure all public items have documentation:

```rust
/// Brief description of the item.
///
/// Longer description with details about usage.
///
/// # Examples
///
/// ```rust
/// // Example code
/// ```
///
/// # Errors
///
/// Returns an error if...
```

### Step 6: Create CHANGELOG

Create `crates/fabryk/CHANGELOG.md`:

```markdown
# Changelog

## [0.1.0] - 2026-02-XX

### Added
- Initial release of Fabryk framework
- fabryk-core: Error types, utilities, ConfigProvider
- fabryk-content: Markdown and frontmatter parsing
- fabryk-fts: Full-text search with Tantivy
- fabryk-graph: Knowledge graph with algorithms
- fabryk-mcp: MCP server infrastructure
- fabryk-mcp-content: Content listing tools
- fabryk-mcp-fts: Search tools
- fabryk-mcp-graph: Graph query tools
- fabryk-cli: CLI framework

### Migration from music-theory
- Extracted ~87% of code from music-theory MCP server
- music-theory now depends on Fabryk crates
```

## Exit Criteria

- [ ] Top-level README with architecture diagram
- [ ] Each crate has a README
- [ ] GraphExtractor implementation guide complete
- [ ] ContentItemProvider guide complete
- [ ] All public APIs have doc comments
- [ ] `cargo doc` builds without warnings
- [ ] CHANGELOG created

## Commit Message

```
docs: add Fabryk documentation

Add comprehensive documentation:
- Top-level README with architecture overview
- Implementation guide for new domains
- GraphExtractor guide with examples
- ContentItemProvider guide
- Doc comments on all public APIs
- CHANGELOG for v0.1.0

Phase 7 milestone 7.3 of Fabryk extraction.

Ref: Doc 0013 Phase 7

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
