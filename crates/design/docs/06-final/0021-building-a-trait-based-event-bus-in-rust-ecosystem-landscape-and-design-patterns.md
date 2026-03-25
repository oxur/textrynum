---
number: 21
title: "Building a trait-based event bus in Rust: ecosystem landscape and design patterns"
author: "hseeberger offers"
component: All
tags: [change-me]
created: 2026-03-25
updated: 2026-03-25
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Building a trait-based event bus in Rust: ecosystem landscape and design patterns

**No single Rust crate provides a production-ready, trait-abstracted, backend-swappable event bus — but the ecosystem offers exceptional building blocks.** The strongest design foundation combines actix's `Handler<M>` typed dispatch pattern, tower's `Layer` middleware composition, and object_store's trait-based backend abstraction with feature-gated dependencies. The recommended GCP Pub/Sub client is `gcloud-pubsub` (yoshidan's battle-tested library, now renamed after donating its crate name to Google), while `pgmq` offers a compelling Postgres-backed middle ground. Starting with `tokio::sync::broadcast` channels behind an `EventBus` trait, with serde derives on event enums from day one, creates the cleanest migration path to distributed backends.

---

## The event bus crate landscape is rich in inspiration but poor in direct solutions

A thorough survey of crates.io and GitHub reveals roughly a dozen Rust event bus libraries, none of which combine typed routing, async/tokio support, trait abstraction, and backend swappability. The ecosystem splits into three tiers: well-maintained heavyweight frameworks (actix, tower) that solve adjacent problems, moderately maintained event-specific crates (eventador, messagebus, event_bus_rs) that lack abstraction, and abandoned proof-of-concepts.

**Actix** provides the gold standard for typed message dispatch. Its `Message` trait with an associated `Result` type and `Handler<M>` trait enable compile-time-checked routing where an actor implements `Handler<M>` for each message type it handles. The pattern `impl Handler<MyEvent> for MyProcessor` creates zero-overhead static dispatch. The derive macro `#[derive(Message)] #[rtype(result = "()")]` adds ergonomics. The key limitation: actix's actor model conflates message routing with state encapsulation and requires its own Arbiter runtime — too heavyweight for just an event bus. Extract the trait pattern, not the framework.

**Tower** is essential for the middleware layer. Its `Service<Request>` trait with `poll_ready()` for backpressure and `Layer` trait for composable middleware (retry, timeout, rate limiting, logging) represents **347 million downloads** of battle-tested abstraction. The `ServiceBuilder` pattern — `ServiceBuilder::new().timeout(30s).retry(policy).service(handler)` — should be adapted for wrapping event handlers. Tower's separation of `tower-service` (trait-only crate) from `tower` (middleware implementations) is a crate hierarchy pattern worth emulating.

**Eventador** (~23K downloads) demonstrates TypeId-based event routing on a lock-free LMAX Disruptor-inspired ring buffer, with feature-gated async support producing `Stream`/`Sink` implementations. Its limitation — enum variants share a parent enum's TypeId, forcing subscribers to filter variants manually — is a cautionary design note. **Messagebus** (~56K downloads) uses tokio with kanal high-performance channels and dashmap for typed inter-module messaging, but only **5.36% documentation coverage** makes it unusable. **event_bus_rs** is the cleanest minimal implementation (type = topic, `async_broadcast` channels, runtime-agnostic), worth studying for its simplicity.

The **disruptor** crate (833 GitHub stars) faithfully implements the LMAX Disruptor pattern with pre-allocated ring buffers, configurable `WaitStrategy` policies (BusySpin, BlockingWait, Yielding), and batch publishing. It's closure-based rather than trait-based and synchronous-only, but the `disruptor-rs` fork adds an `EventHandler<E>` trait that's more idiomatic.

---

## GCP Pub/Sub: yoshidan's library leads today, Google's official SDK approaches GA

The GCP Pub/Sub Rust ecosystem underwent a significant transition in early 2025: **yoshidan donated the `google-cloud-pubsub` crate name to Google**, with his code continuing under `gcloud-pubsub`. This means version ≤0.25.x was the community implementation while ≥0.26.x is Google's official preview SDK.

**`gcloud-pubsub`** (v1.6.0, ~3.5M downloads under old name) is the recommended production choice. It offers batched publishing, StreamingPull subscription (both callback and `futures::Stream` APIs), per-message ack/nack, message ordering via `ordering_key`, dead letter queue configuration, and emulator support via `PUBSUB_EMULATOR_HOST`. RisingWave uses it in production and contributed batch acknowledgment and seek features upstream. The `Client → Topic → Publisher` and `Client → Subscription` hierarchy maps cleanly to a trait-based design:

```rust
// Publishing
let publisher = client.topic("events").new_publisher(None);
let awaiter = publisher.publish(PubsubMessage { data: payload, ..Default::default() }).await;
let id = awaiter.get().await?;

// Subscribing (stream-based)  
let mut stream = subscription.subscribe(None).await?;
while let Some(message) = stream.next().await { message.ack().await?; }
```

Messages use `Vec<u8>` for data — serde serialization is the consumer's responsibility, which actually works well for a generic `EventBus` trait since the trait can be serialization-agnostic at the transport level.

**Google's official SDK** (`google-cloud-pubsub` v0.33.0+, from `googleapis/google-cloud-rust`) was announced September 2025 covering 140+ APIs. As of March 2026, publisher support is functional with batching and ordering, and subscriber streaming was added in late 2025. However, **breaking changes are still happening** — the March 2026 release includes renamed types and moved modules. The team acknowledges: "We do not recommend that you use this crate in production." Plan to migrate when it reaches GA (expected late 2026), since the API patterns are similar enough to make the switch straightforward.

**`gcloud-sdk`** by abdolence (~3M downloads) provides auto-generated low-level gRPC bindings for all Google APIs, but requires manual protobuf handling — too low-level for a clean EventBus abstraction. **`pub-sub-client`** by hseeberger offers the best built-in serde integration (typed publish/pull with automatic JSON serialization) but uses HTTP-only pull (no streaming), limited authentication (service account keys only), and has just 4 GitHub stars.

---

## Postgres as a middle-ground backend: pgmq for durability, PgListener for real-time

Since the target system already has Postgres, two complementary approaches create a middle-ground backend between in-memory and cloud pub/sub.

**pgmq** (by Tembo, v0.32.1) implements durable message queue semantics on Postgres — essentially an SQS-like system using database tables rather than LISTEN/NOTIFY. Messages persist through disconnections, support visibility timeouts, batch reads, long polling, and archival. It provides **generic `Message<T>` where T: Deserialize** with JSON serialization via serde, making typed event routing straightforward. The API is clean: `queue.send(&my_queue, &MyEvent { ... }).await?` to publish, `queue.read::<MyEvent>(&my_queue, 30).await?` for consumption with a 30-second visibility timeout.

**sqlx's `PgListener`** wraps PostgreSQL's native LISTEN/NOTIFY for real-time, transient notifications. It auto-reconnects and re-subscribes on connection loss, implements `into_stream()` for `futures::Stream` compatibility, and works with both tokio and async-std. The limitation is inherent to Postgres LISTEN/NOTIFY: **notifications are transient** — messages received while disconnected are lost, and payload is limited to string data (max 8000 bytes). For an event bus, you'd wrap PgListener with serde deserialization of the string payload.

The recommended Postgres backend strategy: use **pgmq for the primary event bus** (durable, typed, serde-native) and reserve PgListener for supplementary real-time notifications where at-most-once delivery is acceptable.

---

## Trait-based backend abstraction: three proven patterns from the ecosystem

The Rust ecosystem offers three distinct approaches to backend abstraction, each with clear tradeoffs relevant to an event bus design.

**Pattern 1: Dynamic dispatch via `dyn Trait`** — exemplified by Apache Arrow's `object_store` crate. The `ObjectStore` trait defines 7 required async methods using `Pin<Box<dyn Future>>` for object safety. All backends (S3, GCS, Azure, local, in-memory) return `Arc<dyn ObjectStore>`, enabling runtime backend selection. Feature flags (`aws`, `gcp`, `azure`) control which cloud SDK dependencies compile. Key design decisions: minimal core trait with convenience methods in a separate `ObjectStoreExt` extension trait, unified `Error` enum with variants like `NotFound`/`NotImplemented`, and wrapper/adapter types (`PrefixStore`, `ThrottledStore`) that compose via the trait. **This is the most directly applicable pattern for an EventBus trait.**

**Pattern 2: Type erasure at the boundary** — exemplified by Apache OpenDAL's three-layer architecture (Operator → Layers → Services). The internal `Access` trait uses associated types (`type Reader`, `type Writer`) for zero-cost dispatch within a backend, then a `TypeEraseLayer` converts these to trait objects at the public `Operator` API boundary. Users never carry generic parameters. OpenDAL supports 50+ backends with per-service feature flags (`services-s3`, `services-gcs`, `services-memory`), capability checking via `AccessorInfo`, and composable middleware layers (`LoggingLayer`, `RetryLayer`, `TracingLayer`).

**Pattern 3: Compile-time monomorphization** — exemplified by Diesel's `Backend` trait with associated types for `QueryBuilder`, `RawValue`, and `BindCollector`. Feature flags (`postgres`, `sqlite`, `mysql`) determine which types exist at compile time. Everything is monomorphized for maximum performance, but runtime backend switching is impossible. Diesel's `SqlDialect` trait with fine-grained associated types for SQL syntax variations shows how to handle backends with different capabilities.

For an event bus that starts in-memory and migrates to GCP Pub/Sub via feature flags, **Pattern 1 (object_store style) is optimal**: define a minimal `EventBus` trait with async methods returning boxed futures, implement for `InMemoryEventBus` (always compiled) and `GcpPubSubEventBus` (behind `gcp-pubsub` feature), use `Arc<dyn EventBus>` at the application level. Feature flags control dependency inclusion, not API shape.

Best practices distilled across all three patterns:

- Define the **minimal core trait** with only required methods; convenience methods go in an extension trait
- Use a **builder pattern per backend** for configuration
- Create a **unified error type** with backend-specific context available via downcast
- Implement the trait for `Arc<T>` and `Box<T>` for seamless composition
- Add **capability checking** when backends have different feature sets (e.g., GCP supports ordering keys, in-memory might not)

---

## Event envelope design: associated types, enum events, and metadata maps

Across all major Rust event sourcing crates — **cqrs-es** (471 GitHub stars, ~115K downloads), **eventually-rs**, **esrs**, and **thalo** — a clear consensus emerges on event trait design.

**Associated types on the Aggregate/Handler trait** is the dominant pattern. Every major crate defines something like:

```rust
trait Aggregate: Default + Serialize + DeserializeOwned {
    type Event: DomainEvent;
    type Command;
    type Error: Error;
    const TYPE: &'static str;  // aggregate type identification
}
```

Events within an aggregate are **flat enums** — `BankAccountEvent::Deposited { amount, balance }` — with serde derives for serialization. This gives compile-time exhaustive matching in `apply()` methods and automatic JSON discriminants via serde's tagged enum serialization. No major crate uses trait objects (`dyn Event`) for event polymorphism.

**Event envelopes** follow a generic wrapper pattern. cqrs-es uses `EventEnvelope<A: Aggregate>` containing `aggregate_id: String`, `sequence: usize`, `payload: A::Event`, and `metadata: HashMap<String, String>`. esrs uses `StoreEvent<E>` with `id: Uuid`, `aggregate_id: Uuid`, `payload: E`, `occurred_on: DateTime<Utc>`, and `sequence_number: i32`. The metadata map approach (cqrs-es) is more flexible for correlation IDs and causation chains, while dedicated fields (esrs) are more type-safe.

**Correlation IDs and causation chains** are notably under-specified in the Rust ecosystem. Most crates leave these to the free-form metadata map rather than providing first-class fields — a gap compared to .NET/Java frameworks. The recommendation: include dedicated `correlation_id: Option<String>` and `causation_id: Option<String>` fields in the envelope alongside a general-purpose metadata map.

**Event versioning** is handled through upcasters (cqrs-es, esrs) that operate on serialized `serde_json::Value` payloads, transforming old event schemas to current ones at load time. Events are stored with `event_type` and `event_version` string tags. The cqrs-es documentation explicitly advises against maintaining multiple event versions in code, preferring upcasters.

**CloudEvents SDK** (`cloudevents-sdk`, 1M+ downloads) provides an interoperability-focused alternative with required fields (id, source, type, specversion) and integration with actix, axum, warp, reqwest, rdkafka, and nats. Worth adopting as the wire format if cross-system interoperability matters.

---

## Real-world projects reveal five migration strategies

Several production Rust projects demonstrate the in-memory-to-distributed event architecture journey.

**Eventure** (`rust-lang-libs/eventure`) is the closest architectural match to the target system. It explicitly aims for "one model abstraction and different implementations for a variety of message brokers" with planned backends for Kafka, RabbitMQ, Iggy, and in-memory, targeting both modular monolith and microservices scenarios. It's early-stage (82 commits) but worth tracking for design decisions.

**Vector** (Datadog, 21K+ stars, 93+ crates) demonstrates extreme feature-flag-driven architecture. Every source, transform, and sink lives behind a Cargo feature flag (`sinks-kafka`, `sources-aws_s3`). Internal event flow uses tokio channels between DAG components, with the topology constructed at runtime from configuration. The crate hierarchy — `vector-core` for traits, `vector-buffers` for flow control, `vector-config` for configuration macros — is an excellent reference for workspace organization.

**Zenoh** (Eclipse Foundation) achieves the most elegant migration path: its Session-based API works identically whether publishers and subscribers are in-process, same-machine, or distributed across networks. Transport selection (TCP, UDP, QUIC, shared memory) is configuration-driven, not code-driven. Their performance blog reveals an important lesson: **mixing sync and async code** (rather than going pure async) achieved 10x performance gains, reaching 3.5M msg/s.

For messaging client patterns, **async-nats** provides the cleanest trait-friendly API: `Subscriber` implements `futures::Stream`, subject-based routing uses wildcards, and JetStream adds persistence. **rdkafka** demonstrates the `ClientContext` trait pattern for customizing behavior via callbacks.

---

## Tokio channel patterns for the in-memory backend

The in-memory backend should use **`tokio::sync::broadcast`** for the core fan-out mechanism — it's the only tokio channel type supporting multiple producers and multiple consumers where each receiver gets every message.

The proven pattern from production systems wraps broadcast in a typed dispatcher:

```rust
struct TypedEventBus {
    channels: RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}
impl TypedEventBus {
    fn publish<E: Event + Clone + Send + 'static>(&self, event: E) { /* get or create broadcast for TypeId::of::<E>() */ }
    fn subscribe<E: Event + Clone + Send + 'static>(&self) -> broadcast::Receiver<E> { /* ... */ }
}
```

This **TypeMap pattern** — using `HashMap<TypeId, Box<dyn Any>>` where each entry holds a `broadcast::Sender<E>` for a specific event type — eliminates dynamic dispatch overhead on the hot path. The downcast is guaranteed safe because values of type `broadcast::Sender<E>` are only stored under key `TypeId::of::<E>()`. This approach appears in Will Crichton's foundational "Types Over Strings" article and is used by `eventador`, `bevy`, and `actix-web` internally.

**Backpressure handling**: `broadcast` uses a bounded buffer and drops the oldest messages when full, returning `RecvError::Lagged(n)` to slow receivers. For an event bus, this should be configurable — some events tolerate loss (metrics, heartbeats) while others require guaranteed delivery (state changes). The middleware layer (tower-inspired) can add retry/buffering for critical event types.

**Key design consideration**: require `Clone` on all events published to broadcast channels (broadcast requires `Clone`). For large payloads, wrap in `Arc` to make cloning cheap: `broadcast::Sender<Arc<LargeEvent>>`. When migrating to GCP Pub/Sub, this `Clone` requirement disappears since the serialized bytes are sent once to the broker.

---

## Conclusion: a synthesis architecture

The research points to a specific architectural recommendation. Define a minimal `EventBus` trait inspired by object_store's approach — async methods returning boxed futures for object safety, with `publish()` accepting serialized bytes plus metadata and `subscribe()` returning a `Stream`. Implement `InMemoryEventBus` using the TypeMap + broadcast channel pattern for typed dispatch within a process, and `GcpPubSubEventBus` wrapping `gcloud-pubsub` behind the `gcp-pubsub` feature flag.

Layer event handler dispatch on top using actix's `Handler<E>` pattern for compile-time-checked per-event-type processing, and wrap handlers with tower-style `Layer` middleware for cross-cutting concerns. Use cqrs-es's envelope design — generic `EventEnvelope<E>` with sequence number, metadata HashMap, and explicit correlation/causation ID fields. Require `Serialize + DeserializeOwned + Clone` on all event types from day one, even for in-memory use, so the migration to serialized transport requires zero event type changes.

The crate hierarchy should mirror Vector's pattern: a `core` crate defining traits only (like `tower-service`), a `runtime` crate with middleware and the TypeMap-based in-memory backend, and a `gcp` crate behind a feature flag containing the Pub/Sub implementation. This structure keeps the dependency tree lean and compilation fast while preserving a clean migration path from local to distributed event processing.
