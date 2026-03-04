# MCP Metadata & Discoverability How-To

This guide shows how to give AI agents a first-class experience when connecting
to your fabryk-powered MCP server. By the end, your server will:

- Tell agents **what it does** on first connect (via `ServerInfo.instructions`)
- Auto-inject a **directory tool** that returns a JSON manifest of all capabilities
- Enrich every tool's description with structured **WHEN TO USE / RETURNS / NEXT** sections
- Optionally advertise **external connectors**, a **query strategy**, and **data freshness**

## Prerequisites

Add `fabryk-mcp` to your `Cargo.toml` (you likely already have it):

```toml
fabryk-mcp = "0.1"
```

The `DiscoverableRegistry` and `ToolMeta` types are in the crate root — no
feature flags required.

## Quick Start

```rust
use fabryk_mcp::{
    CompositeRegistry, DiscoverableRegistry, FabrykMcpServer, ToolMeta,
};

// 1. Build your composite registry as usual
let registry = CompositeRegistry::new()
    .add(content_tools)
    .add(search_tools)
    .add(graph_tools);

// 2. Wrap it in DiscoverableRegistry
let discoverable = DiscoverableRegistry::new(registry, "myapp")
    .with_tool_metas(vec![
        ("search".into(), ToolMeta {
            summary: "Full-text search across all content.".into(),
            when_to_use: "Looking for items by keyword or name.".into(),
            returns: "Ranked list of matching items with snippets.".into(),
            next: Some("Call get_item for full details.".into()),
            category: Some("search".into()),
        }),
        // ... metadata for each tool
    ]);

// 3. Build the server with discoverable instructions
let server = FabrykMcpServer::new(discoverable)
    .with_name("myapp")
    .with_version(env!("CARGO_PKG_VERSION"))
    .with_discoverable_instructions("myapp");

server.serve_stdio().await?;
```

When an AI agent connects, it sees:

1. **`ServerInfo.instructions`**: "ALWAYS call myapp_directory first — it maps
   all available tools, valid filter values, and the optimal query strategy for
   this session."

2. **`myapp_directory` tool** in the tool list (its description includes the
   total tool count, e.g., "Describes all 12 myapp tools"). When called, it
   returns a JSON manifest with:
   - `myapp_tools` — array of tools with `name`, `category`, `use_when`, `what_it_does`
   - `categories` — summary object (e.g., `{"search": 3, "graph": 8, "content": 6}`)
   - `external_connectors` — (if any) array of external services
   - `optimal_query_strategy` — (if set) ordered steps
   - `data_freshness` — (if set) per-source freshness info

3. **Enriched tool descriptions** like:

   ```
   Full-text search across all content.
   WHEN TO USE: Looking for items by keyword or name.
   RETURNS: Ranked list of matching items with snippets.
   NEXT: Call get_item for full details.
   ```

## API Reference

### `ToolMeta`

Structured metadata for a single tool:

```rust
pub struct ToolMeta {
    /// Brief human-readable summary (1 sentence).
    pub summary: String,
    /// When an AI agent should call this tool.
    pub when_to_use: String,
    /// What the tool returns.
    pub returns: String,
    /// Suggested next tool(s) to call after this one.
    pub next: Option<String>,
    /// Optional category for grouping (e.g., "search", "graph", "content").
    pub category: Option<String>,
}
```

All fields except `next` and `category` should be non-empty. The `summary`
replaces the tool's original description; if empty, the original is preserved.

### `DiscoverableRegistry<R>`

Wraps any `ToolRegistry` to enrich descriptions and auto-inject a directory tool.

| Method | Description |
|--------|-------------|
| `::new(registry, server_name)` | Wrap an existing registry |
| `.with_tool_meta(name, meta)` | Add metadata for a single tool |
| `.with_tool_metas(vec)` | Add metadata for multiple tools at once |
| `.with_connector(connector)` | Advertise an external service |
| `.with_query_strategy(steps)` | Recommended query steps (ordered) |
| `.with_data_freshness(map)` | Data freshness per source |
| `.directory_tool_name()` | Returns `"{server_name}_directory"` |

### `FabrykMcpServer` Builder

| Method | Description |
|--------|-------------|
| `.with_name(name)` | Server name in MCP `ServerInfo` |
| `.with_version(version)` | Server version |
| `.with_description(text)` | Custom `ServerInfo.instructions` |
| `.with_discoverable_instructions(name)` | Auto-generate instructions telling agents to call the directory tool first |

**Composing descriptions:** `with_discoverable_instructions` **composes** with
any previously set description. If you call `with_description` first, the
directory directive is prepended and your custom text is preserved after it.
This lets you combine domain context with the directory instruction:

```rust
FabrykMcpServer::new(discoverable)
    .with_description("Java/Quarkus knowledge server with 1,425 concept cards.")
    .with_discoverable_instructions("kasu")
// Result: "ALWAYS call kasu_directory first — ...\n\nJava/Quarkus knowledge server with 1,425 concept cards."
```

### `ExternalConnector`

Advertise capabilities beyond registered MCP tools:

```rust
use fabryk_mcp::ExternalConnector;

let discoverable = DiscoverableRegistry::new(registry, "myapp")
    .with_connector(ExternalConnector {
        name: "Slack MCP".into(),
        when_to_use: "Looking for recent team messages.".into(),
        description: "Live Slack message search via MCP bridge.".into(),
    });
```

## Complete Example

Here is the full pattern used by the `kasu-server` (Java/Quarkus knowledge
server):

```rust
use std::collections::HashMap;
use fabryk_mcp::{
    CompositeRegistry, DiagnosticTools, DiscoverableRegistry,
    FabrykMcpServer, HealthTools, ServiceAwareRegistry, ToolMeta,
};

// 1. Compose tool registries
let registry = CompositeRegistry::new()
    .add(content_tools)
    .add(ServiceAwareRegistry::new(fts_tools, vec![fts_svc.clone()]))
    .add(ServiceAwareRegistry::new(graph_tools, vec![graph_svc.clone()]))
    .add(ServiceAwareRegistry::new(semantic_tools, vec![fts_svc.clone(), vector_svc.clone()]))
    .add(diagnostics)
    .add(health);

// 2. Wrap in DiscoverableRegistry with full metadata
let discoverable = DiscoverableRegistry::new(registry, "kasu")
    .with_tool_metas(vec![
        ("search".into(), ToolMeta {
            summary: "Full-text keyword search across concept cards.".into(),
            when_to_use: "Looking for exact terms, class names, or API references.".into(),
            returns: "Ranked results with highlighted snippets.".into(),
            next: Some("Call concepts_get with a result slug.".into()),
            category: Some("search".into()),
        }),
        ("concepts_get".into(), ToolMeta {
            summary: "Retrieve a concept card by slug.".into(),
            when_to_use: "You have a slug and need the full card content.".into(),
            returns: "Full YAML frontmatter + markdown body.".into(),
            next: Some("Use graph_related to explore connections.".into()),
            category: Some("content".into()),
        }),
        // ... entries for all other tools
    ])
    .with_query_strategy(vec![
        "1. Call kasu_directory first.",
        "2. Use semantic_search for natural-language questions.",
        "3. Use search for precise keyword lookups.",
        "4. Use concepts_get once you know the slug.",
        "5. Use graph_related / graph_neighborhood for connections.",
    ])
    .with_data_freshness({
        let mut f = HashMap::new();
        f.insert("concept_cards".into(), "1,425 cards — static corpus".into());
        f.insert("graph".into(), "Built at startup from frontmatter".into());
        f
    });

// 3. Build the server
let server = FabrykMcpServer::new(discoverable)
    .with_name("kasu-server")
    .with_version(env!("CARGO_PKG_VERSION"))
    .with_discoverable_instructions("kasu")
    .with_services(vec![graph_svc, fts_svc, vector_svc]);

server.serve_stdio().await?;
```

## Testing the Directory Tool

You can test the directory output without a live server by calling the registry
directly:

```rust
#[tokio::test]
async fn test_directory_output() {
    let registry = build_your_registry();
    let discoverable = DiscoverableRegistry::new(registry, "myapp")
        .with_tool_metas(your_metadata());

    // Call the directory tool
    let result = discoverable
        .call("myapp_directory", serde_json::json!({}))
        .unwrap()
        .await
        .unwrap();

    // Parse the JSON response
    let text = match &result.content[0].raw {
        rmcp::model::RawContent::Text(t) => &t.text,
        _ => panic!("Expected text"),
    };
    let json: serde_json::Value = serde_json::from_str(text).unwrap();

    // Verify structure
    assert!(json.get("myapp_tools").is_some());
    let tools = json["myapp_tools"].as_array().unwrap();
    assert!(tools.iter().any(|t| t["name"] == "search"));

    // Categories summary is auto-generated from ToolMeta.category
    let cats = json.get("categories").expect("Should have categories");
    assert!(cats["search"].as_u64().unwrap() > 0);
}
```

## Design Guidelines

**Write metadata for the AI agent, not for humans.** The `when_to_use` field
should describe the *user intent* that triggers the tool ("Looking for exact
terms") not the tool mechanics ("Performs a Tantivy BM25 query").

**Chain tools with `next`.** Guide agents through multi-step workflows:
search → get → graph_related → get. This reduces wasted calls.

**Use categories.** They appear in both the per-tool entries and the
`categories` summary object in the directory output, helping agents understand
the tool landscape at a glance. Tools without a category default to `"general"`.
Common categories: `content`, `search`, `graph`, `diagnostics`.

**Keep summaries to one sentence.** The enriched description already includes
WHEN TO USE / RETURNS / NEXT sections — the summary just needs to say *what*
the tool does.

**Provide a query strategy.** Numbered steps in `with_query_strategy` give
agents a playbook. Put the directory tool first, the most common workflow
second.
