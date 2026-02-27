//! Generic MCP server for Fabryk domains.
//!
//! Wraps an `rmcp` server with a `ToolRegistry` for runtime-composable
//! tool dispatch. The server implements rmcp's `ServerHandler` trait,
//! delegating tool listing and dispatch to the registry.

use crate::registry::ToolRegistry;
use fabryk_core::service::{ServiceHandle, ServiceState};
use rmcp::model::{
    CallToolResult, Content, ErrorData, Implementation, ProtocolVersion, ServerCapabilities,
    ServerInfo,
};
use rmcp::service::RequestContext;
use rmcp::transport;
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the MCP server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server name shown to clients.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Optional server description / instructions.
    pub description: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            name: "fabryk-server".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: None,
        }
    }
}

/// Generic MCP server for Fabryk domains.
///
/// Bridges the `ToolRegistry` abstraction to rmcp's `ServerHandler`.
/// The server delegates `list_tools` and `call_tool` to the registry,
/// allowing runtime composition of tools from multiple sources.
///
/// # Example
///
/// ```rust,ignore
/// use fabryk_mcp::{FabrykMcpServer, CompositeRegistry};
///
/// let registry = CompositeRegistry::new()
///     .add(content_tools)
///     .add(search_tools);
///
/// let server = FabrykMcpServer::new(registry)
///     .with_name("music-theory")
///     .with_description("Music theory knowledge assistant");
///
/// server.serve_stdio().await?;
/// ```
#[derive(Clone)]
pub struct FabrykMcpServer {
    registry: Arc<dyn ToolRegistry>,
    config: ServerConfig,
    services: Vec<ServiceHandle>,
}

impl FabrykMcpServer {
    /// Create a new MCP server with the given tool registry.
    pub fn new<R: ToolRegistry + 'static>(registry: R) -> Self {
        Self {
            registry: Arc::new(registry),
            config: ServerConfig::default(),
            services: Vec::new(),
        }
    }

    /// Set the server name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the server version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.config.version = version.into();
        self
    }

    /// Set the server description / instructions.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.config.description = Some(description.into());
        self
    }

    /// Register a service handle for health tracking.
    pub fn with_service(mut self, handle: ServiceHandle) -> Self {
        self.services.push(handle);
        self
    }

    /// Register multiple service handles for health tracking.
    pub fn with_services(mut self, handles: Vec<ServiceHandle>) -> Self {
        self.services.extend(handles);
        self
    }

    /// Check if all registered services are ready.
    pub fn is_ready(&self) -> bool {
        self.services.iter().all(|s| s.state().is_ready())
    }

    /// Get health status of all registered services.
    pub fn health(&self) -> Vec<(String, ServiceState)> {
        self.services
            .iter()
            .map(|s| (s.name().to_string(), s.state()))
            .collect()
    }

    /// Wait for all registered services to be ready (with timeout).
    ///
    /// Waits sequentially for each service. Returns all errors if any
    /// services fail to reach Ready within the timeout.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for svc in &self.services {
            if let Err(e) = svc.wait_ready(timeout).await {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get the tool registry.
    pub fn registry(&self) -> &dyn ToolRegistry {
        &*self.registry
    }

    /// Run the server on stdio transport.
    ///
    /// This starts the MCP server listening on stdin/stdout and blocks
    /// until the connection is closed.
    pub async fn serve_stdio(self) -> fabryk_core::Result<()> {
        log::info!(
            "Starting {} v{} with {} tools",
            self.config.name,
            self.config.version,
            self.registry.tool_count()
        );

        let transport = transport::stdio();

        let server = self
            .serve(transport)
            .await
            .map_err(|e| fabryk_core::Error::operation(format!("MCP server error: {e}")))?;

        server
            .waiting()
            .await
            .map_err(|e| fabryk_core::Error::operation(format!("MCP server terminated: {e}")))?;

        log::info!("Server shut down.");
        Ok(())
    }
}

impl ServerHandler for FabrykMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: self.config.name.clone(),
                version: self.config.version.clone(),
                ..Default::default()
            },
            instructions: self.config.description.clone(),
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListToolsResult, ErrorData>> + Send + '_
    {
        let tools = self.registry.tools();
        async move {
            Ok(rmcp::model::ListToolsResult {
                tools,
                next_cursor: None,
                meta: None,
            })
        }
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        let name = request.name.to_string();
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::Value::Null);

        async move {
            match self.registry.call(&name, args) {
                Some(future) => future.await,
                None => Ok(CallToolResult::error(vec![Content::text(format!(
                    "Unknown tool: {name}"
                ))])),
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{CompositeRegistry, ToolResult};
    use fabryk_core::service::ServiceState;
    use rmcp::model::Tool;

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

    struct MockRegistry;

    impl ToolRegistry for MockRegistry {
        fn tools(&self) -> Vec<Tool> {
            vec![make_tool("test_tool", "A test tool")]
        }

        fn call(&self, name: &str, _args: serde_json::Value) -> Option<ToolResult> {
            if name == "test_tool" {
                Some(Box::pin(async {
                    Ok(CallToolResult::success(vec![Content::text("ok")]))
                }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_server_creation() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        assert_eq!(server.config().name, "fabryk-server");
        assert_eq!(server.registry().tool_count(), 0);
    }

    #[test]
    fn test_server_with_name() {
        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_name("my-server");
        assert_eq!(server.config().name, "my-server");
    }

    #[test]
    fn test_server_with_version() {
        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_version("1.0.0");
        assert_eq!(server.config().version, "1.0.0");
    }

    #[test]
    fn test_server_with_description() {
        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_description("Test server");
        assert_eq!(server.config().description, Some("Test server".to_string()));
    }

    #[test]
    fn test_server_with_registry() {
        let server = FabrykMcpServer::new(MockRegistry);
        assert_eq!(server.registry().tool_count(), 1);
    }

    #[test]
    fn test_server_get_info() {
        let server = FabrykMcpServer::new(MockRegistry)
            .with_name("test")
            .with_version("0.1.0")
            .with_description("Test desc");

        let info = server.get_info();
        assert_eq!(info.server_info.name, "test");
        assert_eq!(info.server_info.version, "0.1.0");
        assert_eq!(info.instructions, Some("Test desc".to_string()));
        assert!(info.capabilities.tools.is_some());
    }

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.name, "fabryk-server");
        assert!(config.description.is_none());
    }

    #[test]
    fn test_server_config_serialization() {
        let config = ServerConfig {
            name: "test".to_string(),
            version: "1.0".to_string(),
            description: Some("desc".to_string()),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test"));

        let deserialized: ServerConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
    }

    #[test]
    fn test_server_with_services() {
        let svc1 = ServiceHandle::new("graph");
        let svc2 = ServiceHandle::new("fts");

        let server = FabrykMcpServer::new(CompositeRegistry::new())
            .with_service(svc1)
            .with_services(vec![svc2]);

        assert_eq!(server.health().len(), 2);
    }

    #[test]
    fn test_server_is_ready_all_ready() {
        let svc1 = ServiceHandle::new("graph");
        let svc2 = ServiceHandle::new("fts");
        svc1.set_state(ServiceState::Ready);
        svc2.set_state(ServiceState::Ready);

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_services(vec![svc1, svc2]);

        assert!(server.is_ready());
    }

    #[test]
    fn test_server_is_ready_not_all_ready() {
        let svc1 = ServiceHandle::new("graph");
        let svc2 = ServiceHandle::new("fts");
        svc1.set_state(ServiceState::Ready);
        svc2.set_state(ServiceState::Starting);

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_services(vec![svc1, svc2]);

        assert!(!server.is_ready());
    }

    #[test]
    fn test_server_is_ready_no_services() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        // No services registered — vacuously ready
        assert!(server.is_ready());
    }

    #[test]
    fn test_server_health_reports_states() {
        let svc1 = ServiceHandle::new("graph");
        let svc2 = ServiceHandle::new("fts");
        svc1.set_state(ServiceState::Ready);
        svc2.set_state(ServiceState::Starting);

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_services(vec![svc1, svc2]);

        let health = server.health();
        assert_eq!(health[0], ("graph".to_string(), ServiceState::Ready));
        assert_eq!(health[1], ("fts".to_string(), ServiceState::Starting));
    }

    #[tokio::test]
    async fn test_server_wait_ready_success() {
        let svc = ServiceHandle::new("fast");
        svc.set_state(ServiceState::Ready);

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_service(svc);

        let result = server.wait_ready(Duration::from_millis(50)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_server_wait_ready_timeout() {
        let svc = ServiceHandle::new("slow");
        svc.set_state(ServiceState::Starting);

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_service(svc);

        let result = server.wait_ready(Duration::from_millis(50)).await;
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not ready after"));
    }
}
