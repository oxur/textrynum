//! Generic Tower authentication middleware.
//!
//! `AuthLayer` and `AuthService` wrap any inner service with token validation.
//! Generic over `TokenValidator` — plug in any identity provider.

use std::convert::Infallible;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use axum::body::Body;
use axum::response::IntoResponse;
use http::{Request, StatusCode};
use tower::{Layer, Service};

use crate::{AuthConfig, TokenValidator};

/// Tower `Layer` that wraps services with token authentication.
#[derive(Clone)]
pub struct AuthLayer<V: TokenValidator> {
    validator: Arc<V>,
    config: AuthConfig,
}

impl<V: TokenValidator> AuthLayer<V> {
    /// Create a new auth layer with the given validator and config.
    pub fn new(validator: Arc<V>, config: AuthConfig) -> Self {
        Self { validator, config }
    }
}

impl<V: TokenValidator, S> Layer<S> for AuthLayer<V> {
    type Service = AuthService<V, S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            validator: self.validator.clone(),
            config: self.config.clone(),
        }
    }
}

/// Tower `Service` that validates tokens before forwarding requests.
///
/// On successful validation, inserts `AuthenticatedUser` into request
/// extensions where it's available to downstream handlers.
#[derive(Clone)]
pub struct AuthService<V: TokenValidator, S> {
    inner: S,
    validator: Arc<V>,
    config: AuthConfig,
}

impl<V, S> Service<Request<Body>> for AuthService<V, S>
where
    V: TokenValidator,
    S: Service<Request<Body>, Error = Infallible> + Clone + Send + 'static,
    S::Response: IntoResponse,
    S::Future: Send,
{
    type Response = axum::response::Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        let validator = self.validator.clone();
        let config = self.config.clone();

        Box::pin(async move {
            // Dev mode — no auth required
            if !config.enabled {
                let resp = inner
                    .call(req)
                    .await
                    .unwrap_or_else(|infallible| match infallible {});
                return Ok(resp.into_response());
            }

            // Extract bearer token
            let token = match extract_bearer_token(&req) {
                Some(t) => t.to_string(),
                None => return Ok(unauthorized_response("missing or invalid bearer token")),
            };

            // Validate the token
            match validator.validate(&token, &config).await {
                Ok(user) => {
                    req.extensions_mut().insert(user);
                    let resp = inner
                        .call(req)
                        .await
                        .unwrap_or_else(|infallible| match infallible {});
                    Ok(resp.into_response())
                }
                Err(auth_err) => {
                    log::warn!("Authentication failed: {auth_err}");
                    Ok(unauthorized_response(&auth_err.to_string()))
                }
            }
        })
    }
}

/// Extract bearer token from the Authorization header.
fn extract_bearer_token(req: &Request<Body>) -> Option<&str> {
    req.headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

/// Build a 401 Unauthorized response with WWW-Authenticate header.
fn unauthorized_response(message: &str) -> axum::response::Response {
    let body = serde_json::json!({
        "error": {
            "category": "authentication",
            "message": message,
        }
    });

    let resource_url = std::env::var("KASU_RESOURCE_URL")
        .or_else(|_| std::env::var("TAPROOT_RESOURCE_URL"))
        .unwrap_or_default();
    let www_auth = format!(
        r#"Bearer resource_metadata="{resource_url}/.well-known/oauth-protected-resource""#,
    );

    let mut response = (
        StatusCode::UNAUTHORIZED,
        [(http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_string(&body).unwrap_or_default(),
    )
        .into_response();

    if let Ok(value) = http::HeaderValue::from_str(&www_auth) {
        response
            .headers_mut()
            .insert(http::header::WWW_AUTHENTICATE, value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthError, AuthenticatedUser};
    use std::sync::Mutex;
    use tower::ServiceExt;

    // A simple test validator that accepts "valid-token" and rejects everything else.
    struct TestValidator;

    impl TokenValidator for TestValidator {
        fn validate(
            &self,
            token: &str,
            _config: &AuthConfig,
        ) -> Pin<Box<dyn Future<Output = Result<AuthenticatedUser, AuthError>> + Send + '_>>
        {
            let token = token.to_string();
            Box::pin(async move {
                if token == "valid-token" {
                    Ok(AuthenticatedUser {
                        email: "alice@banyan.com".to_string(),
                        subject: "sub_123".to_string(),
                    })
                } else {
                    Err(AuthError::InvalidSignature("bad token".to_string()))
                }
            })
        }
    }

    fn test_config_enabled() -> AuthConfig {
        AuthConfig {
            enabled: true,
            audience: "test-audience".to_string(),
            domain: "banyan.com".to_string(),
        }
    }

    fn test_config_disabled() -> AuthConfig {
        AuthConfig {
            enabled: false,
            ..Default::default()
        }
    }

    /// Mock inner service that captures the AuthenticatedUser.
    #[derive(Clone)]
    struct MockService {
        captured_user: Arc<Mutex<Option<AuthenticatedUser>>>,
    }

    impl MockService {
        fn new() -> Self {
            Self {
                captured_user: Arc::new(Mutex::new(None)),
            }
        }
    }

    impl Service<Request<Body>> for MockService {
        type Response = axum::response::Response;
        type Error = Infallible;
        type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Request<Body>) -> Self::Future {
            let captured = self.captured_user.clone();
            Box::pin(async move {
                let user = req.extensions().get::<AuthenticatedUser>().cloned();
                *captured.lock().unwrap() = user;
                Ok((StatusCode::OK, "ok").into_response())
            })
        }
    }

    #[test]
    fn test_extract_bearer_token_valid() {
        let req = Request::builder()
            .header("Authorization", "Bearer my-token-123")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_bearer_token(&req), Some("my-token-123"));
    }

    #[test]
    fn test_extract_bearer_token_missing() {
        let req = Request::builder().body(Body::empty()).unwrap();
        assert_eq!(extract_bearer_token(&req), None);
    }

    #[test]
    fn test_extract_bearer_token_wrong_scheme() {
        let req = Request::builder()
            .header("Authorization", "Basic dXNlcjpwYXNz")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_bearer_token(&req), None);
    }

    #[test]
    fn test_unauthorized_response_status() {
        let resp = unauthorized_response("test error");
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_disabled_passes_through() {
        let mock = MockService::new();
        let layer = AuthLayer::new(Arc::new(TestValidator), test_config_disabled());
        let service = layer.layer(mock);

        let req = Request::builder().body(Body::empty()).unwrap();
        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_missing_token_returns_401() {
        let mock = MockService::new();
        let layer = AuthLayer::new(Arc::new(TestValidator), test_config_enabled());
        let service = layer.layer(mock);

        let req = Request::builder().body(Body::empty()).unwrap();
        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_invalid_token_returns_401() {
        let mock = MockService::new();
        let layer = AuthLayer::new(Arc::new(TestValidator), test_config_enabled());
        let service = layer.layer(mock);

        let req = Request::builder()
            .header("Authorization", "Bearer bad-token")
            .body(Body::empty())
            .unwrap();
        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_middleware_valid_token_passes_and_injects_user() {
        let mock = MockService::new();
        let captured = mock.captured_user.clone();
        let layer = AuthLayer::new(Arc::new(TestValidator), test_config_enabled());
        let service = layer.layer(mock);

        let req = Request::builder()
            .header("Authorization", "Bearer valid-token")
            .body(Body::empty())
            .unwrap();
        let resp = service.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let user = captured.lock().unwrap();
        let user = user.as_ref().expect("AuthenticatedUser should be present");
        assert_eq!(user.email, "alice@banyan.com");
        assert_eq!(user.subject, "sub_123");
    }
}
