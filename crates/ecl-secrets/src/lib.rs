//! Pluggable secret resolution for ECL pipelines.
//!
//! Provides the [`SecretResolver`] trait and implementations for resolving
//! secrets from environment variables and files. Additional providers
//! (GCP Secret Manager, etc.) can be added as optional features.

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod env;
pub mod file;

use async_trait::async_trait;
use thiserror::Error;

/// Errors that occur during secret resolution.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SecretError {
    /// The requested secret was not found.
    #[error("secret not found: {name}")]
    NotFound {
        /// Name of the missing secret.
        name: String,
    },
    /// Access to the secret was denied.
    #[error("access denied for secret: {name}")]
    AccessDenied {
        /// Name of the denied secret.
        name: String,
    },
    /// A provider-level error occurred.
    #[error("secret provider error: {message}")]
    Provider {
        /// Error message from the provider.
        message: String,
    },
}

/// Resolves secret values by name.
///
/// Secrets may be strings (UTF-8) or binary (PGP keys, etc.).
/// Implementations must be safe to share across threads and async tasks.
#[async_trait]
pub trait SecretResolver: Send + Sync + std::fmt::Debug {
    /// Resolve a secret by name, returning its value as bytes.
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError>;

    /// Resolve a secret as a UTF-8 string.
    ///
    /// Returns an error if the secret bytes are not valid UTF-8.
    async fn resolve_string(&self, name: &str) -> Result<String, SecretError> {
        let bytes = self.resolve(name).await?;
        String::from_utf8(bytes).map_err(|_| SecretError::Provider {
            message: format!("secret '{name}' is not valid UTF-8"),
        })
    }
}

/// Build a [`SecretResolver`] based on a provider name.
///
/// Currently supports:
/// - `"none"` / `"env"` — environment variable resolver
/// - `"file"` — file-based resolver
///
/// Returns an `EnvResolver` by default.
pub fn build_resolver(provider: &str) -> Result<Box<dyn SecretResolver>, SecretError> {
    match provider {
        "none" | "env" => Ok(Box::new(env::EnvResolver)),
        "file" => Ok(Box::new(file::FileResolver)),
        other => Err(SecretError::Provider {
            message: format!("unsupported secret provider: {other}"),
        }),
    }
}

/// Build a default resolver (environment variables).
pub fn default_resolver() -> Box<dyn SecretResolver> {
    Box::new(env::EnvResolver)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn test_build_resolver_none() {
        let resolver = build_resolver("none").unwrap();
        assert!(format!("{resolver:?}").contains("EnvResolver"));
    }

    #[test]
    fn test_build_resolver_env() {
        let resolver = build_resolver("env").unwrap();
        assert!(format!("{resolver:?}").contains("EnvResolver"));
    }

    #[test]
    fn test_build_resolver_file() {
        let resolver = build_resolver("file").unwrap();
        assert!(format!("{resolver:?}").contains("FileResolver"));
    }

    #[test]
    fn test_build_resolver_unsupported() {
        let err = build_resolver("aws_kms").unwrap_err();
        assert!(err.to_string().contains("unsupported"));
    }

    #[tokio::test]
    async fn test_resolve_string_valid_utf8() {
        // SAFETY: test-only, single-threaded test context.
        unsafe { std::env::set_var("ECL_TEST_SECRET_UTF8", "hello-world") };
        let resolver = env::EnvResolver;
        let result = resolver
            .resolve_string("ECL_TEST_SECRET_UTF8")
            .await
            .unwrap();
        assert_eq!(result, "hello-world");
        unsafe { std::env::remove_var("ECL_TEST_SECRET_UTF8") };
    }

    #[tokio::test]
    async fn test_resolve_string_not_found() {
        let resolver = env::EnvResolver;
        let err = resolver
            .resolve_string("ECL_TEST_NONEXISTENT_SECRET_XYZ")
            .await
            .unwrap_err();
        assert!(matches!(err, SecretError::NotFound { .. }));
    }
}
