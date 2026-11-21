# Release Notes — v0.4.0

**Released:** 2026-03-21
**Scope:** ECL pipeline engine (major), fabryk workspace (maintenance)
**Diff:** 209 files changed, 66,767 insertions(+), 1,239 deletions(-)
**Commits:** 61

---

## Highlights

This release transforms ECL from a document-oriented extraction tool into a
**general-purpose data pipeline engine** capable of running production fintech
workloads. The new pipeline runner — designed from scratch across 25 milestones
— provides TOML-driven pipeline configuration, multi-stream data routing with
joins and aggregations, Kafka/Avro output with Schema Registry integration,
cloud storage adapters (GCS, SFTP), PGP decryption, secret management, and
checkpoint/resume with crash-safe state persistence.

The engine is validated against real-world requirements from four customer
pipelines (Acme Inc., Affinity, Giant Eagle, Chime) with end-to-end integration
tests demonstrating CSV ingestion through receipt assembly and Kafka output —
all configured in TOML with zero customer-specific Rust code.

### New crate count: 14

| Crate | Purpose |
|-------|---------|
| `ecl-pipeline-spec` | TOML specification layer |
| `ecl-pipeline-state` | Execution state & checkpoint persistence |
| `ecl-pipeline-topo` | Topology resolution, traits, DAG scheduling |
| `ecl-pipeline` | Runner orchestration engine |
| `ecl-stages` | 15 built-in stage implementations |
| `ecl-adapter-fs` | Filesystem source adapter |
| `ecl-adapter-gcs` | Google Cloud Storage source adapter |
| `ecl-adapter-gdrive` | Google Drive source adapter |
| `ecl-adapter-slack` | Slack source adapter (stub) |
| `ecl-adapter-zapier` | Zapier webhook push source adapter |
| `ecl-adapter-sftp` | SFTP source adapter |
| `ecl-sink-kafka` | Kafka producer with Avro + Schema Registry |
| `ecl-sink-gcs` | GCS file writer |
| `ecl-secrets` | Pluggable secret resolution (env, file, GCP SM) |

---

## ECL Pipeline Engine — Foundation (Milestones 1.1–3.2)

### Specification layer (`ecl-pipeline-spec`, milestone 1.1)

TOML-driven pipeline configuration with strongly typed source and stage specs.
`PipelineSpec` defines sources (keyed by name), stages (keyed by name), global
defaults (concurrency, retry, checkpoint strategy), and output directory. All
types derive `Serialize + Deserialize` for checkpoint embedding.

### State layer (`ecl-pipeline-state`, milestones 1.2, 2.1)

Complete execution state model: `PipelineState` with per-source `SourceState`,
per-item `ItemState` (Pending/Processing/Completed/Failed/Skipped/Unchanged),
per-stage `StageState`, and aggregate `PipelineStats`. `Checkpoint` embeds the
full spec + schedule + state for resume. `RedbStateStore` provides ACID
crash-safe persistence via the `redb` embedded database. `InMemoryStateStore`
for testing.

### Topology layer (`ecl-pipeline-topo`, milestones 1.3, 2.3)

Core trait definitions: `SourceAdapter` (enumerate + fetch), `PushSourceAdapter`
(start + shutdown), `Stage` (process). `PipelineItem` carries `Arc<[u8]>`
content for zero-copy cloning in concurrent pipelines. Resource graph validation
(no cycles, no missing inputs) with Kahn's algorithm for parallel batch
scheduling. `resolve()` converts spec + adapter/stage lookups into an executable
`PipelineTopology`.

### Runner core (`ecl-pipeline`, milestone 2.2)

Four-phase execution: enumerate sources, apply incrementality (skip unchanged
items via blake3 content hash comparison), execute batches with bounded
concurrency and exponential backoff retry (`backon` crate), and finalize with
completed hash persistence. Push-source event loop for webhook adapters with
`tokio::select!` and graceful shutdown.

### Filesystem adapter & built-in stages (`ecl-adapter-fs`, `ecl-stages`, milestone 3.1)

`FilesystemAdapter`: async directory walk with glob/extension filtering, MIME
type detection, blake3 hashing. Four built-in stages: `ExtractStage` (delegate
to adapter), `NormalizeStage` (passthrough), `FilterStage` (glob
include/exclude), `EmitStage` (write to output directory).

### Integration test suite (milestone 3.2)

Comprehensive tests covering full pipeline execution (10 files, empty dirs,
nested dirs, binary content, extension filters, multi-stage), checkpoint/resume,
config drift detection, and stuck-item recovery.

---

## ECL Pipeline Engine — Adapters (Milestones 4.1–6.1, 7.1–7.2)

### Google Drive adapter (`ecl-adapter-gdrive`, milestones 4.1, 4.2)

OAuth2 via `TokenProvider` (env var, file, or application default credentials).
Recursive folder traversal with breadth-first queuing, file type filters
(extension + MIME), `modified_after` support, pagination. Content download with
Google Docs export for native formats.

### Slack stub adapter (`ecl-adapter-slack`, milestone 6.1)

Validation stub proving the `SourceAdapter` trait works for message-oriented
sources. Fixture-based content injection for testing.

### Zapier webhook adapter (`ecl-adapter-zapier`, milestones 7.1, 7.2)

Complete `PushSourceAdapter` implementation: Axum HTTP server on configurable
bind address, Basic Auth / Bearer token validation, bounded `mpsc` channel with
429 backpressure. Typed schema dispatch for Granola meeting notes, Gmail
messages, Slack messages, and Google Drive file changes, with raw JSON fallback.

---

## ECL Pipeline Engine — Phase 1: Batch Data Processing (Milestones 7.1–7.8)

### Record type & CSV parse stage (milestone 7.1)

`Record` type alias (`serde_json::Map<String, Value>`) added to `PipelineItem`
for structured tabular data. `CsvParseStage`: configurable columns with type
conversion (string, integer, float, boolean), delimiter/quote/header options,
fan-out from one file item to N row items, per-row error handling (skip or
fail).

### Field mapping stage (milestone 7.2)

`FieldMapStage`: configurable rename, drop, set (literal values), copy, date
parsing (strftime format to RFC3339), string padding (left/right), regex
extraction with capture groups, and nested object construction. All operations
configured via TOML params.

### GCS source adapter (`ecl-adapter-gcs`, milestone 7.3)

`SourceAdapter` for Google Cloud Storage: list objects by prefix + glob pattern,
fetch content as bytes. Supports Application Default Credentials, service
account JSON (env var or file). Error mapping: 401/403 to AuthError, 404 to
NotFound, 429 to RateLimited, 5xx to Transient.

### Validation stage (milestone 7.4)

`ValidateStage`: configurable rules (required, regex, date_range, length,
numeric_range) with hard/soft severity classification. Failed records tagged
with `_validation_errors` and `_validation_status` metadata for downstream
routing. Items always pass through — sinks filter by status.

### Kafka sink stage (`ecl-sink-kafka`, milestone 7.5)

`KafkaSinkStage` using `rdkafka` (librdkafka): SASL_SSL + SCRAM-SHA-256 auth,
Confluent Schema Registry integration (register/fetch schemas via HTTP), Avro
binary serialization with Confluent wire format (magic byte + 4-byte schema ID +
payload). `${ENV_VAR}` interpolation in config params for credentials.

### GCS sink stage (`ecl-sink-gcs`, milestone 7.6)

`GcsSinkStage`: write items to GCS as JSON lines, CSV, or raw bytes.
Configurable bucket, prefix, filename template. Filter param for routing
valid-only or errors-only items.

### File lifecycle management (milestone 7.7)

`LifecycleSpec` on `PipelineSpec`: configurable staging, historical, and error
prefixes. Runner hooks for post-run file movement: success moves to historical,
failure moves back to input. GCS copy + delete operations.

### Affinity end-to-end integration tests (milestone 7.8)

Full pipeline test: CSV parse (19 columns) -> field map (rename, drop, set,
date parse, auth code pad, regex extract) -> validate (required, regex,
date_range) -> emit. Validates the complete Phase 1 stack against realistic
Affinity transaction data.

---

## ECL Pipeline Engine — Phase 2: Multi-Stream Joins (Milestones 8.1–8.8)

### Named data streams (milestone 8.1)

`stream: Option<String>` on `PipelineItem`. Sources tag items at creation.
`input_streams` and `output_stream` on `StageSpec` for routing. Runner
maintains an `active_items` pool: stages consume matching items and produce
tagged outputs. Backward compatible — pipelines without streams work unchanged.

### Batch stage trait & JoinStage (milestone 8.2)

`requires_batch()` and `process_batch()` default methods on `Stage` trait.
Existing stages inherit per-item defaults unchanged. `JoinStage`: HashMap-based
index for two-stream joins by key. Supports inner, left, and full join types
with configurable right-side field prefix.

### Aggregation stage (milestone 8.3)

`AggregateStage`: group-by with composite keys, aggregate functions (sum, max,
min, avg, count, first, last), and `collect_arrays` for gathering sub-records
into JSON arrays. String-to-numeric coercion for amount fields.

### Lookup table stage (milestone 8.4)

`LookupStage`: static value mapping via inline TOML tables. Pre-built HashMaps
at construction time. Case-insensitive option. Default values for unmapped
inputs. Multiple lookups per stage instance.

### Date/time & timezone stages (milestone 8.5)

`DateParseStage`: configurable strftime format to RFC3339 conversion with
assumed timezone. `TimezoneStage`: US ZIP code prefix to IANA timezone lookup
(embedded table), per-record store ID overrides, fallback timezone. Uses
`chrono-tz` for timezone arithmetic.

### ZIP decompression stage (milestone 8.6)

`DecompressStage`: ZIP and GZIP extraction with fan-out (one archive to N
files). Extension filter for selective extraction. Operates on in-memory
`Cursor<&[u8]>` via the `zip` crate.

### Receipt assembly stage (milestone 8.7)

`AssembleStage`: batch stage merging multiple streams into nested receipt
structures. Configurable primary stream, primary key, and join definitions with
one-to-one (nest as object) and one-to-many (collect as array) modes.

### Giant Eagle end-to-end integration tests (milestone 8.8)

Full multi-stream pipeline test: 5 CSV file types (stores, transactions, items,
products, tenders) -> per-stream parse + map -> lookup (payment types/schemes)
-> join (items + products by UPC) -> aggregate (tenders by transaction) ->
assemble (nested receipts) -> validate -> emit. Validates the complete Phase 2
stack against realistic Giant Eagle retail data.

---

## ECL Pipeline Engine — Phase 3: Cloud Integration (Milestones 9.1–9.2)

### Secret management (`ecl-secrets`, milestone 9.1)

`SecretResolver` trait with three backends: `EnvResolver` (environment
variables), `FileResolver` (file paths), `GcpSecretResolver` (GCP Secret
Manager, feature-gated). `CredentialRef::Secret` variant for TOML specs.
`SecretsConfig` on `PipelineSpec` for provider configuration. Lazy resolution
at stage construction time.

### SFTP source adapter (`ecl-adapter-sftp`, milestone 9.2)

`SourceAdapter` for SFTP servers via `russh`/`russh-sftp`: SSH key and password
authentication, remote directory listing with glob pattern filtering, file
content download with blake3 hashing. Credentials resolved through
`SecretResolver`.

---

## ECL Pipeline Engine — CLI & Observability (Milestones 5.1–5.2)

### Pipeline CLI commands (`ecl-cli`, milestone 5.1)

Six subcommands under `ecl pipeline`: `run` (load config, resolve topology,
execute, checkpoint), `resume` (load checkpoint, check config drift, continue),
`status` (display pipeline state), `inspect` (show topology, schedule, resolved
stages), `items` (list items with statuses), `diff` (compare two runs' outputs).

### Observability & reporting (milestone 5.2)

Tracing spans for pipeline, batch, stage, and item levels. Structured logging
at appropriate severity (INFO for boundaries, DEBUG for item success, WARN for
skips, ERROR for failures). Duration tracking. `Display` impls for all state
types.

---

## Fabryk Improvements

### MCP discoverability guide

Iterated the MCP metadata/discoverability howto for better agent-driven
results.

### fabryk-cli: Shared config infrastructure

Added `Confygery`-based config loader with TOML file + environment variable
overlay, path resolution against config file parent directory, and section-based
env var namespacing.

### fabryk-mcp: Re-exports and tool registry improvements

`with_names()`/`with_descriptions()` added to all MCP tool registries.
`ErrorCode` and `AnnotateAble` re-exported from `fabryk_mcp::model`. Serde
backward compatibility for older graph JSON formats.

---

## Bug Fixes

- **fabryk-fts:** Use `floor_char_boundary` to prevent UTF-8 panics in snippet
  truncation (two separate fixes for multi-byte character handling).
- **CI:** Upgrade `actions/checkout` v4 to v6 for Node.js 24 support.
- **serde_yaml:** Migrate all references to `yaml_serde` (crate rename).
- **clippy:** Collapse nested if-let statements into let-chains across
  `ecl-stages` and `fabryk-cli` (20 instances).

---

## Dependency Updates

| Dependency | From | To | Notes |
|-----------|------|----|-------|
| `apache-avro` | 0.17 | 0.21 | |
| `lancedb` | 0.26 | 0.27 | Updated `create_table` API for `Scannable` trait change |
| `pgp` | 0.14 | 0.19 | |
| `rdkafka` | 0.36 | 0.39 | Updated `Delivery` from tuple to struct |
| `reqwest` | 0.12 | 0.13 | ecl-core (non-workspace) |
| `toml_edit` | 0.22 | 0.25 | textrynum-util |

**Held back:** `russh`/`russh-keys` remain at 0.46 due to transitive
`rand_core` pre-release conflict with `rmcp` (MCP crate).

---

## Crates Updated

| Crate | Change |
|-------|--------|
| `ecl-pipeline-spec` | New crate — TOML specification layer |
| `ecl-pipeline-state` | New crate — execution state & checkpoint persistence |
| `ecl-pipeline-topo` | New crate — topology resolution, traits, DAG scheduling |
| `ecl-pipeline` | New crate — runner orchestration engine |
| `ecl-stages` | New crate — 15 built-in stage implementations |
| `ecl-adapter-fs` | New crate — filesystem source adapter |
| `ecl-adapter-gcs` | New crate — GCS source adapter |
| `ecl-adapter-gdrive` | New crate — Google Drive source adapter |
| `ecl-adapter-slack` | New crate — Slack source adapter (stub) |
| `ecl-adapter-zapier` | New crate — Zapier webhook push source adapter |
| `ecl-adapter-sftp` | New crate — SFTP source adapter |
| `ecl-sink-kafka` | New crate — Kafka producer with Avro |
| `ecl-sink-gcs` | New crate — GCS file writer |
| `ecl-secrets` | New crate — pluggable secret resolution |
| `ecl-cli` | Pipeline subcommands, adapter/stage registry |
| `fabryk-cli` | Shared config infrastructure, collapsible-if fixes |
| `fabryk-fts` | UTF-8 boundary panic fixes |
| `fabryk-mcp` | Re-exports, tool registry improvements |
| `fabryk-mcp-core` | Service state logging |
| `fabryk-vector` | lancedb 0.27 API migration |
| `textrynum-util` | toml_edit bump |
| `ecl-core` | reqwest bump |
