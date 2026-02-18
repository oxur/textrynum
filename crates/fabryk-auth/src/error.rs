//! Auth-specific error types.

/// Errors that can occur during authentication.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// No Authorization header or bearer token present.
    #[error("missing authentication token")]
    MissingToken,

    /// Token format is invalid (not a valid JWT).
    #[error("invalid token format: {0}")]
    InvalidFormat(String),

    /// JWT signature verification failed.
    #[error("invalid token signature: {0}")]
    InvalidSignature(String),

    /// Token has expired.
    #[error("token has expired")]
    Expired,

    /// Token audience doesn't match configured client ID.
    #[error("invalid audience")]
    InvalidAudience,

    /// Token domain doesn't match configured domain.
    #[error("invalid domain: got '{domain}', expected '{expected}'")]
    InvalidDomain { domain: String, expected: String },

    /// Token is missing the email claim.
    #[error("token missing email claim")]
    MissingEmail,

    /// Failed to fetch JWKS from the identity provider.
    #[error("failed to fetch JWKS: {0}")]
    JwksFetchError(String),

    /// No key in the JWKS matches the token's kid.
    #[error("no matching key for kid '{0}'")]
    NoMatchingKey(String),
}

impl AuthError {
    /// Whether this error should result in a 401 (vs. a 500).
    pub fn is_client_error(&self) -> bool {
        matches!(
            self,
            AuthError::MissingToken
                | AuthError::InvalidFormat(_)
                | AuthError::InvalidSignature(_)
                | AuthError::Expired
                | AuthError::InvalidAudience
                | AuthError::InvalidDomain { .. }
                | AuthError::MissingEmail
                | AuthError::NoMatchingKey(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_display() {
        let e = AuthError::MissingToken;
        assert_eq!(e.to_string(), "missing authentication token");
    }

    #[test]
    fn test_auth_error_invalid_domain_display() {
        let e = AuthError::InvalidDomain {
            domain: "other.com".to_string(),
            expected: "banyan.com".to_string(),
        };
        assert_eq!(
            e.to_string(),
            "invalid domain: got 'other.com', expected 'banyan.com'"
        );
    }

    #[test]
    fn test_is_client_error() {
        assert!(AuthError::MissingToken.is_client_error());
        assert!(AuthError::Expired.is_client_error());
        // JwksFetchError is a server-side issue, not a client error
        assert!(!AuthError::JwksFetchError("err".into()).is_client_error());
    }
}
