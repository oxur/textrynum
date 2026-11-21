# Milestone 4.1: Google Drive Auth & Enumerate (`ecl-adapter-gdrive`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-adapter-gdrive):`

## 1. Goal

Create the `ecl-adapter-gdrive` crate that implements the `SourceAdapter` trait
for Google Drive. When done:

1. The `GoogleDriveAdapter` struct resolves credentials from any
   `CredentialRef` variant (`File`, `EnvVar`, `ApplicationDefault`) using
   `yup-oauth2` and obtains an OAuth2 access token scoped to
   `https://www.googleapis.com/auth/drive.readonly`.
2. The `enumerate()` method calls the Drive Files.list API, handles pagination,
   recursively traverses subfolders, applies filters (file type, glob pattern,
   modified_after), and returns `Vec<SourceItem>`.
3. The `fetch()` method exists as a stub returning `todo!()` with a comment
   pointing to milestone 4.2.
4. All HTTP interactions use `reqwest` and are fully testable via `wiremock`
   mock servers.
5. Code coverage is 95% or better.

**Important:** This milestone implements `enumerate()` ONLY. The `fetch()`
implementation is milestone 4.2.

## 2. Context

### 2.1 Project Conventions

- **Edition:** 2024, **Rust version:** 1.85
- **Lints** (in every crate's `Cargo.toml`):

  ```toml
  [lints.rust]
  unsafe_code = "forbid"
  missing_docs = "warn"

  [lints.clippy]
  unwrap_used = "deny"
  expect_used = "warn"
  panic = "deny"
  ```

- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, GdriveError>;`
- **Test naming:** `test_<fn>_<scenario>_<expectation>`
- **Tests:** Inline `#[cfg(test)] #[allow(clippy::unwrap_used)] mod tests { ... }`
- **Maps:** Always `BTreeMap` (not `HashMap`) for deterministic serialization
- **Params:** Use `&str` not `&String`, `&[T]` not `&Vec<T>` in function signatures
- **No `unwrap()`** in library code — use `?` or `.ok_or()`
- **Doc comments** on all public items

### 2.2 Rust Guides to Load

Before writing any code, read these files (paths relative to workspace root):

1. `assets/ai/ai-rust/guides/11-anti-patterns.md` (ALWAYS first)
2. `assets/ai/ai-rust/guides/01-core-idioms.md`
3. `assets/ai/ai-rust/guides/07-concurrency-async.md`
4. `assets/ai/ai-rust/guides/03-error-handling.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:

- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments

## 3. Prior Art / Dependencies

### From `ecl-pipeline-spec` (Milestone 1.1)

```rust
// --- src/source.rs ---

/// How to resolve credentials for a source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    /// Credentials from a file path.
    #[serde(rename = "file")]
    File { path: PathBuf },
    /// Credentials from an environment variable.
    #[serde(rename = "env")]
    EnvVar { env: String },
    /// Use application default credentials.
    #[serde(rename = "application_default")]
    ApplicationDefault,
}

/// Google Drive source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveSourceSpec {
    pub credentials: CredentialRef,
    pub root_folders: Vec<String>,
    #[serde(default)]
    pub filters: Vec<FilterRule>,
    #[serde(default)]
    pub file_types: Vec<FileTypeFilter>,
    pub modified_after: Option<String>,
}

/// A filter rule for include/exclude patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    pub pattern: String,
    pub action: FilterAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterAction { Include, Exclude }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeFilter {
    pub extension: Option<String>,
    pub mime: Option<String>,
}
```

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
// --- src/traits.rs ---

/// A source adapter handles all interaction with an external data service.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    /// Human-readable name of the source type (e.g., "Google Drive").
    fn source_kind(&self) -> &str;

    /// Enumerate items available from this source.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by enumerate().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    /// Source-specific unique identifier.
    pub id: String,
    /// Human-readable name.
    pub display_name: String,
    /// MIME type (for filtering by file type).
    pub mime_type: String,
    /// Path within the source (for glob-based filtering).
    pub path: String,
    /// Last modified timestamp (for incremental sync).
    pub modified_at: Option<DateTime<Utc>>,
    /// Content hash if cheaply available from the source API.
    pub source_hash: Option<String>,
}

/// A document extracted from a source, in its original format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,
    pub mime_type: String,
    pub provenance: ItemProvenance,
    pub content_hash: Blake3Hash,
}

// --- src/error.rs ---

/// Errors that occur in source adapters.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    #[error("authentication failed for source '{source}': {message}")]
    AuthError { source: String, message: String },

    #[error("rate limited by '{source}': retry after {retry_after_secs}s")]
    RateLimited { source: String, retry_after_secs: u64 },

    #[error("item '{item_id}' not found in source '{source}'")]
    NotFound { source: String, item_id: String },

    #[error("transient error from source '{source}': {message}")]
    Transient { source: String, message: String },

    #[error("permanent error from source '{source}': {message}")]
    Permanent { source: String, message: String },
}
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `Cargo.toml` (root) | Modify | Add workspace deps `wiremock`, `yup-oauth2`, `glob`; add `crates/ecl-adapter-gdrive` to members |
| `crates/ecl-adapter-gdrive/Cargo.toml` | Create | Crate manifest |
| `crates/ecl-adapter-gdrive/src/lib.rs` | Create | `GoogleDriveAdapter` struct, `SourceAdapter` impl, module declarations, re-exports |
| `crates/ecl-adapter-gdrive/src/auth.rs` | Create | Credential resolution: `CredentialRef` to OAuth2 access token via `yup-oauth2` |
| `crates/ecl-adapter-gdrive/src/enumerate.rs` | Create | Drive Files.list API calls, pagination, recursive folder traversal, filter application |
| `crates/ecl-adapter-gdrive/src/types.rs` | Create | `DriveFileListResponse`, `DriveFile` — serde types for Drive API JSON |
| `crates/ecl-adapter-gdrive/src/filter.rs` | Create | Filter logic: file type matching, glob pattern matching, modified_after comparison |
| `crates/ecl-adapter-gdrive/src/error.rs` | Create | `GdriveError` enum, conversion to `SourceError` |

## 5. Cargo.toml

### Root Workspace Additions

Add to `[workspace.dependencies]` in the root `Cargo.toml`:

```toml
wiremock = "0.6"
yup-oauth2 = "13"
glob = "0.3"
```

Add to `[workspace] members`:

```toml
"crates/ecl-adapter-gdrive",
```

### Crate Cargo.toml

```toml
[package]
name = "ecl-adapter-gdrive"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
description = "Google Drive source adapter for ECL pipeline runner"

[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
reqwest = { version = "0.12", features = ["json"] }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
tracing = { workspace = true }
chrono = { workspace = true }
yup-oauth2 = { workspace = true }
glob = { workspace = true }

[dev-dependencies]
wiremock = { workspace = true }
tempfile = { workspace = true }
tokio = { workspace = true, features = ["test-util"] }

[lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"

[lints.clippy]
unwrap_used = "deny"
expect_used = "warn"
panic = "deny"
```

## 6. Type Definitions and Signatures

All types below must be implemented exactly as shown.

### `src/error.rs`

```rust
//! Error types for the Google Drive adapter.

use ecl_pipeline_topo::SourceError;
use thiserror::Error;

/// Errors specific to the Google Drive adapter.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GdriveError {
    /// Failed to read or parse credentials.
    #[error("credential error: {message}")]
    CredentialError {
        /// Error detail.
        message: String,
    },

    /// OAuth2 token exchange failed.
    #[error("OAuth2 token error: {message}")]
    TokenError {
        /// Error detail.
        message: String,
    },

    /// HTTP request to Drive API failed.
    #[error("Drive API request failed: {message}")]
    ApiError {
        /// Error detail.
        message: String,
        /// HTTP status code, if available.
        status: Option<u16>,
    },

    /// Failed to deserialize a Drive API response.
    #[error("failed to parse Drive API response: {message}")]
    ParseError {
        /// Error detail.
        message: String,
    },

    /// A filter glob pattern is invalid.
    #[error("invalid glob pattern '{pattern}': {message}")]
    InvalidPattern {
        /// The invalid pattern.
        pattern: String,
        /// Error detail.
        message: String,
    },

    /// Environment variable not set or not valid UTF-8.
    #[error("environment variable '{name}' error: {message}")]
    EnvError {
        /// The environment variable name.
        name: String,
        /// Error detail.
        message: String,
    },

    /// I/O error (e.g., reading credentials file).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convert `GdriveError` into the canonical `SourceError` for the pipeline.
impl GdriveError {
    /// Convert this adapter-specific error into a `SourceError`.
    ///
    /// Maps credential/token errors to `AuthError`, rate-limit responses
    /// to `RateLimited`, and other errors to `Transient` or `Permanent`
    /// based on whether a retry could succeed.
    pub fn into_source_error(self, source_name: &str) -> SourceError {
        match &self {
            GdriveError::CredentialError { .. }
            | GdriveError::TokenError { .. }
            | GdriveError::EnvError { .. } => SourceError::AuthError {
                source: source_name.to_string(),
                message: self.to_string(),
            },
            GdriveError::ApiError { status, .. } => match status {
                Some(401) | Some(403) => SourceError::AuthError {
                    source: source_name.to_string(),
                    message: self.to_string(),
                },
                Some(404) => SourceError::NotFound {
                    source: source_name.to_string(),
                    item_id: String::new(),
                },
                Some(429) => SourceError::RateLimited {
                    source: source_name.to_string(),
                    retry_after_secs: 60,
                },
                Some(500..=599) => SourceError::Transient {
                    source: source_name.to_string(),
                    message: self.to_string(),
                },
                _ => SourceError::Permanent {
                    source: source_name.to_string(),
                    message: self.to_string(),
                },
            },
            GdriveError::Io(_) => SourceError::Transient {
                source: source_name.to_string(),
                message: self.to_string(),
            },
            GdriveError::ParseError { .. } | GdriveError::InvalidPattern { .. } => {
                SourceError::Permanent {
                    source: source_name.to_string(),
                    message: self.to_string(),
                }
            }
        }
    }
}

/// Result type for Google Drive adapter operations.
pub type Result<T> = std::result::Result<T, GdriveError>;
```

### `src/types.rs`

```rust
//! Deserialization types for the Google Drive Files.list API response.

use serde::Deserialize;

/// Response from the Drive Files.list endpoint.
///
/// See: <https://developers.google.com/drive/api/reference/rest/v3/files/list>
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFileListResponse {
    /// The list of files in this page of results.
    #[serde(default)]
    pub files: Vec<DriveFile>,

    /// Token for the next page of results, if any.
    pub next_page_token: Option<String>,
}

/// A single file or folder from the Drive API.
///
/// See: <https://developers.google.com/drive/api/reference/rest/v3/files>
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFile {
    /// The unique file ID.
    pub id: String,

    /// The file name.
    pub name: String,

    /// The MIME type of the file.
    pub mime_type: String,

    /// The last time the file was modified (RFC 3339 timestamp).
    pub modified_time: Option<String>,

    /// MD5 checksum of the file content (not present for Google Docs types).
    pub md5_checksum: Option<String>,

    /// Parent folder ID(s).
    #[serde(default)]
    pub parents: Vec<String>,
}

impl DriveFile {
    /// The MIME type for Google Drive folders.
    pub const FOLDER_MIME_TYPE: &'static str = "application/vnd.google-apps.folder";

    /// Returns `true` if this file is a folder.
    pub fn is_folder(&self) -> bool {
        self.mime_type == Self::FOLDER_MIME_TYPE
    }
}
```

### `src/auth.rs`

```rust
//! OAuth2 credential resolution for Google Drive.
//!
//! Resolves a `CredentialRef` (from the pipeline spec) into an OAuth2
//! access token using `yup-oauth2`. Token refresh is handled automatically
//! by the `yup-oauth2` authenticator.

use std::path::Path;

use ecl_pipeline_spec::CredentialRef;
use yup_oauth2::authenticator::Authenticator;
use yup_oauth2::hyper_rustls::HttpsConnector;
use yup_oauth2::ServiceAccountAuthenticator;

use crate::error::{GdriveError, Result};

/// The OAuth2 scope required for read-only Drive access.
const DRIVE_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

/// Resolve a `CredentialRef` into a `yup-oauth2` authenticator.
///
/// The returned authenticator handles token caching and automatic refresh.
/// The caller should hold onto it for the lifetime of the adapter.
pub async fn resolve_authenticator(
    credential: &CredentialRef,
) -> Result<Authenticator<HttpsConnector<hyper_util::client::legacy::Client<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, http_body_util::Full<bytes::Bytes>>>>> {
    let service_account_key = match credential {
        CredentialRef::File { path } => read_key_from_file(path).await?,
        CredentialRef::EnvVar { env } => read_key_from_env(env)?,
        CredentialRef::ApplicationDefault => {
            return build_adc_authenticator().await;
        }
    };

    ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await
        .map_err(|e| GdriveError::TokenError {
            message: e.to_string(),
        })
}

/// Obtain an access token string from an authenticator.
///
/// Requests the `drive.readonly` scope. `yup-oauth2` caches and refreshes
/// tokens automatically, so this is cheap to call repeatedly.
pub async fn get_access_token(
    authenticator: &Authenticator<HttpsConnector<hyper_util::client::legacy::Client<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, http_body_util::Full<bytes::Bytes>>>>,
) -> Result<String> {
    let token = authenticator
        .token(&[DRIVE_READONLY_SCOPE])
        .await
        .map_err(|e| GdriveError::TokenError {
            message: e.to_string(),
        })?;

    token
        .token()
        .map(|s| s.to_string())
        .ok_or_else(|| GdriveError::TokenError {
            message: "authenticator returned empty token".to_string(),
        })
}

/// Read a service account key from a JSON file on disk.
async fn read_key_from_file(path: &Path) -> Result<yup_oauth2::ServiceAccountKey> {
    let contents = tokio::fs::read_to_string(path).await.map_err(|e| {
        GdriveError::CredentialError {
            message: format!("failed to read credentials file '{}': {}", path.display(), e),
        }
    })?;
    parse_service_account_key(&contents)
}

/// Read a service account key JSON from an environment variable.
fn read_key_from_env(env_var: &str) -> Result<yup_oauth2::ServiceAccountKey> {
    let contents = std::env::var(env_var).map_err(|e| GdriveError::EnvError {
        name: env_var.to_string(),
        message: e.to_string(),
    })?;
    parse_service_account_key(&contents)
}

/// Parse a service account key from a JSON string.
fn parse_service_account_key(json: &str) -> Result<yup_oauth2::ServiceAccountKey> {
    serde_json::from_str(json).map_err(|e| GdriveError::CredentialError {
        message: format!("invalid service account JSON: {e}"),
    })
}

/// Build an authenticator using Application Default Credentials (ADC).
///
/// ADC checks, in order:
/// 1. `GOOGLE_APPLICATION_CREDENTIALS` env var pointing to a key file
/// 2. Well-known file location (`~/.config/gcloud/application_default_credentials.json`)
/// 3. GCE metadata server (when running on Google Cloud)
async fn build_adc_authenticator() -> Result<Authenticator<HttpsConnector<hyper_util::client::legacy::Client<HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>, http_body_util::Full<bytes::Bytes>>>>> {
    yup_oauth2::ApplicationDefaultCredentialsAuthenticator::builder(
        yup_oauth2::ApplicationDefaultCredentialsFlowOpts::default(),
    )
    .build()
    .await
    .map_err(|e| GdriveError::CredentialError {
        message: format!("failed to initialize ADC: {e}"),
    })
}
```

**Important note on the authenticator type:** The concrete type of
`Authenticator<C>` from `yup-oauth2` has a deeply nested generic connector
parameter. In practice, you will likely need to define a type alias in
`auth.rs` to keep signatures manageable:

```rust
/// Type alias for the yup-oauth2 authenticator with the default HTTPS connector.
pub type DriveAuthenticator = Authenticator<HttpsConnector</* ... */>>;
```

Consult the `yup-oauth2` v13 docs for the exact connector type. If the
generic is unwieldy, consider using `Box<dyn yup_oauth2::GetToken>` instead,
which `yup-oauth2` supports and simplifies the signature at negligible cost.
This is the recommended approach for this crate:

```rust
/// Trait-object authenticator — simplifies generic signatures.
pub type DriveAuthenticator = Box<dyn yup_oauth2::GetToken>;
```

### `src/filter.rs`

```rust
//! Filter logic for Google Drive enumeration results.
//!
//! Applies file type filters, glob patterns, and modified_after constraints
//! to a list of `DriveFile` entries before converting them to `SourceItem`.

use chrono::{DateTime, Utc};
use glob::Pattern;
use ecl_pipeline_spec::{FileTypeFilter, FilterAction, FilterRule};

use crate::error::{GdriveError, Result};
use crate::types::DriveFile;

/// Compiled filters, ready to evaluate against files.
///
/// Pre-compiles glob patterns once at adapter construction time so that
/// enumeration does not pay the compilation cost per file.
#[derive(Debug)]
pub struct CompiledFilters {
    /// Compiled glob rules with their include/exclude action.
    glob_rules: Vec<(Pattern, FilterAction)>,
    /// File type filters (extension and/or MIME).
    file_types: Vec<FileTypeFilter>,
    /// Minimum modification timestamp (files older than this are excluded).
    modified_after: Option<DateTime<Utc>>,
}

impl CompiledFilters {
    /// Compile filters from the source specification.
    ///
    /// Returns an error if any glob pattern is invalid.
    pub fn compile(
        filter_rules: &[FilterRule],
        file_types: &[FileTypeFilter],
        modified_after: Option<&str>,
    ) -> Result<Self> {
        let mut glob_rules = Vec::with_capacity(filter_rules.len());
        for rule in filter_rules {
            let pattern = Pattern::new(&rule.pattern).map_err(|e| {
                GdriveError::InvalidPattern {
                    pattern: rule.pattern.clone(),
                    message: e.to_string(),
                }
            })?;
            glob_rules.push((pattern, rule.action.clone()));
        }

        let modified_after = modified_after
            .filter(|s| *s != "last_run") // "last_run" is resolved by the runner, not here
            .map(|s| {
                s.parse::<DateTime<Utc>>().map_err(|e| GdriveError::ParseError {
                    message: format!("invalid modified_after timestamp '{s}': {e}"),
                })
            })
            .transpose()?;

        Ok(Self {
            glob_rules,
            file_types: file_types.to_vec(),
            modified_after,
        })
    }

    /// Returns `true` if the file passes all active filters.
    ///
    /// Evaluation order:
    /// 1. `modified_after` — reject files older than the threshold
    /// 2. `file_types` — if non-empty, file must match at least one
    /// 3. `glob_rules` — evaluated in order; first matching rule wins
    ///    (if no rule matches, the file is included by default)
    pub fn matches(&self, file: &DriveFile, path: &str) -> bool {
        // 1. modified_after filter
        if let Some(threshold) = &self.modified_after {
            if let Some(ref modified_str) = file.modified_time {
                if let Ok(modified) = modified_str.parse::<DateTime<Utc>>() {
                    if modified < *threshold {
                        return false;
                    }
                }
            }
        }

        // 2. file_types filter (skip for folders — they are traversed, not filtered)
        if !self.file_types.is_empty() && !file.is_folder() {
            let type_match = self.file_types.iter().any(|ft| {
                let ext_match = ft.extension.as_ref().map_or(true, |ext| {
                    file.name
                        .rsplit('.')
                        .next()
                        .map_or(false, |file_ext| file_ext.eq_ignore_ascii_case(ext))
                });
                let mime_match = ft
                    .mime
                    .as_ref()
                    .map_or(true, |mime| file.mime_type == *mime);
                ext_match && mime_match
            });
            if !type_match {
                return false;
            }
        }

        // 3. glob pattern rules (first match wins)
        for (pattern, action) in &self.glob_rules {
            if pattern.matches(path) {
                return matches!(action, FilterAction::Include);
            }
        }

        // No glob rule matched — default to include
        true
    }
}
```

### `src/enumerate.rs`

```rust
//! Google Drive file enumeration via the Files.list API.
//!
//! Handles pagination, recursive folder traversal, and filter application.
//! Produces `Vec<SourceItem>` for the pipeline runner.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use reqwest::Client;
use tracing::{debug, instrument, warn};

use ecl_pipeline_topo::SourceItem;

use crate::error::{GdriveError, Result};
use crate::filter::CompiledFilters;
use crate::types::{DriveFile, DriveFileListResponse};

/// The Drive API v3 Files.list endpoint.
const FILES_LIST_URL: &str = "https://www.googleapis.com/drive/v3/files";

/// Fields to request from the Drive API (minimizes response size).
const FILE_FIELDS: &str = "nextPageToken,files(id,name,mimeType,modifiedTime,md5Checksum,parents)";

/// Maximum number of results per page (Drive API maximum is 1000).
const PAGE_SIZE: u32 = 1000;

/// Enumerate all files matching the source configuration.
///
/// For each root folder ID:
/// 1. List files in the folder (paginating as needed)
/// 2. For each subfolder found, recurse
/// 3. Apply filters to non-folder files
/// 4. Convert passing files to `SourceItem`
///
/// `path_prefix` in the returned `SourceItem.path` is constructed from the
/// folder hierarchy, enabling glob-based filtering.
#[instrument(skip(client, access_token, filters))]
pub async fn enumerate_files(
    client: &Client,
    access_token: &str,
    root_folder_ids: &[String],
    filters: &CompiledFilters,
) -> Result<Vec<SourceItem>> {
    let mut items = Vec::new();

    for folder_id in root_folder_ids {
        enumerate_folder(
            client,
            access_token,
            folder_id,
            "",  // root has empty path prefix
            filters,
            &mut items,
        )
        .await?;
    }

    Ok(items)
}

/// Recursively enumerate files in a single folder.
///
/// `path_prefix` is the path constructed from parent folders, e.g.,
/// `"Engineering/Design"`. Files in this folder get path
/// `"{path_prefix}/{file.name}"`.
async fn enumerate_folder(
    client: &Client,
    access_token: &str,
    folder_id: &str,
    path_prefix: &str,
    filters: &CompiledFilters,
    items: &mut Vec<SourceItem>,
) -> Result<()> {
    let mut page_token: Option<String> = None;

    loop {
        let response = list_files_page(
            client,
            access_token,
            folder_id,
            page_token.as_deref(),
        )
        .await?;

        for file in &response.files {
            let file_path = if path_prefix.is_empty() {
                file.name.clone()
            } else {
                format!("{path_prefix}/{}", file.name)
            };

            if file.is_folder() {
                // Recurse into subfolders
                enumerate_folder(
                    client,
                    access_token,
                    &file.id,
                    &file_path,
                    filters,
                    items,
                )
                .await?;
            } else if filters.matches(file, &file_path) {
                items.push(drive_file_to_source_item(file, &file_path));
            }
        }

        // Pagination: continue if there is a next page
        match response.next_page_token {
            Some(token) if !token.is_empty() => {
                page_token = Some(token);
            }
            _ => break,
        }
    }

    Ok(())
}

/// Call the Drive Files.list API for a single page of results.
async fn list_files_page(
    client: &Client,
    access_token: &str,
    folder_id: &str,
    page_token: Option<&str>,
) -> Result<DriveFileListResponse> {
    let query = format!("'{folder_id}' in parents and trashed = false");

    let mut request = client
        .get(FILES_LIST_URL)
        .bearer_auth(access_token)
        .query(&[
            ("q", query.as_str()),
            ("fields", FILE_FIELDS),
            ("pageSize", &PAGE_SIZE.to_string()),
        ]);

    if let Some(token) = page_token {
        request = request.query(&[("pageToken", token)]);
    }

    debug!(folder_id, "listing files page");

    let response = request.send().await.map_err(|e| GdriveError::ApiError {
        message: e.to_string(),
        status: e.status().map(|s| s.as_u16()),
    })?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "<failed to read body>".to_string());
        return Err(GdriveError::ApiError {
            message: format!("Drive API returned {status}: {body}"),
            status: Some(status.as_u16()),
        });
    }

    response
        .json::<DriveFileListResponse>()
        .await
        .map_err(|e| GdriveError::ParseError {
            message: e.to_string(),
        })
}

/// Convert a `DriveFile` to a `SourceItem`.
fn drive_file_to_source_item(file: &DriveFile, path: &str) -> SourceItem {
    let modified_at = file
        .modified_time
        .as_deref()
        .and_then(|s| s.parse::<DateTime<Utc>>().ok());

    SourceItem {
        id: file.id.clone(),
        display_name: file.name.clone(),
        mime_type: file.mime_type.clone(),
        path: path.to_string(),
        modified_at,
        source_hash: file.md5_checksum.clone(),
    }
}
```

### `src/lib.rs`

```rust
//! Google Drive source adapter for the ECL pipeline runner.
//!
//! This crate implements the `SourceAdapter` trait for Google Drive sources.
//! It handles OAuth2 authentication, file enumeration with pagination,
//! recursive folder traversal, and filter application.
//!
//! ## Milestone scope
//!
//! This crate currently implements `enumerate()` only. The `fetch()`
//! method is a stub — full implementation is in milestone 4.2.

pub mod auth;
pub mod enumerate;
pub mod error;
pub mod filter;
pub mod types;

pub use error::{GdriveError, Result};

use async_trait::async_trait;
use reqwest::Client;
use tracing::instrument;

use ecl_pipeline_spec::GoogleDriveSourceSpec;
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceError, SourceItem};

use crate::filter::CompiledFilters;

/// Google Drive source adapter.
///
/// Implements `SourceAdapter` for Google Drive, handling OAuth2 authentication,
/// file enumeration via the Drive Files.list API, and filtering.
#[derive(Debug)]
pub struct GoogleDriveAdapter {
    /// The source name from the pipeline spec (for error context).
    source_name: String,
    /// The source specification.
    spec: GoogleDriveSourceSpec,
    /// Pre-compiled filters.
    filters: CompiledFilters,
    /// HTTP client (shared, connection-pooled).
    client: Client,
}

impl GoogleDriveAdapter {
    /// Create a new Google Drive adapter from a source specification.
    ///
    /// Compiles filter patterns at construction time. Does NOT authenticate
    /// eagerly — authentication happens on first `enumerate()` or `fetch()` call.
    pub fn new(source_name: String, spec: GoogleDriveSourceSpec) -> Result<Self> {
        let filters = CompiledFilters::compile(
            &spec.filters,
            &spec.file_types,
            spec.modified_after.as_deref(),
        )?;

        Ok(Self {
            source_name,
            spec,
            filters,
            client: Client::new(),
        })
    }

    /// Create a new adapter with a custom `reqwest::Client`.
    ///
    /// Useful for testing (e.g., pointing at a wiremock server) or for
    /// sharing a client across multiple adapters.
    pub fn with_client(
        source_name: String,
        spec: GoogleDriveSourceSpec,
        client: Client,
    ) -> Result<Self> {
        let filters = CompiledFilters::compile(
            &spec.filters,
            &spec.file_types,
            spec.modified_after.as_deref(),
        )?;

        Ok(Self {
            source_name,
            spec,
            filters,
            client,
        })
    }
}

#[async_trait]
impl SourceAdapter for GoogleDriveAdapter {
    fn source_kind(&self) -> &str {
        "Google Drive"
    }

    #[instrument(skip(self), fields(source = %self.source_name))]
    async fn enumerate(&self) -> std::result::Result<Vec<SourceItem>, SourceError> {
        // 1. Authenticate
        let authenticator = auth::resolve_authenticator(&self.spec.credentials)
            .await
            .map_err(|e| e.into_source_error(&self.source_name))?;

        let access_token = auth::get_access_token(&authenticator)
            .await
            .map_err(|e| e.into_source_error(&self.source_name))?;

        // 2. Enumerate files from all root folders
        enumerate::enumerate_files(
            &self.client,
            &access_token,
            &self.spec.root_folders,
            &self.filters,
        )
        .await
        .map_err(|e| e.into_source_error(&self.source_name))
    }

    async fn fetch(
        &self,
        _item: &SourceItem,
    ) -> std::result::Result<ExtractedDocument, SourceError> {
        // TODO(milestone-4.2): Implement file content download.
        // This will use the Drive Files.get endpoint with `alt=media`
        // to download file content, compute blake3 hash, and build
        // an ExtractedDocument with provenance metadata.
        todo!("fetch() is not yet implemented — see milestone 4.2")
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Scaffold crate and verify compilation

- [ ] Create `crates/ecl-adapter-gdrive/` directory structure
- [ ] Create `Cargo.toml` (from section 5)
- [ ] Create minimal `src/lib.rs` with just `//! Google Drive adapter crate.`
- [ ] Add `wiremock = "0.6"`, `yup-oauth2 = "13"`, `glob = "0.3"` to root `Cargo.toml` `[workspace.dependencies]`
- [ ] Add `"crates/ecl-adapter-gdrive"` to root `Cargo.toml` `[workspace] members`
- [ ] Run `cargo check -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): scaffold crate`

### Step 2: Error types

- [ ] Create `src/error.rs` with `GdriveError` enum (from section 6)
- [ ] Add `pub mod error;` and `pub use error::{GdriveError, Result};` to `lib.rs`
- [ ] Write tests:
  - `test_error_display_credential_error` — verify Display output
  - `test_error_display_api_error_with_status` — status code appears
  - `test_error_display_api_error_without_status` — handles None status
  - `test_into_source_error_auth_errors` — credential/token/env errors map to `SourceError::AuthError`
  - `test_into_source_error_401_maps_to_auth` — API 401 maps to `AuthError`
  - `test_into_source_error_403_maps_to_auth` — API 403 maps to `AuthError`
  - `test_into_source_error_404_maps_to_not_found` — API 404 maps to `NotFound`
  - `test_into_source_error_429_maps_to_rate_limited` — API 429 maps to `RateLimited`
  - `test_into_source_error_500_maps_to_transient` — API 5xx maps to `Transient`
  - `test_error_implements_send_sync` — `GdriveError: Send + Sync`
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add GdriveError types`

### Step 3: Drive API types

- [ ] Create `src/types.rs` with `DriveFileListResponse`, `DriveFile` (from section 6)
- [ ] Add `pub mod types;` to `lib.rs`
- [ ] Write tests:
  - `test_drive_file_list_response_deserialize_single_page` — JSON with files, no next_page_token
  - `test_drive_file_list_response_deserialize_with_pagination` — JSON with next_page_token
  - `test_drive_file_list_response_deserialize_empty` — empty files array
  - `test_drive_file_is_folder_true` — folder MIME type
  - `test_drive_file_is_folder_false` — non-folder MIME type
  - `test_drive_file_deserialize_missing_optional_fields` — md5_checksum and modified_time absent
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add Drive API types`

### Step 4: Filter logic

- [ ] Create `src/filter.rs` with `CompiledFilters` (from section 6)
- [ ] Add `pub mod filter;` to `lib.rs`
- [ ] Write tests:
  - `test_filter_compile_valid_patterns` — compiles without error
  - `test_filter_compile_invalid_pattern_returns_error` — bad glob fails
  - `test_filter_matches_file_type_by_extension` — extension match
  - `test_filter_matches_file_type_by_mime` — MIME match
  - `test_filter_rejects_unmatched_file_type` — file excluded by type
  - `test_filter_excludes_glob_pattern` — Exclude rule rejects file
  - `test_filter_includes_glob_pattern` — Include rule accepts file
  - `test_filter_glob_first_match_wins` — Exclude before Include
  - `test_filter_no_rules_default_include` — empty rules = include all
  - `test_filter_modified_after_rejects_old_file` — file before threshold
  - `test_filter_modified_after_accepts_new_file` — file after threshold
  - `test_filter_modified_after_last_run_ignored` — "last_run" treated as no filter
  - `test_filter_folders_skip_file_type_check` — folders bypass type filter
  - `test_filter_file_type_extension_case_insensitive` — "PDF" matches "pdf"
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add filter logic`

### Step 5: Enumeration with wiremock

- [ ] Create `src/enumerate.rs` with `enumerate_files()` and helpers (from section 6)
- [ ] Add `pub mod enumerate;` to `lib.rs`
- [ ] Write tests using `wiremock`:
  - `test_enumerate_single_page_returns_items` — mock single-page response, verify SourceItems
  - `test_enumerate_multi_page_pagination` — mock two pages with nextPageToken, verify all items returned
  - `test_enumerate_recursive_folder_traversal` — mock folder containing files and subfolder, verify paths include folder hierarchy
  - `test_enumerate_empty_folder_returns_empty` — mock empty response
  - `test_enumerate_applies_file_type_filter` — mock mixed file types, verify only matching types returned
  - `test_enumerate_applies_glob_filter` — mock files, verify glob Exclude removes matching paths
  - `test_enumerate_applies_modified_after_filter` — mock old and new files, verify old ones excluded
  - `test_enumerate_api_401_returns_auth_error` — mock 401, verify error mapping
  - `test_enumerate_api_403_returns_auth_error` — mock 403
  - `test_enumerate_api_404_returns_not_found` — mock 404
  - `test_enumerate_api_429_returns_rate_limited` — mock 429
  - `test_enumerate_api_500_returns_transient` — mock 500
  - `test_enumerate_source_item_fields_correct` — verify id, display_name, mime_type, path, modified_at, source_hash fields
  - `test_enumerate_multiple_root_folders` — two root folder IDs, verify files from both
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add file enumeration`

### Step 6: Auth module

- [ ] Create `src/auth.rs` with credential resolution functions (from section 6)
- [ ] Add `pub mod auth;` to `lib.rs`
- [ ] Write tests:
  - `test_read_key_from_env_missing_var_returns_error` — unset env var
  - `test_read_key_from_env_invalid_json_returns_error` — env var with bad JSON
  - `test_read_key_from_file_missing_file_returns_error` — nonexistent path
  - `test_read_key_from_file_invalid_json_returns_error` — file with bad JSON (use `tempfile`)
  - `test_parse_service_account_key_valid` — valid service account JSON
  - `test_parse_service_account_key_invalid` — garbage string
- [ ] Note: Full authenticator tests require real credentials or extensive mocking
  of `yup-oauth2` internals. The HTTP-level integration is tested via wiremock
  in step 5. Auth tests focus on the credential resolution logic (file reading,
  env var reading, JSON parsing).
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add auth module`

### Step 7: GoogleDriveAdapter struct and SourceAdapter impl

- [ ] Add `GoogleDriveAdapter` struct and `SourceAdapter` impl to `lib.rs` (from section 6)
- [ ] Write tests:
  - `test_adapter_new_compiles_filters` — construction succeeds with valid spec
  - `test_adapter_new_invalid_glob_returns_error` — bad glob in spec fails construction
  - `test_adapter_source_kind_returns_google_drive` — verify trait method
  - `test_adapter_with_client_uses_custom_client` — verify custom client is used
- [ ] Run tests, verify pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add GoogleDriveAdapter with SourceAdapter impl`

### Step 8: Final polish and coverage

- [ ] Run `make test` — all tests pass
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Verify all public items have doc comments
- [ ] Run `make coverage` — verify 95% or better
- [ ] Add any missing tests to reach coverage target
- [ ] Commit: `feat(ecl-adapter-gdrive): final polish and coverage`

## 8. Test Fixtures

### Drive Files.list Single-Page Response

```json
{
  "files": [
    {
      "id": "file-001",
      "name": "design-doc.pdf",
      "mimeType": "application/pdf",
      "modifiedTime": "2026-03-01T10:00:00.000Z",
      "md5Checksum": "d41d8cd98f00b204e9800998ecf8427e",
      "parents": ["folder-root"]
    },
    {
      "id": "file-002",
      "name": "notes.docx",
      "mimeType": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
      "modifiedTime": "2026-03-10T15:30:00.000Z",
      "md5Checksum": "098f6bcd4621d373cade4e832627b4f6",
      "parents": ["folder-root"]
    }
  ]
}
```

### Drive Files.list Multi-Page Response (Page 1)

```json
{
  "nextPageToken": "page2-token-abc",
  "files": [
    {
      "id": "file-001",
      "name": "first.pdf",
      "mimeType": "application/pdf",
      "modifiedTime": "2026-03-01T10:00:00.000Z",
      "md5Checksum": "aaa111",
      "parents": ["folder-root"]
    }
  ]
}
```

### Drive Files.list Multi-Page Response (Page 2)

```json
{
  "files": [
    {
      "id": "file-002",
      "name": "second.pdf",
      "mimeType": "application/pdf",
      "modifiedTime": "2026-03-02T10:00:00.000Z",
      "md5Checksum": "bbb222",
      "parents": ["folder-root"]
    }
  ]
}
```

### Drive Files.list with Folder and Files

```json
{
  "files": [
    {
      "id": "subfolder-001",
      "name": "Archive",
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-01-15T00:00:00.000Z",
      "parents": ["folder-root"]
    },
    {
      "id": "file-003",
      "name": "readme.pdf",
      "mimeType": "application/pdf",
      "modifiedTime": "2026-03-05T12:00:00.000Z",
      "md5Checksum": "ccc333",
      "parents": ["folder-root"]
    }
  ]
}
```

### Minimal GoogleDriveSourceSpec (for unit tests)

```rust
use ecl_pipeline_spec::{CredentialRef, GoogleDriveSourceSpec};

fn test_spec() -> GoogleDriveSourceSpec {
    GoogleDriveSourceSpec {
        credentials: CredentialRef::EnvVar {
            env: "TEST_CREDS".to_string(),
        },
        root_folders: vec!["folder-root".to_string()],
        filters: vec![],
        file_types: vec![],
        modified_after: None,
    }
}
```

### Valid Service Account Key JSON (for auth tests)

```json
{
  "type": "service_account",
  "project_id": "test-project",
  "private_key_id": "key-id-123",
  "private_key": "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA2a2rwplBQLf8CA...<truncated for brevity>...\n-----END RSA PRIVATE KEY-----\n",
  "client_email": "test@test-project.iam.gserviceaccount.com",
  "client_id": "123456789",
  "auth_uri": "https://accounts.google.com/o/oauth2/auth",
  "token_uri": "https://oauth2.googleapis.com/token"
}
```

Note: For actual tests, use a properly formed RSA private key or mock the
JSON parsing layer. The key above is intentionally truncated.

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_error_display_credential_error` | `error` | `GdriveError::CredentialError` Display output |
| `test_error_display_api_error_with_status` | `error` | Status code included in Display |
| `test_error_display_api_error_without_status` | `error` | None status handled gracefully |
| `test_into_source_error_auth_errors` | `error` | Credential/token/env errors map to `SourceError::AuthError` |
| `test_into_source_error_401_maps_to_auth` | `error` | API 401 maps to `SourceError::AuthError` |
| `test_into_source_error_403_maps_to_auth` | `error` | API 403 maps to `SourceError::AuthError` |
| `test_into_source_error_404_maps_to_not_found` | `error` | API 404 maps to `SourceError::NotFound` |
| `test_into_source_error_429_maps_to_rate_limited` | `error` | API 429 maps to `SourceError::RateLimited` |
| `test_into_source_error_500_maps_to_transient` | `error` | API 5xx maps to `SourceError::Transient` |
| `test_error_implements_send_sync` | `error` | `GdriveError: Send + Sync` |
| `test_drive_file_list_response_deserialize_single_page` | `types` | Single-page JSON deserializes correctly |
| `test_drive_file_list_response_deserialize_with_pagination` | `types` | Pagination token captured |
| `test_drive_file_list_response_deserialize_empty` | `types` | Empty files array deserializes |
| `test_drive_file_is_folder_true` | `types` | Folder MIME type detected |
| `test_drive_file_is_folder_false` | `types` | Non-folder MIME type |
| `test_drive_file_deserialize_missing_optional_fields` | `types` | Missing md5/modifiedTime handled |
| `test_filter_compile_valid_patterns` | `filter` | Valid globs compile successfully |
| `test_filter_compile_invalid_pattern_returns_error` | `filter` | Invalid glob returns `InvalidPattern` |
| `test_filter_matches_file_type_by_extension` | `filter` | Extension filter matches |
| `test_filter_matches_file_type_by_mime` | `filter` | MIME filter matches |
| `test_filter_rejects_unmatched_file_type` | `filter` | Non-matching type excluded |
| `test_filter_excludes_glob_pattern` | `filter` | Exclude glob rejects file |
| `test_filter_includes_glob_pattern` | `filter` | Include glob accepts file |
| `test_filter_glob_first_match_wins` | `filter` | Exclude before Include = excluded |
| `test_filter_no_rules_default_include` | `filter` | Empty rules = include all |
| `test_filter_modified_after_rejects_old_file` | `filter` | Old file excluded |
| `test_filter_modified_after_accepts_new_file` | `filter` | New file included |
| `test_filter_modified_after_last_run_ignored` | `filter` | "last_run" passes through as no filter |
| `test_filter_folders_skip_file_type_check` | `filter` | Folders bypass type filter |
| `test_filter_file_type_extension_case_insensitive` | `filter` | "PDF" matches "pdf" filter |
| `test_enumerate_single_page_returns_items` | `enumerate` | Single-page response produces correct SourceItems |
| `test_enumerate_multi_page_pagination` | `enumerate` | Two pages collected into single result |
| `test_enumerate_recursive_folder_traversal` | `enumerate` | Subfolder files have correct path prefix |
| `test_enumerate_empty_folder_returns_empty` | `enumerate` | Empty Drive folder returns empty vec |
| `test_enumerate_applies_file_type_filter` | `enumerate` | Only matching file types returned |
| `test_enumerate_applies_glob_filter` | `enumerate` | Glob Exclude removes matching paths |
| `test_enumerate_applies_modified_after_filter` | `enumerate` | Old files excluded |
| `test_enumerate_api_401_returns_auth_error` | `enumerate` | 401 response mapped correctly |
| `test_enumerate_api_403_returns_auth_error` | `enumerate` | 403 response mapped correctly |
| `test_enumerate_api_404_returns_not_found` | `enumerate` | 404 response mapped correctly |
| `test_enumerate_api_429_returns_rate_limited` | `enumerate` | 429 response mapped correctly |
| `test_enumerate_api_500_returns_transient` | `enumerate` | 500 response mapped correctly |
| `test_enumerate_source_item_fields_correct` | `enumerate` | All SourceItem fields populated from DriveFile |
| `test_enumerate_multiple_root_folders` | `enumerate` | Files from multiple roots combined |
| `test_read_key_from_env_missing_var_returns_error` | `auth` | Missing env var fails |
| `test_read_key_from_env_invalid_json_returns_error` | `auth` | Bad JSON in env var fails |
| `test_read_key_from_file_missing_file_returns_error` | `auth` | Nonexistent file fails |
| `test_read_key_from_file_invalid_json_returns_error` | `auth` | Bad JSON in file fails |
| `test_parse_service_account_key_valid` | `auth` | Valid JSON parses |
| `test_parse_service_account_key_invalid` | `auth` | Garbage string fails |
| `test_adapter_new_compiles_filters` | `lib` | Construction with valid spec succeeds |
| `test_adapter_new_invalid_glob_returns_error` | `lib` | Bad glob in spec fails construction |
| `test_adapter_source_kind_returns_google_drive` | `lib` | `source_kind()` returns "Google Drive" |
| `test_adapter_with_client_uses_custom_client` | `lib` | Custom client accepted |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-adapter-gdrive` passes
- [ ] `cargo test -p ecl-adapter-gdrive` passes (all tests green)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT implement `fetch()` (that is milestone 4.2)
- Do NOT modify the pipeline runner (that is milestone 2.2)
- Do NOT add CLI commands (that is milestone 5.1)
- Do NOT modify `ecl-pipeline-spec`, `ecl-pipeline-topo`, or `ecl-pipeline-state`
- Do NOT add retry logic in the adapter (the pipeline runner handles retries)
- Do NOT add caching of enumeration results (that is a future optimization)
- Do NOT implement download/export of Google Docs native formats (that is milestone 4.2)
- `fetch()` should return `todo!("fetch() is not yet implemented — see milestone 4.2")`
