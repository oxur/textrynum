//! Shared credential and token types for Google Cloud APIs.

use serde::Deserialize;

/// Google OAuth2 token endpoint.
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// OAuth2 token response from Google's token endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    /// The access token.
    pub access_token: String,

    /// Token lifetime in seconds.
    pub expires_in: Option<u64>,

    /// Token type (usually "Bearer").
    #[serde(default)]
    pub token_type: Option<String>,
}

/// Service account key file structure.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    /// The service account email.
    pub client_email: String,

    /// The RSA private key in PEM format.
    pub private_key: String,

    /// Token URI for JWT exchange.
    pub token_uri: String,
}

/// Authorized user credentials (from `gcloud auth application-default login`).
#[derive(Debug, Clone, Deserialize)]
pub struct AuthorizedUserCredentials {
    /// OAuth2 client ID.
    pub client_id: String,

    /// OAuth2 client secret.
    pub client_secret: String,

    /// Refresh token.
    pub refresh_token: String,

    /// Credential type (expected: "authorized_user").
    #[serde(rename = "type")]
    pub credential_type: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_token_response_deserialize() {
        let json = r#"{"access_token": "ya29.test", "expires_in": 3600}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "ya29.test");
        assert_eq!(resp.expires_in, Some(3600));
    }

    #[test]
    fn test_token_response_with_token_type() {
        let json = r#"{"access_token": "ya29.test", "expires_in": 3600, "token_type": "Bearer"}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.token_type.as_deref(), Some("Bearer"));
    }

    #[test]
    fn test_service_account_key_deserialize() {
        let json = r#"{
            "client_email": "sa@project.iam.gserviceaccount.com",
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\ntest\n-----END RSA PRIVATE KEY-----\n",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;
        let key: ServiceAccountKey = serde_json::from_str(json).unwrap();
        assert_eq!(key.client_email, "sa@project.iam.gserviceaccount.com");
        assert!(key.private_key.contains("BEGIN RSA PRIVATE KEY"));
    }

    #[test]
    fn test_authorized_user_credentials_deserialize() {
        let json = r#"{
            "client_id": "xxx.apps.googleusercontent.com",
            "client_secret": "secret",
            "refresh_token": "1//refresh",
            "type": "authorized_user"
        }"#;
        let cred: AuthorizedUserCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(cred.credential_type, "authorized_user");
        assert_eq!(cred.refresh_token, "1//refresh");
    }
}
