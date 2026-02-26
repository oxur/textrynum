# Fabryk

[![][logo]][logo-large]

 *A hyper-connected, multi-model knowlege fabric* • Part of the [Textrynum](README.md) project

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

## Crate Map

| Tier | Crate | Purpose |
|------|-------|---------|
| Foundation | `fabryk-core` | Shared types, traits, error handling |
| Content | `fabryk-content` | Markdown parsing, frontmatter extraction |
| Search | `fabryk-fts` | Full-text search (Tantivy backend) |
| Search | `fabryk-graph` | Knowledge graph storage & traversal (petgraph) |
| Search | `fabryk-vector` | Vector/semantic search (LanceDB + fastembed) |
| Auth | `fabryk-auth` | Token validation & Tower middleware |
| Auth | `fabryk-auth-google` | Google OAuth2 / JWKS provider |
| Auth | `fabryk-auth-mcp` | RFC 9728 OAuth2 discovery endpoints |
| MCP | `fabryk-mcp` | Core MCP server infrastructure (rmcp) |
| MCP | `fabryk-mcp-content` | Content & source MCP tools |
| MCP | `fabryk-mcp-fts` | Full-text search MCP tools |
| MCP | `fabryk-mcp-graph` | Graph query MCP tools |
| CLI | `fabryk-cli` | CLI framework with graph commands |
| ACL | `fabryk-acl` | Access control (placeholder) |
| Umbrella | `fabryk` | Re-exports everything, feature-gated |

Use the umbrella crate to pull in what you need:

```toml
# Just graph and FTS
fabryk = { version = "0.1", features = ["graph", "fts-tantivy"] }

# Everything including MCP server
fabryk = { version = "0.1", features = ["full"] }
```

## Key Dependencies

| Component | Library | Purpose |
|-----------|---------|---------|
| Knowledge Graph | [petgraph](https://crates.io/crates/petgraph) | Graph data structures & algorithms |
| Full-Text Search | [Tantivy](https://crates.io/crates/tantivy) | Rust-native search engine |
| Vector Search | [LanceDB](https://lancedb.com/) | Embedded vector database |
| Embeddings | [fastembed](https://crates.io/crates/fastembed) | Local embedding generation |
| MCP Server | [rmcp](https://crates.io/crates/rmcp) | Model Context Protocol |
| Auth | [jsonwebtoken](https://crates.io/crates/jsonwebtoken) | JWT validation |
| CLI | [clap](https://crates.io/crates/clap) | Command-line parsing |
| Configuration | [confyg](https://crates.io/crates/confyg) | Hierarchical config |

[//]: ---Named-Links---

[logo]: assets/images/fabryk/v1-y250.png
[logo-large]: assets/images/fabryk/v1.png
