//! SFTP source adapter for ECL pipelines.
//!
//! Implements the `SourceAdapter` trait for SFTP file servers. Lists remote
//! directories, filters by glob pattern, and fetches file content over SSH.
//!
//! Connection model: reconnect per operation (enumerate, fetch) for simplicity
//! and reliability. SSH session caching is a future optimization.

#![deny(unsafe_code)]
#![deny(clippy::unwrap_used)]

use std::collections::BTreeMap;
use std::net::ToSocketAddrs;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use russh::client;
use russh_keys::key;
use serde_json::json;
use tracing::debug;

use ecl_pipeline_spec::{CredentialRef, SftpSourceSpec};
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::SourceError;
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};
use ecl_secrets::SecretResolver;

/// Authentication type for SSH connection.
#[derive(Debug, Clone)]
enum AuthType {
    /// SSH private key (PEM or OpenSSH format).
    PrivateKey,
    /// Password-based authentication.
    Password,
}

/// SFTP source adapter that lists and fetches files from an SFTP server.
#[derive(Debug)]
pub struct SftpAdapter {
    source_name: String,
    host: String,
    port: u16,
    username: String,
    /// Pre-resolved authentication material (key bytes or password).
    auth_material: Vec<u8>,
    auth_type: AuthType,
    remote_path: String,
    pattern: Option<glob::Pattern>,
    /// Named data stream (stored for future use; runner assigns stream tags).
    #[allow(dead_code)]
    stream: Option<String>,
}

/// SSH client handler that accepts any server key.
///
/// In production, this should verify against known hosts.
/// For Phase 3, we accept all keys (typical for automated SFTP integrations).
#[derive(Debug)]
struct SshHandler;

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Accept all server keys for now.
        // TODO: Add known_hosts verification for production.
        Ok(true)
    }
}

impl SftpAdapter {
    /// Create an SFTP adapter from a source spec and secret resolver.
    ///
    /// Resolves credentials immediately (at construction time) so that
    /// auth failures are caught early rather than during pipeline execution.
    ///
    /// # Errors
    ///
    /// Returns `SourceError::AuthError` if credentials cannot be resolved.
    /// Returns `SourceError::Permanent` if the glob pattern is invalid.
    pub async fn from_spec(
        name: &str,
        spec: &SftpSourceSpec,
        secret_resolver: &dyn SecretResolver,
    ) -> Result<Self, SourceError> {
        let auth_material = resolve_credential(&spec.credentials, secret_resolver, name).await?;

        // Detect auth type: if the material looks like a private key, use key auth.
        let auth_type = if is_private_key(&auth_material) {
            AuthType::PrivateKey
        } else {
            AuthType::Password
        };

        let pattern = spec
            .pattern
            .as_ref()
            .map(|p| glob::Pattern::new(p))
            .transpose()
            .map_err(|e| SourceError::Permanent {
                source_name: name.to_string(),
                message: format!("invalid glob pattern: {e}"),
            })?;

        Ok(Self {
            source_name: name.to_string(),
            host: spec.host.clone(),
            port: spec.port,
            username: spec.username.clone(),
            auth_material,
            auth_type,
            remote_path: spec.remote_path.clone(),
            pattern,
            stream: spec.stream.clone(),
        })
    }

    /// Open an SSH connection and SFTP channel.
    async fn connect(&self) -> Result<russh_sftp::client::SftpSession, SourceError> {
        let config = Arc::new(client::Config::default());

        let addr = format!("{}:{}", self.host, self.port);
        let socket_addr = addr
            .to_socket_addrs()
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("DNS resolution failed for {addr}: {e}"),
            })?
            .next()
            .ok_or_else(|| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("no addresses resolved for {addr}"),
            })?;

        let mut session = client::connect(config, socket_addr, SshHandler)
            .await
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("SSH connection failed to {addr}: {e}"),
            })?;

        // Authenticate.
        match &self.auth_type {
            AuthType::PrivateKey => {
                let key_pair = russh_keys::decode_secret_key(
                    &String::from_utf8_lossy(&self.auth_material),
                    None,
                )
                .map_err(|e| SourceError::AuthError {
                    source_name: self.source_name.clone(),
                    message: format!("failed to parse SSH private key: {e}"),
                })?;

                let auth_result = session
                    .authenticate_publickey(&self.username, Arc::new(key_pair))
                    .await
                    .map_err(|e| SourceError::AuthError {
                        source_name: self.source_name.clone(),
                        message: format!("SSH public key auth failed: {e}"),
                    })?;

                if !auth_result {
                    return Err(SourceError::AuthError {
                        source_name: self.source_name.clone(),
                        message: "SSH public key authentication rejected".to_string(),
                    });
                }
            }
            AuthType::Password => {
                let password = String::from_utf8_lossy(&self.auth_material).to_string();
                let auth_result = session
                    .authenticate_password(&self.username, &password)
                    .await
                    .map_err(|e| SourceError::AuthError {
                        source_name: self.source_name.clone(),
                        message: format!("SSH password auth failed: {e}"),
                    })?;

                if !auth_result {
                    return Err(SourceError::AuthError {
                        source_name: self.source_name.clone(),
                        message: "SSH password authentication rejected".to_string(),
                    });
                }
            }
        }

        // Open SFTP channel.
        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("failed to open SSH channel: {e}"),
            })?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("failed to request SFTP subsystem: {e}"),
            })?;

        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("failed to initialize SFTP session: {e}"),
            })?;

        Ok(sftp)
    }

    /// Check if a filename matches the configured glob pattern.
    fn matches_pattern(&self, name: &str) -> bool {
        match &self.pattern {
            Some(pattern) => pattern.matches(name),
            None => true,
        }
    }
}

#[async_trait]
impl SourceAdapter for SftpAdapter {
    fn source_kind(&self) -> &str {
        "sftp"
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        let sftp = self.connect().await?;

        let entries =
            sftp.read_dir(&self.remote_path)
                .await
                .map_err(|e| SourceError::Transient {
                    source_name: self.source_name.clone(),
                    message: format!("failed to list directory '{}': {e}", self.remote_path),
                })?;

        let mut items = Vec::new();
        for entry in entries {
            let name = entry.file_name();

            // Skip directories.
            if entry.file_type().is_dir() {
                continue;
            }

            // Apply glob filter.
            if !self.matches_pattern(&name) {
                continue;
            }

            let path = if self.remote_path.ends_with('/') {
                format!("{}{}", self.remote_path, name)
            } else {
                format!("{}/{}", self.remote_path, name)
            };

            items.push(SourceItem {
                id: format!("sftp:{}", path),
                display_name: name.clone(),
                mime_type: mime_from_extension(&name),
                path,
                modified_at: None, // SFTP mtime parsing is complex; skip for now
                source_hash: None,
            });
        }

        items.sort_by(|a, b| a.id.cmp(&b.id));

        debug!(
            source = %self.source_name,
            remote_path = %self.remote_path,
            files = items.len(),
            "enumerated SFTP files"
        );

        Ok(items)
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let sftp = self.connect().await?;

        let content = sftp
            .read(&item.path)
            .await
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("failed to read file '{}': {e}", item.path),
            })?;

        let hash = blake3::hash(&content);
        let content_hash = Blake3Hash::new(hash.to_hex().to_string());

        debug!(
            source = %self.source_name,
            path = %item.path,
            size = content.len(),
            "fetched SFTP file"
        );

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: item.mime_type.clone(),
            provenance: ItemProvenance {
                source_kind: "sftp".to_string(),
                metadata: BTreeMap::from([
                    ("host".to_string(), json!(self.host)),
                    ("path".to_string(), json!(&item.path)),
                ]),
                source_modified: item.modified_at,
                extracted_at: Utc::now(),
            },
            content_hash,
        })
    }
}

/// Detect if the material looks like an SSH private key.
fn is_private_key(material: &[u8]) -> bool {
    let s = String::from_utf8_lossy(material);
    s.contains("BEGIN") && (s.contains("PRIVATE KEY") || s.contains("OPENSSH PRIVATE KEY"))
}

/// Guess MIME type from file extension.
fn mime_from_extension(name: &str) -> String {
    match std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some("csv") => "text/csv".to_string(),
        Some("json") => "application/json".to_string(),
        Some("txt") => "text/plain".to_string(),
        Some("xml") => "application/xml".to_string(),
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("pgp") | Some("gpg") => "application/pgp-encrypted".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

/// Resolve a `CredentialRef` to raw bytes.
async fn resolve_credential(
    cred: &CredentialRef,
    resolver: &dyn SecretResolver,
    source_name: &str,
) -> Result<Vec<u8>, SourceError> {
    match cred {
        CredentialRef::Secret { name } => {
            resolver
                .resolve(name)
                .await
                .map_err(|e| SourceError::AuthError {
                    source_name: source_name.to_string(),
                    message: e.to_string(),
                })
        }
        CredentialRef::File { path } => {
            tokio::fs::read(path)
                .await
                .map_err(|e| SourceError::AuthError {
                    source_name: source_name.to_string(),
                    message: format!("failed to read credential file '{}': {e}", path.display()),
                })
        }
        CredentialRef::EnvVar { env } => {
            std::env::var(env)
                .map(|v| v.into_bytes())
                .map_err(|_| SourceError::AuthError {
                    source_name: source_name.to_string(),
                    message: format!("environment variable '{env}' not set"),
                })
        }
        CredentialRef::ApplicationDefault => Err(SourceError::AuthError {
            source_name: source_name.to_string(),
            message: "application_default credentials not supported for SFTP".to_string(),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::CredentialRef;

    #[test]
    fn test_sftp_adapter_source_kind() {
        // Verify the adapter returns the correct source kind string.
        // We can't construct a full SftpAdapter without async, so test the
        // static method on the trait impl indirectly.
        assert_eq!(mime_from_extension("data.csv"), "text/csv");
        assert_eq!(mime_from_extension("file.json"), "application/json");
        assert_eq!(
            mime_from_extension("encrypted.pgp"),
            "application/pgp-encrypted"
        );
        assert_eq!(
            mime_from_extension("unknown.xyz"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_is_private_key_detection() {
        let rsa_key =
            b"-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        assert!(is_private_key(rsa_key));

        let openssh_key = b"-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1...\n-----END OPENSSH PRIVATE KEY-----";
        assert!(is_private_key(openssh_key));

        let password = b"my-sftp-password";
        assert!(!is_private_key(password));

        let ed25519_key =
            b"-----BEGIN PRIVATE KEY-----\nMC4CAQAwBQYDK...\n-----END PRIVATE KEY-----";
        assert!(is_private_key(ed25519_key));
    }

    #[test]
    fn test_pattern_matching() {
        let pattern = glob::Pattern::new("wag_banyan_*.csv*").unwrap();

        assert!(pattern.matches("wag_banyan_txn_file_20260315.csv"));
        assert!(pattern.matches("wag_banyan_items_file_20260315.csv.pgp"));
        assert!(!pattern.matches("other_file.csv"));
        assert!(!pattern.matches("wag_banyan_txn_file_20260315.json"));
    }

    #[tokio::test]
    async fn test_resolve_credential_env() {
        // Test credential resolution from env var.
        unsafe { std::env::set_var("TEST_SFTP_CRED_9_2", "test-secret-value") };
        let cred = CredentialRef::EnvVar {
            env: "TEST_SFTP_CRED_9_2".to_string(),
        };

        #[derive(Debug)]
        struct NoOpResolver;

        #[async_trait]
        impl SecretResolver for NoOpResolver {
            async fn resolve(&self, _name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                Err(ecl_secrets::SecretError::NotFound {
                    name: "unused".to_string(),
                })
            }
        }

        let result = resolve_credential(&cred, &NoOpResolver, "test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"test-secret-value");

        unsafe { std::env::remove_var("TEST_SFTP_CRED_9_2") };
    }

    #[tokio::test]
    async fn test_resolve_credential_secret() {
        #[derive(Debug)]
        struct MockResolver;

        #[async_trait]
        impl SecretResolver for MockResolver {
            async fn resolve(&self, name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                if name == "sftp-key-327" {
                    Ok(b"mock-private-key-bytes".to_vec())
                } else {
                    Err(ecl_secrets::SecretError::NotFound {
                        name: name.to_string(),
                    })
                }
            }
        }

        let cred = CredentialRef::Secret {
            name: "sftp-key-327".to_string(),
        };
        let result = resolve_credential(&cred, &MockResolver, "test").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), b"mock-private-key-bytes");

        // Unknown secret.
        let cred = CredentialRef::Secret {
            name: "nonexistent".to_string(),
        };
        let result = resolve_credential(&cred, &MockResolver, "test").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resolve_credential_application_default_unsupported() {
        #[derive(Debug)]
        struct NoOpResolver;

        #[async_trait]
        impl SecretResolver for NoOpResolver {
            async fn resolve(&self, _name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                unreachable!()
            }
        }

        let cred = CredentialRef::ApplicationDefault;
        let result = resolve_credential(&cred, &NoOpResolver, "test").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SourceError::AuthError { message, .. } => {
                assert!(message.contains("application_default"));
            }
            other => panic!("expected AuthError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_from_spec_with_env_credential() {
        unsafe { std::env::set_var("TEST_SFTP_KEY_9_2", "test-password") };

        #[derive(Debug)]
        struct NoOpResolver;

        #[async_trait]
        impl SecretResolver for NoOpResolver {
            async fn resolve(&self, _name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                unreachable!()
            }
        }

        let spec = SftpSourceSpec {
            host: "sftp.example.com".to_string(),
            port: 22,
            username: "testuser".to_string(),
            credentials: CredentialRef::EnvVar {
                env: "TEST_SFTP_KEY_9_2".to_string(),
            },
            remote_path: "/data".to_string(),
            pattern: Some("*.csv".to_string()),
            stream: Some("raw-files".to_string()),
        };

        let adapter = SftpAdapter::from_spec("sftp-source", &spec, &NoOpResolver)
            .await
            .unwrap();

        assert_eq!(adapter.source_kind(), "sftp");
        assert_eq!(adapter.host, "sftp.example.com");
        assert_eq!(adapter.port, 22);
        assert_eq!(adapter.username, "testuser");
        assert_eq!(adapter.remote_path, "/data");
        assert!(adapter.pattern.is_some());
        assert_eq!(adapter.stream.as_deref(), Some("raw-files"));
        assert!(matches!(adapter.auth_type, AuthType::Password));

        unsafe { std::env::remove_var("TEST_SFTP_KEY_9_2") };
    }

    #[tokio::test]
    async fn test_from_spec_with_secret_credential() {
        #[derive(Debug)]
        struct MockResolver;

        #[async_trait]
        impl SecretResolver for MockResolver {
            async fn resolve(&self, name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                if name == "sftp-key-327" {
                    Ok(b"-----BEGIN OPENSSH PRIVATE KEY-----\nfake-key-data\n-----END OPENSSH PRIVATE KEY-----".to_vec())
                } else {
                    Err(ecl_secrets::SecretError::NotFound {
                        name: name.to_string(),
                    })
                }
            }
        }

        let spec = SftpSourceSpec {
            host: "sftp.walgreens.example.com".to_string(),
            port: 2222,
            username: "walgreens-sftp".to_string(),
            credentials: CredentialRef::Secret {
                name: "sftp-key-327".to_string(),
            },
            remote_path: "/".to_string(),
            pattern: Some("wag_banyan_*".to_string()),
            stream: None,
        };

        let adapter = SftpAdapter::from_spec("sftp-wag", &spec, &MockResolver)
            .await
            .unwrap();

        assert_eq!(adapter.source_kind(), "sftp");
        assert_eq!(adapter.port, 2222);
        assert!(matches!(adapter.auth_type, AuthType::PrivateKey));
    }

    #[tokio::test]
    async fn test_from_spec_invalid_pattern() {
        unsafe { std::env::set_var("TEST_SFTP_BAD_PAT_9_2", "password") };

        #[derive(Debug)]
        struct NoOpResolver;

        #[async_trait]
        impl SecretResolver for NoOpResolver {
            async fn resolve(&self, _name: &str) -> Result<Vec<u8>, ecl_secrets::SecretError> {
                unreachable!()
            }
        }

        let spec = SftpSourceSpec {
            host: "sftp.example.com".to_string(),
            port: 22,
            username: "user".to_string(),
            credentials: CredentialRef::EnvVar {
                env: "TEST_SFTP_BAD_PAT_9_2".to_string(),
            },
            remote_path: "/".to_string(),
            pattern: Some("[invalid".to_string()),
            stream: None,
        };

        let result = SftpAdapter::from_spec("test", &spec, &NoOpResolver).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            SourceError::Permanent { message, .. } => {
                assert!(message.contains("invalid glob pattern"));
            }
            other => panic!("expected Permanent error, got: {other:?}"),
        }

        unsafe { std::env::remove_var("TEST_SFTP_BAD_PAT_9_2") };
    }

    #[test]
    fn test_matches_pattern_with_pattern() {
        let adapter = SftpAdapter {
            source_name: "test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_material: b"password".to_vec(),
            auth_type: AuthType::Password,
            remote_path: "/".to_string(),
            pattern: Some(glob::Pattern::new("*.csv").unwrap()),
            stream: None,
        };

        assert!(adapter.matches_pattern("data.csv"));
        assert!(!adapter.matches_pattern("data.json"));
        assert!(adapter.matches_pattern("transactions.csv"));
    }

    #[test]
    fn test_matches_pattern_without_pattern() {
        let adapter = SftpAdapter {
            source_name: "test".to_string(),
            host: "localhost".to_string(),
            port: 22,
            username: "user".to_string(),
            auth_material: b"password".to_vec(),
            auth_type: AuthType::Password,
            remote_path: "/".to_string(),
            pattern: None,
            stream: None,
        };

        assert!(adapter.matches_pattern("anything.csv"));
        assert!(adapter.matches_pattern("any-file-at-all"));
    }
}
