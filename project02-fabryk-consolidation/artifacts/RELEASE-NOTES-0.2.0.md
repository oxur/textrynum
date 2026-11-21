
<!-- Historical source: workbench/RELEASE-NOTES-0.2.0.md, lines 1-202. Original bytes are recoverable from import commit 2d006a7. -->

# Textrynum / Fabryk — Release Notes v0.2.0

**Release date:** 2026-03-16
**Tag:** `0.2.0`
**Diff:** `0.1.0...0.2.0` — 128 files changed, 11,231 insertions, 2,771 deletions

---

## Highlights

- **Restructured crate hierarchy.** Fabryk now ships two clean umbrella crates (`fabryk` and
  `fabryk-mcp`) that each re-export their constituent sub-crates. Downstream projects that
  previously depended on five or more individual crates can now express that as one or two
  `[dependencies]` lines. This is the most visible breaking change in this release.

- **New crates.** Three crates were added: `fabryk-gcp` for GCP credential detection,
  `fabryk-mcp-semantic` for deferred vector-backend wiring, and `fabryk-redis` for Redis
  integration and deployment utilities.

- **Service lifecycle maturity.** Services now support parallel startup with retry, a generic
  health router, an audit trail, and a structured async startup pattern. These foundations
  underpin the new how-to guides included in the release.

- **Rust edition 2024, MSRV 1.85, and a trimmed dependency tree.** Seven major dependencies
  were upgraded, three were removed, and the workspace migrated to `thiserror` 2 and `toml` 1.

---

## Features

### New Crates

- **`fabryk-gcp`** — GCP credential detection crate. Provides helpers for locating and
  validating Google Cloud credentials at runtime, along with the `log_error_chain` utility
  that is now shared across the workspace. (`8e998e9`)

- **`fabryk-mcp-semantic`** — Adds `VectorSlot`, a deferred-wiring abstraction that lets an
  MCP server register a semantic-search backend without requiring the vector store to be
  available at startup time. (`0d92313`, `4e8cbd5`)

- **`fabryk-redis`** — Redis integration crate including a deploy module and auth middleware
  updates. (`16cb42a`)

### MCP Infrastructure

- **HTTP transport** — `fabryk-mcp` now supports Streamable HTTP transport behind an optional
  `http` feature flag. (`80094c7`)

- **`DiscoverableRegistry`** — A new registry type that exposes rich metadata to AI agents
  when they connect to an MCP server, making tool suites self-describing. (`dbc1b4a`,
  `932b7e4`)

- **MCP schema validation** — Input schemas are now validated on registration; previously,
  tools could be registered with an empty `inputSchema`, which is now caught and rejected.
  (`6589bed`)

- **MCP diagnostics tools** — Diagnostic tool handlers were added alongside the new
  `fabryk-mcp-semantic` crate. (`4e8cbd5`)

### Service Lifecycle

- **Parallel startup with retry** — Services can now be started concurrently and monitored
  with configurable retry logic, reducing total startup time for multi-service deployments.
  (`d111e63`)

- **Generic health router** — `health_router` is now generic over the application state type,
  allowing it to be embedded in any Axum application without type-parameter conflicts.
  (`eba8ca7`)

- **Audit trail** — Service lifecycle events are now recorded in a structured audit trail,
  making it easier to diagnose startup and shutdown sequencing. (`d111e63`)

### CLI

- **`vectordb get-model` command** — `fabryk-cli` gains a new subcommand to query the
  currently configured embedding model. Available when the `vector-fastembed` feature is
  enabled. (`1945aa6`)

### Workspace Utilities (`textrynum-util` / `textyl`)

- **Workspace version management** — `textyl` can now read and write the workspace version,
  query individual crate versions, and bump versions across the workspace in one operation.
  (`6da00bd`)

- **`--project-version` / `--deps-version` flags** — `textyl` is no longer specific to the
  Textrynum repository and can be used against any Rust workspace. (`c55f63f`)

- **`publish-order` and `--exclude` flags** — `textyl` now prints the correct publish order
  for workspace crates and supports excluding crates from that list. (`139bb4c`)

---

## Improvements

### Crate Hierarchy Refactor

The Fabryk workspace was restructured so that `fabryk` and `fabryk-mcp` act as umbrella
crates that re-export all sub-crates. The previous flat structure required consumers to
declare each sub-crate individually. The new layout reduces the integration surface to two
crates in the common case. (`ccb078c`)

### Dependency Upgrades

Seven major-version dependency upgrades were applied across this release cycle:

| Dependency | From | To |
|------------|------|----|
| `rmcp` | 1.0 | 1.1 |
| `toml` | 0.9 | 1.0 |
| `tantivy` | 0.22 | 0.25 |
| `fastembed` | 4 | 5 |
| `jsonwebtoken` | 9 | 10 |
| `rkyv` | 0.7 | 0.8 |
| `async-walkdir` | 1 | 2 |
| `dirs` | 5 | 6 |

Three previously declared dependencies were removed as unused. (`561da8e`, `f142357`)

### Rust Edition and MSRV

The workspace migrated to **Rust edition 2024** with a minimum supported Rust version of
**1.85**. `thiserror` was upgraded from 1 to 2 as part of this migration. (`05d0e39`)

---

## Bug Fixes

- **Serialized env-var tests** — Environment-variable-dependent tests in `fabryk-cli` were
  failing non-deterministically when run in parallel. A mutex now serializes these tests.
  (`8dd9072`, `74032b0`)

- **Reduced `log_error_chain` noise** — `log_error_chain` was emitting redundant output
  when called from high-frequency paths. Logging verbosity was reduced. (`74032b0`)

- **MCP `concepts_get` parameter documentation** — The `id` parameter for `concepts_get` was
  described ambiguously. It is now clearly documented as a slug, obtained from the
  `concepts_list` results. (`e4c6820`)

- **Internal path dependency versions** — After the 0.1.0 release, internal path dependencies
  retained `version = "0.1.0"` while the workspace had moved to `0.1.1`, causing publish
  failures. All internal versions are now kept in sync via `textyl`. (`808e788`)

- **`PUBLISH_ORDER` in Makefile** — The publish target listed crates in an order that did not
  respect dependency edges, causing `cargo publish` to fail on the first attempt. The order
  has been corrected. (`bd4b458`)

- **Minor code quality issues** — Three issues identified in a code audit were addressed:
  an unused import, an unneeded clone, and a redundant type annotation. (`ef30512`)

---

## Documentation

- **How-to guides** — Four new how-to guides were added and are linked from the workspace
  README:
  - Connecting Fabryk MCP servers to Claude Code over Streamable HTTP (`dc8e37a`)
  - The MCP async startup pattern for deferring expensive service initialization (`ae4540a`)
  - Adding a service-aware `/health` endpoint to an MCP server (`eba8ca7`)
  - Giving AI agents rich metadata via `DiscoverableRegistry` (`932b7e4`)

- **How-to directory** — How-to documents were moved from scattered locations into a
  dedicated `docs/howtos/` directory, with a corresponding README section. (`23f750d`)

- **Crate-level READMEs and images** — README files and associated images were relocated into
  their respective crate directories to keep documentation co-located with code. (`b2fe4a1`)

- **Architecture diagram** — The workspace architecture diagram was updated to reflect the
  v0.2.0 crate restructure, along with an AI prompt for maintaining the diagram in future
  releases. (`0850343`, `04d1f2c`)

---

## Infrastructure

- **CI: protobuf installation** — The `arduino/setup-protoc` GitHub Action was replaced with
  a direct `apt` install. The Action was hitting GitHub API rate limits in CI runs and
  producing intermittent failures. (`7447adb`)

---

## Upgrading from 0.1.x

The primary breaking change is the crate hierarchy restructure. If your project depended on
individual Fabryk sub-crates directly, update your `Cargo.toml` to use the umbrella crates:

```toml
# Before (0.1.x)
fabryk-core = "0.1"
fabryk-content = "0.1"
fabryk-fts = "0.1"
fabryk-graph = "0.1"
fabryk-vector = "0.1"

# After (0.2.0)
fabryk = { version = "0.2", features = ["full"] }
```

The `fabryk-mcp` umbrella follows the same pattern. Vendor-specific crates
(`fabryk-auth-google`, `fabryk-gcp`) remain separate and are versioned independently.

The Rust edition upgrade to 2024 and MSRV bump to 1.85 require Rust 1.85 or later in your
toolchain. Run `rustup update stable` if you are on an older toolchain.
