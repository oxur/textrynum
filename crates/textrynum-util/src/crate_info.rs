//! Data types representing workspace crates and their dependencies.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Information about one crate in the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Crate name (e.g., "fabryk-graph").
    pub name: String,
    /// Path to the crate directory relative to workspace root.
    pub path: PathBuf,
    /// The crate's own version (resolved from workspace or explicit).
    pub version: String,
    /// Whether this crate is publishable (not `publish = false`).
    pub publish: bool,
    /// Internal (in-workspace) dependencies with their declared versions.
    pub internal_deps: Vec<DepInfo>,
}

/// A single internal dependency reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepInfo {
    /// Dependency name (e.g., "fabryk-core").
    pub name: String,
    /// Version string declared in Cargo.toml (e.g., "0.1.0").
    pub declared_version: String,
    /// The path value from the dependency specification.
    pub path: String,
    /// TOML section: "dependencies", "dev-dependencies", or "build-dependencies".
    pub section: String,
    /// Whether the dep is optional.
    pub optional: bool,
}

/// A version mismatch detected during checking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionMismatch {
    /// Crate containing the mismatched dependency.
    pub crate_name: String,
    /// The dependency with the wrong version.
    pub dep_name: String,
    /// The version currently declared.
    pub declared_version: String,
    /// The version it should be.
    pub expected_version: String,
    /// Which TOML section the dep is in.
    pub section: String,
}
