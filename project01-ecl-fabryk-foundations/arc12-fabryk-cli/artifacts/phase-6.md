
<!-- Historical source: workbench/old/phase-6.md, lines 1-160. Original bytes are recoverable from import commit 3a39aea. -->

# Phase 6: fabryk-cli — CLI Framework & Graph Commands

## Context

Phases 4-5 are complete (fabryk-graph with 134 tests, fabryk-vector, four fabryk-mcp-* crates with 89 tests). Phase 6 extracts a generic CLI framework into `fabryk-cli` so domains can share common CLI patterns while adding custom subcommands. The `fabryk-cli` skeleton crate already exists in the workspace (Cargo.toml, README.md, stub lib.rs) but has no implementation.

Design docs: 0032 (6.1 — CLI Framework), 0033 (6.2 — Graph CLI Commands), 0034 (6.3 — Music Theory Integration, out of scope for ECL repo).

---

## Milestone 6.1 — CLI Framework

### What to create

**Files:**

- `crates/fabryk-cli/src/lib.rs` — module declarations, re-exports
- `crates/fabryk-cli/src/cli.rs` — `CliArgs`, `BaseCommand`, `GraphCommand`, `CliExtension` trait
- `crates/fabryk-cli/src/app.rs` — `FabrykCli<C: ConfigProvider>` application

### Key types

```
CliArgs (clap Parser): --config, --verbose, --quiet, subcommand
BaseCommand (clap Subcommand): Serve{port}, Index{force,check}, Version, Health, Graph(GraphCommand)
GraphCommand (clap Subcommand): Build{output,dry_run}, Validate, Stats, Query{id,query_type,to}
CliExtension trait: associated Command type + handle_command() using RPITIT
FabrykCli<C: ConfigProvider>: name, config (Arc<C>), version; methods: new(), with_version(), config(), init_logging(), run()
```

### Cargo.toml changes

Replace `twyg`/`log` with `tracing`/`tracing-subscriber` (per design doc spec, both available in workspace):

```toml
[dependencies]
fabryk-core = { path = "../fabryk-core" }
clap = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```

### Tests (in each module)

- `cli.rs`: CliArgs parsing (version flag, verbose, quiet, serve command, graph subcommands)
- `app.rs`: FabrykCli construction, version override, config access, logging init, command dispatch (version, health, no-command)

### Verification

```
cargo check -p fabryk-cli
cargo test -p fabryk-cli
cargo clippy -p fabryk-cli -- -D warnings
cargo fmt --check -p fabryk-cli
```

---

## Milestone 6.2 — Graph CLI Command Handlers

### What to create

**Files:**

- `crates/fabryk-cli/src/graph_handlers.rs` — `BuildOptions`, `QueryOptions`, handler functions

### Cargo.toml additions

```toml
fabryk-graph = { path = "../fabryk-graph" }
fabryk-content = { path = "../fabryk-content" }
```

### Key API corrections from design doc

The design doc references `result.graph` / `result.files_processed`, but `GraphBuilder::build()` returns `Result<(GraphData, BuildStats)>` (a tuple). Corrections:

```rust
// Design doc says:       let result = builder.build().await?;
// Actual API:            let (graph, stats) = builder.build().await?;
// Design doc says:       result.graph.node_count()
// Actual API:            graph.node_count()
// Design doc says:       result.files_processed
// Actual API:            stats.files_processed
```

### Handler functions

```
handle_build<C: ConfigProvider, E: GraphExtractor>(config, extractor, options) -> Result<()>
  - Two-phase: discover content → build graph → save
  - Dry-run support
  - Prints summary (nodes, edges, files processed/skipped, errors)

handle_validate<C: ConfigProvider>(config) -> Result<()>
  - Loads graph from data/graphs/graph.json
  - Runs validate_graph(), prints errors/warnings with codes

handle_stats<C: ConfigProvider>(config) -> Result<()>
  - Loads graph, runs compute_stats()
  - Prints node counts, edge counts, degree stats, categories, relationships

handle_query<C: ConfigProvider>(config, options) -> Result<()>
  - Loads graph, dispatches on query_type: related/prerequisites/path
  - Uses neighborhood(), prerequisites_sorted(), shortest_path()
```

### Update app.rs

Wire `handle_graph()` to delegate Validate/Stats/Query to graph_handlers. Build remains a placeholder (domains override with their extractor).

### Update lib.rs

Add `pub mod graph_handlers` and re-export handler functions + option types.

### Tests

- `graph_handlers.rs`: Mock ConfigProvider tests for validate/stats/query with temp graph files, build dry-run test, missing graph file handling, unknown query type

### Verification

```
cargo check -p fabryk-cli
cargo test -p fabryk-cli
cargo clippy -p fabryk-cli -- -D warnings
cargo fmt --check -p fabryk-cli
cargo test --workspace  # no regressions
```

---

## Key files to read/modify

| File | Action |
|------|--------|
| `crates/fabryk-cli/Cargo.toml` | Modify deps |
| `crates/fabryk-cli/src/lib.rs` | Rewrite (replace stub) |
| `crates/fabryk-cli/src/cli.rs` | Create |
| `crates/fabryk-cli/src/app.rs` | Create |
| `crates/fabryk-cli/src/graph_handlers.rs` | Create (6.2) |

### Reference files (patterns to follow)

- `crates/fabryk-graph/src/builder.rs` — GraphBuilder API, BuildStats fields
- `crates/fabryk-graph/src/persistence.rs` — load_graph, save_graph, GraphMetadata
- `crates/fabryk-graph/src/validation.rs` — validate_graph, ValidationResult, ValidationIssue
- `crates/fabryk-graph/src/stats.rs` — compute_stats, GraphStats
- `crates/fabryk-graph/src/algorithms.rs` — neighborhood, shortest_path, prerequisites_sorted
- `crates/fabryk-core/src/traits.rs` — ConfigProvider trait
- `crates/fabryk-core/src/error.rs` — Error::config(), Error::operation()

## Out of scope

- 6.3 (Music Theory CLI Integration) — domain-specific, belongs in ai-music-theory repo
