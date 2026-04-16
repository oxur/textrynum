# Building a Knowledge Fabric MCP Server with Fabryk

A comprehensive guide to building an MCP (Model Context Protocol) server that
exposes a knowledge corpus through full-text search, knowledge graphs, semantic
vector search, and structured content access — using the Fabryk crate
ecosystem.

**Audience:** Rust developers building MCP servers for AI-assisted knowledge
exploration. Assumes familiarity with Rust async patterns and Cargo workspaces.

**Reference implementation:** [ai-music-theory](https://github.com/oxur/ai-music-theory)
— a production Fabryk MCP server with 50+ tools.

**What you'll build:** An MCP server that:

- Loads a markdown corpus with YAML frontmatter
- Builds a full-text search index (Tantivy)
- Constructs a knowledge graph with relationship extraction
- Creates a vector index for semantic search (LanceDB + FastEmbed)
- Exposes all of these as MCP tools for Claude (or any MCP client)
- Adds custom domain-specific computation tools
- Supports both stdio and HTTP transport with optional OAuth2
- Reports health, provides tool discovery, and handles graceful startup

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Project Setup](#2-project-setup)
3. [Configuration](#3-configuration)
4. [Content Providers](#4-content-providers)
5. [Full-Text Search](#5-full-text-search)
6. [Knowledge Graph](#6-knowledge-graph)
7. [Vector & Semantic Search](#7-vector--semantic-search)
8. [Custom Domain Tools](#8-custom-domain-tools)
9. [Server Composition with ServerBuilder](#9-server-composition-with-serverbuilder)
10. [Tool Composition & Registries](#10-tool-composition--registries)
11. [Service Lifecycle & Health](#11-service-lifecycle--health)
12. [Static Resources](#12-static-resources)
13. [Tool Metadata & Discoverability](#13-tool-metadata--discoverability)
14. [Authentication & HTTP Transport](#14-authentication--http-transport)
15. [CLI Integration](#15-cli-integration)
16. [Cache Management](#16-cache-management)
17. [Testing](#17-testing)
18. [Deployment](#18-deployment)
19. [Quick Start Checklist](#19-quick-start-checklist)
20. [Pattern Reference](#20-pattern-reference)

---

## 1. Architecture Overview

Fabryk organizes into six layers. Your project depends on whichever layers you
need — the umbrella crates (`fabryk`, `fabryk-mcp`) aggregate them for
convenience.

```
┌────────────────────────────────────────────────────────────┐
│                   Your MCP Server Binary                   │
├────────────────────────────────────────────────────────────┤
│  MCP Layer (fabryk-mcp)                                    │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────┐  │
│  │ Content  │ │   FTS    │ │  Graph   │ │   Semantic    │  │
│  │  Tools   │ │  Tools   │ │  Tools   │ │ Search Tools  │  │
│  └────┬─────┘ └────┬─────┘ └────┬─────┘ └──────┬────────┘  │
│       │            │            │              │           │
│  ┌────┴────────────┴────────────┴──────────────┴────────┐  │
│  │  fabryk-mcp-core: ServerBuilder, FabrykMcpServer,    │  │
│  │  CompositeRegistry, ServiceAwareRegistry,            │  │
│  │  DiscoverableRegistry, HealthTools, helpers           │  │
│  └──────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────┤
│  Domain Layer (fabryk)                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Content  │ │   FTS    │ │  Graph   │ │  Vector  │       │
│  │ (md/yaml)│ │(tantivy) │ │(petgraph)│ │(lancedb) │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
├────────────────────────────────────────────────────────────┤
│  Core Layer: fabryk-core (ServiceHandle, BackendSlot,      │
│  ConfigManager, Error, AppState, PathResolver)              │
├────────────────────────────────────────────────────────────┤
│  CLI Layer: fabryk-cli (FabrykCli, cache management,       │
│  config/graph/vector/sources commands)                      │
├────────────────────────────────────────────────────────────┤
│  Infrastructure: fabryk-auth, fabryk-redis, fabryk-gcp     │
└────────────────────────────────────────────────────────────┘
```

**Key design principles:**

- **Trait-driven extensibility** — Your domain implements extractors
  (`GraphExtractor`, `DocumentExtractor`) or uses built-in ones
  (`ConceptCardGraphExtractor`, `ConceptCardDocumentExtractor`).
  Fabryk handles the rest.
- **Feature-gated heavy dependencies** — Tantivy, LanceDB, FastEmbed are
  opt-in. Lightweight fallbacks (`SimpleSearch`) always available.
- **Service lifecycle management** — `BackendSlot<T>` combines service
  state tracking with value storage. `ServiceAwareRegistry` gates tool
  availability on service readiness.
- **Composable registries** — Mix and match tool groups via
  `CompositeRegistry`. Add metadata via `DiscoverableRegistry`.
  Gate on readiness via `ServiceAwareRegistry`.
- **ServerBuilder** — Fluent API for composing the final server from
  tool registries, resources, service handles, and metadata.

---

## 2. Project Setup

### Cargo.toml

```toml
[package]
name = "my-knowledge-server"
version = "0.1.0"
edition = "2024"

[dependencies]
# Fabryk domain layer (umbrella)
fabryk = { version = "0.5", features = [
    "fts-tantivy",       # Full-text search with Tantivy
    "graph-rkyv-cache",  # Graph persistence with rkyv + blake3
    "vector-lancedb",    # Vector search with LanceDB
    "vector-fastembed",  # Local embedding generation
] }

# Fabryk MCP layer (umbrella)
fabryk-mcp = { version = "0.5", features = [
    "http",              # HTTP transport (axum-based)
] }

# CLI framework
fabryk-cli = { version = "0.5", features = ["vector-fastembed"] }

# Standard dependencies
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
clap = { version = "4", features = ["derive"] }
log = "0.4"
```

### Feature Selection Guide

| Scenario | Features |
|----------|----------|
| **Minimal** (content + simple search) | None (defaults work) |
| **Full-text search** | `fts-tantivy` |
| **Knowledge graph** | `graph-rkyv-cache` |
| **Semantic search** | `vector-lancedb`, `vector-fastembed` |
| **HTTP transport** | `http` on fabryk-mcp |
| **CLI tools** | `vector-fastembed` on fabryk-cli |

---

## 3. Configuration

Implement `ConfigManager` and `ConfigProvider` from `fabryk::core`:

```rust
use fabryk::core::{ConfigManager, ConfigProvider};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub paths: PathsConfig,
    pub search: fabryk::fts::SearchConfig,  // Re-use fabryk's type
    pub lancedb: fabryk::fts::LanceDbConfig, // Re-use fabryk's type
}

impl ConfigProvider for Config {
    fn project_name(&self) -> &str { &self.server.name }
    fn base_path(&self) -> fabryk::core::Result<PathBuf> { expand_path(&self.paths.base) }
    fn content_path(&self, content_type: &str) -> fabryk::core::Result<PathBuf> {
        match content_type {
            "concepts" => expand_path(&self.paths.concepts),
            "sources" => expand_path(&self.paths.sources),
            _ => Err(Error::config(format!("Unknown content type: {}", content_type))),
        }
    }
    fn cache_path(&self, cache_type: &str) -> fabryk::core::Result<PathBuf> {
        let base = self.base_path()?;
        match cache_type {
            "fts" => resolve_index_path(&self.search),
            "graph" => Ok(base.join("data/graphs")),
            _ => Ok(base.join(".cache").join(cache_type)),
        }
    }
}

impl ConfigManager for Config {
    fn load(config_path: Option<&str>) -> fabryk::core::Result<Self> {
        // Load from TOML using confyg or similar
    }
}
```

**Tip:** Use `fabryk::fts::SearchConfig` and `fabryk::fts::LanceDbConfig` directly
rather than defining your own. Set project-specific defaults in `Config::default()`:

```rust
pub fn default_search_config() -> SearchConfig {
    SearchConfig {
        backend: "simple".to_string(),
        index_path: Some(".tantivy-index".to_string()),
        fuzzy_distance: 2,
        allowlist: vec!["I", "V", "do", "re", "mi"].into_iter().map(String::from).collect(),
        ..SearchConfig::default()
    }
}
```

---

## 4. Content Providers

Fabryk provides filesystem-based providers out of the box. Use them with
`ContentTools`, `SourceTools`, and `GuideTools`:

```rust
use fabryk_mcp::content::{
    ContentTools, FsContentItemProvider, FsGuideProvider, FsSourceProvider,
    GuideTools, SourceTools,
};

// Concept cards (list, get, categories)
let content_provider = Arc::new(
    FsContentItemProvider::new(&concept_cards_path)
        .with_content_type_name("concept", "concepts"),
);
let concept_tools = ContentTools::with_shared(content_provider)
    .with_names(HashMap::from([
        ("list".into(), "list_concepts".into()),
        ("get".into(), "get_concept".into()),
        ("categories".into(), "list_categories".into()),
    ]))
    .with_descriptions(HashMap::from([
        ("list".into(), "List concept cards with optional filtering".into()),
        ("get".into(), "Retrieve a specific concept card".into()),
    ]))
    .with_get_id_field("concept_id")
    .with_extra_list_schema(serde_json::json!({
        "tier": { "type": "string", "enum": ["foundational", "intermediate", "advanced"] }
    }));

// Source materials (list, chapters, get chapter, availability, PDF path)
let source_provider = Arc::new(FsSourceProvider::new(&sources_path));
let source_tools = SourceTools::with_shared(source_provider)
    .with_names(HashMap::from([...]))
    .with_descriptions(HashMap::from([...]));

// Guides
let guide_provider = FsGuideProvider::new(&guides_path);
let guide_tools = GuideTools::new(guide_provider);
```

### Customizing Tool Names

Every tool registry supports `.with_names()` and `.with_descriptions()` to
override the default slot names. Use `.with_extra_list_schema()`,
`.with_extra_search_schema()`, and `.with_extra_schema()` to add
domain-specific filter parameters to tool schemas.

---

## 5. Full-Text Search

### Multi-Directory Indexing

Index content from multiple directories with per-directory labels:

```rust
use fabryk::fts::{build_index_multi, ConceptCardDocumentExtractor};

let content_dirs: Vec<(PathBuf, &str)> = vec![
    (concept_cards_path, "concept_cards"),
    (sources_md_path, "source_chapters"),
    (guides_path, "guides"),
];

let extractor = ConceptCardDocumentExtractor::new();
let stats = build_index_multi(&content_dirs, &index_path, Box::new(extractor)).await?;

log::info!("Indexed {} docs, {} errors", stats.documents_indexed, stats.errors);
// Per-directory counts available via stats.label_counts
```

### Search Backend Fallback

Use `resolve_search_backend` for transparent FTS-with-simple fallback:

```rust
use fabryk_mcp::{resolve_search_backend, resolve_backend_name};

// Always returns a working backend:
// - FTS if the tantivy index is ready
// - SimpleSearch (linear scan) otherwise
let backend = resolve_search_backend(Some(&fts_slot), &simple_backend);
let name = resolve_backend_name(Some(&fts_slot)); // "tantivy" or "simple"
```

### Register with MCP

```rust
use fabryk_mcp::fts::FtsTools;

let search_tools = FtsTools::with_shared(search_backend)
    .with_names(HashMap::from([("search".into(), "search_concepts".into())]))
    .with_extra_search_schema(serde_json::json!({
        "tier": { "type": "string", "enum": ["foundational", "intermediate", "advanced"] },
        "content_types": { "type": "array", "items": {"type": "string"} }
    }));
```

---

## 6. Knowledge Graph

### Building the Graph

```rust
use fabryk::graph::{GraphBuilder, ConceptCardGraphExtractor, LoadedGraph};

let extractor = ConceptCardGraphExtractor::new();
let mut builder = GraphBuilder::new(extractor).with_content_path(&content_path);

// Optional: merge hand-curated edges
if manual_edges_path.exists() {
    builder = builder.with_manual_edges(&manual_edges_path);
}

let (graph_data, build_stats) = builder.build().await?;
let loaded = LoadedGraph::new(graph_data);
// loaded.stats has node_count, edge_count, type_counts, category_distribution, etc.
```

### Node Inspection

`Node` provides convenience methods for metadata access:

```rust
if node.is_domain() { /* concept node */ }
if node.is_custom_type("source") { /* source node */ }
let category = node.category_or_default(); // "unknown" if None
let author = node.metadata_str("author").unwrap_or("Unknown");
let year = node.metadata_u64("year").map(|y| y as u16);
```

### Register with MCP

```rust
use fabryk_mcp::graph::GraphTools;

let graph_tools = GraphTools::with_shared(Arc::clone(&shared_graph))
    .with_node_filter(node_filter)
    .with_names(HashMap::from([
        (GraphTools::SLOT_RELATED.into(), "get_related_concepts".into()),
        (GraphTools::SLOT_PATH.into(), "find_concept_path".into()),
        (GraphTools::SLOT_PREREQUISITES.into(), "get_prerequisites".into()),
        (GraphTools::SLOT_LEARNING_PATH.into(), "get_learning_path".into()),
        // ... 17 total slots
    ]))
    .with_descriptions(HashMap::from([...]))
    .with_extra_schema(GraphTools::SLOT_RELATED, tier_confidence_schema())
    .with_extra_schema(GraphTools::SLOT_PREREQUISITES, tier_confidence_schema());
```

### Metadata Node Filtering

Filter graph query results by metadata (e.g., tier, confidence):

```rust
use fabryk_mcp::graph::MetadataNodeFilter;

let node_filter = Arc::new(
    MetadataNodeFilter::new()
        .with_exact("tier", "tier")
        .with_ordered("min_confidence", "extraction_confidence", &["low", "medium", "high"]),
);
```

Use `fabryk_mcp::tier_confidence_schema()` for the standard tier/confidence
JSON schema.

---

## 7. Vector & Semantic Search

### Deferred Vector Backend (VectorSlot)

Register semantic search tools before the vector index is ready:

```rust
use fabryk_mcp::semantic::SemanticSearchTools;

let vector_slot: VectorSlot = Arc::new(RwLock::new(None));

// Register immediately — queries fail gracefully until ready
let semantic_tools = SemanticSearchTools::with_vector_slot(
    search_backend,
    vector_slot.clone(),
);

// Later, when index is built:
*vector_slot.write().await = Some(vector_backend);
```

---

## 8. Custom Domain Tools

Most real servers need domain-specific computation tools beyond content/search/graph.
Implement `ToolRegistry` directly:

```rust
use fabryk_mcp::{make_tool, serialize_response, ToolRegistry, ToolResult, McpErrorContextExt};
use fabryk_mcp::model::{ErrorCode, ErrorData, Tool};

struct MyDomainToolsRegistry;

impl ToolRegistry for MyDomainToolsRegistry {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                "compute_something",
                "Compute a domain-specific result",
                json!({
                    "type": "object",
                    "properties": {
                        "input": { "type": "string", "description": "The input value" }
                    },
                    "required": ["input"]
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        match name {
            "compute_something" => Some(Box::pin(async move {
                let args: ComputeArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        format!("Invalid parameters: {}", e),
                        None,
                    ))?;
                let result = do_computation(args)
                    .map_err(|e| e.to_mcp_error_with_context("Computation error"))?;
                serialize_response(&result)
            })),
            _ => None,
        }
    }
}
```

### Key Helpers

- **`make_tool(name, description, schema)`** — Construct a Tool from JSON schema
- **`serialize_response(value)`** — Serialize any `T: Serialize` into a successful CallToolResult
- **`McpErrorContextExt::to_mcp_error_with_context(context)`** — Convert fabryk errors to MCP ErrorData with context

---

## 9. Server Composition with ServerBuilder

`ServerBuilder` is the recommended way to assemble an MCP server:

```rust
use fabryk_mcp::ServerBuilder;

let server = ServerBuilder::new()
    .name("my-knowledge-server")
    .version(env!("CARGO_PKG_VERSION"))
    .description("My server with query strategy guidance for LLMs...")
    .resources_path(skill_docs_path)
    .with_services(vec![fts_service, graph_service, vector_service])
    // Add tool registries
    .add(concept_tools)
    .add(guide_tools)
    .add(source_tools)
    .add(search_tools)
    .add(semantic_tools)
    .add(health_tools)
    .add(my_domain_tools)
    // Add static resources
    .with_resource(StaticResourceDef { ... })
    .build();

// Serve
server.serve_stdio().await?;
// or
server.serve_http(addr).await?;
```

### With DiscoverableRegistry

For tool metadata and a directory tool, use `into_parts()`:

```rust
let (registry, parts) = builder.into_parts();

let discoverable = DiscoverableRegistry::new(registry, "myapp")
    .with_tool_meta("search", ToolMeta { ... })
    .with_tool_meta("get_item", ToolMeta { ... });

let server = ServerBuilder::build_with_registry(discoverable, parts);
```

---

## 10. Tool Composition & Registries

### CompositeRegistry — Combine Independent Tool Groups

```rust
use fabryk_mcp::CompositeRegistry;

let registry = CompositeRegistry::new()
    .add(content_tools)
    .add(search_tools)
    .add(graph_tools)
    .add(health_tools)
    .add(my_domain_tools);
```

### ServiceAwareRegistry — Gate Tools on Service Readiness

Tools are listed but calls return "service is starting" until ready:

```rust
use fabryk_mcp::ServiceAwareRegistry;

let gated_graph = ServiceAwareRegistry::new(
    graph_tools,
    vec![state.graph.service().clone()],
);
```

### DiscoverableRegistry — Add Metadata & Directory Tool

```rust
use fabryk_mcp::{DiscoverableRegistry, ToolMeta};

let discoverable = DiscoverableRegistry::new(registry, "kb")
    .with_tool_meta("search", ToolMeta {
        summary: "Full-text search across all content".into(),
        when_to_use: "When looking for content by keyword".into(),
        returns: "Ranked results with relevance scores".into(),
        next: Some("get_item for full details".into()),
        category: Some("search".into()),
    });
// Auto-generates: kb_directory tool
```

---

## 11. Service Lifecycle & Health

### BackendSlot<T>

`BackendSlot<T>` combines a `ServiceHandle` (lifecycle state) with
an `RwLock<Option<T>>` (stored value). Use it for backends that load
asynchronously:

```rust
use fabryk::core::BackendSlot;

pub struct AppState {
    pub fts: BackendSlot<Arc<TantivySearch>>,
    pub graph: BackendSlot<LoadedGraph>,
    pub vector: BackendSlot<Arc<dyn VectorBackend>>,
}

// Check readiness
if state.fts.is_ready() { ... }

// Store a value
state.graph.set(loaded_graph)?;
state.graph.service().set_state(ServiceState::Ready);

// Read the value
if let Ok(guard) = state.fts.inner().read() {
    if let Some(ref backend) = *guard { ... }
}
```

### Search Fallback

Always-available simple search that upgrades to FTS when ready:

```rust
use fabryk_mcp::resolve_search_backend;

let backend = resolve_search_backend(
    Some(&state.fts),
    &simple_backend,
);
```

### Health Endpoint (HTTP)

Register service handles with `ServerBuilder` for automatic `/health`:

```rust
let server = ServerBuilder::new()
    .with_services(vec![
        state.fts.service().clone(),
        state.graph.service().clone(),
    ])
    // ...
    .build();

// serve_http automatically adds /health endpoint
// Returns 200 when all Ready, 503 when any Starting/Failed
server.serve_http(addr).await?;
```

### Built-in Health Tool (MCP)

```rust
use fabryk_mcp::HealthTools;

let health_tools = HealthTools::new(&name, &version, 0)
    .with_backends(probes)
    .with_search_config(search_config_info);
```

---

## 12. Static Resources

Expose reference documents via MCP resources with fallback content:

```rust
use fabryk_mcp::StaticResourceDef;

builder
    .with_resource(StaticResourceDef {
        uri: "skill://conventions".into(),
        name: "Conventions".into(),
        description: "Notation conventions".into(),
        mime_type: "text/markdown".into(),
        filename: "CONVENTIONS.md".into(),
        fallback: Some(default_conventions_content()),
    })
    .with_resource(StaticResourceDef {
        uri: "skill://scope".into(),
        name: "Scope".into(),
        description: "Topics covered".into(),
        mime_type: "text/markdown".into(),
        filename: "SCOPE.md".into(),
        fallback: Some(default_scope_content()),
    });
```

The `fallback` content is served if the file doesn't exist on disk.

---

## 13. Tool Metadata & Discoverability

### Query Strategy Guidance

The server description doubles as instructions for LLMs:

```rust
.description(
    "My Knowledge Server — comprehensive materials and computation.\n\n\
     QUERY STRATEGY:\n\
     1. Start with semantic_search for open-ended questions\n\
     2. Use search for keyword lookups\n\
     3. Use get_item to read full content\n\
     4. Use graph_related to explore connections\n\
     5. Use graph_prerequisites for learning order",
)
```

### ToolMeta for Every Tool

Write `ToolMeta` for your tools — especially `when_to_use` and `next`:

```rust
.with_tool_meta("get_learning_path", ToolMeta {
    summary: "Topologically sorted prerequisites for a target concept".into(),
    when_to_use: "When planning a study sequence".into(),
    returns: "Ordered learning steps with tier annotations".into(),
    next: Some("get_item for each step".into()),
    category: Some("graph".into()),
})
```

---

## 14. Authentication & HTTP Transport

See the original guide — this section is unchanged. OAuth2 via
`fabryk-auth-google` and `fabryk-mcp-auth` for HTTP transport.

---

## 15. CLI Integration

Use `fabryk-cli` types for standard commands:

```rust
use fabryk_cli::{CacheCommand, CacheAction, ConfigAction, SourcesCommand};
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    Serve { ... },
    #[cfg(feature = "fts")]
    Index { force: bool },
    #[cfg(feature = "graph")]
    Graph(GraphCommands),
    Config { action: Option<ConfigAction> },
    Sources(SourcesCommand),
    Cache(CacheCommand),
}
```

---

## 16. Cache Management

Distribute pre-built caches via GitHub Releases:

```rust
use fabryk_cli::cache::{
    CacheProject, BackendPaths, PackagePaths,
    download_cache, package_cache, cache_status, parse_backend_arg,
};

let project = CacheProject {
    prefix: "my-project".into(),
    release_base_url: "https://github.com/org/repo/releases/download".into(),
};

// Download pre-built caches
download_cache(&CacheBackend::Graph, &base_path, version, &project, false)?;

// Check status
let report = cache_status(&base_path, &backend_paths)?;
println!("{report}");

// Package for distribution
package_cache(&CacheBackend::Fts, &base_path, &output_dir, version, &project, &paths)?;
```

---

## 17. Testing

### Schema Validation

```rust
use fabryk_mcp::assert_tools_valid;

#[tokio::test]
async fn test_all_tools_have_valid_schemas() {
    let server = build_server(state);
    assert_tools_valid(server.registry());
}
```

### Health Endpoint (HTTP)

```rust
use fabryk_mcp::health_router;

#[tokio::test]
async fn test_health_returns_200() {
    let svc = ServiceHandle::new("test");
    svc.set_state(ServiceState::Ready);
    let app = health_router(vec![svc]);
    // ... test with axum oneshot
}
```

---

## 18. Deployment

### Cloud Run

`serve_http` automatically merges a `/health` endpoint that returns 503
while services are loading and 200 when all are Ready. Configure Cloud
Run's startup probe against `/health`.

---

## 19. Quick Start Checklist

- [ ] Create project, add Fabryk dependencies (§2)
- [ ] Implement `ConfigProvider` + `ConfigManager` (§3)
- [ ] Set up content providers with custom tool names (§4)
- [ ] Configure FTS with multi-directory indexing (§5)
- [ ] Configure knowledge graph with metadata filtering (§6)
- [ ] Implement custom domain tools with `make_tool` + `serialize_response` (§8)
- [ ] Compose with `ServerBuilder` (§9)
- [ ] Add `DiscoverableRegistry` with `ToolMeta` (§10, §13)
- [ ] Gate async tools with `ServiceAwareRegistry` (§10)
- [ ] Register service handles for health (§11)
- [ ] Add static resources with fallbacks (§12)
- [ ] Add `assert_tools_valid` test (§17)
- [ ] Register with Claude Code: `claude mcp add my-server -- cargo run`

---

## 20. Pattern Reference

| Pattern | Section | Purpose |
|---------|---------|---------|
| `ConfigManager` + `ConfigProvider` | §3 | Application config + Fabryk builder bridge |
| `FsContentItemProvider` | §4 | Filesystem content → MCP content tools |
| `ConceptCardDocumentExtractor` | §5 | Content → FTS search documents |
| `build_index_multi` | §5 | Multi-directory index building |
| `resolve_search_backend` | §5 | FTS-with-simple fallback |
| `ConceptCardGraphExtractor` | §6 | Content → knowledge graph |
| `LoadedGraph` | §6 | Graph data + stats + timestamp |
| `Node` methods | §6 | `is_domain()`, `metadata_str()`, etc. |
| `MetadataNodeFilter` | §6 | Filter graph results by metadata |
| `tier_confidence_schema` | §6 | Standard tier/confidence filter schema |
| `VectorSlot` | §7 | Deferred vector backend wiring |
| `make_tool` + `serialize_response` | §8 | Custom domain tool helpers |
| `McpErrorContextExt` | §8 | Context-aware error mapping |
| `ServerBuilder` | §9 | Fluent server composition |
| `into_parts` / `build_with_registry` | §9 | Registry wrapping before build |
| `CompositeRegistry` | §10 | Combine independent tool groups |
| `ServiceAwareRegistry` | §10 | Gate tools on service readiness |
| `DiscoverableRegistry` + `ToolMeta` | §10, §13 | Tool metadata + directory |
| `BackendSlot<T>` | §11 | Service lifecycle + value storage |
| `health_router` / `.with_services()` | §11 | HTTP health endpoint |
| `StaticResourceDef` | §12 | MCP resources with fallback content |
| `CacheProject` + cache management | §16 | Pre-built cache distribution |
| `assert_tools_valid` | §17 | Schema validation in tests |
| `FabrykMcpServer` | §9 | MCP server (stdio or HTTP) |

### Related Howtos

- [MCP Async Startup](mcp-async-startup.md) — ServiceHandle, BackendSlot, spawn_with_retry
- [MCP Health](mcp-health.md) — Health endpoint, HTTP status codes
- [MCP Metadata](mcp-metadata.md) — DiscoverableRegistry, ToolMeta
- [MCP with Claude Code](mcp-with-claude-code.md) — Registration, .mcp.json
