
<!-- Historical source: workbench/0027-ecl-banyan-pipeline-implementation-plan.md, lines 1-769. Original bytes are recoverable from import commit 2d006a7. -->

# 0027 — ECL Banyan Data Pipeline Implementation Plan

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0026 (Banyan Data Pipeline Requirements Analysis)
**Purpose:** Gap analysis between current ECL capabilities and Banyan pipeline needs, plus phased implementation plan.

---

## 1. Current ECL Capabilities

### 1.1 What ECL Has Today

| Layer | Capability | Status |
|-------|-----------|--------|
| **Sources** | Filesystem (local dir, glob/ext filters) | Complete |
| | Google Drive (OAuth2, folder traversal) | Complete |
| | Slack (stub adapter, fixture-based) | Stub |
| | Zapier (push webhook, multiple schemas) | Complete |
| **Stages** | ExtractStage (delegate to SourceAdapter.fetch) | Complete |
| | NormalizeStage (passthrough placeholder) | Skeleton |
| | FilterStage (glob include/exclude) | Complete |
| | EmitStage (write to output dir) | Complete |
| **Topology** | TOML spec parsing | Complete |
| | DAG resource graph with cycle detection | Complete |
| | Parallel batch scheduling | Complete |
| | Retry with exponential backoff | Complete |
| | Per-stage concurrency limits | Complete |
| **State** | Checkpoint/resume (redb or in-memory) | Complete |
| | Incrementality (content hash comparison) | Complete |
| | Config drift detection (blake3 spec hash) | Complete |
| **Execution** | Four-phase runner (enumerate, increment, execute, push) | Complete |
| | Push source event loop (select! + batch) | Complete |
| **CLI** | run, resume, status, inspect, items, diff | Complete |
| **Observability** | Tracing spans, structured logging, timing | Complete |

### 1.2 What ECL Does NOT Have

| Gap | Needed For | Priority |
|-----|-----------|----------|
| GCS source adapter | All customers | P0 |
| CSV parsing stage | All customers | P0 |
| Field mapping/transform stage | All customers | P0 |
| Kafka sink (Avro) | WAG, AFF, GE | P0 |
| Schema Registry client | WAG, AFF, GE | P0 |
| Avro serialization | WAG, AFF, GE | P0 |
| GCS sink (file writer) | All customers | P0 |
| Multi-stream join stage | WAG, GE | P0 |
| Aggregation stage | WAG, GE | P0 |
| Validation framework | WAG, AFF, GE | P0 |
| Conditional routing (fan-out) | All customers | P0 |
| File lifecycle management | All customers | P0 |
| Scheduling (cron triggers) | All customers | P0 |
| SFTP source adapter | WAG | P1 |
| S3 sink | Chime | P1 |
| PGP decryption stage | WAG | P1 |
| ZIP decompression stage | GE | P1 |
| Lookup table stage | WAG, GE | P1 |
| Date/time parsing (multi-format) | WAG, AFF, GE | P1 |
| Timezone conversion | WAG, GE | P1 |
| Secret management integration | All customers | P1 |
| BigQuery source adapter | Chime | P2 |
| Pipeline chaining/triggers | WAG (ingestion→transform) | P2 |
| Environment config overlays | All (sand/test/prod) | P2 |
| Deduplication stage | WAG | P2 |
| UPC check digit stage | WAG, GE | P2 |
| Stateful processing (DB-backed) | WAG loyalty | P3 |
| Hashing/tokenization stage | WAG loyalty | P3 |

---

## 2. Architecture Decisions

### 2.1 Design Principles

1. **Pipeline-as-configuration** — Each customer pipeline is a TOML spec, not code. Business logic encoded in stage parameters, not custom Rust per customer.

2. **Composable stages** — Small, focused stages that compose into complex pipelines. A CSV parse stage feeds a field map stage feeds a validate stage, etc.

3. **Multi-stream support** — Pipelines can define multiple named data streams (e.g., "transactions", "items", "stores") that flow independently through stages until a join stage merges them.

4. **Typed records** — Internal representation uses `serde_json::Value` (or a typed record abstraction) so stages can inspect and transform fields generically.

5. **Sink as stage** — Output destinations (Kafka, GCS, S3) are just terminal stages in the pipeline graph, not special infrastructure.

### 2.2 Key Architectural Extensions

#### Multi-Stream Pipeline Model

Current ECL processes items independently through a linear stage chain. Banyan pipelines require **multiple named data streams** that can be processed independently and then joined:

```
[transactions.csv] → parse → transform ──┐
[items.csv]        → parse → transform ──┤
[stores.csv]       → parse → transform ──┼── join_by_key → assemble_receipt → validate → kafka
[products.csv]     → parse → transform ──┤
[tenders.csv]      → parse → transform ──┘
```

**Proposed extension:** Add a `stream` field to `PipelineItem` and `StageSpec`. Stages declare which stream(s) they consume and produce. The DAG scheduler respects stream dependencies. Join stages consume multiple input streams and produce a single output stream.

#### Record-Oriented Processing

Current `PipelineItem` carries opaque `Arc<[u8]>` content. Banyan pipelines need **typed field access** for mapping, validation, and joining:

**Proposed extension:** Add a `Record` type (backed by `serde_json::Map<String, Value>`) that stages can read/write fields on. CSV parse produces Records. Field map transforms Records. Join merges Records. Kafka sink serializes Records to Avro.

```rust
/// A structured data record flowing through the pipeline.
#[derive(Debug, Clone)]
pub struct Record {
    pub fields: serde_json::Map<String, serde_json::Value>,
}
```

The existing `PipelineItem` gains an optional `record: Option<Record>` field alongside the existing `content: Arc<[u8]>` for backward compatibility.

#### Sink Stages

Current pipeline has only `EmitStage` (write to local dir). Need pluggable sinks:

```rust
pub trait SinkStage: Stage {
    // Inherits Stage::process() which can write externally
    // Convention: process() returns empty Vec (terminal stage)
}
```

No new trait needed — sinks are just stages that produce no output items. Register them in the stage lookup.

---

## 3. Implementation Plan

### Phase 1: Foundation (Milestone 7.x) — "Make Affinity Work"

**Goal:** Run the simplest Banyan pipeline (Affinity) end-to-end in ECL.

#### Milestone 7.1: Record Type & CSV Parse Stage
**Effort:** Medium | **Crates:** `ecl-pipeline-topo`, `ecl-stages`

Add the `Record` abstraction and a configurable CSV parsing stage.

**Changes:**
- Add `Record` type to `ecl-pipeline-topo` (serde_json::Map backed)
- Add `record: Option<Record>` to `PipelineItem`
- Implement `CsvParseStage` in `ecl-stages`:
  - Configurable via TOML params: `columns` (name + type), `delimiter`, `quote_char`, `has_headers`
  - Reads `PipelineItem.content` as CSV bytes
  - Produces one `PipelineItem` per CSV row with populated `Record`
  - Fan-out: one input item (file) → N output items (rows)
- Tests: parse sample Affinity CSV, verify field access

**TOML spec example:**
```toml
[[stages]]
name = "parse_transactions"
handler = "csv_parse"
[stages.params]
delimiter = ","
has_headers = true
columns = [
  { name = "Account_ID", type = "string" },
  { name = "Transaction_Amount", type = "string" },
  # ...
]
```

**Rust patterns:**
- EH-01: `From` impls for CSV parse errors
- AP-02: Accept `&[u8]` not `&Vec<u8>` for content
- ID-01: `#[non_exhaustive]` on `Record` if public
- TR-08: Derive Debug, Clone, PartialEq on Record

#### Milestone 7.2: Field Mapping Stage
**Effort:** Medium | **Crates:** `ecl-stages`

Configurable stage that renames, drops, and computes fields on Records.

**Changes:**
- Implement `FieldMapStage`:
  - `rename`: `{from: "Account_ID", to: "finx_consumer_token"}`
  - `drop`: list of fields to remove
  - `set`: `{field: "country", value: "US"}` — literal constants
  - `copy`: `{from: "Merchant_Location_Name", to: "location_name"}`
- All configured via TOML params
- Tests: verify field transformations on sample data

**TOML spec example:**
```toml
[[stages]]
name = "map_fields"
handler = "field_map"
[stages.params]
rename = [
  { from = "Account_ID", to = "finx_consumer_token" },
  { from = "Transaction_Amount", to = "total_amount" },
]
set = [
  { field = "country", value = "US" },
  { field = "currency", value = "USD" },
  { field = "byn_partner_id", value = 290 },
]
drop = ["Channel_Aggregator_ID", "Program_ID"]
```

#### Milestone 7.3: GCS Source Adapter
**Effort:** Medium | **Crates:** `ecl-adapter-gcs` (new crate)

Pull-based adapter that lists and fetches objects from GCS buckets.

**Changes:**
- New crate `ecl-adapter-gcs`
- Implement `SourceAdapter` trait:
  - `enumerate()`: List objects by prefix + glob pattern
  - `fetch()`: Download object content as bytes
- Auth: service account JSON, application default credentials, or env var
- Add `Gcs(GcsSourceSpec)` variant to `SourceSpec` enum
- Register in CLI registry

**Spec fields:**
```toml
[[sources]]
kind = "gcs"
[sources.config]
bucket = "byn-prod-finx-290-affinity"
prefix = "input/"
pattern = "Banyan-txn-file*.csv"
```

**Rust patterns:**
- CA-02: Async functions return Send futures
- CA-05: No blocking I/O in async (use reqwest or google-cloud-storage)
- CA-15: Service type should be Clone (Arc<Inner> pattern)
- EH-01: From impls for GCS errors → SourceError

#### Milestone 7.4: Validation Stage
**Effort:** Medium | **Crates:** `ecl-stages`

Configurable record validation with hard/soft failure routing.

**Changes:**
- Implement `ValidateStage`:
  - Rules defined in TOML params
  - Rule types: `required` (field not null), `regex` (pattern match), `range` (numeric/date bounds), `length` (string length)
  - Classification: `hard` (drop record) vs `soft` (warn, continue)
  - Failed records tagged with error metadata, routed to error stream
- Output: valid records pass through; invalid records get `_validation_errors` metadata

**TOML spec example:**
```toml
[[stages]]
name = "validate"
handler = "validate"
[stages.params]
rules = [
  { field = "finx_transaction_id", check = "required", severity = "hard" },
  { field = "card_last_four", check = "regex", pattern = "^\\d{4}$", severity = "hard" },
  { field = "purchase_ts", check = "range", min = "-10y", max = "+10y", severity = "hard" },
]
```

#### Milestone 7.5: Kafka Sink Stage
**Effort:** High | **Crates:** `ecl-sink-kafka` (new crate), `ecl-stages`

Kafka producer with Avro serialization and Schema Registry integration.

**Changes:**
- New crate `ecl-sink-kafka`
- Dependencies: `rdkafka` (librdkafka wrapper), `apache-avro`, `reqwest` (Schema Registry HTTP)
- Implement `KafkaSinkStage` (implements `Stage` trait):
  - Connects to Kafka brokers (SASL_SSL, SCRAM-SHA-256)
  - Registers/fetches Avro schema from Confluent Schema Registry
  - Serializes Records to Avro binary (magic byte + schema ID + payload)
  - Writes to configured topic
  - Returns empty vec (terminal stage)
- Credentials from secret management (env vars initially)

**TOML spec example:**
```toml
[[stages]]
name = "kafka_out"
handler = "kafka_sink"
[stages.params]
bootstrap_servers = "${KAFKA_BROKERS}"
topic = "by_production_transaction_event_avro_0"
schema_registry_url = "${SCHEMA_REGISTRY_URL}"
avro_schema_file = "schemas/transaction.avsc"
security_protocol = "SASL_SSL"
sasl_mechanism = "SCRAM-SHA-256"
sasl_username = "${KAFKA_USER}"
sasl_password = "${KAFKA_PASS}"
```

**Rust patterns:**
- CA-03: Use spawn_blocking for rdkafka sync operations if needed
- CA-09: Handle producer errors properly
- AP-06: No unwrap() in production Kafka code
- EH-03: Custom KafkaError type with From impls

#### Milestone 7.6: GCS Sink Stage
**Effort:** Low-Medium | **Crates:** `ecl-sink-gcs` (new crate), `ecl-stages`

Write records or raw content to GCS as files.

**Changes:**
- New crate `ecl-sink-gcs`
- Implement `GcsSinkStage`:
  - Write mode: `json_lines` (one JSON per line), `csv`, `raw`
  - Configurable bucket + prefix
  - File naming: timestamp-based or configurable template
- Used for error output, JSON review files, archives

#### Milestone 7.7: File Lifecycle Management
**Effort:** Medium | **Crates:** `ecl-pipeline`, `ecl-stages`

Manage the input → staging → historical/error lifecycle.

**Changes:**
- Add lifecycle config to pipeline spec:
  ```toml
  [lifecycle]
  staging_prefix = "staging/"
  historical_prefix = "historical/"
  error_prefix = "error/"
  on_success = "move_to_historical"
  on_failure = "move_to_input"
  ```
- Runner hooks: pre-run (stage files), post-run (archive/revert)
- GCS move operations (copy + delete)

#### Milestone 7.8: Integration Test — Affinity Pipeline
**Effort:** Medium | **Crates:** integration tests

End-to-end test of the Affinity pipeline.

**Changes:**
- Create `pipelines/affinity.toml` spec
- Create sample CSV test data
- Mock or local Kafka (testcontainers or file sink)
- Verify: CSV parsed, fields mapped, validated, output correct
- This validates the entire Phase 1 stack

---

### Phase 2: Multi-Stream & Joins (Milestone 8.x) — "Make Giant Eagle Work"

**Goal:** Support multi-file pipelines with joins and aggregations.

#### Milestone 8.1: Named Data Streams
**Effort:** High | **Crates:** `ecl-pipeline-spec`, `ecl-pipeline-topo`, `ecl-pipeline`

Extend the pipeline model to support multiple named data streams.

**Changes:**
- Add `stream: Option<String>` to `PipelineItem`
- Add `input_streams` and `output_stream` to `StageSpec`
- Update DAG scheduler to respect stream dependencies
- Sources produce items tagged with their stream name
- Stages filter input by stream

**TOML spec example:**
```toml
[[sources]]
name = "transactions"
kind = "gcs"
stream = "transactions"
[sources.config]
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*transaction*.csv"

[[sources]]
name = "items"
kind = "gcs"
stream = "items"
[sources.config]
bucket = "byn-prod-merchant-296-giant-eagle-inc"
prefix = "staging/"
pattern = "*item*.csv"
```

#### Milestone 8.2: Join Stage (CoGroupByKey)
**Effort:** High | **Crates:** `ecl-stages`

Join records from multiple streams by key.

**Changes:**
- Implement `JoinStage`:
  - Collects records from N input streams
  - Groups by configurable key field(s)
  - Join types: `inner`, `left`, `full`
  - Output: merged record with fields from all streams
  - Handles missing keys (left join produces nulls)
- Memory management: bounded buffer, spill to disk for large joins

**TOML spec example:**
```toml
[[stages]]
name = "join_items_products"
handler = "join"
input_streams = ["items", "products"]
output_stream = "enriched_items"
[stages.params]
join_type = "left"
left_key = "upc"
right_key = "upc"
```

**Rust patterns:**
- PF-01: Use HashMap for join (O(1) lookup)
- CA-13: Yield periodically for large datasets
- AP-14: Don't collect into Vec unnecessarily

#### Milestone 8.3: Aggregation Stage
**Effort:** High | **Crates:** `ecl-stages`

Group-by with configurable aggregate functions.

**Changes:**
- Implement `AggregateStage`:
  - Group by key field(s)
  - Aggregate functions: `sum`, `max`, `min`, `count`, `first`, `last`, `collect` (into array)
  - Output: one record per group with aggregated fields
- Used for: tender aggregation, cash back calculation, item totaling

**TOML spec example:**
```toml
[[stages]]
name = "aggregate_tenders"
handler = "aggregate"
input_streams = ["tenders"]
output_stream = "aggregated_tenders"
[stages.params]
group_by = ["transaction_id"]
aggregates = [
  { field = "charged_amount", function = "sum", output = "total_charged" },
  { field = "tax_amount", function = "max", output = "tax_amount" },
  { field = "shipping_amount", function = "max", output = "shipping_amount" },
]
```

#### Milestone 8.4: Lookup Table Stage
**Effort:** Medium | **Crates:** `ecl-stages`

Static lookup table for value mapping (payment types, schemes, etc.).

**Changes:**
- Implement `LookupStage`:
  - Inline table in TOML or external CSV/JSON file
  - Map field values: input → output
  - Default value for unmapped inputs
  - Multiple lookups per stage

**TOML spec example:**
```toml
[[stages]]
name = "map_payment_types"
handler = "lookup"
[stages.params]
lookups = [
  {
    field = "payment_type",
    output = "canonical_payment_type",
    default = "OTHER",
    table = { "Credit Card" = "CREDIT", "Debit Card" = "DEBIT", "Cash" = "CASH" }
  },
  {
    field = "scheme",
    output = "canonical_scheme",
    table = { "Visa" = "VI", "V" = "VI", "Mastercard" = "MC", "M" = "MC" }
  },
]
```

#### Milestone 8.5: Date/Time & Timezone Stages
**Effort:** Medium | **Crates:** `ecl-stages`

Parse dates from various formats and convert timezones.

**Changes:**
- `DateParseStage`: Parse string field to RFC3339 datetime
  - Configurable input format (strftime pattern)
  - Output as ISO 8601 string
- `TimezoneStage`: Convert local time to UTC using ZIP code lookup
  - Depends on `uszipcode` equivalent (or embedded lookup table)

#### Milestone 8.6: ZIP Decompression Stage
**Effort:** Low | **Crates:** `ecl-stages`

Decompress ZIP archives.

**Changes:**
- Implement `DecompressStage`:
  - Supports ZIP format (via `zip` crate)
  - Fan-out: one ZIP → N extracted files (as PipelineItems)
  - Preserves original filename in metadata

#### Milestone 8.7: Receipt Assembly Stage
**Effort:** Medium | **Crates:** `ecl-stages`

Merge multiple streams into a nested output structure.

**Changes:**
- Implement `AssembleStage`:
  - Takes joined/aggregated records
  - Builds nested JSON structure matching canonical output schema
  - Configurable field placement via template

#### Milestone 8.8: Integration Test — Giant Eagle Pipeline
**Effort:** Medium | **Crates:** integration tests

End-to-end test of the multi-stream Giant Eagle pipeline.

---

### Phase 3: Cloud Integration (Milestone 9.x) — "Production Ready"

**Goal:** Complete cloud integration, scheduling, and operational features.

#### Milestone 9.1: SFTP Source Adapter
**Effort:** Medium | **Crates:** `ecl-adapter-sftp` (new)

For Acme Inc. SFTP ingestion.

**Changes:**
- New crate using `ssh2` or `russh` crate
- Implement `SourceAdapter`: enumerate files on SFTP, fetch content
- Auth: private key or password (from secret management)

#### Milestone 9.2: S3 Sink Stage
**Effort:** Low-Medium | **Crates:** `ecl-sink-s3` (new)

For Chime output to AWS S3.

**Changes:**
- New crate using `aws-sdk-s3`
- Similar interface to GCS sink

#### Milestone 9.3: PGP Decryption Stage
**Effort:** Medium | **Crates:** `ecl-stages`

For Acme Inc. encrypted files.

**Changes:**
- Implement `DecryptStage`:
  - PGP decryption using `pgp` or `sequoia-openpgp` crate
  - Key from secret management (env var or file path)
  - Input: encrypted bytes → Output: decrypted bytes

#### Milestone 9.4: Scheduling & Cron Triggers
**Effort:** Medium | **Crates:** `ecl-cli`, `ecl-scheduler` (new)

Run pipelines on schedules.

**Changes:**
- Cron expression parsing (via `cron` crate)
- Scheduler daemon mode for `ecl-cli`
- Or: integrate with system cron / systemd timers (simpler)

#### Milestone 9.5: Secret Management
**Effort:** Medium | **Crates:** `ecl-secrets` (new)

Pluggable secret resolution for credentials.

**Changes:**
- `${ENV_VAR}` interpolation in TOML specs (may already exist partially)
- `${secret:gcp:secret-name}` for GCP Secret Manager
- `${file:/path/to/secret}` for file-based secrets
- Used by: Kafka credentials, PGP keys, OAuth tokens, API keys

#### Milestone 9.6: Pipeline Chaining
**Effort:** Medium | **Crates:** `ecl-pipeline`

One pipeline triggers another on completion.

**Changes:**
- Add `triggers` to pipeline spec:
  ```toml
  [triggers]
  on_success = ["transformation_pipeline"]
  ```
- Runner calls next pipeline after successful completion
- Pass context (e.g., staging folder path) via environment

#### Milestone 9.7: Deduplication Stage
**Effort:** Medium | **Crates:** `ecl-stages`

For Acme Inc. file deduplication by content hash.

**Changes:**
- `DeduplicateStage`:
  - Group by configurable key
  - Within group: compare content hashes
  - Keep newest (by timestamp) or first
  - Move duplicates to dedup folder

#### Milestone 9.8: Integration Tests — Acme Inc. Pipeline
**Effort:** High | **Crates:** integration tests

End-to-end Acme Inc. receipt pipeline test.

---

### Phase 4: Advanced Features (Milestone 10.x) — "Full Parity"

#### Milestone 10.1: BigQuery Source Adapter
For Chime's SQL-based pipeline.

#### Milestone 10.2: Expression Evaluation Stage
Computed fields, arithmetic, string operations on Records.

#### Milestone 10.3: UPC Check Digit Stage
GS1 check digit calculation for retail UPCs.

#### Milestone 10.4: Auth Code Normalization Stage
Strip leading zeros, pad to N digits.

#### Milestone 10.5: Environment Config Overlays
Support sand/test/prod config variants from a single base spec.

#### Milestone 10.6: Stateful Processing (Loyalty)
Database-backed state for diff-based updates (Acme Inc. loyalty).

#### Milestone 10.7: Integration Test — Chime Pipeline

#### Milestone 10.8: Integration Test — Acme Inc. Loyalty Pipeline

---

## 4. Implementation Timeline

```
Phase 1 (Foundation):       ~8 milestones    "Affinity works"
Phase 2 (Multi-Stream):     ~8 milestones    "Giant Eagle works"
Phase 3 (Cloud Integration): ~8 milestones   "Acme Inc. works"
Phase 4 (Advanced):          ~8 milestones   "Full parity"
```

### Dependency Graph

```
                    ┌──────────────────┐
                    │ 7.1 Record + CSV │
                    └────────┬─────────┘
                    ┌────────┴─────────┐
               ┌────┤  7.2 Field Map   ├────┐
               │    └──────────────────┘    │
               │                            │
    ┌──────────┴───────┐         ┌──────────┴──────────┐
    │ 7.3 GCS Source   │         │ 7.4 Validation      │
    └──────────┬───────┘         └──────────┬──────────┘
               │                            │
    ┌──────────┴───────┐         ┌──────────┴──────────┐
    │ 7.5 Kafka Sink   │         │ 7.6 GCS Sink        │
    └──────────┬───────┘         └──────────┬──────────┘
               │                            │
               │    ┌──────────────────┐    │
               └────┤ 7.7 Lifecycle    ├────┘
                    └────────┬─────────┘
                    ┌────────┴─────────┐
                    │ 7.8 Affinity E2E │
                    └────────┬─────────┘
                             │
                    ┌────────┴─────────┐
                    │ 8.1 Named Streams│
                    └────────┬─────────┘
               ┌─────────────┼─────────────┐
    ┌──────────┴───┐  ┌──────┴──────┐  ┌───┴──────────┐
    │ 8.2 Join     │  │ 8.3 Agg     │  │ 8.4 Lookup   │
    └──────────┬───┘  └──────┬──────┘  └───┬──────────┘
               └─────────────┼─────────────┘
                    ┌────────┴─────────┐
                    │ 8.5-8.7 Date,    │
                    │ Decomp, Assemble │
                    └────────┬─────────┘
                    ┌────────┴─────────┐
                    │ 8.8 Giant Eagle  │
                    └────────┬─────────┘
                             │
                    (Phase 3 & 4 continue)
```

---

## 5. New Crates Summary

| Crate | Purpose | Dependencies |
|-------|---------|-------------|
| `ecl-adapter-gcs` | Google Cloud Storage source | google-cloud-storage, tokio |
| `ecl-adapter-sftp` | SFTP source | russh/ssh2, tokio |
| `ecl-sink-kafka` | Kafka producer + Avro + Schema Registry | rdkafka, apache-avro, reqwest |
| `ecl-sink-gcs` | GCS file writer | google-cloud-storage, tokio |
| `ecl-sink-s3` | AWS S3 file writer | aws-sdk-s3, tokio |
| `ecl-secrets` | Pluggable secret resolution | google-cloud-secret-manager |
| `ecl-scheduler` | Cron-based pipeline triggers | cron, tokio |

Existing crates that gain new stages:
| Crate | New Stages |
|-------|-----------|
| `ecl-stages` | CsvParse, FieldMap, Validate, Join, Aggregate, Lookup, DateParse, Timezone, Decompress, Decrypt, Deduplicate, Assemble, AuthCodeNorm, UpcCheckDigit, Expression |
| `ecl-pipeline-spec` | Record type, stream fields, lifecycle config |
| `ecl-pipeline-topo` | Stream-aware scheduling |
| `ecl-pipeline` | Lifecycle hooks, pipeline chaining |

---

## 6. Risk Assessment

### High Risk
- **Kafka/Avro integration** — rdkafka (librdkafka) has complex build requirements (CMake, libssl). Consider `rskafka` (pure Rust) as alternative.
- **Multi-stream joins** — Memory pressure for large datasets. Need spill-to-disk strategy.
- **Schema Registry** — HTTP-based, need to handle versioning and compatibility.

### Medium Risk
- **GCS auth** — Multiple auth methods (service account, ADC, workload identity). Must handle token refresh.
- **PGP decryption** — `sequoia-openpgp` is mature but heavy dependency tree.
- **Test data** — Need realistic sample data for each customer without exposing real PII.

### Low Risk
- **CSV parsing** — Well-understood problem, `csv` crate is excellent.
- **Field mapping** — Simple serde_json operations.
- **Validation** — Straightforward rule engine.

---

## 7. Success Criteria

### Phase 1 Complete When:
- [ ] Affinity pipeline runs end-to-end in ECL
- [ ] CSV files from GCS parsed correctly (19 columns)
- [ ] Fields mapped to canonical output
- [ ] Validation rules applied (last4, transaction_id, timestamps)
- [ ] Valid records written to Kafka (Avro format)
- [ ] Invalid records written to GCS error folder
- [ ] File lifecycle: staging → historical on success

### Phase 2 Complete When:
- [ ] Giant Eagle pipeline runs end-to-end
- [ ] 5 CSV file types parsed independently
- [ ] Items joined with products by UPC
- [ ] Tenders aggregated by transaction
- [ ] Receipt assembled from all streams
- [ ] Taxonomy extracted and published

### Phase 3 Complete When:
- [ ] Acme Inc. receipt pipeline runs end-to-end
- [ ] PGP decryption works
- [ ] Deduplication by MD5 hash works
- [ ] Pipeline scheduled via cron
- [ ] All credentials from secret management

### Phase 4 Complete When:
- [ ] Chime rewards pipeline runs (BigQuery source → CSV/S3 output)
- [ ] Acme Inc. loyalty pipeline runs (stateful processing)
- [ ] Environment overlays work (one spec, three deployments)

---

## 8. Recommended Starting Point

**Start with Milestone 7.1 (Record + CSV Parse)** because:
1. It's foundational — every subsequent stage depends on Records
2. It's well-scoped — CSV parsing is a solved problem
3. It's immediately testable — parse Affinity sample data
4. It exercises the existing stage trait system with a real-world use case
5. It validates the architecture decision of serde_json::Value for field representation

The `csv` crate handles all the parsing complexity. The interesting Rust work is in the `Record` type design, the fan-out from file→rows, and the TOML-driven schema configuration.
