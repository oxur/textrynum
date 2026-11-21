# Banyan Phase 4 — Full parity

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 1-73. Original bytes are recoverable from import commit 2d006a7. -->

# Phase 4 Detailed Implementation Plan: "Full Parity"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1), 0029 (Phase 2), 0030 (Phase 3)
**Goal:** Complete feature parity with all four Banyan customer pipelines — BigQuery source for Chime, expression evaluation for computed fields, UPC/auth code specialized stages, environment config overlays, stateful processing for Acme Inc. loyalty, and full E2E tests for Chime and loyalty.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 10.1: Expression Evaluation Stage](#2-milestone-101-expression-evaluation-stage)
3. [Milestone 10.2: UPC Check Digit Stage](#3-milestone-102-upc-check-digit-stage)
4. [Milestone 10.3: Auth Code Normalization Stage](#4-milestone-103-auth-code-normalization-stage)
5. [Milestone 10.4: BigQuery Source Adapter](#5-milestone-104-bigquery-source-adapter)
6. [Milestone 10.5: Environment Config Overlays](#6-milestone-105-environment-config-overlays)
7. [Milestone 10.6: Stateful Processing (Loyalty)](#7-milestone-106-stateful-processing-loyalty)
8. [Milestone 10.7: Chime End-to-End Integration Test](#8-milestone-107-chime-end-to-end-integration-test)
9. [Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test](#9-milestone-108-acme-inc-loyalty-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 What Remains

After Phases 1–3, three gaps remain:

| Gap | Customer | Why it's needed |
|-----|----------|----------------|
| BigQuery as a data source | Chime | Rewards calculation queries BQ tables, not files |
| Expression evaluation | Acme Inc. | Digital price correction (`amount / 100`), conditional BOPIS logic |
| UPC check digit | Acme Inc., Giant Eagle | GS1 check digit algorithm for retail UPCs |
| Auth code normalization | Acme Inc., Affinity, Giant Eagle | Strip leading zeros, pad to N digits |
| Environment overlays | All | One base spec, three deployments (sand/test/prod) |
| Stateful processing | Acme Inc. loyalty | Database-backed diff/update for consumer tokens |

### 1.2 Chime Pipeline Shape

Chime is fundamentally different from the other three — it's SQL-driven, not file-streaming:

```
BIN files (GCS) ─→ csv_parse ─→ collect BINs ─────┐
                                                    │
BigQuery Source ─→ query(receipts × items × BINs) ──┤
                                                    │
                              rewards_calculation ←─┘
                                     │
                                     ├─→ csv_emit (GCS output)
                                     └─→ s3_sink (AWS output)
```

### 1.3 Acme Inc. Loyalty Pipeline Shape

```
GCS (loyalty files) ─→ decrypt(PGP) ─→ csv_parse ─→ field_map (hash phones)
                                                          │
                                           stateful_diff (PostgreSQL)
                                                          │
                                              ┌───────────┴───────────┐
                                              │                       │
                                       inserts/updates            deletes
                                              │                       │
                                              └───────────┬───────────┘
                                                          │
                                                   kafka_sink (Avro)
```

---


Historical milestone moved to [2. Milestone 10.1: Expression Evaluation Stage](slice01-expression-evaluation-stage/slice-plan.md).

Historical milestone moved to [3. Milestone 10.2: UPC Check Digit Stage](slice02-upc-check-digit-stage/slice-plan.md).

Historical milestone moved to [4. Milestone 10.3: Auth Code Normalization Stage](slice03-auth-code-normalization-stage/slice-plan.md).

Historical milestone moved to [5. Milestone 10.4: BigQuery Source Adapter](slice04-bigquery-source-adapter/slice-plan.md).

Historical milestone moved to [6. Milestone 10.5: Environment Config Overlays](slice05-environment-config-overlays/slice-plan.md).

Historical milestone moved to [7. Milestone 10.6: Stateful Processing (Loyalty)](slice06-stateful-processing-loyalty/slice-plan.md).

Historical milestone moved to [8. Milestone 10.7: Chime End-to-End Integration Test](slice07-chime-end-to-end-integration-test/slice-plan.md).

Historical milestone moved to [9. Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test](slice08-acme-inc-loyalty-end-to-end-integration-test/slice-plan.md).

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 1187-1355. Original bytes are recoverable from import commit 2d006a7. -->

## 10. Cross-Cutting Concerns

### 10.1 New Dependencies

```toml
# Root Cargo.toml workspace deps:
evalexpr = "14"
gcp-bigquery-client = "0.25"
```

### 10.2 New Crates

| Crate | Purpose |
|-------|---------|
| `ecl-adapter-bigquery` | BigQuery query source |

### 10.3 New Stages in `ecl-stages`

| Stage | File | Type |
|-------|------|------|
| `ExpressionStage` | `expression.rs` | Per-item |
| `UpcCheckDigitStage` | `upc_check_digit.rs` | Per-item |
| `AuthCodeNormStage` | `auth_code_norm.rs` | Per-item |
| `JsonLinesParseStage` | `json_lines_parse.rs` | Per-item (fan-out) |
| `StatefulDiffStage` | `stateful_diff.rs` | Batch (needs StateStore) |

### 10.4 Registry Updates

Register all new stages and the BigQuery adapter:

```rust
"expression" => ExpressionStage::from_params(&spec.params)?,
"upc_check_digit" => UpcCheckDigitStage::from_params(&spec.params)?,
"auth_code_norm" => AuthCodeNormStage::from_params(&spec.params)?,
"json_lines_parse" => JsonLinesParseStage,
"stateful_diff" => StatefulDiffStage::new(&spec.params, state_store.clone())?,
```

Note: `StatefulDiffStage` needs access to the `StateStore` — the registry must pass it through.

### 10.5 StateStore Trait Extension Impact

Adding `state_get`/`state_set`/`state_delete`/`state_keys`/`state_batch_set` to the `StateStore` trait is a breaking change for existing implementations. Both `RedbStateStore` and `InMemoryStateStore` need implementations.

For `RedbStateStore`: use dynamically-named redb tables (`state_{table_name}`).
For `InMemoryStateStore`: use `BTreeMap<String, BTreeMap<String, Vec<u8>>>`.

Provide default implementations that return `StateError::Unsupported` so the trait change is less disruptive:

```rust
async fn state_get(&self, _table: &str, _key: &str) -> Result<Option<Vec<u8>>, StateError> {
    Err(StateError::Unsupported("state_get not implemented".to_string()))
}
```

Then override in RedbStateStore and InMemoryStateStore.

---

## 11. Verification Checklist

### Phase 4 Complete

- [ ] Expression evaluation works (arithmetic, conditionals, functions)
- [ ] UPC check digit correctly computes GS1 algorithm
- [ ] Auth code normalization strips/pads correctly
- [ ] BigQuery adapter executes queries and returns results
- [ ] JSON lines parse fans out query results into records
- [ ] Environment overlays deep-merge specs correctly
- [ ] Stateful diff detects inserts, updates, deletes across runs
- [ ] Chime E2E: BINs + BQ data → reward calculation → CSV output
- [ ] Acme Inc. loyalty E2E: CSV → diff → Kafka (insert/update/delete)
- [ ] All Phase 1/2/3 tests still pass

### Full Project Complete

- [ ] **Affinity** (Phase 1): GCS CSV → parse → map → validate → Kafka
- [ ] **Giant Eagle** (Phase 2): 5 CSVs → multi-stream → join → aggregate → assemble → Kafka
- [ ] **Acme Inc. Receipt** (Phase 3): SFTP → PGP decrypt → dedup → transform → Kafka
- [ ] **Chime** (Phase 4): BINs + BigQuery → reward calculation → CSV/S3
- [ ] **Acme Inc. Loyalty** (Phase 4): CSV → stateful diff → Kafka
- [ ] **Environment overlays**: One spec, three deployments
- [ ] **Scheduling**: Cron-based pipeline execution
- [ ] **Pipeline chaining**: Ingestion triggers transformation
- [ ] **Secret management**: GCP Secret Manager integration
- [ ] **Zero customer-specific Rust code**: All pipelines are TOML configurations

---

## Appendix A: Milestone Dependency Graph

```
10.1 Expression ─────────────────────────┐
                                          │
10.2 UPC Check Digit ───────────────────┤
                                          │
10.3 Auth Code Norm ────────────────────┤  (all independent)
                                          │
10.4 BigQuery + JSON Lines Parse ───────┤
                                          │
10.5 Environment Overlays ──────────────┤
                                          │
10.6 Stateful Diff ─────────────────────┤
                                          │
                         ┌────────────────┘
                         │
              10.7 Chime E2E (needs 10.1, 10.4)
                         │
              10.8 Loyalty E2E (needs 10.6)
```

**All of 10.1–10.6 can proceed in parallel.** 10.7 depends on 10.1 + 10.4. 10.8 depends on 10.6.

## Appendix B: Complete Project Statistics

### All Phases Combined

| Category | Count |
|----------|-------|
| New crates | 10 |
| New stage types | 20 |
| New source adapters | 3 (GCS, SFTP, BigQuery) |
| New sink adapters | 3 (Kafka, GCS, S3) |
| Modified existing crates | 8 |
| Total milestones | 32 |
| Estimated test count | 200+ |

### New Crates (All Phases)

| Crate | Phase | Purpose |
|-------|-------|---------|
| `ecl-adapter-gcs` | 1 | Google Cloud Storage source |
| `ecl-sink-kafka` | 1 | Kafka producer + Avro + Schema Registry |
| `ecl-sink-gcs` | 1 | GCS file writer |
| `ecl-adapter-sftp` | 3 | SFTP source |
| `ecl-sink-s3` | 3 | AWS S3 file writer |
| `ecl-secrets` | 3 | Pluggable secret resolution |
| `ecl-adapter-bigquery` | 4 | BigQuery query source |

### All Stage Types (All Phases)

| Stage | Phase | Type | Purpose |
|-------|-------|------|---------|
| `csv_parse` | 1 | Per-item (fan-out) | CSV → Records |
| `field_map` | 1 | Per-item | Rename, drop, set, copy, date parse, pad, regex, nest |
| `validate` | 1 | Per-item | Configurable validation rules |
| `kafka_sink` | 1 | Per-item (terminal) | Kafka producer with Avro |
| `gcs_sink` | 1 | Per-item (terminal) | GCS file writer |
| `join` | 2 | Batch | Merge two streams by key |
| `aggregate` | 2 | Batch | Group-by with aggregation functions |
| `lookup` | 2 | Per-item | Static value mapping tables |
| `date_parse` | 2 | Per-item | String → RFC3339 datetime |
| `timezone` | 2 | Per-item | Local time → UTC by ZIP code |
| `decompress` | 2 | Per-item (fan-out) | ZIP/gzip extraction |
| `assemble` | 2 | Batch | Multi-stream → nested structure |
| `pgp_decrypt` | 3 | Per-item | PGP decryption |
| `deduplicate` | 3 | Batch | Remove duplicate files by hash |
| `s3_sink` | 3 | Per-item (terminal) | AWS S3 file writer |
| `expression` | 4 | Per-item | Arithmetic, conditionals, functions |
| `upc_check_digit` | 4 | Per-item | GS1 check digit algorithm |
| `auth_code_norm` | 4 | Per-item | Strip zeros, pad to width |
| `json_lines_parse` | 4 | Per-item (fan-out) | JSON lines → Records |
| `stateful_diff` | 4 | Batch | Database-backed diff across runs |

### The Final Insight

**20 reusable stage types + 7 adapter/sink crates = infrastructure to run ANY batch data pipeline as TOML configuration.**

No customer-specific Rust code. Acme Inc., Affinity, Chime, and Giant Eagle are all just `.toml` files using combinations of these generic building blocks. Adding a new customer means writing a new TOML spec, not new code.

## Reconstructed slice locations

- [2. Milestone 10.1: Expression Evaluation Stage](slice01-expression-evaluation-stage/slice-plan.md)
- [3. Milestone 10.2: UPC Check Digit Stage](slice02-upc-check-digit-stage/slice-plan.md)
- [4. Milestone 10.3: Auth Code Normalization Stage](slice03-auth-code-normalization-stage/slice-plan.md)
- [5. Milestone 10.4: BigQuery Source Adapter](slice04-bigquery-source-adapter/slice-plan.md)
- [6. Milestone 10.5: Environment Config Overlays](slice05-environment-config-overlays/slice-plan.md)
- [7. Milestone 10.6: Stateful Processing (Loyalty)](slice06-stateful-processing-loyalty/slice-plan.md)
- [8. Milestone 10.7: Chime End-to-End Integration Test](slice07-chime-end-to-end-integration-test/slice-plan.md)
- [9. Milestone 10.8: Acme Inc. Loyalty End-to-End Integration Test](slice08-acme-inc-loyalty-end-to-end-integration-test/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
