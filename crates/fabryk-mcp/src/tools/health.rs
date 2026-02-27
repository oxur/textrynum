//! Health check tool for MCP servers.
//!
//! Provides a built-in `health` tool that reports server status,
//! tool count, and version information.

use crate::registry::{ToolRegistry, ToolResult};
use rmcp::model::{CallToolResult, Content, ErrorData, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Health check response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status ("healthy").
    pub status: String,
    /// Server name.
    pub server_name: String,
    /// Server version.
    pub version: String,
    /// Number of registered tools.
    pub tool_count: usize,
}

/// A tool registry that provides the `health` tool.
///
/// Captures server metadata at construction time and reports it
/// when the tool is called.
pub struct HealthTools {
    server_name: String,
    version: String,
    total_tool_count: usize,
}

impl HealthTools {
    /// Create health tools with server metadata.
    ///
    /// `total_tool_count` should include the health tool itself.
    pub fn new(
        server_name: impl Into<String>,
        version: impl Into<String>,
        total_tool_count: usize,
    ) -> Self {
        Self {
            server_name: server_name.into(),
            version: version.into(),
            total_tool_count,
        }
    }
}

impl ToolRegistry for HealthTools {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool {
            name: "health".into(),
            description: Some("Check server health and status".into()),
            input_schema: Arc::new(serde_json::Map::new()),
            title: None,
            output_schema: None,
            annotations: None,
            icons: None,
            execution: None,
            meta: None,
        }]
    }

    fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
        if name != "health" {
            return None;
        }

        let response = HealthResponse {
            status: "healthy".to_string(),
            server_name: self.server_name.clone(),
            version: self.version.clone(),
            tool_count: self.total_tool_count,
        };

        Some(Box::pin(async move {
            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }))
    }
}

/// Handle a health check request directly (without registry).
///
/// Useful for testing or when building custom tool dispatchers.
pub async fn handle_health(
    server_name: &str,
    version: &str,
    tool_count: usize,
) -> Result<CallToolResult, ErrorData> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        server_name: server_name.to_string(),
        version: version.to_string(),
        tool_count,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            server_name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            tool_count: 5,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("test-server"));
        assert!(json.contains("0.1.0"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{"status":"healthy","server_name":"test","version":"1.0","tool_count":3}"#;
        let response: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "healthy");
        assert_eq!(response.tool_count, 3);
    }

    #[test]
    fn test_health_tools_creation() {
        let tools = HealthTools::new("server", "1.0", 10);
        assert_eq!(tools.tool_count(), 1);
        assert!(tools.has_tool("health"));
        assert!(!tools.has_tool("other"));
    }

    #[tokio::test]
    async fn test_health_tools_call() {
        let tools = HealthTools::new("test-server", "0.1.0", 5);
        let future = tools.call("health", json!({})).unwrap();
        let result = future.await.unwrap();

        // Should be a success result
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_health_tools_unknown_tool() {
        let tools = HealthTools::new("server", "1.0", 1);
        assert!(tools.call("unknown", json!({})).is_none());
    }

    #[tokio::test]
    async fn test_handle_health_direct() {
        let result = handle_health("my-server", "2.0.0", 10).await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_health_tool_info() {
        let tools = HealthTools::new("server", "1.0", 1);
        let tool_list = tools.tools();
        assert_eq!(tool_list.len(), 1);
        assert_eq!(tool_list[0].name, "health");
        assert!(tool_list[0].description.is_some());
    }
}
