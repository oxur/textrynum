# Building an MCP Server with Fabryk

**A comprehensive guide based on the ai-kasu reference implementation**

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture at a Glance](#2-architecture-at-a-glance)
3. [Dependencies and Cargo Features](#3-dependencies-and-cargo-features)
4. [Project Structure](#4-project-structure)
5. [Configuration](#5-configuration)
6. [Domain Types and Content Loading](#6-domain-types-and-content-loading)
7. [Content Provider](#7-content-provider)
8. [Extractor Adapters](#8-extractor-adapters)
9. [Error Handling](#9-error-handling)
10. [Registry Composition](#10-registry-composition)
11. [Background Builds with Service Gating](#11-background-builds-with-service-gating)
12. [Tool Metadata and Discoverability](#12-tool-metadata-and-discoverability)
13. [Server and Transport](#13-server-and-transport)
14. [HTTP Transport with OAuth2](#14-http-transport-with-oauth2)
15. [CLI Integration](#15-cli-integration)
16. [Testing](#16-testing)
17. [Refactoring an Existing MCP Server to Fabryk](#17-refactoring-an-existing-mcp-server-to-fabryk)
18. [Quick Start Checklist](#18-quick-start-checklist)

---

## 1. Overview

Fabryk is a Rust crate ecosystem for building knowledge-fabric MCP servers. It provides composable building blocks for:

- **Full-text search** (Tantivy-backed)
- **Knowledge graph** traversal (with rkyv-cached serialisation)
- **Semantic/vector search** (LanceDB + FastEmbed)
- **Content item** browsing and retrieval
- **MCP transport** (stdio and streamable HTTP via rmcp)
- **Authentication** (OAuth2/Google JWKS)
- **CLI management** (config, vectordb commands)

This guide walks through building a complete Fabryk MCP server, using **ai-kasu** (a Java/Quarkus knowledge server with 1,400+ concept cards) as the reference implementation. Every code example is taken directly from ai-kasu's production source.

### Key Design Principles

1. **Immediate availability** — The MCP transport starts before indices finish building. Tools become available progressively as their backing services reach `Ready`.
2. **Composable registries** — Tool groups (content, FTS, graph, semantic) are independently gated and composed via `CompositeRegistry`.
3. **Adapter pattern** — Domain-specific logic lives in extractor adapters that implement fabryk traits; the generic builders handle indexing.
4. **Resilient startup** — Background builds use `spawn_with_retry` with exponential backoff.
5. **Discoverability** — A meta `_directory` tool auto-describes all registered tools, query strategy, and data freshness for LLM clients.

---

## 2. Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────┐
│                        main.rs                              │
│  CLI parse → config/vectordb commands OR run_server()       │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                      run_server()                           │
│                                                             │
│  1. Load domain data (fast, blocking)                       │
│  2. Create ServiceHandles (graph, fts, vector)              │
│  3. Create InMemoryProvider                                 │
│  4. Create shared backend slots (RwLock)                    │
│  5. Compose registry:                                       │
│     ┌─────────────────────────────────────────────────┐     │
│     │           CompositeRegistry                     │     │
│     │  ┌──────────────────────────────────────────┐   │     │
│     │  │ ContentTools (always ready)              │   │     │
│     │  ├──────────────────────────────────────────┤   │     │
│     │  │ ServiceAwareRegistry(FtsTools, [fts])    │   │     │
│     │  ├──────────────────────────────────────────┤   │     │
│     │  │ ServiceAwareRegistry(GraphTools, [graph])│   │     │
│     │  ├──────────────────────────────────────────┤   │     │
│     │  │ ServiceAwareRegistry(SemanticTools,      │   │     │
│     │  │   [fts, vector])                         │   │     │
│     │  ├──────────────────────────────────────────┤   │     │
│     │  │ DiagnosticTools (always ready)           │   │     │
│     │  ├──────────────────────────────────────────┤   │     │
│     │  │ HealthTools (always ready)               │   │     │
│     │  └──────────────────────────────────────────┘   │     │
│     └──────────────┬──────────────────────────────────┘     │
│                    │ wrapped in                              │
│     ┌──────────────▼──────────────────────────────────┐     │
│     │ DiscoverableRegistry (meta-tool, query strategy)│     │
│     └─────────────────────────────────────────────────┘     │
│  6. Spawn background builds (graph, FTS, vector)            │
│  7. Start transport (stdio or HTTP)                         │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Dependencies and Cargo Features

```toml
# Cargo.toml

[package]
name = "your-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "your-server"
path = "src/main.rs"

[dependencies]
# --- Fabryk ecosystem ---
# Umbrella crate — enable the features you need:
#   fts-tantivy: Full-text search
#   graph-rkyv-cache: Knowledge graph with fast cache serialisation
#   vector-lancedb: Vector storage backend
#   vector-fastembed: Local embedding via FastEmbed
fabryk = { version = "0.2", features = [
    "fts-tantivy",
    "graph-rkyv-cache",
    "vector-lancedb",
    "vector-fastembed",
] }

# MCP server framework — "http" enables StreamableHttpService
fabryk-mcp = { version = "0.2", features = ["http"] }

# Shared CLI handlers (config show/set/init, vectordb get-model)
fabryk-cli = { version = "0.2", features = ["vector-fastembed"] }

# OAuth2 (optional — only if using HTTP transport with Google auth)
fabryk-auth-google = "0.2"

# --- MCP transport ---
rmcp = { version = "1.1", features = [
    "server",
    "transport-io",
    "transport-streamable-http-server",
] }

# --- HTTP server ---
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = { version = "0.5", features = ["util"] }
http = "1"

# --- Serialisation ---
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
schemars = "1.2"

# --- Config & logging ---
confyg = "0.3"       # TOML + env var config
twyg = "0.6"         # Structured logging
clap = { version = "4", features = ["derive", "env"] }
dirs = "6"           # XDG directory resolution
toml = "0.9"

# --- Utilities ---
anyhow = "1"
thiserror = "2"
async-trait = "0.1"
log = "0.4"
glob = "0.3"

[dev-dependencies]
tempfile = "3"
```

### Feature Selection Guide

| You need | Enable these features |
|---|---|
| FTS only | `fabryk { features = ["fts-tantivy"] }` |
| FTS + graph | `fabryk { features = ["fts-tantivy", "graph-rkyv-cache"] }` |
| Everything | All four features as shown above |
| Stdio only | `fabryk-mcp = "0.2"` (no "http" feature) |
| No auth | Omit `fabryk-auth-google` entirely |

---

## 4. Project Structure

```
your-server/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI routing
│   ├── lib.rs               # run_server(), registry composition, HTTP setup
│   ├── cli.rs               # Clap CLI definition, config/vectordb handlers
│   ├── config.rs            # KasuConfig + ConfigManager + ConfigProvider
│   ├── error.rs             # Domain error type with thiserror
│   └── knowledge/
│       ├── mod.rs           # Data loading, KnowledgeEngine, spawn_*_build()
│       ├── types.rs         # Domain types (frontmatter, card structs)
│       ├── adapters.rs      # GraphExtractor, DocumentExtractor, VectorExtractor
│       └── provider.rs      # InMemoryProvider implementing ContentItemProvider
```

---

## 5. Configuration

Fabryk configuration uses two traits:

- **`ConfigManager`** — Loading, saving, and resolving config files (TOML + env vars via `confyg`)
- **`ConfigProvider`** — Providing content and cache paths to fabryk builders

### Define your config struct

```rust
use confyg::{env, Confygery};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MyConfig {
    #[serde(default = "default_port")]
    pub port: u16,

    #[serde(default = "default_transport")]
    pub transport: String,

    pub cards_path: PathBuf,
    pub sources_path: PathBuf,
    pub index_path: PathBuf,
    pub vector_db_path: PathBuf,
    pub graph_cache_path: PathBuf,
    pub embedding_model: String,
    pub embedding_cache_dir: Option<PathBuf>,
    pub vector_enabled: bool,

    #[serde(default)]
    pub oauth: OAuthConfig,

    #[serde(default = "default_logging")]
    pub logging: twyg::Opts,
}
```

### Implement `ConfigManager`

This tells fabryk how to load, serialise, and locate your config:

```rust
impl fabryk::core::ConfigManager for MyConfig {
    fn load(config_path: Option<&str>) -> fabryk::core::Result<Self> {
        // Use confyg to load TOML + env var overrides
        let mut builder = Confygery::new()?;
        if let Some(path) = config_path {
            builder.add_file(path)?;
        }
        let mut env_opts = env::Options::with_top_level("MYSERVER");
        env_opts.add_section("oauth");
        builder.add_env(env_opts)?;
        Ok(builder.build()?)
    }

    fn resolve_config_path(explicit: Option<&str>) -> Option<PathBuf> {
        explicit.map(PathBuf::from)
            .or_else(|| dirs::config_dir().map(|d| d.join("myserver/config.toml")))
    }

    fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("myserver/config.toml"))
    }

    fn project_name() -> &'static str { "my-server" }

    fn to_toml_string(&self) -> fabryk::core::Result<String> {
        toml::to_string_pretty(self).map_err(|e| fabryk::core::Error::config(e.to_string()))
    }

    fn to_env_vars(&self) -> fabryk::core::Result<Vec<(String, String)>> {
        Ok(vec![
            ("MYSERVER_PORT".into(), self.port.to_string()),
            ("MYSERVER_TRANSPORT".into(), self.transport.clone()),
            // ... all fields
        ])
    }
}
```

### Implement `ConfigProvider`

This tells fabryk builders where to find content and cache directories:

```rust
impl fabryk::core::ConfigProvider for MyConfig {
    fn project_name(&self) -> &str { "myserver" }

    fn base_path(&self) -> fabryk::core::Result<PathBuf> {
        Ok(self.cards_path.parent().unwrap_or(Path::new(".")).to_path_buf())
    }

    fn content_path(&self, content_type: &str) -> fabryk::core::Result<PathBuf> {
        match content_type {
            "concepts" => Ok(self.cards_path.clone()),
            "sources"  => Ok(self.sources_path.clone()),
            _ => Err(fabryk::core::Error::config(
                format!("Unknown content type: {content_type}")
            )),
        }
    }

    fn cache_path(&self, cache_type: &str) -> fabryk::core::Result<PathBuf> {
        match cache_type {
            "graph"  => Ok(self.graph_cache_path.clone()),
            "fts"    => Ok(self.index_path.clone()),
            "vector" => Ok(self.vector_db_path.clone()),
            _ => Err(fabryk::core::Error::config(
                format!("Unknown cache type: {cache_type}")
            )),
        }
    }
}
```

---

## 6. Domain Types and Content Loading

### Define your frontmatter type

Your content files (markdown with YAML frontmatter) need a matching Rust struct:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CardFrontmatter {
    pub concept: String,
    pub slug: String,
    pub category: String,
    pub subcategory: Option<String>,
    pub tier: KasuTier,
    pub source: String,
    pub source_slug: String,
    pub authors: String,
    pub chapter: Option<String>,
    pub extraction_confidence: ExtractionConfidence,
    pub aliases: Vec<String>,
    pub prerequisites: Vec<String>,
    pub extends: Vec<String>,
    pub related: Vec<String>,
    pub contrasts_with: Vec<String>,
    pub answers_questions: Vec<String>,
    pub harmonisation: Option<HarmonisationBlock>,
}
```

### Load content from disk

```rust
pub fn load_cards(cards_path: &Path) -> Result<Vec<ConceptCard>> {
    let pattern = cards_path.join("*.md");
    let mut cards = Vec::new();

    for path in glob::glob(&pattern.to_string_lossy())?.filter_map(|r| r.ok()) {
        let content = std::fs::read_to_string(&path)?;
        match parse_card(&path, &content) {
            Ok(card) => cards.push(card),
            Err(e) => log::warn!("Skipping card {}: {e}", path.display()),
        }
    }
    Ok(cards)
}

fn parse_card(path: &Path, content: &str) -> Result<ConceptCard> {
    let fm_result = fabryk::content::extract_frontmatter(content)?;
    let body = fm_result.body().to_string();
    let yaml_value = fm_result.into_value().ok_or_else(|| /* error */)?;
    let frontmatter: CardFrontmatter = serde_yaml::from_value(yaml_value)?;
    let quick_definition = fabryk::content::extract_section_content(&body, "Quick Definition");

    Ok(ConceptCard { frontmatter, body, quick_definition })
}
```

**Key fabryk APIs used:**
- `fabryk::content::extract_frontmatter(content)` → `FrontmatterResult`
- `.body()` → markdown body without frontmatter
- `.into_value()` → `Option<serde_yaml::Value>` (consumes the result)
- `.value()` → `Option<&serde_yaml::Value>` (borrows)
- `fabryk::content::extract_section_content(body, heading)` → `Option<String>`

---

## 7. Content Provider

The content provider gives `ContentTools` the ability to list and retrieve your domain items.

### Implement `ContentItemProvider`

```rust
use async_trait::async_trait;
use fabryk_mcp_content::{CategoryInfo, ContentItemProvider};

#[async_trait]
impl ContentItemProvider for InMemoryProvider {
    type ItemSummary = CardSummary;  // lightweight listing type
    type ItemDetail = CardDetail;    // full content type

    async fn list_items(
        &self, category: Option<&str>, limit: Option<usize>,
    ) -> fabryk::core::Result<Vec<Self::ItemSummary>> {
        Ok(self.list_filtered(category, None, limit))
    }

    async fn get_item(&self, id: &str) -> fabryk::core::Result<Self::ItemDetail> {
        let card = self.cards.get(id)
            .ok_or_else(|| fabryk::core::Error::not_found("concept card", id))?;
        Ok(CardDetail::from(card))
    }

    async fn list_categories(&self) -> fabryk::core::Result<Vec<CategoryInfo>> {
        // Count items per category, return sorted list
        // ...
    }

    async fn count(&self) -> fabryk::core::Result<usize> {
        Ok(self.cards.len())
    }

    fn content_type_name(&self) -> &str { "concept" }
    fn content_type_name_plural(&self) -> &str { "concepts" }
}
```

**Important:** `ItemSummary` and `ItemDetail` must implement `Serialize` + `schemars::JsonSchema`. The `schemars` derive ensures MCP tool schemas are generated correctly.

---

## 8. Extractor Adapters

Extractors bridge your domain types to fabryk's generic index builders. You implement three traits:

### 8.1 `GraphExtractor`

Tells the `GraphBuilder` how to extract nodes and edges from your content:

```rust
use fabryk::graph::extractor::GraphExtractor;
use fabryk::graph::types::{Edge, Node, Relationship};

pub struct MyGraphExtractor;

impl GraphExtractor for MyGraphExtractor {
    type NodeData = MyNodeData;
    type EdgeData = MyEdgeData;

    fn extract_node(
        &self, _base_path: &Path, file_path: &Path,
        frontmatter: &serde_yaml::Value, _content: &str,
    ) -> fabryk::core::Result<Self::NodeData> {
        let fm: CardFrontmatter = serde_yaml::from_value(frontmatter.clone())?;
        Ok(MyNodeData { slug: fm.slug, concept: fm.concept, /* ... */ })
    }

    fn extract_edges(
        &self, frontmatter: &serde_yaml::Value, _content: &str,
    ) -> fabryk::core::Result<Option<Self::EdgeData>> {
        let fm: CardFrontmatter = serde_yaml::from_value(frontmatter.clone())?;
        // Return None if no edges exist
        Ok(Some(MyEdgeData {
            prerequisites: fm.prerequisites,
            related: fm.related,
            // ...
        }))
    }

    fn to_graph_node(&self, data: &Self::NodeData) -> Node {
        Node::new(&data.slug, &data.concept)
            .with_category(&data.category)
            .with_metadata("tier", serde_json::Value::String(data.tier.clone()))
    }

    fn to_graph_edges(&self, from_id: &str, data: &Self::EdgeData) -> Vec<Edge> {
        let mut edges = Vec::new();
        for target in &data.prerequisites {
            edges.push(Edge::new(from_id, target, Relationship::Prerequisite));
        }
        for target in &data.related {
            edges.push(Edge::new(from_id, target, Relationship::RelatesTo));
        }
        edges
    }

    fn name(&self) -> &str { "my-graph" }
}
```

### 8.2 `DocumentExtractor` (FTS)

Tells the `IndexBuilder` how to extract search documents:

```rust
use fabryk::fts::{DocumentExtractor, SearchDocument};

pub struct MyDocumentExtractor;

impl DocumentExtractor for MyDocumentExtractor {
    fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
        let fm_result = fabryk::content::extract_frontmatter(content).ok()?;
        let fm: CardFrontmatter = fm_result.deserialize().ok()??;
        let body = fm_result.body();

        Some(
            SearchDocument::builder()
                .id(&fm.slug)
                .path(path.to_string_lossy())
                .title(&fm.concept)
                .content(body)
                .category(&fm.category)
                .source(fm.source_slug)
                .tags(fm.aliases)
                .content_type("concept_card")
                .build(),
        )
    }

    fn supported_extensions(&self) -> &[&str] { &["md"] }
}
```

### 8.3 `VectorExtractor`

Tells the `VectorIndexBuilder` how to extract documents for embedding:

```rust
use fabryk::vector::extractor::VectorExtractor;
use fabryk::vector::types::VectorDocument;

pub struct MyVectorExtractor;

impl VectorExtractor for MyVectorExtractor {
    fn extract_document(
        &self, _base_path: &Path, file_path: &Path,
        frontmatter: &serde_yaml::Value, content: &str,
    ) -> fabryk::core::Result<VectorDocument> {
        let fm: CardFrontmatter = serde_yaml::from_value(frontmatter.clone())?;

        // Build embedding text — combine high-signal fields
        let text = format!(
            "{} | {} | {} | {}",
            fm.concept, fm.category,
            fm.aliases.join(", "),
            &content[..content.len().min(500)],
        );

        Ok(VectorDocument::new(&fm.slug, text)
            .with_category(&fm.category)
            .with_metadata("tier", fm.tier.as_str()))
    }

    fn name(&self) -> &str { "my-vector" }
}
```

**Tip:** The embedding text quality directly affects semantic search relevance. Combine your most discriminative fields (title, category, aliases, quick definition) rather than dumping the entire body.

---

## 9. Error Handling

Define a domain error enum with `thiserror`, and implement `From<fabryk::core::Error>`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Card not found: {slug}")]
    CardNotFound { slug: String },

    #[error("Card parse error [{slug}]: {message}")]
    CardParse { slug: String, message: String },

    #[error("Search error: {0}")]
    Search(String),

    #[error("Graph error: {0}")]
    Graph(String),

    #[error("Vector error: {0}")]
    Vector(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Auth error: {0}")]
    Auth(String),
}

impl From<fabryk::core::Error> for MyError {
    fn from(e: fabryk::core::Error) -> Self {
        if e.is_not_found() {
            MyError::CardNotFound { slug: e.to_string() }
        } else if e.is_config() {
            MyError::Config(e.to_string())
        } else {
            MyError::Search(e.to_string())
        }
    }
}
```

---

## 10. Registry Composition

This is the heart of a Fabryk MCP server. You compose tool registries into a single composite, with service-aware gating for tools that depend on background builds.

```rust
use fabryk::core::service::ServiceHandle;
use fabryk_mcp::registry::CompositeRegistry;
use fabryk_mcp::{
    DiagnosticTools, DiscoverableRegistry, HealthTools,
    ServiceAwareRegistry, ToolMeta,
};
use fabryk_mcp_content::ContentTools;
use fabryk_mcp_fts::FtsTools;
use fabryk_mcp_graph::GraphTools;
use fabryk_mcp_semantic::SemanticSearchTools;

pub async fn run_server(config: MyConfig) -> anyhow::Result<()> {
    // 1. Load data
    let cards = load_cards(&config.cards_path)?;
    let config = Arc::new(config);

    // 2. Create service handles (one per background-built index)
    let graph_svc = ServiceHandle::new("graph");
    let fts_svc = ServiceHandle::new("fts");
    let vector_svc = ServiceHandle::new("vector");

    // 3. Create always-ready provider
    let provider = InMemoryProvider::new(cards.clone());

    // 4. Create shared backend slots
    let graph_data: Arc<RwLock<GraphData>> =
        Arc::new(RwLock::new(GraphData::new()));

    let search_config = fabryk::fts::SearchConfig {
        backend: "tantivy".into(),
        index_path: Some(config.index_path.to_string_lossy().to_string()),
        content_path: Some(config.cards_path.to_string_lossy().to_string()),
        ..Default::default()
    };
    let fts_arc: Arc<dyn fabryk::fts::SearchBackend> = Arc::from(
        fabryk::fts::create_search_backend(&search_config).await?,
    );

    let vector_slot: Arc<RwLock<Option<Arc<dyn fabryk::vector::VectorBackend>>>> =
        Arc::new(RwLock::new(None));

    // 5. Compose the registry
    let registry = CompositeRegistry::new()
        // Content tools — always available (in-memory provider)
        .add(ContentTools::new(provider).with_prefix("concepts"))
        // FTS tools — gated on FTS service readiness
        .add(ServiceAwareRegistry::new(
            FtsTools::with_shared(fts_arc.clone()),
            vec![fts_svc.clone()],
        ))
        // Graph tools — gated on graph service
        .add(ServiceAwareRegistry::new(
            GraphTools::with_shared(graph_data.clone()),
            vec![graph_svc.clone()],
        ))
        // Semantic search — gated on BOTH fts AND vector services
        .add(ServiceAwareRegistry::new(
            SemanticSearchTools::with_vector_slot(fts_arc.clone(), vector_slot.clone()),
            vec![fts_svc.clone(), vector_svc.clone()],
        ))
        // Diagnostics — always available, reports service status
        .add(DiagnosticTools::new(config.clone()).with_services(vec![
            graph_svc.clone(), fts_svc.clone(), vector_svc.clone(),
        ]));

    // Health tool needs the total count (including itself)
    let total_tool_count = registry.tool_count() + 1;
    let registry = registry.add(HealthTools::new(
        "my-server",
        env!("CARGO_PKG_VERSION"),
        total_tool_count,
    ));

    // 6. Wrap in DiscoverableRegistry
    let discoverable = DiscoverableRegistry::new(registry, "myprefix")
        .with_tool_metas(tool_metadata())
        .with_query_strategy(vec![
            "1. Call myprefix_directory first.",
            "2. Use semantic_search for natural-language questions.",
            "3. Use search for exact keyword lookups.",
            // ...
        ])
        .with_data_freshness({
            let mut f = std::collections::HashMap::new();
            f.insert("concept_cards".into(), "1,400 cards — static".into());
            f
        });

    // ... (server setup continues in Section 13)
}
```

### How `ServiceAwareRegistry` works

- When a tool's backing service is `Starting`, tool calls return a clear error: _"Service 'fts' is starting up..."_
- When `Ready`, calls pass through to the underlying tool implementation.
- When `Failed`, calls return the failure reason.
- Tools gated on **multiple** services (like semantic search needing both FTS and vector) require **all** listed services to be `Ready`.

### How `DiscoverableRegistry` works

Wrapping the final registry in `DiscoverableRegistry::new(registry, "prefix")` auto-injects:
- A `prefix_directory` tool that LLM clients can call to discover all available tools
- Tool metadata (summary, when to use, what it returns, next steps)
- Query strategy guidance for optimal tool sequencing
- Data freshness indicators

---

## 11. Background Builds with Service Gating

The pattern: create `ServiceHandle`s, start the transport immediately, then spawn builds that update handles on completion.

```rust
use fabryk::core::service::{RetryConfig, ServiceHandle, ServiceState};

pub fn spawn_graph_build(
    config: Arc<MyConfig>,
    graph_data: Arc<RwLock<GraphData>>,
    svc: ServiceHandle,
) {
    fabryk::core::spawn_with_retry(svc, RetryConfig::default(), move || {
        let config = config.clone();
        let graph_data = graph_data.clone();
        async move {
            let (graph, stats) = GraphBuilder::new(MyGraphExtractor)
                .with_content_path(&config.cards_path)
                .with_cache_path(&config.graph_cache_path)
                .with_error_handling(fabryk::graph::builder::ErrorHandling::Collect)
                .build()
                .await
                .map_err(|e| format!("Graph build failed: {e}"))?;

            log::info!(
                "Graph: {} nodes, {} edges ({} errors)",
                stats.nodes_created, stats.edges_created, stats.errors.len(),
            );

            let mut lock = graph_data.write().await;
            *lock = graph;
            Ok(())
        }
    });
}

pub fn spawn_fts_build(config: Arc<MyConfig>, svc: ServiceHandle) {
    fabryk::core::spawn_with_retry(svc, RetryConfig::default(), move || {
        let config = config.clone();
        async move {
            fabryk::fts::IndexBuilder::new()
                .with_extractor(Box::new(MyDocumentExtractor))
                .build(&config.cards_path, &config.index_path)
                .await
                .map_err(|e| format!("FTS build failed: {e}"))?;
            Ok(())
        }
    });
}

pub fn spawn_vector_build(
    config: Arc<MyConfig>,
    vector_slot: Arc<RwLock<Option<Arc<dyn VectorBackend>>>>,
    svc: ServiceHandle,
) {
    if !config.vector_enabled {
        svc.set_state(ServiceState::Ready);
        return;
    }

    fabryk::core::spawn_with_retry(svc, RetryConfig::default(), move || {
        let config = config.clone();
        let vector_slot = vector_slot.clone();
        async move {
            let embedding_provider: Arc<dyn fabryk::vector::EmbeddingProvider> =
                Arc::new(fabryk::vector::FastEmbedProvider::new(
                    &config.embedding_model,
                    config.embedding_cache_dir.as_deref().and_then(|p| p.to_str()),
                )?);

            let (backend, stats) = fabryk::vector::VectorIndexBuilder::new(MyVectorExtractor)
                .with_content_path(&config.cards_path)
                .with_cache_path(&config.vector_db_path)
                .with_embedding_provider(embedding_provider)
                .with_error_handling(fabryk::vector::builder::ErrorHandling::Collect)
                .build()
                .await
                .map_err(|e| format!("Vector build failed: {e}"))?;

            let mut slot = vector_slot.write().await;
            *slot = Some(Arc::from(Box::new(backend) as Box<dyn VectorBackend>));
            Ok(())
        }
    });
}
```

### `spawn_with_retry` behaviour

- Takes ownership of the `ServiceHandle` and manages its state transitions
- On success: sets state to `Ready`
- On failure after all retries: sets state to `Failed(reason)`
- `RetryConfig::default()` provides 3 retries with exponential backoff (1s, 2s, 4s)
- The closure must return `Result<(), String>`

### The vector slot pattern

Vector search uses a `Arc<RwLock<Option<Arc<dyn VectorBackend>>>>` ("slot") because:

1. `SemanticSearchTools::with_vector_slot()` needs a reference at registry construction time
2. The actual backend isn't available until the background build completes
3. The `RwLock<Option<...>>` lets the build populate it asynchronously

---

## 12. Tool Metadata and Discoverability

Tool metadata helps LLM clients understand what each tool does and how to sequence calls:

```rust
fn tool_metadata() -> Vec<(String, ToolMeta)> {
    vec![
        (
            "concepts_list".into(),
            ToolMeta {
                summary: "List concept cards with optional category/tier filtering.".into(),
                when_to_use: "Browsing available cards, filtering by category or tier.".into(),
                returns: "Array of card summaries (slug, title, category, tier).".into(),
                next: Some("Call concepts_get with a slug to read the full card.".into()),
                category: Some("content".into()),
            },
        ),
        (
            "search".into(),
            ToolMeta {
                summary: "Full-text keyword search across all content.".into(),
                when_to_use: "Looking for exact terms, class names, annotations.".into(),
                returns: "Ranked results with highlighted snippets.".into(),
                next: Some("Call concepts_get with a result slug for full content.".into()),
                category: Some("search".into()),
            },
        ),
        // ... metadata for every registered tool
    ]
}
```

Each `ToolMeta` has:
| Field | Purpose |
|---|---|
| `summary` | One-line description (shown in directory listing) |
| `when_to_use` | Guidance for LLMs on when to pick this tool |
| `returns` | What the tool response contains |
| `next` | Suggested follow-up tool call |
| `category` | Grouping (content, search, graph, diagnostics) |

---

## 13. Server and Transport

### Create `FabrykMcpServer`

```rust
let server = FabrykMcpServer::new(discoverable)
    .with_name("my-server")
    .with_version(env!("CARGO_PKG_VERSION"))
    .with_discoverable_instructions("myprefix")
    .with_services(vec![graph_svc.clone(), fts_svc.clone(), vector_svc.clone()]);
```

`.with_discoverable_instructions("myprefix")` auto-generates server instructions that tell LLM clients to call `myprefix_directory` first.

### Start transport

```rust
// Spawn background builds first (they run independently)
spawn_graph_build(config.clone(), graph_data.clone(), graph_svc.clone());
spawn_fts_build(config.clone(), fts_svc.clone());
spawn_vector_build(config.clone(), vector_slot.clone(), vector_svc.clone());

// Start transport immediately
match config.transport.as_str() {
    "stdio" => {
        server.serve_stdio().await?;
    }
    _ => {
        run_http(config, server, vec![graph_svc, fts_svc, vector_svc]).await?;
    }
}
```

### Entry point (`main.rs`)

Keep `main.rs` minimal — route CLI commands or start the server:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config_str().map(String::from);

    match cli.command {
        Some(Command::Config { action }) => {
            handle_config_command(config_path.as_deref(), action)
        }
        Some(Command::Vectordb { action }) => {
            let config = MyConfig::load(cli.config.as_deref())?;
            handle_vectordb_command(action, &config)
        }
        None => {
            let mut config = MyConfig::load(cli.config.as_deref())?;
            config.apply_cli(&cli);
            run_server(config).await
        }
    }
}
```

---

## 14. HTTP Transport with OAuth2

```rust
async fn run_http(
    config: Arc<MyConfig>,
    server: FabrykMcpServer,
    services: Vec<ServiceHandle>,
) -> anyhow::Result<()> {
    use fabryk::auth::{AuthConfig, AuthLayer};
    use fabryk_auth_google::GoogleTokenValidator;
    use rmcp::transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpService,
    };

    // Create the MCP HTTP service
    let mcp_service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Build discovery router (health + OAuth .well-known endpoints)
    let resource_url = std::env::var("MYSERVER_RESOURCE_URL").unwrap_or_default();
    let discovery_router = axum::Router::new()
        .merge(fabryk_mcp::health_router(services))
        .merge(fabryk_mcp_auth::discovery_routes(
            &resource_url,
            "https://accounts.google.com",
        ));

    // Apply auth to /mcp only when enabled; health+discovery stay public
    let router = if config.oauth.enabled {
        let validator = Arc::new(GoogleTokenValidator::new(config.oauth.jwks_url.clone()));
        let auth_config = AuthConfig {
            enabled: true,
            audience: config.oauth.client_id.clone(),
            domain: config.oauth.domain.clone(),
        };
        let auth_layer = AuthLayer::new(validator, auth_config);
        discovery_router.nest_service(
            "/mcp",
            tower::ServiceBuilder::new().layer(auth_layer).service(mcp_service),
        )
    } else {
        discovery_router.nest_service("/mcp", mcp_service)
    };

    let listener = tokio::net::TcpListener::bind(
        format!("0.0.0.0:{}", config.port)
    ).await?;

    axum::serve(listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;

    Ok(())
}
```

### Route layout

| Route | Handler | Auth |
|---|---|---|
| `/mcp` | MCP StreamableHttpService | OAuth2 (when enabled) |
| `/health` | `fabryk_mcp::health_router` | Public |
| `/.well-known/oauth-protected-resource` | RFC 9728 discovery | Public |
| `/.well-known/oauth-authorization-server` | RFC 8414 discovery | Public |
| `/.well-known/oauth-protected-resource/mcp` | MCP-specific RFC 9728 | Public |
| `/.well-known/oauth-authorization-server/mcp` | MCP-specific RFC 8414 | Public |

---

## 15. CLI Integration

Fabryk provides shared CLI handlers via `fabryk-cli`. Wire them into your Clap subcommands:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "my-server", version, about)]
pub struct Cli {
    #[arg(short, long, env = "MYSERVER_CONFIG")]
    pub config: Option<PathBuf>,

    #[arg(short, long, env = "MYSERVER_TRANSPORT")]
    pub transport: Option<String>,

    #[arg(short, long, env = "MYSERVER_PORT")]
    pub port: Option<u16>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Config {
        #[command(subcommand)]
        action: Option<ConfigAction>,
    },
    Vectordb {
        #[command(subcommand)]
        action: fabryk_cli::VectordbAction,
    },
}
```

### Config command handlers

```rust
pub fn handle_config_command(config_path: Option<&str>, action: Option<ConfigAction>) -> Result<()> {
    match action {
        None => fabryk_cli::config_handlers::cmd_config_path::<MyConfig>(config_path),
        Some(ConfigAction::Get { key: None }) => {
            let config = MyConfig::load(config_path)?;
            print!("{}", config.to_toml_string()?);
            Ok(())
        }
        Some(ConfigAction::Get { key: Some(key) }) => {
            fabryk_cli::config_handlers::cmd_config_get::<MyConfig>(config_path, &key)
        }
        Some(ConfigAction::Set { key, value }) => {
            fabryk_cli::config_handlers::cmd_config_set::<MyConfig>(config_path, &key, &value)
        }
        Some(ConfigAction::Init { file, force }) => {
            fabryk_cli::config_handlers::cmd_config_init::<MyConfig>(
                file.as_ref().and_then(|p| p.to_str()), force,
            )
        }
    }
}
```

### Vectordb command handler

```rust
pub fn handle_vectordb_command(action: VectordbAction, config: &MyConfig) -> Result<()> {
    match action {
        VectordbAction::GetModel { model, cache_dir } => {
            let model_name = model.as_deref().unwrap_or(&config.embedding_model);
            let cache = cache_dir.as_deref()
                .or(config.embedding_cache_dir.as_ref().and_then(|p| p.to_str()));
            fabryk_cli::vectordb_handlers::cmd_vectordb_get_model(model_name, cache)
        }
    }
}
```

This gives you `my-server config get`, `my-server config set port 8080`, `my-server config init`, and `my-server vectordb get-model` for free.

---

## 16. Testing

### MCP schema validation

Validate that all tool schemas are spec-compliant (catches empty `inputSchema` that silently breaks Claude Desktop):

```rust
#[tokio::test]
async fn test_all_tools_have_valid_mcp_schemas() {
    use fabryk_mcp::validate::assert_tools_valid;

    // Build a minimal registry with empty backends
    let provider = InMemoryProvider::new(vec![]);
    let tmp = tempfile::TempDir::new().unwrap();
    let fts_config = fabryk::fts::SearchConfig {
        index_path: Some(tmp.path().join("fts").to_string_lossy().to_string()),
        ..Default::default()
    };
    let fts: Arc<dyn fabryk::fts::SearchBackend> = Arc::from(
        fabryk::fts::create_search_backend(&fts_config).await.unwrap(),
    );
    let graph_data = Arc::new(RwLock::new(GraphData::new()));
    let vector_slot = Arc::new(RwLock::new(None));
    let config = Arc::new(MyConfig::default());

    let services = vec![
        ServiceHandle::new("graph"),
        ServiceHandle::new("fts"),
        ServiceHandle::new("vector"),
    ];

    let registry = CompositeRegistry::new()
        .add(ContentTools::new(provider).with_prefix("concepts"))
        .add(ServiceAwareRegistry::new(
            FtsTools::with_shared(fts.clone()), vec![services[1].clone()],
        ))
        .add(ServiceAwareRegistry::new(
            GraphTools::with_shared(graph_data), vec![services[0].clone()],
        ))
        .add(ServiceAwareRegistry::new(
            SemanticSearchTools::with_vector_slot(fts, vector_slot),
            vec![services[1].clone(), services[2].clone()],
        ))
        .add(DiagnosticTools::new(config).with_services(services.clone()));

    let total = registry.tool_count() + 1;
    let registry = registry.add(HealthTools::new("test", "0.0.0", total));

    let discoverable = DiscoverableRegistry::new(registry, "myprefix")
        .with_tool_metas(tool_metadata());

    assert_tools_valid(&discoverable);
}
```

### Health endpoint tests

```rust
#[tokio::test]
async fn test_health_starting() {
    let services = vec![
        ServiceHandle::new("graph"),
        ServiceHandle::new("fts"),
        ServiceHandle::new("vector"),
    ];
    for svc in &services {
        svc.set_state(ServiceState::Starting);
    }
    let router = build_discovery_router(services, String::new());

    let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
    let resp = router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let json: serde_json::Value = /* parse body */;
    assert_eq!(json["status"], "starting");
}

#[tokio::test]
async fn test_health_ready() {
    let services = /* ... */;
    for svc in &services {
        svc.set_state(ServiceState::Ready);
    }
    // ...
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json["status"], "ok");
}
```

### OAuth discovery tests

```rust
#[tokio::test]
async fn test_oauth_protected_resource() {
    let router = build_discovery_router(vec![], "https://example.com".into());
    let req = Request::builder()
        .uri("/.well-known/oauth-protected-resource")
        .body(Body::empty()).unwrap();
    let resp = router.oneshot(req).await.unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json: serde_json::Value = /* ... */;
    assert_eq!(json["resource"], "https://example.com");
}
```

---

## 17. Refactoring an Existing MCP Server to Fabryk

If you have an existing MCP server (e.g., using rmcp directly), here's a phased migration plan.

### Phase 1: Add Dependencies, Keep Existing Code

1. Add `fabryk`, `fabryk-mcp`, and `fabryk-cli` to `Cargo.toml`
2. Create `config.rs` implementing `ConfigManager` and `ConfigProvider`
3. Add `error.rs` with `From<fabryk::core::Error>`
4. Verify compilation — nothing should break yet

### Phase 2: Extract Adapters

1. Create `adapters.rs` with extractor implementations
2. Implement `DocumentExtractor` for FTS (maps your content to `SearchDocument`)
3. Implement `GraphExtractor` if you have relationships
4. Implement `VectorExtractor` if you want semantic search
5. Test extractors in isolation

### Phase 3: Replace Index Builders

1. Replace custom FTS indexing with `fabryk::fts::IndexBuilder`
2. Replace custom graph building with `fabryk::graph::builder::GraphBuilder`
3. Replace custom vector indexing with `fabryk::vector::VectorIndexBuilder`
4. At this point, index builds work via fabryk but you still serve tools your old way

### Phase 4: Switch to Registry Composition

This is the biggest change. Replace your custom tool registration with:

1. Create `InMemoryProvider` implementing `ContentItemProvider`
2. Replace custom tool registration with `CompositeRegistry` composition
3. Add `ServiceHandle`s and `ServiceAwareRegistry` wrappers
4. Add `DiagnosticTools` and `HealthTools`
5. Wrap in `DiscoverableRegistry`

**Before** (typical rmcp-only server):
```rust
// Custom tool definitions scattered across the codebase
let tools = vec![
    search_tool(),
    get_card_tool(),
    list_tool(),
    // ... each with manual schema + handler
];
```

**After** (Fabryk):
```rust
let registry = CompositeRegistry::new()
    .add(ContentTools::new(provider).with_prefix("concepts"))
    .add(ServiceAwareRegistry::new(FtsTools::with_shared(fts), vec![fts_svc]))
    .add(ServiceAwareRegistry::new(GraphTools::with_shared(graph), vec![graph_svc]))
    .add(DiagnosticTools::new(config).with_services(all_services))
    .add(HealthTools::new("server", version, count));
```

### Phase 5: Background Builds

1. Replace synchronous builds (blocking startup) with `spawn_with_retry`
2. Move from `build-then-serve` to `serve-then-build`
3. Users see immediate server availability

### Phase 6: Add Tool Metadata

1. Write `tool_metadata()` returning `Vec<(String, ToolMeta)>`
2. Wrap registry in `DiscoverableRegistry`
3. Add query strategy and data freshness metadata
4. The auto-generated `_directory` tool now guides LLM clients

### Phase 7: CLI and Config

1. Switch to `fabryk-cli` config handlers
2. Add `vectordb get-model` subcommand
3. Add `config get/set/init` subcommands

### Migration Checklist

- [ ] `Cargo.toml` updated with fabryk dependencies
- [ ] `ConfigManager` implemented
- [ ] `ConfigProvider` implemented
- [ ] Error type converts from `fabryk::core::Error`
- [ ] At least one `DocumentExtractor` implemented
- [ ] `GraphExtractor` implemented (if applicable)
- [ ] `VectorExtractor` implemented (if applicable)
- [ ] `ContentItemProvider` implemented
- [ ] `CompositeRegistry` composed with service gating
- [ ] `DiscoverableRegistry` wrapping with tool metadata
- [ ] `FabrykMcpServer` created and configured
- [ ] Background builds with `spawn_with_retry`
- [ ] `assert_tools_valid` test passing
- [ ] Health endpoint tested
- [ ] CLI config/vectordb commands wired

---

## 18. Quick Start Checklist

Minimum steps to get a working Fabryk MCP server:

1. **`Cargo.toml`** — Add `fabryk`, `fabryk-mcp`, `rmcp`, `tokio`, `serde`, `clap`
2. **`config.rs`** — Define config struct, implement `ConfigManager` + `ConfigProvider`
3. **`error.rs`** — Domain error enum with `From<fabryk::core::Error>`
4. **`types.rs`** — Domain types (frontmatter struct, content struct)
5. **`adapters.rs`** — Implement `DocumentExtractor` (at minimum)
6. **`provider.rs`** — Implement `ContentItemProvider`
7. **`lib.rs`** — `run_server()`: load data → create handles → compose registry → start transport
8. **`main.rs`** — CLI routing → `run_server()`
9. **Test** — `assert_tools_valid` + health endpoint tests

For a minimal server (FTS + content tools only, stdio transport, no graph or vector):

```rust
let registry = CompositeRegistry::new()
    .add(ContentTools::new(provider).with_prefix("items"))
    .add(ServiceAwareRegistry::new(
        FtsTools::with_shared(fts_arc),
        vec![fts_svc.clone()],
    ))
    .add(HealthTools::new("my-server", "0.1.0", 5));

let server = FabrykMcpServer::new(registry)
    .with_name("my-server")
    .with_version("0.1.0");

spawn_fts_build(config, fts_svc);
server.serve_stdio().await?;
```

That's approximately 200 lines of actual code (excluding types and tests) for a fully functional MCP server with progressive tool availability, background indexing, and health monitoring.

---

*Based on ai-kasu v0.2.0 — the Java/Quarkus knowledge MCP server at `crates/kasu-server/`.*
