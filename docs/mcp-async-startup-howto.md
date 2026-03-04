# MCP Async Startup Pattern

**How to start your MCP transport instantly while expensive services initialize in the background.**

## The Problem

MCP servers often depend on slow-to-initialize services: embedding model downloads (10-30s), Redis connections (1-5s), index builds, etc. If you wait for all services before starting the transport, clients see long connection timeouts — especially painful during development.

## The Pattern

1. **Start the transport immediately** after loading config and building fast components
2. **Use `ServiceHandle`** (from `fabryk_core::service`) to track lifecycle of each background service
3. **Use `tokio::sync::OnceCell`** for services that initialize asynchronously — the cell is shared via `Arc` before being populated
4. **Guard tool access** with `require_*()` methods that return clear "not ready yet" errors
5. **Expose a `/health` endpoint** via `fabryk_mcp::health_router(services)` — no need to reimplement per-project

## ServiceHandle Lifecycle

```
Stopped ──► Starting ──► Ready
                │
                └──► Failed(reason)
```

Every transition is recorded in an **audit trail** with monotonic timestamps, accessible via `handle.transitions()`.

- **Stopped**: Not configured (e.g., Redis URL empty). Health considers this "ok".
- **Starting**: Background task is running. Health returns 503.
- **Ready**: Fully operational. Health returns 200.
- **Failed**: Initialization failed. Health returns 503. Tools return clear errors.

## Fabryk API Reference

### fabryk_core::service

| API | Purpose |
|-----|---------|
| `ServiceHandle::new(name)` | Create a handle (initial state: Stopped) |
| `handle.set_state(state)` | Update state, broadcast to subscribers, record in audit trail |
| `handle.state()` | Get current state |
| `handle.transitions()` | Get full audit trail with timestamps |
| `handle.wait_ready(timeout)` | Await Ready/Failed/timeout for one service |
| `wait_all_ready(&[handles], timeout)` | Await all services **in parallel** via `futures::join_all` |
| `spawn_with_retry(handle, config, init_fn)` | Background task with exponential backoff retry |
| `RetryConfig { max_attempts, initial_delay, max_delay }` | Configuration for retry behaviour |

### fabryk_mcp::health_router (requires `http` feature)

| API | Purpose |
|-----|---------|
| `health_router(services)` | Build an axum `Router` with `/health` endpoint |
| `ServiceHealthResponse` | JSON response struct (`status`, `services[]`) |
| `ServiceStatus` | Per-service entry (`name`, `state`) |

## Code Examples

### Creating Service Handles

```rust
use fabryk_core::service::{ServiceHandle, ServiceState};

let redis_svc = ServiceHandle::new("redis");
let vector_svc = ServiceHandle::new("vector");
let knowledge_svc = ServiceHandle::new("knowledge");
```

### OnceCell + Background Task (Redis Example)

```rust
use std::sync::Arc;
use tokio::sync::OnceCell;

// Create the cell before starting the transport
let redis_cell: Arc<OnceCell<Arc<dyn RedisOps>>> = Arc::new(OnceCell::new());

// Spawn background connection
let cell_bg = redis_cell.clone();
let svc = redis_svc.clone();
redis_svc.set_state(ServiceState::Starting);

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
```

### Background Task with Retry

For transient failures (network timeouts, DNS hiccups), use `spawn_with_retry`:

```rust
use fabryk_core::service::{spawn_with_retry, RetryConfig};

let svc = ServiceHandle::new("redis");
let cell = Arc::new(OnceCell::new());
let cell_bg = cell.clone();
let url = config.redis.url.clone();

spawn_with_retry(
    svc.clone(),
    RetryConfig {
        max_attempts: 5,
        initial_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
    },
    move || {
        let cell = cell_bg.clone();
        let url = url.clone();
        async move {
            let client = RedisClient::new(&url).await.map_err(|e| format!("{e}"))?;
            cell.set(Arc::new(client) as Arc<dyn RedisOps>).ok();
            Ok(())
        }
    },
);
```

### Parallel Service Wait

Wait for all services concurrently — wall-clock time equals the **slowest** service:

```rust
use fabryk_core::service::wait_all_ready;

// Wait up to 30s for all services to be ready
if let Err(errors) = wait_all_ready(&service_handles, Duration::from_secs(30)).await {
    for e in &errors {
        log::error!("Service failed: {e}");
    }
}
```

### OnceCell for Struct Fields (Vector Engine Example)

When a struct is shared via `Arc` before a sub-component is ready, use `OnceCell` instead of `Option` to avoid needing `&mut self`:

```rust
pub struct KnowledgeEngine {
    provider: InMemoryProvider,
    fts: FtsEngine,
    // OnceCell: populated by background task, no &mut needed
    vector: Arc<tokio::sync::OnceCell<VectorEngine>>,
}

impl KnowledgeEngine {
    pub fn set_vector(&self, v: VectorEngine) {
        let _ = self.vector.set(v);
    }

    pub fn require_vector(&self) -> Result<&VectorEngine, Error> {
        self.vector.get().ok_or_else(|| {
            Error::new("vector engine still loading")
        })
    }
}
```

### require_*() Guards

Tools call `require_redis()`, `require_knowledge()`, etc. These return clear errors when a service isn't ready:

```rust
fn require_redis(&self) -> Result<&dyn RedisOps, Error> {
    let cell = self.redis.as_ref()
        .ok_or_else(|| Error::config("Redis is not configured"))?;
    cell.get()
        .map(|arc| arc.as_ref())
        .ok_or_else(|| Error::config("Redis is still connecting"))
}
```

### Health Endpoint (fabryk_mcp)

Instead of reimplementing `/health` in every project, use the shared router:

```rust
use fabryk_mcp::health_router;

let router = axum::Router::new()
    .merge(health_router(service_handles.clone()))
    .merge(discovery_routes(&resource_url, "https://accounts.google.com"))
    .nest_service("/mcp", authed_service);
```

Response example:
```json
{
  "status": "ok",
  "services": [
    {"name": "redis", "state": "ready"},
    {"name": "knowledge", "state": "ready"},
    {"name": "vector", "state": "starting"}
  ]
}
```

### Transition Audit Trail

Every `set_state` call is recorded with a monotonic timestamp:

```rust
let svc = ServiceHandle::new("redis");
svc.set_state(ServiceState::Starting);
// ... some time passes ...
svc.set_state(ServiceState::Ready);

for t in svc.transitions() {
    println!("{}: {} (at {:?})", svc.name(), t.state, t.elapsed);
}
// redis: stopped (at 0ns)
// redis: starting (at 1.234ms)
// redis: ready (at 523.456ms)
```

### Worker Adaptation

Background workers that depend on an OnceCell service check availability each cycle:

```rust
async fn worker_loop(
    redis: Arc<OnceCell<Arc<dyn RedisOps>>>,
    ...,
) {
    loop {
        match redis.get() {
            Some(client) => { /* do work */ }
            None => {
                log::debug!("Redis not yet connected, skipping cycle");
            }
        }
        tokio::time::sleep(interval).await;
    }
}
```

## Startup Flow Summary

```
1. Config + logging                         (sync, <100ms)
2. Create ServiceHandle instances           (instant)
3. BQ OnceCell                              (deferred to first use)
4. Redis OnceCell + spawn background task   (non-blocking, with retry)
5. KnowledgeEngine::build()                 (sync, <1s)
   → knowledge_svc → Ready
6. Spawn vector engine background task      (10-30s in background)
   → vector_svc → Ready when done
7. UserModelEngine + worker                 (worker checks Redis each cycle)
8. START TRANSPORT IMMEDIATELY              (<2s total)
```

## Testing Strategies

1. **OnceCell in tests**: Create pre-populated cells for unit tests:
   ```rust
   let cell = Arc::new(OnceCell::new());
   cell.set(Arc::new(MockRedis::new()) as Arc<dyn RedisOps>).ok();
   ```

2. **Health endpoint tests**: Use `axum::Router::oneshot()` with different `ServiceState` combinations:
   ```rust
   let handles = vec![
       make_handle("redis", ServiceState::Ready),
       make_handle("vector", ServiceState::Starting),
   ];
   let app = health_router(handles);
   let resp = app.oneshot(Request::get("/health").body(Body::empty()).unwrap()).await.unwrap();
   assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
   ```

3. **Worker tests**: `poll_and_process()` takes `&dyn RedisOps` directly — test it without the OnceCell wrapper.

4. **Retry tests**: Use `spawn_with_retry` with an `AtomicU32` counter to simulate flaky init.

5. **Transition audit**: Assert exact transition sequences and timestamp ordering.

## Projects Using This Pattern

- **ai-kasu**: `CompositeRegistry` + `ServiceAwareRegistry` with `ServiceHandle` lifecycle tracking
- **taproot**: `OnceCell` fields + `require_*()` guards + `fabryk_mcp::health_router`
