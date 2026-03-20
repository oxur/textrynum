//! OAuth2 token management for GCS API access.
//!
//! Supports three credential types:
//! - **Service account** (`CredentialRef::File`): JWT assertion → token endpoint
//! - **Environment variable** (`CredentialRef::EnvVar`): raw bearer token from env
//! - **Application Default Credentials** (`CredentialRef::ApplicationDefault`):
//!   checks `GOOGLE_APPLICATION_CREDENTIALS` env var, then well-known gcloud path

use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use tracing::debug;

use ecl_pipeline_spec::CredentialRef;

use crate::error::GcsAdapterError;
use crate::types::{
    AuthorizedUserCredentials, GCS_READONLY_SCOPE, GOOGLE_TOKEN_URL, ServiceAccountKey,
    TokenResponse,
};

/// Manages OAuth2 tokens for GCS API access.
///
/// Caches tokens and refreshes them before expiry. Thread-safe via `RwLock`.
#[derive(Debug)]
pub struct TokenProvider {
    credential_ref: CredentialRef,
    http_client: reqwest::Client,
    cached: Arc<RwLock<Option<CachedToken>>>,
    /// Override token endpoint URL (for testing).
    token_url_override: Option<String>,
    /// OAuth2 scope to request. Defaults to GCS read-only.
    scope: String,
}

/// A cached access token with expiry tracking.
#[derive(Debug, Clone)]
struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

/// JWT claims for Google OAuth2 service account assertion.
#[derive(Debug, serde::Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: i64,
    iat: i64,
}

impl TokenProvider {
    /// Create a new token provider for the given credential reference.
    pub fn new(credential_ref: CredentialRef, http_client: reqwest::Client) -> Self {
        Self {
            credential_ref,
            http_client,
            cached: Arc::new(RwLock::new(None)),
            token_url_override: None,
            scope: GCS_READONLY_SCOPE.to_string(),
        }
    }

    /// Create a token provider that always returns a static token.
    /// Useful for testing enumerate without auth setup.
    pub fn static_token(token: String) -> Self {
        let cached = CachedToken {
            access_token: token,
            expires_at: DateTime::<Utc>::MAX_UTC,
        };
        Self {
            credential_ref: CredentialRef::ApplicationDefault,
            http_client: reqwest::Client::new(),
            cached: Arc::new(RwLock::new(Some(cached))),
            token_url_override: None,
            scope: GCS_READONLY_SCOPE.to_string(),
        }
    }

    /// Override the token endpoint URL (for testing with wiremock).
    pub fn with_token_url(mut self, url: String) -> Self {
        self.token_url_override = Some(url);
        self
    }

    /// Override the OAuth2 scope (e.g., for read-write access in sinks).
    pub fn with_scope(mut self, scope: String) -> Self {
        self.scope = scope;
        self
    }

    /// Get a valid access token, refreshing if necessary.
    ///
    /// Returns a cached token if still valid (with 30-second buffer).
    pub async fn get_token(&self) -> Result<String, GcsAdapterError> {
        {
            let cached = self.cached.read().await;
            if let Some(token) = cached.as_ref()
                && token.expires_at > Utc::now() + chrono::Duration::seconds(30)
            {
                return Ok(token.access_token.clone());
            }
        }

        debug!("refreshing GCS access token");
        let token = self.refresh_token().await?;
        let access_token = token.access_token.clone();

        let mut cached = self.cached.write().await;
        *cached = Some(token);
        Ok(access_token)
    }

    /// Resolve credentials and obtain a fresh token.
    async fn refresh_token(&self) -> Result<CachedToken, GcsAdapterError> {
        match &self.credential_ref {
            CredentialRef::File { path } => self.service_account_flow(path).await,
            CredentialRef::EnvVar { env } => Self::env_var_flow(env),
            CredentialRef::ApplicationDefault => self.adc_flow().await,
            CredentialRef::Secret { name } => Err(GcsAdapterError::InvalidCredentials {
                message: format!("secret '{name}' must be resolved before adapter construction"),
            }),
        }
    }

    /// Service account flow: read key file → sign JWT → exchange for token.
    async fn service_account_flow(&self, path: &PathBuf) -> Result<CachedToken, GcsAdapterError> {
        let key_json = tokio::fs::read_to_string(path).await.map_err(|e| {
            GcsAdapterError::InvalidCredentials {
                message: format!(
                    "failed to read service account key '{}': {e}",
                    path.display()
                ),
            }
        })?;

        let key: ServiceAccountKey =
            serde_json::from_str(&key_json).map_err(|e| GcsAdapterError::InvalidCredentials {
                message: format!("invalid service account key JSON: {e}"),
            })?;

        let token_url = self.token_url_override.as_deref().unwrap_or(&key.token_uri);
        let jwt = Self::create_service_account_jwt(&key, token_url, &self.scope)?;
        self.exchange_jwt_for_token(&jwt, token_url).await
    }

    /// Create a signed JWT assertion for a service account.
    fn create_service_account_jwt(
        key: &ServiceAccountKey,
        token_url: &str,
        scope: &str,
    ) -> Result<String, GcsAdapterError> {
        let now = Utc::now();
        let claims = JwtClaims {
            iss: key.client_email.clone(),
            scope: scope.to_string(),
            aud: token_url.to_string(),
            iat: now.timestamp(),
            exp: (now + chrono::Duration::seconds(3600)).timestamp(),
        };

        let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(key.private_key.as_bytes())
            .map_err(|e| GcsAdapterError::Auth {
                message: format!("invalid RSA private key: {e}"),
            })?;

        let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
        jsonwebtoken::encode(&header, &claims, &encoding_key).map_err(|e| GcsAdapterError::Auth {
            message: format!("JWT encoding failed: {e}"),
        })
    }

    /// Exchange a JWT assertion for an access token via HTTP POST.
    async fn exchange_jwt_for_token(
        &self,
        jwt: &str,
        token_url: &str,
    ) -> Result<CachedToken, GcsAdapterError> {
        let response = self
            .http_client
            .post(token_url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", jwt),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(GcsAdapterError::Auth {
                message: format!("token exchange failed ({status}): {body}"),
            });
        }

        let token_resp: TokenResponse = response.json().await?;
        let expires_in = token_resp.expires_in.unwrap_or(3600);

        Ok(CachedToken {
            access_token: token_resp.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in as i64),
        })
    }

    /// Environment variable flow: read bearer token directly from env.
    fn env_var_flow(env_var: &str) -> Result<CachedToken, GcsAdapterError> {
        let token = std::env::var(env_var).map_err(|_| GcsAdapterError::Auth {
            message: format!("environment variable '{env_var}' is not set"),
        })?;

        if token.is_empty() {
            return Err(GcsAdapterError::Auth {
                message: format!("environment variable '{env_var}' is empty"),
            });
        }

        Ok(CachedToken {
            access_token: token,
            expires_at: Utc::now() + chrono::Duration::seconds(3600),
        })
    }

    /// Application Default Credentials flow.
    ///
    /// Checks in order:
    /// 1. `GOOGLE_APPLICATION_CREDENTIALS` env var (path to key file)
    /// 2. Well-known gcloud path (`~/.config/gcloud/application_default_credentials.json`)
    async fn adc_flow(&self) -> Result<CachedToken, GcsAdapterError> {
        if let Ok(cred_path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            let path = PathBuf::from(&cred_path);
            debug!(path = %path.display(), "using GOOGLE_APPLICATION_CREDENTIALS for GCS");
            return self.resolve_credential_file(&path).await;
        }

        if let Some(path) = well_known_adc_path()
            && path.exists()
        {
            debug!(path = %path.display(), "using well-known ADC path for GCS");
            return self.resolve_credential_file(&path).await;
        }

        Err(GcsAdapterError::Auth {
            message:
                "no Application Default Credentials found: set GOOGLE_APPLICATION_CREDENTIALS \
                      or run `gcloud auth application-default login`"
                    .to_string(),
        })
    }

    /// Detect credential type from file content and obtain a token.
    async fn resolve_credential_file(
        &self,
        path: &PathBuf,
    ) -> Result<CachedToken, GcsAdapterError> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            GcsAdapterError::InvalidCredentials {
                message: format!("failed to read credentials file '{}': {e}", path.display()),
            }
        })?;

        // Try parsing as service account key first.
        if let Ok(key) = serde_json::from_str::<ServiceAccountKey>(&content) {
            let token_url = self.token_url_override.as_deref().unwrap_or(&key.token_uri);
            let jwt = Self::create_service_account_jwt(&key, token_url, &self.scope)?;
            return self.exchange_jwt_for_token(&jwt, token_url).await;
        }

        // Try parsing as authorized user credentials.
        if let Ok(user_creds) = serde_json::from_str::<AuthorizedUserCredentials>(&content)
            && user_creds.credential_type == "authorized_user"
        {
            return self.refresh_token_flow(&user_creds).await;
        }

        Err(GcsAdapterError::InvalidCredentials {
            message: format!("unrecognized credential format in '{}'", path.display()),
        })
    }

    /// Refresh token flow for authorized_user credentials.
    async fn refresh_token_flow(
        &self,
        creds: &AuthorizedUserCredentials,
    ) -> Result<CachedToken, GcsAdapterError> {
        let token_url = self
            .token_url_override
            .as_deref()
            .unwrap_or(GOOGLE_TOKEN_URL);

        let response = self
            .http_client
            .post(token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &creds.client_id),
                ("client_secret", &creds.client_secret),
                ("refresh_token", &creds.refresh_token),
            ])
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(GcsAdapterError::Auth {
                message: format!("refresh token exchange failed ({status}): {body}"),
            });
        }

        let token_resp: TokenResponse = response.json().await?;
        let expires_in = token_resp.expires_in.unwrap_or(3600);

        Ok(CachedToken {
            access_token: token_resp.access_token,
            expires_at: Utc::now() + chrono::Duration::seconds(expires_in as i64),
        })
    }
}

/// Get the well-known path for Application Default Credentials.
fn well_known_adc_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("gcloud/application_default_credentials.json"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_static_token_returns_immediately() {
        let provider = TokenProvider::static_token("test-token-gcs".to_string());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let token = rt.block_on(provider.get_token()).unwrap();
        assert_eq!(token, "test-token-gcs");
    }

    #[test]
    fn test_env_var_flow_missing_var() {
        let result = TokenProvider::env_var_flow("ECL_TEST_ABSOLUTELY_NONEXISTENT_VAR_GCS_12345");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("not set"));
    }

    #[test]
    fn test_well_known_adc_path_exists() {
        let path = well_known_adc_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("gcloud"));
    }

    #[tokio::test]
    async fn test_service_account_flow_invalid_file() {
        let provider = TokenProvider::new(
            CredentialRef::File {
                path: PathBuf::from("/nonexistent/gcs-path.json"),
            },
            reqwest::Client::new(),
        );
        let result = provider.get_token().await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("failed to read"));
    }

    #[tokio::test]
    async fn test_service_account_flow_invalid_json() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "not valid json").unwrap();

        let provider = TokenProvider::new(
            CredentialRef::File {
                path: tmp.path().to_path_buf(),
            },
            reqwest::Client::new(),
        );
        let result = provider.get_token().await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("invalid service account key"));
    }

    #[tokio::test]
    async fn test_adc_flow_with_nonexistent_env_path() {
        let provider =
            TokenProvider::new(CredentialRef::ApplicationDefault, reqwest::Client::new());

        let result = provider
            .resolve_credential_file(&PathBuf::from("/nonexistent/gcs-credentials.json"))
            .await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("failed to read"));
    }

    #[tokio::test]
    async fn test_token_caching() {
        let provider = TokenProvider::static_token("cached-gcs-token".to_string());

        let t1 = provider.get_token().await.unwrap();
        let t2 = provider.get_token().await.unwrap();
        assert_eq!(t1, "cached-gcs-token");
        assert_eq!(t2, "cached-gcs-token");
    }

    #[tokio::test]
    async fn test_exchange_jwt_wiremock() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": "ya29.gcs-test-token",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })),
            )
            .mount(&mock_server)
            .await;

        let provider =
            TokenProvider::new(CredentialRef::ApplicationDefault, reqwest::Client::new());

        let token_url = format!("{}/token", mock_server.uri());
        let result = provider
            .exchange_jwt_for_token("fake-jwt-assertion", &token_url)
            .await
            .unwrap();

        assert_eq!(result.access_token, "ya29.gcs-test-token");
    }

    #[tokio::test]
    async fn test_exchange_jwt_error_response() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(400)
                    .set_body_string(r#"{"error": "invalid_grant"}"#),
            )
            .mount(&mock_server)
            .await;

        let provider =
            TokenProvider::new(CredentialRef::ApplicationDefault, reqwest::Client::new());

        let token_url = format!("{}/token", mock_server.uri());
        let result = provider.exchange_jwt_for_token("bad-jwt", &token_url).await;

        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("400"));
    }

    #[tokio::test]
    async fn test_refresh_token_flow_wiremock() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/token"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "access_token": "ya29.gcs-refreshed",
                    "expires_in": 3600,
                    "token_type": "Bearer"
                })),
            )
            .mount(&mock_server)
            .await;

        let provider =
            TokenProvider::new(CredentialRef::ApplicationDefault, reqwest::Client::new())
                .with_token_url(format!("{}/token", mock_server.uri()));

        let creds = AuthorizedUserCredentials {
            client_id: "test-client-id".to_string(),
            client_secret: "test-secret".to_string(),
            refresh_token: "test-refresh-token".to_string(),
            credential_type: "authorized_user".to_string(),
        };

        let result = provider.refresh_token_flow(&creds).await.unwrap();
        assert_eq!(result.access_token, "ya29.gcs-refreshed");
    }
}
