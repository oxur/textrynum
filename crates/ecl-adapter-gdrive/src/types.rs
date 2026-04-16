//! Google Drive API v3 response types.

use serde::{Deserialize, Serialize};

// Re-export shared Google auth types from ecl-gcp-auth.
pub use ecl_gcp_auth::{
    AuthorizedUserCredentials, GOOGLE_TOKEN_URL, ServiceAccountKey, TokenResponse,
};

/// Response from the Drive Files.list API endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileListResponse {
    /// List of files matching the query.
    #[serde(default)]
    pub files: Vec<DriveFile>,

    /// Token for the next page of results, if any.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

/// Individual file metadata from the Drive API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DriveFile {
    /// The file's unique Drive ID.
    pub id: String,

    /// The file's name.
    pub name: String,

    /// The file's MIME type.
    #[serde(rename = "mimeType")]
    pub mime_type: String,

    /// Last modified time (RFC 3339 timestamp).
    #[serde(rename = "modifiedTime")]
    pub modified_time: Option<String>,

    /// MD5 checksum (not available for Google Workspace documents).
    #[serde(rename = "md5Checksum")]
    pub md5_checksum: Option<String>,

    /// IDs of parent folders.
    #[serde(default)]
    pub parents: Vec<String>,

    /// File size in bytes (not available for Google Workspace documents).
    pub size: Option<String>,
}

impl DriveFile {
    /// Whether this file is a Google Drive folder.
    pub fn is_folder(&self) -> bool {
        self.mime_type == MIME_FOLDER
    }
}

// -- Google Drive MIME type constants ----------------------------------------

/// MIME type for Google Drive folders.
pub const MIME_FOLDER: &str = "application/vnd.google-apps.folder";

/// MIME type for Google Docs.
pub const MIME_DOCUMENT: &str = "application/vnd.google-apps.document";

/// MIME type for Google Sheets.
pub const MIME_SPREADSHEET: &str = "application/vnd.google-apps.spreadsheet";

/// MIME type for Google Slides.
pub const MIME_PRESENTATION: &str = "application/vnd.google-apps.presentation";

/// Default Google Drive API v3 base URL.
pub const DRIVE_API_BASE_URL: &str = "https://www.googleapis.com";

/// Scope for read-only access to Google Drive.
pub const DRIVE_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_file_list_response_deserialize() {
        let json = r#"{
            "files": [
                {
                    "id": "abc123",
                    "name": "doc.pdf",
                    "mimeType": "application/pdf",
                    "modifiedTime": "2026-03-01T10:00:00Z",
                    "md5Checksum": "d41d8cd98f00b204e9800998ecf8427e",
                    "parents": ["folder1"],
                    "size": "12345"
                }
            ],
            "nextPageToken": "token123"
        }"#;
        let resp: FileListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.files[0].id, "abc123");
        assert_eq!(resp.files[0].name, "doc.pdf");
        assert_eq!(resp.next_page_token.as_deref(), Some("token123"));
    }

    #[test]
    fn test_file_list_response_empty_files() {
        let json = r#"{ "files": [] }"#;
        let resp: FileListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.files.is_empty());
        assert!(resp.next_page_token.is_none());
    }

    #[test]
    fn test_file_list_response_missing_files_field() {
        let json = r#"{}"#;
        let resp: FileListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.files.is_empty());
    }

    #[test]
    fn test_drive_file_is_folder() {
        let folder = DriveFile {
            id: "f1".to_string(),
            name: "Docs".to_string(),
            mime_type: MIME_FOLDER.to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(folder.is_folder());

        let file = DriveFile {
            id: "f2".to_string(),
            name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: None,
            md5_checksum: None,
            parents: vec![],
            size: None,
        };
        assert!(!file.is_folder());
    }

    #[test]
    fn test_drive_file_google_workspace_types() {
        for mime in [MIME_DOCUMENT, MIME_SPREADSHEET, MIME_PRESENTATION] {
            let file = DriveFile {
                id: "f1".to_string(),
                name: "test".to_string(),
                mime_type: mime.to_string(),
                modified_time: None,
                md5_checksum: None,
                parents: vec![],
                size: None,
            };
            assert!(!file.is_folder());
        }
    }

    #[test]
    fn test_token_response_deserialize() {
        let json = r#"{
            "access_token": "ya29.test",
            "expires_in": 3600,
            "token_type": "Bearer"
        }"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "ya29.test");
        assert_eq!(resp.expires_in, Some(3600));
        assert_eq!(resp.token_type.as_deref(), Some("Bearer"));
    }

    #[test]
    fn test_service_account_key_deserialize() {
        let json = r#"{
            "client_email": "sa@project.iam.gserviceaccount.com",
            "private_key": "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----\n",
            "token_uri": "https://oauth2.googleapis.com/token"
        }"#;
        let key: ServiceAccountKey = serde_json::from_str(json).unwrap();
        assert_eq!(key.client_email, "sa@project.iam.gserviceaccount.com");
        assert!(key.private_key.contains("BEGIN RSA PRIVATE KEY"));
        assert_eq!(key.token_uri, "https://oauth2.googleapis.com/token");
    }

    #[test]
    fn test_authorized_user_credentials_deserialize() {
        let json = r#"{
            "client_id": "xxx.apps.googleusercontent.com",
            "client_secret": "secret",
            "refresh_token": "1//refresh",
            "type": "authorized_user"
        }"#;
        let cred: AuthorizedUserCredentials = serde_json::from_str(json).unwrap();
        assert_eq!(cred.credential_type, "authorized_user");
        assert_eq!(cred.refresh_token, "1//refresh");
    }

    #[test]
    fn test_drive_file_optional_fields() {
        let json = r#"{
            "id": "abc",
            "name": "test",
            "mimeType": "text/plain"
        }"#;
        let file: DriveFile = serde_json::from_str(json).unwrap();
        assert!(file.modified_time.is_none());
        assert!(file.md5_checksum.is_none());
        assert!(file.parents.is_empty());
        assert!(file.size.is_none());
    }

    #[test]
    fn test_drive_file_serde_roundtrip() {
        let file = DriveFile {
            id: "abc".to_string(),
            name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            modified_time: Some("2026-03-01T10:00:00Z".to_string()),
            md5_checksum: Some("checksum".to_string()),
            parents: vec!["parent1".to_string()],
            size: Some("1024".to_string()),
        };
        let json = serde_json::to_string(&file).unwrap();
        let roundtripped: DriveFile = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped.id, file.id);
        assert_eq!(roundtripped.name, file.name);
        assert_eq!(roundtripped.md5_checksum, file.md5_checksum);
    }
}
