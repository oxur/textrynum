//! Slack stub source adapter for the ECL pipeline runner.
//!
//! This is a **validation stub** — it implements `SourceAdapter` using fixture
//! data to prove that the trait abstractions hold for a non-filesystem,
//! non-Drive source. No real Slack API calls are made.
//!
//! The adapter reads messages from a fixture directory where each file
//! represents a Slack message (JSON or plain text). This allows full
//! pipeline testing without a Slack workspace.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

mod error;

pub use error::SlackAdapterError;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use chrono::Utc;

use ecl_pipeline_spec::SourceSpec;
use ecl_pipeline_spec::source::SlackSourceSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::error::{ResolveError, SourceError};
use ecl_pipeline_topo::{ExtractedDocument, SourceAdapter, SourceItem};

/// Slack stub source adapter.
///
/// Reads fixture files from a local directory, treating each file as a
/// "Slack message." This validates that `SourceAdapter` works for
/// message-oriented (non-file) sources without requiring API access.
#[derive(Debug)]
pub struct SlackAdapter {
    /// Source name from the pipeline config.
    source_name: String,
    /// Channel IDs from the spec.
    channels: Vec<String>,
    /// Fixture directory containing message files.
    /// Derived from `SLACK_FIXTURE_DIR` env var or defaults to
    /// a conventional path.
    fixture_dir: Option<PathBuf>,
    /// In-memory fixture messages (for programmatic testing).
    fixtures: Vec<FixtureMessage>,
}

/// An in-memory fixture message for testing.
#[derive(Debug, Clone)]
pub struct FixtureMessage {
    /// Unique message ID (e.g., channel + timestamp).
    pub id: String,
    /// Human-readable display name.
    pub display_name: String,
    /// Channel this message belongs to.
    pub channel: String,
    /// Message content (plain text or JSON).
    pub content: Vec<u8>,
    /// MIME type of the content.
    pub mime_type: String,
}

impl SlackAdapter {
    /// Create a `SlackAdapter` from a `SourceSpec`.
    ///
    /// # Errors
    ///
    /// Returns `ResolveError::UnknownAdapter` if the spec is not a Slack source.
    pub fn from_spec(source_name: &str, spec: &SourceSpec) -> Result<Self, ResolveError> {
        let slack_spec = match spec {
            SourceSpec::Slack(s) => s,
            _ => {
                return Err(ResolveError::UnknownAdapter {
                    stage: source_name.to_string(),
                    adapter: "slack".to_string(),
                });
            }
        };

        Ok(Self::from_slack_spec(source_name, slack_spec))
    }

    /// Create a `SlackAdapter` directly from a `SlackSourceSpec`.
    pub fn from_slack_spec(source_name: &str, spec: &SlackSourceSpec) -> Self {
        let fixture_dir = std::env::var("SLACK_FIXTURE_DIR").ok().map(PathBuf::from);

        Self {
            source_name: source_name.to_string(),
            channels: spec.channels.clone(),
            fixture_dir,
            fixtures: Vec::new(),
        }
    }

    /// Create a `SlackAdapter` with in-memory fixture messages.
    ///
    /// This is the preferred constructor for tests — no filesystem needed.
    pub fn with_fixtures(
        source_name: &str,
        channels: Vec<String>,
        fixtures: Vec<FixtureMessage>,
    ) -> Self {
        Self {
            source_name: source_name.to_string(),
            channels,
            fixture_dir: None,
            fixtures,
        }
    }

    /// Read fixture messages from the fixture directory.
    ///
    /// Each file in `fixture_dir/<channel>/` is treated as a message.
    /// The filename (minus extension) becomes the message timestamp ID.
    fn read_fixture_dir(&self, dir: &Path) -> Result<Vec<FixtureMessage>, SourceError> {
        let mut messages = Vec::new();

        for channel in &self.channels {
            let channel_dir = dir.join(channel);
            if !channel_dir.exists() {
                tracing::debug!(channel = %channel, "fixture channel directory not found, skipping");
                continue;
            }

            let entries = std::fs::read_dir(&channel_dir).map_err(|e| SourceError::Permanent {
                source_name: self.source_name.clone(),
                message: format!("failed to read fixture dir {}: {e}", channel_dir.display()),
            })?;

            for entry in entries {
                let entry = entry.map_err(|e| SourceError::Permanent {
                    source_name: self.source_name.clone(),
                    message: format!("failed to read directory entry: {e}"),
                })?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let file_name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content = std::fs::read(&path).map_err(|e| SourceError::Permanent {
                    source_name: self.source_name.clone(),
                    message: format!("failed to read fixture file {}: {e}", path.display()),
                })?;

                let mime_type = if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    "application/json"
                } else {
                    "text/plain"
                };

                let msg_id = format!("{channel}:{file_name}");
                messages.push(FixtureMessage {
                    id: msg_id,
                    display_name: format!("#{channel} — {file_name}"),
                    channel: channel.clone(),
                    content,
                    mime_type: mime_type.to_string(),
                });
            }
        }

        Ok(messages)
    }
}

#[async_trait]
impl SourceAdapter for SlackAdapter {
    fn source_kind(&self) -> &str {
        "slack"
    }

    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
        // Gather messages from either in-memory fixtures or fixture directory.
        let messages = if !self.fixtures.is_empty() {
            self.fixtures.clone()
        } else if let Some(ref dir) = self.fixture_dir {
            self.read_fixture_dir(dir)?
        } else {
            tracing::warn!(
                source = %self.source_name,
                "no fixtures configured and SLACK_FIXTURE_DIR not set; returning empty"
            );
            return Ok(Vec::new());
        };

        tracing::info!(
            source = %self.source_name,
            messages = messages.len(),
            channels = ?self.channels,
            "enumerated Slack messages"
        );

        let items = messages
            .iter()
            .map(|msg| SourceItem {
                id: msg.id.clone(),
                display_name: msg.display_name.clone(),
                mime_type: msg.mime_type.clone(),
                path: format!("slack/{}", msg.id),
                modified_at: None,
                source_hash: None,
            })
            .collect();

        Ok(items)
    }

    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
        // Find the message by ID in fixtures or fixture directory.
        let messages = if !self.fixtures.is_empty() {
            self.fixtures.clone()
        } else if let Some(ref dir) = self.fixture_dir {
            self.read_fixture_dir(dir)?
        } else {
            return Err(SourceError::Permanent {
                source_name: self.source_name.clone(),
                message: format!("no fixtures configured for item '{}'", item.id),
            });
        };

        let msg =
            messages
                .iter()
                .find(|m| m.id == item.id)
                .ok_or_else(|| SourceError::NotFound {
                    source_name: self.source_name.clone(),
                    item_id: item.id.clone(),
                })?;

        let content_hash = Blake3Hash::new(blake3::hash(&msg.content).to_hex().as_str());

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "channel".to_string(),
            serde_json::Value::String(msg.channel.clone()),
        );

        let provenance = ItemProvenance {
            source_kind: "slack".to_string(),
            metadata,
            source_modified: None,
            extracted_at: Utc::now(),
        };

        tracing::debug!(
            source = %self.source_name,
            item_id = %item.id,
            content_bytes = msg.content.len(),
            "fetched Slack message"
        );

        Ok(ExtractedDocument {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            content: msg.content.clone(),
            mime_type: msg.mime_type.clone(),
            provenance,
            content_hash,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::source::CredentialRef;

    fn make_slack_spec() -> SlackSourceSpec {
        SlackSourceSpec {
            credentials: CredentialRef::EnvVar {
                env: "SLACK_TOKEN".to_string(),
            },
            channels: vec!["C001".to_string(), "C002".to_string()],
            thread_depth: 0,
            modified_after: None,
            stream: None,
        }
    }

    fn make_fixtures() -> Vec<FixtureMessage> {
        vec![
            FixtureMessage {
                id: "C001:msg-001".to_string(),
                display_name: "#C001 — msg-001".to_string(),
                channel: "C001".to_string(),
                content: b"Hello from Slack!".to_vec(),
                mime_type: "text/plain".to_string(),
            },
            FixtureMessage {
                id: "C001:msg-002".to_string(),
                display_name: "#C001 — msg-002".to_string(),
                channel: "C001".to_string(),
                content: b"Second message".to_vec(),
                mime_type: "text/plain".to_string(),
            },
            FixtureMessage {
                id: "C002:msg-001".to_string(),
                display_name: "#C002 — msg-001".to_string(),
                channel: "C002".to_string(),
                content: b"{\"text\": \"JSON message\"}".to_vec(),
                mime_type: "application/json".to_string(),
            },
        ]
    }

    // ── Construction tests ──────────────────────────────────────────────

    #[test]
    fn test_from_spec_with_slack_source() {
        let spec = SourceSpec::Slack(make_slack_spec());
        let adapter = SlackAdapter::from_spec("test-slack", &spec).unwrap();
        assert_eq!(adapter.source_kind(), "slack");
        assert_eq!(adapter.source_name, "test-slack");
        assert_eq!(adapter.channels.len(), 2);
    }

    #[test]
    fn test_from_spec_wrong_kind_returns_error() {
        let spec = SourceSpec::Filesystem(ecl_pipeline_spec::source::FilesystemSourceSpec {
            root: "/tmp".into(),
            extensions: vec![],
            filters: vec![],
            stream: None,
        });
        let result = SlackAdapter::from_spec("test", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_with_fixtures_constructor() {
        let fixtures = make_fixtures();
        let adapter =
            SlackAdapter::with_fixtures("test", vec!["C001".to_string()], fixtures.clone());
        assert_eq!(adapter.fixtures.len(), 3);
        assert_eq!(adapter.channels.len(), 1);
    }

    #[test]
    fn test_source_kind_returns_slack() {
        let adapter = SlackAdapter::with_fixtures("s", vec![], vec![]);
        assert_eq!(adapter.source_kind(), "slack");
    }

    // ── Enumerate tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_with_fixtures_returns_all() {
        let fixtures = make_fixtures();
        let adapter = SlackAdapter::with_fixtures(
            "test",
            vec!["C001".to_string(), "C002".to_string()],
            fixtures,
        );

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 3);
    }

    #[tokio::test]
    async fn test_enumerate_empty_fixtures_returns_empty() {
        let adapter = SlackAdapter::with_fixtures("test", vec!["C001".to_string()], vec![]);

        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_enumerate_source_item_fields() {
        let fixtures = vec![FixtureMessage {
            id: "C001:ts-123".to_string(),
            display_name: "#C001 — ts-123".to_string(),
            channel: "C001".to_string(),
            content: b"test".to_vec(),
            mime_type: "text/plain".to_string(),
        }];
        let adapter = SlackAdapter::with_fixtures("s", vec!["C001".to_string()], fixtures);

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "C001:ts-123");
        assert_eq!(items[0].display_name, "#C001 — ts-123");
        assert_eq!(items[0].mime_type, "text/plain");
        assert_eq!(items[0].path, "slack/C001:ts-123");
        assert!(items[0].modified_at.is_none());
        assert!(items[0].source_hash.is_none());
    }

    #[tokio::test]
    async fn test_enumerate_no_fixtures_no_dir_returns_empty() {
        let adapter = SlackAdapter {
            source_name: "test".to_string(),
            channels: vec!["C001".to_string()],
            fixture_dir: None,
            fixtures: Vec::new(),
        };

        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }

    // ── Fetch tests ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_fetch_existing_message() {
        let fixtures = make_fixtures();
        let adapter = SlackAdapter::with_fixtures("test", vec!["C001".to_string()], fixtures);

        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();

        assert_eq!(doc.id, "C001:msg-001");
        assert_eq!(doc.content, b"Hello from Slack!");
        assert_eq!(doc.mime_type, "text/plain");
        assert_eq!(doc.provenance.source_kind, "slack");
        assert!(doc.provenance.metadata.contains_key("channel"));
        assert!(!doc.content_hash.as_str().is_empty());
    }

    #[tokio::test]
    async fn test_fetch_not_found() {
        let adapter =
            SlackAdapter::with_fixtures("test", vec!["C001".to_string()], make_fixtures());

        let fake_item = SourceItem {
            id: "C999:nonexistent".to_string(),
            display_name: "ghost".to_string(),
            mime_type: "text/plain".to_string(),
            path: "slack/C999:nonexistent".to_string(),
            modified_at: None,
            source_hash: None,
        };

        let result = adapter.fetch(&fake_item).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_json_message() {
        let fixtures = make_fixtures();
        let adapter = SlackAdapter::with_fixtures("test", vec!["C002".to_string()], fixtures);

        let items = adapter.enumerate().await.unwrap();
        let json_item = items.iter().find(|i| i.id == "C002:msg-001").unwrap();
        let doc = adapter.fetch(json_item).await.unwrap();

        assert_eq!(doc.mime_type, "application/json");
        assert!(doc.content.starts_with(b"{"));
    }

    #[tokio::test]
    async fn test_fetch_content_hash_is_blake3() {
        let content = b"Hello from Slack!";
        let expected_hash = blake3::hash(content).to_hex().to_string();

        let fixtures = vec![FixtureMessage {
            id: "C001:hash-test".to_string(),
            display_name: "hash test".to_string(),
            channel: "C001".to_string(),
            content: content.to_vec(),
            mime_type: "text/plain".to_string(),
        }];
        let adapter = SlackAdapter::with_fixtures("test", vec!["C001".to_string()], fixtures);

        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();
        assert_eq!(doc.content_hash.as_str(), expected_hash);
    }

    #[tokio::test]
    async fn test_fetch_provenance_has_channel() {
        let fixtures = vec![FixtureMessage {
            id: "C042:prov-test".to_string(),
            display_name: "provenance test".to_string(),
            channel: "C042".to_string(),
            content: b"test".to_vec(),
            mime_type: "text/plain".to_string(),
        }];
        let adapter = SlackAdapter::with_fixtures("test", vec!["C042".to_string()], fixtures);

        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();

        let channel = doc.provenance.metadata.get("channel").unwrap();
        assert_eq!(channel, &serde_json::Value::String("C042".to_string()));
    }

    // ── Fixture directory tests ─────────────────────────────────────────

    #[tokio::test]
    async fn test_enumerate_from_fixture_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let channel_dir = tmp.path().join("C001");
        std::fs::create_dir_all(&channel_dir).unwrap();
        std::fs::write(channel_dir.join("msg-001.txt"), "hello").unwrap();
        std::fs::write(channel_dir.join("msg-002.json"), r#"{"text":"hi"}"#).unwrap();

        let adapter = SlackAdapter {
            source_name: "fixture-test".to_string(),
            channels: vec!["C001".to_string()],
            fixture_dir: Some(tmp.path().to_path_buf()),
            fixtures: Vec::new(),
        };

        let items = adapter.enumerate().await.unwrap();
        assert_eq!(items.len(), 2);

        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert!(ids.contains(&"C001:msg-001"));
        assert!(ids.contains(&"C001:msg-002"));
    }

    #[tokio::test]
    async fn test_fetch_from_fixture_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let channel_dir = tmp.path().join("C001");
        std::fs::create_dir_all(&channel_dir).unwrap();
        std::fs::write(channel_dir.join("msg-001.txt"), "fixture content").unwrap();

        let adapter = SlackAdapter {
            source_name: "fixture-test".to_string(),
            channels: vec!["C001".to_string()],
            fixture_dir: Some(tmp.path().to_path_buf()),
            fixtures: Vec::new(),
        };

        let items = adapter.enumerate().await.unwrap();
        let doc = adapter.fetch(&items[0]).await.unwrap();
        assert_eq!(doc.content, b"fixture content");
        assert_eq!(doc.mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_enumerate_fixture_dir_missing_channel_skips() {
        let tmp = tempfile::tempdir().unwrap();
        // Don't create channel dir — should skip gracefully.

        let adapter = SlackAdapter {
            source_name: "test".to_string(),
            channels: vec!["C999".to_string()],
            fixture_dir: Some(tmp.path().to_path_buf()),
            fixtures: Vec::new(),
        };

        let items = adapter.enumerate().await.unwrap();
        assert!(items.is_empty());
    }
}
