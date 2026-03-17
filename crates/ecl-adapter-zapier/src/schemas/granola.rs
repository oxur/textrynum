//! Granola meeting note payload schema.
//!
//! Granola's Zapier integration sends meeting notes with transcripts,
//! summaries, attendee lists, and calendar event information.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ecl_pipeline_state::ItemProvenance;
use ecl_pipeline_topo::ExtractedDocument;

use crate::schemas::make_content_hash;

/// A Granola meeting note as sent by the Zapier trigger.
///
/// Two trigger types are available in Zapier:
/// - **Note Added to Folder** — automatic, triggers on folder add
/// - **Note Shared to Zapier** — manual, triggered from note sidebar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranolaMeetingNote {
    /// Meeting note title.
    pub title: String,

    /// Creator of the note.
    pub creator: GranolaPerson,

    /// Meeting attendees.
    #[serde(default)]
    pub attendees: Vec<GranolaPerson>,

    /// Associated calendar event (if any).
    pub calendar_event: Option<GranolaCalendarEvent>,

    /// Private notes typed during the meeting.
    pub my_notes: Option<String>,

    /// AI-enhanced summary (Markdown).
    pub summary: String,

    /// Full meeting transcript.
    pub transcript: String,

    /// Share link to note in Granola.
    pub link: String,
}

/// A person reference in a Granola payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranolaPerson {
    /// Person's display name.
    pub name: String,
    /// Person's email address.
    pub email: String,
}

/// Calendar event associated with a Granola meeting note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GranolaCalendarEvent {
    /// Calendar event title.
    pub title: String,
    /// Event date/time (ISO 8601 format).
    pub datetime: String,
}

impl GranolaMeetingNote {
    /// Convert a Granola meeting note into an `ExtractedDocument`.
    ///
    /// Content is the summary + transcript formatted as Markdown.
    /// Metadata includes attendees, calendar event, and link.
    pub fn into_extracted_document(
        self,
        id: String,
        raw_bytes: &[u8],
    ) -> ExtractedDocument {
        // Build markdown content: summary then transcript
        let content = format!(
            "# {}\n\n## Summary\n\n{}\n\n## Transcript\n\n{}\n",
            self.title, self.summary, self.transcript
        );
        let content_bytes = content.into_bytes();
        let content_hash = make_content_hash(raw_bytes);

        // Build metadata
        let mut metadata = BTreeMap::new();
        metadata.insert(
            "link".to_string(),
            serde_json::Value::String(self.link.clone()),
        );
        metadata.insert(
            "creator_name".to_string(),
            serde_json::Value::String(self.creator.name.clone()),
        );
        metadata.insert(
            "creator_email".to_string(),
            serde_json::Value::String(self.creator.email.clone()),
        );

        let attendee_names: Vec<serde_json::Value> = self
            .attendees
            .iter()
            .map(|a| serde_json::Value::String(format!("{} <{}>", a.name, a.email)))
            .collect();
        metadata.insert(
            "attendees".to_string(),
            serde_json::Value::Array(attendee_names),
        );

        if let Some(ref event) = self.calendar_event {
            metadata.insert(
                "calendar_title".to_string(),
                serde_json::Value::String(event.title.clone()),
            );
            metadata.insert(
                "calendar_datetime".to_string(),
                serde_json::Value::String(event.datetime.clone()),
            );
        }

        ExtractedDocument {
            id,
            display_name: self.title,
            content: content_bytes,
            mime_type: "text/markdown".to_string(),
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

    fn sample_note() -> GranolaMeetingNote {
        GranolaMeetingNote {
            title: "Sprint Planning".to_string(),
            creator: GranolaPerson {
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            attendees: vec![
                GranolaPerson {
                    name: "Bob".to_string(),
                    email: "bob@example.com".to_string(),
                },
                GranolaPerson {
                    name: "Charlie".to_string(),
                    email: "charlie@example.com".to_string(),
                },
            ],
            calendar_event: Some(GranolaCalendarEvent {
                title: "Sprint Planning Q1".to_string(),
                datetime: "2026-03-17T10:00:00Z".to_string(),
            }),
            my_notes: Some("Remember to discuss Zapier integration".to_string()),
            summary: "Team discussed Q1 priorities.".to_string(),
            transcript: "Alice: Let's start with the backlog...".to_string(),
            link: "https://granola.so/note/abc123".to_string(),
        }
    }

    #[test]
    fn test_granola_serde_roundtrip() {
        let note = sample_note();
        let json = serde_json::to_string(&note).unwrap();
        let deserialized: GranolaMeetingNote = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.title, "Sprint Planning");
        assert_eq!(deserialized.attendees.len(), 2);
    }

    #[test]
    fn test_granola_into_extracted_document() {
        let note = sample_note();
        let raw = serde_json::to_vec(&note).unwrap();
        let doc = note.into_extracted_document("test-id".to_string(), &raw);

        assert_eq!(doc.id, "test-id");
        assert_eq!(doc.display_name, "Sprint Planning");
        assert_eq!(doc.mime_type, "text/markdown");
        assert_eq!(doc.provenance.source_kind, "zapier");

        let content = String::from_utf8(doc.content).unwrap();
        assert!(content.contains("# Sprint Planning"));
        assert!(content.contains("## Summary"));
        assert!(content.contains("Team discussed Q1 priorities."));
        assert!(content.contains("## Transcript"));
    }

    #[test]
    fn test_granola_minimal_payload() {
        let json = r#"{
            "title": "Quick Chat",
            "creator": {"name": "Alice", "email": "a@b.com"},
            "summary": "Brief sync.",
            "transcript": "Hello.",
            "link": "https://granola.so/note/xyz"
        }"#;
        let note: GranolaMeetingNote = serde_json::from_str(json).unwrap();
        assert!(note.attendees.is_empty());
        assert!(note.calendar_event.is_none());
        assert!(note.my_notes.is_none());
    }
}
