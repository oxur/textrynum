# Milestone 4.2: Google Drive Fetch & Full Pipeline (`ecl-adapter-gdrive` + `ecl-stages`)

## 0. Preamble

> **For the implementing agent:** This document is your complete specification.
> You do not need to read any other design docs, project plans, or prior
> milestone code. Everything you need is contained here. Follow the steps
> in order. Use TDD: write tests first, then implementation. Run
> `make test`, `make lint`, `make format` after each logical step.

**Workspace root:** `/Users/oubiwann/lab/oxur/ecl/`

**Commit prefix:** `feat(ecl-adapter-gdrive):` for adapter code, `feat(ecl-stages):` for normalize stage

## 1. Goal

Implement the `fetch()` method on `GoogleDriveAdapter` in the `ecl-adapter-gdrive`
crate (created in milestone 4.1) and enhance the `NormalizeStage` in `ecl-stages`
to handle Google Drive content types. When done:

1. `GoogleDriveAdapter::fetch()` downloads file content from Google Drive via
   the Files.get API (`alt=media`) for binary files and the Files.export API
   for Google Docs native formats.
2. Native Google Docs are exported as markdown/CSV/plain text depending on
   type; binary files (PDF, DOCX, etc.) are downloaded as raw bytes.
3. A blake3 hash is computed for every fetched document.
4. An `ExtractedDocument` is constructed with content, MIME type, and
   provenance metadata (file_id, path, owner).
5. HTTP 429 (rate limit) responses are mapped to `SourceError::RateLimited`
   so the pipeline's retry mechanism (backon) handles backoff.
6. HTTP 404 responses are mapped to `SourceError::NotFound` (non-retryable).
7. `NormalizeStage` in `ecl-stages` handles `text/markdown`, `text/plain`,
   and `text/csv` as pass-through; all other MIME types pass through with a
   metadata annotation noting they are unconverted.
8. An integration test demonstrates a mock Drive folder flowing through the
   full pipeline to output files.

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
- **Error pattern:** `thiserror::Error` + `#[non_exhaustive]` + `pub type Result<T> = std::result::Result<T, TheError>;`
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
4. `assets/ai/ai-rust/guides/08-performance.md`

### 2.3 Reference Patterns

Follow the structure of these existing crates for conventions:
- `crates/ecl-core/Cargo.toml` — Cargo.toml layout, lint config, workspace dep style
- `crates/ecl-core/src/error.rs` — Error enum pattern (thiserror, non_exhaustive, Result alias)
- `crates/ecl-core/src/lib.rs` — Module structure, re-exports, doc comments
- `crates/ecl-core/src/llm/provider.rs` — `async_trait` pattern for object-safe traits

## 3. Prior Art / Dependencies

This milestone **extends** two crates. Below are the exact public APIs you
depend on. You do NOT implement these — they already exist.

### From `ecl-pipeline-topo` (Milestone 1.3)

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use ecl_pipeline_state::{Blake3Hash, ItemProvenance};

/// A source adapter handles all interaction with an external data service.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    fn source_kind(&self) -> &str;
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by enumerate().
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    pub id: String,
    pub display_name: String,
    pub mime_type: String,
    pub path: String,
    pub modified_at: Option<DateTime<Utc>>,
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

/// The intermediate representation flowing between stages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    pub id: String,
    pub display_name: String,
    #[serde(with = "serde_bytes")]
    pub content: Arc<[u8]>,
    pub mime_type: String,
    pub source_name: String,
    pub source_content_hash: Blake3Hash,
    pub provenance: ItemProvenance,
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
#[derive(Debug, Clone)]
pub struct StageContext {
    pub spec: Arc<PipelineSpec>,
    pub output_dir: PathBuf,
    pub params: serde_json::Value,
    pub span: tracing::Span,
}
```

### From `ecl-pipeline-state` (Milestone 1.2)

```rust
/// Blake3 content hash, stored as hex string for JSON readability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);
impl Blake3Hash {
    pub fn new(hex: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}

/// Provenance metadata for a pipeline item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemProvenance {
    pub source_kind: String,
    pub metadata: BTreeMap<String, serde_json::Value>,
    pub source_modified: Option<DateTime<Utc>>,
    pub extracted_at: DateTime<Utc>,
}
```

### From `ecl-adapter-gdrive` (Milestone 4.1)

```rust
/// The Google Drive source adapter.
/// Created in milestone 4.1 with enumerate() implemented.
/// This milestone adds the fetch() implementation.
#[derive(Debug)]
pub struct GoogleDriveAdapter {
    /// The authenticated HTTP client for Drive API calls.
    client: reqwest::Client,
    /// The source name (for error reporting).
    source_name: String,
    /// The source spec configuration.
    spec: GoogleDriveSourceSpec,
    /// Base URL for the Drive API (overridable for testing).
    base_url: String,
}
```

### From `ecl-pipeline-spec` (Milestone 1.1)

```rust
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
```

## 4. Files to Create/Modify

| File | Action | Purpose |
|------|--------|---------|
| `crates/ecl-adapter-gdrive/src/fetch.rs` | Create | `fetch()` implementation: download, export, hash, construct ExtractedDocument |
| `crates/ecl-adapter-gdrive/src/mime.rs` | Create | Google Docs MIME type constants, export format mapping |
| `crates/ecl-adapter-gdrive/src/lib.rs` | Modify | Add `mod fetch; mod mime;`, wire `fetch()` into `SourceAdapter` impl |
| `crates/ecl-adapter-gdrive/Cargo.toml` | Modify | Add `blake3` dependency (if not already present) |
| `crates/ecl-stages/src/normalize.rs` | Modify | Handle `text/markdown`, `text/plain`, `text/csv` pass-through; annotate unconverted types |
| `crates/ecl-stages/src/lib.rs` | Modify (if needed) | Ensure `NormalizeStage` is exported |

## 5. Cargo.toml

### `ecl-adapter-gdrive` Additions

Ensure these dependencies are present in `crates/ecl-adapter-gdrive/Cargo.toml`
(some may already exist from milestone 4.1):

```toml
[dependencies]
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
reqwest = { workspace = true, features = ["json"] }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
blake3 = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }

[dev-dependencies]
wiremock = { workspace = true }
tokio = { workspace = true, features = ["test-util", "macros"] }
tempfile = { workspace = true }
```

### `ecl-stages` Additions

No new dependencies expected — `ecl-stages` already depends on
`ecl-pipeline-topo` which provides `PipelineItem`, `StageContext`, and
`StageError`.

## 6. Type Definitions and Signatures

### `src/mime.rs` (new file in `ecl-adapter-gdrive`)

```rust
//! Google Drive MIME type constants and export format mapping.

/// MIME type for Google Docs documents.
pub const GOOGLE_DOCS_DOCUMENT: &str = "application/vnd.google-apps.document";

/// MIME type for Google Sheets spreadsheets.
pub const GOOGLE_DOCS_SPREADSHEET: &str = "application/vnd.google-apps.spreadsheet";

/// MIME type for Google Slides presentations.
pub const GOOGLE_DOCS_PRESENTATION: &str = "application/vnd.google-apps.presentation";

/// Returns the export MIME type and the resulting content MIME type for a
/// Google Docs native format. Returns `None` for non-native formats
/// (which should be downloaded directly via Files.get).
///
/// Export strategy:
/// - Documents -> text/markdown (preferred) with text/plain fallback
/// - Spreadsheets -> text/csv
/// - Presentations -> text/plain
pub fn export_mime_type(google_mime: &str) -> Option<ExportMapping> {
    match google_mime {
        GOOGLE_DOCS_DOCUMENT => Some(ExportMapping {
            export_as: "text/markdown",
            result_mime: "text/markdown",
        }),
        GOOGLE_DOCS_SPREADSHEET => Some(ExportMapping {
            export_as: "text/csv",
            result_mime: "text/csv",
        }),
        GOOGLE_DOCS_PRESENTATION => Some(ExportMapping {
            export_as: "text/plain",
            result_mime: "text/plain",
        }),
        _ => None,
    }
}

/// Maps a Google Docs native MIME type to its export format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportMapping {
    /// The MIME type to request from the Files.export API.
    pub export_as: &'static str,
    /// The MIME type of the resulting content.
    pub result_mime: &'static str,
}
```

### `src/fetch.rs` (new file in `ecl-adapter-gdrive`)

```rust
//! Fetch implementation for Google Drive files.
//!
//! Handles two download paths:
//! 1. Native Google Docs formats (Document, Spreadsheet, Presentation) —
//!    exported via the Files.export API to a portable format.
//! 2. Binary files (PDF, DOCX, images, etc.) — downloaded directly via
//!    the Files.get API with `alt=media`.

use chrono::Utc;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::{ExtractedDocument, SourceError, SourceItem};
use std::collections::BTreeMap;

use crate::GoogleDriveAdapter;
use crate::mime;

impl GoogleDriveAdapter {
    /// Fetch the full content of a Google Drive file.
    ///
    /// For native Google Docs formats, this uses the Files.export API to
    /// convert to a portable format (markdown, CSV, plain text). For all
    /// other files, this downloads raw bytes via Files.get with `alt=media`.
    ///
    /// # Errors
    ///
    /// - `SourceError::RateLimited` — HTTP 429 from Drive API (retryable)
    /// - `SourceError::NotFound` — HTTP 404, file deleted or inaccessible
    /// - `SourceError::AuthError` — HTTP 401/403
    /// - `SourceError::Transient` — HTTP 5xx or network errors
    /// - `SourceError::Permanent` — other HTTP errors (4xx except 401/403/404/429)
    pub(crate) async fn fetch_file(
        &self,
        item: &SourceItem,
    ) -> Result<ExtractedDocument, SourceError> {
        let (content, result_mime) = match mime::export_mime_type(&item.mime_type) {
            Some(mapping) => {
                let bytes = self.export_native_doc(&item.id, mapping.export_as).await?;
                (bytes, mapping.result_mime.to_string())
            }
            None => {
                let bytes = self.download_file(&item.id).await?;
                (bytes, item.mime_type.clone())
            }
        };

        // Compute blake3 hash of the fetched content.
        let hash = blake3::hash(&content);
        let content_hash = Blake3Hash::new(hash.to_hex().to_string());

        // Build provenance metadata.
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "file_id".to_string(),
            serde_json::Value::String(item.id.clone()),
        );
        metadata.insert(
            "path".to_string(),
            serde_json::Value::String(item.path.clone()),
        );
        metadata.insert(
            "display_name".to_string(),
            serde_json::Value::String(item.display_name.clone()),
        );

        let provenance = ItemProvenance {
            source_kind: "google_drive".to_string(),
            metadata,
            source_modified: item.modified_at,
            extracted_at: Utc::now(),
        };

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content,
            mime_type: result_mime,
            provenance,
            content_hash,
        })
    }

    /// Download a binary file via Drive Files.get with `alt=media`.
    async fn download_file(&self, file_id: &str) -> Result<Vec<u8>, SourceError> {
        let url = format!("{}/drive/v3/files/{}", self.base_url, file_id);
        let response = self
            .client
            .get(&url)
            .query(&[("alt", "media")])
            .send()
            .await
            .map_err(|e| SourceError::Transient {
                source: self.source_name.clone(),
                message: e.to_string(),
            })?;

        self.handle_response_status(&response, file_id)?;
        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            SourceError::Transient {
                source: self.source_name.clone(),
                message: format!("failed to read response body: {e}"),
            }
        })
    }

    /// Export a native Google Docs file via Drive Files.export.
    async fn export_native_doc(
        &self,
        file_id: &str,
        export_mime: &str,
    ) -> Result<Vec<u8>, SourceError> {
        let url = format!("{}/drive/v3/files/{}/export", self.base_url, file_id);
        let response = self
            .client
            .get(&url)
            .query(&[("mimeType", export_mime)])
            .send()
            .await
            .map_err(|e| SourceError::Transient {
                source: self.source_name.clone(),
                message: e.to_string(),
            })?;

        self.handle_response_status(&response, file_id)?;
        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            SourceError::Transient {
                source: self.source_name.clone(),
                message: format!("failed to read export body: {e}"),
            }
        })
    }

    /// Map HTTP status codes to appropriate SourceError variants.
    fn handle_response_status(
        &self,
        response: &reqwest::Response,
        file_id: &str,
    ) -> Result<(), SourceError> {
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }

        match status.as_u16() {
            429 => {
                // Extract Retry-After header if present, default to 60s.
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);
                Err(SourceError::RateLimited {
                    source: self.source_name.clone(),
                    retry_after_secs: retry_after,
                })
            }
            404 => Err(SourceError::NotFound {
                source: self.source_name.clone(),
                item_id: file_id.to_string(),
            }),
            401 | 403 => Err(SourceError::AuthError {
                source: self.source_name.clone(),
                message: format!("HTTP {status} accessing file {file_id}"),
            }),
            s if (500..600).contains(&s) => Err(SourceError::Transient {
                source: self.source_name.clone(),
                message: format!("HTTP {status} accessing file {file_id}"),
            }),
            _ => Err(SourceError::Permanent {
                source: self.source_name.clone(),
                message: format!("HTTP {status} accessing file {file_id}"),
            }),
        }
    }
}
```

### `src/lib.rs` modifications (in `ecl-adapter-gdrive`)

Add module declarations and wire `fetch()` into the `SourceAdapter` impl:

```rust
// Add these module declarations:
pub mod fetch;
pub mod mime;

// In the SourceAdapter impl for GoogleDriveAdapter, the fetch method:
#[async_trait]
impl SourceAdapter for GoogleDriveAdapter {
    // ... source_kind() and enumerate() already exist from 4.1 ...

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        self.fetch_file(item).await
    }
}
```

### `NormalizeStage` enhancement (in `ecl-stages/src/normalize.rs`)

```rust
//! Normalize stage: converts extracted documents to a uniform text format.
//!
//! Current behavior:
//! - `text/markdown` -> pass through unchanged
//! - `text/plain` -> pass through unchanged
//! - `text/csv` -> pass through unchanged (content preserved as-is)
//! - All other MIME types -> pass through with metadata annotation
//!   `normalize.unconverted = true` and `normalize.original_mime = "<mime>"`
//!
//! Future work: PDF-to-markdown, DOCX-to-markdown, etc.

use async_trait::async_trait;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext, StageError};

/// Stage that normalizes content to a common text format.
#[derive(Debug)]
pub struct NormalizeStage;

impl NormalizeStage {
    /// Known text MIME types that pass through without conversion.
    const PASSTHROUGH_TYPES: &[&str] = &[
        "text/markdown",
        "text/plain",
        "text/csv",
    ];
}

#[async_trait]
impl Stage for NormalizeStage {
    fn name(&self) -> &str {
        "normalize"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        if Self::PASSTHROUGH_TYPES.iter().any(|t| *t == item.mime_type) {
            // Text formats pass through unchanged.
            Ok(vec![item])
        } else {
            // Non-text formats: pass through content unchanged but annotate
            // metadata so downstream stages know this was not converted.
            item.metadata.insert(
                "normalize.unconverted".to_string(),
                serde_json::Value::Bool(true),
            );
            item.metadata.insert(
                "normalize.original_mime".to_string(),
                serde_json::Value::String(item.mime_type.clone()),
            );
            Ok(vec![item])
        }
    }
}
```

## 7. Implementation Steps (TDD Order)

### Step 1: Add MIME type constants and export mapping

- [ ] Create `crates/ecl-adapter-gdrive/src/mime.rs` with constants and `export_mime_type()` function
- [ ] Add `pub mod mime;` to `crates/ecl-adapter-gdrive/src/lib.rs`
- [ ] Write tests:
  - `test_export_mime_type_google_doc_returns_markdown`
  - `test_export_mime_type_google_sheet_returns_csv`
  - `test_export_mime_type_google_slides_returns_plain_text`
  - `test_export_mime_type_pdf_returns_none`
  - `test_export_mime_type_unknown_returns_none`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] `make lint` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add MIME type constants and export mapping`

### Step 2: Add HTTP status code mapping

- [ ] Create `crates/ecl-adapter-gdrive/src/fetch.rs` with `handle_response_status()` method
- [ ] Add `pub mod fetch;` to `crates/ecl-adapter-gdrive/src/lib.rs`
- [ ] Ensure `blake3` is in `Cargo.toml` dependencies
- [ ] Write tests using `wiremock`:
  - `test_handle_response_status_429_returns_rate_limited`
  - `test_handle_response_status_404_returns_not_found`
  - `test_handle_response_status_401_returns_auth_error`
  - `test_handle_response_status_403_returns_auth_error`
  - `test_handle_response_status_500_returns_transient`
  - `test_handle_response_status_400_returns_permanent`
  - `test_handle_response_status_200_returns_ok`
  - `test_handle_response_status_429_with_retry_after_header`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): add HTTP status code to SourceError mapping`

### Step 3: Implement binary file download

- [ ] Implement `download_file()` in `fetch.rs`
- [ ] Write tests using `wiremock`:
  - `test_download_file_success_returns_bytes`
  - `test_download_file_network_error_returns_transient`
  - `test_download_file_sends_alt_media_query_param`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): implement binary file download via Files.get`

### Step 4: Implement Google Docs export

- [ ] Implement `export_native_doc()` in `fetch.rs`
- [ ] Write tests using `wiremock`:
  - `test_export_native_doc_sends_correct_mime_type_query`
  - `test_export_native_doc_success_returns_bytes`
  - `test_export_native_doc_rate_limited_returns_retryable_error`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): implement Google Docs export via Files.export`

### Step 5: Implement fetch_file() and wire into SourceAdapter

- [ ] Implement `fetch_file()` in `fetch.rs`
- [ ] Wire `fetch()` in the `SourceAdapter` impl to delegate to `fetch_file()`
- [ ] Write tests using `wiremock`:
  - `test_fetch_file_binary_pdf_downloads_and_hashes`
  - `test_fetch_file_google_doc_exports_as_markdown`
  - `test_fetch_file_google_sheet_exports_as_csv`
  - `test_fetch_file_google_slides_exports_as_plain_text`
  - `test_fetch_file_computes_blake3_hash`
  - `test_fetch_file_constructs_provenance_metadata`
  - `test_fetch_file_sets_correct_mime_type_for_export`
  - `test_fetch_file_preserves_original_mime_for_binary`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `feat(ecl-adapter-gdrive): implement fetch() for full SourceAdapter`

### Step 6: Enhance NormalizeStage

- [ ] Modify `crates/ecl-stages/src/normalize.rs` to handle text pass-through and metadata annotation
- [ ] Write tests:
  - `test_normalize_markdown_passes_through_unchanged`
  - `test_normalize_plain_text_passes_through_unchanged`
  - `test_normalize_csv_passes_through_unchanged`
  - `test_normalize_pdf_passes_through_with_unconverted_metadata`
  - `test_normalize_unknown_mime_annotates_metadata`
  - `test_normalize_unconverted_metadata_contains_original_mime`
- [ ] Run `cargo test -p ecl-stages` — must pass
- [ ] Commit: `feat(ecl-stages): enhance NormalizeStage for Drive content types`

### Step 7: Integration test

- [ ] Create an integration test in `crates/ecl-adapter-gdrive/tests/` that:
  1. Starts a `wiremock` mock server simulating a Drive folder with 3 files:
     - A Google Doc (responds to export with markdown content)
     - A PDF (responds to download with raw bytes)
     - A Google Sheet (responds to export with CSV content)
  2. Creates a `GoogleDriveAdapter` pointing at the mock server
  3. Calls `enumerate()` to list all 3 files
  4. Calls `fetch()` for each file
  5. Verifies:
     - Content matches the mock responses
     - blake3 hashes are correct
     - MIME types are correct (text/markdown, application/pdf, text/csv)
     - Provenance metadata contains file_id and path
- [ ] Write test: `test_integration_mock_drive_folder_full_pipeline`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `test(ecl-adapter-gdrive): add integration test for mock Drive folder`

### Step 8: Error scenario integration tests

- [ ] Write integration tests:
  - `test_integration_fetch_rate_limited_returns_retryable_error`
  - `test_integration_fetch_not_found_returns_non_retryable_error`
  - `test_integration_fetch_server_error_returns_transient`
- [ ] Run `cargo test -p ecl-adapter-gdrive` — must pass
- [ ] Commit: `test(ecl-adapter-gdrive): add error scenario integration tests`

### Step 9: Final polish

- [ ] Run `make test` — all workspace tests pass
- [ ] Run `make lint` — no warnings
- [ ] Run `make format` — no changes
- [ ] Run `make coverage` — verify 95% or better
- [ ] Verify all public items have doc comments
- [ ] Commit: `feat(ecl-adapter-gdrive): finalize fetch implementation and tests`

## 8. Test Fixtures

### Mock Drive API Responses

#### Files.get with `alt=media` (binary download)

```
GET /drive/v3/files/file-pdf-001?alt=media
→ 200 OK
Content-Type: application/pdf
Body: <raw PDF bytes>
```

#### Files.export (Google Docs native format)

```
GET /drive/v3/files/file-doc-001/export?mimeType=text/markdown
→ 200 OK
Content-Type: text/markdown
Body: "# Meeting Notes\n\nDiscussed Q1 goals..."
```

```
GET /drive/v3/files/file-sheet-001/export?mimeType=text/csv
→ 200 OK
Content-Type: text/csv
Body: "Name,Status,Date\nAlpha,Active,2026-01-15\n"
```

```
GET /drive/v3/files/file-slides-001/export?mimeType=text/plain
→ 200 OK
Content-Type: text/plain
Body: "Slide 1: Introduction\nSlide 2: Architecture..."
```

#### Rate-limited response (429)

```
GET /drive/v3/files/any-file-id?alt=media
→ 429 Too Many Requests
Retry-After: 30
```

#### Not found response (404)

```
GET /drive/v3/files/deleted-file-id?alt=media
→ 404 Not Found
Body: {"error": {"code": 404, "message": "File not found"}}
```

### Sample SourceItem (for fetch tests)

```rust
SourceItem {
    id: "file-doc-001".to_string(),
    display_name: "Q1 Meeting Notes".to_string(),
    mime_type: "application/vnd.google-apps.document".to_string(),
    path: "/Engineering/Q1 Meeting Notes".to_string(),
    modified_at: Some(Utc::now()),
    source_hash: Some("abc123def456".to_string()),
}
```

### Sample PipelineItem (for NormalizeStage tests)

```rust
PipelineItem {
    id: "file-doc-001".to_string(),
    display_name: "Q1 Meeting Notes".to_string(),
    content: Arc::from(b"# Meeting Notes\n\nContent here" as &[u8]),
    mime_type: "text/markdown".to_string(),
    source_name: "engineering-drive".to_string(),
    source_content_hash: Blake3Hash::new("abcdef1234567890"),
    provenance: ItemProvenance {
        source_kind: "google_drive".to_string(),
        metadata: BTreeMap::new(),
        source_modified: Some(Utc::now()),
        extracted_at: Utc::now(),
    },
    metadata: BTreeMap::new(),
}
```

## 9. Test Specifications

| Test Name | Module | What It Verifies |
|-----------|--------|-----------------|
| `test_export_mime_type_google_doc_returns_markdown` | `mime` | Google Docs document maps to text/markdown export |
| `test_export_mime_type_google_sheet_returns_csv` | `mime` | Google Sheets maps to text/csv export |
| `test_export_mime_type_google_slides_returns_plain_text` | `mime` | Google Slides maps to text/plain export |
| `test_export_mime_type_pdf_returns_none` | `mime` | Non-native MIME type returns None |
| `test_export_mime_type_unknown_returns_none` | `mime` | Unknown MIME type returns None |
| `test_handle_response_status_429_returns_rate_limited` | `fetch` | HTTP 429 maps to SourceError::RateLimited |
| `test_handle_response_status_404_returns_not_found` | `fetch` | HTTP 404 maps to SourceError::NotFound |
| `test_handle_response_status_401_returns_auth_error` | `fetch` | HTTP 401 maps to SourceError::AuthError |
| `test_handle_response_status_403_returns_auth_error` | `fetch` | HTTP 403 maps to SourceError::AuthError |
| `test_handle_response_status_500_returns_transient` | `fetch` | HTTP 500 maps to SourceError::Transient |
| `test_handle_response_status_400_returns_permanent` | `fetch` | HTTP 400 maps to SourceError::Permanent |
| `test_handle_response_status_200_returns_ok` | `fetch` | HTTP 200 returns Ok(()) |
| `test_handle_response_status_429_with_retry_after_header` | `fetch` | Retry-After header value is extracted |
| `test_download_file_success_returns_bytes` | `fetch` | Files.get returns expected bytes |
| `test_download_file_network_error_returns_transient` | `fetch` | Network failure maps to Transient |
| `test_download_file_sends_alt_media_query_param` | `fetch` | Request includes `alt=media` query param |
| `test_export_native_doc_sends_correct_mime_type_query` | `fetch` | Export request includes correct mimeType |
| `test_export_native_doc_success_returns_bytes` | `fetch` | Export returns expected bytes |
| `test_export_native_doc_rate_limited_returns_retryable_error` | `fetch` | 429 on export maps to RateLimited |
| `test_fetch_file_binary_pdf_downloads_and_hashes` | `fetch` | PDF file downloaded and hashed correctly |
| `test_fetch_file_google_doc_exports_as_markdown` | `fetch` | Google Doc exported as markdown |
| `test_fetch_file_google_sheet_exports_as_csv` | `fetch` | Google Sheet exported as CSV |
| `test_fetch_file_google_slides_exports_as_plain_text` | `fetch` | Google Slides exported as plain text |
| `test_fetch_file_computes_blake3_hash` | `fetch` | blake3 hash matches expected value |
| `test_fetch_file_constructs_provenance_metadata` | `fetch` | Provenance contains file_id, path, source_kind |
| `test_fetch_file_sets_correct_mime_type_for_export` | `fetch` | Export result MIME type is the exported format, not the native type |
| `test_fetch_file_preserves_original_mime_for_binary` | `fetch` | Binary download preserves original MIME type |
| `test_normalize_markdown_passes_through_unchanged` | `normalize` | text/markdown content and MIME type unchanged |
| `test_normalize_plain_text_passes_through_unchanged` | `normalize` | text/plain content and MIME type unchanged |
| `test_normalize_csv_passes_through_unchanged` | `normalize` | text/csv content and MIME type unchanged |
| `test_normalize_pdf_passes_through_with_unconverted_metadata` | `normalize` | PDF content passed through, metadata.normalize.unconverted = true |
| `test_normalize_unknown_mime_annotates_metadata` | `normalize` | Unknown MIME type annotated as unconverted |
| `test_normalize_unconverted_metadata_contains_original_mime` | `normalize` | metadata.normalize.original_mime set correctly |
| `test_integration_mock_drive_folder_full_pipeline` | `integration` | 3-file mock folder: enumerate, fetch all, verify content/hashes/mime/provenance |
| `test_integration_fetch_rate_limited_returns_retryable_error` | `integration` | Mock 429 response produces RateLimited error |
| `test_integration_fetch_not_found_returns_non_retryable_error` | `integration` | Mock 404 response produces NotFound error |
| `test_integration_fetch_server_error_returns_transient` | `integration` | Mock 500 response produces Transient error |

## 10. Verification Checklist & Scope Boundaries

### Verification

- [ ] `cargo check -p ecl-adapter-gdrive` passes
- [ ] `cargo check -p ecl-stages` passes
- [ ] `cargo test -p ecl-adapter-gdrive` passes (all tests green)
- [ ] `cargo test -p ecl-stages` passes (all tests green)
- [ ] `make test` passes (workspace-wide)
- [ ] `make lint` passes (workspace-wide)
- [ ] `make format` produces no changes
- [ ] All public items have doc comments (`missing_docs = "warn"` produces no warnings)
- [ ] No compiler warnings
- [ ] Achieve 95% or better code coverage per the instructions in `assets/ai/CLAUDE-CODE-COVERAGE.md`

### What NOT to Do

- Do NOT modify the pipeline runner (`ecl-pipeline-runner` crate)
- Do NOT add CLI commands (that is milestone 5.1)
- Do NOT implement PDF text extraction — binary files pass through as raw bytes; real PDF-to-markdown conversion is future work
- Do NOT implement DOCX text extraction — same as PDF, future work
- Do NOT implement the Slack adapter (that is milestone 6.1)
- Do NOT implement Google Drive authentication or `enumerate()` (that is milestone 4.1)
- Do NOT add new crates — this milestone only modifies `ecl-adapter-gdrive` and `ecl-stages`
- Do NOT implement streaming downloads — fetch entire file into memory (our use case is documents, not multi-GB files)
- Do NOT implement caching of fetched content — the pipeline's incrementality mechanism (content hash comparison before fetch) handles this at a higher level
