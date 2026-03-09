//! Discoverable tool registry for MCP servers.
//!
//! Wraps a [`ToolRegistry`] to enrich tool descriptions with structured
//! metadata and auto-inject a `{server_name}_directory` meta-tool.
//!
//! # How it works
//!
//! Developers provide [`ToolMeta`] for each tool — structured fields like
//! `when_to_use`, `returns`, and `next`. The registry auto-generates prose
//! descriptions with `WHEN TO USE / RETURNS / NEXT` sections, and injects
//! a directory tool that returns a JSON manifest of all capabilities.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::{CompositeRegistry, DiscoverableRegistry, ToolMeta};
//!
//! let registry = CompositeRegistry::new()
//!     .add(entity_tools)
//!     .add(search_tools);
//!
//! let discoverable = DiscoverableRegistry::new(registry, "myapp")
//!     .with_tool_meta("search", ToolMeta {
//!         summary: "Full-text search across all entities.".into(),
//!         when_to_use: "Looking for an entity by name or keyword".into(),
//!         returns: "Ranked list of matching entities with snippets".into(),
//!         next: Some("Call get_entity for full details".into()),
//!         category: Some("search".into()),
//!     });
//!
//! FabrykMcpServer::new(discoverable)
//!     .with_name("myapp")
//!     .with_discoverable_instructions("myapp")
//!     .serve_stdio()
//!     .await?;
//! ```

use crate::registry::{ToolRegistry, ToolResult};
use rmcp::model::{CallToolResult, Content, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
/// Structured metadata for a single tool, used to auto-generate rich descriptions.
///
/// When attached to a tool via [`DiscoverableRegistry::with_tool_meta`], the fields
/// are rendered into the tool's description as structured prose that AI agents can
/// parse to understand when and how to use the tool.
#[derive(Clone, Debug, Default)]
pub struct ToolMeta {
    /// Brief human-readable summary (1 sentence).
    pub summary: String,
    /// When an AI agent should call this tool.
    pub when_to_use: String,
    /// What the tool returns.
    pub returns: String,
    /// Suggested next tool(s) to call after this one.
    pub next: Option<String>,
    /// Optional category for grouping in directory output (e.g., "entity", "search", "meta").
    pub category: Option<String>,
}

/// An external service or connector the server can reach (not a Fabryk tool).
///
/// Listed in the directory tool output so AI agents know about capabilities
/// beyond the registered MCP tools.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExternalConnector {
    /// Display name of the connector (e.g., "Slack MCP").
    pub name: String,
    /// When an AI agent should use this connector.
    pub when_to_use: String,
    /// What the connector provides.
    pub description: String,
}

/// A registry wrapper that enriches tool descriptions with structured metadata
/// and auto-injects a `{server_name}_directory` meta-tool.
///
/// Follows the same wrapper pattern as [`ServiceAwareRegistry`](crate::ServiceAwareRegistry):
/// - `tools()` returns the inner registry's tools (with enriched descriptions) plus the
///   auto-generated directory tool.
/// - `call()` handles the directory tool internally and delegates everything else to the
///   inner registry.
pub struct DiscoverableRegistry<R: ToolRegistry> {
    inner: R,
    meta_map: HashMap<String, ToolMeta>,
    server_name: String,
    external_connectors: Vec<ExternalConnector>,
    query_strategy: Vec<String>,
    data_freshness: HashMap<String, String>,
}

impl<R: ToolRegistry> DiscoverableRegistry<R> {
    /// Wrap an existing registry with discoverability.
    pub fn new(registry: R, server_name: impl Into<String>) -> Self {
        Self {
            inner: registry,
            meta_map: HashMap::new(),
            server_name: server_name.into(),
            external_connectors: Vec::new(),
            query_strategy: Vec::new(),
            data_freshness: HashMap::new(),
        }
    }

    /// Register metadata for a tool by name.
    pub fn with_tool_meta(mut self, tool_name: impl Into<String>, meta: ToolMeta) -> Self {
        self.meta_map.insert(tool_name.into(), meta);
        self
    }

    /// Register multiple tool metadata entries at once.
    pub fn with_tool_metas<S: Into<String>>(mut self, metas: Vec<(S, ToolMeta)>) -> Self {
        for (name, meta) in metas {
            self.meta_map.insert(name.into(), meta);
        }
        self
    }

    /// Add an external connector description.
    pub fn with_connector(mut self, connector: ExternalConnector) -> Self {
        self.external_connectors.push(connector);
        self
    }

    /// Set the recommended query strategy (ordered steps).
    pub fn with_query_strategy<S: Into<String>>(mut self, steps: Vec<S>) -> Self {
        self.query_strategy = steps.into_iter().map(Into::into).collect();
        self
    }

    /// Set data freshness information per source.
    pub fn with_data_freshness(mut self, freshness: HashMap<String, String>) -> Self {
        self.data_freshness = freshness;
        self
    }

    /// The name of the auto-injected directory tool.
    pub fn directory_tool_name(&self) -> String {
        format!("{}_directory", self.server_name)
    }
}

// Private implementation details.
impl<R: ToolRegistry> DiscoverableRegistry<R> {
    /// Generate an enriched description from structured metadata.
    fn enrich_description(original: &str, meta: &ToolMeta) -> String {
        let mut parts = Vec::new();

        // Use meta.summary as the lead, or fall back to original
        if !meta.summary.is_empty() {
            parts.push(meta.summary.clone());
        } else if !original.is_empty() {
            parts.push(original.to_string());
        }

        parts.push(format!("WHEN TO USE: {}", meta.when_to_use));
        parts.push(format!("RETURNS: {}", meta.returns));

        if let Some(next) = &meta.next {
            parts.push(format!("NEXT: {}", next));
        }

        parts.join("\n")
    }

    /// Build the Tool definition for the auto-injected directory tool.
    fn directory_tool_def(&self) -> Tool {
        let dir_name = self.directory_tool_name();
        let tool_count = self.inner.tools().len();
        Tool::new(
            dir_name,
            format!(
                "Describes all {count} {name} tools, connected external services, \
                 data freshness, and the optimal query strategy.\n\
                 WHEN TO USE: Call this at the start of every session to understand \
                 what capabilities are available before doing any work.\n\
                 RETURNS: Tool list with when-to-use guidance, external connector list, \
                 optimal query strategy, and data freshness information.",
                count = tool_count,
                name = self.server_name,
            ),
            crate::empty_input_schema(),
        )
    }

    /// Handle a call to the directory tool, returning the JSON manifest.
    fn handle_directory(&self) -> ToolResult {
        let server_name = self.server_name.clone();
        let tools_key = format!("{}_tools", server_name);

        // Build tool entries from inner registry + metadata
        let tool_entries: Vec<Value> = self
            .inner
            .tools()
            .iter()
            .map(|tool| {
                let name = tool.name.to_string();
                let meta = self.meta_map.get(&name);
                serde_json::json!({
                    "name": name,
                    "category": meta.and_then(|m| m.category.as_deref()).unwrap_or("general"),
                    "use_when": meta.map(|m| m.when_to_use.as_str()).unwrap_or(""),
                    "what_it_does": meta.map(|m| m.summary.as_str()).unwrap_or(
                        tool.description.as_deref().unwrap_or("")
                    ),
                })
            })
            .collect();

        // Build optional sections
        let connectors: Vec<Value> = self
            .external_connectors
            .iter()
            .map(|c| {
                serde_json::json!({
                    "name": c.name,
                    "when_to_use": c.when_to_use,
                    "description": c.description,
                })
            })
            .collect();

        let strategy = &self.query_strategy;
        let freshness = &self.data_freshness;

        // Build category summary from tool metadata
        let mut category_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for tool in self.inner.tools().iter() {
            let name = tool.name.to_string();
            let cat = self
                .meta_map
                .get(&name)
                .and_then(|m| m.category.as_deref())
                .unwrap_or("general")
                .to_string();
            *category_counts.entry(cat).or_insert(0) += 1;
        }

        // Build the response, omitting empty sections
        let mut response = serde_json::Map::new();
        response.insert(tools_key, Value::Array(tool_entries));

        if !category_counts.is_empty() {
            response.insert(
                "categories".into(),
                Value::Object(
                    category_counts
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::Number((*v).into())))
                        .collect(),
                ),
            );
        }

        if !connectors.is_empty() {
            response.insert("external_connectors".into(), Value::Array(connectors));
        }
        if !strategy.is_empty() {
            response.insert(
                "optimal_query_strategy".into(),
                Value::Array(strategy.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if !freshness.is_empty() {
            response.insert(
                "data_freshness".into(),
                Value::Object(
                    freshness
                        .iter()
                        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                        .collect(),
                ),
            );
        }

        let json = Value::Object(response);

        Box::pin(async move {
            let text = serde_json::to_string_pretty(&json).unwrap_or_else(|_| json.to_string());
            Ok(CallToolResult::success(vec![Content::text(text)]))
        })
    }
}

impl<R: ToolRegistry + Send + Sync> ToolRegistry for DiscoverableRegistry<R> {
    fn tools(&self) -> Vec<Tool> {
        let mut tools = self.inner.tools();

        // Enrich descriptions for tools that have metadata
        for tool in &mut tools {
            if let Some(meta) = self.meta_map.get(tool.name.as_ref()) {
                tool.description = Some(
                    Self::enrich_description(tool.description.as_deref().unwrap_or(""), meta)
                        .into(),
                );
            }
        }

        // Auto-inject the directory tool
        tools.push(self.directory_tool_def());
        tools
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        // Handle directory tool call
        if name == self.directory_tool_name() {
            return Some(self.handle_directory());
        }

        // Delegate everything else to inner
        self.inner.call(name, args)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CompositeRegistry;
    use rmcp::model::Content;
    use serde_json::json;

    fn make_tool(name: &str, description: &str) -> Tool {
        Tool::new(
            name.to_string(),
            description.to_string(),
            crate::empty_input_schema(),
        )
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

    fn extract_text(result: &CallToolResult) -> String {
        match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        }
    }

    // ── description enrichment ───────────────────────────────────────────────

    #[test]
    fn test_discoverable_enriches_descriptions() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Original description")],
        };

        let registry = DiscoverableRegistry::new(inner, "test").with_tool_meta(
            "search",
            ToolMeta {
                summary: "Full-text search across entities.".into(),
                when_to_use: "Looking for an entity by name".into(),
                returns: "Ranked list of matches".into(),
                next: Some("Call get_entity for details".into()),
                category: Some("search".into()),
            },
        );

        let tools = registry.tools();
        let search_tool = tools.iter().find(|t| t.name == "search").unwrap();
        let desc = search_tool.description.as_deref().unwrap();

        assert!(desc.contains("WHEN TO USE:"), "Missing WHEN TO USE");
        assert!(desc.contains("RETURNS:"), "Missing RETURNS");
        assert!(desc.contains("NEXT:"), "Missing NEXT");
        assert!(
            desc.contains("Full-text search across entities."),
            "Missing summary"
        );
    }

    #[test]
    fn test_discoverable_preserves_original_description_without_meta() {
        let inner = MockRegistry {
            tools: vec![make_tool("other_tool", "Keep this description")],
        };

        let registry = DiscoverableRegistry::new(inner, "test");
        let tools = registry.tools();
        let tool = tools.iter().find(|t| t.name == "other_tool").unwrap();

        assert_eq!(
            tool.description.as_deref().unwrap(),
            "Keep this description"
        );
    }

    // ── directory tool injection ─────────────────────────────────────────────

    #[test]
    fn test_discoverable_injects_directory_tool() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp");
        let tools = registry.tools();

        // Inner has 1 tool, directory adds 1 more
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().any(|t| t.name == "myapp_directory"));
    }

    #[test]
    fn test_directory_tool_description_includes_tool_count() {
        let inner = MockRegistry {
            tools: vec![make_tool("a", "A"), make_tool("b", "B")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp");
        let tools = registry.tools();
        let dir_tool = tools.iter().find(|t| t.name == "myapp_directory").unwrap();
        let desc = dir_tool.description.as_deref().unwrap();
        assert!(
            desc.contains("2 myapp tools"),
            "Should include tool count in description: {desc}"
        );
    }

    #[tokio::test]
    async fn test_directory_tool_returns_json_with_tools_key() {
        let inner = MockRegistry {
            tools: vec![make_tool("get_item", "Get an item")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp");
        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        assert!(
            value.get("myapp_tools").is_some(),
            "Directory output should contain 'myapp_tools' key"
        );
        let tools_arr = value["myapp_tools"].as_array().unwrap();
        assert_eq!(tools_arr.len(), 1);
        assert_eq!(tools_arr[0]["name"], "get_item");
    }

    #[tokio::test]
    async fn test_directory_tool_includes_connectors() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry =
            DiscoverableRegistry::new(inner, "myapp").with_connector(ExternalConnector {
                name: "Slack MCP".into(),
                when_to_use: "Looking for recent Slack messages".into(),
                description: "Live Slack message search".into(),
            });

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let connectors = value["external_connectors"].as_array().unwrap();
        assert_eq!(connectors.len(), 1);
        assert_eq!(connectors[0]["name"], "Slack MCP");
    }

    #[tokio::test]
    async fn test_directory_tool_omits_empty_sections() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        // No connectors, no strategy, no freshness
        let registry = DiscoverableRegistry::new(inner, "myapp");
        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        assert!(
            value.get("external_connectors").is_none(),
            "Empty connectors should be omitted"
        );
        assert!(
            value.get("optimal_query_strategy").is_none(),
            "Empty strategy should be omitted"
        );
        assert!(
            value.get("data_freshness").is_none(),
            "Empty freshness should be omitted"
        );
    }

    #[tokio::test]
    async fn test_directory_tool_includes_query_strategy() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp").with_query_strategy(vec![
            "1. Call myapp_directory first",
            "2. Use search for lookups",
        ]);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let strategy = value["optimal_query_strategy"].as_array().unwrap();
        assert_eq!(strategy.len(), 2);
        assert_eq!(strategy[0], "1. Call myapp_directory first");
    }

    #[tokio::test]
    async fn test_directory_tool_includes_data_freshness() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let mut freshness = HashMap::new();
        freshness.insert("slack".into(), "Near real-time".into());
        freshness.insert("email".into(), "15-minute polling".into());

        let registry = DiscoverableRegistry::new(inner, "myapp").with_data_freshness(freshness);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let df = value.get("data_freshness").unwrap();
        assert_eq!(df["slack"], "Near real-time");
        assert_eq!(df["email"], "15-minute polling");
    }

    // ── call delegation ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_call_delegates_to_inner() {
        let inner = MockRegistry {
            tools: vec![make_tool("get_item", "Get an item")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp");
        let result = registry.call("get_item", json!({})).unwrap().await.unwrap();

        let text = extract_text(&result);
        assert_eq!(text, "result: get_item");
    }

    #[test]
    fn test_call_unknown_returns_none() {
        let inner = MockRegistry {
            tools: vec![make_tool("get_item", "Get an item")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp");
        assert!(registry.call("nonexistent", json!({})).is_none());
    }

    // ── composite registry integration ───────────────────────────────────────

    #[test]
    fn test_discoverable_with_composite_registry() {
        let reg1 = MockRegistry {
            tools: vec![make_tool("tool_a", "Tool A")],
        };
        let reg2 = MockRegistry {
            tools: vec![make_tool("tool_b", "Tool B")],
        };

        let composite = CompositeRegistry::new().add(reg1).add(reg2);
        let registry = DiscoverableRegistry::new(composite, "myapp");
        let tools = registry.tools();

        // 2 inner tools + 1 directory tool
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|t| t.name == "tool_a"));
        assert!(tools.iter().any(|t| t.name == "tool_b"));
        assert!(tools.iter().any(|t| t.name == "myapp_directory"));
    }

    // ── categories summary ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_categories_summary() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search"), make_tool("get_item", "Get")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp")
            .with_tool_meta(
                "search",
                ToolMeta {
                    category: Some("search".into()),
                    ..Default::default()
                },
            )
            .with_tool_meta(
                "get_item",
                ToolMeta {
                    category: Some("content".into()),
                    ..Default::default()
                },
            );

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let cats = value.get("categories").expect("Should have categories key");
        assert_eq!(cats["search"], 1);
        assert_eq!(cats["content"], 1);
    }

    #[tokio::test]
    async fn test_directory_tool_categories_defaults_to_general() {
        let inner = MockRegistry {
            tools: vec![make_tool("unknown_tool", "No meta")],
        };

        // No ToolMeta attached — should default to "general"
        let registry = DiscoverableRegistry::new(inner, "myapp");
        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let cats = value.get("categories").expect("Should have categories key");
        assert_eq!(cats["general"], 1);
    }

    // ── ToolMeta default ─────────────────────────────────────────────────────

    #[test]
    fn test_tool_meta_default() {
        let meta = ToolMeta::default();
        assert!(meta.summary.is_empty());
        assert!(meta.when_to_use.is_empty());
        assert!(meta.returns.is_empty());
        assert!(meta.next.is_none());
        assert!(meta.category.is_none());
    }
}
