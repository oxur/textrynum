//! Source specification types for external data services.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A source is "where does the data come from?"
/// The `kind` field determines which SourceAdapter handles it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum SourceSpec {
    /// Google Drive folder source.
    #[serde(rename = "google_drive")]
    GoogleDrive(GoogleDriveSourceSpec),

    /// Slack workspace source.
    #[serde(rename = "slack")]
    Slack(SlackSourceSpec),

    /// Local filesystem source.
    #[serde(rename = "filesystem")]
    Filesystem(FilesystemSourceSpec),
}

/// Google Drive source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleDriveSourceSpec {
    /// OAuth2 credentials reference.
    pub credentials: CredentialRef,

    /// Root folder ID(s) to scan.
    pub root_folders: Vec<String>,

    /// Include/exclude filter rules, evaluated in order.
    #[serde(default)]
    pub filters: Vec<FilterRule>,

    /// Which file types to process.
    #[serde(default)]
    pub file_types: Vec<FileTypeFilter>,

    /// Only process files modified after this timestamp.
    /// Supports "last_run" as a magic value for incrementality.
    pub modified_after: Option<String>,
}

/// Slack workspace source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSourceSpec {
    /// Bot token credentials reference.
    pub credentials: CredentialRef,

    /// Channel IDs to fetch messages from.
    pub channels: Vec<String>,

    /// How deep to follow threads (0 = top-level only).
    #[serde(default)]
    pub thread_depth: usize,

    /// Only process messages after this timestamp.
    pub modified_after: Option<String>,
}

/// Local filesystem source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemSourceSpec {
    /// Root directory to scan.
    pub root: PathBuf,

    /// Include/exclude filter rules.
    #[serde(default)]
    pub filters: Vec<FilterRule>,

    /// File extensions to include (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
}

/// How to resolve credentials for a source.
///
/// Uses internally-tagged representation (`"type": "file"`) rather than
/// `#[serde(untagged)]` because untagged enums are fragile with TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CredentialRef {
    /// Credentials from a file path.
    #[serde(rename = "file")]
    File {
        /// Path to credentials file.
        path: PathBuf,
    },
    /// Credentials from an environment variable.
    #[serde(rename = "env")]
    EnvVar {
        /// Environment variable name.
        env: String,
    },
    /// Use application default credentials.
    #[serde(rename = "application_default")]
    ApplicationDefault,
}

/// A filter rule for include/exclude patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// Glob pattern matched against the full path.
    pub pattern: String,
    /// Whether this rule includes or excludes matches.
    pub action: FilterAction,
}

/// Whether a filter rule includes or excludes matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FilterAction {
    /// Include items matching the pattern.
    Include,
    /// Exclude items matching the pattern.
    Exclude,
}

/// Filter by file type (extension or MIME type).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTypeFilter {
    /// File extension to match (e.g., "pdf").
    pub extension: Option<String>,
    /// MIME type to match.
    pub mime: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_source_spec_google_drive_serde_roundtrip() {
        let source = SourceSpec::GoogleDrive(GoogleDriveSourceSpec {
            credentials: CredentialRef::EnvVar {
                env: "GOOGLE_CREDS".to_string(),
            },
            root_folders: vec!["folder123".to_string()],
            filters: vec![FilterRule {
                pattern: "**/*.pdf".to_string(),
                action: FilterAction::Include,
            }],
            file_types: vec![FileTypeFilter {
                extension: Some("pdf".to_string()),
                mime: None,
            }],
            modified_after: Some("last_run".to_string()),
        });
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: SourceSpec = serde_json::from_str(&json).unwrap();
        // Verify round-trip by re-serializing and comparing JSON
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_source_spec_slack_serde_roundtrip() {
        let source = SourceSpec::Slack(SlackSourceSpec {
            credentials: CredentialRef::EnvVar {
                env: "SLACK_TOKEN".to_string(),
            },
            channels: vec!["C123".to_string(), "C456".to_string()],
            thread_depth: 3,
            modified_after: Some("2026-01-01T00:00:00Z".to_string()),
        });
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: SourceSpec = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_source_spec_filesystem_serde_roundtrip() {
        let source = SourceSpec::Filesystem(FilesystemSourceSpec {
            root: PathBuf::from("/tmp/data"),
            filters: vec![],
            extensions: vec!["md".to_string()],
        });
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: SourceSpec = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_credential_ref_file_serde() {
        let cred = CredentialRef::File {
            path: PathBuf::from("/etc/creds.json"),
        };
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains(r#""type":"file"#));
        let deserialized: CredentialRef = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_credential_ref_env_serde() {
        let cred = CredentialRef::EnvVar {
            env: "MY_SECRET".to_string(),
        };
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains(r#""type":"env"#));
        let deserialized: CredentialRef = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_credential_ref_application_default_serde() {
        let cred = CredentialRef::ApplicationDefault;
        let json = serde_json::to_string(&cred).unwrap();
        assert!(json.contains(r#""type":"application_default"#));
        let deserialized: CredentialRef = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_filter_rule_serde() {
        let include_rule = FilterRule {
            pattern: "**/*.md".to_string(),
            action: FilterAction::Include,
        };
        let json = serde_json::to_string(&include_rule).unwrap();
        let deserialized: FilterRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action, FilterAction::Include);

        let exclude_rule = FilterRule {
            pattern: "**/Archive/**".to_string(),
            action: FilterAction::Exclude,
        };
        let json = serde_json::to_string(&exclude_rule).unwrap();
        let deserialized: FilterRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.action, FilterAction::Exclude);
    }
}
