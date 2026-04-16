//! OAuth2 token management for GCS API access.
//!
//! Delegates to [`ecl_gcp_auth::TokenProvider`] for all credential flows.

pub use ecl_gcp_auth::TokenProvider;

/// Create a GCS-specific token provider with the GCS read-only scope.
pub fn gcs_token_provider(
    credential_ref: ecl_pipeline_spec::CredentialRef,
    http_client: reqwest::Client,
) -> TokenProvider {
    TokenProvider::new(
        credential_ref,
        http_client,
        crate::types::GCS_READONLY_SCOPE,
    )
}
