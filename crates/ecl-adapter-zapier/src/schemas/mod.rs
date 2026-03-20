//! Typed payload schemas for known Zapier sources.
//!
//! Each schema module defines the expected payload shape for a specific
//! Zapier trigger app and provides conversion to `ExtractedDocument`.
//! The `resolve_payload` function dispatches to the correct schema
//! based on the `source_hint` string.

pub mod gdrive;
pub mod gmail;
pub mod granola;
pub mod slack;

use std::collections::BTreeMap;

use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
use ecl_pipeline_topo::ExtractedDocument;

pub use gdrive::GDriveFileChange;
pub use gmail::GmailMessage;
pub use granola::GranolaMeetingNote;
pub use slack::SlackMessage;

/// Compute a blake3 content hash from raw bytes.
pub fn make_content_hash(raw_bytes: &[u8]) -> Blake3Hash {
    Blake3Hash::new(blake3::hash(raw_bytes).to_hex().to_string())
}

/// Resolve a webhook payload into an `ExtractedDocument` based on the source hint.
///
/// Tries to deserialize into the typed schema for the given hint. If the hint
/// is unrecognized or deserialization fails, falls back to raw JSON storage.
///
/// # Arguments
///
/// * `raw_bytes` — The raw JSON body bytes from the webhook POST.
/// * `source_hint` — The source type hint (from `X-Zapier-Source` header or config default).
/// * `id` — Unique identifier for this document.
pub fn resolve_payload(raw_bytes: &[u8], source_hint: &str, id: String) -> ExtractedDocument {
    match source_hint {
        "granola" => {
            if let Ok(note) = serde_json::from_slice::<GranolaMeetingNote>(raw_bytes) {
                return note.into_extracted_document(id, raw_bytes);
            }
        }
        "gmail" => {
            if let Ok(msg) = serde_json::from_slice::<GmailMessage>(raw_bytes) {
                return msg.into_extracted_document(id, raw_bytes);
            }
        }
        "slack" => {
            if let Ok(msg) = serde_json::from_slice::<SlackMessage>(raw_bytes) {
                return msg.into_extracted_document(id, raw_bytes);
            }
        }
        "gdrive" => {
            if let Ok(change) = serde_json::from_slice::<GDriveFileChange>(raw_bytes) {
                return change.into_extracted_document(id, raw_bytes);
            }
        }
        _ => {}
    }

    // Fallback: store raw JSON as-is.
    raw_json_fallback(raw_bytes, source_hint, id)
}

/// Fallback handler: stores the raw JSON payload as an `ExtractedDocument`.
fn raw_json_fallback(raw_bytes: &[u8], source_hint: &str, id: String) -> ExtractedDocument {
    let content_hash = make_content_hash(raw_bytes);

    let mut metadata = BTreeMap::new();
    metadata.insert(
        "source_hint".to_string(),
        serde_json::Value::String(source_hint.to_string()),
    );
    metadata.insert(
        "schema".to_string(),
        serde_json::Value::String("raw_json".to_string()),
    );

    let display_name = format!("zapier-{source_hint}-webhook");

    ExtractedDocument {
        id,
        display_name,
        content: raw_bytes.to_vec(),
        mime_type: "application/json".to_string(),
        provenance: ItemProvenance {
            source_kind: "zapier".to_string(),
            metadata,
            source_modified: None,
            extracted_at: chrono::Utc::now(),
        },
        content_hash,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_payload_granola() {
        let payload = serde_json::json!({
            "title": "Test Meeting",
            "creator": {"name": "Alice", "email": "a@b.com"},
            "summary": "Summary text.",
            "transcript": "Transcript text.",
            "link": "https://granola.so/note/123"
        });
        let raw = serde_json::to_vec(&payload).unwrap();
        let doc = resolve_payload(&raw, "granola", "id-1".to_string());

        assert_eq!(doc.mime_type, "text/markdown");
        assert_eq!(doc.display_name, "Test Meeting");
    }

    #[test]
    fn test_resolve_payload_gmail() {
        let payload = serde_json::json!({
            "from": "a@b.com",
            "to": "c@d.com",
            "subject": "Test Email",
            "body_plain": "Hello world",
            "date": "2026-03-17"
        });
        let raw = serde_json::to_vec(&payload).unwrap();
        let doc = resolve_payload(&raw, "gmail", "id-2".to_string());

        assert_eq!(doc.mime_type, "text/plain");
        assert!(doc.display_name.contains("Test Email"));
    }

    #[test]
    fn test_resolve_payload_slack() {
        let payload = serde_json::json!({
            "channel": "general",
            "user": "bob",
            "text": "Hello",
            "ts": "123.456"
        });
        let raw = serde_json::to_vec(&payload).unwrap();
        let doc = resolve_payload(&raw, "slack", "id-3".to_string());

        assert_eq!(doc.mime_type, "text/plain");
    }

    #[test]
    fn test_resolve_payload_gdrive() {
        let payload = serde_json::json!({
            "file_id": "abc",
            "file_name": "test.txt",
            "mime_type": "text/plain",
            "modified_time": "2026-03-17"
        });
        let raw = serde_json::to_vec(&payload).unwrap();
        let doc = resolve_payload(&raw, "gdrive", "id-4".to_string());

        assert_eq!(doc.mime_type, "application/json");
        assert_eq!(doc.display_name, "test.txt");
    }

    #[test]
    fn test_resolve_payload_unknown_hint_falls_back_to_raw() {
        let raw = b"{\"foo\": \"bar\"}";
        let doc = resolve_payload(raw, "unknown-service", "id-5".to_string());

        assert_eq!(doc.mime_type, "application/json");
        assert_eq!(doc.display_name, "zapier-unknown-service-webhook");
    }

    #[test]
    fn test_resolve_payload_bad_json_for_typed_falls_back() {
        let raw = b"{\"not_a_granola_field\": true}";
        let doc = resolve_payload(raw, "granola", "id-6".to_string());

        // Granola deserialization fails (missing required fields) → raw fallback
        assert_eq!(doc.mime_type, "application/json");
    }

    #[test]
    fn test_make_content_hash_deterministic() {
        let data = b"hello world";
        let h1 = make_content_hash(data);
        let h2 = make_content_hash(data);
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_make_content_hash_differs_for_different_data() {
        let h1 = make_content_hash(b"hello");
        let h2 = make_content_hash(b"world");
        assert_ne!(h1, h2);
    }
}
