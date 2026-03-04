# MCP Async Startup Pattern

**How to start your MCP transport instantly while expensive services initialize in the background.**

## The Problem

MCP servers often depend on slow-to-initialize services: embedding model downloads (10-30s), Redis connections (1-5s), index builds, etc. If you wait for all services before starting the transport, clients see long connection timeouts — especially painful during development.

## The Pattern

1. **Start the transport immediately** after loading config and building fast components
2. **Use `ServiceHandle`** (from `fabryk_core::service`) to track lifecycle of each background service
3. **Use `tokio::sync::OnceCell`** for services that initialize asynchronously — the cell is shared via `Arc` before being populated
4. **Guard tool access** with `require_*()` methods that return clear "not ready yet" errors
5. **Expose a `/health` endpoint** that reports per-service state

## ServiceHandle Lifecycle

```
Stopped ──► Starting ──► Ready
                │
                └──► Failed(reason)
```

- **Stopped**: Not configured (e.g., Redis URL empty). Health considers this "ok".
- **Starting**: Background task is running. Health returns 503.
- **Ready**: Fully operational. Health returns 200.
- **Failed**: Initialization failed. Health returns 503. Tools return clear errors.

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

### Health Endpoint

```rust
fn health_handler(services: Vec<ServiceHandle>) -> impl IntoResponse {
    let all_ready = services.iter().all(|h| {
        let s = h.state();
        s.is_ready() || s == ServiceState::Stopped
    });

    let status_code = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    // Return JSON with per-service state
    (status_code, Json(HealthResponse { ... }))
}
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
4. Redis OnceCell + spawn background task   (non-blocking)
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

2. **Health endpoint tests**: Use `axum::Router::oneshot()` with different `ServiceState` combinations.

3. **Worker tests**: `poll_and_process()` takes `&dyn RedisOps` directly — test it without the OnceCell wrapper.

## Projects Using This Pattern

- **ai-kasu**: `CompositeRegistry` + `ServiceAwareRegistry` with `ServiceHandle` lifecycle tracking
- **taproot**: `OnceCell` fields + `require_*()` guards + `/health` endpoint
