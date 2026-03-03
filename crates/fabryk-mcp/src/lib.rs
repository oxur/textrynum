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
//! │  └── health — server status and tool count                  │
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
pub mod registry;
pub mod server;
pub mod service_registry;
pub mod tools;

// Re-exports — registry
pub use registry::{CompositeRegistry, ToolRegistry, ToolResult};

// Re-exports — service registry
pub use service_registry::ServiceAwareRegistry;

// Re-exports — server
pub use server::{FabrykMcpServer, ServerConfig};

// Re-exports — error
pub use error::McpErrorExt;

// Re-exports — discoverable registry
pub use discoverable::{DiscoverableRegistry, ExternalConnector, ToolMeta};

// Re-exports — built-in tools
pub use tools::{handle_health, HealthResponse, HealthTools};

// Re-exports — rmcp types used by downstream crates
pub mod model {
    //! Re-exported rmcp model types.
    pub use rmcp::model::{CallToolResult, Content, ErrorData, Tool};
}

// Re-exports — HTTP transport (requires `http` feature)
#[cfg(feature = "http")]
pub use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
