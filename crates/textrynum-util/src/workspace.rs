//! Workspace scanning: find root, read version, enumerate crates.

use crate::crate_info::{CrateInfo, DepInfo};
use crate::error::TextylError;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

/// Check if the given directory's Cargo.toml has a `[workspace]` table.
pub fn is_workspace(root: &Path) -> Result<bool> {
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
        path: cargo_toml.clone(),
        source: e,
    })?;
    let doc: toml::Value = toml::from_str(&content).context("failed to parse Cargo.toml")?;
    Ok(doc.get("workspace").is_some())
}

/// Discover the nearest project root by walking up from `start` looking for any `Cargo.toml`.
/// Returns the directory containing the first Cargo.toml found (workspace or single crate).
pub fn find_project_root(start: &Path) -> Result<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(TextylError::WorkspaceNotFound {
                start_dir: start.to_path_buf(),
            }
            .into());
        }
    }
}

/// Scan a single Cargo.toml for dependencies whose names match any of the given prefixes.
/// Returns `(section, dep_name)` pairs.
pub fn scan_matching_deps(
    cargo_toml_path: &Path,
    prefixes: &[&str],
) -> Result<Vec<(String, String)>> {
    let content = std::fs::read_to_string(cargo_toml_path).map_err(|e| TextylError::FileRead {
        path: cargo_toml_path.to_path_buf(),
        source: e,
    })?;
    let doc: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", cargo_toml_path.display()))?;

    let mut matches = Vec::new();
    for section in &DEP_SECTIONS {
        if let Some(deps) = doc.get(section).and_then(|d| d.as_table()) {
            for dep_name in deps.keys() {
                if prefixes.iter().any(|prefix| dep_name.starts_with(prefix)) {
                    matches.push((section.to_string(), dep_name.clone()));
                }
            }
        }
    }
    Ok(matches)
}

/// Discover workspace root by walking up from `start` looking for a
/// `Cargo.toml` that contains a `[workspace]` table.
#[allow(dead_code)]
pub fn find_workspace_root(start: &Path) -> Result<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content =
                std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
                    path: cargo_toml.clone(),
                    source: e,
                })?;
            let doc: toml::Value =
                toml::from_str(&content).context("failed to parse Cargo.toml")?;
            if doc.get("workspace").is_some() {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            return Err(TextylError::WorkspaceNotFound {
                start_dir: start.to_path_buf(),
            }
            .into());
        }
    }
}

/// Read `[workspace.package.version]` from the root Cargo.toml.
pub fn read_workspace_version(root: &Path) -> Result<String> {
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
        path: cargo_toml.clone(),
        source: e,
    })?;
    let doc: toml::Value = toml::from_str(&content).context("failed to parse root Cargo.toml")?;
    let version = doc
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .context("workspace.package.version not found in root Cargo.toml")?;
    Ok(version.to_string())
}

/// Enumerate all workspace member crate directory paths (absolute).
pub fn list_member_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let cargo_toml = root.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
        path: cargo_toml.clone(),
        source: e,
    })?;
    let doc: toml::Value = toml::from_str(&content).context("failed to parse root Cargo.toml")?;
    let members = doc
        .get("workspace")
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
        .context("workspace.members not found in root Cargo.toml")?;

    let mut paths = Vec::new();
    for member in members {
        if let Some(m) = member.as_str() {
            paths.push(root.join(m));
        }
    }
    Ok(paths)
}

/// Collect all internal crate names from the workspace.
pub fn collect_internal_crate_names(root: &Path) -> Result<HashSet<String>> {
    let member_paths = list_member_paths(root)?;
    let mut names = HashSet::new();
    for member_path in &member_paths {
        let cargo_toml = member_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }
        let content = std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
            path: cargo_toml.clone(),
            source: e,
        })?;
        let doc: toml::Value = toml::from_str(&content)
            .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;
        if let Some(name) = doc
            .get("package")
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
        {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}

const DEP_SECTIONS: [&str; 3] = ["dependencies", "dev-dependencies", "build-dependencies"];

/// Scan a single crate's Cargo.toml and extract internal dependency info.
fn scan_crate(
    crate_path: &Path,
    workspace_version: &str,
    internal_names: &HashSet<String>,
) -> Result<CrateInfo> {
    let cargo_toml = crate_path.join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml).map_err(|e| TextylError::FileRead {
        path: cargo_toml.clone(),
        source: e,
    })?;
    let doc: toml::Value = toml::from_str(&content)
        .with_context(|| format!("failed to parse {}", cargo_toml.display()))?;

    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .context("package.name not found")?
        .to_string();

    // Resolve version: check for workspace inheritance or explicit version.
    let version = resolve_crate_version(&doc, workspace_version);

    // Check publish status: defaults to true unless explicitly `publish = false`.
    let publish = doc
        .get("package")
        .and_then(|p| p.get("publish"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut internal_deps = Vec::new();

    for section in &DEP_SECTIONS {
        if let Some(deps) = doc.get(section).and_then(|d| d.as_table()) {
            for (dep_name, dep_value) in deps {
                // Only include deps that have a path key and are known workspace members.
                if let Some(table) = dep_value.as_table() {
                    if table.contains_key("path") && internal_names.contains(dep_name) {
                        let declared_version = table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let path = table
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let optional = table
                            .get("optional")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        internal_deps.push(DepInfo {
                            name: dep_name.clone(),
                            declared_version,
                            path,
                            section: section.to_string(),
                            optional,
                        });
                    }
                }
            }
        }
    }

    // Compute relative path from workspace root.
    let rel_path = crate_path
        .file_name()
        .map(PathBuf::from)
        .unwrap_or_else(|| crate_path.to_path_buf());

    Ok(CrateInfo {
        name,
        path: rel_path,
        version,
        publish,
        internal_deps,
    })
}

/// Resolve a crate's version, handling workspace inheritance.
fn resolve_crate_version(doc: &toml::Value, workspace_version: &str) -> String {
    let pkg = match doc.get("package") {
        Some(p) => p,
        None => return String::new(),
    };

    // Check for explicit version string.
    if let Some(v) = pkg.get("version").and_then(|v| v.as_str()) {
        return v.to_string();
    }

    // Check for version.workspace = true (parsed as a table with workspace key).
    if let Some(v) = pkg.get("version").and_then(|v| v.as_table()) {
        if v.get("workspace")
            .and_then(|w| w.as_bool())
            .unwrap_or(false)
        {
            return workspace_version.to_string();
        }
    }

    workspace_version.to_string()
}

/// Scan all workspace crates, returning info for each.
pub fn scan_all_crates(root: &Path) -> Result<Vec<CrateInfo>> {
    let workspace_version = read_workspace_version(root)?;
    let internal_names = collect_internal_crate_names(root)?;
    let member_paths = list_member_paths(root)?;

    let mut crates = Vec::new();
    for member_path in &member_paths {
        let info = scan_crate(member_path, &workspace_version, &internal_names)?;
        crates.push(info);
    }
    Ok(crates)
}

/// Return publishable crates in dependency order (topological sort).
///
/// Crates with `publish = false` are excluded. Among crates at the same
/// dependency level, names are sorted alphabetically for determinism.
pub fn publish_order(root: &Path) -> Result<Vec<String>> {
    let crates = scan_all_crates(root)?;

    // Filter to publishable crates only.
    let publishable: HashSet<String> = crates
        .iter()
        .filter(|c| c.publish)
        .map(|c| c.name.clone())
        .collect();

    // Build adjacency list: edges go from dependency -> dependent.
    // in_degree counts how many internal deps each crate has.
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut dependents: HashMap<String, Vec<String>> = HashMap::new();

    for info in &crates {
        if !publishable.contains(&info.name) {
            continue;
        }
        in_degree.entry(info.name.clone()).or_insert(0);
        for dep in &info.internal_deps {
            if publishable.contains(&dep.name) {
                *in_degree.entry(info.name.clone()).or_insert(0) += 1;
                dependents
                    .entry(dep.name.clone())
                    .or_default()
                    .push(info.name.clone());
            }
        }
    }

    // Kahn's algorithm with sorted queue for deterministic output.
    let mut queue: VecDeque<String> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(name, _)| name.clone())
        .collect();
    // Sort initial queue for determinism.
    let mut sorted: Vec<String> = queue.drain(..).collect();
    sorted.sort();
    queue.extend(sorted);

    let mut order = Vec::new();
    while let Some(name) = queue.pop_front() {
        order.push(name.clone());
        if let Some(deps) = dependents.get(&name) {
            let mut ready = Vec::new();
            for dep in deps {
                if let Some(deg) = in_degree.get_mut(dep) {
                    *deg -= 1;
                    if *deg == 0 {
                        ready.push(dep.clone());
                    }
                }
            }
            // Sort newly ready crates for determinism.
            ready.sort();
            queue.extend(ready);
        }
    }

    Ok(order)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_workspace(dir: &Path) {
        fs::write(
            dir.join("Cargo.toml"),
            r#"[workspace]
resolver = "2"
members = ["crates/alpha", "crates/beta", "crates/gamma", "crates/tool"]

[workspace.package]
version = "1.2.3"
edition = "2021"
"#,
        )
        .expect("failed to write root Cargo.toml");

        let alpha_dir = dir.join("crates/alpha");
        fs::create_dir_all(&alpha_dir).expect("failed to create alpha dir");
        fs::write(
            alpha_dir.join("Cargo.toml"),
            r#"[package]
name = "alpha"
version.workspace = true
edition.workspace = true
"#,
        )
        .expect("failed to write alpha Cargo.toml");

        let beta_dir = dir.join("crates/beta");
        fs::create_dir_all(&beta_dir).expect("failed to create beta dir");
        fs::write(
            beta_dir.join("Cargo.toml"),
            r#"[package]
name = "beta"
version.workspace = true
edition.workspace = true

[dependencies]
alpha = { version = "1.0.0", path = "../alpha" }

[dev-dependencies]
alpha = { version = "1.0.0", path = "../alpha", optional = false }
"#,
        )
        .expect("failed to write beta Cargo.toml");

        let gamma_dir = dir.join("crates/gamma");
        fs::create_dir_all(&gamma_dir).expect("failed to create gamma dir");
        fs::write(
            gamma_dir.join("Cargo.toml"),
            r#"[package]
name = "gamma"
version.workspace = true
edition.workspace = true

[dependencies]
alpha = { version = "1.0.0", path = "../alpha" }
beta = { version = "1.0.0", path = "../beta" }
"#,
        )
        .expect("failed to write gamma Cargo.toml");

        let tool_dir = dir.join("crates/tool");
        fs::create_dir_all(&tool_dir).expect("failed to create tool dir");
        fs::write(
            tool_dir.join("Cargo.toml"),
            r#"[package]
name = "tool"
version.workspace = true
edition.workspace = true
publish = false
"#,
        )
        .expect("failed to write tool Cargo.toml");
    }

    #[test]
    fn test_is_workspace_returns_true_for_workspace() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());
        assert!(is_workspace(tmp.path()).expect("should check"));
    }

    #[test]
    fn test_is_workspace_returns_false_for_single_crate() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = "0.1.0"
"#,
        )
        .expect("write");
        assert!(!is_workspace(tmp.path()).expect("should check"));
    }

    #[test]
    fn test_find_project_root_finds_single_crate() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        fs::write(
            tmp.path().join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "1.0.0"
"#,
        )
        .expect("write");
        let subdir = tmp.path().join("src");
        fs::create_dir_all(&subdir).expect("mkdir");
        let root = find_project_root(&subdir).expect("should find root");
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn test_find_project_root_finds_workspace() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());
        let subdir = tmp.path().join("crates/alpha");
        let root = find_project_root(&subdir).expect("should find root");
        // find_project_root stops at nearest Cargo.toml, which is the crate itself.
        assert_eq!(root, subdir);
    }

    #[test]
    fn test_scan_matching_deps_finds_prefixed_deps() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = "0.1.0"
fabryk-graph = { version = "0.1.0", features = ["serde"] }
serde = "1.0"
ecl-design = "0.1.0"

[dev-dependencies]
fabryk-test = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
"#,
        )
        .expect("write");

        let matches = scan_matching_deps(&cargo_toml, &["fabryk-", "ecl-"]).expect("should scan");
        assert_eq!(matches.len(), 4);

        let dep_names: Vec<&str> = matches.iter().map(|(_, n)| n.as_str()).collect();
        assert!(dep_names.contains(&"fabryk-core"));
        assert!(dep_names.contains(&"fabryk-graph"));
        assert!(dep_names.contains(&"ecl-design"));
        assert!(dep_names.contains(&"fabryk-test"));
        assert!(!dep_names.contains(&"serde"));
        assert!(!dep_names.contains(&"tokio"));
    }

    #[test]
    fn test_scan_matching_deps_no_matches_returns_empty() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        let cargo_toml = tmp.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#,
        )
        .expect("write");

        let matches = scan_matching_deps(&cargo_toml, &["fabryk-", "ecl-"]).expect("should scan");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_find_workspace_root_from_subdir_finds_root() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());
        let subdir = tmp.path().join("crates/alpha");

        let root = find_workspace_root(&subdir).expect("should find root");
        assert_eq!(root, tmp.path());
    }

    #[test]
    fn test_find_workspace_root_not_found_returns_error() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        let result = find_workspace_root(tmp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_workspace_version_returns_correct_version() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let version = read_workspace_version(tmp.path()).expect("should read version");
        assert_eq!(version, "1.2.3");
    }

    #[test]
    fn test_list_member_paths_returns_all_members() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let paths = list_member_paths(tmp.path()).expect("should list members");
        assert_eq!(paths.len(), 4);
        assert!(paths[0].ends_with("crates/alpha"));
        assert!(paths[1].ends_with("crates/beta"));
    }

    #[test]
    fn test_collect_internal_crate_names_returns_all_names() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let names = collect_internal_crate_names(tmp.path()).expect("should collect names");
        assert_eq!(names.len(), 4);
        assert!(names.contains("alpha"));
        assert!(names.contains("beta"));
        assert!(names.contains("gamma"));
        assert!(names.contains("tool"));
    }

    #[test]
    fn test_scan_all_crates_detects_internal_deps() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let crates = scan_all_crates(tmp.path()).expect("should scan crates");
        assert_eq!(crates.len(), 4);

        let alpha = crates.iter().find(|c| c.name == "alpha").expect("alpha");
        assert!(alpha.internal_deps.is_empty());
        assert_eq!(alpha.version, "1.2.3");
        assert!(alpha.publish);

        let beta = crates.iter().find(|c| c.name == "beta").expect("beta");
        assert_eq!(beta.version, "1.2.3");
        assert!(!beta.internal_deps.is_empty());

        let dep = beta
            .internal_deps
            .iter()
            .find(|d| d.section == "dependencies")
            .expect("should have dep");
        assert_eq!(dep.name, "alpha");
        assert_eq!(dep.declared_version, "1.0.0");
        assert_eq!(dep.section, "dependencies");

        let tool = crates.iter().find(|c| c.name == "tool").expect("tool");
        assert!(!tool.publish);
    }

    #[test]
    fn test_publish_order_excludes_unpublishable_crates() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let order = publish_order(tmp.path()).expect("should get order");
        assert!(!order.contains(&"tool".to_string()));
    }

    #[test]
    fn test_publish_order_respects_dependency_order() {
        let tmp = TempDir::new().expect("failed to create tempdir");
        create_workspace(tmp.path());

        let order = publish_order(tmp.path()).expect("should get order");
        assert_eq!(order.len(), 3);

        let alpha_pos = order.iter().position(|n| n == "alpha").expect("alpha");
        let beta_pos = order.iter().position(|n| n == "beta").expect("beta");
        let gamma_pos = order.iter().position(|n| n == "gamma").expect("gamma");

        // alpha must come before beta and gamma
        assert!(alpha_pos < beta_pos);
        assert!(alpha_pos < gamma_pos);
        // beta must come before gamma (gamma depends on both alpha and beta)
        assert!(beta_pos < gamma_pos);
    }
}
