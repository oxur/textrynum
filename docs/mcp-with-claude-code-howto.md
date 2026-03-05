# Connecting Fabryk MCP Servers to Claude Code

This guide covers how to connect a fabryk-based MCP server to Claude Code using the Streamable HTTP transport — the current standard from the MCP 2025-03-26 specification.

## Transport options

Claude Code supports three MCP transport types: `stdio`, `http`, and `sse`.

| Transport | When to use |
|-----------|-------------|
| `stdio` | Default. Runs the server as a local subprocess. Best for development. |
| `http` | Recommended for remote/deployed servers. Implements Streamable HTTP. |
| `sse` | Legacy (MCP 2024-11-05). Deprecated — avoid for new servers. |

A fabryk MCP server supports both `stdio` and `http` transports. The `http` transport requires the `http` feature flag:

```toml
[dependencies]
fabryk-mcp = { version = "0.1", features = ["http"] }
```

## Registering a server with Claude Code

### Via CLI

For a local stdio server (the typical development setup):

```bash
claude mcp add my-server /path/to/my-server-binary
```

For a remote HTTP server:

```bash
claude mcp add --transport http my-server https://my-server.example.com/mcp
```

With authentication:

```bash
claude mcp add --transport http my-server \
  https://my-server.example.com/mcp \
  --header "Authorization: Bearer $(gcloud auth print-identity-token)"
```

### Via `.mcp.json`

Place a `.mcp.json` at the project root for team-shareable configuration:

```json
{
  "mcpServers": {
    "my-server": {
      "command": "/path/to/my-server-binary",
      "args": ["--config", "config.toml"]
    }
  }
}
```

For a remote HTTP server:

```json
{
  "mcpServers": {
    "my-server": {
      "type": "http",
      "url": "https://my-server.example.com/mcp",
      "headers": {
        "Authorization": "Bearer ${MCP_AUTH_TOKEN}"
      }
    }
  }
}
```

Environment variable expansion (`${VAR}` and `${VAR:-default}`) is supported in `url`, `headers`, `command`, `args`, and `env` fields, so tokens never need to be hardcoded.

Configuration scopes control visibility:
- `local` (default) — private, stored in `~/.claude.json`
- `project` — shared via `.mcp.json` in the repo
- `user` — cross-project, stored in `~/.claude.json`

## Building a fabryk MCP server

A fabryk MCP server is built by composing tool registries, wrapping them with optional middleware (discovery metadata, service gating), and running via the chosen transport.

### Minimal example (stdio)

```rust
use fabryk_mcp::{FabrykMcpServer, CompositeRegistry};
use fabryk_mcp_content::ContentTools;
use fabryk_mcp_fts::FtsTools;
use fabryk_mcp_graph::GraphTools;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let content_tools = ContentTools::new(my_provider).with_prefix("docs");
    let fts_tools = FtsTools::from_boxed(my_fts_backend);
    let graph_tools = GraphTools::new(my_graph);

    let registry = CompositeRegistry::new()
        .add(content_tools)
        .add(fts_tools)
        .add(graph_tools);

    FabrykMcpServer::new(registry)
        .with_name("my-server")
        .with_version(env!("CARGO_PKG_VERSION"))
        .serve_stdio()
        .await?;

    Ok(())
}
```

### With discovery metadata and service health

```rust
use fabryk_mcp::{
    FabrykMcpServer, CompositeRegistry, DiscoverableRegistry,
    ServiceAwareRegistry, ToolMeta,
};
use fabryk_core::service::ServiceHandle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create service handles for health tracking.
    let fts_svc = ServiceHandle::new("fts");
    let graph_svc = ServiceHandle::new("graph");

    // Build domain tool registries.
    let registry = CompositeRegistry::new()
        .add(content_tools)
        .add(fts_tools)
        .add(graph_tools);

    // Add discovery metadata (auto-injects a directory tool).
    let discoverable = DiscoverableRegistry::new(registry, "my-server")
        .with_tool_meta("docs_list", ToolMeta {
            summary: "List all documents".into(),
            when_to_use: "Finding available documents".into(),
            returns: "List of document names and IDs".into(),
            next: Some("Call docs_get for details".into()),
            category: Some("content".into()),
        });

    // Gate tools on service readiness.
    let gated = ServiceAwareRegistry::new(
        discoverable,
        vec![fts_svc.clone(), graph_svc.clone()],
    );

    FabrykMcpServer::new(gated)
        .with_name("my-server")
        .with_version(env!("CARGO_PKG_VERSION"))
        .with_description("My domain knowledge server")
        .with_services(vec![fts_svc, graph_svc])
        .serve_stdio()
        .await?;

    Ok(())
}
```

### HTTP transport

To serve over HTTP instead of stdio, switch the final call:

```rust
FabrykMcpServer::new(registry)
    .with_name("my-server")
    .serve_http("0.0.0.0:3000".parse()?)
    .await?;
```

This starts an HTTP server with:
- MCP Streamable HTTP endpoint at the root
- `/health` endpoint that returns 200 when all services are ready, 503 otherwise

For custom routing (e.g., mounting the MCP endpoint alongside other routes):

```rust
let mcp_service = server.into_http_service();
let router = axum::Router::new()
    .route("/custom", /* ... */)
    .nest_service("/mcp", mcp_service);
```

## Available fabryk MCP tool crates

| Crate | Tools | Purpose |
|-------|-------|---------|
| `fabryk-mcp-content` | `{prefix}_list`, `{prefix}_get`, `{prefix}_categories` | Content item browsing |
| `fabryk-mcp-fts` | `search`, `search_status` | Full-text search (Tantivy) |
| `fabryk-mcp-graph` | `graph_related`, `graph_path`, `graph_prerequisites`, `graph_neighborhood`, `graph_centrality`, `graph_bridges` | Knowledge graph queries (petgraph) |
| `fabryk-mcp-semantic` | Vector/semantic search | Semantic search (LanceDB + fastembed) |

Each crate implements the `ToolRegistry` trait and can be composed via `CompositeRegistry`.

## SSE vs Streamable HTTP

The legacy SSE transport (MCP 2024-11-05) required two endpoints (`/sse` + `/messages`), a persistent connection, and had no session management or resumability. The Streamable HTTP transport (MCP 2025-03-26) uses a single endpoint, supports per-request operation, and adds session management via `Mcp-Session-Id` headers and resumability via `Last-Event-ID`. Use `http` for all new servers.

## Authentication

Claude Code supports several authentication methods:

- **Custom headers** — pass `--header "Authorization: Bearer token"` on the CLI or set `"headers"` in `.mcp.json`
- **OAuth 2.0/2.1** — Claude Code initiates a browser-based login flow for servers that advertise OAuth
- **Pre-configured OAuth** — provide `--client-id` and `--client-secret` for servers without Dynamic Client Registration
- **GCP identity tokens** — use `gcloud auth print-identity-token` for Cloud Run deployments

## Deploying to GCP Cloud Run

For Cloud Run deployments, the HTTP transport with stateless mode is essential since requests may be routed to different container instances.

Key considerations:
- Set `--min-instances=1` to avoid cold start latency
- Increase `--timeout` for long-running tool calls (default is 300s, max 3600s)
- Use `--no-allow-unauthenticated` and pass identity tokens via headers
- For local development, `gcloud run services proxy` creates an authenticated tunnel:

```bash
gcloud run services proxy my-server --region=us-central1 --port=3000
claude mcp add --transport http my-server http://127.0.0.1:3000/mcp
```

For production:

```bash
claude mcp add --transport http my-server \
  https://my-service-abc123-uc.a.run.app/mcp \
  --header "Authorization: Bearer $(gcloud auth print-identity-token)"
```
