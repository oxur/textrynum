//! File-based secret resolver.

use async_trait::async_trait;

use crate::{SecretError, SecretResolver};

/// Resolves secrets by reading from the filesystem.
///
/// The secret name is interpreted as a file path. The entire file
/// contents are returned as the secret value (bytes, may be binary).
#[derive(Debug)]
pub struct FileResolver;

#[async_trait]
impl SecretResolver for FileResolver {
    async fn resolve(&self, name: &str) -> Result<Vec<u8>, SecretError> {
        tokio::fs::read(name)
            .await
            .map_err(|e| SecretError::NotFound {
                name: format!("{name}: {e}"),
            })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::io::Write;

    #[tokio::test]
    async fn test_file_resolver_found() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "file-secret-content").unwrap();
        let path = tmpfile.path().to_str().unwrap().to_string();

        let resolver = FileResolver;
        let result = resolver.resolve(&path).await.unwrap();
        assert_eq!(result, b"file-secret-content");
    }

    #[tokio::test]
    async fn test_file_resolver_binary_content() {
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        let binary_data: Vec<u8> = vec![0x00, 0x01, 0xFF, 0xFE, 0x80];
        tmpfile.write_all(&binary_data).unwrap();
        let path = tmpfile.path().to_str().unwrap().to_string();

        let resolver = FileResolver;
        let result = resolver.resolve(&path).await.unwrap();
        assert_eq!(result, binary_data);
    }

    #[tokio::test]
    async fn test_file_resolver_not_found() {
        let resolver = FileResolver;
        let err = resolver
            .resolve("/nonexistent/path/to/secret")
            .await
            .unwrap_err();
        assert!(matches!(err, SecretError::NotFound { .. }));
    }
}
