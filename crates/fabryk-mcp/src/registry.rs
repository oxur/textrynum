//! Tool registry trait for MCP servers.
//!
//! This module defines the `ToolRegistry` trait that abstracts over
//! tool registration and dispatch. Domains implement this trait to
//! define their available MCP tools.
//!
//! The `CompositeRegistry` combines multiple registries into one,
//! enabling composition of tools from separate sources (content,
//! search, graph, etc.).

use rmcp::model::{CallToolResult, ErrorData, Tool};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Type alias for async tool handler results.
pub type ToolResult = Pin<Box<dyn Future<Output = Result<CallToolResult, ErrorData>> + Send>>;

/// Trait for registering and dispatching MCP tools.
///
/// Each domain or feature implements this to define its available tools.
/// The `FabrykMcpServer` delegates `list_tools` and `call_tool` to the
/// registry it holds.
///
/// # Example
///
/// ```rust,ignore
/// struct MyTools { /* ... */ }
///
/// impl ToolRegistry for MyTools {
///     fn tools(&self) -> Vec<Tool> {
///         vec![/* tool definitions */]
///     }
///
///     fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
///         match name {
///             "my_tool" => Some(Box::pin(self.handle_my_tool(args))),
///             _ => None,
///         }
///     }
/// }
/// ```
pub trait ToolRegistry: Send + Sync {
    /// Returns information about all available tools.
    fn tools(&self) -> Vec<Tool>;

    /// Dispatches a tool call by name.
    ///
    /// Returns `None` if the tool is not recognized by this registry.
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
/// Useful for composing tools from multiple sources (content, search,
/// graph, etc.) into a single registry for the MCP server.
///
/// # Example
///
/// ```rust,ignore
/// let registry = CompositeRegistry::new()
///     .add(content_tools)
///     .add(search_tools)
///     .add(graph_tools);
///
/// assert_eq!(registry.tool_count(), 14);
/// ```
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
    #[allow(clippy::should_implement_trait)]
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
    fn tools(&self) -> Vec<Tool> {
        self.registries.iter().flat_map(|r| r.tools()).collect()
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::Content;
    use serde_json::json;
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

    struct TestRegistry {
        tool_list: Vec<Tool>,
    }

    impl ToolRegistry for TestRegistry {
        fn tools(&self) -> Vec<Tool> {
            self.tool_list.clone()
        }

        fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
            if self.has_tool(name) {
                let name = name.to_string();
                Some(Box::pin(async move {
                    Ok(CallToolResult::success(vec![Content::text(format!(
                        "called: {name}"
                    ))]))
                }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_tool_count() {
        let registry = TestRegistry {
            tool_list: vec![make_tool("tool1", "First"), make_tool("tool2", "Second")],
        };
        assert_eq!(registry.tool_count(), 2);
    }

    #[test]
    fn test_has_tool() {
        let registry = TestRegistry {
            tool_list: vec![make_tool("exists", "A tool")],
        };
        assert!(registry.has_tool("exists"));
        assert!(!registry.has_tool("missing"));
    }

    #[tokio::test]
    async fn test_call_known_tool() {
        let registry = TestRegistry {
            tool_list: vec![make_tool("greet", "Say hello")],
        };

        let future = registry.call("greet", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_call_unknown_tool() {
        let registry = TestRegistry {
            tool_list: vec![make_tool("greet", "Say hello")],
        };
        assert!(registry.call("missing", json!({})).is_none());
    }

    #[test]
    fn test_composite_registry_empty() {
        let composite = CompositeRegistry::new();
        assert_eq!(composite.tool_count(), 0);
        assert!(!composite.has_tool("anything"));
    }

    #[test]
    fn test_composite_registry_combines_tools() {
        let reg1 = TestRegistry {
            tool_list: vec![make_tool("tool1", "From reg1")],
        };
        let reg2 = TestRegistry {
            tool_list: vec![make_tool("tool2", "From reg2")],
        };

        let composite = CompositeRegistry::new().add(reg1).add(reg2);

        assert_eq!(composite.tool_count(), 2);
        assert!(composite.has_tool("tool1"));
        assert!(composite.has_tool("tool2"));
        assert!(!composite.has_tool("tool3"));
    }

    #[tokio::test]
    async fn test_composite_registry_dispatches() {
        let reg1 = TestRegistry {
            tool_list: vec![make_tool("search", "Search")],
        };
        let reg2 = TestRegistry {
            tool_list: vec![make_tool("graph", "Graph")],
        };

        let composite = CompositeRegistry::new().add(reg1).add(reg2);

        // Should dispatch to reg1
        assert!(composite.call("search", json!({})).is_some());
        // Should dispatch to reg2
        assert!(composite.call("graph", json!({})).is_some());
        // Should return None
        assert!(composite.call("missing", json!({})).is_none());
    }

    #[test]
    fn test_composite_registry_default() {
        let composite = CompositeRegistry::default();
        assert_eq!(composite.tool_count(), 0);
    }

    #[test]
    fn test_composite_registry_multiple_add() {
        let composite = CompositeRegistry::new()
            .add(TestRegistry {
                tool_list: vec![make_tool("a", "A")],
            })
            .add(TestRegistry {
                tool_list: vec![make_tool("b", "B")],
            })
            .add(TestRegistry {
                tool_list: vec![make_tool("c", "C")],
            });

        assert_eq!(composite.tool_count(), 3);
    }

    #[test]
    fn test_trait_object_safety() {
        fn _assert_object_safe(_: &dyn ToolRegistry) {}
    }
}
