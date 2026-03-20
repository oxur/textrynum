//! Axum webhook HTTP server for receiving Zapier POST requests.
//!
//! Validates authentication (Basic Auth or Bearer token), parses the JSON
//! payload, resolves it to a typed schema, and sends the resulting
//! `ExtractedDocument` through a bounded channel for pipeline processing.

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use tokio::sync::{Notify, mpsc};
use tracing::{info, warn};

use ecl_pipeline_topo::ExtractedDocument;
use ecl_pipeline_topo::error::SourceError;

use crate::schemas::resolve_payload;

/// Shared state for the webhook handler.
#[derive(Clone)]
pub struct WebhookState {
    /// Channel sender for passing documents to the pipeline.
    pub sender: mpsc::Sender<ExtractedDocument>,
    /// Expected Basic Auth username.
    pub auth_username: String,
    /// Expected secret (password for Basic Auth or Bearer token).
    pub auth_secret: String,
    /// Source name this adapter is registered under.
    pub source_name: String,
    /// Default source hint if X-Zapier-Source header is absent.
    pub default_source_hint: String,
}

/// Run the webhook HTTP server.
///
/// Binds to `bind_addr`, serves `POST /webhook`, and runs until the
/// `shutdown` signal is received. Uses axum's graceful shutdown to
/// complete in-flight requests before exiting.
pub async fn run_server(
    bind_addr: &str,
    state: WebhookState,
    shutdown: Arc<Notify>,
) -> std::result::Result<(), SourceError> {
    let app = Router::new()
        .route("/webhook", post(webhook_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| SourceError::Permanent {
            source_name: "zapier".to_string(),
            message: format!("failed to bind {bind_addr}: {e}"),
        })?;

    let local_addr = listener.local_addr().map_err(|e| SourceError::Permanent {
        source_name: "zapier".to_string(),
        message: format!("failed to get local address: {e}"),
    })?;
    info!(addr = %local_addr, "zapier webhook server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
            info!("zapier webhook server shutting down");
        })
        .await
        .map_err(|e| SourceError::Permanent {
            source_name: "zapier".to_string(),
            message: format!("server error: {e}"),
        })?;

    Ok(())
}

/// Handle an incoming webhook POST request.
///
/// 1. Validate authentication (Basic Auth or Bearer token).
/// 2. Extract source hint from `X-Zapier-Source` header or config default.
/// 3. Parse the JSON body and resolve to a typed schema.
/// 4. Send the resulting `ExtractedDocument` through the channel.
/// 5. Return 200 OK (or 401/429/400 on errors).
async fn webhook_handler(
    State(state): State<WebhookState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // 1. Validate authentication.
    if !validate_auth(&headers, &state.auth_username, &state.auth_secret) {
        warn!(source = %state.source_name, "webhook auth failed");
        return StatusCode::UNAUTHORIZED;
    }

    // 2. Extract source hint.
    let source_hint = headers
        .get("x-zapier-source")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(&state.default_source_hint);

    // 3. Validate that body is valid JSON.
    if serde_json::from_slice::<serde_json::Value>(&body).is_err() {
        warn!(source = %state.source_name, "invalid JSON body");
        return StatusCode::BAD_REQUEST;
    }

    // 4. Generate unique ID and resolve payload.
    let id = format!(
        "zapier-{}-{}",
        source_hint,
        chrono::Utc::now().timestamp_millis()
    );
    let doc = resolve_payload(&body, source_hint, id);

    // 5. Send to channel (bounded — provides backpressure).
    match state.sender.try_send(doc) {
        Ok(()) => {
            info!(
                source = %state.source_name,
                hint = %source_hint,
                "webhook received and queued"
            );
            StatusCode::OK
        }
        Err(mpsc::error::TrySendError::Full(_)) => {
            warn!(source = %state.source_name, "channel full — backpressure");
            StatusCode::TOO_MANY_REQUESTS
        }
        Err(mpsc::error::TrySendError::Closed(_)) => {
            warn!(source = %state.source_name, "channel closed");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

/// Validate the Authorization header.
///
/// Supports both Basic Auth and Bearer token:
/// - `Authorization: Basic <base64(username:password)>` — compare username + secret
/// - `Authorization: Bearer <token>` — compare token against secret
fn validate_auth(headers: &HeaderMap, expected_username: &str, expected_secret: &str) -> bool {
    let Some(auth_value) = headers.get("authorization") else {
        return false;
    };
    let Ok(auth_str) = auth_value.to_str() else {
        return false;
    };

    if let Some(basic_b64) = auth_str.strip_prefix("Basic ") {
        // Basic Auth: decode base64, split on ':', compare.
        if let Ok(decoded) = BASE64.decode(basic_b64.trim())
            && let Ok(credentials) = std::str::from_utf8(&decoded)
            && let Some((username, password)) = credentials.split_once(':')
        {
            return username == expected_username && password == expected_secret;
        }
        false
    } else if let Some(token) = auth_str.strip_prefix("Bearer ") {
        // Bearer token: compare directly against secret.
        token.trim() == expected_secret
    } else {
        false
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn make_basic_auth(username: &str, password: &str) -> String {
        let encoded = BASE64.encode(format!("{username}:{password}"));
        format!("Basic {encoded}")
    }

    #[test]
    fn test_validate_auth_basic_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "pass")).unwrap(),
        );
        assert!(validate_auth(&headers, "user", "pass"));
    }

    #[test]
    fn test_validate_auth_basic_wrong_password() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "wrong")).unwrap(),
        );
        assert!(!validate_auth(&headers, "user", "pass"));
    }

    #[test]
    fn test_validate_auth_bearer_success() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer my-secret-token"),
        );
        assert!(validate_auth(&headers, "ignored", "my-secret-token"));
    }

    #[test]
    fn test_validate_auth_bearer_wrong_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer wrong-token"),
        );
        assert!(!validate_auth(&headers, "ignored", "my-secret-token"));
    }

    #[test]
    fn test_validate_auth_no_header() {
        let headers = HeaderMap::new();
        assert!(!validate_auth(&headers, "user", "pass"));
    }

    #[test]
    fn test_validate_auth_unknown_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Digest abc123"));
        assert!(!validate_auth(&headers, "user", "pass"));
    }

    #[tokio::test]
    async fn test_webhook_handler_valid_request() {
        let (tx, mut rx) = mpsc::channel(10);
        let state = WebhookState {
            sender: tx,
            auth_username: "user".to_string(),
            auth_secret: "pass".to_string(),
            source_name: "test".to_string(),
            default_source_hint: "unknown".to_string(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "pass")).unwrap(),
        );
        headers.insert("x-zapier-source", HeaderValue::from_static("granola"));

        let body = Bytes::from(
            r#"{"title":"Test","creator":{"name":"A","email":"a@b.com"},"summary":"S","transcript":"T","link":"L"}"#,
        );

        let status = webhook_handler(State(state), headers, body).await;
        assert_eq!(status, StatusCode::OK);

        let doc = rx.try_recv().unwrap();
        assert_eq!(doc.mime_type, "text/markdown");
    }

    #[tokio::test]
    async fn test_webhook_handler_auth_failure() {
        let (tx, _rx) = mpsc::channel(10);
        let state = WebhookState {
            sender: tx,
            auth_username: "user".to_string(),
            auth_secret: "pass".to_string(),
            source_name: "test".to_string(),
            default_source_hint: "unknown".to_string(),
        };

        let headers = HeaderMap::new(); // No auth header

        let body = Bytes::from(r#"{"test": true}"#);
        let status = webhook_handler(State(state), headers, body).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_webhook_handler_invalid_json() {
        let (tx, _rx) = mpsc::channel(10);
        let state = WebhookState {
            sender: tx,
            auth_username: "user".to_string(),
            auth_secret: "pass".to_string(),
            source_name: "test".to_string(),
            default_source_hint: "unknown".to_string(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "pass")).unwrap(),
        );

        let body = Bytes::from("not valid json");
        let status = webhook_handler(State(state), headers, body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_webhook_handler_channel_full() {
        let (tx, _rx) = mpsc::channel(1); // Capacity 1

        let state = WebhookState {
            sender: tx.clone(),
            auth_username: "user".to_string(),
            auth_secret: "pass".to_string(),
            source_name: "test".to_string(),
            default_source_hint: "unknown".to_string(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "pass")).unwrap(),
        );

        let body = Bytes::from(r#"{"test": true}"#);

        // Fill the channel
        let status = webhook_handler(State(state.clone()), headers.clone(), body.clone()).await;
        assert_eq!(status, StatusCode::OK);

        // Second request should hit backpressure
        let status = webhook_handler(State(state), headers, body).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_webhook_handler_uses_default_source_hint() {
        let (tx, mut rx) = mpsc::channel(10);
        let state = WebhookState {
            sender: tx,
            auth_username: "user".to_string(),
            auth_secret: "pass".to_string(),
            source_name: "test".to_string(),
            default_source_hint: "granola".to_string(),
        };

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&make_basic_auth("user", "pass")).unwrap(),
        );
        // No X-Zapier-Source header — should use default "granola"

        let body = Bytes::from(
            r#"{"title":"Test","creator":{"name":"A","email":"a@b.com"},"summary":"S","transcript":"T","link":"L"}"#,
        );

        let status = webhook_handler(State(state), headers, body).await;
        assert_eq!(status, StatusCode::OK);

        let doc = rx.try_recv().unwrap();
        // Should resolve as granola since default_source_hint = "granola"
        assert_eq!(doc.mime_type, "text/markdown");
    }
}
