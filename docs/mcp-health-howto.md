# MCP Health Endpoint How-To

**How to add a service-aware `/health` endpoint to your Fabryk MCP server.**

## The Problem

Every MCP server deployed to Cloud Run (or behind a load balancer) needs a
health endpoint. Without one, orchestrators can't distinguish between "server
is booting" and "server is broken". Hand-rolling this per project leads to
inconsistent response formats, duplicated code, and forgotten edge cases
(what does "healthy" mean when Redis is optional?).

## The Solution

`fabryk_mcp::health_router` provides a single-function, drop-in axum `Router`
that:

- Returns JSON with per-service state
- Uses proper HTTP status codes (200 vs 503)
- Treats unconfigured services (`Stopped`) as healthy
- Works with any router state type (`Router<()>`, `Router<AppState>`, etc.)

## Prerequisites

Enable the `http` feature on `fabryk-mcp`:

```toml
fabryk-mcp = { version = "0.1", features = ["http"] }
```

If you track background services, you also need `fabryk-core` for `ServiceHandle`:

```toml
fabryk-core = "0.1"
```

## Quick Start (No Services)

The simplest case — a health endpoint that always returns 200:

```rust
use fabryk_mcp::health_router;

let router = axum::Router::new()
    .merge(health_router(vec![]))
    .nest_service("/mcp", mcp_service);
```

```sh
$ curl localhost:8080/health
{"status":"ok","services":[]}
```

## With Service Tracking

When your server has background services (Redis, vector engine, FTS index),
pass their `ServiceHandle`s to get per-service visibility:

```rust
use fabryk_core::service::{ServiceHandle, ServiceState};
use fabryk_mcp::health_router;

// Create handles (initial state: Stopped)
let redis_svc = ServiceHandle::new("redis");
let vector_svc = ServiceHandle::new("vector");

// Background tasks update state as they progress
redis_svc.set_state(ServiceState::Starting);
tokio::spawn({
    let svc = redis_svc.clone();
    async move {
        match connect_redis().await {
            Ok(_) => svc.set_state(ServiceState::Ready),
            Err(e) => svc.set_state(ServiceState::Failed(format!("{e}"))),
        }
    }
});

// Pass handles to the health router
let services = vec![redis_svc, vector_svc.clone()];
let router = axum::Router::new()
    .merge(health_router(services))
    .nest_service("/mcp", mcp_service);
```

While Redis is connecting:

```json
HTTP/1.1 503 Service Unavailable

{
  "status": "starting",
  "services": [
    {"name": "redis", "state": "starting"},
    {"name": "vector", "state": "stopped"}
  ]
}
```

After all services are ready (or stopped/unconfigured):

```json
HTTP/1.1 200 OK

{
  "status": "ok",
  "services": [
    {"name": "redis", "state": "ready"},
    {"name": "vector", "state": "stopped"}
  ]
}
```

## Composing with Auth and Discovery Routes

A typical production router combines health, OAuth2 discovery, and the
auth-protected MCP service:

```rust
use fabryk_auth::AuthLayer;
use fabryk_auth_mcp::discovery_routes;
use fabryk_mcp::health_router;

let router = axum::Router::new()
    .merge(health_router(services))
    .merge(discovery_routes(&resource_url, "https://accounts.google.com"))
    .nest_service("/mcp", auth_layer.layer(mcp_service));
```

The health and discovery routes remain **unauthenticated** — only `/mcp` sits
behind the auth middleware. This lets Cloud Run health checks and OAuth2
client discovery work without credentials.

## Composing with Stateful Routers

`health_router` is generic over the axum state type, so it merges cleanly
into routers that carry application state:

```rust
use crate::state::AppState;

// webhooks use State<AppState> extractors
let webhooks = Router::new()
    .route("/webhooks/slack", post(handle_slack_event));

Router::new()
    .merge(webhooks)                    // Router<AppState>
    .merge(mcp)                         // Router<AppState>
    .merge(health_router(vec![]))       // Router<S> — unifies with AppState
    .layer(cors)
    .with_state(state)
```

## Response Format

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `status` | `"ok"` or `"starting"` | Overall health summary |
| `services` | array | Per-service entries |
| `services[].name` | string | Service name (e.g. `"redis"`) |
| `services[].state` | string | Current state as human-readable string |

### HTTP Status Codes

| Condition | Status | `status` field |
|-----------|--------|----------------|
| All services `Ready` or `Stopped` | **200** | `"ok"` |
| Any service `Starting` | **503** | `"starting"` |
| Any service `Failed(reason)` | **503** | `"starting"` |
| No services (empty list) | **200** | `"ok"` |

### ServiceState Semantics

| State | Meaning | Health impact |
|-------|---------|---------------|
| `Stopped` | Not configured (e.g. Redis URL is empty) | Healthy (200) |
| `Starting` | Background init in progress | Unhealthy (503) |
| `Ready` | Fully operational | Healthy (200) |
| `Failed(reason)` | Init failed permanently | Unhealthy (503) |

## API Reference

### fabryk_mcp (requires `http` feature)

| API | Purpose |
|-----|---------|
| `health_router(services)` | Build axum `Router` with `/health` endpoint |
| `ServiceHealthResponse` | JSON body struct (`status`, `services`) |
| `ServiceStatus` | Per-service entry struct (`name`, `state`) |

### fabryk_core::service

| API | Purpose |
|-----|---------|
| `ServiceHandle::new(name)` | Create handle (initial state: Stopped) |
| `handle.set_state(state)` | Update state, broadcast, record in audit trail |
| `handle.state()` | Get current state |

See [mcp-async-startup-howto.md](./mcp-async-startup-howto.md) for the full
`ServiceHandle` lifecycle, retry patterns, and parallel wait APIs.

## FabrykMcpServer Built-in Usage

If you use `FabrykMcpServer::serve_http()` directly (without building a
custom router), the health endpoint is included automatically:

```rust
let server = FabrykMcpServer::new(registry)
    .with_name("my-server")
    .with_version("0.1.0")
    .with_services(vec![redis_svc, vector_svc]);

// serve_http merges health_router internally
server.serve_http(addr).await?;
```

Use `into_http_service()` instead when you need a custom router (auth
middleware, additional routes, CORS, etc.).

## Testing

Test the health endpoint using `axum::Router::oneshot()`:

```rust
use axum::body::Body;
use axum::http::Request;
use fabryk_core::service::{ServiceHandle, ServiceState};
use fabryk_mcp::health_router;
use tower::ServiceExt;

#[tokio::test]
async fn test_health_returns_200_when_all_ready() {
    let redis = ServiceHandle::new("redis");
    redis.set_state(ServiceState::Ready);

    let app = health_router(vec![redis]);
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_health_returns_503_when_starting() {
    let redis = ServiceHandle::new("redis");
    redis.set_state(ServiceState::Starting);

    let app = health_router(vec![redis]);
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(resp.status(), 503);
    let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "starting");
}
```

## Projects Using This Pattern

- **taproot**: `health_router(service_handles)` merged with OAuth discovery routes
- **ai-kasu**: `health_router(services)` in `build_discovery_router()`, gating graph/FTS/vector services
- **keystone**: `health_router(vec![])` — no background services yet, ready for future additions
- **fabryk-mcp**: `serve_http()` uses `health_router` internally for the default router
