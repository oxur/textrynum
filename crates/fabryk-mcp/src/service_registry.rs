//! Service-aware tool registry.
//!
//! Wraps a [`ToolRegistry`] to gate tool calls on service readiness.
//! Tools are always listed (so MCP clients know they exist), but calls
//! return informative errors when backing services aren't ready.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::ServiceAwareRegistry;
//! use fabryk_core::ServiceHandle;
//!
//! let fts_svc = ServiceHandle::new("fts");
//! let gated = ServiceAwareRegistry::new(fts_tools, vec![fts_svc]);
//! ```

use crate::registry::{ToolRegistry, ToolResult};
use fabryk_core::service::ServiceHandle;
use rmcp::model::{CallToolResult, Content, Tool};
use serde_json::Value;

/// A registry wrapper that gates tool calls on service readiness.
///
/// - `tools()` always returns the full tool list (so clients know they exist).
/// - `call()` checks all service handles before delegating. If any service
///   is not available, returns an informative error message.
pub struct ServiceAwareRegistry {
    inner: Box<dyn ToolRegistry>,
    services: Vec<ServiceHandle>,
}

impl ServiceAwareRegistry {
    /// Create a new service-aware registry.
    ///
    /// All `services` must be available (Ready or Degraded) for tool calls
    /// to be delegated to the inner registry.
    pub fn new<R: ToolRegistry + 'static>(registry: R, services: Vec<ServiceHandle>) -> Self {
        Self {
            inner: Box::new(registry),
            services,
        }
    }
}

impl ToolRegistry for ServiceAwareRegistry {
    fn tools(&self) -> Vec<Tool> {
        // Always list tools so clients know they exist
        self.inner.tools()
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        // Check if tool exists in inner registry first
        if !self.inner.has_tool(name) {
            return None;
        }

        // Check all service dependencies (live check, not cached)
        for svc in &self.services {
            let state = svc.state();
            if !state.is_available() {
                let svc_name = svc.name().to_string();
                let tool_name = name.to_string();
                let state_str = state.to_string();
                let elapsed = svc.elapsed();
                return Some(Box::pin(async move {
                    let msg = format!(
                        "Tool '{tool_name}' is unavailable: \
                         service '{svc_name}' is {state_str} \
                         (elapsed: {elapsed:.1?}). Try again shortly."
                    );
                    Ok(CallToolResult::error(vec![Content::text(msg)]))
                }));
            }
        }

        // All services ready — delegate
        self.inner.call(name, args)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use fabryk_core::service::ServiceState;
    use rmcp::model::RawContent;
    use std::sync::Arc;

    fn make_tool(name: &str, description: &str) -> Tool {
        Tool {
            name: name.to_string().into(),
            description: Some(description.to_string().into()),
            input_schema: Arc::new(serde_json::Map::new()),
            title: None,
            output_schema: None,
            annotations: None,
            icons: None,
            execution: None,
            meta: None,
        }
    }

    struct MockRegistry {
        tools: Vec<Tool>,
    }

    impl ToolRegistry for MockRegistry {
        fn tools(&self) -> Vec<Tool> {
            self.tools.clone()
        }

        fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
            if self.has_tool(name) {
                let name = name.to_string();
                Some(Box::pin(async move {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "result: {name}"
                    ))]))
                }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_service_aware_lists_tools_always() {
        let svc = ServiceHandle::new("test");
        // Service is Stopped — tools should still be listed
        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("search", "Search"), make_tool("index", "Index")],
            },
            vec![svc],
        );

        assert_eq!(registry.tools().len(), 2);
        assert!(registry.has_tool("search"));
    }

    #[tokio::test]
    async fn test_service_aware_gates_when_starting() {
        let svc = ServiceHandle::new("fts");
        svc.set_state(ServiceState::Starting);

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("search", "Search")],
            },
            vec![svc],
        );

        let result = registry.call("search", Value::Null).unwrap().await.unwrap();
        assert_eq!(result.is_error, Some(true));

        // Check error message contains service name and state
        let text = match &result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("search"));
        assert!(text.contains("fts"));
        assert!(text.contains("starting"));
    }

    #[tokio::test]
    async fn test_service_aware_delegates_when_ready() {
        let svc = ServiceHandle::new("fts");
        svc.set_state(ServiceState::Ready);

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("search", "Search")],
            },
            vec![svc],
        );

        let result = registry.call("search", Value::Null).unwrap().await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_service_aware_delegates_when_degraded() {
        let svc = ServiceHandle::new("fts");
        svc.set_state(ServiceState::Degraded("slow".to_string()));

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("search", "Search")],
            },
            vec![svc],
        );

        let result = registry.call("search", Value::Null).unwrap().await.unwrap();
        // Degraded is available, so should delegate
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_service_aware_multiple_services() {
        let svc1 = ServiceHandle::new("fts");
        let svc2 = ServiceHandle::new("vector");
        svc1.set_state(ServiceState::Ready);
        svc2.set_state(ServiceState::Starting); // Not ready

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("hybrid_search", "Hybrid search")],
            },
            vec![svc1, svc2],
        );

        let result = registry
            .call("hybrid_search", Value::Null)
            .unwrap()
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(true));

        let text = match &result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        assert!(text.contains("vector"));
    }

    #[test]
    fn test_service_aware_unknown_tool_returns_none() {
        let svc = ServiceHandle::new("fts");
        svc.set_state(ServiceState::Ready);

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("search", "Search")],
            },
            vec![svc],
        );

        // Unknown tool should return None, not an error
        assert!(registry.call("nonexistent", Value::Null).is_none());
    }

    #[tokio::test]
    async fn test_service_aware_error_message_format() {
        let svc = ServiceHandle::new("graph-builder");
        svc.set_state(ServiceState::Starting);

        let registry = ServiceAwareRegistry::new(
            MockRegistry {
                tools: vec![make_tool("traverse", "Graph traversal")],
            },
            vec![svc],
        );

        let result = registry
            .call("traverse", Value::Null)
            .unwrap()
            .await
            .unwrap();
        let text = match &result.content[0].raw {
            RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };

        // Should contain: tool name, service name, state, elapsed, retry hint
        assert!(text.contains("traverse"));
        assert!(text.contains("graph-builder"));
        assert!(text.contains("starting"));
        assert!(text.contains("elapsed"));
        assert!(text.contains("Try again"));
    }
}
