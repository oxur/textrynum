//! Shared configuration utilities for Fabryk-based applications.
//!
//! Provides path resolution, TOML flattening, and environment variable
//! generation utilities that eliminate boilerplate across projects.

use std::path::{Path, PathBuf};

// ============================================================================
// Path resolution
// ============================================================================

/// Determine the base directory for resolving relative config paths.
///
/// Priority:
/// 1. Environment variable named `env_var_name` (resolved against CWD if relative)
/// 2. Config file's parent directory
/// 3. Current working directory
///
/// # Arguments
///
/// * `env_var_name` — e.g., `"TAPROOT_BASE_DIR"` or `"KASU_BASE_DIR"`
/// * `config_path` — the resolved config file path, if known
///
/// # Examples
///
/// ```
/// use fabryk_cli::config_utils::resolve_base_dir;
///
/// let base = resolve_base_dir("MY_APP_BASE_DIR", None);
/// assert!(base.is_absolute() || base == std::path::PathBuf::from("."));
/// ```
pub fn resolve_base_dir(env_var_name: &str, config_path: Option<&Path>) -> PathBuf {
    // 1. Explicit env var override
    if let Ok(env_dir) = std::env::var(env_var_name) {
        let p = PathBuf::from(&env_dir);
        if p.is_absolute() {
            log::debug!("base_dir from {env_var_name}: {}", p.display());
            return p;
        }
        // Relative env var → resolve against CWD
        if let Ok(cwd) = std::env::current_dir() {
            let resolved = cwd.join(&p);
            log::debug!(
                "base_dir from {env_var_name} (relative → absolute): {}",
                resolved.display()
            );
            return resolved;
        }
    }

    // 2. Config file's parent directory
    if let Some(cfg) = config_path {
        if let Some(parent) = cfg.parent() {
            if !parent.as_os_str().is_empty() {
                log::debug!("base_dir from config file parent: {}", parent.display());
                return parent.to_path_buf();
            }
        }
    }

    // 3. CWD fallback
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    log::debug!("base_dir from CWD fallback: {}", cwd.display());
    cwd
}

/// Resolve a single path against a base directory.
///
/// - Empty strings pass through unchanged.
/// - Absolute paths pass through unchanged.
/// - Relative paths are joined with `base`.
pub fn resolve_path(base: &Path, path: &str) -> String {
    if path.is_empty() {
        return path.to_string();
    }
    let p = Path::new(path);
    if p.is_absolute() {
        return path.to_string();
    }
    base.join(p).to_string_lossy().to_string()
}

/// Resolve an `Option<String>` path field in place, logging when a path changes.
///
/// Leaves `None` and empty strings unchanged. Resolves relative paths
/// against `base` and logs the transformation at debug level.
pub fn resolve_opt_path(field: &mut Option<String>, base: &Path, field_name: &str) {
    if let Some(val) = field {
        if !val.is_empty() {
            let resolved = resolve_path(base, val);
            if resolved != *val {
                log::debug!("resolved {field_name}: {val} → {resolved}");
                *val = resolved;
            }
        }
    }
}

// ============================================================================
// TOML → environment variable flattening
// ============================================================================

/// Recursively flatten a TOML value tree into `KEY=value` pairs.
///
/// Tables are expanded with `_` separators and keys are uppercased.
/// Hyphens in keys are replaced with underscores (env vars cannot contain hyphens).
/// Arrays are serialized as JSON (e.g., for table allowlists).
///
/// # Arguments
///
/// * `value` — the TOML value to flatten
/// * `prefix` — the prefix for env var names (e.g., `"TAPROOT"`)
/// * `out` — accumulator for `(KEY, VALUE)` pairs
///
/// # Examples
///
/// ```
/// let val: toml::Value = toml::from_str("[bq]\nproject = \"my-project\"").unwrap();
/// let mut vars = Vec::new();
/// fabryk_cli::config_utils::flatten_toml_value(&val, "APP", &mut vars);
/// assert_eq!(vars, vec![("APP_BQ_PROJECT".to_string(), "my-project".to_string())]);
/// ```
pub fn flatten_toml_value(value: &toml::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let env_key = format!("{}_{}", prefix, key.to_uppercase().replace('-', "_"));
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
    use std::sync::Mutex;

    /// Serialize env-mutating tests.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self {
                key: key.to_string(),
                prev,
            }
        }

        fn remove(key: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            if let Some(val) = &self.prev {
                unsafe { std::env::set_var(&self.key, val) };
            } else {
                unsafe { std::env::remove_var(&self.key) };
            }
        }
    }

    // -- resolve_base_dir tests --

    #[test]
    fn test_resolve_base_dir_from_env_var_absolute() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new("TEST_BASE_DIR_ABS", "/explicit/base");
        let base = resolve_base_dir("TEST_BASE_DIR_ABS", None);
        assert_eq!(base, PathBuf::from("/explicit/base"));
    }

    #[test]
    fn test_resolve_base_dir_from_env_var_relative() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new("TEST_BASE_DIR_REL", "relative/path");
        let base = resolve_base_dir("TEST_BASE_DIR_REL", None);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(base, cwd.join("relative/path"));
    }

    #[test]
    fn test_resolve_base_dir_from_config_parent() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove("TEST_BASE_DIR_CFG");
        let config_path = Path::new("/home/user/.config/app/config.toml");
        let base = resolve_base_dir("TEST_BASE_DIR_CFG", Some(config_path));
        assert_eq!(base, PathBuf::from("/home/user/.config/app"));
    }

    #[test]
    fn test_resolve_base_dir_cwd_fallback() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::remove("TEST_BASE_DIR_CWD");
        let base = resolve_base_dir("TEST_BASE_DIR_CWD", None);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(base, cwd);
    }

    // -- resolve_path tests --

    #[test]
    fn test_resolve_path_empty_passthrough() {
        let base = Path::new("/base");
        assert_eq!(resolve_path(base, ""), "");
    }

    #[test]
    fn test_resolve_path_absolute_passthrough() {
        let base = Path::new("/base");
        assert_eq!(resolve_path(base, "/absolute/path"), "/absolute/path");
    }

    #[test]
    fn test_resolve_path_relative_joined() {
        let base = Path::new("/base");
        assert_eq!(resolve_path(base, "relative/path"), "/base/relative/path");
    }

    #[test]
    fn test_resolve_path_dot_prefix() {
        let base = Path::new("/base");
        assert_eq!(resolve_path(base, "./local"), "/base/./local");
    }

    #[test]
    fn test_resolve_path_bare_filename() {
        let base = Path::new("/config");
        assert_eq!(resolve_path(base, "cert.pem"), "/config/cert.pem");
    }

    // -- resolve_opt_path tests --

    #[test]
    fn test_resolve_opt_path_none_unchanged() {
        let mut field: Option<String> = None;
        resolve_opt_path(&mut field, Path::new("/base"), "test");
        assert!(field.is_none());
    }

    #[test]
    fn test_resolve_opt_path_empty_unchanged() {
        let mut field = Some(String::new());
        resolve_opt_path(&mut field, Path::new("/base"), "test");
        assert_eq!(field, Some(String::new()));
    }

    #[test]
    fn test_resolve_opt_path_absolute_unchanged() {
        let mut field = Some("/absolute/path".to_string());
        resolve_opt_path(&mut field, Path::new("/base"), "test");
        assert_eq!(field, Some("/absolute/path".to_string()));
    }

    #[test]
    fn test_resolve_opt_path_relative_resolved() {
        let mut field = Some("relative/file".to_string());
        resolve_opt_path(&mut field, Path::new("/base"), "test");
        assert_eq!(field, Some("/base/relative/file".to_string()));
    }

    // -- flatten_toml_value tests --

    #[test]
    fn test_flatten_toml_value_string() {
        let val = toml::Value::String("hello".into());
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(out, vec![("APP".to_string(), "hello".to_string())]);
    }

    #[test]
    fn test_flatten_toml_value_nested_table() {
        let val: toml::Value = toml::from_str("[bq]\nproject = \"my-project\"").unwrap();
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(
            out,
            vec![("APP_BQ_PROJECT".to_string(), "my-project".to_string())]
        );
    }

    #[test]
    fn test_flatten_toml_value_array() {
        let val: toml::Value = toml::from_str("items = [\"a\", \"b\"]").unwrap();
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(
            out,
            vec![("APP_ITEMS".to_string(), "[\"a\",\"b\"]".to_string())]
        );
    }

    #[test]
    fn test_flatten_toml_value_boolean() {
        let val: toml::Value = toml::from_str("enabled = true").unwrap();
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(out, vec![("APP_ENABLED".to_string(), "true".to_string())]);
    }

    #[test]
    fn test_flatten_toml_value_integer() {
        let val: toml::Value = toml::from_str("port = 8080").unwrap();
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(out, vec![("APP_PORT".to_string(), "8080".to_string())]);
    }

    #[test]
    fn test_flatten_toml_value_hyphen_to_underscore() {
        let val: toml::Value = toml::from_str("[my-section]\nmy-key = \"value\"").unwrap();
        let mut out = Vec::new();
        flatten_toml_value(&val, "APP", &mut out);
        assert_eq!(
            out,
            vec![("APP_MY_SECTION_MY_KEY".to_string(), "value".to_string())]
        );
    }
}
