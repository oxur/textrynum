//! Slack message payload schema.
//!
//! Slack's Zapier trigger sends channel messages with user info,
//! timestamps, and thread context.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ecl_pipeline_state::ItemProvenance;
use ecl_pipeline_topo::ExtractedDocument;

use crate::schemas::make_content_hash;

/// A Slack message as sent by Zapier's "New Message in Channel" trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    /// Channel name or ID.
    pub channel: String,

    /// User name or ID who sent the message.
    pub user: String,

    /// Message text content.
    pub text: String,

    /// Message timestamp (Slack's unique message identifier).
    pub ts: String,

    /// Thread parent timestamp (if this is a threaded reply).
    pub thread_ts: Option<String>,

    /// Slack team/workspace ID.
    pub team: Option<String>,
}

impl SlackMessage {
    /// Convert a Slack message into an `ExtractedDocument`.
    ///
    /// Content is the message text.
    /// Metadata includes channel, user, timestamps, and team info.
    pub fn into_extracted_document(
        self,
        id: String,
        raw_bytes: &[u8],
    ) -> ExtractedDocument {
        let content = self.text.as_bytes().to_vec();
        let content_hash = make_content_hash(raw_bytes);

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "channel".to_string(),
            serde_json::Value::String(self.channel.clone()),
        );
        metadata.insert(
            "user".to_string(),
            serde_json::Value::String(self.user.clone()),
        );
        metadata.insert(
            "ts".to_string(),
            serde_json::Value::String(self.ts.clone()),
        );

        if let Some(ref tts) = self.thread_ts {
            metadata.insert(
                "thread_ts".to_string(),
                serde_json::Value::String(tts.clone()),
            );
        }

        if let Some(ref team) = self.team {
            metadata.insert(
                "team".to_string(),
                serde_json::Value::String(team.clone()),
            );
        }

        let display_name = format!("#{} — {}", self.channel, self.user);

        ExtractedDocument {
            id,
            display_name,
            content,
            mime_type: "text/plain".to_string(),
            provenance: ItemProvenance {
                source_kind: "zapier".to_string(),
                metadata,
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            content_hash,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample_message() -> SlackMessage {
        SlackMessage {
            channel: "engineering".to_string(),
            user: "alice".to_string(),
            text: "Zapier integration is working!".to_string(),
            ts: "1710672000.000100".to_string(),
            thread_ts: Some("1710671900.000050".to_string()),
            team: Some("T01234ABC".to_string()),
        }
    }

    #[test]
    fn test_slack_serde_roundtrip() {
        let msg = sample_message();
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: SlackMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.channel, "engineering");
        assert_eq!(deserialized.text, "Zapier integration is working!");
    }

    #[test]
    fn test_slack_into_extracted_document() {
        let msg = sample_message();
        let raw = serde_json::to_vec(&msg).unwrap();
        let doc = msg.into_extracted_document("test-id".to_string(), &raw);

        assert_eq!(doc.mime_type, "text/plain");
        let content = String::from_utf8(doc.content).unwrap();
        assert!(content.contains("Zapier integration"));
        assert!(doc.display_name.contains("engineering"));
    }

    #[test]
    fn test_slack_minimal_payload() {
        let json = r#"{
            "channel": "general",
            "user": "bob",
            "text": "Hello",
            "ts": "123.456"
        }"#;
        let msg: SlackMessage = serde_json::from_str(json).unwrap();
        assert!(msg.thread_ts.is_none());
        assert!(msg.team.is_none());
    }
}
