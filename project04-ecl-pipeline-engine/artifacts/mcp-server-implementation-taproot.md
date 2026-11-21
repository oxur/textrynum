
<!-- Historical source: workbench/mcp-server-implementation-taproot.md, lines 1-1135. Original bytes are recoverable from import commit 3a39aea. -->

# Building a Fabryk MCP Server

**A comprehensive guide to implementing and refactoring MCP servers using the
Fabryk ecosystem, based on real-world patterns from the Taproot project.**

This document serves three purposes:

1. **Reference implementation** — Taproot as a well-structured example
2. **New server guide** — How to build an MCP server from scratch with Fabryk
3. **Refactoring guide** — How to migrate an existing MCP server to Fabryk

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Dependencies](#dependencies)
- [Project Structure](#project-structure)
- [Step 1: Configuration](#step-1-configuration)
- [Step 2: Error Handling](#step-2-error-handling)
- [Step 3: The Server Struct](#step-3-the-server-struct)
- [Step 4: Tool Registration](#step-4-tool-registration)
- [Step 5: Tool Business Logic](#step-5-tool-business-logic)
- [Step 6: Service Initialization](#step-6-service-initialization)
- [Step 7: Transport and Auth](#step-7-transport-and-auth)
- [Step 8: Background Services](#step-8-background-services)
- [Step 9: Health Endpoint](#step-9-health-endpoint)
- [Step 10: Deployment](#step-10-deployment)
- [Refactoring an Existing Server](#refactoring-an-existing-server)
- [Pattern Reference](#pattern-reference)

---

## Architecture Overview

A Fabryk MCP server is structured as a Rust binary that exposes domain tools
via the Model Context Protocol. The key architectural decisions:

```
┌──────────────────────────────────────────────────────────────┐
│  main.rs — CLI entry point (clap)                            │
│  └── lib.rs — Startup orchestration                          │
│       ├── Config::load() — multi-source config               │
│       ├── ServiceHandle::new() — lifecycle tracking           │
│       ├── tokio::spawn() — background service init            │
│       ├── TaprootServer::new() — tool registration            │
│       └── run_http() / run_stdio() — transport start          │
├──────────────────────────────────────────────────────────────┤
│  server.rs — MCP server struct                                │
│  ├── #[tool_router] impl — tool dispatch table                │
│  ├── #[tool] methods — individual MCP tools                   │
│  ├── require_*() — service access guards                      │
│  └── OptionalParts — auth identity extraction                 │
├──────────────────────────────────────────────────────────────┤
│  tools/ — Business logic (independently testable)             │
│  ├── schema.rs, query.rs, knowledge.rs, ...                   │
│  └── mod.rs — shared helpers (access control, error format)   │
├──────────────────────────────────────────────────────────────┤
│  Domain engines (knowledge/, bq/, redis/, user_model/)        │
│  └── Adapters mapping domain types → fabryk traits            │
├──────────────────────────────────────────────────────────────┤
│  Fabryk ecosystem                                             │
│  ├── fabryk (core, auth, content, fts, graph, vector)         │
│  ├── fabryk-mcp (server infra, health, auth discovery)        │
│  ├── fabryk-redis, fabryk-auth-google, fabryk-cli             │
│  └── rmcp (MCP protocol implementation)                       │
└──────────────────────────────────────────────────────────────┘
```

**Key principle:** The server struct owns shared state via `Arc` and
`OnceCell`. Tool methods delegate to pure business logic functions in
`tools/`, which are independently testable. Background services initialize
asynchronously while the transport starts immediately.

---

## Dependencies

### Cargo.toml

```toml
[dependencies]
# MCP protocol and transport
rmcp = { version = "0.17", features = [
    "server", "macros",
    "transport-streamable-http-server", "transport-io",
] }

# Fabryk umbrella (core, auth, content, fts, graph, vector)
fabryk = { version = "0.2", features = [
    "fts-tantivy",
    "vector-lancedb",
    "vector-fastembed",
] }

# Fabryk MCP infrastructure (server, health endpoint, auth discovery)
fabryk-mcp = { version = "0.2", features = ["http"] }

# Auth provider (keep separate — not in umbrella)
fabryk-auth-google = "0.2"

# Optional: Redis, CLI framework, GCP utilities
fabryk-redis = "0.2"
fabryk-cli = { version = "0.2", features = ["vector-fastembed"] }

# HTTP framework
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = { version = "0.5", features = ["util"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "1.2"  # Required for MCP tool parameter schemas

# Configuration and logging
confyg = "0.3"
twyg = "0.6"
log = "0.4"
clap = { version = "4", features = ["derive"] }
```

### Import Conventions

With the `fabryk` umbrella crate, use namespaced imports:

```rust
use fabryk::core::service::{ServiceHandle, ServiceState};
use fabryk::core::{ConfigManager, Error};
use fabryk::auth::AuthLayer;
use fabryk::graph::{GraphData, Node, Edge, Relationship};
use fabryk::fts::{Indexer, SearchDocument, SearchSchema};
use fabryk::vector::{VectorBackend, VectorDocument};
use fabryk::content::extract_frontmatter;

use fabryk_mcp::health_router;
use fabryk_mcp::auth::discovery_routes;
```

---

## Project Structure

Organize your crate with clear separation of concerns:

```
src/
├── main.rs                  # CLI entry point
├── lib.rs                   # Startup orchestration
├── server.rs                # MCP server struct + tool registration
├── config.rs                # Configuration types + loading
├── error.rs                 # Error hierarchy
├── deploy.rs                # Environment detection (Cloud Run, etc.)
├── auth/
│   └── mod.rs               # Auth delegation to fabryk
├── tools/                   # Business logic (one file per tool group)
│   ├── mod.rs               # Shared helpers
│   ├── schema.rs            # Schema inspection tools
│   ├── query.rs             # Query execution tools
│   └── knowledge.rs         # Domain knowledge tools
├── knowledge/               # Domain engine (if applicable)
│   ├── mod.rs               # Engine struct + public API
│   ├── types.rs             # Domain types
│   ├── adapters.rs          # Domain → fabryk trait implementations
│   ├── fts.rs               # Full-text search wrapper
│   ├── graph.rs             # Knowledge graph operations
│   └── vector.rs            # Vector/semantic search wrapper
└── redis/                   # Optional persistence layer
    └── mod.rs
```

**Guiding principles:**

- `server.rs` is thin — tool methods call into `tools/` for logic
- `tools/` functions take references, not the full server — independently testable
- Domain engines are self-contained modules with clean public APIs
- `adapters.rs` is the bridge between your domain types and fabryk traits

---

## Step 1: Configuration

### Define Your Config Struct

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub port: u16,
    pub transport: String,  // "http" or "stdio"
    pub logging: twyg::Opts,

    // Domain-specific sections
    pub oauth: OAuthConfig,
    pub redis: RedisConfig,
    // ... your sections here
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub enabled: bool,
    pub client_id: String,
    pub domain: String,
    pub jwks_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub url: String,  // Empty = disabled
}
```

### Implement ConfigManager

The `fabryk::core::ConfigManager` trait gives you integration with
`fabryk-cli` for `config init`, `config get`, `config set` subcommands:

```rust
impl fabryk::core::ConfigManager for Config {
    fn load(config_path: Option<&str>) -> fabryk::core::Result<Self> {
        let mut builder = confyg::Confygery::new()
            .map_err(|e| fabryk::core::Error::config(e.to_string()))?;

        // 1. Load config file if it exists
        if let Some(path) = config_path {
            builder.add_file(path)
                .map_err(|e| fabryk::core::Error::config(e.to_string()))?;
        }

        // 2. Load environment variables with YOUR_PREFIX_
        let mut env_opts = confyg::env::Options::with_top_level("myserver");
        env_opts.add_section("oauth");
        env_opts.add_section("redis");
        builder.add_env(env_opts)
            .map_err(|e| fabryk::core::Error::config(e.to_string()))?;

        // 3. Build
        let config: Self = builder.build()
            .map_err(|e| fabryk::core::Error::config(e.to_string()))?;

        Ok(config)
    }
}
```

### Config Loading Pattern

Environment variables override file values. Cloud Run sets `PORT` directly:

```rust
impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let config = <Self as fabryk::core::ConfigManager>::load(
            Self::resolve_config_path().as_deref()
                .and_then(|p| p.to_str())
        )?;

        // Cloud Run override: PORT env var
        if let Ok(port) = std::env::var("PORT") {
            if let Ok(p) = port.parse::<u16>() {
                config.port = p;
            }
        }

        Ok(config)
    }
}
```

---

## Step 2: Error Handling

### Error Hierarchy with Self-Correction Hints

MCP errors should help AI agents recover. Structure your errors with
categories and actionable hints:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MyServerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    #[error("Knowledge error: {0}")]
    Knowledge(#[from] KnowledgeError),
}

impl MyServerError {
    /// Structured JSON error with self-correction hints for AI agents.
    pub fn to_error_json(&self) -> serde_json::Value {
        match self {
            Self::Permission(msg) => serde_json::json!({
                "error_category": "permission",
                "message": msg,
                "hint": "Check table access configuration. Use list_datasets \
                         to see available tables."
            }),
            Self::NotFound(msg) => serde_json::json!({
                "error_category": "not_found",
                "message": msg,
                "hint": "Verify the identifier exists. Use search or list \
                         tools to discover valid identifiers."
            }),
            // ... more categories
        }
    }
}
```

### Integrate fabryk Errors

Bridge fabryk errors into your error hierarchy:

```rust
impl From<fabryk::core::Error> for KnowledgeError {
    fn from(e: fabryk::core::Error) -> Self {
        match e {
            fabryk::core::Error::NotFound { id, .. } => {
                KnowledgeError::CardNotFound { slug: id }
            }
            fabryk::core::Error::Parse(msg) => {
                KnowledgeError::CardParse { path: String::new(), reason: msg }
            }
            other => KnowledgeError::Internal(other.to_string()),
        }
    }
}
```

### Error-to-Tool-Result Conversion

Every tool returns `Result<CallToolResult, McpError>`. Domain errors become
structured error content, not protocol errors:

```rust
use rmcp::model::{CallToolResult, Content};

pub fn error_to_tool_result(err: MyServerError) -> CallToolResult {
    let json = err.to_error_json();
    CallToolResult::error(vec![Content::text(
        serde_json::to_string_pretty(&json)
            .unwrap_or_else(|_| format!(r#"{{"error": "{err}"}}"#)),
    )])
}
```

---

## Step 3: The Server Struct

### Define the Server

The server struct holds shared state via `Arc` and `OnceCell` for services
that initialize asynchronously:

```rust
use std::sync::Arc;
use fabryk::core::service::ServiceHandle;
use rmcp::handler::server::tool::ToolRouter;

#[derive(Debug, Clone)]
pub struct MyServer {
    tool_router: ToolRouter<Self>,
    config: Arc<Config>,

    // Eager services (ready at startup)
    knowledge: Option<Arc<KnowledgeEngine>>,

    // Deferred services (OnceCell — populated by background tasks)
    redis: Option<Arc<tokio::sync::OnceCell<Arc<dyn RedisOps>>>>,

    // Service lifecycle tracking
    service_handles: Vec<ServiceHandle>,

    // Environment flags
    cloud_run: bool,
}
```

### Service Access Guards

Provide typed accessors that return clear errors when services aren't ready:

```rust
impl MyServer {
    /// Required service — returns error if not configured or not ready.
    fn require_redis(&self) -> Result<&dyn RedisOps, MyServerError> {
        let cell = self.redis.as_ref()
            .ok_or_else(|| MyServerError::Config(
                "Redis is not configured".into()
            ))?;
        cell.get()
            .map(|arc| arc.as_ref())
            .ok_or_else(|| MyServerError::Config(
                "Redis is still connecting".into()
            ))
    }

    /// Optional service — returns None if not available (graceful degradation).
    fn redis_client(&self) -> Option<Arc<dyn RedisOps>> {
        self.redis.as_ref().and_then(|cell| cell.get().cloned())
    }

    /// Required service — no async initialization.
    fn require_knowledge(&self) -> Result<&KnowledgeEngine, MyServerError> {
        self.knowledge.as_deref()
            .ok_or_else(|| MyServerError::Config(
                "Knowledge engine is not configured".into()
            ))
    }
}
```

---

## Step 4: Tool Registration

### The `#[tool_router]` Pattern

rmcp's `#[tool_router]` macro generates a dispatch table from `#[tool]`
methods. Each tool method receives typed parameters and returns a
`CallToolResult`:

```rust
use rmcp::handler::server::tool::tool_router;
use rmcp::{tool, model::*};

#[tool_router]
impl MyServer {
    #[tool(description = "List available datasets and their tables.")]
    async fn list_datasets(
        &self,
    ) -> Result<CallToolResult, McpError> {
        let bq = self.require_bq().await
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        match tools::schema::list_datasets(bq).await {
            Ok(result) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string_pretty(&result).unwrap())
            ])),
            Err(e) => Ok(tools::error_to_tool_result(e)),
        }
    }

    #[tool(description = "Get a concept card by slug identifier.")]
    async fn get_concept(
        &self,
        Parameters(params): Parameters<GetConceptParams>,
    ) -> Result<CallToolResult, McpError> {
        let engine = match self.require_knowledge() {
            Ok(e) => e,
            Err(e) => return Ok(tools::error_to_tool_result(e)),
        };
        match tools::knowledge::get_concept(engine, &params.slug) {
            Ok(json) => Ok(CallToolResult::success(vec![
                Content::text(serde_json::to_string_pretty(&json).unwrap())
            ])),
            Err(e) => Ok(tools::error_to_tool_result(e)),
        }
    }
}
```

### Parameter Structs

Each tool's parameters are a struct with `Deserialize` + `JsonSchema`:

```rust
use schemars::JsonSchema;

#[derive(Debug, Deserialize, JsonSchema)]
struct GetConceptParams {
    /// The concept slug (kebab-case identifier, e.g., "active-user")
    slug: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct SearchParams {
    /// The search query
    query: String,
    /// Maximum results to return (default: 10)
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize { 10 }
```

### Dev-Only Tool Filtering

Use an additive security model — production starts with no dev tools rather
than trying to remove them:

```rust
pub const DEV_ONLY_TOOLS: &[&str] = &[
    "debug_config",
    "debug_credentials",
    "echo",
];

impl MyServer {
    pub fn new(/* ... */, cloud_run: bool) -> Self {
        let mut tool_router = ToolRouter::new();
        let mut dev_routes = Vec::new();

        for route in Self::tool_router() {
            if DEV_ONLY_TOOLS.contains(&route.name()) {
                dev_routes.push(route);
            } else {
                tool_router.add_route(route);
            }
        }

        // Dev tools only in local mode
        if !cloud_run {
            for route in dev_routes {
                tool_router.add_route(route);
            }
        }

        Self { tool_router, /* ... */ }
    }
}
```

### Extracting Auth Identity

Use `OptionalParts` to get the authenticated user's email in tool methods.
Stdio mode has no HTTP parts, so it falls back to a default identity:

```rust
use axum::http::request::Parts;
use rmcp::handler::server::context::{AsRequestContext, FromContextPart};

pub struct OptionalParts(pub Option<Parts>);

impl<C: AsRequestContext> FromContextPart<C> for OptionalParts {
    fn from_context_part(context: &mut C) -> Result<Self, McpError> {
        Ok(OptionalParts(
            context.as_request_context()
                .extensions.get::<Parts>().cloned(),
        ))
    }
}

impl OptionalParts {
    pub fn email(&self) -> &str {
        self.0.as_ref()
            .map(fabryk::auth::email_from_parts)
            .unwrap_or("local-dev@stdio")
    }
}

// Usage in a tool method:
#[tool(description = "Execute a SQL query.")]
async fn execute_query(
    &self,
    parts: OptionalParts,
    Parameters(params): Parameters<QueryParams>,
) -> Result<CallToolResult, McpError> {
    let email = parts.email();
    // ... use email for per-user history, cost tracking, etc.
}
```

---

## Step 5: Tool Business Logic

### Separation of Concerns

Tool methods on the server struct should be thin — delegate to standalone
functions in `tools/` that take specific dependencies:

```rust
// tools/knowledge.rs — independently testable, no server dependency
pub fn get_concept(
    engine: &KnowledgeEngine,
    slug: &str,
) -> Result<serde_json::Value, MyServerError> {
    let card = engine.get_concept(slug)?;
    Ok(serde_json::json!({
        "slug": card.frontmatter.slug,
        "concept": card.frontmatter.concept,
        "category": card.frontmatter.category,
        "body": card.body,
    }))
}

// tools/query.rs — takes trait references, not concrete types
pub async fn execute_query(
    bq: &dyn BqOperations,
    config: &Config,
    sql: &str,
) -> Result<QueryResult, MyServerError> {
    sql::validate_read_only(sql)?;
    let result = bq.execute_raw_query(sql).await?;
    Ok(result)
}
```

### Fire-and-Forget Side Effects

For non-critical operations (history logging, cost tracking), spawn a
background task so the tool response isn't delayed:

```rust
pub async fn execute_query_with_history(
    bq: &dyn BqOperations,
    redis: Option<Arc<dyn RedisOps>>,
    config: &Config,
    sql: &str,
    email: &str,
) -> Result<QueryResult, MyServerError> {
    let result = execute_query(bq, config, sql).await?;

    // Fire-and-forget: don't block the response
    if let Some(redis_client) = redis {
        let entry = build_history_entry(email, sql, &result);
        tokio::spawn(async move {
            if let Err(e) = record_query(&redis_client, &entry).await {
                log::warn!("Failed to record query history: {e}");
            }
        });
    }

    Ok(result)
}
```

### Table Access Control

Implement allow/deny list patterns for data governance:

```rust
pub fn check_table_access(
    config: &Config,
    dataset: &str,
    table: &str,
) -> Result<(), MyServerError> {
    let full_name = format!("{dataset}.{table}");

    if !config.table_allowlist.is_empty() {
        if !config.table_allowlist.contains(&full_name) {
            return Err(MyServerError::Permission(
                format!("Table '{full_name}' is not in the allowlist")
            ));
        }
    } else if config.table_denylist.contains(&full_name) {
        return Err(MyServerError::Permission(
            format!("Table '{full_name}' is blocked")
        ));
    }

    Ok(())
}
```

---

## Step 6: Service Initialization

### The Async Startup Pattern

Start the transport immediately. Expensive services initialize in the
background. Tools return "not ready yet" errors until their backing service
is ready.

```rust
pub async fn run_server() -> anyhow::Result<()> {
    let startup = Instant::now();

    // 1. Config + logging (sync, <100ms)
    let config = Config::load()?;
    twyg::setup(config.logging.clone())?;

    // 2. Create ServiceHandle instances for health reporting
    let redis_svc = ServiceHandle::new("redis");
    let knowledge_svc = ServiceHandle::new("knowledge");
    let vector_svc = ServiceHandle::new("vector");

    // 3. Deferred services via OnceCell + background tasks
    let redis = if config.redis.url.is_empty() {
        None  // redis_svc stays at Stopped — health considers this "ok"
    } else {
        redis_svc.set_state(ServiceState::Starting);
        let cell = Arc::new(tokio::sync::OnceCell::new());
        let cell_bg = cell.clone();
        let url = config.redis.url.clone();
        let svc = redis_svc.clone();

        tokio::spawn(async move {
            match RedisClient::new(&url).await {
                Ok(client) => {
                    let _ = cell_bg.set(Arc::new(client) as Arc<dyn RedisOps>);
                    svc.set_state(ServiceState::Ready);
                }
                Err(e) => {
                    svc.set_state(ServiceState::Failed(format!("{e}")));
                }
            }
        });

        Some(cell)
    };

    // 4. Fast synchronous init (<1s)
    let knowledge = build_knowledge_engine(&config, &knowledge_svc)?;

    // 5. Slow async init (10-30s) — spawned to background
    spawn_vector_build(&config, &knowledge, &vector_svc);

    let config = Arc::new(config);
    let service_handles = vec![redis_svc, knowledge_svc, vector_svc];

    // 6. Start transport IMMEDIATELY (<2s from process start)
    log::info!("Startup complete in {:?}", startup.elapsed());
    run_http(config, redis, knowledge, service_handles).await
}
```

### OnceCell for Struct Fields

When a struct is shared via `Arc` before a component is ready, use `OnceCell`
instead of `Option` to avoid needing `&mut self`:

```rust
pub struct KnowledgeEngine {
    fts: FtsEngine,
    graph: Arc<RwLock<GraphData>>,
    // Populated by background task — no &mut needed
    vector: Arc<tokio::sync::OnceCell<VectorEngine>>,
}

impl KnowledgeEngine {
    pub fn set_vector(&self, v: VectorEngine) {
        let _ = self.vector.set(v);
    }

    pub fn require_vector(&self) -> Result<&VectorEngine, Error> {
        self.vector.get()
            .ok_or_else(|| Error::config("Vector engine still loading"))
    }
}
```

See [mcp-async-startup.md](./mcp-async-startup.md) for the full async
startup pattern with retry, parallel wait, and audit trails.

---

## Step 7: Transport and Auth

### Dual Transport (HTTP + Stdio)

Support both transports for different deployment contexts:

```rust
match config.transport.as_str() {
    "stdio" => {
        // Direct integration with Claude Desktop / Claude Code
        let server = MyServer::new(config, redis, knowledge, handles, cloud_run);
        server.serve(rmcp::transport::io::stdio()).await?;
    }
    _ => {
        run_http(config, redis, knowledge, handles, cloud_run).await?;
    }
}
```

### HTTP with Auth Middleware

For HTTP transport, wrap the MCP service with auth middleware while keeping
health and discovery routes public:

```rust
use fabryk::auth::{AuthConfig, AuthLayer};
use fabryk_auth_google::GoogleTokenValidator;
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

async fn run_http(/* ... */) -> anyhow::Result<()> {
    // Per-session MCP service factory
    let service = StreamableHttpService::new(
        move || Ok(MyServer::new(/* cloned args */)),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Google OAuth2 validator
    let validator = Arc::new(GoogleTokenValidator::new(
        config.oauth.jwks_url.clone()
    ));
    let auth_config = AuthConfig {
        enabled: config.oauth.enabled,
        audience: config.oauth.client_id.clone(),
        domain: config.oauth.domain.clone(),
    };

    // Auth wraps MCP only — health + discovery stay public
    let auth_layer = AuthLayer::new(validator, auth_config);
    let authed_service = auth_layer.layer(service);

    let resource_url = std::env::var("MY_RESOURCE_URL").unwrap_or_default();
    let router = axum::Router::new()
        .merge(fabryk_mcp::health_router(service_handles))
        .merge(fabryk_mcp::auth::discovery_routes(
            &resource_url,
            "https://accounts.google.com",
        ))
        .nest_service("/mcp", authed_service);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.ok(); })
        .await?;

    Ok(())
}
```

---

## Step 8: Background Services

### Worker Pattern with OnceCell Redis

Background workers that depend on an `OnceCell` service check availability
each cycle:

```rust
pub fn spawn_worker(
    redis: Arc<tokio::sync::OnceCell<Arc<dyn RedisOps>>>,
    engine: Arc<MyEngine>,
    config: WorkerConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = Duration::from_secs(config.poll_interval_secs);
        loop {
            match redis.get() {
                Some(client) => {
                    if let Err(e) = poll_and_process(
                        client.as_ref(), &engine, &config
                    ).await {
                        log::warn!("Worker error: {e}");
                    }
                }
                None => {
                    log::debug!("Redis not yet connected, skipping cycle");
                }
            }
            tokio::time::sleep(interval).await;
        }
    })
}
```

### Domain Adapters

Bridge your domain types to fabryk traits for FTS, graph, and vector
integration:

```rust
use fabryk::fts::SearchDocument;
use fabryk::graph::{Edge, GraphExtractor, Node, Relationship};
use fabryk::vector::VectorDocument;

// Domain type → FTS document
impl From<&MyCard> for SearchDocument {
    fn from(card: &MyCard) -> Self {
        SearchDocument::new(&card.slug)
            .with_title(&card.title)
            .with_body(&card.body)
            .with_category(&card.category)
    }
}

// Domain type → Graph node + edges
impl GraphExtractor for MyCard {
    fn to_node(&self) -> Node {
        Node::new(&self.slug, &self.title)
            .with_category(&self.category)
    }

    fn to_edges(&self) -> Vec<Edge> {
        self.prerequisites.iter()
            .map(|prereq| Edge::new(
                &self.slug, prereq, Relationship::Prerequisite
            ))
            .collect()
    }
}

// Domain type → Vector document
impl From<&MyCard> for VectorDocument {
    fn from(card: &MyCard) -> Self {
        VectorDocument {
            id: card.slug.clone(),
            content: format!("{}\n{}", card.title, card.body),
            metadata: Default::default(),
        }
    }
}
```

---

## Step 9: Health Endpoint

Use `fabryk_mcp::health_router` for consistent health reporting:

```rust
let service_handles = vec![redis_svc, knowledge_svc, vector_svc];
let router = axum::Router::new()
    .merge(fabryk_mcp::health_router(service_handles));
```

See [mcp-health.md](./mcp-health.md) for full details on response format,
HTTP status codes, and testing.

---

## Step 10: Deployment

### Cloud Run Detection

Use `fabryk::core::deploy` to detect Cloud Run and adjust behavior:

```rust
pub use fabryk::core::deploy::{is_cloud_run, service_name};

// In server initialization:
let cloud_run = deploy::is_cloud_run();
if cloud_run {
    log::info!("Cloud Run detected — dev-only tools hidden");
} else {
    log::info!("Local development mode — all tools available");
}
```

### Environment Variables

Follow these conventions for Cloud Run compatibility:

| Variable | Purpose | Cloud Run |
|----------|---------|-----------|
| `PORT` | HTTP listen port | Set by Cloud Run |
| `K_SERVICE` | Service name | Set by Cloud Run |
| `K_REVISION` | Revision name | Set by Cloud Run |
| `MYSERVER_*` | App-specific config | Set in deploy |

---

## Refactoring an Existing Server

If you have an existing MCP server and want to migrate to Fabryk, follow
these steps in order. Each step is independently testable — run your tests
after each change.

### Phase 1: Add Fabryk Dependencies

```toml
# Add to Cargo.toml
fabryk = { version = "0.2", features = ["fts-tantivy", "vector-lancedb", "vector-fastembed"] }
fabryk-mcp = { version = "0.2", features = ["http"] }
fabryk-auth-google = "0.2"
```

### Phase 2: Replace Health Endpoint

Replace your hand-rolled `/health` handler with `fabryk_mcp::health_router`:

```rust
// Before:
let health = Router::new()
    .route("/health", get(|| async { "ok" }));

// After:
let health = fabryk_mcp::health_router(vec![]);
```

If you track service readiness, pass `ServiceHandle`s for per-service
visibility. See [mcp-health.md](./mcp-health.md).

### Phase 3: Adopt ServiceHandle Lifecycle

Replace boolean flags or ad-hoc readiness tracking with `ServiceHandle`:

```rust
// Before:
let redis_ready = Arc::new(AtomicBool::new(false));

// After:
let redis_svc = ServiceHandle::new("redis");
redis_svc.set_state(ServiceState::Starting);
// ... on success:
redis_svc.set_state(ServiceState::Ready);
```

### Phase 4: Wire Auth Middleware

Replace custom auth with `fabryk::auth::AuthLayer` +
`fabryk_auth_google::GoogleTokenValidator`:

```rust
// Before (custom):
.layer(middleware::from_fn(my_auth_check))

// After (fabryk):
let validator = Arc::new(GoogleTokenValidator::new(jwks_url));
let auth_layer = AuthLayer::new(validator, auth_config);
router.nest_service("/mcp", auth_layer.layer(mcp_service))
```

### Phase 5: Add OAuth2 Discovery Routes

MCP clients need RFC 9728/8414 discovery endpoints:

```rust
let router = axum::Router::new()
    .merge(fabryk_mcp::health_router(services))
    .merge(fabryk_mcp::auth::discovery_routes(
        &resource_url,
        "https://accounts.google.com",
    ))
    .nest_service("/mcp", authed_service);
```

### Phase 6: Migrate to OnceCell Pattern

Replace blocking initialization with deferred `OnceCell`:

```rust
// Before: blocks startup for 5s
let redis = RedisClient::new(&url).await?;
let server = MyServer { redis: Some(Arc::new(redis)) };

// After: starts transport immediately
let redis_cell = Arc::new(tokio::sync::OnceCell::new());
tokio::spawn(async move {
    if let Ok(client) = RedisClient::new(&url).await {
        let _ = redis_cell_bg.set(Arc::new(client) as Arc<dyn RedisOps>);
    }
});
let server = MyServer { redis: Some(redis_cell) };
```

### Phase 7: Extract Tool Logic

Move business logic from tool methods to standalone functions:

```rust
// Before: logic embedded in tool method
#[tool(description = "...")]
async fn my_tool(&self, params: Parameters<P>) -> Result<CallToolResult, McpError> {
    // 50 lines of business logic here
}

// After: thin method + standalone function
#[tool(description = "...")]
async fn my_tool(&self, params: Parameters<P>) -> Result<CallToolResult, McpError> {
    let engine = self.require_knowledge()?;
    match tools::my_domain::do_thing(engine, &params.0.query) {
        Ok(result) => Ok(CallToolResult::success(vec![Content::text(result)])),
        Err(e) => Ok(tools::error_to_tool_result(e)),
    }
}
```

### Phase 8: Add Structured Error Responses

Replace string errors with structured JSON including hints:

```rust
// Before:
Err(McpError::internal_error("table not found", None))

// After:
Ok(tools::error_to_tool_result(MyServerError::NotFound(
    format!("Table '{table}' not found")
)))
// → {"error_category": "not_found", "message": "...", "hint": "Use list_datasets..."}
```

---

## Pattern Reference

| Pattern | Where | Purpose |
|---------|-------|---------|
| `Arc<OnceCell<Arc<dyn Trait>>>` | lib.rs, server.rs | Deferred async service init |
| `ServiceHandle` lifecycle | lib.rs | Health reporting + readiness gating |
| `#[tool_router]` + `#[tool]` | server.rs | MCP tool registration via macros |
| `require_*()` guards | server.rs | Clear errors when services aren't ready |
| `tools/` delegation | server.rs → tools/*.rs | Testable business logic separation |
| `error_to_tool_result()` | tools/mod.rs | Structured error JSON with hints |
| `OptionalParts` extractor | server.rs | Auth identity in both HTTP and stdio |
| `DEV_ONLY_TOOLS` filtering | deploy.rs, server.rs | Additive security for production |
| `fire-and-forget spawn` | tools/query.rs | Non-blocking side effects |
| Domain adapters | knowledge/adapters.rs | Domain types → fabryk traits |
| `ConfigManager` impl | config.rs | CLI integration + multi-source loading |
| `fabryk_mcp::health_router` | lib.rs | Service-aware JSON health endpoint |
| `fabryk_mcp::auth::discovery_routes` | lib.rs | RFC 9728/8414 OAuth discovery |

## Related Documents

- [mcp-async-startup.md](./mcp-async-startup.md) — ServiceHandle lifecycle,
  retry, parallel wait, OnceCell patterns
- [mcp-health.md](./mcp-health.md) — Health endpoint setup, response format,
  testing
- [mcp-metadata.md](./mcp-metadata.md) — DiscoverableRegistry, ToolMeta,
  query strategies
- [mcp-with-claude-code.md](./mcp-with-claude-code.md) — Testing your MCP
  server with Claude Code

## Projects Using These Patterns

- **taproot** — BigQuery analytics MCP server (reference implementation)
- **ai-kasu** — Java/Quarkus knowledge MCP server (CompositeRegistry +
  ServiceAwareRegistry pattern)
- **keystone** — Activity intelligence MCP server (stateful router +
  webhook composition)
