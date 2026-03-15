//! Core MCP server infrastructure for Fabryk.
//!
//! This crate provides the foundational MCP server components that enable
//! Fabryk-based applications to expose tools via the Model Context Protocol.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      fabryk-mcp                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ToolRegistry trait — tool registration and dispatch        │
//! │  CompositeRegistry — combine multiple tool sources          │
//! ├─────────────────────────────────────────────────────────────┤
//! │  FabrykMcpServer — generic server (implements ServerHandler)│
//! │  ServerConfig — server metadata (name, version, description)│
//! ├─────────────────────────────────────────────────────────────┤
//! │  McpErrorExt — fabryk_core::Error → rmcp::ErrorData         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Built-in tools:                                            │
//! │  ├── health — server status and tool count                  │
//! │  └── diagnostics — config inspection and service status     │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::{FabrykMcpServer, CompositeRegistry};
//!
//! // Create domain-specific tools
//! let content_tools = MyContentTools::new(&config);
//! let search_tools = MySearchTools::new(&state);
//!
//! // Combine into composite registry
//! let registry = CompositeRegistry::new()
//!     .add(content_tools)
//!     .add(search_tools);
//!
//! // Create and run server
//! FabrykMcpServer::new(registry)
//!     .with_name("my-domain")
//!     .serve_stdio()
//!     .await?;
//! ```

pub mod discoverable;
pub mod error;
pub mod guidance;
#[cfg(feature = "http")]
pub mod health_router;
pub mod notifier;
pub mod registry;
pub mod resource;
pub mod server;
pub mod service_registry;
pub mod tools;
pub mod validate;

// Re-exports — registry
pub use registry::{CompositeRegistry, ToolRegistry, ToolResult};

// Re-exports — service registry
pub use service_registry::ServiceAwareRegistry;

// Re-exports — server
pub use server::{FabrykMcpServer, ServerConfig};

// Re-exports — notifier
pub use notifier::Notifier;

// Re-exports — error
pub use error::McpErrorExt;

// Re-exports — guidance
pub use guidance::ServerGuidance;

// Re-exports — discoverable registry
pub use discoverable::{DiscoverableRegistry, ExternalConnector, ToolMeta};

// Re-exports — resource registry
pub use resource::{ResourceFuture, ResourceRegistry};

// Re-exports — built-in tools
pub use tools::{DiagnosticTools, HealthResponse, HealthTools, handle_health};

// Re-exports — validation
pub use validate::{assert_tools_valid, validate_tools, warn_on_invalid_tools};

// Re-exports — rmcp types used by downstream crates
pub mod model {
    //! Re-exported rmcp model types.
    pub use rmcp::model::{
        AnnotateAble, Annotated, CallToolResult, Content, ErrorCode, ErrorData, LoggingLevel,
        RawResource, Resource, ResourceContents, Tool,
    };
}

/// Return a minimal valid JSON Schema for a tool that takes no parameters.
///
/// MCP requires `inputSchema` to have at least `{"type": "object"}`.
/// An empty map `{}` is rejected by some clients (e.g. Claude Desktop).
pub fn empty_input_schema() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "type".to_string(),
        serde_json::Value::String("object".to_string()),
    );
    m
}

// Re-exports — service lifecycle (from fabryk-core)
pub use fabryk_core::service::{ServiceHandle, ServiceState};

// Re-exports — HTTP health router (requires `http` feature)
#[cfg(feature = "http")]
pub use health_router::{ServiceHealthResponse, ServiceStatus, health_router};

// Re-exports — HTTP transport (requires `http` feature)
#[cfg(feature = "http")]
pub use rmcp::transport::streamable_http_server::{
    StreamableHttpServerConfig, StreamableHttpService, session::local::LocalSessionManager,
};

// Re-exports — axum types for custom HTTP composition (requires `http` feature)
#[cfg(feature = "http")]
pub use axum;
