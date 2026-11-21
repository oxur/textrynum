# Banyan Phase 3 — Make Acme Inc. work

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1-174. Original bytes are recoverable from import commit 2d006a7. -->

# Phase 3 Detailed Implementation Plan: "Make Acme Inc. Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1), 0029 (Phase 2)
**Goal:** Run the Acme Inc. (Partner 327) receipt pipeline end-to-end in ECL — SFTP ingestion, PGP decryption, deduplication, multi-file CSV transformation with joins/aggregations, Kafka output, file lifecycle, scheduled execution.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Milestone 9.1: Secret Management](#2-milestone-91-secret-management)
3. [Milestone 9.2: SFTP Source Adapter](#3-milestone-92-sftp-source-adapter)
4. [Milestone 9.3: PGP Decryption Stage](#4-milestone-93-pgp-decryption-stage)
5. [Milestone 9.4: Deduplication Stage](#5-milestone-94-deduplication-stage)
6. [Milestone 9.5: S3 Sink Stage](#6-milestone-95-s3-sink-stage)
7. [Milestone 9.6: Pipeline Chaining](#7-milestone-96-pipeline-chaining)
8. [Milestone 9.7: Scheduling & Cron Triggers](#8-milestone-97-scheduling--cron-triggers)
9. [Milestone 9.8: Acme Inc. End-to-End Integration Test](#9-milestone-98-acme-inc-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Acme Inc. Has Two Pipelines

**Pipeline A: Ingestion** (scheduled, SFTP → GCS staging)

```
SFTP Source ─→ GCS Sink (input/)
               │
               └─→ trigger Pipeline B
```

**Pipeline B: Transformation** (triggered by Pipeline A)

```
GCS (input/) ─→ decrypt(PGP) ─→ dedup(MD5) ─→ move_to_staging
                                                      │
┌─────────────────────────────────────────────────────┘
│
├── [transactions.csv]  ─→ parse ─→ map (price correct, order type, BOPIS) ──────────────────┐
├── [tenders.csv]       ─→ parse ─→ map (payment type/scheme, auth code clean) ─→ aggregate ──┤
├── [items.csv]         ─→ parse ─→ map (UPC check digit) ───────┐                            │
├── [products.csv]      ─→ parse ─→ map ─────────────────────────┤                            │
│                                                                 ├── join(items+products) ────┤
├── [stores.csv]        ─→ parse ─→ map (timezone resolve) ──────┤                            │
│                                                                 │                            │
│                                                         assemble_receipt ←──────────────────┘
│                                                              │
│                                                              ├─→ validate ─→ kafka (receipts)
│                                                              └─→ taxonomy_extract ─→ kafka (taxonomy)
│
└── lifecycle: staging ─→ historical (success) | input (failure)
```

### 1.2 TOML Spec Preview (Acme Inc. Ingestion Pipeline)

```toml
name = "acme-inc-327-ingestion"
version = 1
output_dir = "./output/acme-inc-327-ingestion"

[secrets]
provider = "gcp_secret_manager"
project = "${GCP_PROJECT}"

[sources.sftp-files]
kind = "sftp"
host = "byn-prod-merchant-327-wlgrns-sftp.example.com"
port = 22
username = "acme-inc-sftp"
credentials = { type = "secret", name = "sftp-key-327-acme-inc" }
remote_path = "/"
pattern = "wag_banyan_*_file*.csv*"
stream = "raw-files"

[stages.upload-to-gcs]
adapter = "gcs_sink"
input_streams = ["raw-files"]
resources = { reads = ["raw-files"] }
[stages.upload-to-gcs.params]
bucket = "byn-prod-merchant-327-acme-inc"
prefix = "input/"
format = "raw"

[triggers]
on_success = ["acme-inc-327-transformation"]
```

### 1.3 TOML Spec Preview (Acme Inc. Transformation Pipeline, abbreviated)

```toml
name = "acme-inc-327-transformation"
version = 1
output_dir = "./output/acme-inc-327-transformation"

[secrets]
provider = "gcp_secret_manager"
project = "${GCP_PROJECT}"

[sources.encrypted-files]
kind = "gcs"
bucket = "byn-prod-merchant-327-acme-inc"
prefix = "input/"
pattern = "wag_banyan_*_file*.csv"
stream = "encrypted"

[stages.decrypt]
adapter = "pgp_decrypt"
input_streams = ["encrypted"]
output_stream = "decrypted"
resources = { reads = ["encrypted"], creates = ["decrypted"] }
[stages.decrypt.params]
private_key = { type = "secret", name = "private-pgp-327-acme-inc" }
passphrase = { type = "secret", name = "private-pgp-pass-327-acme-inc" }

[stages.dedup]
adapter = "deduplicate"
input_streams = ["decrypted"]
output_stream = "deduped"
resources = { reads = ["decrypted"], creates = ["deduped"] }
[stages.dedup.params]
key_pattern = "wag_banyan_{type}_file_{date}{time}.csv"
group_by = ["type", "date"]
hash_algorithm = "md5"
keep = "newest"

# ... (per-stream parse/map/join/aggregate/assemble stages from Phase 2)

[stages.kafka-receipts]
adapter = "kafka_sink"
input_streams = ["validated-receipts"]
resources = { reads = ["validated-receipts"] }
[stages.kafka-receipts.params]
topic = "by_production_327_acme_inc_receipt-preprocess_canonical_batch_avro_0"
filter = "valid_only"

[stages.kafka-taxonomy]
adapter = "kafka_sink"
input_streams = ["taxonomy"]
resources = { reads = ["taxonomy"] }
[stages.kafka-taxonomy.params]
topic = "by_production_acme_inc_taxonomy_avro_0"

[lifecycle]
bucket = "byn-prod-merchant-327-acme-inc"
staging_prefix = "staging/"
historical_prefix = "historical/"
error_prefix = "error/"
on_success = "move_to_historical"
on_failure = "move_to_input"

[schedule]
cron = "30 21 * * *"  # 9:30 PM UTC daily
```

### 1.4 Key Design Decisions

**D1: Secrets are resolved lazily at stage construction, not at parse time.** The `PipelineSpec` stores `CredentialRef` variants including a new `Secret { name }` variant. The registry resolves secrets when building adapters/stages. This keeps specs serializable in checkpoints.

**D2: Pipeline chaining is process-level, not in-process.** The runner, on successful completion, spawns child pipeline processes (or invokes `ecl pipeline run <child.toml>`). This keeps the memory model simple — each pipeline is independent, with its own state store and checkpoint.

**D3: Scheduling is a thin wrapper, not a daemon.** We provide a `ecl pipeline schedule <config.toml>` command that runs the pipeline on a cron schedule. Internally it uses the `cron` crate to compute next run times and `tokio::time::sleep_until`. For production, users can also use system cron / systemd timers.

**D4: Deduplication operates on file-level metadata, not record-level.** Acme Inc. dedup groups files by type+date, compares MD5 hashes within groups, and keeps the newest. This is a batch stage operating on `PipelineItem`s (files), not `Record`s (rows).

**D5: PGP decryption is a per-item stage.** Each encrypted file is decrypted independently. The `sequoia-openpgp` crate provides the implementation.

---


Historical milestone moved to [2. Milestone 9.1: Secret Management](slice01-secret-management/slice-plan.md).

Historical milestone moved to [3. Milestone 9.2: SFTP Source Adapter](slice02-sftp-source-adapter/slice-plan.md).

Historical milestone moved to [4. Milestone 9.3: PGP Decryption Stage](slice03-pgp-decryption-stage/slice-plan.md).

Historical milestone moved to [5. Milestone 9.4: Deduplication Stage](slice04-deduplication-stage/slice-plan.md).

Historical milestone moved to [6. Milestone 9.5: S3 Sink Stage](slice05-s3-sink-stage/slice-plan.md).

Historical milestone moved to [7. Milestone 9.6: Pipeline Chaining](slice06-pipeline-chaining/slice-plan.md).

Historical milestone moved to [8. Milestone 9.7: Scheduling & Cron Triggers](slice07-scheduling-cron-triggers/slice-plan.md).

Historical milestone moved to [9. Milestone 9.8: Acme Inc. End-to-End Integration Test](slice08-acme-inc-end-to-end-integration-test/slice-plan.md).

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 1438-1656. Original bytes are recoverable from import commit 2d006a7. -->

## 10. Cross-Cutting Concerns

### 10.1 New Crates Summary

| Crate | Purpose | Key Dependencies |
|-------|---------|-----------------|
| `ecl-secrets` | Secret resolution (env, file, GCP SM) | google-cloud-secretmanager (optional) |
| `ecl-adapter-sftp` | SFTP source adapter | russh, russh-sftp |
| `ecl-sink-s3` | AWS S3 file writer | aws-sdk-s3 |

### 10.2 Modified Crates

| Crate | Changes |
|-------|---------|
| `ecl-pipeline-spec` | `CredentialRef::Secret`, `SecretsConfig`, `TriggersSpec`, `ScheduleSpec` |
| `ecl-stages` | `PgpDecryptStage`, `DeduplicateStage` + deps (pgp, md5) |
| `ecl-pipeline` | Pipeline chaining in runner |
| `ecl-cli` | Schedule command, SFTP adapter registration, S3 sink registration |

### 10.3 Workspace Dependencies

Add to root `Cargo.toml`:

```toml
pgp = "0.14"
md5 = "0.7"
russh = "0.46"
russh-sftp = "2"
russh-keys = "0.46"
aws-sdk-s3 = "1"
aws-config = { version = "1", features = ["behavior-version-latest"] }
google-cloud-secretmanager = "0.5"
cron = "0.15"
```

### 10.4 Secret Resolver Threading Through Registry

The registry functions gain a `secret_resolver` parameter:

```rust
// resolve_adapters gains resolver param:
pub async fn resolve_adapters(
    spec: &PipelineSpec,
    secret_resolver: &dyn SecretResolver,
) -> Result<BTreeMap<String, Arc<dyn SourceAdapter>>, ResolveError> {
    // ... existing logic, pass resolver to adapters that need it ...
    SourceSpec::Sftp(sftp_spec) => {
        let adapter = SftpAdapter::from_spec(sftp_spec, secret_resolver).await?;
        // ...
    }
}

// stage_lookup_fn gains resolver:
pub fn stage_lookup_fn(
    adapters: &BTreeMap<String, Arc<dyn SourceAdapter>>,
    secret_resolver: Arc<dyn SecretResolver>,
) -> impl Fn(&str, &StageSpec) -> Result<Arc<dyn Stage>, ResolveError> {
    // ... existing logic ...
    "pgp_decrypt" => {
        let stage = PgpDecryptStage::from_params(&spec.params, &*secret_resolver).await?;
        // Note: this closure is sync but from_params is async.
        // Solution: make stage_lookup_fn async, or resolve secrets eagerly.
    }
}
```

**The async-in-closure problem:** `stage_lookup_fn` returns a sync closure, but `PgpDecryptStage::from_params` is async (it resolves secrets). Two solutions:

1. **Eager resolution:** Resolve all secrets upfront before building stages. Pass pre-resolved bytes into stage constructors.
2. **Async stage_lookup_fn:** Change the closure to return a `Future`. This cascades into the resolve function.

**Recommended: Option 1 (eager resolution).** Build a `ResolvedSecrets` map during registry setup:

```rust
pub async fn resolve_stage_secrets(
    spec: &PipelineSpec,
    resolver: &dyn SecretResolver,
) -> Result<BTreeMap<String, Vec<u8>>, ResolveError> {
    // Walk all stages, find CredentialRef::Secret references in params,
    // resolve them, return a map of secret_name → bytes.
}
```

Then pass `ResolvedSecrets` to stage constructors as sync lookups.

### 10.5 Backward Compatibility

All changes are additive:

- `CredentialRef::Secret` — new variant, existing variants unchanged
- `PipelineSpec` gains `secrets`, `triggers`, `schedule` — all `Option` with `#[serde(default)]`
- `resolve_adapters` gains optional `secret_resolver` parameter — default `EnvResolver`
- New stages registered in registry — no impact on existing

### 10.6 Feature Flags

The `gcp` feature flag on `ecl-secrets` controls whether GCP Secret Manager is compiled in. This keeps the dependency tree lean for users who don't need GCP:

```toml
# In ecl-cli/Cargo.toml:
ecl-secrets = { path = "../ecl-secrets", features = ["gcp"] }
```

For local development/testing, `ecl-secrets` with just env/file resolvers has zero cloud dependencies.

---

## 11. Verification Checklist

### Per-Milestone

- [ ] `make test` passes
- [ ] `make lint` passes
- [ ] `make format` passes
- [ ] No compiler warnings
- [ ] Coverage ≥ 95% on new code
- [ ] Checked against AP-01 through AP-20

### Phase 3 Complete

- [ ] Secret resolver resolves from env, file, and GCP Secret Manager
- [ ] SFTP adapter lists and fetches files from SFTP server
- [ ] PGP decryption stage decrypts encrypted files
- [ ] Deduplication stage removes duplicate files by content hash
- [ ] S3 sink writes files to AWS S3
- [ ] Pipeline chaining triggers child pipelines on completion
- [ ] Scheduler runs pipelines on cron schedule
- [ ] Acme Inc. E2E integration test passes (decrypt → dedup → transform → output)
- [ ] Phase 1 Affinity E2E still passes
- [ ] Phase 2 Giant Eagle E2E still passes
- [ ] All pre-Phase-1 tests still pass

---

## Appendix A: Milestone Dependency Graph

```
9.1 Secret Management ─────────────────────────────┐
    │                                                │
    ├── 9.2 SFTP Source ─────────────────────┐      │
    │                                         │      │
    ├── 9.3 PGP Decryption ─────────────────┤      │
    │                                         │      │
    ├── 9.4 Deduplication ──────────────────┤      │
    │                                         │      │
    ├── 9.5 S3 Sink ────────────────────────┤      │
    │                                         │      │
    │   9.6 Pipeline Chaining ──────────────┤      │
    │                                         │      │
    │   9.7 Scheduling ─────────────────────┤      │
    │                                         │      │
    └─────────────────────────────────────────┘      │
                              │                       │
                       9.8 Acme Inc. E2E ─────────────┘
```

**Critical path:** 9.1 → 9.3 (PGP needs secrets) → 9.8 (E2E needs everything)

**Parallelizable after 9.1:** 9.2, 9.3, 9.4, 9.5, 9.6, 9.7 can all proceed in parallel.

## Appendix B: Files Changed/Created Summary

### New Files

| File | Milestone |
|------|-----------|
| `crates/ecl-secrets/Cargo.toml` | 9.1 |
| `crates/ecl-secrets/src/lib.rs` | 9.1 |
| `crates/ecl-secrets/src/env.rs` | 9.1 |
| `crates/ecl-secrets/src/file.rs` | 9.1 |
| `crates/ecl-secrets/src/gcp.rs` | 9.1 |
| `crates/ecl-adapter-sftp/Cargo.toml` | 9.2 |
| `crates/ecl-adapter-sftp/src/lib.rs` | 9.2 |
| `crates/ecl-stages/src/pgp_decrypt.rs` | 9.3 |
| `crates/ecl-stages/src/deduplicate.rs` | 9.4 |
| `crates/ecl-sink-s3/Cargo.toml` | 9.5 |
| `crates/ecl-sink-s3/src/lib.rs` | 9.5 |
| `crates/ecl-cli/src/pipeline/schedule.rs` | 9.7 |
| `tests/fixtures/acme-inc/*.csv.pgp` | 9.8 |
| `tests/fixtures/acme_inc_*.toml` | 9.8 |
| `tests/integration/acme_inc_e2e.rs` | 9.8 |

### Modified Files

| File | Milestone | Change |
|------|-----------|--------|
| `Cargo.toml` (workspace) | All | Add crate members + deps |
| `crates/ecl-pipeline-spec/src/source.rs` | 9.1, 9.2 | `CredentialRef::Secret`, `SftpSourceSpec` |
| `crates/ecl-pipeline-spec/src/lib.rs` | 9.1, 9.6, 9.7 | `secrets`, `triggers`, `schedule` fields |
| `crates/ecl-stages/src/lib.rs` | 9.3, 9.4 | Module declarations |
| `crates/ecl-stages/Cargo.toml` | 9.3, 9.4 | pgp, md5 deps |
| `crates/ecl-pipeline/src/runner.rs` | 9.6 | Pipeline chaining |
| `crates/ecl-cli/Cargo.toml` | 9.1, 9.2, 9.5, 9.7 | New crate deps |
| `crates/ecl-cli/src/pipeline/registry.rs` | 9.1-9.5 | Secret resolver threading, new adapters/stages |
| `crates/ecl-cli/src/pipeline/mod.rs` | 9.7 | Schedule command |

## Appendix C: Acme Inc.-Specific Transformation Stages

These are **not new stage types** — they use existing stages from Phase 1 and 2 with Acme Inc.-specific configurations:

| Transformation | Stage Type | Configuration |
|---------------|-----------|---------------|
| Price correction (digital /100) | `field_map` | Arithmetic expression on amount fields |
| Order type mapping | `field_map` + `lookup` | Map fulfillment_type values |
| BOPIS detection | `field_map` | Conditional: if bopisFlag='Y' then set fields |
| Payment type mapping | `lookup` | Credit Card→CREDIT, Debit Card→DEBIT, etc. |
| Payment scheme mapping | `lookup` | Visa→VI, Mastercard→MC, etc. |
| UPC check digit | `field_map` (regex + expression) | GS1 check digit algorithm |
| Auth code cleaning | `field_map` (pad) | Strip leading zeros, pad to 6 |
| Timezone resolution | `timezone` | ZIP code → IANA → UTC |
| Store 5995 override | `timezone` | override: { "5995": "UTC" } |
| Tender aggregation | `aggregate` | Group by receipt_id, sum/max amounts |
| Cash back calculation | `aggregate` | Sum negative amounts from Debit/Cashback |
| Item-Product join | `join` | Left join on UPC |
| Hot key UPC handling | Handled by join's left join | Unmatched UPCs pass through |
| Receipt assembly | `assemble` | Merge 5 streams into nested canonical |
| Taxonomy extraction | `field_map` | Extract product fields into taxonomy stream |

The key insight: **no Acme Inc.-specific Rust code is needed.** All transformations are expressed as stage configurations in TOML. The only new infrastructure needed is SFTP, PGP decryption, deduplication, secrets, and pipeline chaining — which are all generic, reusable capabilities.

## Reconstructed slice locations

- [2. Milestone 9.1: Secret Management](slice01-secret-management/slice-plan.md)
- [3. Milestone 9.2: SFTP Source Adapter](slice02-sftp-source-adapter/slice-plan.md)
- [4. Milestone 9.3: PGP Decryption Stage](slice03-pgp-decryption-stage/slice-plan.md)
- [5. Milestone 9.4: Deduplication Stage](slice04-deduplication-stage/slice-plan.md)
- [6. Milestone 9.5: S3 Sink Stage](slice05-s3-sink-stage/slice-plan.md)
- [7. Milestone 9.6: Pipeline Chaining](slice06-pipeline-chaining/slice-plan.md)
- [8. Milestone 9.7: Scheduling & Cron Triggers](slice07-scheduling-cron-triggers/slice-plan.md)
- [9. Milestone 9.8: Acme Inc. End-to-End Integration Test](slice08-acme-inc-end-to-end-integration-test/slice-plan.md)

Original criteria are also available in the [historical ledger](ledger.md).
