//! File lifecycle management configuration.
//!
//! Defines how processed files move through GCS prefixes:
//! `staging/` → `historical/{run_id}/` on success, or back to `input/` on failure.

use serde::{Deserialize, Serialize};

use crate::source::CredentialRef;

/// File lifecycle management specification.
///
/// Controls automatic movement of source files after pipeline execution:
/// - **On success**: move from staging to historical (or delete)
/// - **On failure**: move back to input (or to error prefix)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleSpec {
    /// GCS bucket for lifecycle operations.
    pub bucket: String,

    /// Prefix where the pipeline reads input files (e.g., `"staging/"`).
    #[serde(default = "default_staging")]
    pub staging_prefix: String,

    /// Prefix for successfully processed files (e.g., `"historical/"`).
    #[serde(default = "default_historical")]
    pub historical_prefix: String,

    /// Prefix for failed files (e.g., `"error/"`).
    #[serde(default = "default_error")]
    pub error_prefix: String,

    /// Action on pipeline success: `"move_to_historical"` (default), `"delete"`, `"none"`.
    #[serde(default = "default_on_success")]
    pub on_success: LifecycleAction,

    /// Action on pipeline failure: `"move_to_input"` (default), `"move_to_error"`, `"none"`.
    #[serde(default = "default_on_failure")]
    pub on_failure: LifecycleAction,

    /// Credential reference for GCS auth.
    #[serde(default = "default_adc")]
    pub credentials: CredentialRef,
}

/// Actions the lifecycle manager can take on processed files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleAction {
    /// Move files to the historical prefix (with run ID subdirectory).
    MoveToHistorical,
    /// Move files back to the original input location.
    MoveToInput,
    /// Move files to the error prefix.
    MoveToError,
    /// Delete the files permanently.
    Delete,
    /// Do nothing.
    None,
}

fn default_staging() -> String {
    "staging/".to_string()
}

fn default_historical() -> String {
    "historical/".to_string()
}

fn default_error() -> String {
    "error/".to_string()
}

fn default_on_success() -> LifecycleAction {
    LifecycleAction::MoveToHistorical
}

fn default_on_failure() -> LifecycleAction {
    LifecycleAction::MoveToInput
}

fn default_adc() -> CredentialRef {
    CredentialRef::ApplicationDefault
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_spec_serde_roundtrip() {
        let spec = LifecycleSpec {
            bucket: "my-bucket".to_string(),
            staging_prefix: "staging/".to_string(),
            historical_prefix: "historical/".to_string(),
            error_prefix: "error/".to_string(),
            on_success: LifecycleAction::MoveToHistorical,
            on_failure: LifecycleAction::MoveToError,
            credentials: CredentialRef::ApplicationDefault,
        };

        let json = serde_json::to_string(&spec).unwrap();
        let back: LifecycleSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bucket, "my-bucket");
        assert_eq!(back.on_success, LifecycleAction::MoveToHistorical);
        assert_eq!(back.on_failure, LifecycleAction::MoveToError);
    }

    #[test]
    fn test_lifecycle_spec_defaults() {
        let json = r#"{"bucket": "b"}"#;
        let spec: LifecycleSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.staging_prefix, "staging/");
        assert_eq!(spec.historical_prefix, "historical/");
        assert_eq!(spec.error_prefix, "error/");
        assert_eq!(spec.on_success, LifecycleAction::MoveToHistorical);
        assert_eq!(spec.on_failure, LifecycleAction::MoveToInput);
        assert!(matches!(
            spec.credentials,
            CredentialRef::ApplicationDefault
        ));
    }

    #[test]
    fn test_lifecycle_spec_toml_parsing() {
        let toml_str = r#"
            bucket = "prod-bucket"
            staging_prefix = "input/"
            on_success = "delete"
            on_failure = "move_to_error"
        "#;
        let spec: LifecycleSpec = toml::from_str(toml_str).unwrap();
        assert_eq!(spec.bucket, "prod-bucket");
        assert_eq!(spec.staging_prefix, "input/");
        assert_eq!(spec.on_success, LifecycleAction::Delete);
        assert_eq!(spec.on_failure, LifecycleAction::MoveToError);
    }

    #[test]
    fn test_lifecycle_action_none() {
        let json = r#"{"bucket": "b", "on_success": "none", "on_failure": "none"}"#;
        let spec: LifecycleSpec = serde_json::from_str(json).unwrap();
        assert_eq!(spec.on_success, LifecycleAction::None);
        assert_eq!(spec.on_failure, LifecycleAction::None);
    }

    #[test]
    fn test_pipeline_spec_with_lifecycle() {
        use crate::PipelineSpec;

        let toml = r#"
            name = "lifecycle-test"
            version = 1
            output_dir = "/tmp/out"

            [sources.gcs-src]
            kind = "gcs"
            bucket = "my-bucket"
            prefix = "staging/"

            [stages.emit]
            adapter = "emit"
            resources = { reads = ["raw"] }

            [lifecycle]
            bucket = "my-bucket"
            staging_prefix = "staging/"
            historical_prefix = "archive/"
            on_success = "move_to_historical"
        "#;

        let spec = PipelineSpec::from_toml(toml).unwrap();
        assert!(spec.lifecycle.is_some());
        let lc = spec.lifecycle.unwrap();
        assert_eq!(lc.bucket, "my-bucket");
        assert_eq!(lc.historical_prefix, "archive/");
    }

    #[test]
    fn test_pipeline_spec_without_lifecycle_backward_compat() {
        use crate::PipelineSpec;

        let toml = r#"
            name = "no-lifecycle"
            version = 1
            output_dir = "/tmp/out"

            [sources.local]
            kind = "filesystem"
            root = "/tmp"

            [stages.emit]
            adapter = "emit"
            resources = { reads = ["raw"] }
        "#;

        let spec = PipelineSpec::from_toml(toml).unwrap();
        assert!(spec.lifecycle.is_none());
    }
}
