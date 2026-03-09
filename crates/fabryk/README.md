# Fabryk

[![][crate-badge]][crate]
[![][docs-badge]][docs]

[![][logo]][logo-large]

 *A hyper-connected, multi-modal knowlege fabric* • Part of the [Textrynum](../../README.md) project

Fabryk turns structured content (Markdown with YAML frontmatter) into a multi-modal knowledge store: a **graph** of relationships, a **full-text search** index, and a **vector space** for semantic similarity — all exposed via **MCP tools** and a **CLI**.

## What it does

- **Content ingestion** — Parse Markdown files with YAML frontmatter, extract sections, resolve sources
- **Knowledge graph** — Build and traverse relationship graphs (prerequisites, extensions, related concepts) with petgraph
- **Full-text search** — Tantivy-backed search with category filtering, incremental indexing, and freshness tracking
- **Vector/semantic search** — LanceDB + fastembed for local embedding generation and similarity search
- **Hybrid search** — Reciprocal rank fusion combining FTS and vector results
- **MCP server** — 25+ tools for AI assistants to query the knowledge base via Model Context Protocol
- **CLI** — Command-line interface for graph operations, search, and configuration
- **Auth** — OAuth2 with Google provider, JWT validation, RFC 9728 discovery

## Quick Start

Fabryk provides two umbrella crates so downstream projects only need one or two dependencies:

```toml
[dependencies]
# Core knowledge fabric (content, graph, search, auth, acl)
fabryk = { version = "0.2", features = ["full"] }

# MCP server tools (adds all MCP tool suites + server infrastructure)
fabryk-mcp = { version = "0.2", features = ["http"] }
```

For finer control, enable only the backends you need:

```toml
[dependencies]
# Just graph and FTS, no vector search
fabryk = { version = "0.2", features = ["fts-tantivy", "graph-rkyv-cache"] }

# MCP tools without HTTP transport (stdio only)
fabryk-mcp = "0.2"
```

Vendor-specific crates are added separately:

```toml
[dependencies]
fabryk-auth-google = "0.2"  # Google OAuth2 / JWKS
fabryk-gcp = "0.2"          # GCP credential detection
```

## What you get

Adding `fabryk` to your project gives you the full Fabryk core library:

| Module | Crate | What it provides |
|--------|-------|------------------|
| `fabryk::core` | `fabryk-core` | Shared types, traits, error handling, service management |
| `fabryk::auth` | `fabryk-auth` | Token validation trait, Tower auth middleware |
| `fabryk::acl` | `fabryk-acl` | Access control primitives |
| `fabryk::content` | `fabryk-content` | Markdown parsing, frontmatter extraction |
| `fabryk::fts` | `fabryk-fts` | Full-text search traits and types |
| `fabryk::graph` | `fabryk-graph` | Knowledge graph storage and traversal |
| `fabryk::vector` | `fabryk-vector` | Vector/semantic search traits and types |

## What you don't get

The following are **not** included in `fabryk` and must be added separately:

- **`fabryk-mcp`** — MCP server infrastructure and tools (see [fabryk-mcp](../fabryk-mcp/README.md))
- **`fabryk-auth-google`** — Google OAuth2 / JWKS provider
- **`fabryk-gcp`** — GCP credential detection utilities
- **`fabryk-redis`** — Redis connection management
- **`fabryk-cli`** — CLI framework

## Feature flags

| Feature | What it enables |
|---------|----------------|
| `fts-tantivy` | Tantivy full-text search backend |
| `graph-rkyv-cache` | rkyv binary serialization for graph caching |
| `vector-lancedb` | LanceDB vector database backend |
| `vector-fastembed` | Local embedding generation via fastembed |
| `full` | All of the above |

Without any features enabled, the search and graph crates provide only their
trait definitions and types — no heavy dependencies are compiled.

## Crate Map

### Core (`fabryk`)

| Crate | Purpose |
|-------|---------|
| `fabryk-core` | Shared types, traits, error handling, service management |
| `fabryk-auth` | Token validation trait and Tower auth middleware |
| `fabryk-acl` | Access control primitives |
| `fabryk-content` | Markdown parsing, frontmatter extraction |
| `fabryk-fts` | Full-text search (Tantivy backend) |
| `fabryk-graph` | Knowledge graph storage and traversal (petgraph) |
| `fabryk-vector` | Vector/semantic search (LanceDB + fastembed) |

### MCP (`fabryk-mcp`)

| Crate | Purpose |
|-------|---------|
| `fabryk-mcp-core` | MCP server infrastructure, tool registry (rmcp) |
| `fabryk-mcp-auth` | RFC 9728/8414 OAuth2 discovery endpoints |
| `fabryk-mcp-content` | Content and source MCP tools |
| `fabryk-mcp-fts` | Full-text search MCP tools |
| `fabryk-mcp-graph` | Graph query MCP tools |
| `fabryk-mcp-semantic` | Hybrid search MCP tools |

### Vendor-specific (separate dependencies)

| Crate | Purpose |
|-------|---------|
| `fabryk-auth-google` | Google OAuth2 / JWKS provider |
| `fabryk-gcp` | GCP credential detection |
| `fabryk-redis` | Redis connection management |
| `fabryk-cli` | CLI framework with graph commands |

## Key Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| Knowledge Graph | [petgraph](https://crates.io/crates/petgraph) | Graph data structures and algorithms |
| Full-Text Search | [Tantivy](https://crates.io/crates/tantivy) | Rust-native search engine |
| Vector Search | [LanceDB](https://lancedb.com/) | Embedded vector database |
| Embeddings | [fastembed](https://crates.io/crates/fastembed) | Local embedding generation |
| MCP Server | [rmcp](https://crates.io/crates/rmcp) | Model Context Protocol SDK |
| Auth | [jsonwebtoken](https://crates.io/crates/jsonwebtoken) | JWT validation |
| CLI | [clap](https://crates.io/crates/clap) | Command-line parsing |
| Configuration | [confyg](https://crates.io/crates/confyg) | Hierarchical config |

## License

Apache-2.0

[//]: ---Named-Links---

[logo]: assets/images/v1-y250.png
[logo-large]: assets/images/v1.png
[crate]: https://crates.io/crates/fabryk
[crate-badge]: https://img.shields.io/crates/v/fabryk.svg
[docs]: https://docs.rs/fabryk/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
