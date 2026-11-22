# Fabryk MCP

This crate's legacy logging notifications remain available for existing MCP
clients. RMCP 3.11 deprecates logging under SEP-2577 and provides no replacement
with the same broadcast semantics; Fabryk keeps the existing notification
contract for compatibility. Clients that have moved beyond SEP-2577 should not
depend on server logging notifications.

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

## Current workspace source compatibility (unreleased 0.5.0)

The examples above retain the historical published `0.2` usage. The current
workspace source is preparing an unreleased `0.5.0`; its MCP SDK update uses
the flat `Resource` and `ContentBlock` model directly, without the former
`Annotated`/`RawResource` wrapper layer. This describes the source checkout,
not a released package.

For custom tool results, replace the old text facade:

```rust,ignore
// Earlier source
use fabryk_mcp::model::{CallToolResult, Content};
let result = CallToolResult::success(vec![Content::text("ready")]);
```

with the current model:

```rust,ignore
use fabryk_mcp::model::{CallToolResult, ContentBlock};
let result = CallToolResult::success(vec![ContentBlock::text("ready")]);
let text = result.content[0].as_text().expect("text result").text.as_str();
```

For resources, replace `Annotated::new(RawResource::new(uri, name), None)`
with `Resource::new(uri, name)`. Description and MIME type are builder
methods on `Resource`:

```rust,ignore
use fabryk_mcp::model::Resource;

let resource = Resource::new("acme://guide", "Guide")
    .with_description("Acme integration guide")
    .with_mime_type("text/markdown");
```

The Fabryk `ToolRegistry` and `CallToolResult` contracts remain the same; the
SDK's `ServerHandler` boundary now returns `CallToolResponse` and
`ReadResourceResponse`, which the server adapts internally. HTTP service
defaults retain stateful legacy sessions and loopback Host protection. For a
public deployment, set the allowed host through the existing configuration:

```rust,ignore
use rmcp::transport::streamable_http_server::StreamableHttpServerConfig;

let config = StreamableHttpServerConfig::default()
    .with_allowed_hosts(["mcp.acme.example"]);
let service = server.into_http_service_with_config(config);
```

The workspace minimum Rust version is 1.94.

## License

Apache-2.0

[//]: ---Named-Links---

[logo]: assets/images/v1-y250.png
[logo-large]: assets/images/v1.png
[crate]: https://crates.io/crates/fabryk-mcp
[crate-badge]: https://img.shields.io/crates/v/fabryk-mcp.svg
[docs]: https://docs.rs/fabryk-mcp/
[docs-badge]: https://img.shields.io/badge/rust-documentation-blue.svg
