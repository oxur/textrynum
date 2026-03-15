//! Format-preserving TOML editing operations.
//!
//! Uses `toml_edit` to modify Cargo.toml files while preserving
//! comments, formatting, and key ordering.

use crate::error::TextylError;
use anyhow::Result;
use std::path::Path;
use toml_edit::DocumentMut;

/// Update the version of a single dependency in a single Cargo.toml file.
///
/// Only modifies dependencies that have a `path` key (internal deps).
/// Returns `Ok(true)` if a change was made, `Ok(false)` if already at target version.
pub fn update_dep_version(
    cargo_toml_path: &Path,
    section: &str,
    dep_name: &str,
    new_version: &str,
) -> Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path).map_err(|e| TextylError::FileRead {
        path: cargo_toml_path.to_path_buf(),
        source: e,
    })?;
    let mut doc: DocumentMut =
        content
            .parse()
            .map_err(|e: toml_edit::TomlError| TextylError::TomlParse {
                path: cargo_toml_path.to_path_buf(),
                source: e,
            })?;

    let changed = update_dep_in_doc(&mut doc, section, dep_name, new_version);

    if changed {
        std::fs::write(cargo_toml_path, doc.to_string()).map_err(|e| TextylError::FileWrite {
            path: cargo_toml_path.to_path_buf(),
            source: e,
        })?;
    }

    Ok(changed)
}

/// Update a dependency version within a parsed document (no I/O).
/// Returns `true` if a change was made.
fn update_dep_in_doc(
    doc: &mut DocumentMut,
    section: &str,
    dep_name: &str,
    new_version: &str,
) -> bool {
    let Some(deps) = doc.get_mut(section) else {
        return false;
    };
    let Some(dep) = deps.get_mut(dep_name) else {
        return false;
    };

    // Handle inline table: { version = "X", path = "..." }
    if let Some(table) = dep.as_inline_table_mut() {
        if !table.contains_key("path") {
            return false;
        }
        if let Some(current) = table.get("version")
            && current.as_str() == Some(new_version)
        {
            return false;
        }
        table.insert("version", new_version.into());
        return true;
    }

    // Handle standard table: [dependencies.dep-name]
    if let Some(table) = dep.as_table_like_mut() {
        if !table.contains_key("path") {
            return false;
        }
        if let Some(item) = table.get("version")
            && item.as_str() == Some(new_version)
        {
            return false;
        }
        table.insert("version", toml_edit::value(new_version));
        return true;
    }

    false
}

/// Update the version of a dependency by name, regardless of whether it has a `path` key.
///
/// Handles three forms:
/// - Plain string: `dep = "1.0"` → `dep = "2.0"`
/// - Inline table: `dep = { version = "1.0", features = [...] }` → version updated
/// - Standard table: `[dependencies.dep]` with `version = "1.0"` → version updated
///
/// Returns `Ok(true)` if a change was made, `Ok(false)` if already at target version or dep not found.
pub fn update_any_dep_version(
    cargo_toml_path: &Path,
    section: &str,
    dep_name: &str,
    new_version: &str,
) -> Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path).map_err(|e| TextylError::FileRead {
        path: cargo_toml_path.to_path_buf(),
        source: e,
    })?;
    let mut doc: DocumentMut =
        content
            .parse()
            .map_err(|e: toml_edit::TomlError| TextylError::TomlParse {
                path: cargo_toml_path.to_path_buf(),
                source: e,
            })?;

    let changed = update_any_dep_in_doc(&mut doc, section, dep_name, new_version);

    if changed {
        std::fs::write(cargo_toml_path, doc.to_string()).map_err(|e| TextylError::FileWrite {
            path: cargo_toml_path.to_path_buf(),
            source: e,
        })?;
    }

    Ok(changed)
}

/// Update a dependency version within a parsed document, regardless of `path` key.
/// Handles plain string deps, inline tables, and standard tables.
/// Returns `true` if a change was made.
fn update_any_dep_in_doc(
    doc: &mut DocumentMut,
    section: &str,
    dep_name: &str,
    new_version: &str,
) -> bool {
    let Some(deps) = doc.get_mut(section) else {
        return false;
    };
    let Some(dep) = deps.get_mut(dep_name) else {
        return false;
    };

    // Handle plain string: dep = "1.0"
    if let Some(current) = dep.as_str() {
        if current == new_version {
            return false;
        }
        *dep = toml_edit::value(new_version);
        return true;
    }

    // Handle inline table: { version = "X", ... }
    if let Some(table) = dep.as_inline_table_mut() {
        if let Some(current) = table.get("version")
            && current.as_str() == Some(new_version)
        {
            return false;
        }
        table.insert("version", new_version.into());
        return true;
    }

    // Handle standard table: [dependencies.dep-name]
    if let Some(table) = dep.as_table_like_mut() {
        if let Some(item) = table.get("version")
            && item.as_str() == Some(new_version)
        {
            return false;
        }
        table.insert("version", toml_edit::value(new_version));
        return true;
    }

    false
}

/// Update the `[package] version` in a single-crate Cargo.toml.
/// Returns `Ok(true)` if a change was made, `Ok(false)` if already at target.
pub fn update_package_version(cargo_toml_path: &Path, new_version: &str) -> Result<bool> {
    let content = std::fs::read_to_string(cargo_toml_path).map_err(|e| TextylError::FileRead {
        path: cargo_toml_path.to_path_buf(),
        source: e,
    })?;
    let mut doc: DocumentMut =
        content
            .parse()
            .map_err(|e: toml_edit::TomlError| TextylError::TomlParse {
                path: cargo_toml_path.to_path_buf(),
                source: e,
            })?;

    let current = doc
        .get("package")
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if current.as_deref() == Some(new_version) {
        return Ok(false);
    }

    if let Some(pkg) = doc.get_mut("package")
        && let Some(table) = pkg.as_table_like_mut()
    {
        table.insert("version", toml_edit::value(new_version));
        std::fs::write(cargo_toml_path, doc.to_string()).map_err(|e| TextylError::FileWrite {
            path: cargo_toml_path.to_path_buf(),
            source: e,
        })?;
        return Ok(true);
    }

    Ok(false)
}

/// Update the `[workspace.package] version` in a root Cargo.toml.
/// Returns `Ok(true)` if a change was made, `Ok(false)` if already at target.
pub fn update_workspace_version(root_cargo_toml: &Path, new_version: &str) -> Result<bool> {
    let content = std::fs::read_to_string(root_cargo_toml).map_err(|e| TextylError::FileRead {
        path: root_cargo_toml.to_path_buf(),
        source: e,
    })?;
    let mut doc: DocumentMut =
        content
            .parse()
            .map_err(|e: toml_edit::TomlError| TextylError::TomlParse {
                path: root_cargo_toml.to_path_buf(),
                source: e,
            })?;

    let current = doc
        .get("workspace")
        .and_then(|ws| ws.get("package"))
        .and_then(|pkg| pkg.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    if current.as_deref() == Some(new_version) {
        return Ok(false);
    }

    if let Some(ws) = doc.get_mut("workspace")
        && let Some(pkg) = ws.get_mut("package")
        && let Some(table) = pkg.as_table_like_mut()
    {
        table.insert("version", toml_edit::value(new_version));
        std::fs::write(root_cargo_toml, doc.to_string()).map_err(|e| TextylError::FileWrite {
            path: root_cargo_toml.to_path_buf(),
            source: e,
        })?;
        return Ok(true);
    }

    Ok(false)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_update_dep_version_inline_table_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
alpha = { version = "0.1.0", path = "../alpha" }
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "0.2.0""#));
        assert!(content.contains(r#"path = "../alpha""#));
    }

    #[test]
    fn test_update_dep_version_already_current_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
alpha = { version = "0.2.0", path = "../alpha" }
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_dep_version_preserves_comments() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
# This is an important dep
alpha = { version = "0.1.0", path = "../alpha" }
# Another comment
beta = "1.0"
"#,
        )
        .expect("write");

        update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("# This is an important dep"));
        assert!(content.contains("# Another comment"));
        assert!(content.contains(r#"beta = "1.0""#));
    }

    #[test]
    fn test_update_dep_version_optional_dep_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
alpha = { version = "0.1.0", path = "../alpha", optional = true }
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "0.2.0""#));
        assert!(content.contains("optional = true"));
    }

    #[test]
    fn test_update_dep_version_missing_section_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_dep_version_missing_dep_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
beta = { version = "1.0.0", path = "../beta" }
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_dep_version_no_path_key_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
"#,
        )
        .expect("write");

        let changed = update_dep_version(&path, "dependencies", "serde", "2.0").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_workspace_version_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[workspace]
resolver = "2"
members = ["crates/alpha"]

[workspace.package]
version = "0.1.0"
edition = "2021"
"#,
        )
        .expect("write");

        let changed = update_workspace_version(&path, "0.2.0").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "0.2.0""#));
        assert!(content.contains(r#"edition = "2021""#));
    }

    // ---- update_any_dep_version tests ----

    #[test]
    fn test_update_any_dep_version_plain_string_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = "0.1.0"
serde = "1.0"
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"fabryk-core = "0.1.2""#));
        assert!(content.contains(r#"serde = "1.0""#));
    }

    #[test]
    fn test_update_any_dep_version_inline_table_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = { version = "0.1.0", features = ["redis"] }
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "0.1.2""#));
        assert!(content.contains(r#"features = ["redis"]"#));
    }

    #[test]
    fn test_update_any_dep_version_already_current_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = "0.1.2"
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_any_dep_version_inline_table_already_current_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
fabryk-core = { version = "0.1.2", features = ["redis"] }
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_any_dep_version_missing_dep_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
serde = "1.0"
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_any_dep_version_preserves_comments() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"

[dependencies]
# Fabryk framework
fabryk-core = "0.1.0"
# Serialization
serde = "1.0"
"#,
        )
        .expect("write");

        update_any_dep_version(&path, "dependencies", "fabryk-core", "0.1.2").expect("update");

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains("# Fabryk framework"));
        assert!(content.contains("# Serialization"));
        assert!(content.contains(r#"serde = "1.0""#));
    }

    #[test]
    fn test_update_any_dep_version_with_path_key_still_works() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
alpha = { version = "0.1.0", path = "../alpha" }
"#,
        )
        .expect("write");

        let changed =
            update_any_dep_version(&path, "dependencies", "alpha", "0.2.0").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "0.2.0""#));
        assert!(content.contains(r#"path = "../alpha""#));
    }

    // ---- update_package_version tests ----

    #[test]
    fn test_update_package_version_updates_correctly() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "1.0.0"
edition = "2021"
"#,
        )
        .expect("write");

        let changed = update_package_version(&path, "2.0.0").expect("update");
        assert!(changed);

        let content = fs::read_to_string(&path).expect("read");
        assert!(content.contains(r#"version = "2.0.0""#));
        assert!(content.contains(r#"edition = "2021""#));
    }

    #[test]
    fn test_update_package_version_already_current_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[package]
name = "myapp"
version = "2.0.0"
"#,
        )
        .expect("write");

        let changed = update_package_version(&path, "2.0.0").expect("update");
        assert!(!changed);
    }

    #[test]
    fn test_update_workspace_version_already_current_returns_false() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("Cargo.toml");
        fs::write(
            &path,
            r#"[workspace]
resolver = "2"

[workspace.package]
version = "0.2.0"
"#,
        )
        .expect("write");

        let changed = update_workspace_version(&path, "0.2.0").expect("update");
        assert!(!changed);
    }
}
