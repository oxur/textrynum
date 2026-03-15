//! Path resolution utilities.
//!
//! Provides generic path helpers used across all Fabryk domains.
//! Domain-specific path resolution (config dirs, project roots) should
//! be implemented in domain crates using these primitives.

use std::env;
use std::path::{Path, PathBuf};

/// Maximum number of parent directories to walk when searching for a marker.
pub const MAX_WALK_LEVELS: usize = 10;

/// Returns the absolute path to the currently running binary.
pub fn binary_path() -> Option<PathBuf> {
    env::current_exe().ok()
}

/// Returns the directory containing the currently running binary.
pub fn binary_dir() -> Option<PathBuf> {
    binary_path().and_then(|p| p.parent().map(|p| p.to_path_buf()))
}

/// Walks up the directory tree from `start` looking for a directory containing `marker`.
///
/// Returns the directory containing the marker file/directory, or None if not found
/// within [`MAX_WALK_LEVELS`] iterations.
///
/// # Example
///
/// ```no_run
/// use fabryk_core::util::paths::find_dir_with_marker;
///
/// // Find a project root by looking for Cargo.toml
/// if let Some(root) = find_dir_with_marker(".", "Cargo.toml") {
///     println!("Project root: {:?}", root);
/// }
/// ```
pub fn find_dir_with_marker<P: AsRef<Path>>(start: P, marker: &str) -> Option<PathBuf> {
    let mut current = start.as_ref().to_path_buf();

    for _ in 0..MAX_WALK_LEVELS {
        let candidate = current.join(marker);
        if candidate.exists() {
            return Some(current);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}

/// Expands `~` to the user's home directory.
///
/// If the path starts with `~`, replaces it with the user's home directory.
/// Otherwise returns the path unchanged.
///
/// # Example
///
/// ```
/// use fabryk_core::util::paths::expand_tilde;
///
/// let expanded = expand_tilde("~/documents");
/// assert!(!expanded.starts_with("~"));
/// ```
pub fn expand_tilde<P: AsRef<Path>>(path: P) -> PathBuf {
    let path = path.as_ref();
    if let Ok(stripped) = path.strip_prefix("~")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(stripped);
    }
    path.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_path_exists() {
        let path = binary_path();
        assert!(path.is_some(), "Binary path should be found");
        let path = path.unwrap();
        assert!(path.exists(), "Binary path should exist: {:?}", path);
        assert!(path.is_file(), "Binary path should be a file: {:?}", path);
    }

    #[test]
    fn test_binary_dir_exists() {
        let dir = binary_dir();
        assert!(dir.is_some(), "Binary dir should be found");
        let dir = dir.unwrap();
        assert!(dir.exists(), "Binary dir should exist: {:?}", dir);
        assert!(dir.is_dir(), "Binary dir should be a directory: {:?}", dir);
    }

    #[test]
    fn test_expand_tilde_with_tilde() {
        let path = expand_tilde("~/test/path");
        assert!(!path.starts_with("~"), "Tilde should be expanded");
        if let Some(home) = dirs::home_dir() {
            assert!(path.starts_with(&home), "Path should start with home dir");
            assert!(path.ends_with("test/path"), "Path should preserve suffix");
        }
    }

    #[test]
    fn test_expand_tilde_without_tilde() {
        let original = PathBuf::from("/absolute/path");
        let expanded = expand_tilde(&original);
        assert_eq!(original, expanded, "Absolute path should not change");
    }

    #[test]
    fn test_expand_tilde_relative_without_tilde() {
        let original = PathBuf::from("relative/path");
        let expanded = expand_tilde(&original);
        assert_eq!(
            original, expanded,
            "Relative path without tilde should not change"
        );
    }

    #[test]
    fn test_expand_tilde_tilde_only() {
        let path = expand_tilde("~");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(path, home, "~ should expand to home directory");
        }
    }

    #[test]
    fn test_expand_tilde_tilde_with_slash() {
        let path = expand_tilde("~/");
        if let Some(home) = dirs::home_dir() {
            assert!(
                path.starts_with(&home),
                "~/ should expand to home directory"
            );
        }
    }

    #[test]
    fn test_find_dir_with_marker_basic() {
        // Create a temp directory structure with a marker
        let temp_dir = std::env::temp_dir().join("fabryk_test_find_marker");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::write(temp_dir.join("marker.txt"), "test");

        let result = find_dir_with_marker(&temp_dir, "marker.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_find_dir_with_marker_nested() {
        // Create nested directory structure
        let temp_base = std::env::temp_dir().join("fabryk_test_find_marker_nested");
        let nested = temp_base.join("level1").join("level2");
        let _ = std::fs::create_dir_all(&nested);
        let _ = std::fs::write(temp_base.join("marker.txt"), "test");

        // Should find marker when starting from nested path
        let result = find_dir_with_marker(&nested, "marker.txt");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_base);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_find_dir_with_marker_not_found() {
        let result = find_dir_with_marker("/tmp", "nonexistent_marker_xyz_fabryk");
        assert!(result.is_none());
    }

    #[test]
    fn test_find_dir_with_marker_max_levels() {
        // Create a deeply nested structure
        let temp_base = std::env::temp_dir().join("fabryk_test_find_marker_deep");
        let _ = std::fs::create_dir_all(&temp_base);

        let mut deep_path = temp_base.clone();
        for i in 0..15 {
            deep_path = deep_path.join(format!("level{}", i));
        }
        let _ = std::fs::create_dir_all(&deep_path);

        // Put marker at the base (too far from deep_path)
        let _ = std::fs::write(temp_base.join("marker.txt"), "test");

        // Should not find it because it's beyond MAX_WALK_LEVELS
        let result = find_dir_with_marker(&deep_path, "marker.txt");
        // Result depends on depth vs MAX_WALK_LEVELS
        // With 15 levels and MAX_WALK_LEVELS=10, it should not find the marker
        assert!(result.is_none());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_base);
    }

    #[test]
    fn test_find_dir_with_marker_no_parent() {
        // Test with root path that has no parent
        let result = find_dir_with_marker("/", "nonexistent_marker_fabryk");
        assert!(result.is_none());
    }

    #[test]
    fn test_max_walk_levels_value() {
        assert_eq!(MAX_WALK_LEVELS, 10);
    }
}
