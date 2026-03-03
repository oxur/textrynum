//! Handler for `textyl set-version` and `textyl check-versions`.

use crate::crate_info::VersionMismatch;
use crate::editor;
use crate::error::TextylError;
use crate::output;
use crate::workspace;
use anyhow::Result;
use std::path::Path;

/// Run the set-version command.
///
/// If `check` is true, only report mismatches without modifying files.
/// If `check` is false, update the workspace version and all internal deps.
pub fn run(root: &Path, version: &str, check: bool) -> Result<()> {
    let crates = workspace::scan_all_crates(root)?;

    if check {
        return run_check(&crates, version);
    }

    run_update(root, &crates, version)
}

/// Check mode: report mismatches and exit non-zero if any found.
fn run_check(crates: &[crate::crate_info::CrateInfo], expected_version: &str) -> Result<()> {
    let mismatches = collect_mismatches(crates, expected_version);

    if mismatches.is_empty() {
        println!("All internal dependency versions match \"{expected_version}\".");
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

/// Update mode: set workspace version and sync all internal deps.
fn run_update(
    root: &Path,
    crates: &[crate::crate_info::CrateInfo],
    new_version: &str,
) -> Result<()> {
    let root_cargo_toml = root.join("Cargo.toml");

    // Update workspace.package.version.
    if editor::update_workspace_version(&root_cargo_toml, new_version)? {
        println!("Updated workspace version to \"{new_version}\".");
    } else {
        println!("Workspace version already at \"{new_version}\".");
    }

    // Update all internal path dep versions.
    let member_paths = workspace::list_member_paths(root)?;
    let mut total_changes = 0u32;

    for crate_info in crates {
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
                editor::update_dep_version(&cargo_toml, &dep.section, &dep.name, new_version)?;
            if changed {
                total_changes += 1;
                println!(
                    "  Updated {} [{}] {} = \"{new_version}\"",
                    crate_info.name, dep.section, dep.name
                );
            }
        }
    }

    if total_changes == 0 {
        println!("All internal dependency versions already at \"{new_version}\".");
    } else {
        println!("{total_changes} dependency version(s) updated to \"{new_version}\".");
    }

    Ok(())
}

/// Collect all version mismatches across the workspace.
fn collect_mismatches(
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

    #[test]
    fn test_run_check_no_mismatches_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        let result = run(tmp.path(), "0.1.0", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_check_with_mismatches_returns_error() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        let result = run(tmp.path(), "0.2.0", true);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_update_sets_workspace_and_dep_versions() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        run(tmp.path(), "0.2.0", false).expect("update");

        // Check workspace version was updated.
        let root = fs::read_to_string(tmp.path().join("Cargo.toml")).expect("read root");
        assert!(root.contains(r#"version = "0.2.0""#));

        // Check beta's dep on alpha was updated.
        let beta = fs::read_to_string(tmp.path().join("crates/beta/Cargo.toml")).expect("read");
        assert!(beta.contains(r#"version = "0.2.0""#));

        // Comments should be preserved.
        assert!(beta.contains("# Core dep"));
    }

    #[test]
    fn test_run_update_already_at_version_is_noop() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        // Update to current version should be a no-op.
        run(tmp.path(), "0.1.0", false).expect("update");

        let beta = fs::read_to_string(tmp.path().join("crates/beta/Cargo.toml")).expect("read");
        assert!(beta.contains(r#"version = "0.1.0""#));
    }

    #[test]
    fn test_check_after_update_succeeds() {
        let tmp = TempDir::new().expect("tempdir");
        create_test_workspace(tmp.path());

        // First update.
        run(tmp.path(), "0.2.0", false).expect("update");

        // Then check should pass.
        let result = run(tmp.path(), "0.2.0", true);
        assert!(result.is_ok());
    }
}
