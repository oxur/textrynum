//! Generic authentication primitives for Fabryk.
//!
//! Provides:
//! - [`AuthenticatedUser`] — Identity extracted from a validated token
//! - [`TokenValidator`] — Trait for async token validation (implement per provider)
//! - [`AuthLayer`] / [`AuthService`] — Tower middleware parameterised over `TokenValidator`
//! - [`AuthConfig`] — Configuration for the auth layer
//! - [`AuthError`] — Auth-specific error types

mod error;
mod middleware;
mod user;

pub use error::AuthError;
pub use middleware::{AuthLayer, AuthService};
pub use user::{email_from_parts, user_from_parts, AuthenticatedUser};

/// Configuration for the auth middleware.
#[derive(Clone, Debug, Default)]
pub struct AuthConfig {
    /// Whether authentication is enabled. When false, all requests pass through.
    pub enabled: bool,
    /// Expected audience (e.g., OAuth client ID).
    pub audience: String,
    /// Allowed email domain (e.g., "banyan.com"). Empty string means any domain.
    pub domain: String,
}

/// Trait for validating tokens and extracting user identity.
///
/// Implement this for each identity provider (Google, Auth0, etc.).
/// The middleware calls `validate()` with the bearer token and returns
/// the authenticated user on success.
pub trait TokenValidator: Send + Sync + 'static {
    /// Validate a token and return the authenticated user.
    fn validate(
        &self,
        token: &str,
        config: &AuthConfig,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<AuthenticatedUser, AuthError>> + Send + '_>,
    >;
}
