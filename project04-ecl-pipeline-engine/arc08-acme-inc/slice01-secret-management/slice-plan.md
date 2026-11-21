# 2. Milestone 9.1: Secret Management

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 175-437. Original bytes are recoverable from import commit 3a39aea. -->

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

