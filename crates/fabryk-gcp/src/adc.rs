//! Application Default Credentials (ADC) resolution.
//!
//! Smart credential detection that avoids yup-oauth2's default ADC chain,
//! which falls back to the GCE metadata server and hangs on non-GCP machines.
//!
//! Resolution order:
//! 1. `GOOGLE_APPLICATION_CREDENTIALS` environment variable (if file exists)
//! 2. macOS: `~/Library/Application Support/gcloud/application_default_credentials.json`
//! 3. Linux / fallback: `~/.config/gcloud/application_default_credentials.json`

use std::fmt;
use std::path::{Path, PathBuf};

/// The type of credential found in a JSON credential file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialType {
    /// Credentials from `gcloud auth application-default login` (local dev).
    AuthorizedUser,
    /// Service account key file (CI, Cloud Run, domain-wide delegation).
    ServiceAccount,
    /// An unrecognized credential type.
    Other(String),
}

impl fmt::Display for CredentialType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthorizedUser => write!(f, "authorized_user"),
            Self::ServiceAccount => write!(f, "service_account"),
            Self::Other(s) => write!(f, "{s}"),
        }
    }
}

impl From<&str> for CredentialType {
    fn from(s: &str) -> Self {
        match s {
            "authorized_user" => Self::AuthorizedUser,
            "service_account" => Self::ServiceAccount,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Locate the Application Default Credentials file on disk.
///
/// Checks in order:
/// 1. `GOOGLE_APPLICATION_CREDENTIALS` environment variable
/// 2. macOS: `~/Library/Application Support/gcloud/application_default_credentials.json`
/// 3. Linux / fallback: `~/.config/gcloud/application_default_credentials.json`
///
/// Returns `None` if no candidate file exists.
pub fn resolve_adc_path() -> Option<PathBuf> {
    if let Ok(env_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        if !env_path.is_empty() {
            let p = PathBuf::from(&env_path);
            if p.exists() {
                log::debug!("ADC path from GOOGLE_APPLICATION_CREDENTIALS: {env_path}");
                return Some(p);
            }
            log::warn!(
                "GOOGLE_APPLICATION_CREDENTIALS set to '{env_path}' but file does not exist"
            );
        }
    }

    let home = dirs::home_dir()?;

    if cfg!(target_os = "macos") {
        let lib_path =
            home.join("Library/Application Support/gcloud/application_default_credentials.json");
        if lib_path.exists() {
            return Some(lib_path);
        }
    }

    let config_path = home.join(".config/gcloud/application_default_credentials.json");
    if config_path.exists() {
        return Some(config_path);
    }

    None
}

/// Read a JSON credential file and extract the credential type.
///
/// Inspects the `"type"` field of the JSON file to determine whether it is
/// an `authorized_user` credential (from `gcloud auth`) or a `service_account`
/// key file.
///
/// Returns `None` if the file cannot be read, is not valid JSON, or lacks a
/// `"type"` field.
pub fn read_credential_type(path: &Path) -> Option<CredentialType> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| log::warn!("Failed to read credential file {}: {e}", path.display()))
        .ok()?;

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| log::warn!("Failed to parse credential JSON {}: {e}", path.display()))
        .ok()?;

    parsed
        .get("type")
        .and_then(|v| v.as_str())
        .map(CredentialType::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CredentialType ──────────────────────────────────────────────────

    #[test]
    fn test_credential_type_from_str() {
        assert_eq!(
            CredentialType::from("authorized_user"),
            CredentialType::AuthorizedUser
        );
        assert_eq!(
            CredentialType::from("service_account"),
            CredentialType::ServiceAccount
        );
        assert_eq!(
            CredentialType::from("external_account"),
            CredentialType::Other("external_account".to_string())
        );
    }

    #[test]
    fn test_credential_type_display() {
        assert_eq!(
            CredentialType::AuthorizedUser.to_string(),
            "authorized_user"
        );
        assert_eq!(
            CredentialType::ServiceAccount.to_string(),
            "service_account"
        );
        assert_eq!(
            CredentialType::Other("external_account".to_string()).to_string(),
            "external_account"
        );
    }

    // ── read_credential_type ────────────────────────────────────────────

    #[test]
    fn test_read_credential_type_authorized_user() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("creds.json");
        std::fs::write(
            &path,
            r#"{"type": "authorized_user", "client_id": "x", "refresh_token": "y"}"#,
        )
        .unwrap();
        assert_eq!(
            read_credential_type(&path),
            Some(CredentialType::AuthorizedUser)
        );
    }

    #[test]
    fn test_read_credential_type_service_account() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sa.json");
        std::fs::write(&path, r#"{"type": "service_account", "project_id": "p"}"#).unwrap();
        assert_eq!(
            read_credential_type(&path),
            Some(CredentialType::ServiceAccount)
        );
    }

    #[test]
    fn test_read_credential_type_other() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ext.json");
        std::fs::write(&path, r#"{"type": "external_account", "audience": "a"}"#).unwrap();
        assert_eq!(
            read_credential_type(&path),
            Some(CredentialType::Other("external_account".to_string()))
        );
    }

    #[test]
    fn test_read_credential_type_missing_type_field() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("no_type.json");
        std::fs::write(&path, r#"{"client_id": "x"}"#).unwrap();
        assert_eq!(read_credential_type(&path), None);
    }

    #[test]
    fn test_read_credential_type_nonexistent_file() {
        let path = Path::new("/tmp/fabryk-gcp-test-does-not-exist-98765.json");
        assert_eq!(read_credential_type(path), None);
    }

    #[test]
    fn test_read_credential_type_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "not json at all").unwrap();
        assert_eq!(read_credential_type(&path), None);
    }

    // ── resolve_adc_path ────────────────────────────────────────────────

    /// Mutex to serialize tests that manipulate environment variables.
    /// `std::env::set_var` / `remove_var` are process-global and unsafe in
    /// Rust 2024; concurrent tests must not race on the same env key.
    static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// RAII guard that sets an env var and restores it on drop.
    struct EnvGuard {
        key: String,
        prev: Option<String>,
    }

    impl EnvGuard {
        fn new(key: &str, val: &str) -> Self {
            let prev = std::env::var(key).ok();
            unsafe { std::env::set_var(key, val) };
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => unsafe { std::env::set_var(&self.key, v) },
                None => unsafe { std::env::remove_var(&self.key) },
            }
        }
    }

    #[test]
    fn test_resolve_adc_path_from_env() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("adc.json");
        std::fs::write(&path, r#"{"type":"authorized_user"}"#).unwrap();

        let _guard = EnvGuard::new("GOOGLE_APPLICATION_CREDENTIALS", path.to_str().unwrap());

        let result = resolve_adc_path();
        assert_eq!(result, Some(path));
    }

    #[test]
    fn test_resolve_adc_path_env_nonexistent_file() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new(
            "GOOGLE_APPLICATION_CREDENTIALS",
            "/tmp/fabryk-gcp-test-no-such-file-12345.json",
        );

        // Should NOT return the nonexistent path — falls through to well-known
        let result = resolve_adc_path();
        if let Some(p) = &result {
            assert_ne!(
                p.to_str().unwrap(),
                "/tmp/fabryk-gcp-test-no-such-file-12345.json"
            );
        }
    }

    #[test]
    fn test_resolve_adc_path_env_empty_string() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let _guard = EnvGuard::new("GOOGLE_APPLICATION_CREDENTIALS", "");

        // Empty string should be treated as unset — falls through to well-known
        let result = resolve_adc_path();
        if let Some(p) = &result {
            assert!(!p.to_string_lossy().is_empty());
        }
    }
}
