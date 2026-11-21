---
title: "CC Prompt: Fabryk 5.1 — MCP Core Infrastructure"
milestone: "5.1"
phase: 5
author: "Claude (Opus 4.5)"
created: 2026-02-06
updated: 2026-02-06
prerequisites: ["Phase 4 complete"]
governing-docs: [0011-audit §4.6, 0013-project-plan]
---

# CC Prompt: Fabryk 5.1 — MCP Core Infrastructure

## Context

Phase 5 extracts the MCP (Model Context Protocol) server infrastructure into
reusable crates. This phase creates four crates:

- `fabryk-mcp` - Core MCP server and tool registry
- `fabryk-mcp-content` - Content listing/retrieval tools
- `fabryk-mcp-fts` - Full-text search tools
- `fabryk-mcp-graph` - Graph query tools

This milestone focuses on the core infrastructure in `fabryk-mcp`.

## Objective

Create `fabryk-mcp` crate with:

1. `FabrykMcpServer<C: ConfigProvider>` - Generic MCP server
2. `ToolRegistry` trait - Tool registration abstraction
3. Health check tool (extracted from music-theory)
4. Server lifecycle management

## Implementation Steps

### Step 1: Create fabryk-mcp crate scaffold

```bash
cd ~/lab/oxur/ecl/crates
mkdir -p fabryk-mcp/src
```

Create `fabryk-mcp/Cargo.toml`:

```toml
[package]
name = "fabryk-mcp"
version = "0.1.0"
edition = "2021"
description = "MCP server infrastructure for Fabryk domains"
license = "Apache-2.0"
repository = "https://github.com/oxur/ecl"

[dependencies]
fabryk-core = { path = "../fabryk-core" }

# MCP protocol
mcp-server = "0.2"  # Or current version
mcp-core = "0.2"

# Async runtime
tokio = { version = "1.0", features = ["full"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Logging
tracing = "0.1"

[dev-dependencies]
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }
```

### Step 2: Define ToolRegistry trait

Create `fabryk-mcp/src/registry.rs`:

```rust
//! Tool registry trait for MCP servers.
//!
//! Domains implement this trait to register their MCP tools.

use fabryk_core::Result;
use mcp_core::{Tool, ToolInfo};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Type alias for async tool handler results.
pub type ToolResult = Pin<Box<dyn Future<Output = Result<Value>> + Send>>;

/// Trait for registering and dispatching MCP tools.
///
/// Each domain implements this to define its available tools.
///
/// # Example
///
/// ```rust,ignore
/// struct MusicTheoryTools { /* ... */ }
///
/// impl ToolRegistry for MusicTheoryTools {
///     fn tools(&self) -> Vec<ToolInfo> {
///         vec![
///             ToolInfo { name: "search", description: "Search concepts", ... },
///             ToolInfo { name: "graph_related", description: "Find related concepts", ... },
///         ]
///     }
///
///     fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
///         match name {
///             "search" => Some(Box::pin(self.handle_search(args))),
///             "graph_related" => Some(Box::pin(self.handle_graph_related(args))),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait ToolRegistry: Send + Sync {
    /// Returns information about all available tools.
    fn tools(&self) -> Vec<ToolInfo>;

    /// Dispatches a tool call by name.
    ///
    /// Returns `None` if the tool is not recognized.
    fn call(&self, name: &str, args: Value) -> Option<ToolResult>;

    /// Returns the number of registered tools.
    fn tool_count(&self) -> usize {
        self.tools().len()
    }

    /// Check if a tool exists by name.
    fn has_tool(&self, name: &str) -> bool {
        self.tools().iter().any(|t| t.name == name)
    }
}

/// A registry that combines multiple sub-registries.
///
/// Useful for composing tools from multiple sources (content, search, graph).
pub struct CompositeRegistry {
    registries: Vec<Box<dyn ToolRegistry>>,
}

impl CompositeRegistry {
    /// Create a new empty composite registry.
    pub fn new() -> Self {
        Self {
            registries: Vec::new(),
        }
    }

    /// Add a sub-registry.
    pub fn add<R: ToolRegistry + 'static>(mut self, registry: R) -> Self {
        self.registries.push(Box::new(registry));
        self
    }
}

impl Default for CompositeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry for CompositeRegistry {
    fn tools(&self) -> Vec<ToolInfo> {
        self.registries
            .iter()
            .flat_map(|r| r.tools())
            .collect()
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        for registry in &self.registries {
            if let Some(result) = registry.call(name, args.clone()) {
                return Some(result);
            }
        }
        None
    }
}
```

### Step 3: Create MCP server wrapper

Create `fabryk-mcp/src/server.rs`:

```rust
//! Generic MCP server for Fabryk domains.

use crate::registry::ToolRegistry;
use fabryk_core::traits::ConfigProvider;
use fabryk_core::Result;
use std::sync::Arc;
use tracing::{info, error};

/// Configuration for the MCP server.
#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Server name shown to clients.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Optional server description.
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
/// Parameterized over:
/// - `C: ConfigProvider` - Domain configuration
/// - `R: ToolRegistry` - Tool registration
///
/// # Example
///
/// ```rust,ignore
/// let config = MusicTheoryConfig::load()?;
/// let tools = MusicTheoryTools::new(&config);
///
/// let server = FabrykMcpServer::new(config, tools)
///     .with_name("music-theory")
///     .with_description("Music theory knowledge assistant");
///
/// server.run().await?;
/// ```
pub struct FabrykMcpServer<C, R>
where
    C: ConfigProvider,
    R: ToolRegistry,
{
    config: Arc<C>,
    registry: Arc<R>,
    server_config: ServerConfig,
}

impl<C, R> FabrykMcpServer<C, R>
where
    C: ConfigProvider + 'static,
    R: ToolRegistry + 'static,
{
    /// Create a new MCP server.
    pub fn new(config: C, registry: R) -> Self {
        Self {
            config: Arc::new(config),
            registry: Arc::new(registry),
            server_config: ServerConfig::default(),
        }
    }

    /// Set the server name.
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.server_config.name = name.into();
        self
    }

    /// Set the server version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.server_config.version = version.into();
        self
    }

    /// Set the server description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.server_config.description = Some(description.into());
        self
    }

    /// Get the domain configuration.
    pub fn config(&self) -> &C {
        &self.config
    }

    /// Get the tool registry.
    pub fn registry(&self) -> &R {
        &self.registry
    }

    /// Run the MCP server.
    ///
    /// This starts the server and blocks until shutdown.
    pub async fn run(self) -> Result<()> {
        info!(
            "Starting {} v{} with {} tools",
            self.server_config.name,
            self.server_config.version,
            self.registry.tool_count()
        );

        // TODO: Implement actual MCP server integration
        // This will depend on the specific MCP library being used
        // For now, this is a placeholder that shows the structure

        info!("Server running. Press Ctrl+C to stop.");

        // Wait for shutdown signal
        tokio::signal::ctrl_c().await.map_err(|e| {
            fabryk_core::Error::operation(format!("Failed to listen for shutdown: {}", e))
        })?;

        info!("Shutting down...");
        Ok(())
    }
}
```

### Step 4: Create health tool

Create `fabryk-mcp/src/tools/health.rs`:

```rust
//! Health check tool for MCP servers.

use fabryk_core::Result;
use mcp_core::ToolInfo;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Health check response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status.
    pub status: String,
    /// Server name.
    pub server_name: String,
    /// Server version.
    pub version: String,
    /// Number of registered tools.
    pub tool_count: usize,
    /// Optional additional info.
    pub info: Option<Value>,
}

/// Get the health tool definition.
pub fn health_tool_info() -> ToolInfo {
    ToolInfo {
        name: "health".to_string(),
        description: "Check server health and status".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

/// Handle the health check tool call.
pub async fn handle_health(
    server_name: &str,
    version: &str,
    tool_count: usize,
    additional_info: Option<Value>,
) -> Result<Value> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        server_name: server_name.to_string(),
        version: version.to_string(),
        tool_count,
        info: additional_info,
    };

    Ok(serde_json::to_value(response)?)
}
```

### Step 5: Create tools module

Create `fabryk-mcp/src/tools/mod.rs`:

```rust
//! Built-in MCP tools.

pub mod health;

pub use health::{handle_health, health_tool_info, HealthResponse};
```

### Step 6: Create lib.rs

Create `fabryk-mcp/src/lib.rs`:

```rust
//! MCP server infrastructure for Fabryk domains.
//!
//! This crate provides the core MCP server components that enable
//! Fabryk-based applications to expose tools via the Model Context Protocol.
//!
//! # Architecture
//!
//! - `FabrykMcpServer<C, R>` - Generic server parameterized over config and registry
//! - `ToolRegistry` trait - Tool registration and dispatch
//! - `CompositeRegistry` - Combine multiple tool sources
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::{FabrykMcpServer, ToolRegistry, CompositeRegistry};
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
//! let server = FabrykMcpServer::new(config, registry)
//!     .with_name("my-domain")
//!     .run()
//!     .await?;
//! ```

pub mod registry;
pub mod server;
pub mod tools;

pub use registry::{CompositeRegistry, ToolRegistry, ToolResult};
pub use server::{FabrykMcpServer, ServerConfig};
pub use tools::{handle_health, health_tool_info, HealthResponse};
```

### Step 7: Add tests

Create `fabryk-mcp/src/registry.rs` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct TestRegistry {
        tools: Vec<ToolInfo>,
    }

    impl ToolRegistry for TestRegistry {
        fn tools(&self) -> Vec<ToolInfo> {
            self.tools.clone()
        }

        fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
            if self.has_tool(name) {
                Some(Box::pin(async { Ok(json!({"called": name})) }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_tool_count() {
        let registry = TestRegistry {
            tools: vec![
                ToolInfo {
                    name: "tool1".to_string(),
                    description: "Test".to_string(),
                    input_schema: json!({}),
                },
                ToolInfo {
                    name: "tool2".to_string(),
                    description: "Test".to_string(),
                    input_schema: json!({}),
                },
            ],
        };

        assert_eq!(registry.tool_count(), 2);
    }

    #[test]
    fn test_has_tool() {
        let registry = TestRegistry {
            tools: vec![ToolInfo {
                name: "exists".to_string(),
                description: "Test".to_string(),
                input_schema: json!({}),
            }],
        };

        assert!(registry.has_tool("exists"));
        assert!(!registry.has_tool("missing"));
    }

    #[test]
    fn test_composite_registry() {
        let reg1 = TestRegistry {
            tools: vec![ToolInfo {
                name: "tool1".to_string(),
                description: "From reg1".to_string(),
                input_schema: json!({}),
            }],
        };

        let reg2 = TestRegistry {
            tools: vec![ToolInfo {
                name: "tool2".to_string(),
                description: "From reg2".to_string(),
                input_schema: json!({}),
            }],
        };

        let composite = CompositeRegistry::new().add(reg1).add(reg2);

        assert_eq!(composite.tool_count(), 2);
        assert!(composite.has_tool("tool1"));
        assert!(composite.has_tool("tool2"));
    }
}
```

### Step 8: Verify compilation

```bash
cd ~/lab/oxur/ecl
cargo check -p fabryk-mcp
cargo test -p fabryk-mcp
cargo clippy -p fabryk-mcp -- -D warnings
```

## Exit Criteria

- [ ] `fabryk-mcp` crate created
- [ ] `ToolRegistry` trait defined with `tools()` and `call()` methods
- [ ] `CompositeRegistry` for combining multiple registries
- [ ] `FabrykMcpServer<C, R>` generic server struct
- [ ] `ServerConfig` with name, version, description
- [ ] Health check tool extracted
- [ ] All tests pass
- [ ] Clippy clean

## Commit Message

```
feat(mcp): add fabryk-mcp crate with core infrastructure

Add MCP server infrastructure for Fabryk domains:
- ToolRegistry trait for tool registration and dispatch
- CompositeRegistry for combining multiple tool sources
- FabrykMcpServer<C, R> generic server parameterized over
  ConfigProvider and ToolRegistry
- ServerConfig for server metadata
- Health check tool

Phase 5 milestone 5.1 of Fabryk extraction.

Ref: Doc 0011 §4.6 (MCP infrastructure)
Ref: Doc 0013 Phase 5

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```
