---
number: 14
title: "Fabryk Architecture - 2026-02-24"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-02-24
updated: 2026-02-24
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Fabryk Architecture - 2026-02-24

**Date:** 2026-02-24
**Version:** 0.1.0-alpha.0
**Status:** Active development (extracted from ai-music-theory MCP server)

---

## 1. Overview

Fabryk is a modular Rust **knowledge fabric framework** — a set of composable crates that provide persistent, queryable knowledge storage across three search modalities (full-text, knowledge graph, and vector/semantic), exposed natively via the Model Context Protocol (MCP).

Fabryk lives within the ECL ("Extract, Cogitate, Load") workspace. Where ECL orchestrates *what AI does* (durable workflow execution via Restate), Fabryk manages *what AI knows* (persistent, indexed, searchable knowledge). The two are designed to converge — ECL workflow outputs flowing into Fabryk for persistence and retrieval — but this integration is not yet implemented. Today they are code-level independent.

Fabryk was extracted from a working Music Theory MCP server (~12,000 lines of Rust) where ~87% of the code turned out to be domain-agnostic infrastructure. The extraction yielded 15 crates organized into a clean, layered architecture with trait-based extension points that allow any knowledge domain to plug in.

### Key Characteristics

- **15 crates** in a tiered dependency hierarchy (Levels 0–3)
- **12 core traits** as extension points for consuming applications
- **Feature-gated** heavy backends (Tantivy, LanceDB, fastembed, rkyv) — compile only what you need
- **MCP-native** serving via rmcp with composite tool registries
- **Service lifecycle** management with readiness gating
- **Content-hash freshness** across all three search backends
- **Provider-agnostic auth** with pluggable token validators and Tower middleware

---

## 2. Design Principles

### 2.1 Trait-Based Domain Abstraction

Every domain-specific concern is abstracted behind a trait. Fabryk provides the infrastructure; consuming applications implement traits to inject their domain knowledge. A music theory app, a programming knowledge base, and a legal document system all implement the same `GraphExtractor`, `DocumentExtractor`, and `ContentItemProvider` traits with their own content interpretation.

### 2.2 Feature-Gated Heavy Dependencies

Heavy backend dependencies are optional via Cargo features:

- **Tantivy** (search engine, ~200MB compiled) → `fts-tantivy`
- **LanceDB + Arrow** (vector DB) → `vector-lancedb`
- **fastembed** (local embeddings) → `vector-fastembed`
- **rkyv + memmap2 + blake3** (binary graph cache) → `graph-rkyv-cache`

Each has a lightweight fallback: `SimpleSearch`, `SimpleVectorBackend` (in-memory cosine similarity with JSON cache), `MockEmbeddingProvider`, and JSON persistence.

### 2.3 Service Lifecycle & Readiness Gating

Background services (index builders, cache loaders) broadcast their state via `tokio::sync::watch` channels. The MCP tool layer gates tool calls on service readiness — tools are always *listed* (so MCP clients know they exist) but return informative errors when backing services aren't ready. Startup can block until all services reach `Ready`.

### 2.4 Composite Registry Pattern

MCP tools from different domains are composed at runtime via a builder pattern:

```rust
CompositeRegistry::new()
    .add(content_tools)    // fabryk-mcp-content
    .add(fts_tools)        // fabryk-mcp-fts
    .add(graph_tools)      // fabryk-mcp-graph
    .add(health_tools)     // fabryk-mcp (built-in)
```

Each sub-registry implements `ToolRegistry` independently. `CompositeRegistry` merges them into a unified tool surface.

### 2.5 Content-Hash Freshness

All three search backends (FTS, graph, vector) implement content-hash based freshness checking: hash content files on disk, compare against stored metadata, skip rebuild if fresh. This makes startup fast when content hasn't changed.

### 2.6 Provider-Agnostic Authentication

The auth layer is a standalone Tower middleware parameterized by a `TokenValidator` trait. Different OAuth2/OIDC providers implement the trait; the middleware handles token extraction, validation dispatch, and error responses uniformly.

---

## 3. Crate Dependency Layers

### 3.1 Level 0 — Foundation (no internal Fabryk dependencies)

| Crate | Purpose | Key Types & Traits | Notable External Deps |
|-------|---------|-------------------|----------------------|
| `fabryk-core` | Shared types, errors, config, utilities | `Error`, `Result`, `ConfigProvider` trait, `AppState<C>`, `ServiceHandle`, `ServiceState`, `PathResolver` | tokio, async-trait, serde, thiserror, glob, dirs |
| `fabryk-auth` | Auth primitives & Tower middleware | `AuthConfig`, `TokenValidator` trait, `AuthLayer<V>`, `AuthService<V,S>`, `AuthenticatedUser`, `AuthError` | axum, tower, http, serde, thiserror |
| `fabryk-auth-mcp` | RFC 9728/8414 OAuth2 discovery endpoints | `discovery_routes(resource_url, auth_server_url) -> Router` | axum, serde_json |

**Notes:**

- `fabryk-core` is the foundation every other Fabryk crate depends on (except the auth crates).
- `fabryk-auth` and `fabryk-auth-mcp` are deliberately independent of `fabryk-core` — they depend only on standard HTTP/web crates, making them reusable outside Fabryk.

### 3.2 Level 1 — Domain Engines & Infrastructure (depend on Level 0)

| Crate | Deps | Purpose | Key Types & Traits | Notable External Deps |
|-------|------|---------|-------------------|----------------------|
| `fabryk-content` | core | Markdown parsing & frontmatter extraction | `FrontmatterResult`, `extract_frontmatter()`, `extract_first_heading()`, `extract_text_content()`, `strip_frontmatter()` | pulldown-cmark, serde_yaml, regex |
| `fabryk-fts` | core | Full-text search infrastructure | `SearchBackend` trait, `DocumentExtractor` trait, `SearchParams`, `SearchResult(s)`, `TantivySearch`, `SimpleSearch`, `SearchSchema` (14 fields), `QueryBuilder`, `IndexBuilder` | tantivy (gated), stop-words (gated), chrono |
| `fabryk-graph` | core, content | Knowledge graph storage & algorithms | `GraphExtractor` trait, `Node`, `Edge`, `GraphData` (petgraph DiGraph), `Relationship` (10 variants), `GraphBuilder`, `neighborhood()`, `shortest_path()`, `prerequisites_sorted()`, `calculate_centrality()`, `find_bridges()` | petgraph, rkyv (gated), memmap2 (gated), blake3 (gated), chrono |
| `fabryk-vector` | core, content | Vector/semantic search | `VectorBackend` trait, `EmbeddingProvider` trait, `VectorExtractor` trait, `SimpleVectorBackend`, `VectorIndexBuilder`, `reciprocal_rank_fusion()` | lancedb (gated), arrow (gated), fastembed (gated), blake3 |
| `fabryk-mcp` | core | MCP server infrastructure | `ToolRegistry` trait, `CompositeRegistry`, `ServiceAwareRegistry`, `FabrykMcpServer`, `ServerConfig`, `serve_stdio()`, `HealthTools` | rmcp (server + transport-io), schemars |
| `fabryk-acl` | core | Access control (**placeholder**) | *None — deferred to v0.2/v0.3* | async-trait, serde |
| `fabryk-auth-google` | auth | Google OAuth2 token validation | `GoogleTokenValidator` (implements `TokenValidator`), JWKS caching (1hr TTL), JWT + userinfo dual validation | jsonwebtoken, reqwest |

### 3.3 Level 2 — MCP Tool Bridges & CLI (depend on Levels 0+1)

| Crate | Deps | Purpose | Key Types & Traits | Tools Exposed |
|-------|------|---------|-------------------|--------------|
| `fabryk-mcp-content` | core, content, mcp | Content/source MCP tools | `ContentItemProvider` trait, `SourceProvider` trait, `ContentTools<P>`, `SourceTools<P>` | list-items, get-item, get-chapter, list-sources, get-source |
| `fabryk-mcp-fts` | core, fts, mcp | Full-text search MCP tools | `FtsTools` | search, search-status |
| `fabryk-mcp-graph` | core, graph, mcp | Knowledge graph MCP tools | `GraphTools` | graph-related, graph-path, graph-prerequisites, graph-neighborhood, graph-info, graph-validate, graph-centrality, graph-bridges |
| `fabryk-cli` | core, graph, content | CLI framework | `FabrykCli<C>`, `FabrykConfig`, `CliExtension` trait, `BaseCommand` | serve, index, version, health, graph (build/validate/stats/query), config (path/get/set/init/export) |

### 3.4 Level 3 — Umbrella

| Crate | Purpose | Feature Flags |
|-------|---------|--------------|
| `fabryk` | Feature-gated re-exports of all sub-crates | `default=[]`, `full=[fts,graph,vector,mcp,cli]`, `fts`, `graph`, `vector`, `mcp`, `cli` |

### 3.5 Dependency Diagram

```
                    ┌─────────┐
                    │ fabryk  │  (umbrella, feature-gated)
                    └────┬────┘
                         │
         ┌───────────────┼───────────────┐
         │               │               │
    Level 2         Level 2         Level 2
  ┌──────┴──────┐  ┌────┴────┐   ┌──────┴──────┐
  │ mcp-content │  │   cli   │   │  mcp-graph  │
  │  mcp-fts    │  └────┬────┘   └──────┬──────┘
  └──────┬──────┘       │               │
         │              │               │
    Level 1        Level 1         Level 1
  ┌──────┴──────────────┴───────────────┴──────┐
  │  content   fts   graph   vector   mcp      │
  │                                    acl*    │
  └──────────────────┬─────────────────────────┘
                     │
                Level 0
           ┌─────────┴─────────┐
           │    fabryk-core    │
           └───────────────────┘

  * acl is placeholder

  Auth stack (independent of core):
  ┌─────────────────┐    ┌──────────────┐
  │ fabryk-auth-mcp │    │ auth-google  │
  └────────┬────────┘    └──────┬───────┘
           │                    │
      ┌────┴────────────────────┘
      │  (no dep)       (depends on)
  ┌───┴──────────┐
  │ fabryk-auth  │
  └──────────────┘
```

---

## 4. Core Trait Surface

These 12 traits form Fabryk's extension API. Traits marked "Consumer implements" are the integration points for domain applications.

| Trait | Crate | Purpose | Consumer Implements? |
|-------|-------|---------|---------------------|
| `ConfigProvider` | fabryk-core | Project name, base path, content/cache paths | Yes |
| `SearchBackend` | fabryk-fts | Full-text search (search, name, is_ready) | No (use provided backends) |
| `DocumentExtractor` | fabryk-fts | Domain content → search documents for batch indexing | Yes |
| `GraphExtractor` | fabryk-graph | Domain content → graph nodes and edges | Yes |
| `VectorBackend` | fabryk-vector | Vector search (search, name, is_ready, doc count) | No (use provided backends) |
| `EmbeddingProvider` | fabryk-vector | Text → embedding vectors (embed, embed_batch, dimension) | No (use provided providers) |
| `VectorExtractor` | fabryk-vector | Domain content → text for embedding | Yes |
| `ToolRegistry` | fabryk-mcp | MCP tool listing and dispatch (tools, call) | Rarely (use provided registries) |
| `ContentItemProvider` | fabryk-mcp-content | Domain content listing and retrieval | Yes |
| `SourceProvider` | fabryk-mcp-content | Source material access | Yes |
| `TokenValidator` | fabryk-auth | Token string → AuthenticatedUser | Only for custom auth providers |
| `CliExtension` | fabryk-cli | Domain-specific CLI subcommands | Yes |

### Typical Consumer Implementation Set

A minimal domain application implements 4 traits:

1. `ConfigProvider` — where your content lives
2. `GraphExtractor` — how your content maps to knowledge graph nodes/edges
3. `DocumentExtractor` — how your content maps to search documents
4. `ContentItemProvider` — how to list and retrieve content items via MCP

Add `VectorExtractor` for semantic search, `SourceProvider` for source material access, `CliExtension` for custom CLI commands, and `TokenValidator` only for non-Google auth providers.

---

## 5. Feature Flags

### Umbrella Crate (`fabryk`)

| Flag | Enables | Default |
|------|---------|---------|
| `fts` | `fabryk-fts` with `fts-tantivy` | Off |
| `graph` | `fabryk-graph` with `graph-rkyv-cache` | Off |
| `vector` | `fabryk-vector` with `vector-lancedb` + `vector-fastembed` | Off |
| `mcp` | `fabryk-mcp`, `fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph` | Off |
| `cli` | `fabryk-cli` | Off |
| `full` | All of the above | Off |

### Per-Crate Feature Flags

| Crate | Flag | What It Enables | Underlying Dependency |
|-------|------|-----------------|-----------------------|
| `fabryk-fts` | `fts-tantivy` | Tantivy search engine backend | tantivy, stop-words |
| `fabryk-graph` | `graph-rkyv-cache` | Binary graph persistence with content-hash validation | rkyv, memmap2, blake3 |
| `fabryk-graph` | `test-utils` | Mock types for downstream testing | — |
| `fabryk-vector` | `vector-lancedb` | LanceDB ANN search backend | lancedb, arrow-array, arrow-schema |
| `fabryk-vector` | `vector-fastembed` | Local embedding generation | fastembed |
| `fabryk-mcp-fts` | `fts-tantivy` | Pass-through to fabryk-fts | tantivy |
| `fabryk-mcp-graph` | `graph-rkyv-cache` | Pass-through to fabryk-graph | rkyv |

### Lightweight Fallbacks

When feature-gated backends are disabled, each subsystem provides a fallback:

| Subsystem | Feature Backend | Fallback | Trade-off |
|-----------|----------------|----------|-----------|
| FTS | `TantivySearch` | `SimpleSearch` | No ranking, no schema — basic substring matching |
| Graph | rkyv binary cache | JSON persistence | Slower load for large graphs |
| Vector | `LancedbBackend` | `SimpleVectorBackend` | Brute-force cosine similarity, JSON cache — fine for <10K docs |
| Embedding | `FastEmbedProvider` | `MockEmbeddingProvider` | Deterministic unit vectors — for testing only |

---

## 6. Subsystem Details

### 6.1 Content Pipeline (`fabryk-content`)

The content layer provides domain-agnostic markdown processing:

- **Frontmatter extraction** — Parses YAML frontmatter from markdown files, returns generic `serde_yaml::Value` (domain crates deserialize into their own structs)
- **Structural extraction** — First heading, first paragraph, text content, list items, section content
- **ID computation** — `normalize_id()` and `id_from_path()` in `fabryk-core` generate stable identifiers from file paths

Design: Returns generic types (strings, `serde_yaml::Value`). Domain crates define their own structs and deserialize. This keeps the content layer domain-free.

### 6.2 Full-Text Search (`fabryk-fts`)

**Architecture:**

```
DocumentExtractor (domain impl)
    ↓ extract documents
IndexBuilder
    ↓ batch index
Indexer → TantivySearch (SearchBackend)
    ↓ query
QueryBuilder → SearchResults
```

**Schema** — 14 fields: id, path, title, description, content, category, source, tags, chapter, part, author, date, content_type, section. Covers most knowledge domain needs.

**Query modes** — `QueryMode::Smart` (default, auto-boost title/description), `And`, `Or`, `MinimumMatch`. `QueryBuilder` applies per-field weights.

**Freshness** — `IndexMetadata` stores a content hash. `is_index_fresh()` re-hashes content and compares, enabling skip-rebuild on unchanged content.

**Stopwords** — Built-in `StopwordFilter` (English, configurable).

### 6.3 Knowledge Graph (`fabryk-graph`)

**Architecture:**

```
GraphExtractor (domain impl)
    ↓ extract_node(), extract_edges()
GraphBuilder (two-phase: nodes then edges)
    ↓ build
GraphData (petgraph DiGraph + lookup tables)
    ↓ query
Algorithms (neighborhood, shortest_path, prerequisites, centrality, bridges)
```

**Node types** — `Domain`, `UserQuery`, `Custom(String)`

**Relationship types** (10) — `Prerequisite`, `LeadsTo`, `RelatesTo`, `Extends`, `Introduces`, `Covers`, `VariantOf`, `ContrastsWith`, `AnswersQuestion`, `Custom(String)`

**Edge origins** — `Frontmatter`, `ContentBody`, `Manual`, `Inferred`

**Graph algorithms:**

- `neighborhood(node, depth)` — N-hop BFS (max depth 10)
- `shortest_path(from, to)` — Dijkstra-based path finding
- `prerequisites_sorted(node)` — Topological sort of prerequisites
- `get_related(node)` — Direct neighbors with relationship info
- `calculate_centrality()` — Degree-based centrality scores
- `find_bridges()` — Gateway nodes connecting clusters

**Persistence** — `SerializableGraph` → JSON or rkyv binary (feature-gated). `GraphMetadata` stores content hash for freshness.

**Runtime mutation** — `GraphData` supports `add_node()`, `add_edge()`, `remove_node()` with automatic index rebuilding.

**Validation** — `validate_graph()` checks for dangling references, orphan nodes, duplicate edges.

### 6.4 Vector/Semantic Search (`fabryk-vector`)

**Architecture:**

```
VectorExtractor (domain impl)
    ↓ compose text for embedding
EmbeddingProvider (fastembed or mock)
    ↓ embed / embed_batch
VectorIndexBuilder
    ↓ build
VectorBackend (LanceDB or SimpleVectorBackend)
    ↓ search
VectorSearchResults
    ↓ (optional)
reciprocal_rank_fusion() → HybridSearchResult
```

**SimpleVectorBackend** — In-memory brute-force cosine similarity with JSON cache persistence. Supports `save_cache()` / `load_cache()` / `is_cache_fresh()`. Suitable for collections under ~10K documents.

**Hybrid search** — `reciprocal_rank_fusion()` merges FTS results with vector results using RRF scoring, producing a unified ranked list.

### 6.5 MCP Server Infrastructure (`fabryk-mcp`)

**Architecture:**

```
ToolRegistry implementations (ContentTools, FtsTools, GraphTools, HealthTools)
    ↓ compose
CompositeRegistry
    ↓ wrap
ServiceAwareRegistry (gates on ServiceHandle readiness)
    ↓ serve
FabrykMcpServer (rmcp ServerHandler)
    ↓ transport
serve_stdio() / (future: streamable HTTP)
```

**ToolRegistry trait** — `tools() -> Vec<Tool>` lists available tools; `call(name, args) -> Option<ToolResult>` dispatches by name.

**CompositeRegistry** — Builder pattern combining multiple sub-registries. Tool name conflicts are last-wins.

**ServiceAwareRegistry** — Wraps any `ToolRegistry`. Tools are always listed (MCP clients see the full tool surface). Calls are gated: if the backing `ServiceHandle` isn't `Ready`, returns an informative JSON error explaining the current state.

**FabrykMcpServer** — Implements rmcp's `ServerHandler`. Accepts a vector of `ServiceHandle`s. `wait_ready(timeout)` blocks until all handles report `Ready` or times out. `ServerConfig` specifies name, version, description for the MCP server info response.

**Health tools** — Built-in `HealthTools` registry provides a `health` MCP tool reporting service states.

### 6.6 Authentication (`fabryk-auth`, `fabryk-auth-google`, `fabryk-auth-mcp`)

**Architecture:**

```
TokenValidator trait (fabryk-auth)
    ↑ implements
GoogleTokenValidator (fabryk-auth-google)

AuthLayer<V: TokenValidator> (Tower Layer)
    ↓ wraps
AuthService<V, S> (Tower Service)
    ↓ on each request
  1. Check config.enabled (dev-mode bypass)
  2. Extract Bearer token from Authorization header
  3. Call validator.validate(token, config)
  4. Insert AuthenticatedUser into request extensions
  5. Or return 401 with WWW-Authenticate header

discovery_routes() (fabryk-auth-mcp)
    ↓ serves
  /.well-known/oauth-protected-resource (RFC 9728)
  /.well-known/oauth-authorization-server (RFC 8414)
  + /mcp variants of both
```

**AuthenticatedUser** — `email: String`, `subject: String` (JWT "sub" claim). Extracted downstream via `user_from_parts()` or `email_from_parts()` (returns `"anonymous"` if unauthenticated).

**AuthError** — Categorized: `MissingToken`, `InvalidFormat`, `InvalidSignature`, `Expired`, `InvalidAudience`, `InvalidDomain`, `MissingEmail`, `JwksFetchError`, `NoMatchingKey`. `is_client_error()` distinguishes 401 vs 500.

**Google validation** — Dual path: (1) JWT id_tokens decoded locally via JWKS (RS256, cached 1hr), (2) opaque access_tokens validated via Google userinfo endpoint. Checks audience, issuer (`accounts.google.com`), email presence, email_verified, and domain (`hd` claim or email domain fallback).

**Current status:** The auth crates are fully implemented and tested, but **not yet wired into any serving path**. No MCP handler or HTTP route currently installs `AuthLayer` or reads `AuthenticatedUser`. The auth infrastructure is ready; integration is pending.

### 6.7 CLI Framework (`fabryk-cli`)

**Architecture:**

```
CliArgs (clap derive: --config, --verbose, --quiet)
    ↓ parse
FabrykCli<C: ConfigProvider>
    ↓ from_args() loads config
BaseCommand dispatch:
  Serve → placeholder
  Index → placeholder
  Version → print version
  Health → check services
  Graph → validate / stats / query
  Config → path / get / set / init / export

CliExtension trait
    ↓ domain crates add custom subcommands
```

**Config loading** — `FabrykConfig` loaded via `confyg` from TOML. Priority: (1) `--config <path>`, (2) `FABRYK_CONFIG` env, (3) `~/.config/fabryk/config.toml`, (4) built-in defaults. Implements `ConfigProvider`.

**Config sections** — project_name, base_path, content (paths, types), graph (settings), server (host, port).

### 6.8 Service Lifecycle (`fabryk-core`)

**ServiceState transitions:**

```
Stopped → Starting → Ready
                   → Degraded
                   → Failed
         Stopping ← (any active state)
```

**ServiceHandle** — Uses `tokio::sync::watch` for broadcast. Multiple consumers can `wait_ready(timeout)` concurrently. Cheap to clone.

**Integration with MCP:**

1. Index builders (FTS, graph, vector) create `ServiceHandle`s and update state as they progress
2. `ServiceAwareRegistry` observes handles and gates tool calls
3. `FabrykMcpServer` collects all handles and offers `wait_ready()` for startup orchestration
4. `HealthTools` reports all service states via MCP

---

## 7. Integration Guide for Consuming Applications

### Step 1: Implement `ConfigProvider`

Tell Fabryk where your content lives:

```rust
impl ConfigProvider for MyConfig {
    fn project_name(&self) -> &str { "my-knowledge-base" }
    fn base_path(&self) -> &Path { &self.base }
    fn content_path(&self, content_type: &str) -> PathBuf { ... }
    fn cache_path(&self, cache_type: &str) -> PathBuf { ... }
}
```

### Step 2: Implement Domain Extractors

**GraphExtractor** — Convert your content into graph nodes and edges:

```rust
impl GraphExtractor for MyExtractor {
    type NodeData = MyFrontmatter;
    type EdgeData = MyEdgeInfo;
    fn extract_node(&self, data: &MyFrontmatter) -> Node { ... }
    fn extract_edges(&self, data: &MyEdgeInfo) -> Vec<Edge> { ... }
}
```

**DocumentExtractor** — Convert your content into search documents for FTS indexing.

**VectorExtractor** — Compose text from your content for embedding generation.

### Step 3: Implement Content Providers

**ContentItemProvider** — How to list and retrieve content items for MCP:

```rust
impl ContentItemProvider for MyProvider {
    type ItemSummary = MySummary;
    type ItemDetail = MyDetail;
    async fn list_items(&self, ...) -> Vec<MySummary> { ... }
    async fn get_item(&self, id: &str) -> Option<MyDetail> { ... }
}
```

### Step 4: Wire Up the MCP Server

```rust
let registry = CompositeRegistry::new()
    .add(ContentTools::new(my_content_provider))
    .add(FtsTools::new(search_backend.clone()))
    .add(GraphTools::new(graph_data.clone()))
    .add(HealthTools::new(service_handles.clone()));

let service_registry = ServiceAwareRegistry::new(registry, service_handles);

let server = FabrykMcpServer::new(
    ServerConfig { name: "my-kb", version: "0.1.0", description: "..." },
    service_registry,
);

server.serve_stdio().await?;
```

### Step 5: Select Feature Flags

In your `Cargo.toml`:

```toml
[dependencies]
fabryk = { version = "0.1", features = ["full"] }
# Or selectively:
# fabryk = { version = "0.1", features = ["fts", "graph", "mcp"] }
```

### Optional: Custom Auth

Implement `TokenValidator` for non-Google OAuth2 providers. Wire `AuthLayer` into your HTTP server's middleware stack.

### Optional: CLI Extension

Implement `CliExtension` to add domain-specific CLI subcommands alongside the built-in graph, config, and health commands.

---

## 8. Planned / Future Work

### fabryk-acl (v0.2/v0.3)

Currently a placeholder crate. Will provide:

- User/tenant identification
- Permission checking
- Resource ownership
- Multi-tenancy isolation

The auth layer authenticates (confirms identity) but does not authorize (restrict data access). `AuthenticatedUser` carries only `email` and `subject` — no scopes, roles, or resource grants. `fabryk-acl` will bridge this gap.

### Additional Auth Providers

The `TokenValidator` trait supports any provider. Google is implemented; others (Auth0, Clerk, custom JWT) can be added as separate crates following the `fabryk-auth-google` pattern.

### Streamable HTTP Transport

Currently MCP serves over stdio via rmcp's `transport-io`. Streamable HTTP transport is planned for web-accessible deployments.

### ECL Integration

ECL workflow outputs flowing into Fabryk for persistence and retrieval. This will connect the "do" layer to the "know" layer.

---

## 9. Appendix: Crate Manifest

| Crate | Dep Level | Internal Deps | Status |
|-------|-----------|---------------|--------|
| `fabryk-core` | 0 | — | Complete |
| `fabryk-auth` | 0 | — | Complete |
| `fabryk-auth-mcp` | 0 | — | Complete |
| `fabryk-content` | 1 | core | Complete |
| `fabryk-fts` | 1 | core | Complete |
| `fabryk-graph` | 1 | core, content | Complete |
| `fabryk-vector` | 1 | core, content | Complete |
| `fabryk-mcp` | 1 | core | Complete |
| `fabryk-acl` | 1 | core | Placeholder |
| `fabryk-auth-google` | 1 | auth | Complete |
| `fabryk-mcp-content` | 2 | core, content, mcp | Complete |
| `fabryk-mcp-fts` | 2 | core, fts, mcp | Complete |
| `fabryk-mcp-graph` | 2 | core, graph, mcp | Complete |
| `fabryk-cli` | 2 | core, graph, content | Complete |
| `fabryk` | 3 | all (feature-gated) | Complete |
