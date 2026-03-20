//! Pipeline specification types for the ECL pipeline runner.
//!
//! This crate defines the TOML-driven configuration layer. All types are
//! immutable after parsing and derive `Serialize + Deserialize` for
//! embedding in checkpoints.

pub mod defaults;
pub mod error;
pub mod lifecycle;
pub mod source;
pub mod stage;
pub mod validation;

pub use defaults::{CheckpointStrategy, DefaultsSpec, RetrySpec};
pub use error::{Result, SpecError};
pub use lifecycle::LifecycleSpec;
pub use source::{
    CredentialRef, FileTypeFilter, FilesystemSourceSpec, FilterAction, FilterRule, GcsSourceSpec,
    GoogleDriveSourceSpec, SftpSourceSpec, SlackSourceSpec, SourceSpec,
};
pub use stage::{ResourceSpec, StageSpec};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Configuration for secret resolution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "provider")]
pub enum SecretsConfig {
    /// No secret management — only env vars and files.
    #[default]
    #[serde(rename = "none")]
    None,
    /// GCP Secret Manager.
    #[serde(rename = "gcp_secret_manager")]
    GcpSecretManager {
        /// GCP project ID.
        project: String,
    },
}

/// Pipeline trigger configuration for chaining pipelines.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TriggersSpec {
    /// Pipeline TOML config paths to trigger on success.
    #[serde(default)]
    pub on_success: Vec<String>,
    /// Pipeline TOML config paths to trigger on failure.
    #[serde(default)]
    pub on_failure: Vec<String>,
}

/// Schedule configuration for cron-based pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    /// Cron expression (5 or 7 fields). Example: "30 21 * * *"
    pub cron: String,
}

/// The root configuration, deserialized from TOML.
/// Immutable after load. This is the "what do you want to happen" layer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSpec {
    /// Human-readable name for this pipeline.
    pub name: String,

    /// Schema version for forward compatibility.
    pub version: u32,

    /// Where pipeline state and outputs are written.
    pub output_dir: PathBuf,

    /// Source definitions, keyed by user-chosen name.
    /// BTreeMap for deterministic serialization order.
    pub sources: BTreeMap<String, SourceSpec>,

    /// Stage definitions, keyed by user-chosen name.
    /// Ordering is declarative; the topology layer resolves execution order
    /// from resource declarations.
    pub stages: BTreeMap<String, StageSpec>,

    /// Global defaults that apply across all sources/stages.
    #[serde(default)]
    pub defaults: DefaultsSpec,

    /// Optional file lifecycle management for cloud storage sources.
    /// When set, the runner moves processed files (e.g., staging → historical)
    /// after pipeline completion or failure.
    #[serde(default)]
    pub lifecycle: Option<LifecycleSpec>,

    /// Secret management configuration.
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// Pipeline chaining: trigger other pipelines on completion.
    #[serde(default)]
    pub triggers: Option<TriggersSpec>,

    /// Optional schedule configuration for cron-based execution.
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
}

impl PipelineSpec {
    /// Parse a PipelineSpec from a TOML string.
    pub fn from_toml(toml_str: &str) -> Result<Self> {
        let spec: Self = toml::from_str(toml_str).map_err(|e| SpecError::ParseError {
            message: e.to_string(),
        })?;
        spec.validate()?;
        Ok(spec)
    }

    /// Validate the spec (delegates to validation module).
    pub fn validate(&self) -> Result<()> {
        validation::validate(self)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const FULL_EXAMPLE_TOML: &str = r#"
name = "q1-knowledge-sync"
version = 1
output_dir = "./output/q1-sync"

[defaults]
concurrency = 4
checkpoint = { every = "Batch" }

[defaults.retry]
max_attempts = 3
initial_backoff_ms = 1000
backoff_multiplier = 2.0
max_backoff_ms = 30000

[sources.engineering-drive]
kind = "google_drive"
credentials = { type = "env", env = "GOOGLE_CREDENTIALS" }
root_folders = ["1abc123def456"]
file_types = [
    { extension = "docx" },
    { extension = "pdf" },
    { mime = "application/vnd.google-apps.document" },
]
modified_after = "last_run"

  [[sources.engineering-drive.filters]]
  pattern = "**/Archive/**"
  action = "Exclude"

  [[sources.engineering-drive.filters]]
  pattern = "**"
  action = "Include"

[sources.team-slack]
kind = "slack"
credentials = { type = "env", env = "SLACK_BOT_TOKEN" }
channels = ["C01234ABCDE", "C05678FGHIJ"]
thread_depth = 3
modified_after = "2026-01-01T00:00:00Z"

[stages.fetch-gdrive]
adapter = "extract"
source = "engineering-drive"
resources = { reads = ["gdrive-api"], creates = ["raw-gdrive-docs"] }
retry = { max_attempts = 3, initial_backoff_ms = 1000, backoff_multiplier = 2.0, max_backoff_ms = 30000 }
timeout_secs = 300

[stages.fetch-slack]
adapter = "extract"
source = "team-slack"
resources = { reads = ["slack-api"], creates = ["raw-slack-messages"] }
retry = { max_attempts = 3, initial_backoff_ms = 500, backoff_multiplier = 2.0, max_backoff_ms = 10000 }

[stages.normalize-gdrive]
adapter = "normalize"
source = "engineering-drive"
resources = { reads = ["raw-gdrive-docs"], creates = ["normalized-docs"] }

[stages.normalize-slack]
adapter = "slack-normalize"
source = "team-slack"
resources = { reads = ["raw-slack-messages"], creates = ["normalized-messages"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["normalized-docs", "normalized-messages"] }

[stages.emit.params]
subdir = "normalized"
"#;

    const MINIMAL_TOML: &str = r#"
name = "test-pipeline"
version = 1
output_dir = "./output/test"

[sources.local]
kind = "filesystem"
root = "/tmp/test-data"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw-docs"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw-docs"] }
"#;

    #[test]
    fn test_pipeline_spec_from_example_toml() {
        let spec = PipelineSpec::from_toml(FULL_EXAMPLE_TOML).unwrap();
        assert_eq!(spec.name, "q1-knowledge-sync");
        assert_eq!(spec.version, 1);
        assert_eq!(spec.sources.len(), 2);
        assert_eq!(spec.stages.len(), 5);
        assert_eq!(spec.defaults.concurrency, 4);
        assert!(spec.sources.contains_key("engineering-drive"));
        assert!(spec.sources.contains_key("team-slack"));
    }

    #[test]
    fn test_pipeline_spec_roundtrip_toml_json() {
        let spec = PipelineSpec::from_toml(MINIMAL_TOML).unwrap();

        // Serialize to JSON
        let json = serde_json::to_string(&spec).unwrap();

        // Deserialize from JSON
        let from_json: PipelineSpec = serde_json::from_str(&json).unwrap();

        // Re-serialize and compare
        let json2 = serde_json::to_string(&from_json).unwrap();
        assert_eq!(json, json2);

        // Verify key fields survived
        assert_eq!(from_json.name, "test-pipeline");
        assert_eq!(from_json.sources.len(), 1);
        assert_eq!(from_json.stages.len(), 2);
    }

    #[test]
    fn test_pipeline_spec_from_toml_invalid() {
        let result = PipelineSpec::from_toml("this is not valid toml {{{{");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SpecError::ParseError { .. }));
    }

    #[test]
    fn test_secrets_config_serde_none() {
        let config = SecretsConfig::None;
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(r#""provider":"none"#));
        let deserialized: SecretsConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, SecretsConfig::None));
    }

    #[test]
    fn test_secrets_config_serde_gcp() {
        let config = SecretsConfig::GcpSecretManager {
            project: "my-project".to_string(),
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains(r#""provider":"gcp_secret_manager"#));
        assert!(json.contains(r#""project":"my-project"#));
        let deserialized: SecretsConfig = serde_json::from_str(&json).unwrap();
        assert!(matches!(
            deserialized,
            SecretsConfig::GcpSecretManager { .. }
        ));
    }

    #[test]
    fn test_triggers_spec_serde() {
        let triggers = TriggersSpec {
            on_success: vec!["pipeline-b.toml".to_string()],
            on_failure: vec!["alert-pipeline.toml".to_string()],
        };
        let json = serde_json::to_string(&triggers).unwrap();
        let deserialized: TriggersSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.on_success, vec!["pipeline-b.toml"]);
        assert_eq!(deserialized.on_failure, vec!["alert-pipeline.toml"]);
    }

    #[test]
    fn test_schedule_spec_serde() {
        let schedule = ScheduleSpec {
            cron: "30 21 * * *".to_string(),
        };
        let json = serde_json::to_string(&schedule).unwrap();
        let deserialized: ScheduleSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.cron, "30 21 * * *");
    }

    #[test]
    fn test_pipeline_spec_with_secrets_and_triggers() {
        let toml_str = r#"
name = "walgreens-327"
version = 1
output_dir = "./output/walgreens"

[secrets]
provider = "gcp_secret_manager"
project = "my-gcp-project"

[triggers]
on_success = ["walgreens-transformation.toml"]

[schedule]
cron = "30 21 * * *"

[sources.local]
kind = "filesystem"
root = "/tmp/test"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["raw"] }

[stages.emit]
adapter = "emit"
resources = { reads = ["raw"] }
"#;
        let spec = PipelineSpec::from_toml(toml_str).unwrap();
        assert!(matches!(
            spec.secrets,
            SecretsConfig::GcpSecretManager { .. }
        ));
        let triggers = spec.triggers.unwrap();
        assert_eq!(triggers.on_success, vec!["walgreens-transformation.toml"]);
        let schedule = spec.schedule.unwrap();
        assert_eq!(schedule.cron, "30 21 * * *");
    }
}
