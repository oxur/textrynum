//! Generic configuration loader that encapsulates the confyg build pattern.
//!
//! Eliminates the boilerplate that every Fabryk-based project duplicates:
//! `Confygery::new()` → `add_file()` → `add_env(sections)` → `build()`.

use confyg::{Confygery, env};
use fabryk_core::{Error, Result};
use serde::de::DeserializeOwned;
use std::path::PathBuf;

/// Builder for loading configuration from TOML files and environment variables.
///
/// # Example
///
/// ```no_run
/// use fabryk_cli::config_loader::ConfigLoaderBuilder;
/// use serde::Deserialize;
///
/// #[derive(Default, Deserialize)]
/// struct MyConfig {
///     port: u16,
/// }
///
/// let resolve = |explicit: Option<&str>| -> Option<std::path::PathBuf> {
///     explicit.map(std::path::PathBuf::from)
/// };
///
/// let (config, path) = ConfigLoaderBuilder::new("myapp")
///     .section("server")
///     .section("logging")
///     .port_env_override("PORT")
///     .build::<MyConfig>(None, resolve)
///     .unwrap();
/// ```
pub struct ConfigLoaderBuilder {
    prefix: String,
    sections: Vec<String>,
    port_env_var: Option<String>,
}

impl ConfigLoaderBuilder {
    /// Create a new builder with the given environment variable prefix.
    ///
    /// The prefix is used for env var namespacing (e.g., `"taproot"` →
    /// `TAPROOT_*` environment variables).
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
            sections: Vec::new(),
            port_env_var: None,
        }
    }

    /// Register a config section for environment variable mapping.
    ///
    /// Each section becomes a namespace in the env var hierarchy.
    /// For example, section `"bq"` maps `TAPROOT_BQ_PROJECT` → `bq.project`.
    pub fn section(mut self, name: &str) -> Self {
        self.sections.push(name.to_string());
        self
    }

    /// Register a bare environment variable that overrides the `port` field.
    ///
    /// Cloud Run sets `PORT` (without prefix). This option lets the loader
    /// check that env var and apply it as a post-load override.
    ///
    /// The override only applies if the env var exists and parses as `u16`.
    /// The caller is responsible for applying the override to the correct
    /// field after `build()` returns.
    pub fn port_env_override(mut self, env_var: &str) -> Self {
        self.port_env_var = Some(env_var.to_string());
        self
    }

    /// Build the configuration, returning the deserialized struct and
    /// the resolved config file path (if one was found).
    ///
    /// # Arguments
    ///
    /// * `config_path` — explicit config file path (e.g., from `--config` flag)
    /// * `resolve_fn` — function to resolve the config path when not explicit
    ///
    /// # Errors
    ///
    /// Returns an error if the config file exists but cannot be parsed,
    /// or if environment variables contain invalid values.
    pub fn build<C: DeserializeOwned + Default>(
        self,
        config_path: Option<&str>,
        resolve_fn: impl Fn(Option<&str>) -> Option<PathBuf>,
    ) -> Result<(C, Option<PathBuf>)> {
        let resolved_path = resolve_fn(config_path);

        let mut builder =
            Confygery::new().map_err(|e| Error::config(format!("config init: {e}")))?;

        // Load config file if it exists
        if let Some(ref path) = resolved_path {
            if path.exists() {
                builder
                    .add_file(&path.to_string_lossy())
                    .map_err(|e| Error::config(format!("config file: {e}")))?;
            }
        }

        // Load environment variables with prefix and sections
        let mut env_opts = env::Options::with_top_level(&self.prefix);
        for section in &self.sections {
            env_opts.add_section(section);
        }
        builder
            .add_env(env_opts)
            .map_err(|e| Error::config(format!("config env: {e}")))?;

        let config: C = builder
            .build()
            .map_err(|e| Error::config(format!("config build: {e}")))?;

        Ok((config, resolved_path))
    }

    /// Check the port environment variable override, if configured.
    ///
    /// Returns `Some(port)` if the env var exists and parses as `u16`.
    /// Intended to be called after `build()` to apply the override.
    pub fn check_port_override(&self) -> Option<u16> {
        self.port_env_var
            .as_ref()
            .and_then(|var| std::env::var(var).ok())
            .and_then(|val| val.parse::<u16>().ok())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Default, Deserialize, PartialEq)]
    struct TestConfig {
        #[serde(default)]
        name: String,
        #[serde(default)]
        port: u16,
    }

    fn no_resolve(_: Option<&str>) -> Option<PathBuf> {
        None
    }

    #[test]
    fn test_config_loader_defaults() {
        let (config, path) = ConfigLoaderBuilder::new("test_loader")
            .build::<TestConfig>(None, no_resolve)
            .unwrap();
        assert_eq!(config.name, "");
        assert_eq!(config.port, 0);
        assert!(path.is_none());
    }

    #[test]
    fn test_config_loader_from_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let file = dir.path().join("config.toml");
        std::fs::write(&file, "name = \"loaded\"\nport = 9090").unwrap();

        let resolve = |explicit: Option<&str>| explicit.map(PathBuf::from);
        let (config, path) = ConfigLoaderBuilder::new("test_loader")
            .build::<TestConfig>(Some(file.to_str().unwrap()), resolve)
            .unwrap();

        assert_eq!(config.name, "loaded");
        assert_eq!(config.port, 9090);
        assert!(path.is_some());
    }

    #[test]
    fn test_config_loader_missing_file_uses_defaults() {
        let resolve = |explicit: Option<&str>| explicit.map(PathBuf::from);
        let (config, _) = ConfigLoaderBuilder::new("test_loader")
            .build::<TestConfig>(Some("/nonexistent/config.toml"), resolve)
            .unwrap();
        assert_eq!(config.name, "");
    }

    #[test]
    fn test_config_loader_sections() {
        // Just verify the builder accepts sections without error
        let builder = ConfigLoaderBuilder::new("app")
            .section("server")
            .section("logging")
            .section("oauth");
        assert_eq!(builder.sections.len(), 3);
    }

    #[test]
    fn test_config_loader_port_override_not_set() {
        let builder = ConfigLoaderBuilder::new("app").port_env_override("PORT");
        // PORT is unlikely to be set in test env
        // Just verify the method works without panicking
        let _ = builder.check_port_override();
    }
}
