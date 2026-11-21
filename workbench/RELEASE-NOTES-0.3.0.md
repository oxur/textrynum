# Release Notes — v0.3.0

**Released:** 2026-03-17
**Scope:** fabryk / ecl workspace
**Diff:** 26 files changed, 1063 insertions(+), 83 deletions(-)

---

## Highlights

This release significantly expands the `fabryk-mcp-core` crate with three
major additions: a composable `ServerGuidance` builder that unifies server
instruction generation, a `ResourceRegistry` trait that enables MCP resource
support (list, read, subscribe, unsubscribe), and per-URI subscription
tracking in `Notifier`. The `DiscoverableRegistry` gains a `from_guidance()`
constructor that eliminates repetitive wiring between guidance and registry
setup. Downstream MCP servers can now compose custom HTTP routers without
adding direct dependencies on `fabryk-core` or `axum`, thanks to new
re-exports from `fabryk-mcp-core`. Workspace dependency versions were also
broadly updated across crates.

---

## Features

### fabryk-mcp-core: ServerGuidance builder (`guidance.rs`)

A new `ServerGuidance` struct provides a single composable builder for all
server instruction mechanisms. It replaces scattered `with_description`,
`with_discoverable_instructions`, and `with_query_strategy` calls with a
declarative structure that can be shared between `FabrykMcpServer` and
`DiscoverableRegistry`.

Key capabilities:

- `ServerGuidance::for_domain(domain)` — creates guidance scoped to a named
  domain.
- Builder methods: `.context()`, `.workflow()`, `.convention()`,
  `.constraint()`, `.subscribe()`, `.connector()`, `.data_freshness()`,
  `.tool_meta()`, `.tool_metas()`.
- `to_instructions()` — generates the `ServerInfo.instructions` text,
  including a directory-first directive, context, subscription
  recommendations, conventions, and constraints. Sections are omitted when
  empty.
- `directory_tool_name()` — returns the canonical `{domain}_directory` tool
  name.
- `FabrykMcpServer::with_guidance(&guidance)` — apply guidance to the server
  in one call.

### fabryk-mcp-core: ResourceRegistry trait (`resource.rs`)

A new `ResourceRegistry` trait abstracts over MCP resource listing and
reading, allowing domains to expose subscribable resources to MCP clients.

- `resources() -> Vec<Resource>` — lists all available resources.
- `read(uri) -> Option<ResourceFuture>` — async read by URI; returns `None`
  for unrecognized URIs.
- `FabrykMcpServer::with_resources(registry)` — registers a resource registry
  and advertises `resources/list`, `resources/read`, `resources/subscribe`,
  and `resources/unsubscribe` in server capabilities.

### fabryk-mcp-core: Per-URI subscription tracking in Notifier (`notifier.rs`)

`Notifier` now maintains a per-URI subscription map in addition to the
broadcast peer list.

- `subscribe_resource(uri, peer)` — registers a peer's interest in a specific
  URI.
- `unsubscribe_resource(uri)` — removes all subscriptions for a URI.
- `resource_updated(uri)` — if any peers have subscribed to the URI, only
  those peers are notified; otherwise all connected peers receive the
  notification (backwards-compatible broadcast).
- `resource_list_changed()` — broadcasts a resource-list-changed notification
  to all peers.
- Dead peers are pruned automatically on each broadcast.

### fabryk-mcp-core: DiscoverableRegistry.from_guidance() (`discoverable.rs`)

`DiscoverableRegistry` gains a `from_guidance(registry, &guidance)`
constructor that populates all fields (tool metadata, workflow/query strategy,
connectors, subscriptions, conventions, constraints) directly from a
`ServerGuidance` value, removing the need to pass each field individually.

The directory tool output now includes sections for recommended subscriptions,
conventions, and constraints alongside the existing tool catalog.

### fabryk-mcp-core: Re-exports of ServiceHandle, ServiceState, and axum

Downstream MCP servers can now access service lifecycle types and HTTP
composition primitives without adding direct crate dependencies:

- `fabryk_mcp_core::ServiceHandle` and `fabryk_mcp_core::ServiceState`
  (re-exported from `fabryk-core`).
- `fabryk_mcp_core::axum` (re-exported behind the `http` feature flag),
  enabling custom HTTP router composition.

---

## Improvements

### fabryk-mcp-core: Server resource handler integration (`server.rs`)

`FabrykMcpServer` integrates the new `ResourceRegistry` and subscription
infrastructure into the `ServerHandler` implementation. The server now
handles `resources/list`, `resources/read`, `resources/subscribe`, and
`resources/unsubscribe` protocol messages and wires subscription/
unsubscription events to the `Notifier` automatically.

---

## Bug Fixes

### Publish tooling: differentiated rate-limit delays for new vs existing crates

The `publish` Makefile target now queries the crates.io API before each
publish to determine whether a crate is new or already exists, then applies
the appropriate delay:

- **Existing crates:** 72 seconds (crates.io allows 1 publish per minute with
  a burst of 30, plus index propagation buffer).
- **New crates:** 624 seconds (crates.io allows 1 new crate per 10 minutes
  with a burst of 5, plus initial indexing buffer).

Previously a single fixed delay of 372 seconds was applied to every publish,
which was both too slow for existing crates and too fast for first-time
publishes of new crates.

---

## Infrastructure

### Workspace dependency version bumps

`Cargo.toml` dependency versions were updated across the following crates:
`ecl-cli`, `ecl-steps`, `ecl-workflows`, `fabryk-acl`, `fabryk-auth-google`,
`fabryk-cli`, `fabryk-content`, `fabryk-fts`, `fabryk-graph`,
`fabryk-mcp-content`, `fabryk-mcp-fts`, `fabryk-mcp-graph`,
`fabryk-mcp-semantic`, `fabryk-mcp`, `fabryk-vector`, and `fabryk`. The
`Cargo.lock` file was updated to reflect resolved dependency changes.

---

## Crates Updated

| Crate | Change |
|---|---|
| `fabryk-mcp-core` | Major feature additions (guidance, resources, subscriptions, re-exports) |
| `fabryk-mcp` | Dependency version bump |
| `fabryk-mcp-content` | Dependency version bump |
| `fabryk-mcp-fts` | Dependency version bump |
| `fabryk-mcp-graph` | Dependency version bump |
| `fabryk-mcp-semantic` | Dependency version bump |
| `fabryk` | Dependency version bump |
| `fabryk-acl` | Dependency version bump |
| `fabryk-auth-google` | Dependency version bump |
| `fabryk-cli` | Dependency version bump |
| `fabryk-content` | Dependency version bump |
| `fabryk-fts` | Dependency version bump |
| `fabryk-graph` | Dependency version bump |
| `fabryk-vector` | Dependency version bump |
| `ecl-cli` | Dependency version bump |
| `ecl-steps` | Dependency version bump |
| `ecl-workflows` | Dependency version bump |
