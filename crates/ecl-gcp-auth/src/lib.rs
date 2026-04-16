//! Shared Google Cloud OAuth2 token management for ECL pipeline adapters.
//!
//! Provides a [`TokenProvider`] that handles service account JWT flows,
//! environment variable tokens, Application Default Credentials, and
//! refresh token flows. Used by `ecl-adapter-gcs` and `ecl-adapter-gdrive`.

pub mod error;
pub mod types;

mod auth;

pub use auth::{TokenProvider, well_known_adc_path};
pub use error::GcpAuthError;
pub use types::{AuthorizedUserCredentials, GOOGLE_TOKEN_URL, ServiceAccountKey, TokenResponse};
