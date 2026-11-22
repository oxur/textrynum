use std::convert::Infallible;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

use axum::{body::{Body, to_bytes}, response::Response};
use fabryk_auth::{AuthConfig, AuthError, AuthLayer, AuthenticatedUser, TokenValidator};
use fabryk_core::{BackendSlot, ServiceState, service::{RetryConfig, ServiceHandle, spawn_with_retry}};
use fabryk_mcp_content::{ContentItemProvider, ContentTools, FsContentItemProvider, FsGuideProvider, GuideProvider, GuideTools};
use fabryk_mcp_core::ToolRegistry;
use fabryk_redis::{MockRedis, RedisOps, get_multi_json};
use http::Request;
use serde_json::Value;
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use tower::{Layer, ServiceExt, service_fn};

struct ProbeValidator;

impl TokenValidator for ProbeValidator {
    fn validate(&self, token: &str, _: &AuthConfig) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<AuthenticatedUser, AuthError>> + Send + '_>> {
        let result = match token {
            "valid" => Ok(AuthenticatedUser { email: "auditor@acme.example".into(), subject: "acme-user-1".into() }),
            "infra" => Err(AuthError::JwksFetchError("fixture upstream timeout".into())),
            _ => Err(AuthError::InvalidSignature("fixture invalid token".into())),
        };
        Box::pin(async move { result })
    }
}

async fn auth_probe() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_inner = calls.clone();
    let inner = service_fn(move |_req: Request<Body>| {
        calls_inner.fetch_add(1, Ordering::SeqCst);
        async { Ok::<_, Infallible>(Response::new(Body::from("inner-ok"))) }
    });
    let svc = AuthLayer::new(Arc::new(ProbeValidator), AuthConfig { enabled: true, audience: "acme-client".into(), domain: "acme.example".into() }).layer(inner);
    for token in ["infra", "invalid", "valid"] {
        let req = Request::builder().uri("/").header("authorization", format!("Bearer {token}")).body(Body::empty()).unwrap();
        let resp = svc.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let challenge = resp.headers().get("www-authenticate").and_then(|v| v.to_str().ok()).unwrap_or("<absent>").to_string();
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        println!("AUTH token={token} status={status} challenge={challenge:?} body={}", String::from_utf8_lossy(&body));
    }
    let req = Request::builder().uri("/").header("authorization", "Bearer ").body(Body::empty()).unwrap();
    let resp = svc.clone().oneshot(req).await.unwrap();
    println!("AUTH empty status={} inner_calls={}", resp.status(), calls.load(Ordering::SeqCst));
    let req = Request::builder().uri("/").header("authorization", "bearer valid").body(Body::empty()).unwrap();
    let resp = svc.oneshot(req).await.unwrap();
    println!("AUTH lower_scheme status={} inner_calls={}", resp.status(), calls.load(Ordering::SeqCst));
}

async fn file_probe() {
    let root = tempfile::tempdir().unwrap();
    let content_root = root.path().join("content");
    let guides_root = root.path().join("guides");
    tokio::fs::create_dir_all(&content_root).await.unwrap();
    tokio::fs::create_dir_all(&guides_root).await.unwrap();
    tokio::fs::write(root.path().join("outside.md"), "Acme outside fixture").await.unwrap();
    tokio::fs::write(content_root.join("public.md"), "public fixture").await.unwrap();
    tokio::fs::write(root.path().join("outside-guide.md"), "Acme guide outside fixture").await.unwrap();
    tokio::fs::create_dir(content_root.join("folder.md")).await.unwrap();
    let items = FsContentItemProvider::new(&content_root);
    let guides = FsGuideProvider::new(&guides_root);
    for (label, result) in [("get_item traversal", items.get_item("../outside").await.map(|v| v.content)), ("get_guide traversal", guides.get_guide("../outside-guide").await)] {
        println!("FILE {label}: {}", match result { Ok(s) => format!("OK content={s:?}"), Err(e) => format!("ERR {e}") });
    }
    let dir_result = items.get_item("folder").await;
    println!("FILE directory_named_md: {}", match dir_result { Ok(v) => format!("OK content={:?}", v.content), Err(e) => format!("ERR {e}") });
    let item_tool = ContentTools::new(FsContentItemProvider::new(&content_root));
    let item_dispatch = item_tool.call("get", serde_json::json!({"id":"../outside"})).unwrap().await;
    println!("MCP_CONTENT get dispatch traversal: {item_dispatch:?}");
    let guide_tool = GuideTools::new(FsGuideProvider::new(&guides_root));
    let guide_dispatch = guide_tool.call("get_guide", serde_json::json!({"id":"../outside-guide"})).unwrap().await;
    println!("MCP_CONTENT get_guide dispatch traversal: {guide_dispatch:?}");
}

async fn lifecycle_probe() {
    let zero = ServiceHandle::new("zero-attempts");
    let join = spawn_with_retry(zero.clone(), RetryConfig { max_attempts: 0, initial_delay: Duration::ZERO, max_delay: Duration::ZERO }, || async { Ok(()) });
    join.await.unwrap();
    println!("LIFECYCLE max_attempts=0 state={}", zero.state());

    let retry = ServiceHandle::new("retry-once");
    let attempts = Arc::new(AtomicUsize::new(0));
    let attempts_init = attempts.clone();
    let join = spawn_with_retry(retry.clone(), RetryConfig { max_attempts: 2, initial_delay: Duration::ZERO, max_delay: Duration::ZERO }, move || {
        let attempts = attempts_init.clone();
        async move {
            if attempts.fetch_add(1, Ordering::SeqCst) == 0 { Err("fixture first failure".into()) } else { Ok(()) }
        }
    });
    join.await.unwrap();
    println!("LIFECYCLE successful_retry attempts={} state={}", attempts.load(Ordering::SeqCst), retry.state());

    let exhausted = ServiceHandle::new("exhausted");
    let join = spawn_with_retry(exhausted.clone(), RetryConfig { max_attempts: 2, initial_delay: Duration::ZERO, max_delay: Duration::ZERO }, || async { Err("fixture exhausted".into()) });
    join.await.unwrap();
    println!("LIFECYCLE exhausted state={}", exhausted.state());

    let panicked = ServiceHandle::new("panic-init");
    let join = spawn_with_retry(panicked.clone(), RetryConfig { max_attempts: 1, initial_delay: Duration::ZERO, max_delay: Duration::ZERO }, || async { panic!("fixture panic") });
    let join_result = join.await;
    let wait = panicked.wait_ready(Duration::from_millis(5)).await;
    println!("LIFECYCLE panic join_error={} state={} wait_ready={wait:?}", join_result.is_err(), panicked.state());

    let slot: BackendSlot<u32> = BackendSlot::new("empty-ready");
    slot.service().set_state(ServiceState::Ready);
    let guard = slot.require().unwrap();
    println!("LIFECYCLE manually_ready_empty_slot={:?}", *guard);
}

async fn discovery_probe() {
    let app = fabryk_mcp_auth::discovery_routes("https://api.acme.example/mcp", "https://login.acme.example");
    for path in ["/.well-known/oauth-authorization-server", "/.well-known/oauth-authorization-server/mcp", "/.well-known/oauth-authorization-server/mcp/", "/.well-known/oauth-protected-resource", "/.well-known/oauth-protected-resource/mcp", "/.well-known/oauth-protected-resource/mcp/"] {
        let response = app.clone().oneshot(Request::builder().uri(path).body(Body::empty()).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let metadata: Result<Value, _> = serde_json::from_slice(&bytes);
        println!("DISCOVERY path={path} status={status} metadata={metadata:?}");
    }
}

async fn google_jwks_probe() {
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => {
            println!("GOOGLE local_jwks_fixture unavailable: {error}; no provider request attempted");
            return;
        }
    };
    let addr = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = requests.clone();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let counter = counter.clone();
            tokio::spawn(async move {
                let mut request = [0u8; 2048];
                let _ = stream.read(&mut request).await;
                counter.fetch_add(1, Ordering::SeqCst);
                let body = br#"{"keys":[]}"#;
                let response = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let _ = stream.write_all(response.as_bytes()).await;
                let _ = stream.write_all(body).await;
            });
        }
    });
    let validator = fabryk_auth_google::GoogleTokenValidator::new(format!("http://{addr}/jwks"));
    let token = "eyJhbGciOiJSUzI1NiIsImtpZCI6InVua25vd24ifQ.e30.eA";
    let config = AuthConfig { enabled: true, audience: "acme-client".into(), domain: "acme.example".into() };
    for _ in 0..2 {
        let result = validator.validate(token, &config).await;
        println!("GOOGLE unknown_kid result={result:?} fixture_requests={}", requests.load(Ordering::SeqCst));
    }
}

async fn redis_probe() {
    let redis = MockRedis::new();
    redis.set_str("counter", "not-a-number").await.unwrap();
    println!("REDIS mock get_u64 invalid={:?}", redis.get_u64("counter").await);
    println!("REDIS mock incr_by invalid-current={:?}", redis.incr_by("counter", 1).await);
    redis.set_str("ab1", "v").await.unwrap();
    println!("REDIS mock scan_keys glob_a_question_star={:?}", redis.scan_keys("a?*").await);
    redis.set_str("bad-json", "not-json").await.unwrap();
    println!("REDIS mock get_multi_json invalid-json={:?}", get_multi_json::<String>(&redis, &["bad-json".into()]).await);
}

fn adc_probe() {
    println!("ADC GOOGLE_APPLICATION_CREDENTIALS={:?} resolved={:?}", std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok(), fabryk_gcp::resolve_adc_path());
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("adc") { adc_probe(); return; }
    auth_probe().await;
    file_probe().await;
    lifecycle_probe().await;
    discovery_probe().await;
    google_jwks_probe().await;
    redis_probe().await;
}
