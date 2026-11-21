# 4. Milestone 7.3: GCS Source Adapter

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0026-phase-1-detailed-implementation-plan-make-affinity-work.md, lines 535-744. Original bytes are recoverable from import commit 3a39aea. -->

## 4. Milestone 7.3: GCS Source Adapter

### 4.1 Scope

New crate `ecl-adapter-gcs` implementing `SourceAdapter` for Google Cloud Storage.

### 4.2 Crate Structure

```
crates/ecl-adapter-gcs/
├── Cargo.toml
└── src/
    └── lib.rs
```

### 4.3 Cargo.toml

```toml
[package]
name = "ecl-adapter-gcs"
version.workspace = true
edition.workspace = true
rust-version.workspace = true

[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }

# GCS client
google-cloud-storage = { version = "0.22", features = ["default-tls"] }
google-cloud-auth = { version = "0.19", features = ["default-tls"] }

# Async
tokio = { workspace = true }
async-trait = { workspace = true }

# Hashing
blake3 = { workspace = true }

# Serde
serde = { workspace = true }
serde_json = { workspace = true }

# Matching
glob = { workspace = true }

# Logging
tracing = { workspace = true }

# Error handling
thiserror = { workspace = true }

# Time
chrono = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
tempfile = { workspace = true }
```

**Note:** If `google-cloud-storage` proves too heavy or has compile issues, fallback to raw `reqwest` + GCS JSON API (same pattern as `ecl-adapter-gdrive`). Evaluate during implementation.

### 4.4 SourceSpec Extension

#### File: `crates/ecl-pipeline-spec/src/source.rs`

Add new variant:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    // ... existing variants ...
    #[serde(rename = "gcs")]
    Gcs(GcsSourceSpec),
}

/// Google Cloud Storage source: list and fetch objects from a bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsSourceSpec {
    /// GCS bucket name (without gs:// prefix).
    pub bucket: String,
    /// Object prefix to filter listing (e.g., "staging/").
    #[serde(default)]
    pub prefix: String,
    /// Glob pattern to match object names (e.g., "Banyan-txn-file*.csv").
    #[serde(default)]
    pub pattern: Option<String>,
    /// Credentials for authentication.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
}

fn default_adc() -> CredentialRef {
    CredentialRef::ApplicationDefault
}
```

### 4.5 Adapter Implementation

#### File: `crates/ecl-adapter-gcs/src/lib.rs`

```rust
#[derive(Debug)]
pub struct GcsAdapter {
    bucket: String,
    prefix: String,
    pattern: Option<glob::Pattern>,
    // Client initialized lazily or eagerly based on auth method
    client: google_cloud_storage::client::Client,
}

impl GcsAdapter {
    pub async fn from_spec(spec: &GcsSourceSpec) -> Result<Self, SourceError> {
        // 1. Resolve credentials (ADC, env var, file)
        // 2. Build GCS client
        // 3. Compile glob pattern if present
        // 4. Return adapter
    }
}

#[async_trait]
impl SourceAdapter for GcsAdapter {
    fn source_kind(&self) -> &str { "gcs" }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        // 1. List objects with prefix
        // 2. Filter by glob pattern
        // 3. Map to SourceItem { id: object_name, display_name, mime_type, path, modified_at, source_hash (md5/crc32c) }
        // 4. Sort by name for determinism
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        // 1. Download object content as bytes
        // 2. Compute blake3 hash
        // 3. Build ItemProvenance with GCS metadata
        // 4. Return ExtractedDocument
    }
}
```

**Auth strategy:**

- `ApplicationDefault`: Use ADC (works on GCE, Cloud Run, local with `gcloud auth application-default login`)
- `EnvVar`: Read service account JSON from env var, parse, build credentials
- `File`: Read service account JSON from file path

**Error mapping:**

- 401/403 → `SourceError::AuthError`
- 404 → `SourceError::NotFound`
- 429 → `SourceError::RateLimited`
- 5xx / network → `SourceError::Transient`
- Other → `SourceError::Permanent`

**Tests:**

1. `test_gcs_adapter_from_spec_default_credentials`
2. `test_gcs_adapter_enumerate_filters_by_pattern`
3. `test_gcs_adapter_enumerate_no_pattern_returns_all`
4. `test_gcs_adapter_fetch_computes_hash`
5. `test_gcs_adapter_source_kind`
6. `test_gcs_adapter_serde_roundtrip` — GcsSourceSpec serialization

Note: Real GCS tests require either a test bucket or a mock. For unit tests, consider a thin HTTP abstraction that can be mocked (same approach as gdrive adapter with `wiremock`).

### 4.6 Registry Integration

#### File: `crates/ecl-cli/src/pipeline/registry.rs`

In `resolve_adapters`:

```rust
SourceSpec::Gcs(gcs_spec) => {
    let adapter = ecl_adapter_gcs::GcsAdapter::from_spec(gcs_spec).await
        .map_err(|e| ResolveError::UnknownAdapter {
            stage: name.to_string(),
            adapter: format!("gcs: {e}"),
        })?;
    adapters.insert(name.to_string(), Arc::new(adapter) as Arc<dyn SourceAdapter>);
}
```

In `source_kind`:

```rust
SourceSpec::Gcs(_) => "gcs",
```

### 4.7 Workspace Wiring

Add to root `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing ...
    "crates/ecl-adapter-gcs",
]
```

Add to `ecl-cli/Cargo.toml`:

```toml
ecl-adapter-gcs = { path = "../ecl-adapter-gcs" }
```

---

