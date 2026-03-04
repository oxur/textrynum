//! Reusable HTTP health endpoint for Fabryk MCP servers.
//!
//! Provides [`health_router`] which builds an axum `Router` with a `/health`
//! endpoint that reports per-service state as JSON. This eliminates the need
//! for each MCP server (taproot, ai-kasu, etc.) to reimplement the same pattern.
//!
//! # Response format
//!
//! ```json
//! {
//!   "status": "ok",
//!   "services": [
//!     {"name": "redis", "state": "ready"},
//!     {"name": "vector", "state": "starting"}
//!   ]
//! }
//! ```
//!
//! - **200** when all services are `Ready` or `Stopped` (not configured).
//! - **503** when any service is `Starting`, `Failed`, or `Degraded`.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::health_router;
//! use fabryk_core::service::ServiceHandle;
//!
//! let services = vec![
//!     ServiceHandle::new("redis"),
//!     ServiceHandle::new("vector"),
//! ];
//!
//! let router = axum::Router::new()
//!     .merge(health_router(services))
//!     .nest_service("/mcp", mcp_service);
//! ```

use axum::http::StatusCode;
use axum::response::IntoResponse;
use fabryk_core::service::{ServiceHandle, ServiceState};
use serde::Serialize;

/// JSON body returned by `GET /health`.
#[derive(Debug, Serialize)]
pub struct ServiceHealthResponse {
    /// `"ok"` when all services are ready, `"starting"` otherwise.
    pub status: &'static str,
    /// Per-service status entries.
    pub services: Vec<ServiceStatus>,
}

/// Per-service status entry in the health response.
#[derive(Debug, Serialize)]
pub struct ServiceStatus {
    /// Service name (e.g. `"redis"`, `"vector"`).
    pub name: String,
    /// Current state as a human-readable string.
    pub state: String,
}

/// Build an axum `Router` with a `/health` endpoint reporting per-service state.
///
/// Returns 200 when all services are `Ready` or `Stopped` (not configured),
/// 503 otherwise.
pub fn health_router(services: Vec<ServiceHandle>) -> axum::Router {
    axum::Router::new().route(
        "/health",
        axum::routing::get(move || {
            let svcs = services.clone();
            async move { health_handler(svcs) }
        }),
    )
}

fn health_handler(services: Vec<ServiceHandle>) -> impl IntoResponse {
    let statuses: Vec<ServiceStatus> = services
        .iter()
        .map(|h| ServiceStatus {
            name: h.name().to_string(),
            state: h.state().to_string(),
        })
        .collect();

    let all_ready = services.iter().all(|h| {
        let s = h.state();
        s.is_ready() || s == ServiceState::Stopped
    });

    let body = ServiceHealthResponse {
        status: if all_ready { "ok" } else { "starting" },
        services: statuses,
    };

    let json = serde_json::to_string(&body).unwrap_or_else(|_| r#"{"status":"error"}"#.to_string());

    let status_code = if all_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        json,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn make_handles(states: &[(&str, ServiceState)]) -> Vec<ServiceHandle> {
        states
            .iter()
            .map(|(name, state)| {
                let h = ServiceHandle::new(*name);
                h.set_state(state.clone());
                h
            })
            .collect()
    }

    #[tokio::test]
    async fn test_health_all_ready_returns_200() {
        let handles = make_handles(&[
            ("redis", ServiceState::Ready),
            ("knowledge", ServiceState::Ready),
        ]);
        let app = health_router(handles);
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["services"][0]["name"], "redis");
        assert_eq!(json["services"][0]["state"], "ready");
    }

    #[tokio::test]
    async fn test_health_starting_returns_503() {
        let handles = make_handles(&[
            ("redis", ServiceState::Starting),
            ("knowledge", ServiceState::Ready),
        ]);
        let app = health_router(handles);
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "starting");
    }

    #[tokio::test]
    async fn test_health_stopped_counts_as_ok() {
        let handles = make_handles(&[
            ("redis", ServiceState::Stopped),
            ("knowledge", ServiceState::Ready),
        ]);
        let app = health_router(handles);
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_failed_returns_503() {
        let handles = make_handles(&[
            ("redis", ServiceState::Failed("connection refused".into())),
            ("knowledge", ServiceState::Ready),
        ]);
        let app = health_router(handles);
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            json["services"][0]["state"]
                .as_str()
                .unwrap()
                .contains("failed")
        );
    }

    #[tokio::test]
    async fn test_health_empty_services_returns_200() {
        let app = health_router(vec![]);
        let resp = app
            .oneshot(Request::get("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
