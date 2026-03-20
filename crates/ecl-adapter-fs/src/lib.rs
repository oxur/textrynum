//! Filesystem source adapter for the ECL pipeline runner.
//!
//! Provides `FilesystemAdapter`, which implements `SourceAdapter` by walking
//! a local directory tree, applying extension and glob filters, and reading
//! file contents with blake3 hashing.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

mod error;

pub use error::FsAdapterError;

use async_trait::async_trait;
use async_walkdir::WalkDir;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use glob::Pattern;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tracing::debug;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::source::{FilesystemSourceSpec, FilterAction, FilterRule};
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::{ResolveError, SourceError};
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

/// Filesystem source adapter.
///
/// Recursively walks a root directory, applying extension and glob filters
/// during enumeration. Fetches file content and computes blake3 hashes.
#[derive(Debug)]
pub struct FilesystemAdapter {
    /// Root directory to scan.
    root: PathBuf,
    /// File extensions to include (empty = all).
    extensions: Vec<String>,
    /// Compiled filter rules (include/exclude globs).
    filters: Vec<CompiledFilter>,
    /// Source name (for error reporting and provenance).
    source_name: String,
}

/// A compiled filter rule with a pre-parsed glob pattern.
#[derive(Debug)]
struct CompiledFilter {
    pattern: Pattern,
    action: FilterAction,
}

impl FilesystemAdapter {
    /// Create a new `FilesystemAdapter` from a `SourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if the spec is not a filesystem source.
    /// Returns `ResolveError::Io` if a glob pattern is invalid.
    pub fn from_spec(source_name: &str, spec: &SourceSpec) -> Result<Self, ResolveError> {
        let fs_spec = match spec {
            SourceSpec::Filesystem(fs) => fs,
            _ => {
                return Err(ResolveError::UnknownAdapter {
                    stage: source_name.to_string(),
                    adapter: "filesystem".to_string(),
                });
            }
        };

        Self::from_fs_spec(source_name, fs_spec)
    }

    /// Create a new `FilesystemAdapter` directly from a `FilesystemSourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::Io` if a glob pattern is invalid.
    pub fn from_fs_spec(
        source_name: &str,
        spec: &FilesystemSourceSpec,
    ) -> Result<Self, ResolveError> {
        let filters = compile_filters(&spec.filters)?;

        Ok(Self {
            root: spec.root.clone(),
            extensions: spec.extensions.clone(),
            filters,
            source_name: source_name.to_string(),
        })
    }

    /// Check whether a path passes the extension filter.
    fn matches_extension(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| {
                self.extensions
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(ext))
            })
    }

    /// Evaluate filter rules against a path. Rules are evaluated in order;
    /// the last matching rule wins. If no rule matches, the item is included.
    fn passes_filters(&self, path: &str) -> bool {
        let mut included = true;
        for filter in &self.filters {
            if filter.pattern.matches(path) {
                included = filter.action == FilterAction::Include;
            }
        }
        included
    }

    /// Compute a relative path string for filtering and identification.
    fn relative_path(&self, abs_path: &Path) -> String {
        abs_path
            .strip_prefix(&self.root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .into_owned()
    }

    /// Detect MIME type from file extension.
    fn mime_from_extension(path: &Path) -> String {
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext.to_ascii_lowercase().as_str() {
            "md" | "markdown" => "text/markdown".to_string(),
            "txt" => "text/plain".to_string(),
            "html" | "htm" => "text/html".to_string(),
            "json" => "application/json".to_string(),
            "toml" => "application/toml".to_string(),
            "yaml" | "yml" => "application/yaml".to_string(),
            "pdf" => "application/pdf".to_string(),
            "csv" => "text/csv".to_string(),
            "xml" => "application/xml".to_string(),
            "rs" => "text/x-rust".to_string(),
            "py" => "text/x-python".to_string(),
            "js" => "text/javascript".to_string(),
            "ts" => "text/typescript".to_string(),
            _ => "application/octet-stream".to_string(),
        }
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
impl SourceAdapter for FilesystemAdapter {
    fn source_kind(&self) -> &str {
        "filesystem"
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        let mut items = Vec::new();
        let mut walker = WalkDir::new(&self.root);

        while let Some(entry) = walker.next().await {
            let entry = entry.map_err(|e| SourceError::Permanent {
                source_name: self.source_name.clone(),
                message: format!("directory walk error: {e}"),
            })?;

            let path = entry.path();

            // Skip directories
            let metadata =
                tokio::fs::metadata(&path)
                    .await
                    .map_err(|e| SourceError::Transient {
                        source_name: self.source_name.clone(),
                        message: format!("failed to read metadata for {}: {e}", path.display()),
                    })?;

            if metadata.is_dir() {
                continue;
            }

            // Apply extension filter
            if !self.matches_extension(&path) {
                debug!(path = %path.display(), "skipped: extension filter");
                continue;
            }

            // Apply glob filters
            let rel_path = self.relative_path(&path);
            if !self.passes_filters(&rel_path) {
                debug!(path = %path.display(), "skipped: glob filter");
                continue;
            }

            let modified_at: Option<DateTime<Utc>> =
                metadata.modified().ok().map(DateTime::<Utc>::from);

            let display_name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| rel_path.clone());

            let mime_type = Self::mime_from_extension(&path);

            items.push(SourceItem {
                id: rel_path.clone(),
                display_name,
                mime_type,
                path: rel_path,
                modified_at,
                source_hash: None,
            });
        }

        // Sort for deterministic ordering
        items.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(items)
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        let abs_path = self.root.join(&item.path);

        let content = tokio::fs::read(&abs_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SourceError::NotFound {
                    source_name: self.source_name.clone(),
                    item_id: item.id.clone(),
                }
            } else {
                SourceError::Transient {
                    source_name: self.source_name.clone(),
                    message: format!("failed to read {}: {e}", abs_path.display()),
                }
            }
        })?;

        let content_hash = Blake3Hash::new(blake3::hash(&content).to_hex().as_str());

        let metadata = tokio::fs::metadata(&abs_path).await.ok();
        let source_modified = metadata
            .and_then(|m| m.modified().ok())
            .map(DateTime::<Utc>::from);

        let mut prov_metadata = BTreeMap::new();
        prov_metadata.insert(
            "path".to_string(),
            serde_json::Value::String(item.path.clone()),
        );

        let provenance = ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: prov_metadata,
            source_modified,
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
    use ecl_pipeline_spec::source::FilterRule;
    use std::fs;
    use tempfile::TempDir;

    fn make_fs_spec(root: &Path) -> FilesystemSourceSpec {
        FilesystemSourceSpec {
            root: root.to_path_buf(),
            filters: vec![],
            extensions: vec![],
            stream: None,
        }
    }

    fn make_adapter(root: &Path) -> FilesystemAdapter {
        FilesystemAdapter::from_fs_spec("test-source", &make_fs_spec(root)).unwrap()
    }

    fn create_test_files(dir: &Path) {
        fs::write(dir.join("readme.md"), "# Hello").unwrap();
        fs::write(dir.join("notes.txt"), "Some notes").unwrap();
        fs::write(dir.join("data.json"), r#"{"key": "value"}"#).unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/nested.md"), "# Nested").unwrap();
        fs::write(dir.join("sub/image.png"), [0x89, 0x50, 0x4E, 0x47]).unwrap();
    }

    // ── Construction tests ─────────────────────────────────────────

    #[test]
    fn test_from_spec_filesystem_source() {
        let spec = SourceSpec::Filesystem(FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec![],
            stream: None,
        });
        let adapter = FilesystemAdapter::from_spec("local", &spec).unwrap();
        assert_eq!(adapter.source_kind(), "filesystem");
        assert_eq!(adapter.root, PathBuf::from("/tmp"));
    }

    #[test]
    fn test_from_spec_wrong_kind_returns_error() {
        let spec = SourceSpec::Slack(ecl_pipeline_spec::source::SlackSourceSpec {
            credentials: ecl_pipeline_spec::source::CredentialRef::ApplicationDefault,
            channels: vec![],
            thread_depth: 0,
            modified_after: None,
            stream: None,
        });
        let result = FilesystemAdapter::from_spec("local", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_fs_spec_with_extensions() {
        let spec = FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![],
            extensions: vec!["md".to_string(), "txt".to_string()],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("local", &spec).unwrap();
        assert_eq!(adapter.extensions, vec!["md", "txt"]);
    }

    #[test]
    fn test_from_fs_spec_with_filters() {
        let spec = FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![
                FilterRule {
                    pattern: "**/*.md".to_string(),
                    action: FilterAction::Include,
                },
                FilterRule {
                    pattern: "**/Archive/**".to_string(),
                    action: FilterAction::Exclude,
                },
            ],
            extensions: vec![],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("local", &spec).unwrap();
        assert_eq!(adapter.filters.len(), 2);
    }

    #[test]
    fn test_invalid_glob_pattern_returns_error() {
        let spec = FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![FilterRule {
                pattern: "[invalid".to_string(),
                action: FilterAction::Include,
            }],
            extensions: vec![],
            stream: None,
        };
        let result = FilesystemAdapter::from_fs_spec("local", &spec);
        assert!(result.is_err());
    }

    // ── Filter tests ───────────────────────────────────────────────

    #[test]
    fn test_matches_extension_empty_allows_all() {
        let adapter = FilesystemAdapter {
            root: PathBuf::from("/tmp"),
            extensions: vec![],
            filters: vec![],
            source_name: "test".to_string(),
        };
        assert!(adapter.matches_extension(Path::new("file.md")));
        assert!(adapter.matches_extension(Path::new("file.rs")));
        assert!(adapter.matches_extension(Path::new("file")));
    }

    #[test]
    fn test_matches_extension_filters_correctly() {
        let adapter = FilesystemAdapter {
            root: PathBuf::from("/tmp"),
            extensions: vec!["md".to_string(), "txt".to_string()],
            filters: vec![],
            source_name: "test".to_string(),
        };
        assert!(adapter.matches_extension(Path::new("readme.md")));
        assert!(adapter.matches_extension(Path::new("notes.txt")));
        assert!(!adapter.matches_extension(Path::new("data.json")));
        assert!(!adapter.matches_extension(Path::new("no_ext")));
    }

    #[test]
    fn test_matches_extension_case_insensitive() {
        let adapter = FilesystemAdapter {
            root: PathBuf::from("/tmp"),
            extensions: vec!["md".to_string()],
            filters: vec![],
            source_name: "test".to_string(),
        };
        assert!(adapter.matches_extension(Path::new("readme.MD")));
        assert!(adapter.matches_extension(Path::new("readme.Md")));
    }

    #[test]
    fn test_passes_filters_no_rules_includes_all() {
        let adapter = FilesystemAdapter {
            root: PathBuf::from("/tmp"),
            extensions: vec![],
            filters: vec![],
            source_name: "test".to_string(),
        };
        assert!(adapter.passes_filters("any/path.md"));
    }

    #[test]
    fn test_passes_filters_exclude_rule() {
        let spec = FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![FilterRule {
                pattern: "**/Archive/**".to_string(),
                action: FilterAction::Exclude,
            }],
            extensions: vec![],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("test", &spec).unwrap();
        assert!(!adapter.passes_filters("Archive/old.md"));
        assert!(adapter.passes_filters("docs/new.md"));
    }

    #[test]
    fn test_passes_filters_last_rule_wins() {
        let spec = FilesystemSourceSpec {
            root: PathBuf::from("/tmp"),
            filters: vec![
                FilterRule {
                    pattern: "**/*.md".to_string(),
                    action: FilterAction::Exclude,
                },
                FilterRule {
                    pattern: "**/important/*.md".to_string(),
                    action: FilterAction::Include,
                },
            ],
            extensions: vec![],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("test", &spec).unwrap();
        // First rule excludes all .md, but second re-includes important/*.md
        assert!(!adapter.passes_filters("docs/readme.md"));
        assert!(adapter.passes_filters("important/readme.md"));
    }

    // ── MIME detection tests ───────────────────────────────────────

    #[test]
    fn test_mime_from_extension() {
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("x.md")),
            "text/markdown"
        );
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("x.txt")),
            "text/plain"
        );
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("x.json")),
            "application/json"
        );
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("x.pdf")),
            "application/pdf"
        );
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("x.unknown")),
            "application/octet-stream"
        );
        assert_eq!(
            FilesystemAdapter::mime_from_extension(Path::new("no_ext")),
            "application/octet-stream"
        );
    }

    // ── Enumerate tests ────────────────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_empty_directory() {
        let tmp = TempDir::new().unwrap();
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_enumerate_finds_all_files() {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path());
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 5);
    }

    #[tokio::test]
    async fn test_enumerate_applies_extension_filter() {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path());
        let spec = FilesystemSourceSpec {
            root: tmp.path().to_path_buf(),
            filters: vec![],
            extensions: vec!["md".to_string()],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("test", &spec).unwrap();
        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 2); // readme.md and sub/nested.md
        assert!(items.iter().all(|i| i.path.ends_with(".md")));
    }

    #[tokio::test]
    async fn test_enumerate_applies_glob_filter() {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path());
        let spec = FilesystemSourceSpec {
            root: tmp.path().to_path_buf(),
            filters: vec![FilterRule {
                pattern: "sub/**".to_string(),
                action: FilterAction::Exclude,
            }],
            extensions: vec![],
            stream: None,
        };
        let adapter = FilesystemAdapter::from_fs_spec("test", &spec).unwrap();
        let items = adapter.enumerate().await.unwrap();
        // Should exclude sub/nested.md and sub/image.png
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| !i.path.starts_with("sub/")));
    }

    #[tokio::test]
    async fn test_enumerate_returns_sorted_results() {
        let tmp = TempDir::new().unwrap();
        create_test_files(tmp.path());
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
    }

    #[tokio::test]
    async fn test_enumerate_source_items_have_correct_fields() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.md"), "# Test").unwrap();
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 1);
        let item = &items[0];
        assert_eq!(item.id, "test.md");
        assert_eq!(item.display_name, "test.md");
        assert_eq!(item.mime_type, "text/markdown");
        assert_eq!(item.path, "test.md");
        assert!(item.modified_at.is_some());
        assert!(item.source_hash.is_none());
    }

    // ── Fetch tests ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_reads_content() {
        let tmp = TempDir::new().unwrap();
        let content = "# Hello World";
        fs::write(tmp.path().join("test.md"), content).unwrap();
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();
        assert_eq!(doc.content, content.as_bytes());
        assert_eq!(doc.id, "test.md");
        assert_eq!(doc.display_name, "test.md");
        assert_eq!(doc.mime_type, "text/markdown");
    }

    #[tokio::test]
    async fn test_fetch_computes_blake3_hash() {
        let tmp = TempDir::new().unwrap();
        let content = b"hash me";
        fs::write(tmp.path().join("test.txt"), content).unwrap();
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();
        let expected = blake3::hash(content).to_hex().to_string();
        assert_eq!(doc.content_hash.as_str(), expected);
    }

    #[tokio::test]
    async fn test_fetch_includes_provenance() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("test.md"), "content").unwrap();
        let adapter = make_adapter(tmp.path());
        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();
        assert_eq!(doc.provenance.source_kind, "filesystem");
        assert!(doc.provenance.metadata.contains_key("path"));
        assert!(doc.provenance.source_modified.is_some());
    }

    #[tokio::test]
    async fn test_fetch_not_found_returns_error() {
        let tmp = TempDir::new().unwrap();
        let adapter = make_adapter(tmp.path());
        let fake_item = SourceItem {
            id: "nonexistent.txt".to_string(),
            display_name: "nonexistent.txt".to_string(),
            mime_type: "text/plain".to_string(),
            path: "nonexistent.txt".to_string(),
            modified_at: None,
            source_hash: None,
        };
        let result = adapter.fetch(&fake_item).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SourceError::NotFound { .. }));
    }

    // ── Relative path tests ────────────────────────────────────────

    #[test]
    fn test_relative_path() {
        let adapter = FilesystemAdapter {
            root: PathBuf::from("/data/root"),
            extensions: vec![],
            filters: vec![],
            source_name: "test".to_string(),
        };
        assert_eq!(
            adapter.relative_path(Path::new("/data/root/sub/file.md")),
            "sub/file.md"
        );
        assert_eq!(
            adapter.relative_path(Path::new("/data/root/file.md")),
            "file.md"
        );
    }

    // ── Compile filters test ───────────────────────────────────────

    #[test]
    fn test_compile_filters_valid_patterns() {
        let rules = vec![
            FilterRule {
                pattern: "**/*.md".to_string(),
                action: FilterAction::Include,
            },
            FilterRule {
                pattern: "Archive/**".to_string(),
                action: FilterAction::Exclude,
            },
        ];
        let compiled = compile_filters(&rules).unwrap();
        assert_eq!(compiled.len(), 2);
    }

    #[test]
    fn test_compile_filters_invalid_pattern() {
        let rules = vec![FilterRule {
            pattern: "[bad".to_string(),
            action: FilterAction::Include,
        }];
        assert!(compile_filters(&rules).is_err());
    }
}
