//! MCP OAuth2 discovery metadata endpoints.
//!
//! Serves two discovery documents required by the MCP auth spec:
//!
//! - **Protected Resource Metadata** (RFC 9728) at
//!   `/.well-known/oauth-protected-resource`
//!
//! - **Authorization Server Metadata** (RFC 8414) at
//!   `/.well-known/oauth-authorization-server`
//!
//! Both are configurable: pass the resource URL and authorization server URL
//! when creating the routes.

use axum::Json;
use serde_json::{json, Value};

/// Create an axum `Router` with the MCP discovery routes.
///
/// Mounts:
/// - `/.well-known/oauth-protected-resource` (+ `/mcp` suffix)
/// - `/.well-known/oauth-authorization-server` (+ `/mcp` suffix)
///
/// # Arguments
///
/// - `resource_url`: The URL of this resource (e.g., `https://kasu.example.com`)
/// - `auth_server_url`: The authorization server URL (e.g., `https://accounts.google.com`)
pub fn discovery_routes(resource_url: &str, auth_server_url: &str) -> axum::Router {
    let resource_url = resource_url.to_string();
    let auth_server_url = auth_server_url.to_string();

    let resource_url_1 = resource_url.clone();
    let resource_url_2 = resource_url.clone();
    let auth_server_url_1 = auth_server_url.clone();
    let auth_server_url_2 = auth_server_url.clone();

    axum::Router::new()
        .route(
            "/.well-known/oauth-protected-resource",
            axum::routing::get(move || {
                protected_resource_metadata(resource_url_1.clone(), auth_server_url_1.clone())
            }),
        )
        .route(
            "/.well-known/oauth-protected-resource/mcp",
            axum::routing::get(move || {
                protected_resource_metadata(resource_url_2.clone(), auth_server_url_2.clone())
            }),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            axum::routing::get(move || authorization_server_metadata(auth_server_url.clone())),
        )
        .route(
            "/.well-known/oauth-authorization-server/mcp",
            axum::routing::get(move || {
                let _url = resource_url.clone();
                async move { authorization_server_metadata_google().await }
            }),
        )
}

/// Returns OAuth2 Protected Resource Metadata (RFC 9728).
async fn protected_resource_metadata(resource_url: String, auth_server_url: String) -> Json<Value> {
    Json(json!({
        "resource": resource_url,
        "authorization_servers": [auth_server_url],
        "scopes_supported": ["openid", "email", "profile"],
        "bearer_methods_supported": ["header"]
    }))
}

/// Returns OAuth2 Authorization Server Metadata (RFC 8414) for Google.
async fn authorization_server_metadata(issuer: String) -> Json<Value> {
    Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{}/o/oauth2/v2/auth", issuer.trim_end_matches('/')),
        "token_endpoint": "https://oauth2.googleapis.com/token",
        "jwks_uri": "https://www.googleapis.com/oauth2/v3/certs",
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": ["openid", "email", "profile"],
        "code_challenge_methods_supported": ["S256"]
    }))
}

/// Google-specific authorization server metadata (used for /mcp path).
async fn authorization_server_metadata_google() -> Json<Value> {
    authorization_server_metadata("https://accounts.google.com".to_string()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protected_resource_metadata() {
        let Json(value) = protected_resource_metadata(
            "https://kasu.example.com".to_string(),
            "https://accounts.google.com".to_string(),
        )
        .await;
        assert!(value.is_object());
        assert_eq!(value["resource"], "https://kasu.example.com");
        assert_eq!(
            value["authorization_servers"][0],
            "https://accounts.google.com"
        );
        assert!(value["scopes_supported"].is_array());
        assert_eq!(value["bearer_methods_supported"][0], "header");
    }

    #[tokio::test]
    async fn test_authorization_server_metadata() {
        let Json(value) =
            authorization_server_metadata("https://accounts.google.com".to_string()).await;
        assert!(value.is_object());
        assert_eq!(value["issuer"], "https://accounts.google.com");
        assert!(value["authorization_endpoint"]
            .as_str()
            .unwrap()
            .contains("oauth2"));
        assert!(value["token_endpoint"].is_string());
        assert!(value["jwks_uri"].is_string());
        assert!(value.get("registration_endpoint").is_none());
    }

    #[tokio::test]
    async fn test_authorization_server_metadata_google() {
        let Json(value) = authorization_server_metadata_google().await;
        assert_eq!(value["issuer"], "https://accounts.google.com");
    }
}
