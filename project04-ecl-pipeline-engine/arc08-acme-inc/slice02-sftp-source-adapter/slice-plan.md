# 3. Milestone 9.2: SFTP Source Adapter

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 438-707. Original bytes are recoverable from import commit 3a39aea. -->

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

