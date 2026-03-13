---
number: 16
title: "Research - 2026 State of Rust Ecosystem for Workflow/Pipeline Frameworks"
author: "jblondin actually"
component: All
tags: [change-me]
created: 2026-03-13
updated: 2026-03-13
state: Final
supersedes: null
superseded-by: null
version: 1.0
---

# Research - 2026 State of Rust Ecosystem for Workflow/Pipeline Frameworks

## Overview

**Rust has the parts but not the whole pipeline framework**

*No single crate in the Rust ecosystem delivers all five properties you need — durability, configurability, observability, incrementality, and composability — in a lightweight CLI-friendly package.* This is a genuine gap, not an oversight in research. However, the ecosystem provides excellent building blocks that compose cleanly into a custom solution. The practical recommendation: build a thin pipeline runner (~500–1000 lines) atop proven crates for persistence, hashing, DAG execution, and API access. This section-by-section evaluation covers every viable crate found across the ecosystem as of early 2026, with concrete architectural guidance.

---

## DAG and workflow crates exist but none check all five boxes

The most mature option is **dagrs** (v0.6.0, updated days ago), a flow-based programming framework with async Tokio execution, bounded channels, conditional nodes, and — critically — **configurable parsers supporting TOML, YAML, and JSON** via a `Parser` trait. It has ~38,000 downloads and active development with derive macro support. However, dagrs has no built-in checkpointing or resume capability. It solves composability and configurability but leaves durability to you.

**erio-workflow** (v0.1.0) is the only crate found with an explicit `Checkpoint` module for workflow recovery, plus DAG execution, typed context (`WorkflowContext`), and conditional step execution. Its module structure — `builder`, `dag`, `engine`, `step`, `context`, `checkpoint`, `conditional`, `error` — maps almost perfectly to your requirements. The catch: it's v0.1.0 with **77% documentation coverage** and was built for the Erio AI agent ecosystem, making it tightly coupled and immature. **oxigdal-workflow** (v0.1.0) similarly offers state persistence with save/restore, retry with exponential backoff, and optional scheduling — but was designed for geospatial processing and is equally early-stage.

**dagx** deserves mention for type safety: it uses a type-state pattern for **compile-time cycle prevention** in DAGs, provides `#[task]` proc macros, and supports stateless/read-only/mutable state tasks. Its `data_pipeline.rs` example demonstrates ETL patterns directly. But again, no checkpointing.

For pure DAG scheduling without execution, **dagga** (v0.2.3, ~41,000 downloads) handles create/read/write/consume resource semantics with parallel batch scheduling. And **petgraph** (18M+ downloads, 50 versions) remains the foundational graph library that most workflow crates build upon — its `Acyclic` wrapper, topological sort, serde support, and rayon parallel iterators make it the safest base for a custom DAG executor.

The ETL space is sparse. **rettle** offers a Keras-inspired ETL API with Fill/Transform/Pour ingredient types and multithreaded execution, but appears low-activity. An older **etl** crate by jblondin actually supported TOML-based pipeline configuration, but is effectively abandoned. Supabase's ETL framework (2,200 GitHub stars, very active) is impressive but server-oriented and Postgres-specific — not CLI material.

---

## redb is the right persistence layer for CLI checkpointing

For filesystem-based state persistence, the field has consolidated around a few strong options. **redb** (v3.1.0, September 2025) stands out as the best fit: **pure Rust, single-file storage, full ACID with copy-on-write B-trees**, stable file format since 1.0, crash recovery without a WAL, typed table API, and zero external C dependencies. Compile time is ~23 seconds, startup is fast, and the footprint is small (~985 KB source). It has 4,247 GitHub stars and is actively maintained by Christopher Berner.

**fjall** (v3.0.0, January 2026) is the runner-up — an LSM-tree store (like RocksDB but pure Rust) with multiple keyspaces, configurable durability via `PersistMode::SyncAll`, MVCC snapshots, and built-in compression. It compiles in ~3.5 seconds versus RocksDB's ~40 seconds. The tradeoff: it stores data across a directory rather than a single file, and runs background compaction threads, making it slightly heavier.

Other options evaluated:

- **jammdb** — Pure Rust BoltDB port with ACID, single-file, copy-on-write, minimal API. Excellent simplicity but hasn't reached 1.0.
- **persy** — Pure Rust, single-file, ACID with WAL-based crash recovery and a 2-phase commit model. Has reached 1.0 and has an active community.
- **heed** — LMDB wrapper maintained by the Meilisearch team. Blazing-fast reads via memory mapping, but requires a C dependency and LMDB does not perform checksum verification on reads.
- **sled** — Once the darling of Rust KV stores (8,895 stars), but **the stable release (0.34.7) hasn't been updated since ~2021** and the 1.0 alpha remains unstable. Not recommended for new projects.
- **rocksdb** — Powerful but overkill for CLI: ~40-second compile, ~12 MB binary, requires a C++ toolchain.

For state schema evolution — inevitable as your pipeline config changes — **revision** by SurrealDB lets you annotate struct fields with `#[revision(start=N, end=M)]` for automatic migration from old serialized formats. **versionize** from Amazon's Firecracker team is a battle-tested alternative for snapshot/restore versioning.

**The practical checkpointing pattern** requires no dedicated crate: after each pipeline step, serialize a `PipelineState { current_step, completed_steps, intermediate_results, item_hashes }` struct into redb. On startup, check for existing state and resume from the last successful checkpoint. This is ~50–100 lines of code atop redb + serde + bincode/postcard.

---

## Composability comes from a custom Stage trait, not a framework

The tower ecosystem's `Service<Request>` trait — async `fn(Request) → Future<Result<Response, Error>>` with `Layer`-based middleware composition — is the gold standard pattern. But tower is async-only, networking-focused, and heavier than needed for a CLI data pipeline. The recommendation: **adopt the pattern, not the dependency**. Define a custom `Stage<I, O>` trait with typed inputs/outputs, and compose stages into pipelines programmatically.

Two small crates directly implement this pattern. **composable** provides `Composable<Input, Output>` with an `.apply()` method and a `composed![]` macro for declarative pipeline assembly — it supports structs, closures, and functions with typed I/O and error propagation. **iterpipes** defines a `Pipe` trait with `type InputItem` and `type OutputItem`, composable via a `>>` operator. Both are lightweight, sync-capable, and require no async runtime. Either could serve as a starting point or inspiration, though both have modest adoption.

For expression-level chaining, **tap** (1.0.1, millions of downloads) adds universal `.pipe(fn)` and `.tap(fn)` methods to all types, enabling fluent transformation chains. **pipe-trait** does the same standalone. These are utilities, not frameworks, but they improve ergonomics in pipeline stage implementations.

For parallel execution within stages, **rayon** remains the standard for CPU-bound work with its drop-in parallel iterators, and **futures-util**'s `Stream` combinators handle async fan-out patterns cleanly.

---

## blake3 and merkle_hash solve incrementality cleanly

Content-based change detection is well-served. **blake3** (v1.8.2, **80M+ downloads**) is the clear choice for content hashing: SIMD-optimized (SSE2/4.1, AVX2/512, NEON), supports incremental hashing and multithreaded hashing via rayon, and runs significantly faster than SHA-256. For a pipeline that processes files from Google Drive or Slack, hashing each item's content to detect changes between runs is straightforward with blake3.

**merkle_hash** builds on blake3 to compute Merkle tree hashes of entire directory trees in parallel via rayon, returning path→hash mappings. This is ideal for detecting which files in a local cache or staging directory have changed since the last run.

For caching pipeline outputs by content, **cacache** (v13.1.0) provides a content-addressable disk cache ported from npm's cacache, with integrity verification and atomic writes. It's async-first (tokio or async-std) and well-maintained (695 GitHub stars, by Kat Marchán).

**salsa** (v0.24.0, 3.5M+ downloads, maintained by Niko Matsakis) is the full incremental recomputation framework used by rust-analyzer. It tracks dependencies between queries automatically, memoizes results, and only recomputes what's invalidated — with early cutoff optimization when inputs change but outputs remain the same. For your use case, salsa is likely overkill unless your pipeline has deeply interdependent computation steps. A simpler approach — **store blake3 hashes of each item in redb, skip items whose hash matches the previous run** — covers the incrementality requirement with minimal complexity.

For filesystem watching (if a watch/daemon mode is ever needed), **notify** (v7+) is the de facto standard with cross-platform support (inotify/FSEvents/ReadDirectoryChanges), debounced events, and recursive watching. It's used by cargo-watch, rust-analyzer, deno, and mdBook.

---

## Mature API client crates exist for all three target services

**Google Drive and Sheets** are best served by Byron's google-apis-rs project: **google-drive3** (v7.0.0+20251218) and **google-sheets4** (v7.0.0+20251215) provide complete, auto-generated API coverage that's regularly regenerated from Google's API Discovery docs. Both integrate directly with **yup-oauth2** (v12.1.2, January 2026), which handles InstalledFlow (local HTTP redirect for CLI auth), DeviceFlow (headless auth), ServiceAccount flow, **built-in token persistence to disk**, and **automatic token refresh**. The `google-drive3` crate even ships its own CLI tool. This stack eliminates all OAuth boilerplate for Google services.

For **Slack**, **slack-morphism** (v2.17.0, ~89,634 monthly downloads) dominates with comprehensive, type-safe coverage of the Web API, Events API, Socket Mode, and Block Kit. The base `SlackClient` works in any context — create a session with a bot token and call any API method. For OAuth2 with Slack (and any non-Google provider), the generic **oauth2** crate (v5.0.0, January 2025) supports authorization code flow with PKCE, device authorization flow (RFC 8628), and refresh token exchange. It works with reqwest (async + blocking), curl, or ureq, but requires you to handle token persistence manually.

For resilient HTTP in all adapters, the **reqwest-middleware** (v0.4.2) + **reqwest-retry** (v0.7.x) stack from TrueLayer provides composable middleware chains with configurable exponential backoff and transient/permanent error classification. **backon** (v0.4.4+) is a modern alternative offering `.retry()` as a trait method on any function. All recommended API crates are async and Tokio-based, which aligns naturally.

---

## Recommended architecture and crate composition

The evidence strongly supports **building a custom pipeline runner** composed from proven crates rather than adopting any single framework. Here is the recommended stack mapped to your five requirements:

- **Durability**: `redb` (v3.1.0) for ACID checkpoint storage + `serde` + `postcard` for compact binary serialization + `revision` for state schema evolution
- **Configurability**: `toml` crate for parsing declarative stage definitions + custom `PipelineConfig` structs with serde derives
- **Observability**: Serialize pipeline state to redb in an inspectable format (JSON sidecar or `redb`'s typed tables) + `tracing` for structured logging
- **Incrementality**: `blake3` (v1.8.2) for content hashing + hash-per-item stored in redb + skip-if-unchanged logic (~30 lines)
- **Composability**: Custom `Stage<I, O>` trait inspired by tower's Service pattern or built atop `composable`/`iterpipes` + `petgraph` if DAG execution ordering is needed

For source adapters: `google-drive3` + `google-sheets4` + `yup-oauth2` for Google, `slack-morphism` + `oauth2` for Slack, `reqwest-middleware` + `reqwest-retry` for resilient HTTP across all adapters.

## Conclusion

The Rust ecosystem in early 2026 has **no lightweight pipeline framework with built-in checkpointing** ready for production CLI use. The two crates that come closest — erio-workflow and oxigdal-workflow — are both v0.1.0 with narrow domain focus. This is not a temporary gap; pipeline orchestration in Rust has fragmented into either heavy distributed systems (Restate, Temporal adapters) or minimal DAG libraries without persistence. The right move is a custom runner: **the total integration surface is ~500 lines of pipeline orchestration code** atop redb, blake3, serde, and a custom Stage trait. The API client ecosystem, by contrast, is mature — google-drive3, slack-morphism, and yup-oauth2 are production-ready and actively maintained, making source adapter development straightforward. The build-vs-buy decision here is clear: buy the components, build the glue.
