//! Diagnostic MCP tools for Fabryk servers.
//!
//! Provides generic diagnostic tools that work with any Fabryk-based
//! MCP server:
//!
//! - `debug_config` — show server configuration (with optional redaction)
//! - `service_status` — report status of all background services

use crate::helpers::{make_tool_no_params, serialize_response};
use crate::registry::{ToolRegistry, ToolResult};
use fabryk_core::service::ServiceHandle;
use rmcp::model::{CallToolResult, ErrorData, Tool};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// DiagnosticTools
// ---------------------------------------------------------------------------

/// Generic diagnostic tools for Fabryk MCP servers.
///
/// Provides `debug_config` and `service_status` tools. The config type
/// is generic — any `Serialize` type works.
///
/// # Redaction
///
/// Fields named `client_id` or `client_secret` under an `oauth` key are
/// automatically redacted in the `debug_config` output. For custom
/// redaction, pre-process the config before passing it.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_mcp::DiagnosticTools;
///
/// let diagnostics = DiagnosticTools::new(config.clone())
///     .with_services(vec![graph_svc, fts_svc]);
///
/// let registry = CompositeRegistry::new().add(diagnostics);
/// ```
pub struct DiagnosticTools<C: Serialize + Send + Sync> {
    config: Arc<C>,
    services: Vec<ServiceHandle>,
}

impl<C: Serialize + Send + Sync> DiagnosticTools<C> {
    /// Create diagnostic tools with a shared config reference.
    pub fn new(config: Arc<C>) -> Self {
        Self {
            config,
            services: Vec::new(),
        }
    }

    /// Add service handles for status reporting.
    pub fn with_services(mut self, services: Vec<ServiceHandle>) -> Self {
        self.services = services;
        self
    }
}

impl<C: Serialize + Send + Sync + 'static> ToolRegistry for DiagnosticTools<C> {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool_no_params(
                "debug_config",
                "Show the server configuration (sensitive values redacted)",
            ),
            make_tool_no_params(
                "service_status",
                "Show the status of all background services",
            ),
        ]
    }

    fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
        match name {
            "debug_config" => {
                let config = self.config.clone();
                Some(Box::pin(async move {
                    let mut config_json = serde_json::to_value(&*config)
                        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

                    // Redact sensitive fields under oauth
                    if let Some(oauth) = config_json.get_mut("oauth")
                        && let Some(obj) = oauth.as_object_mut()
                    {
                        for key in &["client_id", "client_secret"] {
                            if obj.contains_key(*key) {
                                obj.insert(
                                    key.to_string(),
                                    Value::String("***redacted***".to_string()),
                                );
                            }
                        }
                    }

                    serialize_response(&config_json)
                }))
            }
            "service_status" => {
                let services = self.services.clone();
                Some(Box::pin(async move {
                    let statuses: Vec<_> = services
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "name": s.name(),
                                "state": s.state().to_string(),
                                "elapsed": format!("{:.1?}", s.elapsed()),
                            })
                        })
                        .collect();
                    let all_ready = services.iter().all(|s| s.state().is_ready());
                    let response = serde_json::json!({
                        "status": if all_ready { "ready" } else { "starting" },
                        "services": statuses,
                    });
                    serialize_response(&response)
                }))
            }
            _ => None,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fabryk_core::service::ServiceState;
    use serde::Serialize;

    #[derive(Clone, Serialize)]
    struct TestConfig {
        name: String,
        port: u16,
        oauth: TestOAuth,
    }

    #[derive(Clone, Serialize)]
    struct TestOAuth {
        client_id: String,
        client_secret: String,
        enabled: bool,
    }

    fn test_config() -> Arc<TestConfig> {
        Arc::new(TestConfig {
            name: "test-server".to_string(),
            port: 8080,
            oauth: TestOAuth {
                client_id: "secret-id".to_string(),
                client_secret: "secret-secret".to_string(),
                enabled: true,
            },
        })
    }

    fn test_tools() -> DiagnosticTools<TestConfig> {
        DiagnosticTools::new(test_config())
    }

    #[test]
    fn test_tool_count() {
        let tools = test_tools();
        assert_eq!(tools.tools().len(), 2);
    }

    #[test]
    fn test_has_debug_config() {
        let tools = test_tools();
        assert!(tools.has_tool("debug_config"));
    }

    #[test]
    fn test_has_service_status() {
        let tools = test_tools();
        assert!(tools.has_tool("service_status"));
    }

    #[test]
    fn test_unknown_tool_returns_none() {
        let tools = test_tools();
        assert!(tools.call("nonexistent", Value::Null).is_none());
    }

    #[tokio::test]
    async fn test_debug_config_redacts_oauth() {
        let tools = test_tools();
        let result = tools
            .call("debug_config", Value::Null)
            .unwrap()
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        let text = format!("{result:?}");
        assert!(text.contains("test-server"));
        assert!(text.contains("***redacted***"));
        assert!(!text.contains("secret-id"));
        assert!(!text.contains("secret-secret"));
    }

    #[tokio::test]
    async fn test_service_status_no_services() {
        let tools = test_tools();
        let result = tools
            .call("service_status", Value::Null)
            .unwrap()
            .await
            .unwrap();

        assert_eq!(result.is_error, Some(false));
        let text = format!("{result:?}");
        assert!(text.contains("ready")); // No services = all ready (vacuous truth)
    }

    #[tokio::test]
    async fn test_service_status_with_services() {
        let svc = ServiceHandle::new("test-svc");
        svc.set_state(ServiceState::Ready);
        let tools = DiagnosticTools::new(test_config()).with_services(vec![svc]);

        let result = tools
            .call("service_status", Value::Null)
            .unwrap()
            .await
            .unwrap();

        let text = format!("{result:?}");
        assert!(text.contains("ready"));
        assert!(text.contains("test-svc"));
    }

    #[tokio::test]
    async fn test_service_status_starting() {
        let svc = ServiceHandle::new("building-svc");
        let tools = DiagnosticTools::new(test_config()).with_services(vec![svc]);

        let result = tools
            .call("service_status", Value::Null)
            .unwrap()
            .await
            .unwrap();

        let text = format!("{result:?}");
        assert!(text.contains("starting"));
    }
}
