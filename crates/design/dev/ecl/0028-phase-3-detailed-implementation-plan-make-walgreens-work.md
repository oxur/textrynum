# Phase 3 Detailed Implementation Plan: "Make Walgreens Work"

**Date:** 2026-03-17
**Status:** Draft
**Depends on:** 0028 (Phase 1), 0029 (Phase 2)
**Goal:** Run the Walgreens (Partner 327) receipt pipeline end-to-end in ECL — SFTP ingestion, PGP decryption, deduplication, multi-file CSV transformation with joins/aggregations, Kafka output, file lifecycle, scheduled execution.

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
9. [Milestone 9.8: Walgreens End-to-End Integration Test](#9-milestone-98-walgreens-end-to-end-integration-test)
10. [Cross-Cutting Concerns](#10-cross-cutting-concerns)
11. [Verification Checklist](#11-verification-checklist)

---

## 1. Architecture Overview

### 1.1 Walgreens Has Two Pipelines

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

### 1.2 TOML Spec Preview (Walgreens Ingestion Pipeline)

```toml
name = "walgreens-327-ingestion"
version = 1
output_dir = "./output/walgreens-327-ingestion"

[secrets]
provider = "gcp_secret_manager"
project = "${GCP_PROJECT}"

[sources.sftp-files]
kind = "sftp"
host = "byn-prod-merchant-327-wlgrns-sftp.example.com"
port = 22
username = "walgreens-sftp"
credentials = { type = "secret", name = "sftp-key-327-walgreens" }
remote_path = "/"
pattern = "wag_banyan_*_file*.csv*"
stream = "raw-files"

[stages.upload-to-gcs]
adapter = "gcs_sink"
input_streams = ["raw-files"]
resources = { reads = ["raw-files"] }
[stages.upload-to-gcs.params]
bucket = "byn-prod-merchant-327-walgreens"
prefix = "input/"
format = "raw"

[triggers]
on_success = ["walgreens-327-transformation"]
```

### 1.3 TOML Spec Preview (Walgreens Transformation Pipeline, abbreviated)

```toml
name = "walgreens-327-transformation"
version = 1
output_dir = "./output/walgreens-327-transformation"

[secrets]
provider = "gcp_secret_manager"
project = "${GCP_PROJECT}"

[sources.encrypted-files]
kind = "gcs"
bucket = "byn-prod-merchant-327-walgreens"
prefix = "input/"
pattern = "wag_banyan_*_file*.csv"
stream = "encrypted"

[stages.decrypt]
adapter = "pgp_decrypt"
input_streams = ["encrypted"]
output_stream = "decrypted"
resources = { reads = ["encrypted"], creates = ["decrypted"] }
[stages.decrypt.params]
private_key = { type = "secret", name = "private-pgp-327-walgreens" }
passphrase = { type = "secret", name = "private-pgp-pass-327-walgreens" }

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
topic = "by_production_327_walgreens_receipt-preprocess_canonical_batch_avro_0"
filter = "valid_only"

[stages.kafka-taxonomy]
adapter = "kafka_sink"
input_streams = ["taxonomy"]
resources = { reads = ["taxonomy"] }
[stages.kafka-taxonomy.params]
topic = "by_production_walgreens_taxonomy_avro_0"

[lifecycle]
bucket = "byn-prod-merchant-327-walgreens"
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

**D4: Deduplication operates on file-level metadata, not record-level.** Walgreens dedup groups files by type+date, compares MD5 hashes within groups, and keeps the newest. This is a batch stage operating on `PipelineItem`s (files), not `Record`s (rows).

**D5: PGP decryption is a per-item stage.** Each encrypted file is decrypted independently. The `sequoia-openpgp` crate provides the implementation.

---

## 2. Milestone 9.1: Secret Management

### 2.1 Scope

New crate `ecl-secrets` providing pluggable secret resolution. Extend `CredentialRef` with a `Secret` variant. Integrate into registry.

### 2.2 Crate Structure

```
crates/ecl-secrets/
├── Cargo.toml
└── src/
    ├── lib.rs          # SecretResolver trait + factory
    ├── env.rs          # Environment variable resolver
    ├── file.rs         # File-based resolver
    └── gcp.rs          # GCP Secret Manager resolver
```

### 2.3 Cargo.toml

```toml
[package]
name = "ecl-secrets"
version.workspace = true
edition.workspace = true

[dependencies]
# GCP Secret Manager
google-cloud-secretmanager = { version = "0.5", optional = true }
google-cloud-auth = { version = "0.19", features = ["default-tls"], optional = true }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Error handling
thiserror = { workspace = true }
tracing = { workspace = true }

# Serde (for config)
serde = { workspace = true }
serde_json = { workspace = true }

[features]
default = ["gcp"]
gcp = ["dep:google-cloud-secretmanager", "dep:google-cloud-auth"]

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### 2.4 SecretResolver Trait

#### File: `crates/ecl-secrets/src/lib.rs`

```rust
pub mod env;
pub mod file;
#[cfg(feature = "gcp")]
pub mod gcp;

use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SecretError {
    #[error("secret not found: {name}")]
    NotFound { name: String },
    #[error("access denied for secret: {name}")]
    AccessDenied { name: String },
    #[error("secret provider error: {message}")]
    Provider { message: String },
}

/// Resolves secret values by name.
#[async_trait]
pub trait SecretResolver: Send + Sync + std::fmt::Debug {
    /// Resolve a secret by name, returning its value as bytes.
    /// Secrets may be strings (UTF-8) or binary (PGP keys).
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError>;

    /// Resolve a secret as a UTF-8 string.
    async fn resolve_string(&self, name: &str) -> Result<String, SecretError> {
        let bytes = self.resolve(name).await?;
        String::from_utf8(bytes).map_err(|_| SecretError::Provider {
            message: format!("secret '{name}' is not valid UTF-8"),
        })
    }
}

/// Configuration for secret resolution.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "provider")]
pub enum SecretsConfig {
    /// No secret management — only env vars and files.
    #[serde(rename = "none")]
    None,
    /// GCP Secret Manager.
    #[serde(rename = "gcp_secret_manager")]
    GcpSecretManager {
        project: String,
    },
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self::None
    }
}

/// Build a SecretResolver from configuration.
pub async fn build_resolver(config: &SecretsConfig) -> Result<Box<dyn SecretResolver>, SecretError> {
    match config {
        SecretsConfig::None => Ok(Box::new(env::EnvResolver)),
        #[cfg(feature = "gcp")]
        SecretsConfig::GcpSecretManager { project } => {
            let resolver = gcp::GcpSecretResolver::new(project).await?;
            Ok(Box::new(resolver))
        }
        #[cfg(not(feature = "gcp"))]
        SecretsConfig::GcpSecretManager { .. } => {
            Err(SecretError::Provider {
                message: "GCP secret manager support not compiled (enable 'gcp' feature)".to_string(),
            })
        }
    }
}
```

### 2.5 Implementations

#### `env.rs` — Environment Variable Resolver

```rust
#[derive(Debug)]
pub struct EnvResolver;

#[async_trait]
impl SecretResolver for EnvResolver {
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError> {
        std::env::var(name)
            .map(|v| v.into_bytes())
            .map_err(|_| SecretError::NotFound { name: name.to_string() })
    }
}
```

#### `file.rs` — File-Based Resolver

```rust
#[derive(Debug)]
pub struct FileResolver;

#[async_trait]
impl SecretResolver for FileResolver {
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError> {
        // Interpret `name` as a file path.
        tokio::fs::read(name).await
            .map_err(|e| SecretError::NotFound { name: format!("{name}: {e}") })
    }
}
```

#### `gcp.rs` — GCP Secret Manager

```rust
#[derive(Debug)]
pub struct GcpSecretResolver {
    client: google_cloud_secretmanager::client::Client,
    project: String,
}

impl GcpSecretResolver {
    pub async fn new(project: &str) -> Result<Self, SecretError> {
        let client = google_cloud_secretmanager::client::Client::default().await
            .map_err(|e| SecretError::Provider { message: e.to_string() })?;
        Ok(Self { client, project: project.to_string() })
    }
}

#[async_trait]
impl SecretResolver for GcpSecretResolver {
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError> {
        let secret_name = format!("projects/{}/secrets/{}/versions/latest", self.project, name);
        let response = self.client
            .access_secret_version(&secret_name, None).await
            .map_err(|e| match e {
                // Map specific errors
                _ => SecretError::Provider { message: e.to_string() },
            })?;
        Ok(response.payload.unwrap_or_default().data)
    }
}
```

### 2.6 CredentialRef Extension

#### File: `crates/ecl-pipeline-spec/src/source.rs`

Add new variant to `CredentialRef`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    #[serde(rename = "file")]
    File { path: PathBuf },
    #[serde(rename = "env")]
    EnvVar { env: String },
    #[serde(rename = "application_default")]
    ApplicationDefault,
    /// Secret from a secret management provider (GCP Secret Manager, etc.).
    /// Resolved at runtime by the SecretResolver.
    #[serde(rename = "secret")]
    Secret { name: String },
}
```

### 2.7 PipelineSpec Extension

#### File: `crates/ecl-pipeline-spec/src/lib.rs`

```rust
pub struct PipelineSpec {
    // ... existing fields ...
    /// Secret management configuration.
    #[serde(default)]
    pub secrets: SecretsConfig,
}
```

Import `SecretsConfig` from `ecl-secrets` (or define a spec-level type that `ecl-secrets` converts from, to avoid the dependency direction issue). **Recommended:** Define `SecretsConfig` in `ecl-pipeline-spec` since it's a serializable config type, and `ecl-secrets` depends on `ecl-pipeline-spec` to read it.

### 2.8 Registry Integration

The registry builds a `SecretResolver` at startup and passes it to adapters/stages that need secrets:

```rust
// In run.rs:
let secret_resolver = ecl_secrets::build_resolver(&spec.secrets).await?;

// Pass to registry functions:
let adapters = registry::resolve_adapters(&spec, &*secret_resolver).await?;
```

Adapters and stages that accept `CredentialRef::Secret` call `secret_resolver.resolve(name)` during construction.

### 2.9 Tests

1. `test_env_resolver_found` — set env var, resolve
2. `test_env_resolver_not_found` — missing var → NotFound
3. `test_file_resolver_found` — temp file with secret
4. `test_file_resolver_not_found` — missing file → NotFound
5. `test_credential_ref_secret_serde` — roundtrip serialize
6. `test_secrets_config_serde_none`
7. `test_secrets_config_serde_gcp`
8. `test_build_resolver_none` — returns EnvResolver
9. `test_resolve_string_valid_utf8`
10. `test_resolve_string_invalid_utf8` — binary secret → error

---

## 3. Milestone 9.2: SFTP Source Adapter

### 3.1 Scope

New crate `ecl-adapter-sftp` implementing `SourceAdapter` for SFTP file servers.

### 3.2 Crate Structure

```
crates/ecl-adapter-sftp/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 3.3 Cargo.toml

```toml
[package]
name = "ecl-adapter-sftp"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
ecl-secrets = { path = "../ecl-secrets" }

# SSH/SFTP
russh = "0.46"
russh-sftp = "2"
russh-keys = "0.46"

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Hashing
blake3 = { workspace = true }

# Pattern matching
glob = { workspace = true }

# Serde
serde = { workspace = true }
serde_json = { workspace = true }

# Logging & errors
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile = { workspace = true }
```

### 3.4 SourceSpec Extension

```rust
// In ecl-pipeline-spec/src/source.rs:
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    // ... existing ...
    #[serde(rename = "sftp")]
    Sftp(SftpSourceSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SftpSourceSpec {
    /// SFTP server hostname.
    pub host: String,
    /// SFTP server port. Default: 22.
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    /// SSH username.
    pub username: String,
    /// Credentials for authentication (private key or password).
    pub credentials: CredentialRef,
    /// Remote directory to list.
    #[serde(default = "default_remote_root")]
    pub remote_path: String,
    /// Glob pattern to match filenames.
    #[serde(default)]
    pub pattern: Option<String>,
    /// Named data stream.
    #[serde(default)]
    pub stream: Option<String>,
}

fn default_ssh_port() -> u16 { 22 }
fn default_remote_root() -> String { "/".to_string() }
```

### 3.5 Adapter Implementation

```rust
#[derive(Debug)]
pub struct SftpAdapter {
    host: String,
    port: u16,
    username: String,
    /// Pre-resolved private key bytes or password.
    auth_material: Vec<u8>,
    auth_type: AuthType,
    remote_path: String,
    pattern: Option<glob::Pattern>,
    stream: Option<String>,
}

#[derive(Debug)]
enum AuthType {
    PrivateKey,
    Password,
}

impl SftpAdapter {
    pub async fn from_spec(
        spec: &SftpSourceSpec,
        secret_resolver: &dyn SecretResolver,
    ) -> Result<Self, SourceError> {
        // Resolve credentials via secret_resolver
        let auth_material = match &spec.credentials {
            CredentialRef::Secret { name } => {
                secret_resolver.resolve(name).await
                    .map_err(|e| SourceError::AuthError {
                        source_name: "sftp".to_string(),
                        message: e.to_string(),
                    })?
            }
            CredentialRef::File { path } => {
                tokio::fs::read(path).await
                    .map_err(|e| SourceError::AuthError {
                        source_name: "sftp".to_string(),
                        message: e.to_string(),
                    })?
            }
            CredentialRef::EnvVar { env } => {
                std::env::var(env)
                    .map(|v| v.into_bytes())
                    .map_err(|_| SourceError::AuthError {
                        source_name: "sftp".to_string(),
                        message: format!("env var '{env}' not set"),
                    })?
            }
            _ => return Err(SourceError::AuthError {
                source_name: "sftp".to_string(),
                message: "unsupported credential type for SFTP".to_string(),
            }),
        };

        let pattern = spec.pattern.as_ref()
            .map(|p| glob::Pattern::new(p))
            .transpose()
            .map_err(|e| SourceError::Permanent {
                source_name: "sftp".to_string(),
                message: format!("invalid glob pattern: {e}"),
            })?;

        Ok(Self {
            host: spec.host.clone(),
            port: spec.port,
            username: spec.username.clone(),
            auth_material,
            auth_type: AuthType::PrivateKey, // detect from content
            remote_path: spec.remote_path.clone(),
            pattern,
            stream: spec.stream.clone(),
        })
    }

    /// Open an SSH session and SFTP channel.
    async fn connect(&self) -> Result<SftpSession, SourceError> {
        // 1. Parse private key from auth_material
        // 2. Connect to host:port via russh
        // 3. Authenticate with key or password
        // 4. Open SFTP subsystem
        // 5. Return session wrapper
    }
}

#[async_trait]
impl SourceAdapter for SftpAdapter {
    fn source_kind(&self) -> &str { "sftp" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        let session = self.connect().await?;
        let entries = session.read_dir(&self.remote_path).await
            .map_err(|e| SourceError::Transient {
                source_name: "sftp".to_string(),
                message: e.to_string(),
            })?;

        let mut items = Vec::new();
        for entry in entries {
            if entry.is_dir() { continue; }
            let name = entry.file_name();
            if let Some(ref pattern) = self.pattern {
                if !pattern.matches(&name) { continue; }
            }
            items.push(SourceItem {
                id: format!("sftp:{}/{}", self.remote_path, name),
                display_name: name.clone(),
                mime_type: mime_from_extension(&name),
                path: format!("{}/{}", self.remote_path, name),
                modified_at: entry.modified_at(),
                source_hash: None,
            });
        }

        items.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(items)
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let session = self.connect().await?;
        let path = &item.path;
        let content = session.read_file(path).await
            .map_err(|e| SourceError::Transient {
                source_name: "sftp".to_string(),
                message: e.to_string(),
            })?;

        let content_hash = Blake3Hash::new(&hex::encode(blake3::hash(&content).as_bytes()));

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: item.mime_type.clone(),
            provenance: ItemProvenance {
                source_kind: "sftp".to_string(),
                metadata: BTreeMap::from([
                    ("host".to_string(), json!(self.host)),
                    ("path".to_string(), json!(path)),
                ]),
                source_modified: item.modified_at,
                extracted_at: Utc::now(),
            },
            content_hash,
        })
    }
}
```

**Connection management note:** SSH connections are expensive. For enumerate + N fetches, we should ideally reuse one connection. Two approaches:

1. **Simple (Phase 3):** Reconnect per operation. Reliable, no state management.
2. **Optimized (future):** Cache connection in `Arc<Mutex<Option<Session>>>`.

Go with approach 1 for Phase 3. Reconnect per call.

### 3.6 Tests

1. `test_sftp_spec_serde_roundtrip`
2. `test_sftp_spec_defaults` — port 22, remote_path "/"
3. `test_sftp_adapter_source_kind`
4. `test_sftp_adapter_from_spec_with_env_credential`
5. `test_sftp_adapter_from_spec_with_secret_credential` (mock resolver)
6. `test_sftp_pattern_matching` — glob filter logic

**Integration tests** (require SFTP server — use `testcontainers` with `atmoz/sftp` Docker image):
7. `test_sftp_enumerate_lists_files`
8. `test_sftp_fetch_downloads_content`
9. `test_sftp_enumerate_with_pattern`

---

## 4. Milestone 9.3: PGP Decryption Stage

### 4.1 Scope

Per-item stage that decrypts PGP-encrypted content using a private key and passphrase.

### 4.2 Implementation

#### File: `crates/ecl-stages/src/pgp_decrypt.rs` (new)

**Dependencies to add to `ecl-stages/Cargo.toml`:**

```toml
pgp = "0.14"   # Pure Rust PGP implementation (lighter than sequoia)
```

**Note on crate choice:** `pgp` (rpgp) is a pure-Rust implementation — no C dependencies, simpler build. `sequoia-openpgp` is more complete but heavier. For decryption-only, `pgp` is sufficient.

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct PgpDecryptConfig {
    /// Private key credential reference.
    pub private_key: CredentialRef,
    /// Passphrase credential reference.
    pub passphrase: CredentialRef,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct PgpDecryptStage {
    /// Pre-loaded and parsed private key.
    secret_key: pgp::SignedSecretKey,
}

impl PgpDecryptStage {
    pub async fn from_params(
        params: &serde_json::Value,
        secret_resolver: &dyn SecretResolver,
    ) -> Result<Self, StageError> {
        let config: PgpDecryptConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // Resolve private key.
        let key_bytes = resolve_credential(&config.private_key, secret_resolver).await
            .map_err(|e| StageError::Permanent {
                stage: "pgp_decrypt".to_string(),
                item_id: String::new(),
                message: format!("failed to resolve PGP private key: {e}"),
            })?;

        // Resolve passphrase.
        let passphrase = resolve_credential_string(&config.passphrase, secret_resolver).await
            .map_err(|e| StageError::Permanent {
                stage: "pgp_decrypt".to_string(),
                item_id: String::new(),
                message: format!("failed to resolve PGP passphrase: {e}"),
            })?;

        // Parse the armored private key.
        let (secret_key, _) = pgp::SignedSecretKey::from_armor_single(
            std::io::Cursor::new(&key_bytes)
        ).map_err(|e| StageError::Permanent {
            stage: "pgp_decrypt".to_string(),
            item_id: String::new(),
            message: format!("failed to parse PGP key: {e}"),
        })?;

        // Verify passphrase works.
        secret_key.unlock(|| passphrase.clone(), |_| Ok(()))
            .map_err(|e| StageError::Permanent {
                stage: "pgp_decrypt".to_string(),
                item_id: String::new(),
                message: format!("PGP key passphrase incorrect: {e}"),
            })?;

        Ok(Self { secret_key })
    }
}

#[async_trait]
impl Stage for PgpDecryptStage {
    fn name(&self) -> &str { "pgp_decrypt" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let encrypted_data = item.content.as_ref();

        // Parse the PGP message.
        let (message, _) = pgp::Message::from_armor_single(
            std::io::Cursor::new(encrypted_data)
        ).or_else(|_| {
            // Try binary format if armor fails.
            pgp::Message::from_bytes(encrypted_data)
                .map(|m| (m, None))
        }).map_err(|e| StageError::Permanent {
            stage: "pgp_decrypt".to_string(),
            item_id: item.id.clone(),
            message: format!("invalid PGP message: {e}"),
        })?;

        // Decrypt.
        let decrypted = message
            .decrypt(|| String::new(), &[&self.secret_key])
            .map_err(|e| StageError::Permanent {
                stage: "pgp_decrypt".to_string(),
                item_id: item.id.clone(),
                message: format!("PGP decryption failed: {e}"),
            })?;

        // Get the decrypted content bytes.
        let content = decrypted.get_content()
            .map_err(|e| StageError::Permanent { /* ... */ })?
            .unwrap_or_default();

        Ok(vec![PipelineItem {
            content: Arc::from(content.as_slice()),
            mime_type: infer_decrypted_mime(&item.display_name),
            ..item
        }])
    }
}

/// Helper: resolve a CredentialRef to bytes using the secret resolver.
async fn resolve_credential(
    cred: &CredentialRef,
    resolver: &dyn SecretResolver,
) -> Result<Vec<u8>, String> {
    match cred {
        CredentialRef::Secret { name } => resolver.resolve(name).await
            .map_err(|e| e.to_string()),
        CredentialRef::File { path } => tokio::fs::read(path).await
            .map_err(|e| e.to_string()),
        CredentialRef::EnvVar { env } => std::env::var(env)
            .map(|v| v.into_bytes())
            .map_err(|e| e.to_string()),
        CredentialRef::ApplicationDefault => Err("application_default not supported for PGP".to_string()),
    }
}
```

**Note:** PGP decryption is CPU-bound. For large files or many files, consider wrapping in `spawn_blocking`. For typical Walgreens files (few MB each, ~10 files), inline is fine.

### 4.3 Tests

1. `test_pgp_decrypt_roundtrip` — encrypt then decrypt a test message
2. `test_pgp_decrypt_armored_format`
3. `test_pgp_decrypt_binary_format`
4. `test_pgp_decrypt_wrong_key` — returns Permanent error
5. `test_pgp_decrypt_wrong_passphrase` — returns Permanent error
6. `test_pgp_decrypt_corrupt_message` — returns Permanent error
7. `test_pgp_decrypt_preserves_item_metadata`
8. `test_pgp_decrypt_from_params_missing_key` — config error

**Test key generation:** Generate a test PGP keypair at test time using `pgp::SecretKeyParamsBuilder`. Embed the public key for encryption and private key for decryption.

---

## 5. Milestone 9.4: Deduplication Stage

### 5.1 Scope

Batch stage that groups files by pattern-derived keys, compares content hashes, and keeps only the newest unique file per group.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/deduplicate.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DeduplicateConfig {
    /// Regex pattern with named groups to extract grouping key from filename.
    /// Example: "wag_banyan_(?P<type>\\w+)_file_(?P<date>\\d{8})(?P<time>\\d{6}).csv"
    pub filename_pattern: String,
    /// Named groups to use as the dedup key.
    /// Example: ["type", "date"]
    pub group_by: Vec<String>,
    /// Hash algorithm: "md5", "blake3". Default: "md5"
    #[serde(default = "default_md5")]
    pub hash_algorithm: String,
    /// Which file to keep when duplicates found: "newest", "oldest". Default: "newest"
    #[serde(default = "default_newest")]
    pub keep: String,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct DeduplicateStage {
    config: DeduplicateConfig,
    pattern: regex::Regex,
}

impl DeduplicateStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: DeduplicateConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        let pattern = regex::Regex::new(&config.filename_pattern)
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        Ok(Self { config, pattern })
    }
}

#[async_trait]
impl Stage for DeduplicateStage {
    fn name(&self) -> &str { "deduplicate" }
    fn requires_batch(&self) -> bool { true }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: "deduplicate".to_string(),
            item_id: String::new(),
            message: "deduplicate requires batch mode".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Group items by extracted key.
        let mut groups: BTreeMap<String, Vec<PipelineItem>> = BTreeMap::new();
        let mut ungrouped: Vec<PipelineItem> = Vec::new();

        for item in items {
            if let Some(key) = self.extract_group_key(&item.display_name) {
                groups.entry(key).or_default().push(item);
            } else {
                // Files not matching the pattern pass through.
                ungrouped.push(item);
            }
        }

        // 2. Within each group, deduplicate by content hash.
        let mut results: Vec<PipelineItem> = ungrouped;

        for (_key, mut group) in groups {
            // Sort by timestamp extracted from filename (newest first or oldest first).
            if self.config.keep == "newest" {
                group.sort_by(|a, b| b.display_name.cmp(&a.display_name));
            } else {
                group.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            }

            // Keep unique hashes only.
            let mut seen_hashes = std::collections::HashSet::new();
            for item in group {
                let hash = self.compute_hash(&item.content);
                if seen_hashes.insert(hash) {
                    results.push(item);
                } else {
                    tracing::info!(
                        item_id = %item.id,
                        display_name = %item.display_name,
                        "deduplicated (duplicate content hash)"
                    );
                }
            }
        }

        Ok(results)
    }
}

impl DeduplicateStage {
    fn extract_group_key(&self, filename: &str) -> Option<String> {
        let caps = self.pattern.captures(filename)?;
        let key_parts: Vec<&str> = self.config.group_by.iter()
            .filter_map(|name| caps.name(name).map(|m| m.as_str()))
            .collect();
        if key_parts.is_empty() { None } else { Some(key_parts.join("|")) }
    }

    fn compute_hash(&self, content: &[u8]) -> String {
        match self.config.hash_algorithm.as_str() {
            "md5" => format!("{:x}", md5::compute(content)),
            "blake3" => blake3::hash(content).to_hex().to_string(),
            _ => blake3::hash(content).to_hex().to_string(),
        }
    }
}
```

**Dependencies:**

```toml
md5 = "0.7"
```

### 5.3 Tests

1. `test_dedup_no_duplicates` — 3 unique files → 3 output
2. `test_dedup_exact_duplicate_removed` — 2 files same hash → 1 output
3. `test_dedup_keeps_newest` — newest timestamp file kept
4. `test_dedup_keeps_oldest` — config keep="oldest"
5. `test_dedup_groups_by_type_and_date` — transaction + item files grouped separately
6. `test_dedup_unmatched_pattern_passes_through`
7. `test_dedup_walgreens_naming` — `wag_banyan_transaction_file_20260315143022.csv`
8. `test_dedup_requires_batch`
9. `test_dedup_empty_input`

---

## 6. Milestone 9.5: S3 Sink Stage

### 6.1 Scope

New crate `ecl-sink-s3` for writing to AWS S3 (needed for Chime, also useful generally).

### 6.2 Crate Structure

```
crates/ecl-sink-s3/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 6.3 Cargo.toml

```toml
[package]
name = "ecl-sink-s3"
version.workspace = true
edition.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

aws-sdk-s3 = "1"
aws-config = { version = "1", features = ["behavior-version-latest"] }

tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

### 6.4 Implementation

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct S3SinkConfig {
    pub bucket: String,
    pub prefix: String,
    /// Output format: "raw", "json_lines", "csv"
    #[serde(default = "default_raw")]
    pub format: String,
    /// AWS region. Default: "us-west-2"
    #[serde(default = "default_region")]
    pub region: String,
    /// Credential source: env vars (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    /// or a secret name for JSON credentials.
    #[serde(default)]
    pub credentials: Option<CredentialRef>,
    /// Filter: "all", "valid_only", "errors_only"
    #[serde(default = "default_all")]
    pub filter: String,
}

#[derive(Debug)]
pub struct S3SinkStage {
    config: S3SinkConfig,
    client: aws_sdk_s3::Client,
}

impl S3SinkStage {
    pub async fn from_params(
        params: &serde_json::Value,
        secret_resolver: &dyn SecretResolver,
    ) -> Result<Self, StageError> {
        let config: S3SinkConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // Build AWS config (from env, instance profile, or explicit creds).
        let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_config::Region::new(config.region.clone()))
            .load()
            .await;

        let client = aws_sdk_s3::Client::new(&aws_config);

        Ok(Self { config, client })
    }
}

#[async_trait]
impl Stage for S3SinkStage {
    fn name(&self) -> &str { "s3_sink" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // Apply filter.
        if !passes_filter(&item, &self.config.filter) {
            return Ok(vec![]);
        }

        let key = format!("{}{}", self.config.prefix, item.display_name);
        let body = match self.config.format.as_str() {
            "json_lines" => {
                let record = item.record.as_ref()
                    .ok_or_else(|| StageError::Permanent { /* ... */ })?;
                serde_json::to_vec(record)
                    .map_err(|e| StageError::Permanent { /* ... */ })?
            }
            _ => item.content.to_vec(),
        };

        self.client.put_object()
            .bucket(&self.config.bucket)
            .key(&key)
            .body(body.into())
            .send()
            .await
            .map_err(|e| StageError::Transient {
                stage: "s3_sink".to_string(),
                item_id: item.id.clone(),
                message: e.to_string(),
            })?;

        Ok(vec![])  // Terminal stage
    }
}
```

### 6.5 Tests

1. `test_s3_sink_config_deserialize`
2. `test_s3_sink_filter_logic`
3. `test_s3_sink_key_construction`

---

## 7. Milestone 9.6: Pipeline Chaining

### 7.1 Scope

Allow a pipeline to trigger one or more child pipelines on completion.

### 7.2 Spec Extension

#### File: `crates/ecl-pipeline-spec/src/lib.rs`

```rust
pub struct PipelineSpec {
    // ... existing fields ...
    /// Pipeline chaining: trigger other pipelines on completion.
    #[serde(default)]
    pub triggers: Option<TriggersSpec>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersSpec {
    /// Pipeline TOML config paths to trigger on success.
    #[serde(default)]
    pub on_success: Vec<String>,
    /// Pipeline TOML config paths to trigger on failure.
    #[serde(default)]
    pub on_failure: Vec<String>,
}
```

### 7.3 Runner Integration

#### File: `crates/ecl-pipeline/src/runner.rs`

After `run()` completes:

```rust
pub async fn run(&mut self) -> Result<&PipelineState> {
    // ... existing execution ...

    // Pipeline chaining.
    if let Some(triggers) = &self.topology.spec.triggers {
        match &self.state.status {
            PipelineStatus::Completed { .. } => {
                for child_path in &triggers.on_success {
                    self.trigger_child_pipeline(child_path).await?;
                }
            }
            PipelineStatus::Failed { .. } => {
                for child_path in &triggers.on_failure {
                    self.trigger_child_pipeline(child_path).await?;
                }
            }
            _ => {}
        }
    }

    Ok(&self.state)
}

async fn trigger_child_pipeline(&self, config_path: &str) -> Result<()> {
    tracing::info!(child = %config_path, "triggering child pipeline");

    // Spawn as a child process so it has its own memory space,
    // state store, and checkpoint.
    let status = tokio::process::Command::new(std::env::current_exe()?)
        .args(["pipeline", "run", config_path])
        .status()
        .await
        .map_err(|e| PipelineError::PushSource {
            source_name: "trigger".to_string(),
            detail: format!("failed to spawn child pipeline: {e}"),
        })?;

    if !status.success() {
        tracing::warn!(
            child = %config_path,
            exit_code = ?status.code(),
            "child pipeline exited with error"
        );
    }

    Ok(())
}
```

**Alternative (simpler, better for testing):** Instead of spawning a process, load and run the child pipeline in-process with a fresh `PipelineRunner`:

```rust
async fn trigger_child_pipeline(&self, config_path: &str) -> Result<()> {
    let toml_str = tokio::fs::read_to_string(config_path).await?;
    let spec = PipelineSpec::from_toml(&toml_str)?;
    // ... resolve topology, create runner, run ...
}
```

**Decision:** Support both. Default is process-level (isolation). Add `trigger_mode = "in_process"` option for testing.

### 7.4 Tests

1. `test_triggers_spec_serde`
2. `test_triggers_on_success_called` — mock child pipeline
3. `test_triggers_on_failure_called`
4. `test_triggers_none_no_error`
5. `test_triggers_child_failure_logged_not_fatal`

---

## 8. Milestone 9.7: Scheduling & Cron Triggers

### 8.1 Scope

Add `ecl pipeline schedule <config.toml>` CLI command that runs pipelines on a cron schedule.

### 8.2 Implementation

#### File: `crates/ecl-cli/src/pipeline/schedule.rs` (new)

```rust
use cron::Schedule;
use std::str::FromStr;

pub async fn execute(config_path: PathBuf, cron_expr: Option<String>) -> Result<()> {
    let toml_str = tokio::fs::read_to_string(&config_path).await?;
    let spec = PipelineSpec::from_toml(&toml_str)?;

    // Get cron expression from CLI arg or spec.
    let cron_str = cron_expr
        .or_else(|| spec.schedule.as_ref().map(|s| s.cron.clone()))
        .ok_or_else(|| anyhow::anyhow!("no cron schedule specified"))?;

    let schedule = Schedule::from_str(&cron_str)
        .map_err(|e| anyhow::anyhow!("invalid cron expression: {e}"))?;

    tracing::info!(cron = %cron_str, pipeline = %spec.name, "starting scheduler");

    loop {
        let next = schedule.upcoming(chrono::Utc).next()
            .ok_or_else(|| anyhow::anyhow!("no upcoming schedule"))?;

        let wait_duration = (next - chrono::Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::from_secs(1));

        tracing::info!(
            next_run = %next,
            wait_secs = wait_duration.as_secs(),
            "waiting for next scheduled run"
        );

        tokio::time::sleep(wait_duration).await;

        tracing::info!(pipeline = %spec.name, "starting scheduled run");
        match super::run::execute(config_path.clone()).await {
            Ok(()) => tracing::info!(pipeline = %spec.name, "scheduled run completed"),
            Err(e) => tracing::error!(pipeline = %spec.name, error = %e, "scheduled run failed"),
        }
    }
}
```

### 8.3 Spec Extension

```rust
pub struct PipelineSpec {
    // ... existing fields ...
    /// Optional schedule configuration.
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    /// Cron expression (5 or 7 fields). Example: "30 21 * * *"
    pub cron: String,
}
```

### 8.4 Dependencies

```toml
# In ecl-cli/Cargo.toml:
cron = "0.15"
```

### 8.5 Tests

1. `test_schedule_spec_serde`
2. `test_cron_parse_valid` — "30 21 ** *"
3. `test_cron_parse_invalid` — error handling
4. `test_schedule_computes_next_run` — verify upcoming time calculation

---

## 9. Milestone 9.8: Walgreens End-to-End Integration Test

### 9.1 Test Data

Create fixture directory `crates/ecl-pipeline/tests/fixtures/walgreens/`:

Generate PGP-encrypted test CSVs:

- `wag_banyan_transaction_file_20260315143022.csv.pgp` — 5 transactions
- `wag_banyan_payment_file_20260315143022.csv.pgp` — 8 tenders (some split)
- `wag_banyan_item_file_20260315143022.csv.pgp` — 12 items
- `wag_banyan_store_file_20260315143022.csv.pgp` — 3 stores
- `wag_banyan_product_file_20260315143022.csv.pgp` — 10 products

Plus a duplicate transaction file (same content, different timestamp):

- `wag_banyan_transaction_file_20260315153022.csv.pgp` — duplicate for dedup testing

Test data includes:

- Digital transaction (prices in cents, needs /100)
- BOPIS transaction (bopisFlag = 'Y')
- Transaction with online store 5995 (UTC timezone)
- Item with hot-key UPC 999999999999
- Split payment tender (2 payment methods on one transaction)
- Debit Card/Cashback negative amount

### 9.2 Test Pipeline Specs

Create two specs:

- `walgreens_ingestion.toml` — filesystem source → GCS sink (mocked)
- `walgreens_transformation.toml` — filesystem source (pre-decrypted for speed), full pipeline

### 9.3 Test Scenarios

#### File: `crates/ecl-pipeline/tests/integration/walgreens_e2e.rs` (new)

1. **`test_wag_e2e_pgp_decrypt`**
   - Encrypt test CSV with test PGP key
   - Run decrypt stage
   - Verify decrypted content matches original

2. **`test_wag_e2e_deduplication`**
   - 2 transaction files with same content, different timestamps
   - After dedup: only 1 file remains (newest)

3. **`test_wag_e2e_full_transformation`**
   - Parse 5 CSV types into 5 streams
   - Map fields per stream (price correction, payment mapping, UPC check digit, timezone)
   - Join items with products
   - Aggregate tenders by transaction
   - Assemble receipts
   - Validate
   - Emit to output directory

4. **`test_wag_e2e_digital_price_correction`**
   - Digital file amounts divided by 100

5. **`test_wag_e2e_bopis_detection`**
   - bopisFlag='Y' → order_type='web', fulfillment_type='in_store'

6. **`test_wag_e2e_payment_type_mapping`**
   - Credit Card→CREDIT, Visa→VI, etc.

7. **`test_wag_e2e_pipeline_chaining`**
   - Ingestion pipeline triggers transformation pipeline
   - Both complete successfully

8. **`test_wag_e2e_lifecycle_on_success`**
   - Verify files moved from staging to historical

9. **`test_wag_e2e_lifecycle_on_failure`**
   - Inject failure, verify files moved back to input

---

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
- [ ] Walgreens E2E integration test passes (decrypt → dedup → transform → output)
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
                       9.8 Walgreens E2E ─────────────┘
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
| `tests/fixtures/walgreens/*.csv.pgp` | 9.8 |
| `tests/fixtures/walgreens_*.toml` | 9.8 |
| `tests/integration/walgreens_e2e.rs` | 9.8 |

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

## Appendix C: Walgreens-Specific Transformation Stages

These are **not new stage types** — they use existing stages from Phase 1 and 2 with Walgreens-specific configurations:

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

The key insight: **no Walgreens-specific Rust code is needed.** All transformations are expressed as stage configurations in TOML. The only new infrastructure needed is SFTP, PGP decryption, deduplication, secrets, and pipeline chaining — which are all generic, reusable capabilities.
