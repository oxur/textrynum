---
number: 2
title: "ECL Library Research & Recommendations"
author: "Duncan McGreggor"
component: All
tags: [change-me]
created: 2026-01-15
updated: 2026-01-15
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# ECL Library Research & Recommendations

## Rust Ecosystem Analysis for AI Workflow Orchestration

**Version**: 1.0
**Date**: January 2026
**Status**: Research Complete

---

## Overview

This document details the research findings for each component of the ECL architecture, providing specific library recommendations with justifications based on:

- Maintenance status and community health
- Production readiness and adoption
- Tokio compatibility
- Feature completeness for our requirements
- Documentation quality

---

## 1. Workflow Orchestration: Restate

### Recommendation: **Restate** (Primary)

| Attribute | Value |
|-----------|-------|
| Repository | github.com/restatedev/restate |
| GitHub Stars | ~3,400 |
| License | BSL 1.1 (permissive, Amazon defense) |
| Rust SDK | github.com/restatedev/sdk-rust |
| SDK Version | 0.7.0 (as of Jan 2026) |
| Tokio Support | Native |

### Why Restate?

**Durable Execution Model**: Restate's journal-based approach means every `ctx.run()` call is persisted before execution and replayed on recovery. This is exactly what we need for LLM calls—expensive operations that must not be re-executed.

**Workflow Primitives**: The SDK provides three service types:

- **Services**: Stateless handlers
- **Virtual Objects**: Stateful entities with K/V state and single-writer guarantee
- **Workflows**: Run-once execution with query/signal handlers

The **Workflow** type maps directly to our needs:

```rust
#[restate_sdk::workflow]
pub trait AgentWorkflow {
    async fn run(input: WorkflowInput) -> Result<WorkflowOutput, HandlerError>;

    #[shared]
    async fn request_revision(feedback: Feedback) -> Result<(), HandlerError>;

    #[shared]
    async fn get_status() -> Result<WorkflowStatus, HandlerError>;
}
```

**Durable Promises for Feedback Loops**: This is the key feature for managed serialism:

```rust
// In Step 1: wait for feedback
let feedback = ctx.promise::<Feedback>("revision_request").await?;

// In Step 2: send feedback
ctx.resolve_promise::<Feedback>("revision_request", feedback);
```

**Operational Simplicity**:

- Single binary, no external dependencies
- Embedded persistence (RocksDB-based journal)
- HTTP admin API for introspection
- SQL interface for querying state

### Alternatives Considered

| Library | Reason Not Selected |
|---------|---------------------|
| Temporal | Rust SDK in alpha, no official production timeline |
| Acts | Lower adoption (54 stars), less mature |
| Custom (Ractor + SQLx) | Would require 1000+ lines of orchestration code |

### Risk Assessment

- **SDK Maturity**: Rust SDK is newer than TypeScript/Java SDKs, but actively maintained
- **BSL License**: Converts to Apache 2.0 after 4 years; only restricts competing managed services
- **External Process**: Requires running `restate-server` alongside application

**Mitigation**: Start with Restate, but keep step implementations behind trait abstractions to allow future migration if needed.

---

## 2. LLM Integration: llm Crate (with async-anthropic as alternative)

### Primary Recommendation: **llm** crate

| Attribute | Value |
|-----------|-------|
| Repository | github.com/graniet/llm |
| Crates.io | crates.io/crates/llm |
| Downloads | ~52,000 total |
| License | MIT |
| Anthropic Support | Yes (Claude 3.5 Sonnet, Opus, Haiku) |

### Why llm?

**Multi-Provider Abstraction**: Clean separation between provider and usage:

```rust
let client = LLMBuilder::new()
    .backend(LLMBackend::Anthropic)
    .api_key(&api_key)
    .model("claude-sonnet-4-20250514")
    .build()?;

// Same interface regardless of backend
let response = client.chat(&messages).await?;
```

**Feature Coverage**:

- Streaming responses
- Tool/function calling
- Built-in retry with exponential backoff
- Token counting
- Conversation history management

**Extensibility**: If we ever need to add OpenAI or local models for testing, same interface works.

### Alternative: **async-anthropic**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/bosun-ai/async-anthropic |
| Downloads | ~26,000 |
| License | MIT |
| Streaming | Yes |
| Tools | Yes |

**When to use instead**: If we need maximum control over Anthropic-specific features or encounter limitations in the `llm` crate's Anthropic implementation.

### Implementation Notes

Regardless of which library:

1. **Wrap in trait abstraction**:

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse>;
    async fn complete_stream(&self, request: CompletionRequest) -> BoxStream<'_, Result<StreamChunk>>;
}
```

1. **Always use ctx.run() for LLM calls**:

```rust
let response = ctx.run(|| async {
    llm_client.complete(request).await
}).await?;
```

This ensures Restate journals the result and won't re-call Claude on recovery.

---

## 3. Database Abstraction: SQLx

### Recommendation: **SQLx**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/launchbadge/sqlx |
| GitHub Stars | ~13,000+ |
| Downloads | ~55 million |
| License | MIT/Apache-2.0 |
| Async Runtime | Tokio (native) |

### Why SQLx?

**Compile-Time Query Checking**: The `query!()` macro validates SQL against your actual database schema at compile time:

```rust
let workflow = sqlx::query_as!(
    WorkflowRecord,
    "SELECT * FROM workflows WHERE id = $1",
    workflow_id
)
.fetch_one(&pool)
.await?;
```

**Multi-Backend Support**: Same code works across:

- SQLite (development, testing)
- PostgreSQL (production)
- MySQL (if ever needed)

**In-Memory SQLite**: Perfect for fast test suites:

```rust
let pool = SqlitePool::connect("sqlite::memory:").await?;
```

**Runtime Backend Switching**: The `Any` driver allows runtime selection:

```rust
// Enable "any" feature
let pool = AnyPool::connect(&database_url).await?;
```

### Repository Pattern

We'll implement a repository trait for clean separation:

```rust
#[async_trait]
pub trait WorkflowRepository: Send + Sync {
    async fn create(&self, workflow: &NewWorkflow) -> Result<WorkflowId>;
    async fn get(&self, id: WorkflowId) -> Result<Option<Workflow>>;
    async fn update_status(&self, id: WorkflowId, status: WorkflowStatus) -> Result<()>;
    async fn list_by_status(&self, status: WorkflowStatus) -> Result<Vec<Workflow>>;
}

// Implementations
pub struct SqliteWorkflowRepository { pool: SqlitePool }
pub struct PostgresWorkflowRepository { pool: PgPool }
```

### Migrations

Use `sqlx-cli` for schema migrations:

```bash
sqlx migrate add create_workflows_table
sqlx migrate run
```

### Alternatives Considered

| Library | Reason Not Selected |
|---------|---------------------|
| Diesel | No async SQLite support in diesel-async |
| SeaORM | Higher abstraction than needed; adds complexity |
| rusqlite | Sync-only; would need spawn_blocking |

---

## 4. Retry & Resilience: backon + failsafe

### Retry Logic: **backon**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/Xuanwo/backon |
| Downloads | ~886,000/month |
| License | Apache-2.0 |
| Project | Part of OpenDAL |

**Ergonomic async-first API**:

```rust
use backon::{ExponentialBuilder, Retryable};

let response = call_claude_api
    .retry(ExponentialBuilder::default()
        .with_max_times(5)
        .with_max_delay(Duration::from_secs(30)))
    .when(|e| e.is_retryable())  // Conditional retry
    .notify(|err, dur| {
        tracing::warn!("Retry in {:?}: {}", dur, err);
    })
    .await?;
```

### Circuit Breaker: **failsafe**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/dmexe/failsafe-rs |
| License | MIT |
| Async Support | Yes |

**Protect against cascading failures**:

```rust
use failsafe::{Config, CircuitBreaker};

let circuit_breaker = Config::new()
    .failure_policy(consecutive_failures(5, Duration::from_secs(30)))
    .success_policy(consecutive_successes(3))
    .build();

let result = circuit_breaker.call_async(|| {
    call_claude_api(request)
}).await;
```

### Integration with Restate

Note: Restate provides its own retry mechanism via `ctx.run()`. Use `backon` and `failsafe` for:

- LLM calls **within** a `ctx.run()` block
- External service calls that need finer-grained control than Restate's defaults

---

## 5. Configuration: figment

### Recommendation: **figment**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/SergioBenitez/Figment |
| Downloads | ~17.3 million |
| Stars | 861 |
| License | MIT/Apache-2.0 |
| Creator | Sergio Benitez (Rocket framework) |

**Hierarchical configuration with excellent error provenance**:

```rust
use figment::{Figment, providers::{Env, Toml, Format}};

#[derive(Deserialize)]
struct Config {
    restate: RestateConfig,
    llm: LLMConfig,
    database: DatabaseConfig,
}

let config: Config = Figment::new()
    .merge(Toml::file("ecl.toml"))
    .merge(Toml::file("ecl.local.toml"))  // Local overrides
    .merge(Env::prefixed("ECL_"))          // Environment variables
    .extract()?;
```

**Error messages show exact source**:

```
Error: invalid type: found string "abc", expected u16 for key "restate.port"
       in ECL_RESTATE_PORT environment variable
```

---

## 6. Serialization & Schema: serde + schemars

### Serialization: **serde** (standard)

Universal in Rust ecosystem. All our libraries (Restate, SQLx, figment) use it.

### Schema Generation: **schemars**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/GREsau/schemars |
| Downloads | High |
| License | MIT |

**Generate JSON Schema from Rust types**:

```rust
use schemars::JsonSchema;

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub files: Vec<PathBuf>,
    pub instructions: String,
    #[schemars(range(min = 1, max = 10))]
    pub max_iterations: u32,
}

// Generate schema for documentation/validation
let schema = schemars::schema_for!(WorkflowInput);
```

Useful for:

- API documentation
- Input validation
- External tool integration

---

## 7. Observability: tracing ecosystem

### Structured Logging: **tracing**

| Attribute | Value |
|-----------|-------|
| Repository | github.com/tokio-rs/tracing |
| Status | Standard in Tokio ecosystem |

**Span-based instrumentation**:

```rust
#[tracing::instrument(skip(ctx, input))]
async fn execute_step(
    ctx: &WorkflowContext<'_>,
    input: StepInput,
) -> Result<StepOutput> {
    tracing::info!(step_id = %input.step_id, "Starting step execution");
    // ...
}
```

### Restate Integration

Restate SDK uses `tracing` internally. Configure with `ReplayAwareFilter` to suppress logs during replay:

```rust
use restate_sdk::filter::ReplayAwareFilter;

tracing_subscriber::fmt()
    .with_env_filter(ReplayAwareFilter::new(EnvFilter::from_default_env()))
    .init();
```

---

## 8. Inter-Component Communication: Tokio Channels

For communication **within** our service (not between Restate services), use Tokio's built-in channels:

### Patterns

| Pattern | Channel Type |
|---------|--------------|
| Request-Response | `oneshot` |
| Multiple producers, single consumer | `mpsc` |
| Broadcast to multiple consumers | `broadcast` |
| Latest value only | `watch` |

**Example: Step-internal feedback handling**:

```rust
use tokio::sync::{mpsc, oneshot};

enum StepCommand {
    Execute {
        input: StepInput,
        respond_to: oneshot::Sender<StepOutput>,
    },
    Revise {
        feedback: Feedback,
        respond_to: oneshot::Sender<StepOutput>,
    },
}
```

For **cross-step** communication, use Restate's Durable Promises instead.

---

## Summary: Recommended Stack

| Layer | Library | Version | Purpose |
|-------|---------|---------|---------|
| **Orchestration** | restate-sdk | 0.7.x | Durable workflow execution |
| **LLM Integration** | llm | latest | Claude API abstraction |
| **Database** | sqlx | 0.8.x | Async SQL with compile-time checks |
| **Retry** | backon | latest | Async retry with backoff |
| **Circuit Breaker** | failsafe | latest | Failure protection |
| **Configuration** | figment | 0.10.x | Hierarchical config |
| **Serialization** | serde + serde_json | 1.x | JSON serialization |
| **Schema** | schemars | 1.x | JSON Schema generation |
| **Logging** | tracing + tracing-subscriber | 0.1.x | Structured observability |
| **Async Runtime** | tokio | 1.x | Async foundation |

---

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────┐
│                           ECL                                   │
├─────────────────────────────────────────────────────────────────┤
│  restate-sdk ─────────────────────────────────────────────────┐ │
│       │                                                       │ │
│       ├── tokio (async runtime)                               │ │
│       ├── serde (serialization)                               │ │
│       └── tracing (observability)                             │ │
├─────────────────────────────────────────────────────────────────┤
│  llm ─────────────────────────────────────────────────────────┤ │
│       │                                                       │ │
│       ├── reqwest (HTTP client)                               │ │
│       ├── serde_json                                          │ │
│       └── tokio                                               │ │
├─────────────────────────────────────────────────────────────────┤
│  sqlx ────────────────────────────────────────────────────────┤ │
│       │                                                       │ │
│       ├── tokio                                               │ │
│       └── (sqlite/postgres features)                          │ │
├─────────────────────────────────────────────────────────────────┤
│  figment ─────────────────────────────────────────────────────┤ │
│       │                                                       │ │
│       └── serde                                               │ │
├─────────────────────────────────────────────────────────────────┤
│  backon + failsafe ───────────────────────────────────────────┤ │
│       │                                                       │ │
│       └── tokio                                               │ │
└─────────────────────────────────────────────────────────────────┘
```

---

## Cargo.toml Dependencies (Proposed)

```toml
[dependencies]
# Orchestration
restate-sdk = { version = "0.7", features = ["http_server"] }

# LLM
llm = "0.1"  # Or async-anthropic = "0.6"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "postgres"] }

# Resilience
backon = "1.0"
failsafe = "1.0"

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
schemars = "1.0"

# Observability
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Async
tokio = { version = "1", features = ["full"] }

# Error handling
thiserror = "2.0"
anyhow = "1.0"
```

---

## Next Steps

1. **Prototype Validation**: Build minimal workflow with Restate + Claude integration
2. **Performance Testing**: Measure Restate overhead for LLM-bound workloads
3. **API Surface Design**: Define public types and workflow submission interface
