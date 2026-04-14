//! Pre-built cache distribution for Fabryk-based applications.
//!
//! Manages download, packaging, and status reporting for three backend caches
//! (graph, FTS, vector) distributed as GitHub Release assets. Each cache is
//! packaged as a `.tar.gz` archive with a `.sha256` sidecar for integrity
//! verification.
//!
//! # Configuration
//!
//! Functions that generate URLs or archive names accept a [`CacheProject`]
//! struct with the project-specific prefix and release base URL. This allows
//! each downstream application to provide its own GitHub release coordinates.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;

use fabryk_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Filename for the cache manifest stored alongside downloaded caches.
const MANIFEST_FILENAME: &str = "cache-manifest.json";

// ---------------------------------------------------------------------------
// Project configuration
// ---------------------------------------------------------------------------

/// Project-specific configuration for cache archive naming and URLs.
///
/// Each downstream application provides its own values (e.g., project prefix
/// `"music-theory"`, release base URL pointing to its GitHub releases).
#[derive(Debug, Clone)]
pub struct CacheProject {
    /// Project prefix for archive names (e.g., `"music-theory"`).
    pub prefix: String,
    /// Base URL for GitHub Release assets (e.g.,
    /// `"https://github.com/oxur/ai-music-theory/releases/download"`).
    pub release_base_url: String,
}

// ---------------------------------------------------------------------------
// Backend paths
// ---------------------------------------------------------------------------

/// Paths to check for cache presence on disk.
///
/// Each downstream project provides its own paths based on its cache layout.
pub struct BackendPaths {
    /// Path to check for graph cache presence (e.g., `base/data/graphs/concept_graph.json`).
    pub graph: PathBuf,
    /// Path to check for FTS cache presence (e.g., `base/.tantivy-index`).
    pub fts: PathBuf,
    /// Path to check for vector cache presence (e.g., `base/.cache/vector/vector-cache.json`).
    pub vector: PathBuf,
}

/// Paths to include when packaging a cache backend.
pub struct PackagePaths {
    /// Relative paths to include for graph backend packaging.
    pub graph: Vec<String>,
    /// Relative paths to include for FTS backend packaging.
    pub fts: Vec<String>,
    /// Tar extra args for FTS (e.g., `["--exclude=*.lock"]`).
    pub fts_excludes: Vec<String>,
    /// Relative paths to include for vector backend packaging.
    pub vector: Vec<String>,
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Identifies one of the three cache backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheBackend {
    /// Knowledge graph.
    Graph,
    /// Full-text search (Tantivy index).
    Fts,
    /// Vector embeddings.
    Vector,
}

impl fmt::Display for CacheBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph => write!(f, "graph"),
            Self::Fts => write!(f, "fts"),
            Self::Vector => write!(f, "vector"),
        }
    }
}

impl FromStr for CacheBackend {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "graph" => Ok(Self::Graph),
            "fts" => Ok(Self::Fts),
            "vector" => Ok(Self::Vector),
            other => Err(Error::operation(format!(
                "Unknown cache backend: '{other}'. Expected one of: graph, fts, vector"
            ))),
        }
    }
}

/// Persistent record of which caches have been downloaded and their versions.
///
/// Stored as `cache-manifest.json` in the base data directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph: Option<CacheEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fts: Option<CacheEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector: Option<CacheEntry>,
}

impl CacheManifest {
    /// Return the entry for a given backend, if present.
    pub fn get(&self, backend: &CacheBackend) -> Option<&CacheEntry> {
        match backend {
            CacheBackend::Graph => self.graph.as_ref(),
            CacheBackend::Fts => self.fts.as_ref(),
            CacheBackend::Vector => self.vector.as_ref(),
        }
    }

    /// Set the entry for a given backend.
    pub fn set(&mut self, backend: &CacheBackend, entry: CacheEntry) {
        match backend {
            CacheBackend::Graph => self.graph = Some(entry),
            CacheBackend::Fts => self.fts = Some(entry),
            CacheBackend::Vector => self.vector = Some(entry),
        }
    }
}

/// Metadata for a single downloaded cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Server version the cache was built for.
    pub version: String,
    /// ISO 8601 timestamp of when the cache was downloaded.
    pub downloaded_at: String,
    /// SHA-256 checksum of the archive that was verified at download time.
    pub checksum: String,
}

/// Per-backend status information.
#[derive(Debug, Clone)]
pub struct BackendStatus {
    pub backend: CacheBackend,
    pub installed_version: Option<String>,
    pub files_present: bool,
}

/// Aggregated status report across all three cache backends.
#[derive(Debug, Clone)]
pub struct CacheStatusReport {
    pub graph: BackendStatus,
    pub fts: BackendStatus,
    pub vector: BackendStatus,
}

impl fmt::Display for CacheStatusReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Cache Status:")?;
        for status in [&self.graph, &self.fts, &self.vector] {
            let state = if status.files_present {
                match &status.installed_version {
                    Some(v) => format!("installed (v{v})"),
                    None => "files present (not tracked)".to_string(),
                }
            } else {
                match &status.installed_version {
                    Some(v) => format!("manifest says v{v}, but files missing"),
                    None => "not installed".to_string(),
                }
            };
            writeln!(f, "  {}: {}", status.backend, state)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// URL / naming helpers
// ---------------------------------------------------------------------------

/// Return the archive filename for a backend at a given version.
///
/// Format: `{project_prefix}-cache-{backend}-v{version}.tar.gz`
pub fn archive_name(backend: &CacheBackend, version: &str, project: &CacheProject) -> String {
    format!("{}-cache-{backend}-v{version}.tar.gz", project.prefix)
}

/// Return the full GitHub Release download URL for a cache archive.
pub fn release_url(backend: &CacheBackend, version: &str, project: &CacheProject) -> String {
    let name = archive_name(backend, version, project);
    format!("{}/v{version}/{name}", project.release_base_url)
}

/// Return the URL for the `.sha256` sidecar checksum file.
pub fn checksum_url(backend: &CacheBackend, version: &str, project: &CacheProject) -> String {
    format!("{}.sha256", release_url(backend, version, project))
}

// ---------------------------------------------------------------------------
// Manifest persistence
// ---------------------------------------------------------------------------

/// Load the cache manifest from `{base_path}/cache-manifest.json`.
///
/// Returns an empty default manifest if the file does not exist.
pub fn load_manifest(base_path: &Path) -> Result<CacheManifest> {
    let path = base_path.join(MANIFEST_FILENAME);
    if !path.exists() {
        return Ok(CacheManifest::default());
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| Error::io_with_path(e, &path))?;
    serde_json::from_str(&contents).map_err(|e| {
        Error::operation(format!(
            "Failed to parse cache manifest at {}: {e}",
            path.display()
        ))
    })
}

/// Write the cache manifest as pretty-printed JSON.
pub fn save_manifest(base_path: &Path, manifest: &CacheManifest) -> Result<()> {
    let path = base_path.join(MANIFEST_FILENAME);
    let json = serde_json::to_string_pretty(manifest)
        .map_err(|e| Error::operation(format!("Failed to serialize cache manifest: {e}")))?;
    std::fs::write(&path, json).map_err(|e| Error::io_with_path(e, &path))
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

/// Check the installation status of all three cache backends.
///
/// Uses the provided [`BackendPaths`] to verify that expected files exist on disk.
pub fn cache_status(base_path: &Path, paths: &BackendPaths) -> Result<CacheStatusReport> {
    let manifest = load_manifest(base_path)?;

    Ok(CacheStatusReport {
        graph: BackendStatus {
            backend: CacheBackend::Graph,
            installed_version: manifest.graph.as_ref().map(|e| e.version.clone()),
            files_present: paths.graph.exists(),
        },
        fts: BackendStatus {
            backend: CacheBackend::Fts,
            installed_version: manifest.fts.as_ref().map(|e| e.version.clone()),
            files_present: paths.fts.is_dir(),
        },
        vector: BackendStatus {
            backend: CacheBackend::Vector,
            installed_version: manifest.vector.as_ref().map(|e| e.version.clone()),
            files_present: paths.vector.exists(),
        },
    })
}

// ---------------------------------------------------------------------------
// Shell helpers
// ---------------------------------------------------------------------------

/// Download a URL to a local file using `curl`.
pub fn shell_download(url: &str, dest: &Path) -> Result<()> {
    log::info!("Downloading {} ...", url);
    let output = Command::new("curl")
        .args(["-fSL", "-o"])
        .arg(dest)
        .arg(url)
        .output()
        .map_err(|e| Error::operation(format!("Failed to execute curl: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::operation(format!(
            "curl exited with status {}: {stderr}",
            output.status
        )));
    }
    Ok(())
}

/// Verify the SHA-256 checksum of a file.
pub fn verify_checksum(archive: &Path, expected_hash: &str) -> Result<()> {
    log::debug!("Verifying checksum of {} ...", archive.display());
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(archive)
        .output()
        .map_err(|e| Error::operation(format!("Failed to execute shasum: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::operation(format!(
            "shasum exited with status {}: {stderr}",
            output.status
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual_hash = stdout.split_whitespace().next().unwrap_or("").trim();

    if actual_hash != expected_hash.trim() {
        return Err(Error::operation(format!(
            "Checksum mismatch for {}: expected {}, got {actual_hash}",
            archive.display(),
            expected_hash.trim()
        )));
    }
    log::debug!("Checksum verified: {actual_hash}");
    Ok(())
}

/// Extract a `.tar.gz` archive into a target directory.
pub fn extract_archive(archive: &Path, target_dir: &Path) -> Result<()> {
    log::info!(
        "Extracting {} to {} ...",
        archive.display(),
        target_dir.display()
    );
    let output = Command::new("tar")
        .args(["-xzf"])
        .arg(archive)
        .arg("-C")
        .arg(target_dir)
        .output()
        .map_err(|e| Error::operation(format!("Failed to execute tar: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::operation(format!(
            "tar exited with status {}: {stderr}",
            output.status
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Download orchestration
// ---------------------------------------------------------------------------

/// Download and install a pre-built cache for the given backend.
///
/// Skips the download if the manifest already records an installation at the
/// given version, unless `force` is `true`.
pub fn download_cache(
    backend: &CacheBackend,
    base_path: &Path,
    version: &str,
    project: &CacheProject,
    force: bool,
) -> Result<()> {
    let mut manifest = load_manifest(base_path)?;
    if !force {
        if let Some(entry) = manifest.get(backend) {
            if entry.version == version {
                log::info!(
                    "{} cache v{version} already installed, skipping (use --force to re-download)",
                    backend
                );
                return Ok(());
            }
        }
    }

    log::info!("Installing {backend} cache v{version} ...");

    let archive_file = base_path.join(format!(".cache-download-{backend}.tar.gz"));
    let checksum_file = base_path.join(format!(".cache-download-{backend}.sha256"));

    let archive_url = release_url(backend, version, project);
    let cs_url = checksum_url(backend, version, project);

    shell_download(&archive_url, &archive_file)?;
    shell_download(&cs_url, &checksum_file)?;

    let expected_hash = std::fs::read_to_string(&checksum_file)
        .map_err(|e| Error::io_with_path(e, &checksum_file))?;
    let expected_hash = expected_hash.split_whitespace().next().unwrap_or("").trim();

    verify_checksum(&archive_file, expected_hash)?;
    extract_archive(&archive_file, base_path)?;

    manifest.set(
        backend,
        CacheEntry {
            version: version.to_string(),
            downloaded_at: fabryk_core::util::time::iso8601_now(),
            checksum: expected_hash.to_string(),
        },
    );
    save_manifest(base_path, &manifest)?;

    let _ = std::fs::remove_file(&archive_file);
    let _ = std::fs::remove_file(&checksum_file);

    log::info!("{backend} cache v{version} installed successfully.");
    Ok(())
}

// ---------------------------------------------------------------------------
// Packaging
// ---------------------------------------------------------------------------

/// Create a distributable `.tar.gz` archive for a cache backend.
///
/// Uses the [`PackagePaths`] to determine which files to include per backend.
pub fn package_cache(
    backend: &CacheBackend,
    base_path: &Path,
    output_dir: &Path,
    version: &str,
    project: &CacheProject,
    paths: &PackagePaths,
) -> Result<PathBuf> {
    std::fs::create_dir_all(output_dir).map_err(|e| Error::io_with_path(e, output_dir))?;

    let name = archive_name(backend, version, project);
    let output_path = output_dir.join(&name);

    log::info!("Packaging {backend} cache as {name} ...");

    let mut cmd = Command::new("tar");
    cmd.arg("-czf").arg(&output_path).arg("-C").arg(base_path);

    let (file_args, excludes) = match backend {
        CacheBackend::Graph => (&paths.graph, &vec![]),
        CacheBackend::Fts => (&paths.fts, &paths.fts_excludes),
        CacheBackend::Vector => (&paths.vector, &vec![]),
    };

    for exclude in excludes {
        cmd.arg(format!("--exclude={exclude}"));
    }
    for arg in file_args {
        cmd.arg(arg);
    }

    let output = cmd
        .output()
        .map_err(|e| Error::operation(format!("Failed to execute tar: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::operation(format!(
            "tar exited with status {}: {stderr}",
            output.status
        )));
    }

    log::info!("Created {}", output_path.display());
    Ok(output_path)
}

// ---------------------------------------------------------------------------
// Argument parsing helper
// ---------------------------------------------------------------------------

/// Parse a backend argument string into one or more [`CacheBackend`] values.
///
/// Accepts `"all"` (case-insensitive) to return all three backends, or a
/// single backend name such as `"graph"`, `"fts"`, or `"vector"`.
pub fn parse_backend_arg(arg: &str) -> Result<Vec<CacheBackend>> {
    if arg.eq_ignore_ascii_case("all") {
        return Ok(vec![
            CacheBackend::Graph,
            CacheBackend::Fts,
            CacheBackend::Vector,
        ]);
    }
    let backend = CacheBackend::from_str(arg)?;
    Ok(vec![backend])
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_project() -> CacheProject {
        CacheProject {
            prefix: "test-project".to_string(),
            release_base_url: "https://github.com/test/repo/releases/download".to_string(),
        }
    }

    #[test]
    fn test_archive_name() {
        let project = test_project();
        assert_eq!(
            archive_name(&CacheBackend::Graph, "1.2.3", &project),
            "test-project-cache-graph-v1.2.3.tar.gz"
        );
        assert_eq!(
            archive_name(&CacheBackend::Fts, "0.1.0", &project),
            "test-project-cache-fts-v0.1.0.tar.gz"
        );
    }

    #[test]
    fn test_release_url() {
        let project = test_project();
        let url = release_url(&CacheBackend::Graph, "1.0.0", &project);
        assert_eq!(
            url,
            "https://github.com/test/repo/releases/download/v1.0.0/\
             test-project-cache-graph-v1.0.0.tar.gz"
        );
    }

    #[test]
    fn test_checksum_url() {
        let project = test_project();
        let url = checksum_url(&CacheBackend::Fts, "1.0.0", &project);
        assert!(url.ends_with(".tar.gz.sha256"));
    }

    #[test]
    fn test_cache_backend_display() {
        assert_eq!(CacheBackend::Graph.to_string(), "graph");
        assert_eq!(CacheBackend::Fts.to_string(), "fts");
        assert_eq!(CacheBackend::Vector.to_string(), "vector");
    }

    #[test]
    fn test_cache_backend_from_str() {
        assert_eq!(CacheBackend::from_str("graph").unwrap(), CacheBackend::Graph);
        assert_eq!(CacheBackend::from_str("fts").unwrap(), CacheBackend::Fts);
        assert_eq!(CacheBackend::from_str("GRAPH").unwrap(), CacheBackend::Graph);
        assert!(CacheBackend::from_str("unknown").is_err());
    }

    #[test]
    fn test_parse_backend_arg_all() {
        let backends = parse_backend_arg("all").unwrap();
        assert_eq!(backends.len(), 3);
        let backends = parse_backend_arg("ALL").unwrap();
        assert_eq!(backends.len(), 3);
    }

    #[test]
    fn test_parse_backend_arg_single() {
        let backends = parse_backend_arg("graph").unwrap();
        assert_eq!(backends, vec![CacheBackend::Graph]);
    }

    #[test]
    fn test_parse_backend_arg_invalid() {
        assert!(parse_backend_arg("nope").is_err());
    }

    #[test]
    fn test_manifest_roundtrip() {
        let manifest = CacheManifest {
            graph: Some(CacheEntry {
                version: "1.0.0".to_string(),
                downloaded_at: "2025-01-15T10:30:00Z".to_string(),
                checksum: "abc123".to_string(),
            }),
            fts: None,
            vector: Some(CacheEntry {
                version: "1.0.0".to_string(),
                downloaded_at: "2025-01-15T11:00:00Z".to_string(),
                checksum: "def456".to_string(),
            }),
        };

        let json = serde_json::to_string_pretty(&manifest).unwrap();
        assert!(!json.contains("\"fts\""));

        let parsed: CacheManifest = serde_json::from_str(&json).unwrap();
        assert!(parsed.graph.is_some());
        assert!(parsed.fts.is_none());
        assert!(parsed.vector.is_some());
    }

    #[test]
    fn test_manifest_load_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = load_manifest(dir.path()).unwrap();
        assert!(manifest.graph.is_none());
    }

    #[test]
    fn test_manifest_save_and_load() {
        let dir = tempfile::tempdir().unwrap();

        let manifest = CacheManifest {
            graph: Some(CacheEntry {
                version: "2.0.0".to_string(),
                downloaded_at: "2025-06-01T00:00:00Z".to_string(),
                checksum: "aabbcc".to_string(),
            }),
            fts: None,
            vector: None,
        };

        save_manifest(dir.path(), &manifest).unwrap();
        let loaded = load_manifest(dir.path()).unwrap();
        assert_eq!(loaded.graph.as_ref().unwrap().version, "2.0.0");
    }

    #[test]
    fn test_manifest_get_set() {
        let mut manifest = CacheManifest::default();
        assert!(manifest.get(&CacheBackend::Graph).is_none());

        let entry = CacheEntry {
            version: "1.0.0".to_string(),
            downloaded_at: "2025-01-01T00:00:00Z".to_string(),
            checksum: "abc".to_string(),
        };

        manifest.set(&CacheBackend::Graph, entry.clone());
        assert!(manifest.get(&CacheBackend::Graph).is_some());

        manifest.set(&CacheBackend::Fts, entry.clone());
        assert!(manifest.get(&CacheBackend::Fts).is_some());

        manifest.set(&CacheBackend::Vector, entry);
        assert!(manifest.get(&CacheBackend::Vector).is_some());
    }

    #[test]
    fn test_cache_status_empty() {
        let dir = tempfile::tempdir().unwrap();
        let paths = BackendPaths {
            graph: dir.path().join("nonexistent_graph"),
            fts: dir.path().join("nonexistent_fts"),
            vector: dir.path().join("nonexistent_vector"),
        };

        let report = cache_status(dir.path(), &paths).unwrap();
        assert!(!report.graph.files_present);
        assert!(!report.fts.files_present);
        assert!(!report.vector.files_present);

        let display = format!("{report}");
        assert!(display.contains("not installed"));
    }

    #[test]
    fn test_cache_status_report_display() {
        let report = CacheStatusReport {
            graph: BackendStatus {
                backend: CacheBackend::Graph,
                installed_version: Some("1.0.0".to_string()),
                files_present: true,
            },
            fts: BackendStatus {
                backend: CacheBackend::Fts,
                installed_version: None,
                files_present: false,
            },
            vector: BackendStatus {
                backend: CacheBackend::Vector,
                installed_version: Some("1.0.0".to_string()),
                files_present: false,
            },
        };

        let display = format!("{report}");
        assert!(display.contains("graph: installed (v1.0.0)"));
        assert!(display.contains("fts: not installed"));
        assert!(display.contains("vector: manifest says v1.0.0, but files missing"));
    }
}
