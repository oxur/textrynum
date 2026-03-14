# Building a Knowledge Fabric MCP Server with Fabryk

A comprehensive guide to building an MCP (Model Context Protocol) server that
exposes a knowledge corpus through full-text search, knowledge graphs, semantic
vector search, and structured content access — using the Fabryk crate
ecosystem.

**Audience:** Rust developers building MCP servers for AI-assisted knowledge
exploration. Assumes familiarity with Rust async patterns and Cargo workspaces.

**What you'll build:** An MCP server that:

- Loads a markdown corpus with YAML frontmatter
- Builds a full-text search index (Tantivy)
- Constructs a knowledge graph with relationship extraction
- Creates a vector index for semantic search (LanceDB + FastEmbed)
- Exposes all of these as MCP tools for Claude (or any MCP client)
- Supports both stdio and HTTP transport with optional OAuth2
- Reports health, provides tool discovery, and handles graceful startup

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Project Setup](#2-project-setup)
3. [Configuration](#3-configuration)
4. [Domain Types & Content Loading](#4-domain-types--content-loading)
5. [Content Provider](#5-content-provider)
6. [Full-Text Search](#6-full-text-search)
7. [Knowledge Graph](#7-knowledge-graph)
8. [Vector & Semantic Search](#8-vector--semantic-search)
9. [MCP Server Core](#9-mcp-server-core)
10. [Tool Composition & Registries](#10-tool-composition--registries)
11. [Service Lifecycle & Health](#11-service-lifecycle--health)
12. [Tool Metadata & Discoverability](#12-tool-metadata--discoverability)
13. [Authentication & HTTP Transport](#13-authentication--http-transport)
14. [CLI Integration](#14-cli-integration)
15. [Testing](#15-testing)
16. [Deployment](#16-deployment)
17. [Quick Start Checklist](#17-quick-start-checklist)
18. [Pattern Reference](#18-pattern-reference)

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
│  │  fabryk-mcp-core: FabrykMcpServer, CompositeRegistry,│  │
│  │  ServiceAwareRegistry, DiscoverableRegistry, Health  │  │
│  └──────────────────────────────────────────────────────┘  │
├────────────────────────────────────────────────────────────┤
│  Domain Layer (fabryk)                                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐       │
│  │ Content  │ │   FTS    │ │  Graph   │ │  Vector  │       │
│  │ (md/yaml)│ │(tantivy) │ │(petgraph)│ │(lancedb) │       │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘       │
├────────────────────────────────────────────────────────────┤
│  Core Layer: fabryk-core (ServiceHandle, ConfigManager,    │
│  Error, AppState, PathResolver, spawn_with_retry)          │
├────────────────────────────────────────────────────────────┤
│  Infrastructure: fabryk-auth, fabryk-redis, fabryk-gcp     │
└────────────────────────────────────────────────────────────┘
```

**Key design principles:**

- **Trait-driven extensibility** — Your domain implements 3-4 traits
  (`GraphExtractor`, `VectorExtractor`, `ContentItemProvider`, optionally
  `ConfigManager`). Fabryk handles the rest.
- **Feature-gated heavy dependencies** — Tantivy, LanceDB, FastEmbed are
  opt-in. Lightweight fallbacks (SimpleSearch, SimpleVectorBackend) always
  available.
- **Service lifecycle management** — Background index building with
  `spawn_with_retry`, progressive tool availability via
  `ServiceAwareRegistry`.
- **Composable registries** — Mix and match tool groups. Each group can be
  independently gated on service readiness.

---

## 2. Project Setup

### Cargo.toml

```toml
[package]
name = "my-knowledge-server"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[dependencies]
# Fabryk domain layer (umbrella — picks up core, content, fts, graph, vector)
fabryk = { version = "0.3", features = [
    "fts-tantivy",       # Full-text search with Tantivy
    "graph-rkyv-cache",  # Graph persistence with rkyv + blake3
    "vector-lancedb",    # Vector search with LanceDB
    "vector-fastembed",  # Local embedding generation
] }

# Fabryk MCP layer (umbrella — picks up mcp-core + all tool crates)
fabryk-mcp = { version = "0.3", features = [
    "http",              # HTTP transport (axum-based)
    "fts-tantivy",       # Pass-through to fabryk-fts
    "graph-rkyv-cache",  # Pass-through to fabryk-graph
] }

# Auth (optional — only for HTTP transport with OAuth2)
fabryk-auth = "0.3"
fabryk-auth-google = "0.3"
fabryk-mcp-auth = "0.3"

# CLI framework (optional — for config/graph/vectordb commands)
fabryk-cli = { version = "0.3", features = ["vector-fastembed"] }

# MCP SDK
rmcp = { version = "1.1", features = ["server", "transport-io"] }

# Standard dependencies
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs", "sync", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
clap = { version = "4", features = ["derive", "env"] }
confyg = "0.1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
thiserror = "2"
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
schemars = "1.2"

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

### Feature Selection Guide

| Scenario | Features |
|----------|----------|
| **Minimal** (content + simple search) | None (defaults work) |
| **Full-text search** | `fts-tantivy` |
| **Knowledge graph** | `graph-rkyv-cache` |
| **Semantic search** | `vector-lancedb`, `vector-fastembed` |
| **Full stack** | All of the above (`full` shorthand) |
| **HTTP transport** | `http` on fabryk-mcp |
| **CLI tools** | `vector-fastembed` on fabryk-cli |

### Project Structure

```
my-knowledge-server/
├── Cargo.toml
├── config.toml              # Application configuration
├── src/
│   ├── main.rs              # Entry point, CLI parsing, server startup
│   ├── config.rs            # ConfigManager + ConfigProvider impl
│   ├── types.rs             # Domain types (deserialized from frontmatter)
│   ├── content.rs           # ContentItemProvider implementation
│   ├── extractors.rs        # GraphExtractor + VectorExtractor implementations
│   ├── error.rs             # Domain error types
│   ├── server.rs            # MCP server composition
│   └── tools/
│       └── mod.rs           # Custom domain-specific MCP tools (if any)
├── content/                 # Your markdown corpus
│   ├── concepts/
│   │   ├── topic-a.md
│   │   └── topic-b.md
│   └── guides/
│       └── getting-started.md
└── cache/                   # Generated indices (gitignored)
    ├── fts/                 # Tantivy index
    ├── graph/               # rkyv-cached graph
    └── vector/              # LanceDB vector index
```

---

## 3. Configuration

Fabryk provides two configuration traits. Implement both for your project.

### ConfigManager — Application Configuration

`ConfigManager` handles loading, saving, and serializing your config:

```rust
use fabryk::core::ConfigManager;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_name: String,
    pub content_path: PathBuf,
    pub cache_path: PathBuf,
    pub port: u16,

    #[serde(default)]
    pub oauth: Option<OAuthConfig>,

    #[serde(default)]
    pub redis: Option<RedisConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub client_id: String,
    pub allowed_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,
}

impl ConfigManager for Config {
    fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let config: Self = confyg::load_from_file(path)?;
        Ok(config)
    }

    fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let toml_str = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_str)?;
        Ok(())
    }

    fn resolve_paths(&mut self, base: &Path) {
        if self.content_path.is_relative() {
            self.content_path = base.join(&self.content_path);
        }
        if self.cache_path.is_relative() {
            self.cache_path = base.join(&self.cache_path);
        }
    }

    fn to_toml(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(toml::to_string_pretty(self)?)
    }

    fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("PROJECT_NAME".into(), self.project_name.clone()),
            ("CONTENT_PATH".into(), self.content_path.display().to_string()),
            ("CACHE_PATH".into(), self.cache_path.display().to_string()),
            ("PORT".into(), self.port.to_string()),
        ]
    }
}
```

### ConfigProvider — Bridge to Fabryk Builders

`ConfigProvider` tells Fabryk builders where to find content and caches:

```rust
use fabryk::core::ConfigProvider;

impl ConfigProvider for Config {
    fn content_path(&self) -> &Path {
        &self.content_path
    }

    fn cache_path(&self, cache_type: &str) -> PathBuf {
        self.cache_path.join(cache_type)
    }

    fn project_name(&self) -> &str {
        &self.project_name
    }
}
```

### Example config.toml

```toml
project_name = "my-knowledge-base"
content_path = "./content"
cache_path = "./cache"
port = 8080

[oauth]
client_id = "123456.apps.googleusercontent.com"
allowed_domain = "mycompany.com"

[redis]
url = "redis://localhost:6379"
```

---

## 4. Domain Types & Content Loading

Your markdown files use YAML frontmatter to carry structured metadata.
Fabryk's content crate parses this generically — you define the domain types.

### Frontmatter Schema

```yaml
---
title: "Functional Harmony"
category: "music-theory"
tags: ["harmony", "chords", "progressions"]
prerequisites: ["intervals", "scales"]
related: ["voice-leading", "chord-substitution"]
summary: "How chords function within a key..."
---

# Functional Harmony

Content goes here...
```

### Domain Types

```rust
use serde::{Deserialize, Serialize};

/// Metadata extracted from YAML frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub title: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

/// A loaded content item with parsed metadata and raw content.
#[derive(Debug, Clone)]
pub struct ContentItem {
    pub id: String,
    pub path: std::path::PathBuf,
    pub metadata: ItemMetadata,
    pub content: String,
}
```

### Loading Content

```rust
use fabryk::content::{extract_frontmatter, extract_text_content};
use fabryk::core::PathResolver;

pub fn load_items(content_path: &Path) -> Result<Vec<ContentItem>, Error> {
    let resolver = PathResolver::new(content_path);
    let mut items = Vec::new();

    for entry in std::fs::read_dir(content_path.join("concepts"))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "md") {
            let raw = std::fs::read_to_string(&path)?;
            let frontmatter = extract_frontmatter(&raw)?;
            let metadata: ItemMetadata = serde_yaml::from_value(frontmatter.data)?;
            let id = fabryk::core::id_from_path(&path, content_path);

            items.push(ContentItem {
                id,
                path,
                metadata,
                content: raw,
            });
        }
    }

    Ok(items)
}
```

---

## 5. Content Provider

The `ContentItemProvider` trait connects your domain types to Fabryk's
MCP content tools. Implement it and `ContentTools` automatically generates
`{prefix}_list`, `{prefix}_get`, and `{prefix}_categories` MCP tools.

```rust
use async_trait::async_trait;
use fabryk_mcp::content::{ContentItemProvider, CategoryInfo, ListItemsArgs, GetItemArgs};
use fabryk::core::Error;

/// Summary returned by list operations.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ItemSummary {
    pub id: String,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub summary: String,
}

/// Full detail returned by get operations.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ItemDetail {
    pub id: String,
    pub title: String,
    pub category: String,
    pub tags: Vec<String>,
    pub summary: String,
    pub content: String,
    pub prerequisites: Vec<String>,
    pub related: Vec<String>,
}

pub struct MyContentProvider {
    items: Vec<ContentItem>,
}

#[async_trait]
impl ContentItemProvider for MyContentProvider {
    type ItemSummary = ItemSummary;
    type ItemDetail = ItemDetail;

    async fn list_items(&self, args: &ListItemsArgs) -> Result<Vec<ItemSummary>, Error> {
        let mut results: Vec<_> = self.items.iter()
            .filter(|item| {
                args.category.as_ref()
                    .map_or(true, |cat| item.metadata.category == *cat)
            })
            .map(|item| ItemSummary {
                id: item.id.clone(),
                title: item.metadata.title.clone(),
                category: item.metadata.category.clone(),
                tags: item.metadata.tags.clone(),
                summary: item.metadata.summary.clone(),
            })
            .collect();

        if let Some(limit) = args.limit {
            results.truncate(limit);
        }
        Ok(results)
    }

    async fn get_item(&self, args: &GetItemArgs) -> Result<Option<ItemDetail>, Error> {
        Ok(self.items.iter()
            .find(|item| item.id == args.id)
            .map(|item| ItemDetail {
                id: item.id.clone(),
                title: item.metadata.title.clone(),
                category: item.metadata.category.clone(),
                tags: item.metadata.tags.clone(),
                summary: item.metadata.summary.clone(),
                content: item.content.clone(),
                prerequisites: item.metadata.prerequisites.clone(),
                related: item.metadata.related.clone(),
            }))
    }

    async fn categories(&self) -> Result<Vec<CategoryInfo>, Error> {
        let mut cats = std::collections::BTreeMap::new();
        for item in &self.items {
            *cats.entry(item.metadata.category.clone()).or_insert(0usize) += 1;
        }
        Ok(cats.into_iter()
            .map(|(name, count)| CategoryInfo { name, count })
            .collect())
    }

    fn item_count(&self) -> usize {
        self.items.len()
    }
}
```

### Register with MCP

```rust
use fabryk_mcp::content::ContentTools;

let content_tools = ContentTools::new(Arc::new(provider))
    .with_prefix("concepts");
// Generates: concepts_list, concepts_get, concepts_categories
```

---

## 6. Full-Text Search

Fabryk's FTS layer wraps Tantivy with a domain-friendly API. You describe
how to extract searchable fields from your content; Fabryk handles indexing
and querying.

### Building the Index

```rust
use fabryk::fts::{IndexBuilder, DocumentExtractor, SearchDocument, SearchSchema, SearchConfig};

/// Tell Fabryk how to turn your items into search documents.
struct MyDocumentExtractor;

impl DocumentExtractor for MyDocumentExtractor {
    fn extract(&self, path: &Path, content: &str) -> Option<SearchDocument> {
        let frontmatter = fabryk::content::extract_frontmatter(content).ok()?;
        let meta: ItemMetadata = serde_yaml::from_value(frontmatter.data).ok()?;
        let text = fabryk::content::extract_text_content(content);

        Some(SearchDocument {
            id: fabryk::core::id_from_path(path, &self.base_path),
            path: path.display().to_string(),
            title: meta.title,
            description: meta.summary,
            content: text,
            category: meta.category,
            tags: meta.tags,
            ..Default::default()
        })
    }
}

// Build the index
let config = SearchConfig {
    index_path: config.cache_path("fts"),
    ..Default::default()
};

let builder = IndexBuilder::new(&config, MyDocumentExtractor)?;
let stats = builder.build_from_directory(&config.content_path).await?;
tracing::info!("Indexed {} documents", stats.documents_indexed);
```

### Index Freshness

Before rebuilding, check if the index is still fresh:

```rust
use fabryk::fts::is_index_fresh;

if !is_index_fresh(&config.cache_path("fts"), &config.content_path).await? {
    tracing::info!("Content changed, rebuilding FTS index...");
    builder.build_from_directory(&config.content_path).await?;
}
```

### Querying

```rust
use fabryk::fts::{create_search_backend, SearchParams};

let backend = create_search_backend(&config).await?;
let results = backend.search(SearchParams {
    query: "functional harmony".into(),
    limit: Some(10),
    category: Some("music-theory".into()),
    ..Default::default()
}).await?;

for result in &results.results {
    println!("{}: {} (score: {:.2})", result.id, result.title, result.score);
}
```

### Register with MCP

```rust
use fabryk_mcp::fts::FtsTools;

let fts_tools = FtsTools::from_boxed(backend);
// Generates: search, search_status
```

---

## 7. Knowledge Graph

The knowledge graph captures relationships between concepts. You implement
`GraphExtractor` to tell Fabryk how your domain items relate to each other.

### Implementing GraphExtractor

```rust
use fabryk::graph::{GraphExtractor, Node, Edge, NodeType, Relationship, EdgeOrigin};

struct MyGraphExtractor;

impl GraphExtractor for MyGraphExtractor {
    fn extract_nodes(&self, id: &str, content: &str) -> Vec<Node> {
        let frontmatter = fabryk::content::extract_frontmatter(content).ok();
        let meta: Option<ItemMetadata> = frontmatter
            .and_then(|fm| serde_yaml::from_value(fm.data).ok());

        let title = meta.as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| id.to_string());

        vec![Node {
            id: id.to_string(),
            label: title,
            node_type: NodeType::Domain,
            metadata: serde_json::json!({
                "category": meta.as_ref().map(|m| &m.category),
                "tags": meta.as_ref().map(|m| &m.tags),
            }),
        }]
    }

    fn extract_edges(&self, id: &str, content: &str) -> Vec<Edge> {
        let frontmatter = fabryk::content::extract_frontmatter(content).ok();
        let meta: Option<ItemMetadata> = frontmatter
            .and_then(|fm| serde_yaml::from_value(fm.data).ok());

        let mut edges = Vec::new();
        if let Some(meta) = meta {
            // Prerequisites → "requires" relationship
            for prereq in &meta.prerequisites {
                edges.push(Edge {
                    source: id.to_string(),
                    target: prereq.clone(),
                    relationship: Relationship::Requires,
                    weight: 1.0,
                    origin: EdgeOrigin::Extracted,
                    metadata: serde_json::Value::Null,
                });
            }
            // Related → "related_to" relationship
            for related in &meta.related {
                edges.push(Edge {
                    source: id.to_string(),
                    target: related.clone(),
                    relationship: Relationship::RelatedTo,
                    weight: 0.5,
                    origin: EdgeOrigin::Extracted,
                    metadata: serde_json::Value::Null,
                });
            }
        }
        edges
    }
}
```

### Building the Graph

```rust
use fabryk::graph::{GraphBuilder, load_graph, save_graph, is_cache_fresh};

// Check freshness first
let graph_cache = config.cache_path("graph");
if is_cache_fresh(&graph_cache, &config.content_path).await? {
    let graph = load_graph(&graph_cache)?;
    tracing::info!("Loaded cached graph ({} nodes)", graph.nodes.len());
} else {
    let mut builder = GraphBuilder::new();
    builder.add_items_from_directory(&config.content_path, MyGraphExtractor).await?;
    let graph = builder.build()?;

    // Validate
    let validation = fabryk::graph::validate_graph(&graph);
    if !validation.is_valid() {
        for issue in &validation.issues {
            tracing::warn!("Graph issue: {}", issue);
        }
    }

    save_graph(&graph, &graph_cache)?;
    tracing::info!("Built graph: {} nodes, {} edges", graph.nodes.len(), graph.edges.len());
}
```

### Using Graph Algorithms

```rust
use fabryk::graph::{shortest_path, prerequisites_sorted, neighborhood, calculate_centrality};

// Find the learning path between two concepts
let path = shortest_path(&graph, "intervals", "jazz-harmony")?;

// Get prerequisites in learning order
let prereqs = prerequisites_sorted(&graph, "functional-harmony")?;

// Explore a concept's neighborhood (2 hops)
let neighbors = neighborhood(&graph, "chord-substitution", 2)?;

// Find the most important concepts
let centrality = calculate_centrality(&graph);
```

### Register with MCP

```rust
use fabryk_mcp::graph::GraphTools;

let graph_tools = GraphTools::new(Arc::new(graph));
// Generates: graph_related, graph_path, graph_prerequisites,
//            graph_neighborhood, graph_info, graph_validate,
//            graph_centrality, graph_bridges
```

---

## 8. Vector & Semantic Search

Vector search enables "find items similar to this query" using embedding
models. Fabryk supports LanceDB for storage and FastEmbed for local
embedding generation.

### Implementing VectorExtractor

```rust
use fabryk::vector::VectorExtractor;

/// Tell Fabryk how to compose text for embedding from your content items.
struct MyVectorExtractor;

impl VectorExtractor for MyVectorExtractor {
    fn extract_text(&self, id: &str, content: &str) -> Option<String> {
        let frontmatter = fabryk::content::extract_frontmatter(content).ok()?;
        let meta: ItemMetadata = serde_yaml::from_value(frontmatter.data).ok()?;
        let body = fabryk::content::extract_text_content(content);

        // Compose embedding text: title + summary + body
        // Title and summary are weighted by appearing first
        Some(format!("{}\n{}\n{}", meta.title, meta.summary, body))
    }

    fn extract_metadata(&self, id: &str, content: &str) -> serde_json::Value {
        let frontmatter = fabryk::content::extract_frontmatter(content).ok();
        let meta: Option<ItemMetadata> = frontmatter
            .and_then(|fm| serde_yaml::from_value(fm.data).ok());

        serde_json::json!({
            "category": meta.as_ref().map(|m| &m.category),
            "tags": meta.as_ref().map(|m| &m.tags),
        })
    }
}
```

### Building the Vector Index

```rust
use fabryk::vector::{
    VectorIndexBuilder, VectorConfig,
    create_vector_backend, FastEmbedProvider,
};
use std::sync::Arc;

let embedding_provider = Arc::new(FastEmbedProvider::new()?);
let vector_config = VectorConfig {
    db_path: config.cache_path("vector"),
    ..Default::default()
};

let backend = create_vector_backend(&vector_config).await?;

let mut builder = VectorIndexBuilder::new(embedding_provider.clone());
builder.add_items_from_directory(
    &config.content_path,
    MyVectorExtractor,
).await?;
builder.build_into(&backend).await?;

tracing::info!("Built vector index: {} documents", builder.document_count());
```

### Hybrid Search (Keyword + Semantic)

Fabryk provides reciprocal rank fusion (RRF) to merge FTS and vector
results:

```rust
use fabryk::vector::reciprocal_rank_fusion;

let fts_results = fts_backend.search(query_params).await?;
let vector_results = vector_backend.search(vector_params).await?;

let hybrid = reciprocal_rank_fusion(
    &fts_results.results,
    &vector_results.results,
    60, // RRF constant (higher = more weight to top results)
);
```

### Register with MCP

```rust
use fabryk_mcp::semantic::SemanticSearchTools;

let semantic_tools = SemanticSearchTools::new(
    fts_backend,       // keyword search
    vector_backend,    // vector search
);
// Generates: semantic_search (with mode: keyword | vector | hybrid)
```

### Deferred Vector Backend (VectorSlot)

If your vector index takes time to build, use `VectorSlot` to register
the semantic search tools before the backend is ready:

```rust
use fabryk_mcp::semantic::VectorSlot;
use std::sync::Arc;
use tokio::sync::RwLock;

let vector_slot: VectorSlot = Arc::new(RwLock::new(None));

// Register tools immediately (queries will fail gracefully until ready)
let semantic_tools = SemanticSearchTools::with_vector_slot(
    fts_backend,
    vector_slot.clone(),
);

// Later, when the vector index is built:
tokio::spawn(async move {
    let backend = build_vector_index().await;
    *vector_slot.write().await = Some(backend);
    // semantic_search now works with vector + hybrid modes
});
```

---

## 9. MCP Server Core

`FabrykMcpServer` is the central server type. It wraps a `ToolRegistry`
(typically a `CompositeRegistry`) and handles MCP protocol negotiation.

```rust
use fabryk_mcp::{FabrykMcpServer, ServerConfig};

let server = FabrykMcpServer::new(registry)
    .with_config(ServerConfig {
        name: "my-knowledge-server".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        description: Some("Knowledge exploration MCP server".into()),
    })
    .with_instructions(
        "This server provides access to a knowledge corpus. \
         Use semantic_search for open-ended queries, graph tools \
         for relationship exploration, and concepts_get for \
         reading specific items."
    );
```

### Stdio Transport (for Claude Code)

```rust
server.serve_stdio().await?;
```

### HTTP Transport (for web clients)

```rust
server.serve_http("0.0.0.0", config.port).await?;
```

See [Section 13](#13-authentication--http-transport) for adding
authentication to the HTTP transport.

---

## 10. Tool Composition & Registries

Fabryk provides three registry types that compose like layers:

### CompositeRegistry — Combine Independent Tool Groups

```rust
use fabryk_mcp::CompositeRegistry;

let registry = CompositeRegistry::new()
    .add(content_tools)     // concepts_list, concepts_get, concepts_categories
    .add(fts_tools)         // search, search_status
    .add(graph_tools)       // graph_related, graph_path, ...
    .add(semantic_tools)    // semantic_search
    .add(health_tools);     // health, diagnostics
```

### ServiceAwareRegistry — Gate Tools on Service Readiness

Wrap any registry to gate its tools on service readiness. Before the
service reports `Ready`, tool calls return a "service is starting" error:

```rust
use fabryk_mcp::ServiceAwareRegistry;
use fabryk::core::ServiceHandle;

let graph_svc = ServiceHandle::new("graph");
let vector_svc = ServiceHandle::new("vector");

// These tools are only available after their services are ready
let gated_graph = ServiceAwareRegistry::new(
    graph_tools,
    vec![graph_svc.clone()],
);

let gated_semantic = ServiceAwareRegistry::new(
    semantic_tools,
    vec![vector_svc.clone()],
);

let registry = CompositeRegistry::new()
    .add(content_tools)        // Always available (loaded at startup)
    .add(fts_tools)            // Always available
    .add(gated_graph)          // Available after graph builds
    .add(gated_semantic)       // Available after vector index builds
    .add(health_tools);
```

### DiscoverableRegistry — Add Metadata & Directory Tool

Wrap any registry to attach tool metadata and auto-generate a
`{prefix}_directory` tool:

```rust
use fabryk_mcp::{DiscoverableRegistry, ToolMeta};

let discoverable = DiscoverableRegistry::new(registry, "kb")
    .with_tool_meta("concepts_list", ToolMeta {
        summary: "List all concepts, optionally filtered by category".into(),
        when_to_use: "When the user wants to browse or explore available topics".into(),
        returns: "Array of concept summaries with title, category, tags".into(),
        next: vec!["concepts_get".into(), "graph_related".into()],
        category: "content".into(),
    })
    .with_tool_meta("semantic_search", ToolMeta {
        summary: "Search by meaning using keyword, vector, or hybrid mode".into(),
        when_to_use: "When the user asks a question or searches for something".into(),
        returns: "Ranked results with relevance scores".into(),
        next: vec!["concepts_get".into(), "graph_neighborhood".into()],
        category: "search".into(),
    });
// Auto-generates: kb_directory tool
```

See the [MCP Metadata howto](mcp-metadata.md) for full `ToolMeta` design
guidelines.

---

## 11. Service Lifecycle & Health

For services that take time to initialize (vector index building, graph
construction), Fabryk provides `ServiceHandle` with lifecycle management.

### spawn_with_retry

```rust
use fabryk::core::{ServiceHandle, spawn_with_retry};

let graph_svc = ServiceHandle::new("graph");
let vector_svc = ServiceHandle::new("vector");

// Build graph in background with retry
spawn_with_retry(
    graph_svc.clone(),
    || async {
        let graph = build_graph(&config).await?;
        // Store in shared state...
        Ok(())
    },
    3,     // max attempts
    1000,  // initial delay ms
    2.0,   // backoff multiplier
).await;

// Build vector index in background
spawn_with_retry(
    vector_svc.clone(),
    || async {
        let index = build_vector_index(&config).await?;
        *vector_slot.write().await = Some(index);
        Ok(())
    },
    3,
    2000,
    2.0,
).await;
```

### Wait for Services (Optional)

```rust
use fabryk::core::wait_all_ready;

// Block until all services are ready (with timeout)
wait_all_ready(
    &[graph_svc.clone(), vector_svc.clone()],
    std::time::Duration::from_secs(120),
).await?;
```

### Health Endpoint (HTTP)

```rust
use fabryk_mcp::health_router;

let services = vec![graph_svc.clone(), vector_svc.clone()];

// Returns 200 when all services are ready, 503 otherwise
let health = health_router(services);
```

### Built-in Health Tool (MCP)

```rust
use fabryk_mcp::HealthTools;

let health_tools = HealthTools::new(vec![graph_svc, vector_svc]);
// Generates: health tool (returns service status + tool count)
```

See the [MCP Async Startup howto](mcp-async-startup.md) and
[MCP Health howto](mcp-health.md) for detailed patterns.

---

## 12. Tool Metadata & Discoverability

The `DiscoverableRegistry` (section 10) generates a directory tool that
tells AI clients what's available, when to use each tool, and what
workflow to follow.

### Query Strategy Guidance

Your server instructions should describe the multi-tool workflow:

```rust
let instructions = r#"
This server exposes a knowledge corpus through multiple tools.

**Query Strategy:**
1. Start with `semantic_search` (mode: hybrid) for open-ended questions
2. Use `concepts_get` to read full content of interesting results
3. Use `graph_related` or `graph_neighborhood` to explore connections
4. Use `graph_prerequisites` to find learning order
5. Use `graph_path` to find how concepts connect

**Tool Categories:**
- Search: semantic_search, search
- Content: concepts_list, concepts_get, concepts_categories
- Graph: graph_related, graph_path, graph_prerequisites, etc.
- Meta: kb_directory, health
"#;

server.with_instructions(instructions);
```

See the [MCP Metadata howto](mcp-metadata.md) for full design guidelines.

---

## 13. Authentication & HTTP Transport

For HTTP-accessible MCP servers, Fabryk provides Google OAuth2
authentication and RFC 9728/8414 discovery endpoints.

### OAuth2 Discovery Routes

```rust
use fabryk_mcp_auth::discovery_routes;

let resource_url = "https://my-server.run.app";
let auth_server_url = "https://accounts.google.com";

let discovery = discovery_routes(resource_url, auth_server_url);
// Serves:
//   /.well-known/oauth-protected-resource  (RFC 9728)
//   /.well-known/oauth-authorization-server (RFC 8414)
```

### Google Token Validation

```rust
use fabryk_auth::{AuthLayer, AuthConfig};
use fabryk_auth_google::GoogleTokenValidator;

let validator = GoogleTokenValidator::new(
    "https://www.googleapis.com/oauth2/v3/certs".into(),
);

let auth_config = AuthConfig {
    enabled: config.oauth.is_some(),
    audience: config.oauth.as_ref()
        .map(|o| o.client_id.clone())
        .unwrap_or_default(),
    domain: config.oauth.as_ref()
        .and_then(|o| o.allowed_domain.clone())
        .unwrap_or_default(),
};
```

### Composing the HTTP Router

```rust
use axum::Router;

let app = Router::new()
    .merge(health_router(services))
    .merge(discovery_routes(resource_url, auth_server_url))
    .layer(AuthLayer::new(validator, auth_config));

// Bind with port override for Cloud Run
let port = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(config.port);

let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
axum::serve(listener, app).await?;
```

### Dual-Mode Support (HTTP + Stdio)

Support both transports from the same binary:

```rust
#[derive(clap::Parser)]
struct Args {
    /// Run in stdio mode (for Claude Code)
    #[arg(long)]
    stdio: bool,

    /// HTTP port (overridden by PORT env var in Cloud Run)
    #[arg(long, default_value = "8080")]
    port: u16,
}

async fn main() -> Result<()> {
    let args = Args::parse();

    if args.stdio {
        server.serve_stdio().await?;
    } else {
        server.serve_http("0.0.0.0", args.port).await?;
    }

    Ok(())
}
```

See the [MCP with Claude Code howto](mcp-with-claude-code.md) for
registration details.

---

## 14. CLI Integration

Fabryk's CLI framework provides config validation, graph inspection, and
vector database commands. Extend it with your domain-specific commands.

```rust
use fabryk_cli::{FabrykCli, CliExtension};

struct MyCliExtension;

impl CliExtension for MyCliExtension {
    // Add domain-specific subcommands
    fn register(app: clap::Command) -> clap::Command {
        app.subcommand(
            clap::Command::new("validate")
                .about("Validate all content files")
        )
    }
}

let cli = FabrykCli::<Config>::new()
    .with_extension::<MyCliExtension>();

cli.run().await?;
```

### Built-in Commands

```bash
# Show/validate configuration
my-server config show
my-server config validate

# Graph operations
my-server graph validate    # Check graph structure
my-server graph stats       # Node/edge counts, centrality

# Vector database (with vector-fastembed feature)
my-server vectordb get-model  # Download embedding model
```

---

## 15. Testing

### Schema Validation

Verify your MCP tools have valid schemas before deployment:

```rust
use fabryk_mcp::assert_tools_valid;

#[tokio::test]
async fn test_all_tools_have_valid_schemas() {
    let registry = build_test_registry().await;
    let discoverable = DiscoverableRegistry::new(registry, "kb");

    // Panics if any tool has an invalid inputSchema
    assert_tools_valid(&discoverable);
}
```

### Health Endpoint Tests

```rust
use axum::http::StatusCode;

#[tokio::test]
async fn test_health_returns_200_when_ready() {
    let svc = ServiceHandle::new("test");
    svc.set_ready();

    let app = health_router(vec![svc]);
    let response = app
        .oneshot(axum::http::Request::get("/health").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_returns_503_when_starting() {
    let svc = ServiceHandle::new("test");
    // Don't set ready

    let app = health_router(vec![svc]);
    let response = app
        .oneshot(axum::http::Request::get("/health").body(axum::body::Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}
```

### Mock Backends for Unit Tests

```rust
use fabryk::vector::{MockEmbeddingProvider, SimpleVectorBackend};
use fabryk::graph::MockExtractor; // requires graph/test-utils feature

// In-memory vector backend (no LanceDB needed)
let mock_embeddings = Arc::new(MockEmbeddingProvider::new(384));
let simple_backend = SimpleVectorBackend::new(mock_embeddings);

// In-memory Redis (no Redis server needed)
use fabryk_redis::MockRedis;
let mock_redis = MockRedis::new();
```

---

## 16. Deployment

### Cloud Run

Fabryk detects Cloud Run automatically and adjusts:

```dockerfile
FROM rust:1.85 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my-knowledge-server /usr/local/bin/
COPY content/ /app/content/

ENV CONTENT_PATH=/app/content
ENV CACHE_PATH=/app/cache

EXPOSE 8080
CMD ["my-knowledge-server", "--port", "8080"]
```

Cloud Run sets the `PORT` environment variable — your server should
respect it (see section 13).

### Health Check Configuration

Cloud Run uses HTTP health checks:

```yaml
# Cloud Run service.yaml
spec:
  template:
    spec:
      containers:
        - image: my-knowledge-server
          ports:
            - containerPort: 8080
          startupProbe:
            httpGet:
              path: /health
              port: 8080
            initialDelaySeconds: 10
            periodSeconds: 5
            failureThreshold: 30
```

The `/health` endpoint returns 503 while indices are building and 200
when all services are ready — Cloud Run won't route traffic until
indices are built.

---

## 17. Quick Start Checklist

Minimum steps to get a working MCP server:

- [ ] Create project with `cargo new my-knowledge-server`
- [ ] Add Fabryk dependencies to Cargo.toml (section 2)
- [ ] Create `config.toml` with content/cache paths (section 3)
- [ ] Define domain types for your frontmatter schema (section 4)
- [ ] Implement `ContentItemProvider` for content access (section 5)
- [ ] Implement `DocumentExtractor` for FTS indexing (section 6)
- [ ] Implement `GraphExtractor` for relationship extraction (section 7)
- [ ] Implement `VectorExtractor` for embedding text composition (section 8)
- [ ] Compose tools into a `CompositeRegistry` (section 10)
- [ ] Create `FabrykMcpServer` and call `serve_stdio()` (section 9)
- [ ] Register with Claude Code: `claude mcp add my-server -- cargo run -- --stdio`
- [ ] Test: ask Claude to search your knowledge base

---

## 18. Pattern Reference

| Pattern | Where | Purpose |
|---------|-------|---------|
| `ConfigManager` + `ConfigProvider` | Section 3 | Application config + Fabryk builder bridge |
| `ContentItemProvider` | Section 5 | Domain content → MCP content tools |
| `DocumentExtractor` | Section 6 | Domain content → FTS search documents |
| `GraphExtractor` | Section 7 | Domain content → knowledge graph nodes/edges |
| `VectorExtractor` | Section 8 | Domain content → embedding text |
| `CompositeRegistry` | Section 10 | Combine independent tool groups |
| `ServiceAwareRegistry` | Section 10 | Gate tools on service readiness |
| `DiscoverableRegistry` | Section 12 | Add metadata + directory tool |
| `spawn_with_retry` | Section 11 | Resilient background service startup |
| `VectorSlot` | Section 8 | Deferred vector backend wiring |
| `health_router` | Section 11 | HTTP health endpoint |
| `discovery_routes` | Section 13 | OAuth2 metadata endpoints |
| `AuthLayer` | Section 13 | Tower auth middleware |
| `FabrykMcpServer` | Section 9 | MCP server (stdio or HTTP) |
| `FabrykCli` | Section 14 | CLI framework with built-in commands |

### Related Howtos

- [MCP Async Startup](mcp-async-startup.md) — ServiceHandle lifecycle, spawn_with_retry, parallel wait
- [MCP Health](mcp-health.md) — Health endpoint, HTTP status codes, testing
- [MCP Metadata](mcp-metadata.md) — DiscoverableRegistry, ToolMeta, ExternalConnector
- [MCP with Claude Code](mcp-with-claude-code.md) — Registration, .mcp.json, transport options
