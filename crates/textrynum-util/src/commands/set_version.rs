//! Handler for `textyl set-version` and `textyl check-versions`.

use crate::crate_info::VersionMismatch;
use crate::editor;
use crate::error::TextylError;
use crate::output;
use crate::workspace;
use anyhow::Result;
use std::path::Path;

/// Name prefixes for dependencies that should be updated in external project mode.
const DEP_PREFIXES: [&str; 2] = ["fabryk-", "ecl-"];

/// Run the set-version command.
///
/// - `project_version`: if Some, update the project's own version.
/// - `deps_version`: if Some, update all fabryk-*/ecl-* dependencies.
/// - `check`: if true, only report mismatches without modifying files.
pub fn run(
    root: &Path,
    project_version: Option<&str>,
    deps_version: Option<&str>,
    check: bool,
) -> Result<()> {
    let is_workspace = workspace::is_workspace(root)?;

    if check {
        return run_check(root, is_workspace, project_version, deps_version);
    }

    run_update(root, is_workspace, project_version, deps_version)
}

/// Check mode: report mismatches and exit non-zero if any found.
fn run_check(
    root: &Path,
    is_workspace: bool,
    project_version: Option<&str>,
    deps_version: Option<&str>,
) -> Result<()> {
    let mut mismatches = Vec::new();

    // Check internal path deps (workspace mode only, when project_version is set).
    if is_workspace {
        if let Some(pv) = project_version {
            let crates = workspace::scan_all_crates(root)?;
            mismatches.extend(collect_internal_mismatches(&crates, pv));
        }
    }

    // Check fabryk-*/ecl-* deps.
    if let Some(dv) = deps_version {
        let prefixes: Vec<&str> = DEP_PREFIXES.to_vec();
        mismatches.extend(collect_dep_mismatches(root, is_workspace, &prefixes, dv)?);
    }

    if mismatches.is_empty() {
        let version_desc = match (project_version, deps_version) {
            (Some(pv), Some(dv)) if pv == dv => format!("\"{pv}\""),
            (Some(pv), Some(dv)) => format!("project \"{pv}\", deps \"{dv}\""),
            (Some(pv), None) => format!("\"{pv}\""),
            (None, Some(dv)) => format!("deps \"{dv}\""),
            (None, None) => unreachable!(),
        };
        println!("All versions match {version_desc}.");
        return Ok(());
    }

    eprintln!(
        "Found {} version mismatch(es):\n{}",
        mismatches.len(),
        output::format_mismatches(&mismatches)
    );

    Err(TextylError::VersionMismatches {
        count: mismatches.len(),
    }
    .into())
}

/// Update mode: set versions and sync dependencies.
fn run_update(
    root: &Path,
    is_workspace: bool,
    project_version: Option<&str>,
    deps_version: Option<&str>,
) -> Result<()> {
    let root_cargo_toml = root.join("Cargo.toml");
    let mut total_changes = 0u32;

    // Update project version.
    if let Some(pv) = project_version {
        let changed = if is_workspace {
            editor::update_workspace_version(&root_cargo_toml, pv)?
        } else {
            editor::update_package_version(&root_cargo_toml, pv)?
        };
        if changed {
            println!("Updated project version to \"{pv}\".");
        } else {
            println!("Project version already at \"{pv}\".");
        }
    }

    // Update internal path deps (workspace mode, when project_version is set).
    if is_workspace {
        if let Some(pv) = project_version {
            let crates = workspace::scan_all_crates(root)?;
            let member_paths = workspace::list_member_paths(root)?;

            for crate_info in &crates {
                let crate_path = member_paths.iter().find(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n == crate_info.name || p.ends_with(&crate_info.path))
                        .unwrap_or(false)
                });

                let Some(crate_path) = crate_path else {
                    continue;
                };

                let cargo_toml = crate_path.join("Cargo.toml");

                for dep in &crate_info.internal_deps {
                    let changed =
                        editor::update_dep_version(&cargo_toml, &dep.section, &dep.name, pv)?;
                    if changed {
                        total_changes += 1;
                        println!(
                            "  Updated {} [{}] {} = \"{pv}\"",
                            crate_info.name, dep.section, dep.name
                        );
                    }
                }
            }
        }
    }

    // Update fabryk-*/ecl-* deps.
    if let Some(dv) = deps_version {
        let prefixes: Vec<&str> = DEP_PREFIXES.to_vec();
        total_changes += update_matching_deps(root, is_workspace, &prefixes, dv)?;
    }

    if total_changes == 0 {
        println!("All dependency versions already up to date.");
    } else {
        println!("{total_changes} dependency version(s) updated.");
    }

    Ok(())
}

/// Update all deps matching prefixes across the project.
fn update_matching_deps(
    root: &Path,
    is_workspace: bool,
    prefixes: &[&str],
    new_version: &str,
) -> Result<u32> {
    let mut changes = 0u32;

    if is_workspace {
        let member_paths = workspace::list_member_paths(root)?;
        for member_path in &member_paths {
            let cargo_toml = member_path.join("Cargo.toml");
            changes += update_matching_deps_in_file(&cargo_toml, prefixes, new_version)?;
        }
    } else {
        let cargo_toml = root.join("Cargo.toml");
        changes += update_matching_deps_in_file(&cargo_toml, prefixes, new_version)?;
    }

    Ok(changes)
}

/// Update matching deps in a single Cargo.toml file.
fn update_matching_deps_in_file(
    cargo_toml: &Path,
    prefixes: &[&str],
    new_version: &str,
) -> Result<u32> {
    let matches = workspace::scan_matching_deps(cargo_toml, prefixes)?;
    let mut changes = 0u32;

    // Get crate name for display.
    let crate_name = cargo_toml
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    for (section, dep_name) in &matches {
        let changed = editor::update_any_dep_version(cargo_toml, section, dep_name, new_version)?;
        if changed {
            changes += 1;
            println!("  Updated {crate_name} [{section}] {dep_name} = \"{new_version}\"");
        }
    }

    Ok(changes)
}

/// Collect version mismatches for internal path deps.
fn collect_internal_mismatches(
    crates: &[crate::crate_info::CrateInfo],
    expected_version: &str,
) -> Vec<VersionMismatch> {
    let mut mismatches = Vec::new();
    for crate_info in crates {
        for dep in &crate_info.internal_deps {
            if dep.declared_version != expected_version {
                mismatches.push(VersionMismatch {
                    crate_name: crate_info.name.clone(),
                    dep_name: dep.name.clone(),
                    declared_version: dep.declared_version.clone(),
                    expected_version: expected_version.to_string(),
                    section: dep.section.clone(),
                });
            }
        }
    }
    mismatches
}

/// Collect version mismatches for deps matching prefixes.
fn collect_dep_mismatches(
    root: &Path,
    is_workspace: bool,
    prefixes: &[&str],
    expected_version: &str,
) -> Result<Vec<VersionMismatch>> {
    let mut mismatches = Vec::new();

    let cargo_tomls: Vec<std::path::PathBuf> = if is_workspace {
        workspace::list_member_paths(root)?
            .iter()
            .map(|p| p.join("Cargo.toml"))
            .collect()
    } else {
        vec![root.join("Cargo.toml")]
    };

    for cargo_toml in &cargo_tomls {
        let crate_name = cargo_toml
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let matches = workspace::scan_matching_deps(cargo_toml, prefixes)?;
        for (section, dep_name) in &matches {
            let declared = read_dep_version(cargo_toml, section, dep_name)?;
            if let Some(declared) = declared {
                if declared != expected_version {
                    mismatches.push(VersionMismatch {
                        crate_name: crate_name.to_string(),
                        dep_name: dep_name.clone(),
                        declared_version: declared,
                        expected_version: expected_version.to_string(),
                        section: section.clone(),
                    });
                }
            }
        }
    }

    Ok(mismatches)
}

/// Read the declared version of a specific dep from a Cargo.toml.
fn read_dep_version(cargo_toml: &Path, section: &str, dep_name: &str) -> Result<Option<String>> {
    let content = std::fs::read_to_string(cargo_toml)?;
    let doc: toml::Value = toml::from_str(&content)?;

    let version = doc
        .get(section)
        .and_then(|deps| deps.get(dep_name))
        .and_then(|dep| {
            // Plain string: dep = "1.0"
            if let Some(v) = dep.as_str() {
                return Some(v.to_string());
            }
            // Table: dep = { version = "1.0", ... }
            dep.get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });

    Ok(version)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace(dir: &Path) {
        fs::write(
            dir.join("Cargo.toml"),
            r#"[workspace]
resolver = "2"
members = ["crates/alpha", "crates/beta"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write root");

        let alpha = dir.join("crates/alpha");
        fs::create_dir_all(&alpha).expect("mkdir alpha");
        fs::write(
            alpha.join("Cargo.toml"),
            r#"[package]
name = "alpha"
version.workspace = true
edition.workspace = true
"#,
        )
        .expect("write alpha");

        let beta = dir.join("crates/beta");
        fs::create_dir_all(&beta).expect("mkdir beta");
        fs::write(
            beta.join("Cargo.toml"),
            r#"[package]
name = "beta"
version.workspace = true
edition.workspace = true

[dependencies]
# Core dep
alpha = { version = "0.1.0", path = "../alpha" }
"#,
        )
        .expect("write beta");
    }

    fn create_single_crate(dir: &Path) {
        fs::write(
            dir.join("Cargo.toml"),
            r#"[package]
name = "myapp"
version = "1.0.0"
edition = "2021"

[dependencies]
fabryk-core = "0.1.0"
fabryk-graph = { version = "0.1.0", features = ["serde"] }
serde = "1.0"

[dev-dependencies]
ecl-design = "0.1.0"
"#,
        )
        .expect("write");
    }

    // ---- Workspace mode (backward compat) ----

    #[test]
    fn test_run_check_no_mismatches_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        let result = run(tmp.path(), Some("0.1.0"), None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_check_with_mismatches_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        let result = run(tmp.path(), Some("0.2.0"), None, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_update_sets_workspace_and_dep_versions() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        run(tmp.path(), Some("0.2.0"), None, false).expect("update");

        let root = fs::read_to_string(tmp.path().join("Cargo.toml")).expect("read root");
        assert!(root.contains(r#"version = "0.2.0""#));

        let beta = fs::read_to_string(tmp.path().join("crates/beta/Cargo.toml")).expect("read");
        assert!(beta.contains(r#"version = "0.2.0""#));
        assert!(beta.contains("# Core dep"));
    }

    #[test]
    fn test_run_update_already_at_version_is_noop() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        run(tmp.path(), Some("0.1.0"), None, false).expect("update");

        let beta = fs::read_to_string(tmp.path().join("crates/beta/Cargo.toml")).expect("read");
        assert!(beta.contains(r#"version = "0.1.0""#));
    }

    #[test]
    fn test_check_after_update_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        run(tmp.path(), Some("0.2.0"), None, false).expect("update");

        let result = run(tmp.path(), Some("0.2.0"), None, true);
        assert!(result.is_ok());
    }

    // ---- Single crate mode ----

    #[test]
    fn test_run_single_crate_updates_project_version() {
        let tmp = TempDir::new().expect("tempdir");
        create_single_crate(tmp.path());

        run(tmp.path(), Some("2.0.0"), None, false).expect("update");

        let content = fs::read_to_string(tmp.path().join("Cargo.toml")).expect("read");
        assert!(content.contains(r#"version = "2.0.0""#));
        // Deps should be untouched.
        assert!(content.contains(r#"fabryk-core = "0.1.0""#));
    }

    #[test]
    fn test_run_single_crate_updates_deps_version() {
        let tmp = TempDir::new().expect("tempdir");
        create_single_crate(tmp.path());

        run(tmp.path(), None, Some("0.1.2"), false).expect("update");

        let content = fs::read_to_string(tmp.path().join("Cargo.toml")).expect("read");
        // Project version unchanged.
        assert!(content.contains(r#"version = "1.0.0""#));
        // Deps updated.
        assert!(content.contains(r#"fabryk-core = "0.1.2""#));
        assert!(content.contains(r#"version = "0.1.2""#)); // fabryk-graph
        assert!(content.contains(r#"ecl-design = "0.1.2""#));
        // Non-matching dep unchanged.
        assert!(content.contains(r#"serde = "1.0""#));
    }

    #[test]
    fn test_run_single_crate_updates_both_versions() {
        let tmp = TempDir::new().expect("tempdir");
        create_single_crate(tmp.path());

        run(tmp.path(), Some("2.0.0"), Some("0.1.2"), false).expect("update");

        let content = fs::read_to_string(tmp.path().join("Cargo.toml")).expect("read");
        assert!(content.contains(r#"name = "myapp""#));
        // Check that project version is 2.0.0 (in [package]).
        // We need a smarter check since fabryk-graph also has version = "0.1.2".
        let lines: Vec<&str> = content.lines().collect();
        let pkg_version_line = lines
            .iter()
            .find(|l| l.starts_with("version"))
            .expect("version");
        assert!(pkg_version_line.contains("2.0.0"));
        // Deps updated.
        assert!(content.contains(r#"fabryk-core = "0.1.2""#));
        assert!(content.contains(r#"ecl-design = "0.1.2""#));
    }

    #[test]
    fn test_run_single_crate_check_detects_dep_mismatches() {
        let tmp = TempDir::new().expect("tempdir");
        create_single_crate(tmp.path());

        let result = run(tmp.path(), None, Some("0.1.2"), true);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_single_crate_check_passes_when_matching() {
        let tmp = TempDir::new().expect("tempdir");
        create_single_crate(tmp.path());

        let result = run(tmp.path(), None, Some("0.1.0"), true);
        assert!(result.is_ok());
    }
}
