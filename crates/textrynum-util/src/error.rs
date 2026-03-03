//! Error types for the textyl utility.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during textyl operations.
#[derive(Error, Debug)]
pub enum TextylError {
    #[error("workspace root not found (searched upward from {start_dir})")]
    WorkspaceNotFound { start_dir: PathBuf },

    #[error("failed to read {path}: {source}")]
    FileRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to write {path}: {source}")]
    FileWrite {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse TOML in {path}: {source}")]
    TomlParse {
        path: PathBuf,
        source: toml_edit::TomlError,
    },

    #[error("crate not found in workspace: {name}")]
    CrateNotFound { name: String },

    #[error("dependency {dep} not found in {crate_name}")]
    DepNotFound { crate_name: String, dep: String },

    #[error("version mismatches found ({count} dependencies need updating)")]
    VersionMismatches { count: usize },
}
