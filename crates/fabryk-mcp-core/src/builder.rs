//! Server builder for Fabryk MCP applications.
//!
//! Provides [`ServerBuilder`] for composing a [`FabrykMcpServer`] from
//! a [`CompositeRegistry`] and optional resources. Handles the common
//! pattern of assembling tool registries, health diagnostics, and
//! static resources into a server ready for stdio or HTTP transport.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::ServerBuilder;
//!
//! let server = ServerBuilder::new()
//!     .name("my-skill")
//!     .version("0.1.0")
//!     .description("My domain skill server")
//!     .add(content_tools)
//!     .add(source_tools)
//!     .add(search_tools)
//!     .add(health_tools)
//!     .add(my_domain_tools)
//!     .with_resource(conventions_resource)
//!     .with_resource(scope_resource)
//!     .build();
//! ```

use crate::registry::{CompositeRegistry, ToolRegistry};
use crate::server::FabrykMcpServer;
use crate::static_resources::{StaticResourceDef, StaticResources};
use std::path::PathBuf;

/// Fluent builder for assembling a [`FabrykMcpServer`].
///
/// Collects tool registries, server metadata, and optional static resources,
/// then produces a fully configured server ready to run.
pub struct ServerBuilder {
    name: String,
    version: String,
    description: Option<String>,
    registry: CompositeRegistry,
    resources_path: PathBuf,
    resource_defs: Vec<StaticResourceDef>,
}

impl ServerBuilder {
    /// Create a new builder with empty defaults.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            version: String::new(),
            description: None,
            registry: CompositeRegistry::new(),
            resources_path: PathBuf::from("."),
            resource_defs: Vec::new(),
        }
    }

    /// Set the server name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Set the server version.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Set the server description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the base path for static resource files.
    pub fn resources_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.resources_path = path.into();
        self
    }

    /// Add a tool registry to the composite.
    ///
    /// Tool registries are added in order and queried in order for tool
    /// dispatch. Add domain-specific registries after framework registries.
    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, registry: impl ToolRegistry + 'static) -> Self {
        self.registry = self.registry.add(registry);
        self
    }

    /// Add a static resource definition.
    ///
    /// Resources are exposed via the MCP protocol's resource listing.
    /// Each resource has a URI, name, description, MIME type, filename
    /// (relative to `resources_path`), and optional fallback content.
    pub fn with_resource(mut self, def: StaticResourceDef) -> Self {
        self.resource_defs.push(def);
        self
    }

    /// Consume the builder and return the composed registry for further
    /// wrapping (e.g., with [`DiscoverableRegistry`]).
    ///
    /// Use this when you need to wrap the registry before creating the server.
    /// Call [`Self::build_with_registry`] afterward to finish construction.
    pub fn into_parts(self) -> (CompositeRegistry, ServerBuilderParts) {
        (
            self.registry,
            ServerBuilderParts {
                name: self.name,
                version: self.version,
                description: self.description,
                resources_path: self.resources_path,
                resource_defs: self.resource_defs,
            },
        )
    }

    /// Build the server from a pre-wrapped registry and the remaining builder parts.
    pub fn build_with_registry(
        registry: impl ToolRegistry + 'static,
        parts: ServerBuilderParts,
    ) -> FabrykMcpServer {
        let mut server = FabrykMcpServer::new(registry)
            .with_name(&parts.name)
            .with_version(&parts.version);

        if let Some(desc) = &parts.description {
            server = server.with_description(desc);
        }

        if !parts.resource_defs.is_empty() {
            let mut resources = StaticResources::new(parts.resources_path);
            for def in parts.resource_defs {
                resources = resources.with_resource(def);
            }
            server = server.with_resources(resources);
        }

        server
    }

    /// Build the final [`FabrykMcpServer`].
    ///
    /// Assembles the composite registry, static resources, and server
    /// metadata into a server ready for transport.
    pub fn build(self) -> FabrykMcpServer {
        let mut server = FabrykMcpServer::new(self.registry)
            .with_name(&self.name)
            .with_version(&self.version);

        if let Some(desc) = &self.description {
            server = server.with_description(desc);
        }

        if !self.resource_defs.is_empty() {
            let mut resources = StaticResources::new(self.resources_path);
            for def in self.resource_defs {
                resources = resources.with_resource(def);
            }
            server = server.with_resources(resources);
        }

        server
    }
}

/// Remaining builder state after extracting the registry via [`ServerBuilder::into_parts`].
pub struct ServerBuilderParts {
    /// Server name.
    pub name: String,
    /// Server version.
    pub version: String,
    /// Optional description / instructions.
    pub description: Option<String>,
    /// Base path for static resource files.
    pub resources_path: PathBuf,
    /// Static resource definitions.
    pub resource_defs: Vec<StaticResourceDef>,
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_minimal() {
        let server = ServerBuilder::new()
            .name("test-server")
            .version("0.1.0")
            .build();

        assert_eq!(server.config().name, "test-server");
        assert_eq!(server.config().version, "0.1.0");
    }

    #[test]
    fn test_builder_with_description() {
        let server = ServerBuilder::new()
            .name("test")
            .version("1.0")
            .description("A test server")
            .build();

        assert_eq!(
            server.config().description.as_deref(),
            Some("A test server")
        );
    }

    #[test]
    fn test_builder_default() {
        let builder = ServerBuilder::default();
        assert!(builder.name.is_empty());
        assert!(builder.resource_defs.is_empty());
    }

    #[test]
    fn test_builder_with_resource() {
        let server = ServerBuilder::new()
            .name("test")
            .version("1.0")
            .resources_path("/tmp")
            .with_resource(StaticResourceDef {
                uri: "skill://test".into(),
                name: "Test Resource".into(),
                description: "A test".into(),
                mime_type: "text/plain".into(),
                filename: "test.txt".into(),
                fallback: Some("fallback content".into()),
            })
            .build();

        assert_eq!(server.config().name, "test");
    }
}
