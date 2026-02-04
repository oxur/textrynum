//! Configurable path resolver for domain-specific directories.
//!
//! `PathResolver` provides a configurable way to locate project directories
//! using environment variables, directory markers, and fallback paths.
//! Each domain (music-theory, another-project, etc.) creates its own resolver
//! with appropriate configuration.
//!
//! # Example
//!
//! ```no_run
//! use fabryk_core::util::resolver::PathResolver;
//!
//! // Create a resolver for "music-theory" project
//! let resolver = PathResolver::new("music-theory")
//!     .with_config_marker("config/default.toml")
//!     .with_project_markers(&["SKILL.md", "CONVENTIONS.md"]);
//!
//! // These check MUSIC_THEORY_CONFIG_DIR, then search for markers
//! if let Some(config) = resolver.config_dir() {
//!     println!("Config: {:?}", config);
//! }
//! ```

use std::env;
use std::path::PathBuf;

use crate::util::paths::{binary_dir, expand_tilde, find_dir_with_marker};

/// Configurable path resolver for a specific project/domain.
#[derive(Debug, Clone)]
pub struct PathResolver {
    /// Project name (e.g., "music-theory")
    project_name: String,
    /// Environment variable prefix (e.g., "MUSIC_THEORY")
    env_prefix: String,
    /// Marker file/dir to identify config directory (e.g., "config/default.toml")
    config_marker: Option<String>,
    /// Marker files to identify project root (e.g., ["SKILL.md", "Cargo.toml"])
    project_markers: Vec<String>,
    /// Fallback config path (expanded with tilde)
    config_fallback: Option<PathBuf>,
    /// Fallback project root (expanded with tilde)
    project_fallback: Option<PathBuf>,
}

impl PathResolver {
    /// Create a new resolver for the given project name.
    ///
    /// The project name is converted to an environment variable prefix:
    /// - "music-theory" → "MUSIC_THEORY"
    /// - "my_project" → "MY_PROJECT"
    pub fn new(project_name: &str) -> Self {
        let env_prefix = project_name.to_uppercase().replace(['-', ' '], "_");

        Self {
            project_name: project_name.to_string(),
            env_prefix,
            config_marker: None,
            project_markers: vec![],
            config_fallback: None,
            project_fallback: None,
        }
    }

    /// Set the marker file/directory that identifies a config directory.
    pub fn with_config_marker(mut self, marker: &str) -> Self {
        self.config_marker = Some(marker.to_string());
        self
    }

    /// Set marker files that identify the project root.
    pub fn with_project_markers(mut self, markers: &[&str]) -> Self {
        self.project_markers = markers.iter().map(|s| (*s).to_string()).collect();
        self
    }

    /// Set a fallback path for config directory (supports ~ expansion).
    pub fn with_config_fallback(mut self, path: &str) -> Self {
        self.config_fallback = Some(expand_tilde(path));
        self
    }

    /// Set a fallback path for project root (supports ~ expansion).
    pub fn with_project_fallback(mut self, path: &str) -> Self {
        self.project_fallback = Some(expand_tilde(path));
        self
    }

    /// Get the environment variable name for a given suffix.
    ///
    /// # Example
    /// ```
    /// use fabryk_core::util::resolver::PathResolver;
    ///
    /// let resolver = PathResolver::new("music-theory");
    /// assert_eq!(resolver.env_var("CONFIG_DIR"), "MUSIC_THEORY_CONFIG_DIR");
    /// ```
    pub fn env_var(&self, suffix: &str) -> String {
        format!("{}_{}", self.env_prefix, suffix)
    }

    /// Resolve the config directory.
    ///
    /// Checks in order:
    /// 1. `{PROJECT}_CONFIG_DIR` environment variable
    /// 2. Walk up from binary looking for config marker
    /// 3. Fallback path (if configured)
    pub fn config_dir(&self) -> Option<PathBuf> {
        // 1. Check environment variable
        let env_var = self.env_var("CONFIG_DIR");
        if let Ok(path) = env::var(&env_var) {
            let path = expand_tilde(&path);
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Walk up from binary location
        if let (Some(bin_dir), Some(marker)) = (binary_dir(), &self.config_marker) {
            if let Some(root) = find_dir_with_marker(&bin_dir, marker) {
                // The marker might be nested (e.g., "config/default.toml")
                // Return the directory containing the first component
                if let Some(first_component) = marker.split('/').next() {
                    let config_path = root.join(first_component);
                    if config_path.exists() {
                        return Some(config_path);
                    }
                }
                return Some(root);
            }
        }

        // 3. Try fallback
        if let Some(fallback) = &self.config_fallback {
            if fallback.exists() {
                return Some(fallback.clone());
            }
        }

        None
    }

    /// Resolve the project root directory.
    ///
    /// Checks in order:
    /// 1. `{PROJECT}_ROOT` environment variable
    /// 2. Walk up from binary looking for project markers
    /// 3. Fallback path (if configured)
    pub fn project_root(&self) -> Option<PathBuf> {
        // 1. Check environment variable
        let env_var = self.env_var("ROOT");
        if let Ok(path) = env::var(&env_var) {
            let path = expand_tilde(&path);
            if path.exists() {
                return Some(path);
            }
        }

        // 2. Walk up from binary location, trying each marker
        if let Some(bin_dir) = binary_dir() {
            for marker in &self.project_markers {
                if let Some(root) = find_dir_with_marker(&bin_dir, marker) {
                    return Some(root);
                }
            }
        }

        // 3. Try fallback
        if let Some(fallback) = &self.project_fallback {
            if fallback.exists() {
                return Some(fallback.clone());
            }
        }

        None
    }

    /// Get the project name.
    pub fn project_name(&self) -> &str {
        &self.project_name
    }

    /// Get the environment variable prefix.
    pub fn env_prefix(&self) -> &str {
        &self.env_prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_simple_name() {
        let resolver = PathResolver::new("myproject");
        assert_eq!(resolver.project_name(), "myproject");
        assert_eq!(resolver.env_prefix(), "MYPROJECT");
    }

    #[test]
    fn test_new_kebab_case_name() {
        let resolver = PathResolver::new("music-theory");
        assert_eq!(resolver.project_name(), "music-theory");
        assert_eq!(resolver.env_prefix(), "MUSIC_THEORY");
    }

    #[test]
    fn test_new_snake_case_name() {
        let resolver = PathResolver::new("my_project");
        assert_eq!(resolver.project_name(), "my_project");
        assert_eq!(resolver.env_prefix(), "MY_PROJECT");
    }

    #[test]
    fn test_env_var() {
        let resolver = PathResolver::new("music-theory");
        assert_eq!(resolver.env_var("CONFIG_DIR"), "MUSIC_THEORY_CONFIG_DIR");
        assert_eq!(resolver.env_var("ROOT"), "MUSIC_THEORY_ROOT");
        assert_eq!(resolver.env_var("DATA_DIR"), "MUSIC_THEORY_DATA_DIR");
    }

    #[test]
    fn test_config_dir_from_env() {
        let resolver = PathResolver::new("fabryk-test-config");

        // Create temp directory
        let temp_dir = std::env::temp_dir().join("fabryk_test_config_resolver");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Set env var
        env::set_var("FABRYK_TEST_CONFIG_CONFIG_DIR", &temp_dir);

        let result = resolver.config_dir();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        env::remove_var("FABRYK_TEST_CONFIG_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_project_root_from_env() {
        let resolver = PathResolver::new("fabryk-test-root");

        // Create temp directory
        let temp_dir = std::env::temp_dir().join("fabryk_test_root_resolver");
        let _ = std::fs::create_dir_all(&temp_dir);

        // Set env var
        env::set_var("FABRYK_TEST_ROOT_ROOT", &temp_dir);

        let result = resolver.project_root();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        env::remove_var("FABRYK_TEST_ROOT_ROOT");
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_config_dir_with_fallback() {
        let temp_dir = std::env::temp_dir().join("fabryk_test_config_fallback");
        let _ = std::fs::create_dir_all(&temp_dir);

        let resolver = PathResolver::new("nonexistent-fabryk-project")
            .with_config_fallback(&temp_dir.to_string_lossy());

        let result = resolver.config_dir();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_project_root_with_fallback() {
        let temp_dir = std::env::temp_dir().join("fabryk_test_root_fallback");
        let _ = std::fs::create_dir_all(&temp_dir);

        let resolver = PathResolver::new("nonexistent-fabryk-project")
            .with_project_fallback(&temp_dir.to_string_lossy());

        let result = resolver.project_root();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), temp_dir);

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_builder_pattern() {
        let resolver = PathResolver::new("my-app")
            .with_config_marker("config/settings.toml")
            .with_project_markers(&["Cargo.toml", "package.json"])
            .with_config_fallback("~/.config/my-app")
            .with_project_fallback("~/projects/my-app");

        assert_eq!(resolver.project_name(), "my-app");
        assert_eq!(resolver.env_prefix(), "MY_APP");
    }

    #[test]
    fn test_config_dir_nonexistent_env_var() {
        let resolver = PathResolver::new("definitely-nonexistent-fabryk-xyz");

        // No env var set, no markers, no fallback
        let result = resolver.config_dir();
        assert!(result.is_none());
    }

    #[test]
    fn test_project_root_nonexistent() {
        let resolver = PathResolver::new("definitely-nonexistent-fabryk-xyz");

        // No env var set, no markers, no fallback
        let result = resolver.project_root();
        assert!(result.is_none());
    }
}
