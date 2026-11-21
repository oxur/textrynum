# 4. Milestone 9.3: PGP Decryption Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 708-872. Original bytes are recoverable from import commit 3a39aea. -->

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

**Note:** PGP decryption is CPU-bound. For large files or many files, consider wrapping in `spawn_blocking`. For typical Acme Inc. files (few MB each, ~10 files), inline is fine.

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

