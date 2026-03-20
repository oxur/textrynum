# MCP Metadata & Discoverability How-To

This guide shows how to give AI agents a first-class experience when connecting
to your fabryk-powered MCP server. By the end, your server will:

- Tell agents **what it does** on first connect (via `ServerInfo.instructions`)
- Auto-inject a **directory tool** that returns a JSON manifest of all capabilities
- Enrich every tool's description with structured **WHEN TO USE / RETURNS / NEXT** sections
- Advertise **external connectors**, a **query strategy**, and **data freshness**
- Expose the **domain model** so agents understand the data, not just the tools

## Why This Matters

AI agents discover MCP servers through a narrow keyhole: a list of tool names,
parameter schemas, and short descriptions. That's enough to know the API surface
but not enough to understand *what the server is for* or *how to use it well*.

Without discoverability metadata, an agent's first session looks like this:

1. Call `health` to check if the server is alive
2. Call `list_sources`, `list_categories`, `graph_stats` to understand the data
3. Try a search to see what comes back and how IDs work
4. Only *then* start doing useful work

That's 3–5 wasted round-trips just to build a mental model. With proper
metadata, the agent gets all of this in a single directory call and goes
straight to productive work.

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
   - `domain_model` — what the data *is* (see Domain Model section below)
   - `id_conventions` — how identifiers work across tools
   - `backend_status` — which capabilities are available right now
   - `external_connectors` — (if any) array of external services
   - `optimal_query_strategy` — (if set) ordered steps by task type
   - `data_freshness` — (if set) per-source freshness info

3. **Enriched tool descriptions** like:

   ```
   Full-text search across all content.
   WHEN TO USE: Looking for items by keyword or name.
   RETURNS: Ranked list of matching items with snippets.
   NEXT: Call get_item for full details.
   ```

## Domain Model

This is the most important addition to the directory output. Tools tell agents
*how* to interact with a server. The domain model tells them *what they're
interacting with*.

### The Problem

An agent connecting to a music theory knowledge server sees tools named
`search_concepts`, `get_learning_path`, `list_sources`. It can infer parameter
schemas. But it doesn't know:

- What is a "concept"? How big is the corpus?
- What sources exist? Are they books, papers, scraped websites?
- What relationship types does the graph use?
- How do IDs work — are they slugs, UUIDs, integers?
- Is vector search actually available right now, or only keyword?

Without this context, the agent makes exploratory calls: `list_sources`,
`list_categories`, `graph_stats`, `health` — 3–5 round-trips before it can
do anything meaningful.

### The Solution

Add a `domain_model` section to your directory output:

```rust
let discoverable = DiscoverableRegistry::new(registry, "music-theory")
    .with_domain_model(DomainModel {
        summary: "Music theory knowledge base with 4,300+ concept cards \
                  extracted from 16 source texts (textbooks, papers, open \
                  educational resources). Concepts are organized into a \
                  prerequisite graph with 18,600+ edges across 4 relationship \
                  types. Searchable via full-text (Tantivy) and vector \
                  (embeddings) backends.".into(),
        entities: vec![
            Entity {
                name: "concept_card".into(),
                description: "A single music theory concept extracted from \
                    a source text. Contains title, category, tier, description, \
                    competency questions, and prerequisite links.".into(),
                id_format: "Kebab-case slug derived from title, e.g., \
                    'neapolitan-sixth', 'sonata-form-exposition'. The same \
                    slug may appear across multiple sources.".into(),
                count: Some(4315),
            },
            Entity {
                name: "source".into(),
                description: "A textbook or reference work from which concepts \
                    were extracted. Sources have chapters and may be in markdown \
                    (converted) or original format (PDF/EPUB, not yet indexed).\
                    ".into(),
                id_format: "Kebab-case slug, e.g., 'open-music-theory', \
                    'geometry-of-music'.".into(),
                count: Some(31),
            },
        ],
        relationships: vec![
            "Prerequisite (6,400+ edges) — concept A is required before concept B",
            "RelatesTo (9,000+ edges) — concepts cover similar or overlapping material",
            "Extends (1,600+ edges) — concept B builds on or deepens concept A",
            "ContrastsWith (1,400+ edges) — concepts differ in a meaningful way",
        ],
    });
```

This produces a `domain_model` section in the directory JSON:

```json
{
  "domain_model": {
    "summary": "Music theory knowledge base with 4,300+ concept cards ...",
    "entities": [
      {
        "name": "concept_card",
        "description": "A single music theory concept ...",
        "id_format": "Kebab-case slug derived from title ...",
        "count": 4315
      }
    ],
    "relationships": [
      "Prerequisite (6,400+ edges) — concept A is required before concept B",
      "..."
    ]
  }
}
```

An agent reading this immediately knows: the corpus is ~4,300 concept cards from
~31 sources, IDs are kebab-case slugs, the graph has 4 named relationship types,
and search uses Tantivy. Zero exploratory calls needed.

### ID Conventions

A surprisingly common source of agent confusion is not knowing how identifiers
flow between tools. Search returns results with an `id` field — is that the
same `concept_id` that graph tools expect? What format are they in?

Add `id_conventions` to the directory:

```rust
let discoverable = discoverable
    .with_id_conventions(vec![
        IdConvention {
            name: "concept_id".into(),
            format: "Kebab-case slug, e.g., 'neapolitan-sixth'.".into(),
            used_by: vec![
                "search_concepts (returned as 'id' in results)".into(),
                "get_concept (input parameter)".into(),
                "get_prerequisites (input parameter)".into(),
                "get_learning_path (input parameter)".into(),
                "find_concept_path (from_id / to_id parameters)".into(),
            ],
        },
        IdConvention {
            name: "source_id".into(),
            format: "Kebab-case slug, e.g., 'open-music-theory'.".into(),
            used_by: vec![
                "list_sources (returned as 'id')".into(),
                "get_source_coverage (input parameter)".into(),
                "list_source_chapters (input parameter)".into(),
            ],
        },
    ]);
```

This eliminates the guesswork agents face when chaining tools together.

## Backend Status

Health endpoints tell you whether a service is running. Backend status in the
directory tells agents what they can *do* right now.

```rust
let discoverable = discoverable
    .with_backend_status(BackendStatus {
        capabilities: vec![
            Capability {
                name: "full_text_search".into(),
                ready: true,
                note: None,
            },
            Capability {
                name: "vector_search".into(),
                ready: false,
                note: Some("Embedding index not built. Use keyword mode \
                    for semantic_search.".into()),
            },
            Capability {
                name: "graph_traversal".into(),
                ready: true,
                note: None,
            },
        ],
    });
```

This is distinct from the `health` tool. Health is for operational monitoring.
Backend status is for **capability negotiation** — it tells the agent which
tools will actually work and suggests alternatives when something is degraded.

The directory JSON includes:

```json
{
  "backend_status": {
    "capabilities": [
      { "name": "full_text_search", "ready": true },
      { "name": "vector_search", "ready": false,
        "note": "Embedding index not built. Use keyword mode for semantic_search." },
      { "name": "graph_traversal", "ready": true }
    ]
  }
}
```

An agent seeing this knows to avoid `mode: "hybrid"` or `mode: "vector"` on
`semantic_search` and will use `mode: "keyword"` instead — without needing to
make a failing call first.

## Task-Oriented Query Strategies

The original `with_query_strategy` produces a flat numbered list of tool calls.
This works for simple servers, but for richer servers with multiple workflows,
task-oriented strategies are more effective.

Instead of:

```
1. Call directory first.
2. Use semantic_search for questions.
3. Use search for keywords.
4. Use get_concept for details.
5. Use graph tools for connections.
```

Provide strategies keyed by **what the agent is trying to accomplish**:

```rust
let discoverable = discoverable
    .with_task_strategies(vec![
        TaskStrategy {
            task: "Explore a topic".into(),
            steps: vec![
                "search_concepts with the topic as query".into(),
                "get_concept on the most relevant result".into(),
                "get_related_concepts to see what connects to it".into(),
            ],
        },
        TaskStrategy {
            task: "Build a learning path".into(),
            steps: vec![
                "search_concepts to find the target concept".into(),
                "get_learning_path with the concept ID".into(),
                "get_concept on each step if full content is needed".into(),
            ],
        },
        TaskStrategy {
            task: "Compare how different sources treat a topic".into(),
            steps: vec![
                "search_concepts with source filter for each source".into(),
                "get_concept_sources to see all sources covering it".into(),
                "get_concept on each source's version".into(),
            ],
        },
        TaskStrategy {
            task: "Find connections between two domains".into(),
            steps: vec![
                "find_bridge_concepts with the two category names".into(),
                "find_concept_path between specific concepts".into(),
                "get_concept on bridge concepts for details".into(),
            ],
        },
        TaskStrategy {
            task: "Audit data quality or coverage".into(),
            steps: vec![
                "health for backend status and index stats".into(),
                "graph_stats for relationship distribution".into(),
                "list_sources for conversion status".into(),
                "graph_validate for structural issues".into(),
            ],
        },
    ]);
```

The directory JSON includes:

```json
{
  "query_strategies": [
    {
      "task": "Explore a topic",
      "steps": ["search_concepts with the topic as query", "..."]
    },
    {
      "task": "Build a learning path",
      "steps": ["search_concepts to find the target concept", "..."]
    }
  ]
}
```

Agents can match the user's request to a task pattern and follow the
corresponding recipe, reducing both wasted calls and hallucinated workflows.

## Filter Values

Another source of wasted calls: agents don't know what values are valid for
filter parameters. If `search_concepts` accepts a `category` filter, the agent
needs to know the valid categories — otherwise it guesses, gets zero results,
then calls `list_categories` to find the right value.

Include summary filter values in the directory:

```rust
let discoverable = discoverable
    .with_filter_summary(FilterSummary {
        categories: vec![
            "harmony (503)", "analysis (316)", "form (259)",
            "sonata-form (239)", "voice-leading (170)", "... 72 more",
        ],
        tiers: vec!["foundational", "intermediate", "advanced"],
        sources: vec![
            "open-music-theory (123 chapters)",
            "complete-musician (42 chapters)",
            "... 14 more converted, 15 not yet converted",
        ],
    });
```

You don't need to enumerate every value — the top N plus a count is enough
to orient the agent and prevent the most common guessing errors.

## External Connectors

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

### `DomainModel`

Describes what the server's data *is*, not what the tools *do*:

```rust
pub struct DomainModel {
    /// One-paragraph summary of the entire dataset/corpus.
    pub summary: String,
    /// The core entity types an agent will encounter.
    pub entities: Vec<Entity>,
    /// Named relationship types (for graph-based servers).
    pub relationships: Vec<String>,
}

pub struct Entity {
    /// Entity type name as it appears in tool parameters/results.
    pub name: String,
    /// What this entity represents.
    pub description: String,
    /// How IDs are formatted and where they flow between tools.
    pub id_format: String,
    /// Approximate count (helps agents calibrate expectations).
    pub count: Option<u64>,
}
```

### `BackendStatus`

Runtime capability information for agents to negotiate available features:

```rust
pub struct BackendStatus {
    pub capabilities: Vec<Capability>,
}

pub struct Capability {
    /// Capability name matching a tool or tool family.
    pub name: String,
    /// Whether this capability is currently functional.
    pub ready: bool,
    /// Agent-facing guidance when not ready (e.g., "Use keyword mode instead").
    pub note: Option<String>,
}
```

### `TaskStrategy`

Task-oriented query recipe:

```rust
pub struct TaskStrategy {
    /// What the user is trying to accomplish.
    pub task: String,
    /// Ordered tool calls to achieve it.
    pub steps: Vec<String>,
}
```

### `IdConvention`

Documents how identifiers flow between tools:

```rust
pub struct IdConvention {
    /// The ID field name as it appears in parameters.
    pub name: String,
    /// Format description (slug, UUID, integer, etc.).
    pub format: String,
    /// Which tools produce or consume this ID.
    pub used_by: Vec<String>,
}
```

### `DiscoverableRegistry<R>`

Wraps any `ToolRegistry` to enrich descriptions and auto-inject a directory tool.

| Method | Description |
|--------|-------------|
| `::new(registry, server_name)` | Wrap an existing registry |
| `.with_tool_meta(name, meta)` | Add metadata for a single tool |
| `.with_tool_metas(vec)` | Add metadata for multiple tools at once |
| `.with_domain_model(model)` | Describe the data entities and relationships |
| `.with_id_conventions(vec)` | Document ID formats and cross-tool flow |
| `.with_backend_status(status)` | Runtime capability availability |
| `.with_task_strategies(vec)` | Task-oriented query recipes |
| `.with_filter_summary(summary)` | Valid values for common filter parameters |
| `.with_connector(connector)` | Advertise an external service |
| `.with_query_strategy(steps)` | Simple numbered query steps (legacy) |
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
    .with_description("Music theory knowledge server with 4,300+ concept cards.")
    .with_discoverable_instructions("music-theory")
// Result: "ALWAYS call music_theory_directory first — ...\n\nMusic theory knowledge server with 4,300+ concept cards."
```

## Complete Example

Here is the full pattern for a music theory knowledge server:

```rust
use std::collections::HashMap;
use fabryk_mcp::{
    BackendStatus, Capability, CompositeRegistry, DiagnosticTools,
    DiscoverableRegistry, DomainModel, Entity, FabrykMcpServer, FilterSummary,
    HealthTools, IdConvention, ServiceAwareRegistry, TaskStrategy, ToolMeta,
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
let discoverable = DiscoverableRegistry::new(registry, "music-theory")
    .with_domain_model(DomainModel {
        summary: "Music theory knowledge base with 4,300+ concept cards \
                  extracted from 16 source texts spanning tonal harmony, \
                  counterpoint, form, set theory, neo-Riemannian theory, \
                  and mathematical music theory. Concepts are organized \
                  into a prerequisite graph with 18,600+ edges.".into(),
        entities: vec![
            Entity {
                name: "concept_card".into(),
                description: "A single music theory concept with title, \
                    category, tier, description, and competency questions.\
                    ".into(),
                id_format: "Kebab-case slug, e.g., 'neapolitan-sixth'. \
                    Same slug may exist across multiple sources.".into(),
                count: Some(4315),
            },
            Entity {
                name: "source".into(),
                description: "A textbook or reference. 16 converted to \
                    markdown (chapters searchable), 15 in original format \
                    (registered but not yet indexed).".into(),
                id_format: "Kebab-case slug, e.g., 'open-music-theory'.\
                    ".into(),
                count: Some(31),
            },
        ],
        relationships: vec![
            "Prerequisite (6,471) — A is required before B".into(),
            "RelatesTo (9,095) — overlapping or similar material".into(),
            "Extends (1,615) — B deepens or builds on A".into(),
            "ContrastsWith (1,437) — meaningful difference".into(),
        ],
    })
    .with_id_conventions(vec![
        IdConvention {
            name: "concept_id".into(),
            format: "Kebab-case slug".into(),
            used_by: vec![
                "search_concepts → 'id' in results".into(),
                "semantic_search → 'id' in results".into(),
                "get_concept → concept_id param".into(),
                "get_prerequisites → concept_id param".into(),
                "get_learning_path → target_id param".into(),
                "find_concept_path → from_id / to_id params".into(),
                "get_concept_sources → concept_id param".into(),
            ],
        },
        IdConvention {
            name: "source_id".into(),
            format: "Kebab-case slug".into(),
            used_by: vec![
                "list_sources → 'id' in results".into(),
                "get_source_coverage → source_id param".into(),
                "list_source_chapters → source_id param".into(),
                "check_source_availability → source_id param".into(),
            ],
        },
    ])
    .with_backend_status(BackendStatus {
        // NOTE: Populate these dynamically from actual service state
        capabilities: vec![
            Capability {
                name: "full_text_search".into(),
                ready: fts_svc.is_ready(),
                note: None,
            },
            Capability {
                name: "vector_search".into(),
                ready: vector_svc.is_ready(),
                note: if !vector_svc.is_ready() {
                    Some("Embedding index not loaded. Use mode:'keyword' \
                          for semantic_search.".into())
                } else { None },
            },
            Capability {
                name: "graph".into(),
                ready: graph_svc.is_ready(),
                note: None,
            },
        ],
    })
    .with_task_strategies(vec![
        TaskStrategy {
            task: "Explore a topic".into(),
            steps: vec![
                "search_concepts with topic keywords".into(),
                "get_concept on the best-matching slug".into(),
                "get_related_concepts to see connections".into(),
            ],
        },
        TaskStrategy {
            task: "Build a learning path to a concept".into(),
            steps: vec![
                "search_concepts to find the target concept slug".into(),
                "get_learning_path with that slug as target_id".into(),
                "get_concept on individual steps for full content".into(),
            ],
        },
        TaskStrategy {
            task: "Compare sources on a topic".into(),
            steps: vec![
                "get_concept_sources to find all sources covering it".into(),
                "get_concept with source filter for each version".into(),
            ],
        },
        TaskStrategy {
            task: "Find cross-domain connections".into(),
            steps: vec![
                "find_bridge_concepts between two category names".into(),
                "find_concept_path between specific concept slugs".into(),
            ],
        },
        TaskStrategy {
            task: "QA / audit the knowledge base".into(),
            steps: vec![
                "health for backend + index stats".into(),
                "graph_stats for relationship distribution".into(),
                "graph_validate for structural issues".into(),
                "list_sources for conversion coverage".into(),
            ],
        },
    ])
    .with_filter_summary(FilterSummary {
        note: "Top categories and sources shown; use list_categories \
               or list_sources for complete lists.".into(),
        filters: vec![
            FilterInfo {
                param: "category".into(),
                top_values: vec![
                    "harmony (503)", "analysis (316)", "form (259)",
                    "sonata-form (239)", "voice-leading (170)",
                    "technique (167)", "fundamentals (149)", "chords (146)",
                ],
                total: 77,
            },
            FilterInfo {
                param: "tier".into(),
                top_values: vec![
                    "foundational", "intermediate", "advanced",
                ],
                total: 3,
            },
            FilterInfo {
                param: "source".into(),
                top_values: vec![
                    "open-music-theory (123 ch)",
                    "complete-musician (42 ch)",
                    "21st-century-classroom (40 ch)",
                    "tonality-owners-manual (30 ch)",
                ],
                total: 31,
            },
        ],
    })
    .with_tool_metas(vec![
        ("search_concepts".into(), ToolMeta {
            summary: "Full-text keyword search across concept cards.".into(),
            when_to_use: "Looking for concepts by name, term, or keyword.".into(),
            returns: "Ranked results with id, title, category, source, \
                      snippet, and relevance score.".into(),
            next: Some("get_concept with a result's id for full content, \
                        or get_concept_sources to see cross-source coverage.\
                        ".into()),
            category: Some("search".into()),
        }),
        ("semantic_search".into(), ToolMeta {
            summary: "Natural-language concept search (keyword, vector, \
                      or hybrid mode).".into(),
            when_to_use: "Asking a question rather than searching a keyword. \
                          Falls back to keyword mode if vector index is \
                          unavailable.".into(),
            returns: "Results with id, score, source metadata.".into(),
            next: Some("get_concept for full card content.".into()),
            category: Some("search".into()),
        }),
        // ... remaining tool metas
    ])
    .with_data_freshness({
        let mut f = HashMap::new();
        f.insert("concept_cards".into(), "4,315 cards — static corpus, \
                  last indexed at startup".into());
        f.insert("graph".into(), "18,618 edges — built at startup from \
                  frontmatter".into());
        f.insert("sources".into(), "16 of 31 sources converted to markdown; \
                  15 registered but not yet indexed".into());
        f
    });

// 3. Build the server
let server = FabrykMcpServer::new(discoverable)
    .with_name("music-theory")
    .with_version(env!("CARGO_PKG_VERSION"))
    .with_description("Music theory knowledge server: 4,300+ concept cards, \
        18,600+ graph edges, 16 indexed source texts.")
    .with_discoverable_instructions("music-theory")
    .with_services(vec![graph_svc, fts_svc, vector_svc]);

server.serve_stdio().await?;
```

## Testing the Directory Tool

```rust
#[tokio::test]
async fn test_directory_output() {
    let registry = build_your_registry();
    let discoverable = DiscoverableRegistry::new(registry, "music-theory")
        .with_domain_model(your_domain_model())
        .with_tool_metas(your_metadata());

    let result = discoverable
        .call("music_theory_directory", serde_json::json!({}))
        .unwrap()
        .await
        .unwrap();

    let text = match &result.content[0].raw {
        rmcp::model::RawContent::Text(t) => &t.text,
        _ => panic!("Expected text"),
    };
    let json: serde_json::Value = serde_json::from_str(text).unwrap();

    // Verify all sections are present
    assert!(json.get("music_theory_tools").is_some());
    assert!(json.get("domain_model").is_some());
    assert!(json.get("id_conventions").is_some());
    assert!(json.get("backend_status").is_some());
    assert!(json.get("query_strategies").is_some());

    // Verify domain model has substance
    let model = &json["domain_model"];
    assert!(!model["summary"].as_str().unwrap().is_empty());
    assert!(model["entities"].as_array().unwrap().len() > 0);

    // Verify ID conventions connect tools
    let ids = json["id_conventions"].as_array().unwrap();
    assert!(ids.iter().any(|c| c["name"] == "concept_id"));

    // Verify backend status is populated
    let caps = json["backend_status"]["capabilities"].as_array().unwrap();
    assert!(caps.iter().any(|c| c["name"] == "full_text_search"));
}
```

## Design Guidelines

### Write metadata for the AI agent, not for humans

The `when_to_use` field should describe the **user intent** that triggers the
tool ("Looking for exact terms") not the tool mechanics ("Performs a Tantivy
BM25 query"). The agent needs to map a user's request to a tool — it doesn't
need to know the implementation.

### Describe the data, not just the tools

The single biggest improvement you can make to agent experience is telling it
**what the data is**. "4,300 concept cards from 16 source texts, organized in
a prerequisite graph with 4 relationship types" is worth more than perfect tool
descriptions, because it lets the agent calibrate expectations, explain the
corpus to the user, and avoid pointless exploratory calls.

### Document ID flow between tools

Agents chain tools. If `search_concepts` returns results with an `id` field and
`get_learning_path` expects a `target_id` parameter, the agent needs to know
these are the same value. Explicit `id_conventions` in the directory eliminate
this guesswork.

### Expose degraded capabilities, not just health

`health` tells you the server is running. `backend_status` tells you what
**works right now**. If vector search isn't ready, say so in the directory and
suggest an alternative ("Use keyword mode"). This prevents agents from making
calls that will fail and then having to diagnose why.

### Use task-oriented strategies, not tool sequences

"Call search, then get, then graph" is a tool sequence. "To build a learning
path, do X → Y → Z" is a task strategy. Agents match user requests to tasks,
not to tool chains. Write strategies from the user's perspective.

### Chain tools with `next`

Guide agents through multi-step workflows: search → get → graph_related → get.
This reduces wasted calls and teaches the agent the natural rhythm of your
server's API.

### Provide summary filter values

Include the top N values for filterable parameters (categories, tiers, sources)
in the directory. Agents guess filter values constantly — giving them the
vocabulary up front prevents empty-result round-trips.

### Keep summaries to one sentence

The enriched description already includes WHEN TO USE / RETURNS / NEXT
sections — the summary just needs to say *what* the tool does.

### Use categories to group tools

Categories appear in both the per-tool entries and the `categories` summary in
the directory, helping agents understand the tool landscape at a glance. Tools
without a category default to `"general"`. Common categories: `content`,
`search`, `graph`, `diagnostics`.

### Make backend_status dynamic

The `BackendStatus` should reflect **actual runtime state**, not a static
declaration. Query your service health at directory-call time so the agent
gets current information. A vector index that was unavailable at startup might
be ready 10 minutes later.
