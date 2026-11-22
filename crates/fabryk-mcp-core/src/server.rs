//! Generic MCP server for Fabryk domains.
//!
//! Wraps an `rmcp` server with a `ToolRegistry` for runtime-composable
//! tool dispatch. The server implements rmcp's `ServerHandler` trait,
//! delegating tool listing and dispatch to the registry.

use crate::notifier::Notifier;
use crate::registry::ToolRegistry;
use crate::resource::ResourceRegistry;
use fabryk_core::service::{ServiceHandle, ServiceState};
use rmcp::model::{
    CallToolResponse, CallToolResult, ContentBlock, ErrorData, Implementation, ListResourcesResult,
    ListToolsResult, ProtocolVersion, ReadResourceRequestParams, ReadResourceResponse,
    ReadResourceResult, ServerCapabilities, ServerConfig as RmcpServerConfig,
    SubscribeRequestParams, UnsubscribeRequestParams,
};
use rmcp::service::{NotificationContext, RequestContext};
use rmcp::transport;
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "http")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

/// HTTP service type alias — hides the session-manager generic (API-01).
#[cfg(feature = "http")]
pub type HttpService = StreamableHttpService<FabrykMcpServer, LocalSessionManager>;

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
    notifier: Notifier,
    resource_registry: Option<Arc<dyn ResourceRegistry>>,
}

impl FabrykMcpServer {
    /// Create a new MCP server with the given tool registry.
    pub fn new<R: ToolRegistry + 'static>(registry: R) -> Self {
        Self {
            registry: Arc::new(registry),
            config: ServerConfig::default(),
            services: Vec::new(),
            notifier: Notifier::new(),
            resource_registry: None,
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
    /// Waits for all services **in parallel** using `futures::join_all`.
    /// The wall-clock time equals the slowest service, not the sum of all.
    /// Returns all errors if any services fail to reach Ready within the timeout.
    pub async fn wait_ready(&self, timeout: Duration) -> Result<(), Vec<String>> {
        fabryk_core::service::wait_all_ready(&self.services, timeout).await
    }

    /// Auto-generate server instructions for discoverability.
    ///
    /// Prepends a directive telling AI agents to call the directory tool first.
    /// If a description was already set via [`with_description`](Self::with_description),
    /// it is preserved after the directive. Use with
    /// [`DiscoverableRegistry`](crate::DiscoverableRegistry).
    pub fn with_discoverable_instructions(mut self, server_name: &str) -> Self {
        let directive = format!(
            "ALWAYS call {server_name}_directory first — it maps all available tools, \
             valid filter values, and the optimal query strategy for this session."
        );
        self.config.description = Some(match self.config.description.take() {
            Some(existing) => format!("{directive}\n\n{existing}"),
            None => directive,
        });
        self
    }

    /// Set server guidance, populating the description/instructions.
    pub fn with_guidance(mut self, guidance: &crate::guidance::ServerGuidance) -> Self {
        self.config.description = Some(guidance.to_instructions());
        self
    }

    /// Register a resource registry for MCP resource support.
    ///
    /// Enables `resources/list`, `resources/read`, `resources/subscribe`,
    /// and `resources/unsubscribe` in the server capabilities.
    pub fn with_resources<R: ResourceRegistry + 'static>(mut self, registry: R) -> Self {
        self.resource_registry = Some(Arc::new(registry));
        self
    }

    /// Get the server configuration.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get the tool registry.
    pub fn registry(&self) -> &dyn ToolRegistry {
        &*self.registry
    }

    /// Get the notifier for broadcasting to connected clients.
    ///
    /// The returned `Notifier` is cheaply cloneable and can be stored
    /// in any async context. Call this before `serve_stdio()` or
    /// `serve_http()`, which consume the server.
    pub fn notifier(&self) -> Notifier {
        self.notifier.clone()
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

    /// Build a streamable HTTP service for composing into an axum router.
    ///
    /// Returns a Tower `Service<Request<Body>>` that handles MCP over
    /// streamable HTTP. Callers can wrap it with auth middleware, nest
    /// it in an axum router, etc.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let mcp_service = server.into_http_service();
    /// let router = Router::new()
    ///     .route("/health", get(|| async { "ok" }))
    ///     .nest_service("/mcp", mcp_service);
    /// ```
    #[cfg(feature = "http")]
    pub fn into_http_service(self) -> HttpService {
        self.into_http_service_with_config(StreamableHttpServerConfig::default())
    }

    /// Build a streamable HTTP service with custom config.
    #[cfg(feature = "http")]
    pub fn into_http_service_with_config(self, config: StreamableHttpServerConfig) -> HttpService {
        log::info!(
            "Building HTTP service for {} v{} with {} tools",
            self.config.name,
            self.config.version,
            self.registry.tool_count()
        );
        StreamableHttpService::new(
            move || Ok(self.clone()),
            Arc::new(LocalSessionManager::default()),
            config,
        )
    }

    /// Convenience: serve HTTP on a given address with a minimal router.
    ///
    /// Creates a router with the MCP service at `/mcp` and a health
    /// endpoint at `/health`. For custom routers or auth middleware,
    /// use `into_http_service()` instead.
    #[cfg(feature = "http")]
    pub async fn serve_http(self, addr: std::net::SocketAddr) -> fabryk_core::Result<()> {
        let service = self.clone().into_http_service();
        let server_name = self.config.name.clone();
        let services = self.services.clone();

        let router = axum::Router::new()
            .merge(crate::health_router::health_router(services))
            .nest_service("/mcp", service);

        log::info!("{server_name} HTTP server listening on {addr}");

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| fabryk_core::Error::operation(format!("bind failed: {e}")))?;

        axum::serve(listener, router)
            .await
            .map_err(|e| fabryk_core::Error::operation(format!("HTTP server error: {e}")))?;

        log::info!("Server shut down.");
        Ok(())
    }
}

// Helper methods extracted for testability (ServerHandler methods need
// RequestContext which is difficult to construct in tests).
impl FabrykMcpServer {
    /// List all tools from the registry.
    pub(crate) fn list_tools_inner(&self) -> Vec<rmcp::model::Tool> {
        self.registry.tools()
    }

    /// Call a tool by name with the given arguments.
    pub(crate) async fn call_tool_inner(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<CallToolResult, ErrorData> {
        match self.registry.call(name, args) {
            Some(future) => future.await,
            None => Ok(CallToolResult::error(vec![ContentBlock::text(format!(
                "Unknown tool: {name}"
            ))])),
        }
    }

    /// List all resources from the resource registry.
    pub(crate) fn list_resources_inner(&self) -> ListResourcesResult {
        match &self.resource_registry {
            Some(registry) => ListResourcesResult::with_all_items(registry.resources()),
            None => ListResourcesResult::default(),
        }
    }

    /// Read a resource by URI.
    pub(crate) async fn read_resource_inner(
        &self,
        uri: &str,
    ) -> Result<ReadResourceResult, ErrorData> {
        let registry = self
            .resource_registry
            .as_ref()
            .ok_or_else(|| ErrorData::invalid_params("Resources not enabled", None))?;

        match registry.read(uri) {
            Some(future) => {
                let contents = future.await?;
                Ok(ReadResourceResult::new(contents))
            }
            None => Err(ErrorData::invalid_params(
                format!("Unknown resource: {uri}"),
                None,
            )),
        }
    }
}

impl ServerHandler for FabrykMcpServer {
    #[expect(
        deprecated,
        reason = "Retain the server logging capability for clients using the existing MCP logging contract."
    )]
    fn get_info(&self) -> RmcpServerConfig {
        let capabilities = if self.resource_registry.is_some() {
            ServerCapabilities::builder()
                .enable_tools()
                .enable_logging()
                .enable_resources()
                .enable_resources_subscribe()
                .enable_resources_list_changed()
                .build()
        } else {
            ServerCapabilities::builder()
                .enable_tools()
                .enable_logging()
                .build()
        };

        let mut info = RmcpServerConfig::new(capabilities)
            .with_protocol_version(ProtocolVersion::LATEST)
            .with_server_info(Implementation::new(
                self.config.name.clone(),
                self.config.version.clone(),
            ));
        if let Some(desc) = &self.config.description {
            info = info.with_instructions(desc.clone());
        }
        info
    }

    fn on_initialized(
        &self,
        context: NotificationContext<RoleServer>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let peer = context.peer;
        async move {
            self.notifier.add_peer(peer).await;
            log::info!(
                "MCP client connected ({} active)",
                self.notifier.client_count().await
            );
        }
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListToolsResult, ErrorData>> + Send + '_
    {
        let tools = self.list_tools_inner();
        async move { Ok(ListToolsResult::with_all_items(tools)) }
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResponse, ErrorData>> + Send + '_ {
        let name = request.name.to_string();
        let args = request
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or(serde_json::Value::Null);

        async move { self.call_tool_inner(&name, args).await.map(Into::into) }
    }

    #[allow(clippy::manual_async_fn)]
    fn list_resources(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListResourcesResult, ErrorData>> + Send + '_ {
        async move { Ok(self.list_resources_inner()) }
    }

    #[allow(clippy::manual_async_fn)]
    fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ReadResourceResponse, ErrorData>> + Send + '_
    {
        async move { self.read_resource_inner(&request.uri).await.map(Into::into) }
    }

    #[allow(clippy::manual_async_fn)]
    fn subscribe(
        &self,
        request: SubscribeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move {
            self.notifier
                .subscribe_resource(&request.uri, context.peer)
                .await;
            log::debug!("Client subscribed to {}", request.uri);
            Ok(())
        }
    }

    #[allow(clippy::manual_async_fn)]
    fn unsubscribe(
        &self,
        request: UnsubscribeRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), ErrorData>> + Send + '_ {
        async move {
            self.notifier.unsubscribe_resource(&request.uri).await;
            log::debug!("Client unsubscribed from {}", request.uri);
            Ok(())
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
        Tool::new(
            name.to_string(),
            description.to_string(),
            crate::empty_input_schema(),
        )
    }

    struct MockRegistry;

    impl ToolRegistry for MockRegistry {
        fn tools(&self) -> Vec<Tool> {
            vec![make_tool("test_tool", "A test tool")]
        }

        fn call(&self, name: &str, _args: serde_json::Value) -> Option<ToolResult> {
            if name == "test_tool" {
                Some(Box::pin(async {
                    Ok(CallToolResult::success(vec![ContentBlock::text("ok")]))
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

    #[tokio::test]
    async fn test_server_notifier_starts_empty() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        let notifier = server.notifier();
        assert_eq!(notifier.client_count().await, 0);
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
        assert!(info.capabilities.logging.is_some());
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
    fn test_with_discoverable_instructions() {
        let server =
            FabrykMcpServer::new(CompositeRegistry::new()).with_discoverable_instructions("myapp");
        let desc = server.config().description.as_deref().unwrap();
        assert!(desc.contains("myapp_directory"));
        assert!(desc.contains("ALWAYS call"));
    }

    #[test]
    fn test_discoverable_instructions_compose_with_description() {
        let server = FabrykMcpServer::new(CompositeRegistry::new())
            .with_description("My custom description.")
            .with_discoverable_instructions("myapp");
        let desc = server.config().description.as_deref().unwrap();
        assert!(
            desc.starts_with("ALWAYS call myapp_directory first"),
            "Directive should come first: {desc}"
        );
        assert!(
            desc.contains("My custom description."),
            "Original description should be preserved: {desc}"
        );
    }

    #[test]
    fn test_discoverable_instructions_without_prior_description() {
        let server =
            FabrykMcpServer::new(CompositeRegistry::new()).with_discoverable_instructions("myapp");
        let desc = server.config().description.as_deref().unwrap();
        assert!(
            !desc.contains("\n\n"),
            "Should not have double newline when no prior description"
        );
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

    #[test]
    fn test_server_with_guidance() {
        use crate::guidance::ServerGuidance;

        let guidance = ServerGuidance::for_domain("nms")
            .context("Galactic copilot")
            .subscribe("nms://player", "Live tracking");

        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_guidance(&guidance);
        let desc = server.config().description.as_deref().unwrap();
        assert!(desc.contains("ALWAYS call nms_directory first"));
        assert!(desc.contains("Galactic copilot"));
        assert!(desc.contains("nms://player"));
    }

    #[test]
    fn test_server_get_info_without_resources() {
        let server = FabrykMcpServer::new(MockRegistry).with_name("test");
        let info = server.get_info();
        assert!(info.capabilities.resources.is_none());
    }

    #[test]
    fn test_server_get_info_with_resources() {
        use crate::resource::ResourceRegistry;
        use rmcp::model::Resource;

        struct EmptyResources;
        impl ResourceRegistry for EmptyResources {
            fn resources(&self) -> Vec<Resource> {
                vec![]
            }
            fn read(&self, _uri: &str) -> Option<crate::resource::ResourceFuture> {
                None
            }
        }

        let server = FabrykMcpServer::new(MockRegistry)
            .with_name("test")
            .with_resources(EmptyResources);
        let info = server.get_info();
        assert!(info.capabilities.resources.is_some());
    }

    #[tokio::test]
    async fn test_list_resources_delegates_to_registry() {
        use crate::resource::ResourceRegistry;
        use rmcp::model::Resource;

        struct TestResources;
        impl ResourceRegistry for TestResources {
            fn resources(&self) -> Vec<Resource> {
                vec![Resource::new("test://res", "Test Resource")]
            }
            fn read(&self, _uri: &str) -> Option<crate::resource::ResourceFuture> {
                None
            }
        }

        let server = FabrykMcpServer::new(MockRegistry).with_resources(TestResources);
        let info = server.get_info();
        assert!(info.capabilities.resources.is_some());
    }

    #[test]
    fn test_list_tools_inner_returns_tools() {
        let server = FabrykMcpServer::new(MockRegistry);
        let tools = server.list_tools_inner();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name.to_string(), "test_tool");
    }

    #[test]
    fn test_list_tools_inner_empty_registry() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        let tools = server.list_tools_inner();
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_call_tool_inner_known_tool() {
        let server = FabrykMcpServer::new(MockRegistry);
        let result = server
            .call_tool_inner("test_tool", serde_json::Value::Null)
            .await
            .unwrap();
        assert!(!result.is_error.unwrap_or(false));
    }

    #[tokio::test]
    async fn test_call_tool_inner_unknown_tool() {
        let server = FabrykMcpServer::new(MockRegistry);
        let result = server
            .call_tool_inner("nonexistent", serde_json::Value::Null)
            .await
            .unwrap();
        // Unknown tools return an error result (not an Err)
        let text = &result.content[0];
        assert!(format!("{text:?}").contains("Unknown tool"));
    }

    #[test]
    fn test_list_resources_inner_no_registry() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        let result = server.list_resources_inner();
        assert!(result.resources.is_empty());
    }

    #[test]
    fn test_list_resources_inner_with_registry() {
        use crate::resource::ResourceRegistry;
        use rmcp::model::Resource;

        struct TestResources;
        impl ResourceRegistry for TestResources {
            fn resources(&self) -> Vec<Resource> {
                vec![Resource::new("test://res", "Test Resource")]
            }
            fn read(&self, _uri: &str) -> Option<crate::resource::ResourceFuture> {
                None
            }
        }

        let server = FabrykMcpServer::new(MockRegistry).with_resources(TestResources);
        let result = server.list_resources_inner();
        assert_eq!(result.resources.len(), 1);
    }

    #[tokio::test]
    async fn test_read_resource_inner_no_registry() {
        let server = FabrykMcpServer::new(CompositeRegistry::new());
        let result = server.read_resource_inner("test://doc").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_resource_inner_unknown_uri() {
        use crate::resource::ResourceRegistry;
        use rmcp::model::Resource;

        struct EmptyResources;
        impl ResourceRegistry for EmptyResources {
            fn resources(&self) -> Vec<Resource> {
                vec![]
            }
            fn read(&self, _uri: &str) -> Option<crate::resource::ResourceFuture> {
                None
            }
        }

        let server = FabrykMcpServer::new(MockRegistry).with_resources(EmptyResources);
        let result = server.read_resource_inner("test://missing").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_resource_inner_known_uri() {
        use crate::resource::ResourceRegistry;
        use rmcp::model::{Resource, ResourceContents};

        struct DocResources;
        impl ResourceRegistry for DocResources {
            fn resources(&self) -> Vec<Resource> {
                vec![Resource::new("test://doc", "Test Doc")]
            }
            fn read(&self, uri: &str) -> Option<crate::resource::ResourceFuture> {
                if uri == "test://doc" {
                    Some(Box::pin(async {
                        Ok(vec![ResourceContents::text("content", "test://doc")])
                    }))
                } else {
                    None
                }
            }
        }

        let server = FabrykMcpServer::new(MockRegistry).with_resources(DocResources);
        let result = server.read_resource_inner("test://doc").await.unwrap();
        assert_eq!(result.contents.len(), 1);
        assert!(matches!(
            &result.contents[0],
            ResourceContents::TextResourceContents { uri, text, .. }
                if uri == "test://doc" && text == "content"
        ));
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

#[cfg(all(test, feature = "http"))]
mod http_tests {
    use super::*;
    use crate::registry::{CompositeRegistry, ToolResult};
    use crate::static_resources::{StaticResourceDef, StaticResources};
    use rmcp::model::{CallToolResult, ContentBlock, Tool};
    use serde_json::{Value, json};
    use std::time::Duration;
    use tokio::task::JoinHandle;

    struct AbortOnDrop(JoinHandle<()>);

    impl Drop for AbortOnDrop {
        fn drop(&mut self) {
            self.0.abort();
        }
    }

    struct MockRegistry;

    impl ToolRegistry for MockRegistry {
        fn tools(&self) -> Vec<Tool> {
            vec![
                Tool::new("test_tool", "A test tool", crate::empty_input_schema()),
                Tool::new("test_error", "An error tool", crate::empty_input_schema()),
            ]
        }

        fn call(&self, name: &str, _args: serde_json::Value) -> Option<ToolResult> {
            match name {
                "test_tool" => Some(Box::pin(async {
                    Ok(CallToolResult::success(vec![ContentBlock::text(
                        "tool payload",
                    )]))
                })),
                "test_error" => Some(Box::pin(async {
                    Ok(CallToolResult::error(vec![ContentBlock::text(
                        "expected tool error",
                    )]))
                })),
                _ => None,
            }
        }
    }

    async fn send_legacy(
        client: &reqwest::Client,
        url: &str,
        session_id: Option<&str>,
        body: Value,
    ) -> reqwest::Response {
        let mut request = client
            .post(url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&body);
        if let Some(session_id) = session_id {
            request = request.header("Mcp-Session-Id", session_id);
        }
        tokio::time::timeout(Duration::from_secs(5), request.send())
            .await
            .expect("bounded MCP request")
            .expect("send MCP request")
    }

    async fn sse_json(response: reqwest::Response) -> Value {
        let body = tokio::time::timeout(Duration::from_secs(5), response.text())
            .await
            .expect("bounded MCP event response")
            .expect("read MCP event response");
        let data = body
            .lines()
            .filter_map(|line| line.strip_prefix("data: "))
            .find(|line| !line.is_empty())
            .expect("legacy response contains a JSON-RPC event");
        serde_json::from_str(data).expect("event is JSON-RPC")
    }

    #[tokio::test]
    async fn default_http_service_preserves_legacy_mcp_wire_behavior() {
        let missing_resource_root = tempfile::tempdir().expect("owned missing-resource directory");
        let resources =
            StaticResources::new(missing_resource_root.path()).with_resource(StaticResourceDef {
                uri: "acme://guide".into(),
                name: "Guide".into(),
                description: "Acme test guide".into(),
                mime_type: "text/markdown".into(),
                filename: "guide.md".into(),
                fallback: Some("# Acme Guide".into()),
            });
        let server = FabrykMcpServer::new(MockRegistry).with_resources(resources);
        let notifier = server.notifier();
        let service = server.into_http_service();
        let router = axum::Router::new().nest_service("/mcp", service);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind loopback listener");
        let address = listener.local_addr().expect("bound address");
        let task = AbortOnDrop(tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        }));
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("build bounded test client");
        let url = format!("http://{address}/mcp");

        let initialize = send_legacy(
            &client,
            &url,
            None,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25",
                    "capabilities": {},
                    "clientInfo": {"name": "Acme test client", "version": "1.0"}
                }
            }),
        )
        .await;
        assert_eq!(initialize.status(), reqwest::StatusCode::OK);
        let session_id = initialize
            .headers()
            .get("Mcp-Session-Id")
            .expect("legacy initialize returns session id")
            .to_str()
            .expect("session id is valid header text")
            .to_owned();
        let initialized = sse_json(initialize).await;
        assert_eq!(initialized["result"]["protocolVersion"], "2025-11-25");

        let notification = send_legacy(
            &client,
            &url,
            Some(&session_id),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        )
        .await;
        assert!(notification.status().is_success());

        let listed_tools = sse_json(
            send_legacy(
                &client,
                &url,
                Some(&session_id),
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
            )
            .await,
        )
        .await;
        let tool_names: Vec<&str> = listed_tools["result"]["tools"]
            .as_array()
            .expect("listed tools array")
            .iter()
            .map(|tool| tool["name"].as_str().expect("tool name"))
            .collect();
        assert_eq!(tool_names, ["test_tool", "test_error"]);

        let called = sse_json(
            send_legacy(
                &client,
                &url,
                Some(&session_id),
                json!({
                    "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                    "params": {"name": "test_tool", "arguments": {}}
                }),
            )
            .await,
        )
        .await;
        assert_eq!(called["result"]["content"][0]["text"], "tool payload");
        assert_eq!(called["result"]["isError"], false);

        let failed = sse_json(
            send_legacy(
                &client,
                &url,
                Some(&session_id),
                json!({
                    "jsonrpc": "2.0", "id": 4, "method": "tools/call",
                    "params": {"name": "test_error", "arguments": {}}
                }),
            )
            .await,
        )
        .await;
        assert_eq!(failed["result"]["isError"], true);
        assert_eq!(
            failed["result"]["content"][0]["text"],
            "expected tool error"
        );

        let listed_resources = sse_json(
            send_legacy(
                &client,
                &url,
                Some(&session_id),
                json!({"jsonrpc": "2.0", "id": 5, "method": "resources/list", "params": {}}),
            )
            .await,
        )
        .await;
        let resource = &listed_resources["result"]["resources"][0];
        assert_eq!(resource["uri"], "acme://guide");
        assert_eq!(resource["name"], "Guide");
        assert_eq!(resource["description"], "Acme test guide");
        assert_eq!(resource["mimeType"], "text/markdown");

        let read = sse_json(
            send_legacy(
                &client,
                &url,
                Some(&session_id),
                json!({
                    "jsonrpc": "2.0", "id": 6, "method": "resources/read",
                    "params": {"uri": "acme://guide"}
                }),
            )
            .await,
        )
        .await;
        let content = &read["result"]["contents"][0];
        assert_eq!(content["uri"], "acme://guide");
        assert_eq!(content["text"], "# Acme Guide");
        assert_eq!(content["mimeType"], "text/plain");

        // A request barrier after initialized ensures the server registered this peer.
        assert_eq!(notifier.client_count().await, 1);
        let mut log_stream = client
            .get(&url)
            .header("Accept", "text/event-stream")
            .header("Mcp-Session-Id", &session_id)
            .header("Mcp-Protocol-Version", "2025-11-25")
            .send()
            .await
            .expect("open legacy server event stream");
        assert_eq!(log_stream.status(), reqwest::StatusCode::OK);
        #[expect(
            deprecated,
            reason = "Verify the SEP-2577 legacy logging notification still reaches a connected client; no same-semantics replacement exists."
        )]
        let recipient_count = notifier
            .log(
                rmcp::model::LoggingLevel::Warning,
                "compatibility-oracle",
                json!({"event": "legacy-log", "sequence": 42}),
            )
            .await;
        assert_eq!(recipient_count, 1);
        let mut event_bytes = Vec::new();
        let event = tokio::time::timeout(Duration::from_secs(5), async {
            'events: loop {
                let chunk = log_stream
                    .chunk()
                    .await
                    .expect("read SSE chunk")
                    .expect("logging event arrives");
                event_bytes.extend_from_slice(&chunk);
                let text = String::from_utf8_lossy(&event_bytes);
                for block in text.split("\n\n") {
                    let data = block
                        .lines()
                        .filter_map(|line| {
                            line.trim_end_matches('\r')
                                .strip_prefix("data:")
                                .map(str::trim_start)
                        })
                        .filter(|line| !line.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                    if !data.is_empty()
                        && let Ok(notification) = serde_json::from_str::<Value>(&data)
                    {
                        break 'events notification;
                    }
                }
            }
        })
        .await
        .expect("bounded connected logging event");
        assert_eq!(event["jsonrpc"], "2.0");
        assert_eq!(event["method"], "notifications/message");
        assert_eq!(event["params"]["level"], "warning");
        assert_eq!(event["params"]["logger"], "compatibility-oracle");
        assert_eq!(
            event["params"]["data"],
            json!({"event": "legacy-log", "sequence": 42})
        );

        let rejected_host = client
            .post(&url)
            .header("Host", "untrusted.acme.example")
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0", "id": 7, "method": "initialize",
                "params": {
                    "protocolVersion": "2025-11-25", "capabilities": {},
                    "clientInfo": {"name": "Acme test client", "version": "1.0"}
                }
            }))
            .send()
            .await
            .expect("send disallowed Host regression request");
        assert_eq!(rejected_host.status(), reqwest::StatusCode::FORBIDDEN);

        drop(task);
    }

    #[test]
    fn test_into_http_service_creation() {
        let server = FabrykMcpServer::new(MockRegistry)
            .with_name("test-http")
            .with_version("0.1.0");
        // Construction should not panic
        let _service = server.into_http_service();
    }

    #[test]
    fn test_into_http_service_with_custom_config() {
        let server = FabrykMcpServer::new(MockRegistry).with_name("test-http");
        let config = StreamableHttpServerConfig::default();
        let _service = server.into_http_service_with_config(config);
    }

    #[test]
    fn test_into_http_service_empty_registry() {
        let server = FabrykMcpServer::new(CompositeRegistry::new()).with_name("empty");
        let _service = server.into_http_service();
    }

    #[tokio::test]
    async fn test_serve_http_health_endpoint() {
        let server = FabrykMcpServer::new(MockRegistry)
            .with_name("test-http")
            .with_version("0.1.0");

        // Bind to a random available port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener);

        let handle = tokio::spawn(async move { server.serve_http(addr).await });

        // Give the server a moment to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Hit the health endpoint — now returns JSON
        let resp = reqwest::get(format!("http://{addr}/health")).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = resp.text().await.unwrap();
        let json: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(json["status"], "ok");

        handle.abort();
    }
}
