//! Environment variable secret resolver.

use async_trait::async_trait;

use crate::{SecretError, SecretResolver};

/// Resolves secrets from environment variables.
///
/// The secret name is used directly as the environment variable name.
#[derive(Debug)]
pub struct EnvResolver;

#[async_trait]
impl SecretResolver for EnvResolver {
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError> {
        std::env::var(name)
            .map(|v| v.into_bytes())
            .map_err(|_| SecretError::NotFound {
                name: name.to_string(),
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, unsafe_code)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_env_resolver_found() {
        // SAFETY: test-only, single-threaded test context.
        unsafe { std::env::set_var("ECL_TEST_ENV_SECRET", "my-secret-value") };
        let resolver = EnvResolver;
        let result = resolver.resolve("ECL_TEST_ENV_SECRET").await.unwrap();
        assert_eq!(result, b"my-secret-value");
        unsafe { std::env::remove_var("ECL_TEST_ENV_SECRET") };
    }

    #[tokio::test]
    async fn test_env_resolver_not_found() {
        let resolver = EnvResolver;
        let err = resolver
            .resolve("ECL_TEST_NONEXISTENT_VAR_ABC123")
            .await
            .unwrap_err();
        assert!(matches!(err, SecretError::NotFound { .. }));
        assert!(err.to_string().contains("ECL_TEST_NONEXISTENT_VAR_ABC123"));
    }
}
