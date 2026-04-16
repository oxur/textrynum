//! Google Cloud Storage source adapter for the ECL pipeline runner.
//!
//! Implements `SourceAdapter` for GCS buckets using the JSON API via
//! `reqwest`. Supports glob-based object filtering, prefix scoping,
//! and three credential types (service account, env var, ADC).

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod auth;
pub mod error;
pub mod types;

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tracing::debug;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::{ResolveError, SourceError};
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

use crate::auth::TokenProvider;
use crate::error::GcsAdapterError;
use crate::types::{GCS_API_BASE_URL, GcsObject, ObjectListResponse};

/// Google Cloud Storage source adapter.
///
/// Lists and fetches objects from a GCS bucket using the JSON API.
/// Supports prefix filtering, glob pattern matching, and three
/// credential types via `CredentialRef`.
#[derive(Debug)]
pub struct GcsAdapter {
    source_name: String,
    bucket: String,
    prefix: String,
    pattern: Option<glob::Pattern>,
    http_client: reqwest::Client,
    token_provider: TokenProvider,
    base_url: String,
}

impl GcsAdapter {
    /// Create a new GCS adapter from a `SourceSpec`.
    ///
    /// Dispatches on the `SourceSpec` variant — returns `ResolveError` if
    /// the spec is not a GCS source.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError` if the spec is the wrong variant or if
    /// glob pattern compilation fails.
    pub fn from_spec(source_name: &str, spec: &SourceSpec) -> Result<Self, ResolveError> {
        let gcs_spec = match spec {
            SourceSpec::Gcs(gs) => gs,
            _ => {
                return Err(ResolveError::UnknownAdapter {
                    stage: source_name.to_string(),
                    adapter: "gcs".to_string(),
                });
            }
        };
        Self::from_gcs_spec(source_name, gcs_spec)
    }

    /// Create a new GCS adapter directly from a `GcsSourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError` if glob pattern compilation fails.
    pub fn from_gcs_spec(
        source_name: &str,
        spec: &ecl_pipeline_spec::source::GcsSourceSpec,
    ) -> Result<Self, ResolveError> {
        let pattern = spec
            .pattern
            .as_ref()
            .map(|p| {
                glob::Pattern::new(p).map_err(|e| {
                    ResolveError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        format!("invalid glob pattern '{p}': {e}"),
                    ))
                })
            })
            .transpose()?;

        let http_client = reqwest::Client::new();
        let token_provider = TokenProvider::new(
            spec.credentials.clone(),
            http_client.clone(),
            crate::types::GCS_READONLY_SCOPE,
        );

        Ok(Self {
            source_name: source_name.to_string(),
            bucket: spec.bucket.clone(),
            prefix: spec.prefix.clone(),
            pattern,
            http_client,
            token_provider,
            base_url: GCS_API_BASE_URL.to_string(),
        })
    }

    /// Override the GCS API base URL (for testing with wiremock).
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Override the token provider (for testing).
    pub fn with_token_provider(mut self, provider: TokenProvider) -> Self {
        self.token_provider = provider;
        self
    }

    /// List objects in the bucket with pagination.
    async fn list_objects(&self, token: &str) -> Result<Vec<GcsObject>, GcsAdapterError> {
        let mut all_objects = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let mut url = format!("{}/b/{}/o", self.base_url, self.bucket);

            let mut query_params = vec![];
            if !self.prefix.is_empty() {
                query_params.push(("prefix", self.prefix.as_str()));
            }
            if let Some(ref pt) = page_token {
                query_params.push(("pageToken", pt.as_str()));
            }
            // Request relevant fields only.
            query_params.push((
                "fields",
                "items(name,bucket,size,contentType,updated,md5Hash,generation),nextPageToken",
            ));

            if !query_params.is_empty() {
                url.push('?');
                let pairs: Vec<String> = query_params
                    .iter()
                    .map(|(k, v)| format!("{k}={}", urlencoded(v)))
                    .collect();
                url.push_str(&pairs.join("&"));
            }

            let response = self.http_client.get(&url).bearer_auth(token).send().await?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(GcsAdapterError::ApiError {
                    status: status.as_u16(),
                    message: body,
                });
            }

            let list_resp: ObjectListResponse = response.json().await?;
            all_objects.extend(list_resp.items);

            match list_resp.next_page_token {
                Some(pt) => page_token = Some(pt),
                None => break,
            }
        }

        Ok(all_objects)
    }

    /// Download the content of a single object.
    async fn download_object(
        &self,
        token: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, GcsAdapterError> {
        // URL-encode the object name for the path segment.
        let encoded_name = urlencoded(object_name);
        let url = format!(
            "{}/b/{}/o/{}?alt=media",
            self.base_url, self.bucket, encoded_name
        );

        let response = self.http_client.get(&url).bearer_auth(token).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GcsAdapterError::ApiError {
                status: status.as_u16(),
                message: body,
            });
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Map a GCS API error to a SourceError.
    fn map_api_error(err: GcsAdapterError, source_name: &str, item_id: &str) -> SourceError {
        match &err {
            GcsAdapterError::ApiError { status, .. } => match *status {
                401 | 403 => SourceError::AuthError {
                    source_name: source_name.to_string(),
                    message: err.to_string(),
                },
                404 => SourceError::NotFound {
                    source_name: source_name.to_string(),
                    item_id: item_id.to_string(),
                },
                429 => SourceError::RateLimited {
                    source_name: source_name.to_string(),
                    retry_after_secs: 0,
                },
                500..=599 => SourceError::Transient {
                    source_name: source_name.to_string(),
                    message: err.to_string(),
                },
                _ => SourceError::Permanent {
                    source_name: source_name.to_string(),
                    message: err.to_string(),
                },
            },
            GcsAdapterError::Http(_) => SourceError::Transient {
                source_name: source_name.to_string(),
                message: err.to_string(),
            },
            _ => SourceError::Permanent {
                source_name: source_name.to_string(),
                message: err.to_string(),
            },
        }
    }
}

/// Minimal URL percent-encoding for query parameters and path segments.
fn urlencoded(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            ' ' => result.push_str("%20"),
            '/' => result.push_str("%2F"),
            '#' => result.push_str("%23"),
            '?' => result.push_str("%3F"),
            '&' => result.push_str("%26"),
            '=' => result.push_str("%3D"),
            '+' => result.push_str("%2B"),
            '%' => result.push_str("%25"),
            _ => result.push(c),
        }
    }
    result
}

#[async_trait]
impl SourceAdapter for GcsAdapter {
    fn source_kind(&self) -> &str {
        "gcs"
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        let token = self
            .token_provider
            .get_token()
            .await
            .map_err(|e| SourceError::AuthError {
                source_name: self.source_name.clone(),
                message: e.to_string(),
            })?;

        let objects = self
            .list_objects(&token)
            .await
            .map_err(|e| Self::map_api_error(e, &self.source_name, ""))?;

        debug!(
            source = %self.source_name,
            bucket = %self.bucket,
            prefix = %self.prefix,
            total_objects = objects.len(),
            "GCS listing complete"
        );

        let mut items: Vec<SourceItem> = objects
            .into_iter()
            // Skip "directory" markers (names ending in /).
            .filter(|obj| !obj.name.ends_with('/'))
            // Apply glob pattern filter.
            .filter(|obj| self.pattern.as_ref().map_or(true, |p| p.matches(&obj.name)))
            .map(|obj| {
                let modified_at = obj
                    .updated
                    .as_ref()
                    .and_then(|s| s.parse::<DateTime<Utc>>().ok());

                let mime_type = obj
                    .content_type
                    .unwrap_or_else(|| "application/octet-stream".to_string());

                SourceItem {
                    id: obj.name.clone(),
                    display_name: obj.name.rsplit('/').next().unwrap_or(&obj.name).to_string(),
                    mime_type,
                    path: obj.name,
                    modified_at,
                    source_hash: obj.md5_hash,
                }
            })
            .collect();

        // Sort by name for determinism.
        items.sort_by(|a, b| a.id.cmp(&b.id));

        debug!(
            source = %self.source_name,
            filtered_count = items.len(),
            "GCS enumeration filtered"
        );

        Ok(items)
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let token = self
            .token_provider
            .get_token()
            .await
            .map_err(|e| SourceError::AuthError {
                source_name: self.source_name.clone(),
                message: e.to_string(),
            })?;

        debug!(
            source = %self.source_name,
            object = %item.id,
            "fetching GCS object"
        );

        let content = self
            .download_object(&token, &item.id)
            .await
            .map_err(|e| Self::map_api_error(e, &self.source_name, &item.id))?;

        let content_hash = Blake3Hash::new(blake3::hash(&content).to_hex().to_string());

        let mut prov_metadata = BTreeMap::new();
        prov_metadata.insert(
            "bucket".to_string(),
            serde_json::Value::String(self.bucket.clone()),
        );
        prov_metadata.insert(
            "object_name".to_string(),
            serde_json::Value::String(item.id.clone()),
        );
        if let Some(hash) = &item.source_hash {
            prov_metadata.insert(
                "md5_hash".to_string(),
                serde_json::Value::String(hash.clone()),
            );
        }

        let provenance = ItemProvenance {
            source_kind: "gcs".to_string(),
            metadata: prov_metadata,
            source_modified: item.modified_at,
            extracted_at: Utc::now(),
        };

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: item.mime_type.clone(),
            provenance,
            content_hash,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::CredentialRef;
    use ecl_pipeline_spec::source::GcsSourceSpec;

    fn make_gcs_spec() -> GcsSourceSpec {
        GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: "staging/".to_string(),
            pattern: Some("*.csv".to_string()),
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        }
    }

    #[test]
    fn test_gcs_adapter_source_kind() {
        let spec = make_gcs_spec();
        let adapter = GcsAdapter::from_gcs_spec("gcs-source", &spec).unwrap();
        assert_eq!(adapter.source_kind(), "gcs");
    }

    #[test]
    fn test_gcs_adapter_from_spec_wrong_variant() {
        let fs_spec = SourceSpec::Filesystem(ecl_pipeline_spec::source::FilesystemSourceSpec {
            root: std::path::PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
            stream: None,
        });
        let result = GcsAdapter::from_spec("test", &fs_spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_gcs_adapter_from_spec_gcs_variant() {
        let spec = SourceSpec::Gcs(make_gcs_spec());
        let adapter = GcsAdapter::from_spec("gcs-source", &spec).unwrap();
        assert_eq!(adapter.bucket, "test-bucket");
        assert_eq!(adapter.prefix, "staging/");
        assert!(adapter.pattern.is_some());
    }

    #[test]
    fn test_gcs_adapter_from_spec_no_pattern() {
        let mut spec = make_gcs_spec();
        spec.pattern = None;
        let adapter = GcsAdapter::from_gcs_spec("gcs-source", &spec).unwrap();
        assert!(adapter.pattern.is_none());
    }

    #[test]
    fn test_gcs_adapter_from_spec_invalid_pattern() {
        let mut spec = make_gcs_spec();
        spec.pattern = Some("[invalid".to_string());
        let result = GcsAdapter::from_gcs_spec("gcs-source", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_urlencoded_basic() {
        assert_eq!(urlencoded("staging/file.csv"), "staging%2Ffile.csv");
        assert_eq!(urlencoded("hello world"), "hello%20world");
        assert_eq!(urlencoded("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn test_urlencoded_no_encoding_needed() {
        assert_eq!(urlencoded("simple.txt"), "simple.txt");
    }

    #[tokio::test]
    async fn test_gcs_adapter_enumerate_with_wiremock() {
        let mock_server = wiremock::MockServer::start().await;

        // Mock the objects listing endpoint.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/b/test-bucket/o"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "items": [
                        {
                            "name": "staging/transactions-001.csv",
                            "bucket": "test-bucket",
                            "size": "2048",
                            "contentType": "text/csv",
                            "updated": "2026-03-15T10:00:00Z",
                            "md5Hash": "abc123=="
                        },
                        {
                            "name": "staging/transactions-002.csv",
                            "bucket": "test-bucket",
                            "size": "4096",
                            "contentType": "text/csv",
                            "updated": "2026-03-15T11:00:00Z"
                        },
                        {
                            "name": "staging/readme.txt",
                            "bucket": "test-bucket",
                            "contentType": "text/plain"
                        },
                        {
                            "name": "staging/subdir/",
                            "bucket": "test-bucket"
                        }
                    ]
                })),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: "staging/".to_string(),
            pattern: Some("staging/*.csv".to_string()),
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let items = adapter.enumerate().await.unwrap();

        // Should include 2 CSV files, exclude readme.txt and the directory marker.
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "staging/transactions-001.csv");
        assert_eq!(items[0].display_name, "transactions-001.csv");
        assert_eq!(items[0].mime_type, "text/csv");
        assert_eq!(items[0].source_hash, Some("abc123==".to_string()));
        assert_eq!(items[1].id, "staging/transactions-002.csv");
    }

    #[tokio::test]
    async fn test_gcs_adapter_enumerate_no_pattern_returns_all() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/b/test-bucket/o"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "items": [
                        {"name": "a.csv", "bucket": "test-bucket", "contentType": "text/csv"},
                        {"name": "b.txt", "bucket": "test-bucket", "contentType": "text/plain"}
                    ]
                })),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: String::new(),
            pattern: None,
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 2);
    }

    #[tokio::test]
    async fn test_gcs_adapter_fetch_computes_hash() {
        let mock_server = wiremock::MockServer::start().await;

        let object_content = b"name,amount\nAlice,100\nBob,200\n";

        // Mock the download endpoint.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/b/test-bucket/o/staging%2Fdata.csv",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_bytes(object_content.to_vec()),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: String::new(),
            pattern: None,
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let source_item = SourceItem {
            id: "staging/data.csv".to_string(),
            display_name: "data.csv".to_string(),
            mime_type: "text/csv".to_string(),
            path: "staging/data.csv".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&source_item).await.unwrap();

        assert_eq!(doc.id, "staging/data.csv");
        assert_eq!(doc.display_name, "data.csv");
        assert_eq!(doc.content, object_content);
        assert_eq!(doc.mime_type, "text/csv");
        assert_eq!(doc.provenance.source_kind, "gcs");

        // Verify blake3 hash is correct.
        let expected_hash = blake3::hash(object_content).to_hex().to_string();
        assert_eq!(doc.content_hash.as_str(), expected_hash);
    }

    #[tokio::test]
    async fn test_gcs_adapter_fetch_404_returns_not_found() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path(
                "/b/test-bucket/o/missing%2Ffile.csv",
            ))
            .respond_with(
                wiremock::ResponseTemplate::new(404)
                    .set_body_string(r#"{"error": {"code": 404, "message": "Not Found"}}"#),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: String::new(),
            pattern: None,
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let source_item = SourceItem {
            id: "missing/file.csv".to_string(),
            display_name: "file.csv".to_string(),
            mime_type: "text/csv".to_string(),
            path: "missing/file.csv".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&source_item).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceError::NotFound { .. }));
    }

    #[tokio::test]
    async fn test_gcs_adapter_enumerate_auth_error() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/b/test-bucket/o"))
            .respond_with(
                wiremock::ResponseTemplate::new(401)
                    .set_body_string(r#"{"error": {"code": 401, "message": "Unauthorized"}}"#),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: String::new(),
            pattern: None,
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("bad-token".to_string()));

        let result = adapter.enumerate().await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceError::AuthError { .. }));
    }

    #[test]
    fn test_gcs_source_spec_serde_roundtrip() {
        let spec = SourceSpec::Gcs(GcsSourceSpec {
            bucket: "my-bucket".to_string(),
            prefix: "staging/".to_string(),
            pattern: Some("*.csv".to_string()),
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        });
        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: SourceSpec = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[tokio::test]
    async fn test_gcs_adapter_enumerate_pagination() {
        let mock_server = wiremock::MockServer::start().await;

        // Page 1 returns a next page token.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/b/test-bucket/o"))
            .and(wiremock::matchers::query_param_is_missing("pageToken"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "items": [
                        {"name": "a.csv", "bucket": "test-bucket", "contentType": "text/csv"}
                    ],
                    "nextPageToken": "page2"
                })),
            )
            .mount(&mock_server)
            .await;

        // Page 2 has no next page token.
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/b/test-bucket/o"))
            .and(wiremock::matchers::query_param("pageToken", "page2"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "items": [
                        {"name": "b.csv", "bucket": "test-bucket", "contentType": "text/csv"}
                    ]
                })),
            )
            .mount(&mock_server)
            .await;

        let spec = GcsSourceSpec {
            bucket: "test-bucket".to_string(),
            prefix: String::new(),
            pattern: None,
            credentials: CredentialRef::EnvVar {
                env: "GCS_TOKEN".to_string(),
            },
            stream: None,
        };

        let adapter = GcsAdapter::from_gcs_spec("gcs-test", &spec)
            .unwrap()
            .with_base_url(mock_server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "a.csv");
        assert_eq!(items[1].id, "b.csv");
    }
}
