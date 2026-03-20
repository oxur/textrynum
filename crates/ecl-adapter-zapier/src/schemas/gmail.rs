//! Gmail message payload schema.
//!
//! Gmail's Zapier trigger sends email metadata and body content
//! when new emails arrive matching configured filters.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ecl_pipeline_state::ItemProvenance;
use ecl_pipeline_topo::ExtractedDocument;

use crate::schemas::make_content_hash;

/// A Gmail message as sent by Zapier's "New Email" trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailMessage {
    /// Sender address.
    pub from: String,

    /// Recipient address.
    pub to: String,

    /// Email subject line.
    pub subject: String,

    /// Plain text body (if available).
    pub body_plain: Option<String>,

    /// HTML body (if available).
    pub body_html: Option<String>,

    /// Date the email was sent (ISO 8601 or RFC 2822).
    pub date: String,

    /// Gmail labels applied to the message.
    #[serde(default)]
    pub labels: Vec<String>,

    /// Gmail thread ID.
    pub thread_id: Option<String>,

    /// Gmail message ID.
    pub message_id: Option<String>,
}

impl GmailMessage {
    /// Convert a Gmail message into an `ExtractedDocument`.
    ///
    /// Content is the plain text body (preferred) or HTML body.
    /// Metadata includes sender, recipient, subject, labels, and thread info.
    pub fn into_extracted_document(self, id: String, raw_bytes: &[u8]) -> ExtractedDocument {
        let (content, mime_type) = if let Some(ref plain) = self.body_plain {
            (plain.as_bytes().to_vec(), "text/plain".to_string())
        } else if let Some(ref html) = self.body_html {
            (html.as_bytes().to_vec(), "text/html".to_string())
        } else {
            (Vec::new(), "text/plain".to_string())
        };

        let content_hash = make_content_hash(raw_bytes);

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "from".to_string(),
            serde_json::Value::String(self.from.clone()),
        );
        metadata.insert("to".to_string(), serde_json::Value::String(self.to.clone()));
        metadata.insert(
            "subject".to_string(),
            serde_json::Value::String(self.subject.clone()),
        );
        metadata.insert(
            "date".to_string(),
            serde_json::Value::String(self.date.clone()),
        );

        if !self.labels.is_empty() {
            let label_values: Vec<serde_json::Value> = self
                .labels
                .iter()
                .map(|l| serde_json::Value::String(l.clone()))
                .collect();
            metadata.insert("labels".to_string(), serde_json::Value::Array(label_values));
        }

        if let Some(ref tid) = self.thread_id {
            metadata.insert(
                "thread_id".to_string(),
                serde_json::Value::String(tid.clone()),
            );
        }

        if let Some(ref mid) = self.message_id {
            metadata.insert(
                "message_id".to_string(),
                serde_json::Value::String(mid.clone()),
            );
        }

        let display_name = format!("{} — {}", self.subject, self.from);

        ExtractedDocument {
            id,
            display_name,
            content,
            mime_type,
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

    fn sample_message() -> GmailMessage {
        GmailMessage {
            from: "sender@example.com".to_string(),
            to: "recipient@example.com".to_string(),
            subject: "Q1 Report".to_string(),
            body_plain: Some("Please review the attached report.".to_string()),
            body_html: Some("<p>Please review the attached report.</p>".to_string()),
            date: "2026-03-17T10:00:00Z".to_string(),
            labels: vec!["INBOX".to_string(), "IMPORTANT".to_string()],
            thread_id: Some("thread-abc123".to_string()),
            message_id: Some("msg-xyz789".to_string()),
        }
    }

    #[test]
    fn test_gmail_serde_roundtrip() {
        let msg = sample_message();
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: GmailMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.subject, "Q1 Report");
        assert_eq!(deserialized.labels.len(), 2);
    }

    #[test]
    fn test_gmail_into_extracted_document_prefers_plain() {
        let msg = sample_message();
        let raw = serde_json::to_vec(&msg).unwrap();
        let doc = msg.into_extracted_document("test-id".to_string(), &raw);

        assert_eq!(doc.mime_type, "text/plain");
        let content = String::from_utf8(doc.content).unwrap();
        assert!(content.contains("Please review"));
    }

    #[test]
    fn test_gmail_falls_back_to_html() {
        let mut msg = sample_message();
        msg.body_plain = None;
        let raw = serde_json::to_vec(&msg).unwrap();
        let doc = msg.into_extracted_document("test-id".to_string(), &raw);

        assert_eq!(doc.mime_type, "text/html");
    }

    #[test]
    fn test_gmail_minimal_payload() {
        let json = r#"{
            "from": "a@b.com",
            "to": "c@d.com",
            "subject": "Test",
            "date": "2026-03-17"
        }"#;
        let msg: GmailMessage = serde_json::from_str(json).unwrap();
        assert!(msg.body_plain.is_none());
        assert!(msg.labels.is_empty());
    }
}
