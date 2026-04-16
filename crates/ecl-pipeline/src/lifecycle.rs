//! File lifecycle management for GCS-based pipelines.
//!
//! After pipeline execution, moves processed files between GCS prefixes:
//! - **On success**: `staging/` → `historical/{run_id}/` (or delete)
//! - **On failure**: `staging/` → `input/` (or `error/`)
//!
//! Uses the GCS JSON API (copy + delete) for moves, reusing the
//! `TokenProvider` from `ecl-adapter-gcs` for authentication.

use ecl_adapter_gcs::auth::TokenProvider;
use ecl_adapter_gcs::types::{GCS_API_BASE_URL, GCS_READWRITE_SCOPE};
use ecl_pipeline_spec::lifecycle::{LifecycleAction, LifecycleSpec};
use tracing::{debug, warn};

use crate::error::{PipelineError, Result};

/// Manages file lifecycle operations on GCS.
#[derive(Debug)]
pub struct LifecycleManager {
    spec: LifecycleSpec,
    http_client: reqwest::Client,
    token_provider: TokenProvider,
    base_url: String,
}

impl LifecycleManager {
    /// Create a new lifecycle manager from a `LifecycleSpec`.
    pub fn new(spec: &LifecycleSpec) -> Self {
        let http_client = reqwest::Client::new();
        let token_provider = TokenProvider::new(
            spec.credentials.clone(),
            http_client.clone(),
            GCS_READWRITE_SCOPE,
        );

        Self {
            spec: spec.clone(),
            http_client,
            token_provider,
            base_url: GCS_API_BASE_URL.to_string(),
        }
    }

    /// Override the GCS API base URL (for testing with wiremock).
    #[cfg(test)]
    fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Override the token provider (for testing).
    #[cfg(test)]
    fn with_token_provider(mut self, provider: TokenProvider) -> Self {
        self.token_provider = provider;
        self
    }

    /// Execute the on-success lifecycle action.
    ///
    /// Lists objects under the staging prefix and moves them to the
    /// historical prefix (under a run-ID subdirectory).
    pub async fn on_success(&self, run_id: &str, source_objects: &[String]) -> Result<()> {
        match self.spec.on_success {
            LifecycleAction::MoveToHistorical => {
                let dest_prefix = format!(
                    "{}{}/",
                    self.spec.historical_prefix.trim_end_matches('/'),
                    if run_id.is_empty() { "" } else { "/" }
                );
                let dest_prefix = if run_id.is_empty() {
                    dest_prefix
                } else {
                    format!(
                        "{}{}/",
                        self.spec.historical_prefix.trim_end_matches('/'),
                        format_args!("/{run_id}")
                    )
                };
                self.move_objects(source_objects, &dest_prefix).await?;
            }
            LifecycleAction::Delete => {
                self.delete_objects(source_objects).await?;
            }
            LifecycleAction::None => {
                debug!("lifecycle on_success: no action configured");
            }
            _ => {
                warn!(action = ?self.spec.on_success, "unexpected on_success action, skipping");
            }
        }
        Ok(())
    }

    /// Execute the on-failure lifecycle action.
    ///
    /// Depending on configuration, moves staging files back to input
    /// or to an error prefix.
    pub async fn on_failure(&self, source_objects: &[String]) -> Result<()> {
        match self.spec.on_failure {
            LifecycleAction::MoveToInput => {
                // "Move to input" means removing the staging prefix.
                // If staging_prefix is "staging/foo.csv", the input is just "foo.csv"
                // at the bucket root (or wherever the original was).
                // For simplicity, we move to the input prefix (which is the bucket root
                // minus the staging prefix). In practice, the user configures where
                // "input" lives. For Phase 1, we treat it as moving objects to the
                // same name without the staging prefix — i.e., the object stays in
                // place. This is a no-op since the pipeline reads from staging.
                debug!(
                    "lifecycle on_failure: move_to_input is a no-op in Phase 1 (files stay in staging for retry)"
                );
            }
            LifecycleAction::MoveToError => {
                let error_prefix = format!("{}/", self.spec.error_prefix.trim_end_matches('/'));
                self.move_objects(source_objects, &error_prefix).await?;
            }
            LifecycleAction::None => {
                debug!("lifecycle on_failure: no action configured");
            }
            _ => {
                warn!(action = ?self.spec.on_failure, "unexpected on_failure action, skipping");
            }
        }
        Ok(())
    }

    /// Move objects by copying to a new prefix, then deleting the originals.
    async fn move_objects(&self, objects: &[String], dest_prefix: &str) -> Result<()> {
        let token = self.get_token().await?;

        for object_name in objects {
            let filename = object_name.rsplit('/').next().unwrap_or(object_name);
            let dest_name = format!("{dest_prefix}{filename}");

            self.copy_object(object_name, &dest_name, &token).await?;
            self.delete_object(object_name, &token).await?;

            debug!(
                from = %object_name,
                to = %dest_name,
                "lifecycle: moved object"
            );
        }

        Ok(())
    }

    /// Delete objects from the bucket.
    async fn delete_objects(&self, objects: &[String]) -> Result<()> {
        let token = self.get_token().await?;

        for object_name in objects {
            self.delete_object(object_name, &token).await?;
            debug!(object = %object_name, "lifecycle: deleted object");
        }

        Ok(())
    }

    /// Copy a GCS object to a new name within the same bucket.
    async fn copy_object(&self, source: &str, dest: &str, token: &str) -> Result<()> {
        let url = format!(
            "{}/b/{}/o/{}/copyTo/b/{}/o/{}",
            self.base_url,
            self.spec.bucket,
            urlencoded(source),
            self.spec.bucket,
            urlencoded(dest),
        );

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| PipelineError::Lifecycle {
                message: format!("GCS copy HTTP error: {e}"),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PipelineError::Lifecycle {
                message: format!("GCS copy failed ({status}): {body}"),
            });
        }

        Ok(())
    }

    /// Delete a GCS object.
    async fn delete_object(&self, object_name: &str, token: &str) -> Result<()> {
        let url = format!(
            "{}/b/{}/o/{}",
            self.base_url,
            self.spec.bucket,
            urlencoded(object_name),
        );

        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| PipelineError::Lifecycle {
                message: format!("GCS delete HTTP error: {e}"),
            })?;

        // 204 No Content is the expected success response for delete.
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PipelineError::Lifecycle {
                message: format!("GCS delete failed ({status}): {body}"),
            });
        }

        Ok(())
    }

    /// Get an auth token from the token provider.
    async fn get_token(&self) -> Result<String> {
        self.token_provider
            .get_token()
            .await
            .map_err(|e| PipelineError::Lifecycle {
                message: format!("GCS auth error: {e}"),
            })
    }
}

/// Minimal percent-encoding for GCS object names in URL paths.
fn urlencoded(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '/' => out.push_str("%2F"),
            ' ' => out.push_str("%20"),
            '#' => out.push_str("%23"),
            '?' => out.push_str("%3F"),
            '&' => out.push_str("%26"),
            '+' => out.push_str("%2B"),
            '%' => out.push_str("%25"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::CredentialRef;
    use ecl_pipeline_spec::lifecycle::LifecycleAction;

    fn test_spec() -> LifecycleSpec {
        LifecycleSpec {
            bucket: "test-bucket".to_string(),
            staging_prefix: "staging/".to_string(),
            historical_prefix: "historical/".to_string(),
            error_prefix: "error/".to_string(),
            on_success: LifecycleAction::MoveToHistorical,
            on_failure: LifecycleAction::MoveToError,
            credentials: CredentialRef::ApplicationDefault,
        }
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("simple.csv"), "simple.csv");
        assert_eq!(urlencoded("staging/file.csv"), "staging%2Ffile.csv");
        assert_eq!(urlencoded("has space"), "has%20space");
    }

    #[tokio::test]
    async fn test_lifecycle_on_success_move_to_historical() {
        let mock_server = wiremock::MockServer::start().await;

        // Mock the copy endpoint.
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path_regex(".*/copyTo/.*"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"name": "copied"})),
            )
            .mount(&mock_server)
            .await;

        // Mock the delete endpoint.
        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let manager = LifecycleManager::new(&test_spec())
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let objects = vec![
            "staging/file1.csv".to_string(),
            "staging/file2.csv".to_string(),
        ];
        manager.on_success("run-001", &objects).await.unwrap();
    }

    #[tokio::test]
    async fn test_lifecycle_on_success_delete() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let mut spec = test_spec();
        spec.on_success = LifecycleAction::Delete;

        let manager = LifecycleManager::new(&spec)
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let objects = vec!["staging/file1.csv".to_string()];
        manager.on_success("run-001", &objects).await.unwrap();
    }

    #[tokio::test]
    async fn test_lifecycle_on_success_none() {
        let mut spec = test_spec();
        spec.on_success = LifecycleAction::None;

        let manager = LifecycleManager::new(&spec);
        // Should succeed without any HTTP calls.
        manager.on_success("run-001", &[]).await.unwrap();
    }

    #[tokio::test]
    async fn test_lifecycle_on_failure_move_to_error() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path_regex(".*/copyTo/.*"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"name": "copied"})),
            )
            .mount(&mock_server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(wiremock::ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let manager = LifecycleManager::new(&test_spec())
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let objects = vec!["staging/bad-file.csv".to_string()];
        manager.on_failure(&objects).await.unwrap();
    }

    #[tokio::test]
    async fn test_lifecycle_on_failure_none() {
        let mut spec = test_spec();
        spec.on_failure = LifecycleAction::None;

        let manager = LifecycleManager::new(&spec);
        manager.on_failure(&[]).await.unwrap();
    }

    #[tokio::test]
    async fn test_lifecycle_copy_error_propagates() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .respond_with(wiremock::ResponseTemplate::new(403).set_body_string("Access Denied"))
            .mount(&mock_server)
            .await;

        let manager = LifecycleManager::new(&test_spec())
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let objects = vec!["staging/file.csv".to_string()];
        let result = manager.on_success("run-001", &objects).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("403"),
            "error should contain status: {err}"
        );
    }

    #[tokio::test]
    async fn test_lifecycle_delete_error_propagates() {
        let mock_server = wiremock::MockServer::start().await;

        // Copy succeeds but delete fails.
        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path_regex(".*/copyTo/.*"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({"name": "copied"})),
            )
            .mount(&mock_server)
            .await;

        wiremock::Mock::given(wiremock::matchers::method("DELETE"))
            .respond_with(
                wiremock::ResponseTemplate::new(500).set_body_string("Internal Server Error"),
            )
            .mount(&mock_server)
            .await;

        let manager = LifecycleManager::new(&test_spec())
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let objects = vec!["staging/file.csv".to_string()];
        let result = manager.on_success("run-001", &objects).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_lifecycle_move_to_input_is_noop() {
        let mut spec = test_spec();
        spec.on_failure = LifecycleAction::MoveToInput;

        let manager = LifecycleManager::new(&spec);
        // Should succeed without HTTP calls — it's a no-op in Phase 1.
        let objects = vec!["staging/file.csv".to_string()];
        manager.on_failure(&objects).await.unwrap();
    }
}
