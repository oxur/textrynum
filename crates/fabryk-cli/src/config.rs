//! Configuration for Fabryk CLI applications.
//!
//! Provides the [`FabrykConfig`] struct that loads from TOML files,
//! environment variables, and defaults using the `confyg` crate.
//!
//! # Loading Priority
//!
//! 1. Explicit `--config <path>` flag
//! 2. `FABRYK_CONFIG` environment variable
//! 3. XDG default: `~/.config/fabryk/config.toml`
//! 4. Built-in defaults

use confyg::{Confygery, env};
use fabryk_core::traits::ConfigProvider;
use fabryk_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ============================================================================
// Configuration structs
// ============================================================================

/// Main configuration for Fabryk CLI applications.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FabrykConfig {
    /// Project name, used for env var prefixes and default paths.
    pub project_name: String,

    /// Base path for all project data.
    pub base_path: Option<String>,

    /// Content-related configuration.
    pub content: ContentConfig,

    /// Graph-related configuration.
    pub graph: GraphConfig,

    /// Server configuration.
    pub server: ServerConfig,
}

/// Content storage configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ContentConfig {
    /// Path to content directory.
    pub path: Option<String>,
}

/// Graph storage configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphConfig {
    /// Output path for graph files.
    pub output_path: Option<String>,
}

/// Server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    /// Port to listen on.
    pub port: u16,

    /// Host address to bind to.
    pub host: String,
}

// ============================================================================
// Default implementations
// ============================================================================

impl Default for FabrykConfig {
    fn default() -> Self {
        Self {
            project_name: "fabryk".to_string(),
            base_path: None,
            content: ContentConfig::default(),
            graph: GraphConfig::default(),
            server: ServerConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            host: "127.0.0.1".to_string(),
        }
    }
}

// ============================================================================
// Config loading
// ============================================================================

impl FabrykConfig {
    /// Load configuration from file, environment, and defaults.
    ///
    /// Loading priority:
    /// 1. Explicit `config_path` (from `--config` flag)
    /// 2. `FABRYK_CONFIG` env var
    /// 3. XDG default: `~/.config/fabryk/config.toml`
    /// 4. Built-in defaults
    pub fn load(config_path: Option<&str>) -> Result<Self> {
        let mut builder =
            Confygery::new().map_err(|e| Error::config(format!("config init: {e}")))?;

        if let Some(path) = Self::resolve_config_path(config_path)
            && path.exists()
        {
            builder
                .add_file(&path.to_string_lossy())
                .map_err(|e| Error::config(format!("config file: {e}")))?;
        }

        let mut env_opts = env::Options::with_top_level("FABRYK");
        env_opts.add_section("content");
        env_opts.add_section("graph");
        env_opts.add_section("server");
        builder
            .add_env(env_opts)
            .map_err(|e| Error::config(format!("config env: {e}")))?;

        let config: Self = builder
            .build()
            .map_err(|e| Error::config(format!("config build: {e}")))?;

        Ok(config)
    }

    /// Resolve the config file path from explicit flag, env var, or XDG default.
    pub fn resolve_config_path(explicit: Option<&str>) -> Option<PathBuf> {
        // 1. Explicit --config flag
        if let Some(path) = explicit {
            return Some(PathBuf::from(path));
        }

        // 2. FABRYK_CONFIG env var
        if let Ok(path) = std::env::var("FABRYK_CONFIG") {
            return Some(PathBuf::from(path));
        }

        // 3. XDG default
        Self::default_config_path()
    }

    /// Return the XDG default config path.
    pub fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("fabryk").join("config.toml"))
    }

    /// Serialize this config to a pretty-printed TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| Error::config(e.to_string()))
    }

    /// Flatten this config into environment variable pairs with `FABRYK_` prefix.
    pub fn to_env_vars(&self) -> Result<Vec<(String, String)>> {
        let value: toml::Value =
            toml::Value::try_from(self).map_err(|e| Error::config(e.to_string()))?;
        let mut vars = Vec::new();
        flatten_toml_value(&value, "FABRYK", &mut vars);
        Ok(vars)
    }
}

// ============================================================================
// ConfigProvider implementation
// ============================================================================

impl fabryk_core::ConfigManager for FabrykConfig {
    fn load(config_path: Option<&str>) -> Result<Self> {
        FabrykConfig::load(config_path)
    }

    fn resolve_config_path(explicit: Option<&str>) -> Option<PathBuf> {
        FabrykConfig::resolve_config_path(explicit)
    }

    fn default_config_path() -> Option<PathBuf> {
        FabrykConfig::default_config_path()
    }

    fn project_name() -> &'static str {
        "fabryk"
    }

    fn to_toml_string(&self) -> Result<String> {
        FabrykConfig::to_toml_string(self)
    }

    fn to_env_vars(&self) -> Result<Vec<(String, String)>> {
        FabrykConfig::to_env_vars(self)
    }
}

impl ConfigProvider for FabrykConfig {
    fn project_name(&self) -> &str {
        &self.project_name
    }

    fn base_path(&self) -> Result<PathBuf> {
        match &self.base_path {
            Some(p) => Ok(PathBuf::from(p)),
            None => std::env::current_dir()
                .map_err(|e| Error::config(format!("Could not determine base path: {e}"))),
        }
    }

    fn content_path(&self, content_type: &str) -> Result<PathBuf> {
        match &self.content.path {
            Some(p) => Ok(PathBuf::from(p)),
            None => Ok(self.base_path()?.join(content_type)),
        }
    }
}

// ============================================================================
// Helper: flatten TOML to env vars
// ============================================================================

/// Recursively flatten a TOML value into `KEY=value` pairs.
fn flatten_toml_value(value: &toml::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let env_key = format!("{}_{}", prefix, key.to_uppercase());
                flatten_toml_value(val, &env_key, out);
            }
        }
        toml::Value::Array(arr) => {
            if let Ok(json) = serde_json::to_string(arr) {
                out.push((prefix.to_string(), json));
            }
        }
        toml::Value::String(s) => {
            out.push((prefix.to_string(), s.clone()));
        }
        toml::Value::Integer(i) => {
            out.push((prefix.to_string(), i.to_string()));
        }
        toml::Value::Float(f) => {
            out.push((prefix.to_string(), f.to_string()));
        }
        toml::Value::Boolean(b) => {
            out.push((prefix.to_string(), b.to_string()));
        }
        toml::Value::Datetime(dt) => {
            out.push((prefix.to_string(), dt.to_string()));
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Serializes tests that manipulate environment variables.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    /// RAII guard for env var manipulation in tests.
    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: Test-only helper; tests using this guard run serially.
            unsafe { std::env::set_var(key, value) };
            Self {
                key: key.to_string(),
                prev,
            }
        }

        fn remove(key: &str) -> Self {
            let prev = std::env::var(key).ok();
            // SAFETY: Test-only helper; tests using this guard run serially.
            unsafe { std::env::remove_var(key) };
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: Test-only helper; tests using this guard run serially.
            if let Some(val) = &self.prev {
                unsafe { std::env::set_var(&self.key, val) };
            } else {
                unsafe { std::env::remove_var(&self.key) };
            }
        }
    }

    // ------------------------------------------------------------------------
    // Default tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_default() {
        let config = FabrykConfig::default();
        assert_eq!(config.project_name, "fabryk");
        assert!(config.base_path.is_none());
        assert!(config.content.path.is_none());
        assert!(config.graph.output_path.is_none());
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.server.host, "127.0.0.1");
    }

    // ------------------------------------------------------------------------
    // Serialization tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_from_toml() {
        let toml_str = r#"
            project_name = "my-app"
            base_path = "/data"

            [content]
            path = "/data/content"

            [graph]
            output_path = "/data/graphs"

            [server]
            port = 8080
            host = "0.0.0.0"
        "#;

        let config: FabrykConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.project_name, "my-app");
        assert_eq!(config.base_path.as_deref(), Some("/data"));
        assert_eq!(config.content.path.as_deref(), Some("/data/content"));
        assert_eq!(config.graph.output_path.as_deref(), Some("/data/graphs"));
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.server.host, "0.0.0.0");
    }

    #[test]
    fn test_fabryk_config_to_toml() {
        let config = FabrykConfig::default();
        let toml_str = config.to_toml_string().unwrap();
        assert!(toml_str.contains("project_name = \"fabryk\""));
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("port = 3000"));

        // Round-trip
        let parsed: FabrykConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.project_name, config.project_name);
        assert_eq!(parsed.server.port, config.server.port);
    }

    // ------------------------------------------------------------------------
    // Loading tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_load_from_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
                project_name = "loaded-app"
                [server]
                port = 9090
            "#,
        )
        .unwrap();

        let config = FabrykConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.project_name, "loaded-app");
        assert_eq!(config.server.port, 9090);
    }

    #[test]
    fn test_fabryk_config_load_defaults() {
        // Load with a nonexistent file falls back to defaults
        let config = FabrykConfig::load(Some("/nonexistent/config.toml")).unwrap();
        assert_eq!(config.project_name, "fabryk");
        assert_eq!(config.server.port, 3000);
    }

    #[test]
    fn test_fabryk_config_load_env_overlay() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
                project_name = "file-app"
                [server]
                host = "127.0.0.1"
            "#,
        )
        .unwrap();

        // Env vars override file values (confyg passes env values as strings,
        // so we test with a string field — numeric fields require manual handling).
        let _guard = EnvGuard::new("FABRYK_SERVER_HOST", "0.0.0.0");
        let config = FabrykConfig::load(Some(path.to_str().unwrap())).unwrap();
        assert_eq!(config.server.host, "0.0.0.0");
    }

    // ------------------------------------------------------------------------
    // resolve_config_path tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_resolve_config_path_explicit() {
        let path = FabrykConfig::resolve_config_path(Some("/explicit/config.toml"));
        assert_eq!(path, Some(PathBuf::from("/explicit/config.toml")));
    }

    #[test]
    fn test_fabryk_config_resolve_config_path_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new("FABRYK_CONFIG", "/env/config.toml");
        let path = FabrykConfig::resolve_config_path(None);
        assert_eq!(path, Some(PathBuf::from("/env/config.toml")));
    }

    #[test]
    fn test_fabryk_config_resolve_config_path_default() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove("FABRYK_CONFIG");
        let path = FabrykConfig::resolve_config_path(None);
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_str().unwrap().contains("fabryk"));
        assert!(p.to_str().unwrap().ends_with("config.toml"));
    }

    // ------------------------------------------------------------------------
    // ConfigProvider tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_provider_project_name() {
        let config = FabrykConfig {
            project_name: "test-project".into(),
            ..Default::default()
        };
        assert_eq!(config.project_name(), "test-project");
    }

    #[test]
    fn test_fabryk_config_provider_base_path() {
        let config = FabrykConfig {
            base_path: Some("/my/data".into()),
            ..Default::default()
        };
        assert_eq!(config.base_path().unwrap(), PathBuf::from("/my/data"));
    }

    #[test]
    fn test_fabryk_config_provider_base_path_default() {
        let config = FabrykConfig::default();
        let base = config.base_path().unwrap();
        // Falls back to cwd
        assert_eq!(base, std::env::current_dir().unwrap());
    }

    #[test]
    fn test_fabryk_config_provider_content_path() {
        let config = FabrykConfig {
            base_path: Some("/project".into()),
            ..Default::default()
        };
        let path = config.content_path("concepts").unwrap();
        assert_eq!(path, PathBuf::from("/project/concepts"));
    }

    #[test]
    fn test_fabryk_config_provider_content_path_explicit() {
        let config = FabrykConfig {
            content: ContentConfig {
                path: Some("/custom/content".into()),
            },
            ..Default::default()
        };
        let path = config.content_path("anything").unwrap();
        assert_eq!(path, PathBuf::from("/custom/content"));
    }

    // ------------------------------------------------------------------------
    // to_env_vars tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_to_env_vars() {
        let config = FabrykConfig::default();
        let vars = config.to_env_vars().unwrap();
        let map: HashMap<_, _> = vars.into_iter().collect();
        assert_eq!(map.get("FABRYK_PROJECT_NAME").unwrap(), "fabryk");
        assert_eq!(map.get("FABRYK_SERVER_PORT").unwrap(), "3000");
        assert_eq!(map.get("FABRYK_SERVER_HOST").unwrap(), "127.0.0.1");
    }

    // ------------------------------------------------------------------------
    // Clone + Send + Sync
    // ------------------------------------------------------------------------

    #[test]
    fn test_fabryk_config_is_clone() {
        let config = FabrykConfig::default();
        let cloned = config.clone();
        assert_eq!(config.project_name, cloned.project_name);
    }

    #[test]
    fn test_fabryk_config_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<FabrykConfig>();
    }
}
