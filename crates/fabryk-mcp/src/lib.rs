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

pub mod error;
pub mod registry;
pub mod server;
pub mod tools;

// Re-exports — registry
pub use registry::{CompositeRegistry, ToolRegistry, ToolResult};

// Re-exports — server
pub use server::{FabrykMcpServer, ServerConfig};

// Re-exports — error
pub use error::McpErrorExt;

// Re-exports — built-in tools
pub use tools::{handle_health, HealthResponse, HealthTools};
