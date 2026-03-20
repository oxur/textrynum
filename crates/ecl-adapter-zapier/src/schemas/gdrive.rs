//! Google Drive file change payload schema.
//!
//! Google Drive's Zapier trigger sends file metadata when files
//! are created or updated in a monitored folder.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use ecl_pipeline_state::ItemProvenance;
use ecl_pipeline_topo::ExtractedDocument;

use crate::schemas::make_content_hash;

/// A Google Drive file change as sent by Zapier's "New or Updated File" trigger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GDriveFileChange {
    /// Google Drive file ID.
    pub file_id: String,

    /// File name.
    pub file_name: String,

    /// MIME type of the file.
    pub mime_type: String,

    /// Last modified time (ISO 8601).
    pub modified_time: String,

    /// Web view link for the file (if available).
    pub web_view_link: Option<String>,
}

impl GDriveFileChange {
    /// Convert a Google Drive file change into an `ExtractedDocument`.
    ///
    /// Content is the raw JSON payload (file metadata only — the actual file
    /// content would need a separate fetch via Google Drive API).
    /// Metadata includes file ID, MIME type, and web link.
    pub fn into_extracted_document(self, id: String, raw_bytes: &[u8]) -> ExtractedDocument {
        let content_hash = make_content_hash(raw_bytes);

        let mut metadata = BTreeMap::new();
        metadata.insert(
            "file_id".to_string(),
            serde_json::Value::String(self.file_id.clone()),
        );
        metadata.insert(
            "file_mime_type".to_string(),
            serde_json::Value::String(self.mime_type.clone()),
        );
        metadata.insert(
            "modified_time".to_string(),
            serde_json::Value::String(self.modified_time.clone()),
        );

        if let Some(ref link) = self.web_view_link {
            metadata.insert(
                "web_view_link".to_string(),
                serde_json::Value::String(link.clone()),
            );
        }

        ExtractedDocument {
            id,
            display_name: self.file_name,
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sample_change() -> GDriveFileChange {
        GDriveFileChange {
            file_id: "1abc123def456".to_string(),
            file_name: "Q1 Report.docx".to_string(),
            mime_type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
                .to_string(),
            modified_time: "2026-03-17T10:00:00Z".to_string(),
            web_view_link: Some("https://docs.google.com/document/d/1abc123".to_string()),
        }
    }

    #[test]
    fn test_gdrive_serde_roundtrip() {
        let change = sample_change();
        let json = serde_json::to_string(&change).unwrap();
        let deserialized: GDriveFileChange = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_id, "1abc123def456");
        assert_eq!(deserialized.file_name, "Q1 Report.docx");
    }

    #[test]
    fn test_gdrive_into_extracted_document() {
        let change = sample_change();
        let raw = serde_json::to_vec(&change).unwrap();
        let doc = change.into_extracted_document("test-id".to_string(), &raw);

        assert_eq!(doc.display_name, "Q1 Report.docx");
        assert_eq!(doc.mime_type, "application/json");
        assert!(!doc.content.is_empty());
    }

    #[test]
    fn test_gdrive_minimal_payload() {
        let json = r#"{
            "file_id": "abc",
            "file_name": "test.txt",
            "mime_type": "text/plain",
            "modified_time": "2026-03-17"
        }"#;
        let change: GDriveFileChange = serde_json::from_str(json).unwrap();
        assert!(change.web_view_link.is_none());
    }
}
