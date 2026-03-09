# Fabryk MCP

[![][crate-badge]][crate]
[![][docs-badge]][docs]

[![][logo]][logo-large]

*MCP tools for the Fabryk knowledge fabric* • Part of the [Textrynum](../../README.md) project

Fabryk MCP exposes the full [Fabryk](../fabryk/README.md) knowledge fabric — content, graph, search, and auth — as **25+ MCP tools** for AI assistants via the [Model Context Protocol](https://modelcontextprotocol.io/).

## What you get

Adding `fabryk-mcp` to your project gives you the full MCP toolkit:

| Module | Crate | What it provides |
|--------|-------|------------------|
| *(root)* | `fabryk-mcp-core` | Server infrastructure, ToolRegistry, CompositeRegistry, FabrykMcpServer |
| `fabryk_mcp::auth` | `fabryk-mcp-auth` | RFC 9728/8414 OAuth2 discovery endpoints |
| `fabryk_mcp::content` | `fabryk-mcp-content` | Content and source MCP tools |
| `fabryk_mcp::fts` | `fabryk-mcp-fts` | Full-text search MCP tools |
| `fabryk_mcp::graph` | `fabryk-mcp-graph` | Graph query MCP tools |
| `fabryk_mcp::semantic` | `fabryk-mcp-semantic` | Hybrid search MCP tools (FTS + vector) |

All `fabryk-mcp-core` symbols are re-exported at the crate root for backward
compatibility, so `fabryk_mcp::FabrykMcpServer`, `fabryk_mcp::CompositeRegistry`,
etc. continue to work.

## What you don't get

The following are **not** included and must be added separately:

- **`fabryk`** — The core knowledge fabric (content, graph, search, auth)
- **`fabryk-auth-google`** — Google OAuth2 provider
- **`fabryk-gcp`** — GCP credential detection

## Usage

```toml
[dependencies]
# MCP tools (no heavy backends by default)
fabryk-mcp = "0.2"

# Enable backends and HTTP transport
fabryk-mcp = { version = "0.2", features = ["http", "fts-tantivy", "graph-rkyv-cache"] }
```

## Feature flags

| Feature | What it enables |
|---------|----------------|
| `http` | Streamable HTTP transport via rmcp (adds axum) |
| `fts-tantivy` | Tantivy backend for FTS tools |
| `graph-rkyv-cache` | rkyv caching for graph tools |

## License

Apache-2.0

[//]: ---Named-Links---

[logo]: assets/images/v1-y250.png
[logo-large]: assets/images/v1.png
[crate]: https://crates.io/crates/fabryk-mcp
[crate-badge]: https://img.shields.io/crates/v/fabryk-mcp.svg
[docs]: https://docs.rs/fabryk-mcp/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
