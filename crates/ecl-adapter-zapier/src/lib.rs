//! Zapier webhook push source adapter for the ECL pipeline.
//!
//! Receives webhook POST requests from Zapier and converts them into
//! `ExtractedDocument`s for pipeline processing. Supports Basic Auth
//! and Bearer token authentication.
//!
//! # Architecture
//!
//! The adapter runs an axum HTTP server that listens for incoming webhook
//! POSTs. Each valid request is parsed, hashed (blake3), and sent through
//! a bounded `mpsc` channel to the pipeline runner. The bounded channel
//! provides natural backpressure: if the runner falls behind, the HTTP
//! handler returns 429 to Zapier (which will retry later).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod error;
pub mod schemas;
pub mod server;

pub use error::ZapierAdapterError;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, Notify, mpsc};

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::source::ZapierSourceSpec;
use ecl_pipeline_topo::ExtractedDocument;
use ecl_pipeline_topo::PushSourceAdapter;
use ecl_pipeline_topo::error::{ResolveError, SourceError};

use crate::server::{WebhookState, run_server};

/// Zapier webhook push source adapter.
///
/// Implements `PushSourceAdapter` by running an axum HTTP server that
/// receives webhook POST requests from Zapier. Each request is validated,
/// parsed, and sent through a bounded channel to the pipeline runner.
#[derive(Debug)]
pub struct ZapierAdapter {
    source_name: String,
    spec: ZapierSourceSpec,
    /// Shared shutdown signal for the HTTP server.
    shutdown: Arc<Notify>,
    /// Sender half of the bounded channel (cloned into the HTTP handler).
    sender: mpsc::Sender<ExtractedDocument>,
    /// Receiver half — taken once when `start()` is called.
    receiver: Mutex<Option<mpsc::Receiver<ExtractedDocument>>>,
    /// Handle to the spawned server task (for monitoring).
    server_handle: Mutex<Option<tokio::task::JoinHandle<std::result::Result<(), SourceError>>>>,
}

impl ZapierAdapter {
    /// Create a new adapter from a `SourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError` if the spec is not a `Zapier` variant.
    pub fn from_spec(
        source_name: &str,
        spec: &SourceSpec,
    ) -> std::result::Result<Self, ResolveError> {
        let zapier_spec = match spec {
            SourceSpec::Zapier(s) => s.clone(),
            _ => {
                return Err(ResolveError::UnknownAdapter {
                    stage: source_name.to_string(),
                    adapter: "expected zapier source spec".to_string(),
                });
            }
        };

        let (sender, receiver) = mpsc::channel(zapier_spec.channel_capacity);

        Ok(Self {
            source_name: source_name.to_string(),
            spec: zapier_spec,
            shutdown: Arc::new(Notify::new()),
            sender,
            receiver: Mutex::new(Some(receiver)),
            server_handle: Mutex::new(None),
        })
    }

    /// Resolve the auth secret from the `CredentialRef`.
    ///
    /// Currently supports `EnvVar` (reads from environment) and
    /// `File` (reads file contents as the secret).
    fn resolve_secret(&self) -> std::result::Result<String, SourceError> {
        use ecl_pipeline_spec::source::CredentialRef;

        match &self.spec.credentials {
            CredentialRef::EnvVar { env } => {
                std::env::var(env).map_err(|_| SourceError::AuthError {
                    source_name: self.source_name.clone(),
                    message: format!("environment variable '{env}' not set"),
                })
            }
            CredentialRef::File { path } => std::fs::read_to_string(path)
                .map(|s| s.trim().to_string())
                .map_err(|e| SourceError::AuthError {
                    source_name: self.source_name.clone(),
                    message: format!("failed to read credentials file {}: {e}", path.display()),
                }),
            CredentialRef::ApplicationDefault => Err(SourceError::AuthError {
                source_name: self.source_name.clone(),
                message: "ApplicationDefault not supported for Zapier adapter".to_string(),
            }),
        }
    }
}

#[async_trait]
impl PushSourceAdapter for ZapierAdapter {
    fn source_kind(&self) -> &str {
        "zapier"
    }

    async fn start(&self) -> std::result::Result<mpsc::Receiver<ExtractedDocument>, SourceError> {
        // Take the receiver (can only start once).
        let receiver = self
            .receiver
            .lock()
            .await
            .take()
            .ok_or_else(|| SourceError::Permanent {
                source_name: self.source_name.clone(),
                message: "adapter already started".to_string(),
            })?;

        // Resolve the auth secret.
        let auth_secret = self.resolve_secret()?;

        // Build the shared state for the HTTP handler.
        let state = WebhookState {
            sender: self.sender.clone(),
            auth_username: self.spec.auth_username.clone(),
            auth_secret,
            source_name: self.source_name.clone(),
            default_source_hint: self
                .spec
                .default_source_hint
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
        };

        // Spawn the HTTP server.
        let bind_addr = self.spec.bind_addr.clone();
        let shutdown = self.shutdown.clone();
        let handle = tokio::spawn(async move { run_server(&bind_addr, state, shutdown).await });

        *self.server_handle.lock().await = Some(handle);

        Ok(receiver)
    }

    async fn shutdown(&self) -> std::result::Result<(), SourceError> {
        // Signal the server to stop.
        self.shutdown.notify_one();

        // Wait for the server task to complete.
        if let Some(handle) = self.server_handle.lock().await.take() {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    tracing::warn!(source = %self.source_name, "server shutdown error: {e}");
                }
                Err(e) => {
                    tracing::warn!(source = %self.source_name, "server task join error: {e}");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::source::CredentialRef;

    fn make_zapier_spec() -> SourceSpec {
        SourceSpec::Zapier(ZapierSourceSpec {
            bind_addr: "127.0.0.1:0".to_string(),
            auth_username: "test-user".to_string(),
            credentials: CredentialRef::EnvVar {
                env: "TEST_ZAPIER_SECRET".to_string(),
            },
            batch_max_items: 10,
            batch_timeout_secs: 5,
            channel_capacity: 100,
            default_source_hint: Some("granola".to_string()),
            stream: None,
        })
    }

    #[test]
    fn test_from_spec_success() {
        let adapter = ZapierAdapter::from_spec("test-zapier", &make_zapier_spec());
        assert!(adapter.is_ok());
        let adapter = adapter.unwrap();
        assert_eq!(adapter.source_name, "test-zapier");
        assert_eq!(adapter.spec.bind_addr, "127.0.0.1:0");
    }

    #[test]
    fn test_from_spec_wrong_variant() {
        let fs_spec = SourceSpec::Filesystem(ecl_pipeline_spec::source::FilesystemSourceSpec {
            root: std::path::PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
            stream: None,
        });
        let result = ZapierAdapter::from_spec("test", &fs_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_source_kind() {
        let adapter = ZapierAdapter::from_spec("test", &make_zapier_spec()).unwrap();
        assert_eq!(adapter.source_kind(), "zapier");
    }

    #[test]
    fn test_push_source_adapter_object_safety() {
        let adapter = ZapierAdapter::from_spec("test", &make_zapier_spec()).unwrap();
        let _dyn_adapter: Arc<dyn PushSourceAdapter> = Arc::new(adapter);
    }

    fn make_secret_file(secret: &str) -> tempfile::NamedTempFile {
        use std::io::Write;
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{secret}").unwrap();
        f
    }

    #[tokio::test]
    async fn test_start_with_file_credential() {
        let secret_file = make_secret_file("my-secret");
        let spec = SourceSpec::Zapier(ZapierSourceSpec {
            bind_addr: "127.0.0.1:0".to_string(),
            auth_username: "test".to_string(),
            credentials: CredentialRef::File {
                path: secret_file.path().to_path_buf(),
            },
            batch_max_items: 10,
            batch_timeout_secs: 5,
            channel_capacity: 10,
            default_source_hint: None,
            stream: None,
        });

        let adapter = ZapierAdapter::from_spec("test", &spec).unwrap();
        let mut rx = adapter.start().await.unwrap();

        // Shutdown and verify the receiver eventually closes.
        adapter.shutdown().await.unwrap();

        // After shutdown, receiver should eventually return None.
        // The server is stopped, sender is still alive in the adapter
        // but no one is sending.
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_start_twice_fails() {
        let secret_file = make_secret_file("secret");
        let spec = SourceSpec::Zapier(ZapierSourceSpec {
            bind_addr: "127.0.0.1:0".to_string(),
            auth_username: "test".to_string(),
            credentials: CredentialRef::File {
                path: secret_file.path().to_path_buf(),
            },
            batch_max_items: 10,
            batch_timeout_secs: 5,
            channel_capacity: 10,
            default_source_hint: None,
            stream: None,
        });

        let adapter = ZapierAdapter::from_spec("test", &spec).unwrap();
        let _rx = adapter.start().await.unwrap();

        // Second start should fail.
        let result = adapter.start().await;
        assert!(result.is_err());

        adapter.shutdown().await.unwrap();
    }

    #[test]
    fn test_resolve_secret_missing_env_var() {
        let spec = SourceSpec::Zapier(ZapierSourceSpec {
            bind_addr: "127.0.0.1:0".to_string(),
            auth_username: "test".to_string(),
            credentials: CredentialRef::EnvVar {
                env: "NONEXISTENT_ZAPIER_VAR_12345".to_string(),
            },
            batch_max_items: 10,
            batch_timeout_secs: 5,
            channel_capacity: 10,
            default_source_hint: None,
            stream: None,
        });

        let adapter = ZapierAdapter::from_spec("test", &spec).unwrap();
        let result = adapter.resolve_secret();
        assert!(result.is_err());
    }
}
