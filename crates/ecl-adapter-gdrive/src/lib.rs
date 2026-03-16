//! Google Drive source adapter for the ECL pipeline runner.
//!
//! Provides `GoogleDriveAdapter`, which implements `SourceAdapter` by
//! authenticating with the Drive API, recursively traversing folders,
//! and enumerating files with filtering by type, glob, and modified date.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod auth;
pub mod error;
pub mod types;

pub use error::DriveAdapterError;

use std::collections::{BTreeMap, VecDeque};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use glob::Pattern;
use tracing::debug;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::source::{FileTypeFilter, FilterAction, FilterRule, GoogleDriveSourceSpec};
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::{ResolveError, SourceError};
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

use crate::auth::TokenProvider;
use crate::types::{
    DRIVE_API_BASE_URL, DriveFile, FileListResponse, MIME_DOCUMENT, MIME_PRESENTATION,
    MIME_SPREADSHEET,
};

/// Google Drive source adapter.
///
/// Authenticates with the Drive API and recursively enumerates files
/// in configured root folders. Applies file type, glob, and modified-after
/// filters during enumeration.
#[derive(Debug)]
pub struct GoogleDriveAdapter {
    /// Source name (for error reporting and provenance).
    source_name: String,
    /// Source configuration.
    spec: GoogleDriveSourceSpec,
    /// HTTP client for Drive API calls.
    http_client: reqwest::Client,
    /// Token provider for authentication.
    token_provider: TokenProvider,
    /// Compiled glob filter patterns.
    filters: Vec<CompiledFilter>,
    /// File type filters.
    file_type_filters: Vec<FileTypeFilter>,
    /// Parsed modified_after threshold.
    modified_after: Option<DateTime<Utc>>,
    /// Drive API base URL (overridable for testing).
    base_url: String,
}

/// A compiled filter rule with a pre-parsed glob pattern.
#[derive(Debug)]
struct CompiledFilter {
    pattern: Pattern,
    action: FilterAction,
}

/// Fields for the Drive Files.list API response.
const FILES_LIST_FIELDS: &str =
    "nextPageToken,files(id,name,mimeType,modifiedTime,md5Checksum,parents,size)";

impl GoogleDriveAdapter {
    /// Create a new adapter from a `SourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if the spec is not a Google Drive source.
    /// Returns `ResolveError::Io` if a glob pattern is invalid.
    pub fn from_spec(source_name: &str, spec: &SourceSpec) -> Result<Self, ResolveError> {
        let gdrive_spec = match spec {
            SourceSpec::GoogleDrive(gs) => gs,
            _ => {
                return Err(ResolveError::UnknownAdapter {
                    stage: source_name.to_string(),
                    adapter: "google_drive".to_string(),
                });
            }
        };

        Self::from_gdrive_spec(source_name, gdrive_spec)
    }

    /// Create a new adapter directly from a `GoogleDriveSourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::Io` if a glob pattern is invalid.
    pub fn from_gdrive_spec(
        source_name: &str,
        spec: &GoogleDriveSourceSpec,
    ) -> Result<Self, ResolveError> {
        let filters = compile_filters(&spec.filters)?;
        let http_client = reqwest::Client::new();
        let token_provider = TokenProvider::new(spec.credentials.clone(), http_client.clone());

        let modified_after = spec.modified_after.as_ref().and_then(|s| {
            if s == "last_run" {
                // "last_run" requires state store integration — not supported yet.
                None
            } else {
                s.parse::<DateTime<Utc>>().ok()
            }
        });

        Ok(Self {
            source_name: source_name.to_string(),
            spec: spec.clone(),
            http_client,
            token_provider,
            filters,
            file_type_filters: spec.file_types.clone(),
            modified_after,
            base_url: DRIVE_API_BASE_URL.to_string(),
        })
    }

    /// Override the Drive API base URL (for testing with wiremock).
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Override the token provider (for testing).
    pub fn with_token_provider(mut self, provider: TokenProvider) -> Self {
        self.token_provider = provider;
        self
    }

    /// Recursively enumerate all files under the configured root folders.
    async fn enumerate_recursive(&self, token: &str) -> Result<Vec<SourceItem>, SourceError> {
        let mut all_items = Vec::new();
        // Queue of (folder_id, path_prefix) to traverse.
        let mut queue: VecDeque<(String, String)> = VecDeque::new();

        for folder_id in &self.spec.root_folders {
            queue.push_back((folder_id.clone(), String::new()));
        }

        while let Some((folder_id, prefix)) = queue.pop_front() {
            let files = self.list_folder(token, &folder_id).await?;

            for file in files {
                let path = if prefix.is_empty() {
                    file.name.clone()
                } else {
                    format!("{prefix}/{}", file.name)
                };

                if file.is_folder() {
                    debug!(folder_id = %file.id, path = %path, "descending into folder");
                    queue.push_back((file.id.clone(), path));
                    continue;
                }

                if !self.should_include(&file, &path) {
                    debug!(file_id = %file.id, path = %path, "skipped by filter");
                    continue;
                }

                let modified_at = file
                    .modified_time
                    .as_ref()
                    .and_then(|t| t.parse::<DateTime<Utc>>().ok());

                all_items.push(SourceItem {
                    id: file.id.clone(),
                    display_name: file.name.clone(),
                    mime_type: file.mime_type.clone(),
                    path,
                    modified_at,
                    source_hash: file.md5_checksum.clone(),
                });
            }
        }

        // Sort by ID for deterministic ordering.
        all_items.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(all_items)
    }

    /// List all files in a single folder, handling pagination.
    async fn list_folder(
        &self,
        token: &str,
        folder_id: &str,
    ) -> Result<Vec<DriveFile>, SourceError> {
        let mut all_files = Vec::new();
        let mut page_token: Option<String> = None;
        let query = format!("'{folder_id}' in parents and trashed = false");

        loop {
            let mut request = self
                .http_client
                .get(format!("{}/drive/v3/files", self.base_url))
                .bearer_auth(token)
                .query(&[
                    ("q", query.as_str()),
                    ("fields", FILES_LIST_FIELDS),
                    ("pageSize", "1000"),
                ]);

            if let Some(pt) = &page_token {
                request = request.query(&[("pageToken", pt.as_str())]);
            }

            let response = request.send().await.map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("Drive API request failed: {e}"),
            })?;

            let status = response.status();

            if status.as_u16() == 401 {
                return Err(SourceError::AuthError {
                    source_name: self.source_name.clone(),
                    message: "Drive API authentication failed".to_string(),
                });
            }

            if status.as_u16() == 429 {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);
                return Err(SourceError::RateLimited {
                    source_name: self.source_name.clone(),
                    retry_after_secs: retry_after,
                });
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(SourceError::Permanent {
                    source_name: self.source_name.clone(),
                    message: format!("Drive API error ({status}): {body}"),
                });
            }

            let file_list: FileListResponse =
                response.json().await.map_err(|e| SourceError::Permanent {
                    source_name: self.source_name.clone(),
                    message: format!("failed to parse Drive API response: {e}"),
                })?;

            all_files.extend(file_list.files);

            match file_list.next_page_token {
                Some(token) => page_token = Some(token),
                None => break,
            }
        }

        Ok(all_files)
    }

    /// Determine whether a file should be included based on filters.
    fn should_include(&self, file: &DriveFile, path: &str) -> bool {
        // File type filter.
        if !self.file_type_filters.is_empty() {
            let matches_any = self.file_type_filters.iter().any(|f| {
                let ext_match = f.extension.as_ref().is_some_and(|ext| {
                    path.rsplit('.')
                        .next()
                        .is_some_and(|file_ext| file_ext.eq_ignore_ascii_case(ext))
                });
                let mime_match = f.mime.as_ref().is_some_and(|mime| file.mime_type == *mime);
                ext_match || mime_match
            });
            if !matches_any {
                return false;
            }
        }

        // Glob filter rules (last-rule-wins).
        let mut included = true;
        for filter in &self.filters {
            if filter.pattern.matches(path) {
                included = filter.action == FilterAction::Include;
            }
        }
        if !included {
            return false;
        }

        // Modified-after filter.
        if let Some(threshold) = &self.modified_after
            && let Some(modified_str) = &file.modified_time
            && let Ok(modified) = modified_str.parse::<DateTime<Utc>>()
            && modified < *threshold
        {
            return false;
        }

        true
    }

    /// Download file content from Drive.
    ///
    /// Google Workspace documents (Docs, Sheets, Slides) are exported via
    /// the export API. Regular files are downloaded via `Files.get?alt=media`.
    async fn download_content(
        &self,
        token: &str,
        item: &SourceItem,
    ) -> Result<Vec<u8>, SourceError> {
        let (url, is_export) = match item.mime_type.as_str() {
            MIME_DOCUMENT => (
                format!("{}/drive/v3/files/{}/export", self.base_url, item.id),
                true,
            ),
            MIME_SPREADSHEET => (
                format!("{}/drive/v3/files/{}/export", self.base_url, item.id),
                true,
            ),
            MIME_PRESENTATION => (
                format!("{}/drive/v3/files/{}/export", self.base_url, item.id),
                true,
            ),
            _ => (
                format!("{}/drive/v3/files/{}", self.base_url, item.id),
                false,
            ),
        };

        let mut request = self.http_client.get(&url).bearer_auth(token);

        if is_export {
            let export_mime = export_mime_type(&item.mime_type);
            request = request.query(&[("mimeType", export_mime)]);
        } else {
            request = request.query(&[("alt", "media")]);
        }

        let response = request.send().await.map_err(|e| SourceError::Transient {
            source_name: self.source_name.clone(),
            message: format!("Drive download failed: {e}"),
        })?;

        let status = response.status();

        if status.as_u16() == 401 {
            return Err(SourceError::AuthError {
                source_name: self.source_name.clone(),
                message: "Drive API authentication failed during fetch".to_string(),
            });
        }

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(SourceError::RateLimited {
                source_name: self.source_name.clone(),
                retry_after_secs: retry_after,
            });
        }

        if status.as_u16() == 404 {
            return Err(SourceError::NotFound {
                source_name: self.source_name.clone(),
                item_id: item.id.clone(),
            });
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(SourceError::Permanent {
                source_name: self.source_name.clone(),
                message: format!("Drive download error ({status}): {body}"),
            });
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| SourceError::Transient {
                source_name: self.source_name.clone(),
                message: format!("failed to read response body: {e}"),
            })
    }
}

/// Determine the export MIME type for a Google Workspace document.
///
/// Google Docs → text/markdown, Sheets → text/csv, Slides → text/plain.
fn export_mime_type(source_mime: &str) -> &'static str {
    match source_mime {
        MIME_DOCUMENT => "text/markdown",
        MIME_SPREADSHEET => "text/csv",
        MIME_PRESENTATION => "text/plain",
        _ => "text/plain",
    }
}

/// Compile filter rules into glob patterns.
fn compile_filters(rules: &[FilterRule]) -> Result<Vec<CompiledFilter>, ResolveError> {
    rules
        .iter()
        .map(|rule| {
            let pattern = Pattern::new(&rule.pattern).map_err(|e| {
                ResolveError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("invalid glob pattern '{}': {e}", rule.pattern),
                ))
            })?;
            Ok(CompiledFilter {
                pattern,
                action: rule.action.clone(),
            })
        })
        .collect()
}

#[async_trait]
impl SourceAdapter for GoogleDriveAdapter {
    fn source_kind(&self) -> &str {
        "google_drive"
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

        self.enumerate_recursive(&token).await
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

        let content = self.download_content(&token, item).await?;

        let content_hash = Blake3Hash::new(blake3::hash(&content).to_hex().as_str());

        let mut prov_metadata = BTreeMap::new();
        prov_metadata.insert(
            "file_id".to_string(),
            serde_json::Value::String(item.id.clone()),
        );
        prov_metadata.insert(
            "path".to_string(),
            serde_json::Value::String(item.path.clone()),
        );
        if let Some(hash) = &item.source_hash {
            prov_metadata.insert(
                "md5_checksum".to_string(),
                serde_json::Value::String(hash.clone()),
            );
        }

        let provenance = ItemProvenance {
            source_kind: "google_drive".to_string(),
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
    use ecl_pipeline_spec::source::CredentialRef;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_test_spec() -> GoogleDriveSourceSpec {
        GoogleDriveSourceSpec {
            credentials: CredentialRef::EnvVar {
                env: "TEST_TOKEN".to_string(),
            },
            root_folders: vec!["root-folder-id".to_string()],
            filters: vec![],
            file_types: vec![],
            modified_after: None,
        }
    }

    fn make_test_adapter(base_url: &str) -> GoogleDriveAdapter {
        let spec = make_test_spec();
        GoogleDriveAdapter::from_gdrive_spec("test-drive", &spec)
            .unwrap()
            .with_base_url(base_url.to_string())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()))
    }

    fn make_drive_file(id: &str, name: &str, mime: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "mimeType": mime,
            "modifiedTime": "2026-03-01T10:00:00Z",
            "md5Checksum": format!("md5-{id}"),
            "parents": ["parent1"],
            "size": "1024"
        })
    }

    fn make_folder(id: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": ["parent1"]
        })
    }

    // ── Construction tests ─────────────────────────────────────────

    #[test]
    fn test_from_spec_google_drive_source() {
        let spec = SourceSpec::GoogleDrive(make_test_spec());
        let adapter = GoogleDriveAdapter::from_spec("drive", &spec).unwrap();
        assert_eq!(adapter.source_kind(), "google_drive");
        assert_eq!(adapter.source_name, "drive");
    }

    #[test]
    fn test_from_spec_wrong_kind_returns_error() {
        let spec = SourceSpec::Filesystem(ecl_pipeline_spec::source::FilesystemSourceSpec {
            root: std::path::PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
        });
        let result = GoogleDriveAdapter::from_spec("drive", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_gdrive_spec_with_filters() {
        let mut spec = make_test_spec();
        spec.filters = vec![
            FilterRule {
                pattern: "**/*.pdf".to_string(),
                action: FilterAction::Include,
            },
            FilterRule {
                pattern: "**/Archive/**".to_string(),
                action: FilterAction::Exclude,
            },
        ];
        let adapter = GoogleDriveAdapter::from_gdrive_spec("drive", &spec).unwrap();
        assert_eq!(adapter.filters.len(), 2);
    }

    #[test]
    fn test_invalid_glob_pattern_returns_error() {
        let mut spec = make_test_spec();
        spec.filters = vec![FilterRule {
            pattern: "[invalid".to_string(),
            action: FilterAction::Include,
        }];
        let result = GoogleDriveAdapter::from_gdrive_spec("drive", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_modified_after_parsed() {
        let mut spec = make_test_spec();
        spec.modified_after = Some("2026-01-01T00:00:00Z".to_string());
        let adapter = GoogleDriveAdapter::from_gdrive_spec("drive", &spec).unwrap();
        assert!(adapter.modified_after.is_some());
    }

    #[test]
    fn test_modified_after_last_run_ignored() {
        let mut spec = make_test_spec();
        spec.modified_after = Some("last_run".to_string());
        let adapter = GoogleDriveAdapter::from_gdrive_spec("drive", &spec).unwrap();
        assert!(adapter.modified_after.is_none());
    }

    // ── Filter tests ───────────────────────────────────────────────

    #[test]
    fn test_should_include_no_filters() {
        let adapter = make_test_adapter("http://unused");
        let file = DriveFile {
            id: "f1".to_string(),
            name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: Some("2026-03-01T10:00:00Z".to_string()),
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(adapter.should_include(&file, "doc.pdf"));
    }

    #[test]
    fn test_should_include_file_type_extension_filter() {
        let mut spec = make_test_spec();
        spec.file_types = vec![FileTypeFilter {
            extension: Some("pdf".to_string()),
            mime: None,
        }];
        let adapter = GoogleDriveAdapter::from_gdrive_spec("test", &spec)
            .unwrap()
            .with_base_url("http://unused".to_string());

        let pdf = DriveFile {
            id: "f1".to_string(),
            name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        let txt = DriveFile {
            id: "f2".to_string(),
            name: "notes.txt".to_string(),
            mime_type: "text/plain".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(adapter.should_include(&pdf, "doc.pdf"));
        assert!(!adapter.should_include(&txt, "notes.txt"));
    }

    #[test]
    fn test_should_include_file_type_mime_filter() {
        let mut spec = make_test_spec();
        spec.file_types = vec![FileTypeFilter {
            extension: None,
            mime: Some("application/vnd.google-apps.document".to_string()),
        }];
        let adapter = GoogleDriveAdapter::from_gdrive_spec("test", &spec)
            .unwrap()
            .with_base_url("http://unused".to_string());

        let gdoc = DriveFile {
            id: "f1".to_string(),
            name: "My Doc".to_string(),
            mime_type: "application/vnd.google-apps.document".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        let pdf = DriveFile {
            id: "f2".to_string(),
            name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(adapter.should_include(&gdoc, "My Doc"));
        assert!(!adapter.should_include(&pdf, "doc.pdf"));
    }

    #[test]
    fn test_should_include_glob_filter() {
        let mut spec = make_test_spec();
        spec.filters = vec![FilterRule {
            pattern: "**/Archive/**".to_string(),
            action: FilterAction::Exclude,
        }];
        let adapter = GoogleDriveAdapter::from_gdrive_spec("test", &spec)
            .unwrap()
            .with_base_url("http://unused".to_string());

        let file = DriveFile {
            id: "f1".to_string(),
            name: "old.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(!adapter.should_include(&file, "Archive/old.pdf"));
        assert!(adapter.should_include(&file, "docs/new.pdf"));
    }

    #[test]
    fn test_should_include_modified_after_filter() {
        let mut spec = make_test_spec();
        spec.modified_after = Some("2026-02-01T00:00:00Z".to_string());
        let adapter = GoogleDriveAdapter::from_gdrive_spec("test", &spec)
            .unwrap()
            .with_base_url("http://unused".to_string());

        let recent = DriveFile {
            id: "f1".to_string(),
            name: "new.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: Some("2026-03-01T10:00:00Z".to_string()),
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        let old = DriveFile {
            id: "f2".to_string(),
            name: "old.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: Some("2026-01-15T10:00:00Z".to_string()),
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(adapter.should_include(&recent, "new.pdf"));
        assert!(!adapter.should_include(&old, "old.pdf"));
    }

    // ── Enumerate tests (wiremock) ─────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_single_folder() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "'root-folder-id' in parents and trashed = false",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    make_drive_file("f1", "doc.pdf", "application/pdf"),
                    make_drive_file("f2", "notes.txt", "text/plain"),
                ]
            })))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();

        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, "f1");
        assert_eq!(items[0].display_name, "doc.pdf");
        assert_eq!(items[0].mime_type, "application/pdf");
        assert_eq!(items[0].path, "doc.pdf");
        assert_eq!(items[0].source_hash.as_deref(), Some("md5-f1"));
        assert!(items[0].modified_at.is_some());
    }

    #[tokio::test]
    async fn test_enumerate_empty_folder() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({ "files": [] })),
            )
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_enumerate_pagination() {
        let server = MockServer::start().await;

        // Page 1: returns nextPageToken.
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "'root-folder-id' in parents and trashed = false",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [make_drive_file("f1", "page1.txt", "text/plain")],
                "nextPageToken": "page2token"
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Page 2: no nextPageToken.
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageToken", "page2token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [make_drive_file("f2", "page2.txt", "text/plain")]
            })))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();

        assert_eq!(items.len(), 2);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"f1"));
        assert!(ids.contains(&"f2"));
    }

    #[tokio::test]
    async fn test_enumerate_recursive_folders() {
        let server = MockServer::start().await;

        // Root folder contains a file and a subfolder.
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "'root-folder-id' in parents and trashed = false",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    make_drive_file("f1", "root.txt", "text/plain"),
                    make_folder("subfolder-id", "Docs"),
                ]
            })))
            .mount(&server)
            .await;

        // Subfolder contains a file.
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param(
                "q",
                "'subfolder-id' in parents and trashed = false",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    make_drive_file("f2", "nested.pdf", "application/pdf"),
                ]
            })))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();

        assert_eq!(items.len(), 2);
        let paths: Vec<&str> = items.iter().map(|i| i.path.as_str()).collect();
        assert!(paths.contains(&"root.txt"));
        assert!(paths.contains(&"Docs/nested.pdf"));
    }

    #[tokio::test]
    async fn test_enumerate_applies_file_type_filter() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    make_drive_file("f1", "doc.pdf", "application/pdf"),
                    make_drive_file("f2", "notes.txt", "text/plain"),
                    make_drive_file("f3", "report.pdf", "application/pdf"),
                ]
            })))
            .mount(&server)
            .await;

        let mut spec = make_test_spec();
        spec.file_types = vec![FileTypeFilter {
            extension: Some("pdf".to_string()),
            mime: None,
        }];
        let adapter = GoogleDriveAdapter::from_gdrive_spec("test", &spec)
            .unwrap()
            .with_base_url(server.uri())
            .with_token_provider(TokenProvider::static_token("test-token".to_string()));

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.path.ends_with(".pdf")));
    }

    #[tokio::test]
    async fn test_enumerate_sorted_by_id() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [
                    make_drive_file("c", "c.txt", "text/plain"),
                    make_drive_file("a", "a.txt", "text/plain"),
                    make_drive_file("b", "b.txt", "text/plain"),
                ]
            })))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();

        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }

    #[tokio::test]
    async fn test_enumerate_populates_source_hash() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "files": [make_drive_file("f1", "doc.pdf", "application/pdf")]
            })))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let items = adapter.enumerate().await.unwrap();

        assert_eq!(items[0].source_hash.as_deref(), Some("md5-f1"));
    }

    // ── Error handling tests ───────────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_auth_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let result = adapter.enumerate().await;
        assert!(matches!(result, Err(SourceError::AuthError { .. })));
    }

    #[tokio::test]
    async fn test_enumerate_rate_limited() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "30"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let result = adapter.enumerate().await;
        assert!(matches!(
            result,
            Err(SourceError::RateLimited {
                retry_after_secs: 30,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_enumerate_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let result = adapter.enumerate().await;
        assert!(matches!(result, Err(SourceError::Permanent { .. })));
    }

    // ── Fetch tests (wiremock) ─────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_regular_file() {
        let server = MockServer::start().await;

        // Enumerate mock (needed for the adapter to have items).
        Mock::given(method("GET"))
            .and(path("/drive/v3/files/file-abc"))
            .and(query_param("alt", "media"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(b"Hello, World!" as &[u8])
                    .insert_header("content-type", "application/pdf"),
            )
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "file-abc".to_string(),
            display_name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            path: "doc.pdf".to_string(),
            modified_at: None,
            source_hash: Some("md5-abc".to_string()),
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert_eq!(doc.id, "file-abc");
        assert_eq!(doc.display_name, "doc.pdf");
        assert_eq!(doc.content, b"Hello, World!");
        assert_eq!(doc.mime_type, "application/pdf");
        assert_eq!(doc.provenance.source_kind, "google_drive");
        assert_eq!(
            doc.provenance.metadata.get("file_id"),
            Some(&serde_json::Value::String("file-abc".to_string()))
        );
        assert_eq!(
            doc.provenance.metadata.get("md5_checksum"),
            Some(&serde_json::Value::String("md5-abc".to_string()))
        );
    }

    #[tokio::test]
    async fn test_fetch_computes_blake3_hash() {
        let server = MockServer::start().await;
        let content = b"hash me please";

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(content as &[u8]))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "f1".to_string(),
            display_name: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "test.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        let expected_hash = blake3::hash(content).to_hex().to_string();
        assert_eq!(doc.content_hash.as_str(), expected_hash);
    }

    #[tokio::test]
    async fn test_fetch_google_doc_exports_as_markdown() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/doc-id/export"))
            .and(query_param("mimeType", "text/markdown"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string("# Exported Document\n\nContent here."),
            )
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "doc-id".to_string(),
            display_name: "My Doc".to_string(),
            mime_type: "application/vnd.google-apps.document".to_string(),
            path: "My Doc".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert_eq!(
            std::str::from_utf8(&doc.content).unwrap(),
            "# Exported Document\n\nContent here."
        );
    }

    #[tokio::test]
    async fn test_fetch_google_sheet_exports_as_csv() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/sheet-id/export"))
            .and(query_param("mimeType", "text/csv"))
            .respond_with(ResponseTemplate::new(200).set_body_string("name,value\nfoo,42\n"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "sheet-id".to_string(),
            display_name: "My Sheet".to_string(),
            mime_type: "application/vnd.google-apps.spreadsheet".to_string(),
            path: "My Sheet".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert!(
            std::str::from_utf8(&doc.content)
                .unwrap()
                .contains("foo,42")
        );
    }

    #[tokio::test]
    async fn test_fetch_google_slides_exports_as_text() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/slides-id/export"))
            .and(query_param("mimeType", "text/plain"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Slide 1: Title"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "slides-id".to_string(),
            display_name: "My Slides".to_string(),
            mime_type: "application/vnd.google-apps.presentation".to_string(),
            path: "My Slides".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert!(
            std::str::from_utf8(&doc.content)
                .unwrap()
                .contains("Slide 1")
        );
    }

    #[tokio::test]
    async fn test_fetch_binary_content() {
        let server = MockServer::start().await;
        let binary = vec![0x00, 0x01, 0x02, 0xFF, 0xFE, 0xFD];

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/bin-id"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(binary.clone()))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "bin-id".to_string(),
            display_name: "data.bin".to_string(),
            mime_type: "application/octet-stream".to_string(),
            path: "data.bin".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert_eq!(doc.content, binary);
    }

    #[tokio::test]
    async fn test_fetch_auth_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "f1".to_string(),
            display_name: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "test.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&item).await;
        assert!(matches!(result, Err(SourceError::AuthError { .. })));
    }

    #[tokio::test]
    async fn test_fetch_rate_limited() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "45"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "f1".to_string(),
            display_name: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "test.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&item).await;
        assert!(matches!(
            result,
            Err(SourceError::RateLimited {
                retry_after_secs: 45,
                ..
            })
        ));
    }

    #[tokio::test]
    async fn test_fetch_not_found() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/gone"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "gone".to_string(),
            display_name: "deleted.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "deleted.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&item).await;
        assert!(matches!(result, Err(SourceError::NotFound { .. })));
    }

    #[tokio::test]
    async fn test_fetch_server_error() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "f1".to_string(),
            display_name: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "test.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&item).await;
        assert!(matches!(result, Err(SourceError::Permanent { .. })));
    }

    #[tokio::test]
    async fn test_fetch_provenance_includes_path() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/drive/v3/files/f1"))
            .respond_with(ResponseTemplate::new(200).set_body_string("content"))
            .mount(&server)
            .await;

        let adapter = make_test_adapter(&server.uri());
        let item = SourceItem {
            id: "f1".to_string(),
            display_name: "test.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "Docs/Sub/test.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let doc = adapter.fetch(&item).await.unwrap();
        assert_eq!(
            doc.provenance.metadata.get("path"),
            Some(&serde_json::Value::String("Docs/Sub/test.txt".to_string()))
        );
    }

    // ── Export MIME type mapping tests ──────────────────────────────

    #[test]
    fn test_export_mime_type_document() {
        assert_eq!(
            super::export_mime_type("application/vnd.google-apps.document"),
            "text/markdown"
        );
    }

    #[test]
    fn test_export_mime_type_spreadsheet() {
        assert_eq!(
            super::export_mime_type("application/vnd.google-apps.spreadsheet"),
            "text/csv"
        );
    }

    #[test]
    fn test_export_mime_type_presentation() {
        assert_eq!(
            super::export_mime_type("application/vnd.google-apps.presentation"),
            "text/plain"
        );
    }

    #[test]
    fn test_export_mime_type_unknown_defaults_to_text() {
        assert_eq!(super::export_mime_type("something/unknown"), "text/plain");
    }

    // ── Source kind test ───────────────────────────────────────────

    #[test]
    fn test_source_kind() {
        let adapter = make_test_adapter("http://unused");
        assert_eq!(adapter.source_kind(), "google_drive");
    }
}
