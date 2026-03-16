//! Reusable configuration section structs for Fabryk-based MCP servers.
//!
//! These types represent common configuration concerns shared across
//! multiple Fabryk applications. Projects embed them as named fields
//! in their domain-specific `Config` struct.

use fabryk_core::{Error, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// TLS configuration
// ============================================================================

/// TLS configuration with paired cert/key validation.
///
/// Both `cert_path` and `key_path` must be set together (or both unset).
/// When set, the files must exist on disk.
///
/// # Example
///
/// ```
/// use fabryk_cli::config_sections::TlsConfig;
///
/// let tls = TlsConfig::default();
/// assert!(!tls.enabled());
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Path to TLS certificate PEM file. When set with key_path, enables HTTPS.
    #[serde(default)]
    pub cert_path: Option<String>,

    /// Path to TLS private key PEM file. When set with cert_path, enables HTTPS.
    #[serde(default)]
    pub key_path: Option<String>,
}

impl TlsConfig {
    /// Returns true if TLS is configured (both cert and key paths are set and non-empty).
    pub fn enabled(&self) -> bool {
        self.cert_path.as_ref().is_some_and(|p| !p.is_empty())
            && self.key_path.as_ref().is_some_and(|p| !p.is_empty())
    }

    /// Validate TLS configuration.
    ///
    /// Checks that:
    /// - cert_path and key_path are both set or both unset
    /// - When set, both files exist on disk
    ///
    /// Returns `Ok(())` on success, or an error describing the issue.
    pub fn validate(&self) -> Result<()> {
        let has_cert = self.cert_path.as_ref().is_some_and(|p| !p.is_empty());
        let has_key = self.key_path.as_ref().is_some_and(|p| !p.is_empty());

        if has_cert != has_key {
            return Err(Error::config(
                "tls.cert_path and tls.key_path must both be set (or both unset). \
                 Only one was provided.",
            ));
        }

        if has_cert && has_key {
            let cert_path = self.cert_path.as_ref().unwrap();
            let key_path = self.key_path.as_ref().unwrap();

            if !std::path::Path::new(cert_path).exists() {
                return Err(Error::config(format!(
                    "tls.cert_path '{cert_path}' does not exist"
                )));
            }
            if !std::path::Path::new(key_path).exists() {
                return Err(Error::config(format!(
                    "tls.key_path '{key_path}' does not exist"
                )));
            }

            log::info!("TLS enabled — cert: {cert_path}, key: {key_path}");
        }

        Ok(())
    }
}

// ============================================================================
// OAuth2 configuration
// ============================================================================

/// Default Google JWKS URL for fetching public keys.
fn default_jwks_url() -> String {
    "https://www.googleapis.com/oauth2/v3/certs".to_string()
}

/// OAuth2 authentication configuration (Google-style).
///
/// When `enabled` is true, `client_id` is required. The `domain` field
/// optionally restricts access to a specific email domain.
///
/// # Example
///
/// ```
/// use fabryk_cli::config_sections::OAuthConfig;
///
/// let oauth = OAuthConfig::default();
/// assert!(!oauth.enabled);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    /// Enable OAuth2 authentication (default: false for dev mode).
    #[serde(default)]
    pub enabled: bool,

    /// Google OAuth2 client ID (required when enabled is true).
    #[serde(default)]
    pub client_id: String,

    /// Allowed email domain (e.g., "banyan.com").
    #[serde(default)]
    pub domain: String,

    /// JWKS URL for fetching Google public keys.
    #[serde(default = "default_jwks_url")]
    pub jwks_url: String,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            client_id: String::new(),
            domain: String::new(),
            jwks_url: default_jwks_url(),
        }
    }
}

impl OAuthConfig {
    /// Validate OAuth configuration.
    ///
    /// When enabled:
    /// - `client_id` must be non-empty (hard error)
    /// - `domain` should be set (logged warning)
    ///
    /// When disabled, no validation is performed.
    pub fn validate(&self, env_prefix: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if self.client_id.is_empty() {
            return Err(Error::config(format!(
                "oauth.client_id is required when oauth.enabled is true. \
                 Set {env_prefix}_OAUTH_CLIENT_ID or [oauth] client_id in config file."
            )));
        }

        if self.domain.is_empty() {
            log::warn!(
                "oauth.domain is not set — any Google account can authenticate. \
                 Set {env_prefix}_OAUTH_DOMAIN to restrict access."
            );
        }

        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- TlsConfig tests --

    #[test]
    fn test_tls_default_disabled() {
        let tls = TlsConfig::default();
        assert!(!tls.enabled());
    }

    #[test]
    fn test_tls_enabled_both_set() {
        let tls = TlsConfig {
            cert_path: Some("/path/to/cert.pem".to_string()),
            key_path: Some("/path/to/key.pem".to_string()),
        };
        assert!(tls.enabled());
    }

    #[test]
    fn test_tls_disabled_empty_strings() {
        let tls = TlsConfig {
            cert_path: Some(String::new()),
            key_path: Some(String::new()),
        };
        assert!(!tls.enabled());
    }

    #[test]
    fn test_tls_disabled_only_cert() {
        let tls = TlsConfig {
            cert_path: Some("/path/cert.pem".to_string()),
            key_path: None,
        };
        assert!(!tls.enabled());
    }

    #[test]
    fn test_tls_validate_both_unset_ok() {
        let tls = TlsConfig::default();
        assert!(tls.validate().is_ok());
    }

    #[test]
    fn test_tls_validate_asymmetric_error() {
        let tls = TlsConfig {
            cert_path: Some("/path/cert.pem".to_string()),
            key_path: None,
        };
        let err = tls.validate().unwrap_err();
        assert!(err.to_string().contains("both be set"));
    }

    #[test]
    fn test_tls_validate_nonexistent_cert() {
        let tls = TlsConfig {
            cert_path: Some("/nonexistent/cert.pem".to_string()),
            key_path: Some("/nonexistent/key.pem".to_string()),
        };
        let err = tls.validate().unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn test_tls_validate_existing_files() {
        let dir = tempfile::TempDir::new().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, "cert").unwrap();
        std::fs::write(&key, "key").unwrap();

        let tls = TlsConfig {
            cert_path: Some(cert.to_str().unwrap().to_string()),
            key_path: Some(key.to_str().unwrap().to_string()),
        };
        assert!(tls.validate().is_ok());
    }

    // -- OAuthConfig tests --

    #[test]
    fn test_oauth_default_disabled() {
        let oauth = OAuthConfig::default();
        assert!(!oauth.enabled);
        assert!(oauth.client_id.is_empty());
        assert!(!oauth.jwks_url.is_empty());
    }

    #[test]
    fn test_oauth_validate_disabled_ok() {
        let oauth = OAuthConfig::default();
        assert!(oauth.validate("APP").is_ok());
    }

    #[test]
    fn test_oauth_validate_enabled_no_client_id() {
        let oauth = OAuthConfig {
            enabled: true,
            ..Default::default()
        };
        let err = oauth.validate("APP").unwrap_err();
        assert!(err.to_string().contains("client_id"));
    }

    #[test]
    fn test_oauth_validate_enabled_with_client_id() {
        let oauth = OAuthConfig {
            enabled: true,
            client_id: "my-client-id".to_string(),
            domain: "example.com".to_string(),
            ..Default::default()
        };
        assert!(oauth.validate("APP").is_ok());
    }

    #[test]
    fn test_oauth_default_jwks_url() {
        let oauth = OAuthConfig::default();
        assert!(oauth.jwks_url.contains("googleapis.com"));
    }

    // -- Serialization round-trip --

    #[test]
    fn test_tls_config_serde_roundtrip() {
        let tls = TlsConfig {
            cert_path: Some("/cert.pem".to_string()),
            key_path: Some("/key.pem".to_string()),
        };
        let toml_str = toml::to_string_pretty(&tls).unwrap();
        let parsed: TlsConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.cert_path, tls.cert_path);
        assert_eq!(parsed.key_path, tls.key_path);
    }

    #[test]
    fn test_oauth_config_serde_roundtrip() {
        let oauth = OAuthConfig {
            enabled: true,
            client_id: "test-id".to_string(),
            domain: "test.com".to_string(),
            ..Default::default()
        };
        let toml_str = toml::to_string_pretty(&oauth).unwrap();
        let parsed: OAuthConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.enabled, oauth.enabled);
        assert_eq!(parsed.client_id, oauth.client_id);
        assert_eq!(parsed.domain, oauth.domain);
        assert_eq!(parsed.jwks_url, oauth.jwks_url);
    }
}
