//! GCP credential detection and utilities for the Fabryk ecosystem.
//!
//! Provides Application Default Credentials (ADC) resolution that works
//! reliably across local development, CI, and Cloud Run environments.
//!
//! # Usage
//!
//! ```rust,no_run
//! use fabryk_gcp::{resolve_adc_path, read_credential_type, CredentialType};
//!
//! if let Some(path) = resolve_adc_path() {
//!     match read_credential_type(&path) {
//!         Some(CredentialType::AuthorizedUser) => {
//!             // Use authorized_user flow (local dev via `gcloud auth`)
//!         }
//!         Some(CredentialType::ServiceAccount) => {
//!             // Use service_account flow (CI, domain-wide delegation)
//!         }
//!         _ => {
//!             // Unknown type — try default ADC chain
//!         }
//!     }
//! } else {
//!     // No ADC file — try metadata server (Cloud Run / GCE)
//! }
//! ```

pub mod adc;
pub mod error;

pub use adc::{CredentialType, read_credential_type, resolve_adc_path};
pub use error::{GcpError, Result};
