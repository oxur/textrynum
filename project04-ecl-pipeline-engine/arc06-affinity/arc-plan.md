# Banyan Phase 1 — Make Affinity work

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1-124. Original bytes are recoverable from import commit 3a39aea. -->

# Phase 1 Detailed Implementation Plan: "Make Affinity Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0026 (Requirements), 0027 (High-Level Plan)
**Goal:** Run the Affinity (Partner 290) pipeline end-to-end in ECL — GCS CSV files → parse → map → validate → Kafka (Avro) + GCS error output.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 7.1: Record Type & CSV Parse Stage](#2-milestone-71-record-type--csv-parse-stage)
3. [Milestone 7.2: Field Mapping Stage](#3-milestone-72-field-mapping-stage)
4. [Milestone 7.3: GCS Source Adapter](#4-milestone-73-gcs-source-adapter)
5. [Milestone 7.4: Validation Stage](#5-milestone-74-validation-stage)
6. [Milestone 7.5: Kafka Sink Stage](#6-milestone-75-kafka-sink-stage)
7. [Milestone 7.6: GCS Sink Stage](#7-milestone-76-gcs-sink-stage)
8. [Milestone 7.7: File Lifecycle Management](#8-milestone-77-file-lifecycle-management)
9. [Milestone 7.8: Affinity End-to-End Integration Test](#9-milestone-78-affinity-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Target Pipeline Shape

```
GCS Source (input/) ─→ CSV Parse ─→ Field Map ─→ Validate ─┬─→ Kafka Sink (valid)
                                                            └─→ GCS Sink (errors)
```

### 1.2 TOML Spec Preview (Affinity)

```toml
name = "affinity-290-transformation"
version = 1
output_dir = "./output/affinity-290"

[sources.transactions]
kind = "gcs"
bucket = "byn-prod-finx-290-affinity"
prefix = "staging/"
pattern = "Banyan-txn-file*.csv"
credentials = { type = "application_default" }

[stages.parse]
adapter = "csv_parse"
resources = { reads = ["gcs-files"], creates = ["parsed-records"] }
[stages.parse.params]
has_headers = true
delimiter = ","
columns = [
  { name = "Channel_Aggregator_ID", type = "string" },
  { name = "Program_ID", type = "string" },
  { name = "Account_ID", type = "string" },
  # ... (19 columns total)
]

[stages.map]
adapter = "field_map"
resources = { reads = ["parsed-records"], creates = ["mapped-records"] }
[stages.map.params]
rename = [
  { from = "Account_ID", to = "finx_consumer_token" },
  { from = "Transaction_Amount", to = "total_amount" },
  { from = "Merchant_Location_MID", to = "finx_mid" },
]
set = [
  { field = "country", value = "US" },
  { field = "currency", value = "USD" },
  { field = "byn_partner_id", value = 290 },
]
drop = ["Channel_Aggregator_ID", "Program_ID", "Card_BIN",
        "Account_Postal_Code", "Transaction_Settlement_Date",
        "Transaction_Code", "Merchant_Location_Category_Code"]

[stages.validate]
adapter = "validate"
resources = { reads = ["mapped-records"], creates = ["validated-records"] }
skip_on_error = true
[stages.validate.params]
rules = [
  { field = "finx_transaction_id", check = "required", severity = "hard" },
  { field = "card_last_four", check = "regex", pattern = "^\\d{4}$", severity = "hard" },
  { field = "purchase_ts", check = "date_range", min = "-10y", max = "+10y", severity = "hard" },
]

[stages.kafka_out]
adapter = "kafka_sink"
resources = { reads = ["validated-records"] }
[stages.kafka_out.params]
topic = "by_production_transaction_event_avro_0"
bootstrap_servers = "${KAFKA_BROKERS}"
schema_registry_url = "${SCHEMA_REGISTRY_URL}"
security_protocol = "SASL_SSL"
sasl_mechanism = "SCRAM-SHA-256"

[stages.error_out]
adapter = "gcs_sink"
resources = { reads = ["validated-records"] }
[stages.error_out.params]
bucket = "byn-prod-finx-290-affinity"
prefix = "error/"
format = "json_lines"
filter = "errors_only"
```

### 1.3 Key Design Decisions

**D1: Record as part of PipelineItem** — Add `record: Option<serde_json::Map<String, Value>>` to `PipelineItem`. Stages that operate on structured data read/write this field. Stages that operate on raw bytes use `content`. This preserves backward compatibility with existing stages (extract, normalize, filter, emit).

**D2: Fan-out model** — CSV parse is a fan-out stage: one PipelineItem (file) → N PipelineItems (rows). The Stage trait already supports this via `Result<Vec<PipelineItem>>`. Each output item gets a unique ID: `{file_id}:row:{line_number}`.

**D3: Validation produces metadata, not separate streams** — Failed validation adds `_validation_errors: [...]` to the record's metadata. Downstream stages (kafka_sink, gcs_sink) use a `filter` param to select valid-only or errors-only items. This avoids needing multi-stream support in Phase 1.

**D4: Sinks are regular stages** — Kafka and GCS sinks implement the `Stage` trait. They return `Ok(vec![])` (terminal, no output items). Registered in `stage_lookup_fn` like any other stage.

**D5: Environment variable interpolation** — Stage params support `${ENV_VAR}` syntax. Resolved at stage construction time in the registry, not at parse time. Keeps PipelineSpec serializable for checkpoints.

---


Historical milestone moved to [2. Milestone 7.1: Record Type & CSV Parse Stage](slice01-record-type-csv-parse-stage/slice-plan.md).

Historical milestone moved to [3. Milestone 7.2: Field Mapping Stage](slice02-field-mapping-stage/slice-plan.md).

Historical milestone moved to [4. Milestone 7.3: GCS Source Adapter](slice03-gcs-source-adapter/slice-plan.md).

Historical milestone moved to [5. Milestone 7.4: Validation Stage](slice04-validation-stage/slice-plan.md).

Historical milestone moved to [6. Milestone 7.5: Kafka Sink Stage](slice05-kafka-sink-stage/slice-plan.md).

Historical milestone moved to [7. Milestone 7.6: GCS Sink Stage](slice06-gcs-sink-stage/slice-plan.md).

Historical milestone moved to [8. Milestone 7.7: File Lifecycle Management](slice07-file-lifecycle-management/slice-plan.md).

Historical milestone moved to [9. Milestone 7.8: Affinity End-to-End Integration Test](slice08-affinity-end-to-end-integration-test/slice-plan.md).

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 1317-1477. Original bytes are recoverable from import commit 3a39aea. -->

## 10. Cross-Cutting Concerns

### 10.1 Workspace Dependencies

Add to root `Cargo.toml` `[workspace.dependencies]`:

```toml
csv = "1"
regex = "1"
rdkafka = { version = "0.37", features = ["cmake-build", "ssl", "sasl"] }
apache-avro = "0.17"
google-cloud-storage = { version = "0.22", features = ["default-tls"] }
google-cloud-auth = { version = "0.19", features = ["default-tls"] }
```

### 10.2 Linting Rules

All new crates inherit workspace linting:

```toml
[lints.clippy]
unwrap_used = "deny"
panic = "deny"
```

All `#[cfg(test)]` modules use `#[allow(clippy::unwrap_used)]` per existing convention.

### 10.3 Error Handling Pattern

All new stages follow the established pattern:

- Parse errors → `StageError::Permanent` (config is wrong, won't fix itself)
- Network/transient errors → `StageError::Transient` (retry will help)
- Missing record → `StageError::Permanent` (pipeline is misconfigured)
- Individual row parse errors → skip or fail per config (not stage-level error)

### 10.4 Documentation

Each new public type gets a doc comment per TR-09 / DC-01:

- Types: one-line summary + field documentation
- Trait impls: note any non-obvious behavior
- Modules: `//!` header explaining purpose

### 10.5 Test Coverage Target

Per `CLAUDE-CODE-COVERAGE.md`: ≥95% coverage on all new code.

Each milestone's test list above is designed to cover:

- Happy path
- Error paths (invalid config, missing data, type conversion failures)
- Edge cases (empty input, single row, large input)
- Serde roundtrips for all config types
- Integration with existing types (`PipelineItem`, `StageContext`)

### 10.6 Backward Compatibility

**No breaking changes to existing types:**

- `PipelineItem` gains `record: Option<Record>` with `#[serde(default)]` — existing serialized checkpoints deserialize with `record: None`
- `SourceSpec` gains `Gcs` variant — existing TOML specs are unaffected
- `PipelineSpec` gains `lifecycle: Option<LifecycleSpec>` with `#[serde(default)]`
- All existing stages continue to work unchanged
- Existing integration tests must still pass

---

## 11. Verification Checklist

### Per-Milestone

- [ ] `make test` passes (all existing + new tests)
- [ ] `make lint` passes
- [ ] `make format` passes
- [ ] No compiler warnings
- [ ] Coverage ≥ 95% on new code
- [ ] Checked against AP-01 through AP-20
- [ ] Doc comments on all public items
- [ ] Serde roundtrip tests for all config types

### Phase 1 Complete

- [ ] Affinity pipeline spec parses from TOML
- [ ] GCS source lists and fetches objects
- [ ] CSV parse produces Records from file content
- [ ] Field map transforms Records per configuration
- [ ] Validation checks rules, classifies hard/soft
- [ ] Kafka sink serializes to Avro wire format and produces
- [ ] GCS sink writes error records
- [ ] File lifecycle moves files on success/failure
- [ ] End-to-end integration test passes
- [ ] Existing pipeline tests still pass (no regressions)

---

## Appendix A: Milestone Dependency Graph

```
7.1 Record + CSV Parse ──────────────────────────────┐
    │                                                 │
    ├── 7.2 Field Map ───────────────┐                │
    │                                │                │
    ├── 7.3 GCS Source ──────────────┤                │
    │                                │                │
    ├── 7.4 Validation ──────────────┤                │
    │                                │                │
    │   7.5 Kafka Sink ─────────────┤ (can parallel  │
    │                                │  with 7.3-7.4) │
    │   7.6 GCS Sink ──────────────┤                 │
    │                                │                │
    └───────────────────────────────┘                │
                                     │                │
                              7.7 File Lifecycle ─────┤
                                     │                │
                              7.8 Affinity E2E ───────┘
```

**Parallelizable work:**

- 7.2 + 7.3 can proceed in parallel after 7.1
- 7.4 can proceed in parallel with 7.3
- 7.5 + 7.6 can proceed in parallel with 7.2-7.4
- 7.7 depends on 7.3 (GCS operations)
- 7.8 depends on all prior milestones

## Appendix B: Files Changed/Created Summary

### New Files

| File | Milestone |
|------|-----------|
| `crates/ecl-stages/src/csv_parse.rs` | 7.1 |
| `crates/ecl-stages/src/field_map.rs` | 7.2 |
| `crates/ecl-adapter-gcs/Cargo.toml` | 7.3 |
| `crates/ecl-adapter-gcs/src/lib.rs` | 7.3 |
| `crates/ecl-stages/src/validate.rs` | 7.4 |
| `crates/ecl-sink-kafka/Cargo.toml` | 7.5 |
| `crates/ecl-sink-kafka/src/lib.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/producer.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/avro.rs` | 7.5 |
| `crates/ecl-sink-kafka/src/registry.rs` | 7.5 |
| `crates/ecl-sink-gcs/Cargo.toml` | 7.6 |
| `crates/ecl-sink-gcs/src/lib.rs` | 7.6 |
| `tests/fixtures/affinity_sample.csv` | 7.8 |
| `tests/fixtures/affinity_pipeline.toml` | 7.8 |
| `tests/integration/affinity_e2e.rs` | 7.8 |

### Modified Files

| File | Milestone | Change |
|------|-----------|--------|
| `Cargo.toml` (workspace) | 7.3, 7.5, 7.6 | Add new crate members + deps |
| `crates/ecl-pipeline-topo/src/traits.rs` | 7.1 | Add `Record` type alias + `record` field to `PipelineItem` |
| `crates/ecl-pipeline-spec/src/source.rs` | 7.3 | Add `Gcs` variant + `GcsSourceSpec` |
| `crates/ecl-pipeline-spec/src/lib.rs` | 7.7 | Add `lifecycle` field + `LifecycleSpec` |
| `crates/ecl-stages/src/lib.rs` | 7.1, 7.2, 7.4 | Add module declarations + re-exports |
| `crates/ecl-stages/Cargo.toml` | 7.1, 7.2 | Add `csv`, `regex`, `chrono` deps |
| `crates/ecl-cli/Cargo.toml` | 7.3, 7.5, 7.6 | Add new crate deps |
| `crates/ecl-cli/src/pipeline/registry.rs` | 7.1-7.6 | Register new stages + GCS adapter |
| `crates/ecl-pipeline/src/runner.rs` | 7.7 | Add lifecycle hooks |

## Reconstructed slice locations

- [2. Milestone 7.1: Record Type & CSV Parse Stage](slice01-record-type-csv-parse-stage/slice-plan.md)
- [3. Milestone 7.2: Field Mapping Stage](slice02-field-mapping-stage/slice-plan.md)
- [4. Milestone 7.3: GCS Source Adapter](slice03-gcs-source-adapter/slice-plan.md)
- [5. Milestone 7.4: Validation Stage](slice04-validation-stage/slice-plan.md)
- [6. Milestone 7.5: Kafka Sink Stage](slice05-kafka-sink-stage/slice-plan.md)
- [7. Milestone 7.6: GCS Sink Stage](slice06-gcs-sink-stage/slice-plan.md)
- [8. Milestone 7.7: File Lifecycle Management](slice07-file-lifecycle-management/slice-plan.md)
- [9. Milestone 7.8: Affinity End-to-End Integration Test](slice08-affinity-end-to-end-integration-test/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
