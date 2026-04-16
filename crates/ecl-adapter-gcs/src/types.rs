//! GCS JSON API response types.

use serde::{Deserialize, Serialize};

// Re-export shared Google auth types from ecl-gcp-auth.
pub use ecl_gcp_auth::{
    AuthorizedUserCredentials, GOOGLE_TOKEN_URL, ServiceAccountKey, TokenResponse,
};

/// GCS JSON API base URL.
pub const GCS_API_BASE_URL: &str = "https://storage.googleapis.com/storage/v1";

/// GCS JSON API download URL (for object content).
pub const GCS_DOWNLOAD_BASE_URL: &str = "https://storage.googleapis.com/storage/v1";

/// Google OAuth2 scope for read-only GCS access.
pub const GCS_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/devstorage.read_only";

/// Google OAuth2 scope for read-write GCS access.
pub const GCS_READWRITE_SCOPE: &str = "https://www.googleapis.com/auth/devstorage.read_write";

/// GCS JSON API upload URL base (for object uploads).
pub const GCS_UPLOAD_BASE_URL: &str = "https://storage.googleapis.com/upload/storage/v1";

/// Response from the GCS Objects.list API endpoint.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ObjectListResponse {
    /// List of objects matching the query.
    #[serde(default)]
    pub items: Vec<GcsObject>,

    /// Token for the next page of results, if any.
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

/// Individual object metadata from the GCS API.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GcsObject {
    /// The object's name (full path within the bucket).
    pub name: String,

    /// The bucket this object belongs to.
    pub bucket: String,

    /// Object size in bytes.
    #[serde(default)]
    pub size: Option<String>,

    /// Content type (MIME type).
    #[serde(rename = "contentType")]
    pub content_type: Option<String>,

    /// Last modified time (RFC 3339 timestamp).
    pub updated: Option<String>,

    /// MD5 hash of object content (base64-encoded).
    #[serde(rename = "md5Hash")]
    pub md5_hash: Option<String>,

    /// Object generation (version identifier).
    pub generation: Option<String>,

    /// Object metageneration.
    pub metageneration: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_object_list_response_deserialize() {
        let json = r#"{
            "items": [
                {
                    "name": "staging/file.csv",
                    "bucket": "my-bucket",
                    "size": "1024",
                    "contentType": "text/csv",
                    "updated": "2026-03-15T10:00:00Z",
                    "md5Hash": "abc123=="
                }
            ],
            "nextPageToken": "token123"
        }"#;
        let resp: ObjectListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].name, "staging/file.csv");
        assert_eq!(resp.next_page_token, Some("token123".to_string()));
    }

    #[test]
    fn test_object_list_response_empty() {
        let json = r#"{}"#;
        let resp: ObjectListResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
        assert!(resp.next_page_token.is_none());
    }

    #[test]
    fn test_gcs_object_deserialize_minimal() {
        let json = r#"{"name": "test.csv", "bucket": "b"}"#;
        let obj: GcsObject = serde_json::from_str(json).unwrap();
        assert_eq!(obj.name, "test.csv");
        assert!(obj.content_type.is_none());
        assert!(obj.updated.is_none());
    }

    #[test]
    fn test_token_response_deserialize() {
        let json = r#"{"access_token": "ya29.test", "expires_in": 3600}"#;
        let resp: TokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.access_token, "ya29.test");
        assert_eq!(resp.expires_in, Some(3600));
    }
}
