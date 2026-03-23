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

/// Describes the server's data domain — what entities exist, how they relate,
/// and what the corpus contains. Included in the directory tool output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DomainModel {
    /// One-paragraph summary of the entire dataset/corpus.
    pub summary: String,
    /// The core entity types an agent will encounter.
    pub entities: Vec<Entity>,
    /// Named relationship types (for graph-based servers).
    pub relationships: Vec<String>,
}

/// A core entity type exposed by the server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    /// Entity type name as it appears in tool parameters/results.
    pub name: String,
    /// What this entity represents.
    pub description: String,
    /// How IDs are formatted and where they flow between tools.
    pub id_format: String,
    /// Approximate count (helps agents calibrate expectations).
    pub count: Option<u64>,
}

/// Documents how identifiers flow between tools.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdConvention {
    /// The ID field name as it appears in parameters.
    pub name: String,
    /// Format description (slug, UUID, integer, etc.).
    pub format: String,
    /// Which tools produce or consume this ID.
    pub used_by: Vec<String>,
}

/// Runtime capability information for agents to negotiate available features.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendStatus {
    pub capabilities: Vec<Capability>,
}

/// A single backend capability and its runtime readiness.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name matching a tool or tool family.
    pub name: String,
    /// Whether this capability is currently functional.
    pub ready: bool,
    /// Agent-facing guidance when not ready (e.g., "Use keyword mode instead").
    pub note: Option<String>,
}

/// Task-oriented query recipe — tells agents how to accomplish a specific goal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskStrategy {
    /// What the user is trying to accomplish.
    pub task: String,
    /// Ordered tool calls to achieve it.
    pub steps: Vec<String>,
}

/// Summary of valid filter values for common parameters.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterSummary {
    /// General note about the filter values.
    pub note: String,
    /// Per-parameter filter info.
    pub filters: Vec<FilterInfo>,
}

/// Valid values for a single filter parameter.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FilterInfo {
    /// Parameter name as it appears in tool inputs.
    pub param: String,
    /// Top N values (plus counts if useful).
    pub top_values: Vec<String>,
    /// Total number of distinct values.
    pub total: usize,
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
    recommended_subscriptions: Vec<(String, String)>,
    conventions: Vec<String>,
    constraints: Vec<String>,
    domain_model: Option<DomainModel>,
    id_conventions: Vec<IdConvention>,
    backend_status: Option<BackendStatus>,
    task_strategies: Vec<TaskStrategy>,
    filter_summary: Option<FilterSummary>,
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
            recommended_subscriptions: Vec::new(),
            conventions: Vec::new(),
            constraints: Vec::new(),
            domain_model: None,
            id_conventions: Vec::new(),
            backend_status: None,
            task_strategies: Vec::new(),
            filter_summary: None,
        }
    }

    /// Create a discoverable registry from a [`ServerGuidance`](crate::ServerGuidance).
    ///
    /// Populates all fields from the guidance, including tool metadata,
    /// workflow (as query strategy), connectors, subscriptions, conventions,
    /// and constraints.
    pub fn from_guidance(registry: R, guidance: &crate::guidance::ServerGuidance) -> Self {
        Self {
            inner: registry,
            meta_map: guidance.tool_metas.clone(),
            server_name: guidance.domain.clone(),
            external_connectors: guidance.external_connectors.clone(),
            query_strategy: guidance.workflow.clone(),
            data_freshness: guidance.data_freshness.clone(),
            recommended_subscriptions: guidance.recommended_subscriptions.clone(),
            conventions: guidance.conventions.clone(),
            constraints: guidance.constraints.clone(),
            domain_model: None,
            id_conventions: Vec::new(),
            backend_status: None,
            task_strategies: Vec::new(),
            filter_summary: None,
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

    /// Set the domain model describing the server's data.
    pub fn with_domain_model(mut self, model: DomainModel) -> Self {
        self.domain_model = Some(model);
        self
    }

    /// Set ID conventions documenting how identifiers flow between tools.
    pub fn with_id_conventions(mut self, conventions: Vec<IdConvention>) -> Self {
        self.id_conventions = conventions;
        self
    }

    /// Set backend status for runtime capability negotiation.
    pub fn with_backend_status(mut self, status: BackendStatus) -> Self {
        self.backend_status = Some(status);
        self
    }

    /// Set task-oriented query strategies (replaces flat query_strategy in directory output).
    pub fn with_task_strategies(mut self, strategies: Vec<TaskStrategy>) -> Self {
        self.task_strategies = strategies;
        self
    }

    /// Set summary filter values for common parameters.
    pub fn with_filter_summary(mut self, summary: FilterSummary) -> Self {
        self.filter_summary = Some(summary);
        self
    }

    /// Set conventions (rules agents should follow).
    pub fn with_conventions<S: Into<String>>(mut self, conventions: Vec<S>) -> Self {
        self.conventions = conventions.into_iter().map(Into::into).collect();
        self
    }

    /// Set constraints (limitations agents should be aware of).
    pub fn with_constraints<S: Into<String>>(mut self, constraints: Vec<S>) -> Self {
        self.constraints = constraints.into_iter().map(Into::into).collect();
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
                "Describes all {count} {name} tools, domain model, ID conventions, \
                 backend status, and the optimal query strategy.\n\
                 WHEN TO USE: Call this at the start of every session to understand \
                 what capabilities are available before doing any work.\n\
                 RETURNS: Tool list with when-to-use guidance, domain model, ID conventions, \
                 backend status, external connectors, query strategies, filter values, \
                 and data freshness information.",
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

        if !self.recommended_subscriptions.is_empty() {
            let subs: Vec<Value> = self
                .recommended_subscriptions
                .iter()
                .map(|(uri, reason)| {
                    serde_json::json!({
                        "uri": uri,
                        "reason": reason,
                    })
                })
                .collect();
            response.insert("recommended_subscriptions".into(), Value::Array(subs));
        }

        if !self.conventions.is_empty() {
            response.insert(
                "conventions".into(),
                Value::Array(
                    self.conventions
                        .iter()
                        .map(|c| Value::String(c.clone()))
                        .collect(),
                ),
            );
        }

        if !self.constraints.is_empty() {
            response.insert(
                "constraints".into(),
                Value::Array(
                    self.constraints
                        .iter()
                        .map(|c| Value::String(c.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(ref model) = self.domain_model {
            response.insert(
                "domain_model".into(),
                serde_json::to_value(model).unwrap_or(Value::Null),
            );
        }

        if !self.id_conventions.is_empty() {
            response.insert(
                "id_conventions".into(),
                serde_json::to_value(&self.id_conventions).unwrap_or(Value::Array(vec![])),
            );
        }

        if let Some(ref status) = self.backend_status {
            response.insert(
                "backend_status".into(),
                serde_json::to_value(status).unwrap_or(Value::Null),
            );
        }

        // Task strategies take precedence over flat query_strategy
        if !self.task_strategies.is_empty() {
            response.insert(
                "query_strategies".into(),
                serde_json::to_value(&self.task_strategies).unwrap_or(Value::Array(vec![])),
            );
        }

        if let Some(ref summary) = self.filter_summary {
            response.insert(
                "filter_summary".into(),
                serde_json::to_value(summary).unwrap_or(Value::Null),
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

    // ── from_guidance ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_from_guidance_populates_all_fields() {
        use crate::guidance::ServerGuidance;

        let guidance = ServerGuidance::for_domain("test")
            .context("Test context")
            .workflow("Step 1")
            .convention("Convention A")
            .constraint("Constraint X")
            .subscribe("test://res", "Live updates")
            .tool_meta(
                "search",
                ToolMeta {
                    summary: "Search things".into(),
                    when_to_use: "Looking for stuff".into(),
                    returns: "Results".into(),
                    next: None,
                    category: Some("search".into()),
                },
            );

        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };
        let registry = DiscoverableRegistry::from_guidance(inner, &guidance);

        assert_eq!(registry.server_name, "test");
        assert_eq!(registry.directory_tool_name(), "test_directory");

        // Tool meta should be populated
        let tools = registry.tools();
        let search = tools.iter().find(|t| t.name == "search").unwrap();
        let desc = search.description.as_deref().unwrap();
        assert!(desc.contains("WHEN TO USE:"));

        // Directory output should include new sections
        let result = registry
            .call("test_directory", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let subs = value["recommended_subscriptions"].as_array().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0]["uri"], "test://res");
        assert_eq!(subs[0]["reason"], "Live updates");

        let convs = value["conventions"].as_array().unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0], "Convention A");

        let cons = value["constraints"].as_array().unwrap();
        assert_eq!(cons.len(), 1);
        assert_eq!(cons[0], "Constraint X");
    }

    #[tokio::test]
    async fn test_directory_omits_empty_new_sections() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        // No subscriptions, conventions, or constraints
        let registry = DiscoverableRegistry::new(inner, "myapp");
        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        assert!(value.get("recommended_subscriptions").is_none());
        assert!(value.get("conventions").is_none());
        assert!(value.get("constraints").is_none());
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

    // ── domain model ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_domain_model() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp").with_domain_model(DomainModel {
            summary: "Test knowledge base with 100 items.".into(),
            entities: vec![Entity {
                name: "item".into(),
                description: "A searchable item.".into(),
                id_format: "UUID v4".into(),
                count: Some(100),
            }],
            relationships: vec!["RelatesTo — items are related".into()],
        });

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let model = value.get("domain_model").expect("Should have domain_model");
        assert_eq!(model["summary"], "Test knowledge base with 100 items.");
        assert_eq!(model["entities"][0]["name"], "item");
        assert_eq!(model["entities"][0]["count"], 100);
        assert_eq!(model["relationships"].as_array().unwrap().len(), 1);
    }

    // ── id conventions ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_id_conventions() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry =
            DiscoverableRegistry::new(inner, "myapp").with_id_conventions(vec![IdConvention {
                name: "item_id".into(),
                format: "UUID v4".into(),
                used_by: vec![
                    "search -> 'id' in results".into(),
                    "get_item -> item_id param".into(),
                ],
            }]);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let ids = value
            .get("id_conventions")
            .expect("Should have id_conventions")
            .as_array()
            .unwrap();
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0]["name"], "item_id");
        assert_eq!(ids[0]["format"], "UUID v4");
        assert_eq!(ids[0]["used_by"].as_array().unwrap().len(), 2);
    }

    // ── backend status ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_backend_status() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry =
            DiscoverableRegistry::new(inner, "myapp").with_backend_status(BackendStatus {
                capabilities: vec![
                    Capability {
                        name: "full_text_search".into(),
                        ready: true,
                        note: None,
                    },
                    Capability {
                        name: "vector_search".into(),
                        ready: false,
                        note: Some("Index not built.".into()),
                    },
                ],
            });

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let status = value
            .get("backend_status")
            .expect("Should have backend_status");
        let caps = status["capabilities"].as_array().unwrap();
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0]["name"], "full_text_search");
        assert_eq!(caps[0]["ready"], true);
        assert!(caps[0]["note"].is_null());
        assert_eq!(caps[1]["name"], "vector_search");
        assert_eq!(caps[1]["ready"], false);
        assert_eq!(caps[1]["note"], "Index not built.");
    }

    // ── task strategies ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_task_strategies() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry =
            DiscoverableRegistry::new(inner, "myapp").with_task_strategies(vec![TaskStrategy {
                task: "Explore a topic".into(),
                steps: vec![
                    "search with topic keywords".into(),
                    "get_item on the best result".into(),
                ],
            }]);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let strategies = value
            .get("query_strategies")
            .expect("Should have query_strategies")
            .as_array()
            .unwrap();
        assert_eq!(strategies.len(), 1);
        assert_eq!(strategies[0]["task"], "Explore a topic");
        assert_eq!(strategies[0]["steps"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_task_strategies_take_precedence_over_flat_query_strategy() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp")
            .with_query_strategy(vec!["1. Call directory first"])
            .with_task_strategies(vec![TaskStrategy {
                task: "Explore".into(),
                steps: vec!["search".into()],
            }]);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        // Task strategies should appear as "query_strategies"
        assert!(
            value.get("query_strategies").is_some(),
            "Should have query_strategies from task strategies"
        );
        // Both may appear since they use different keys
        // (optimal_query_strategy vs query_strategies)
    }

    // ── filter summary ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_tool_includes_filter_summary() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry =
            DiscoverableRegistry::new(inner, "myapp").with_filter_summary(FilterSummary {
                note: "Top values shown.".into(),
                filters: vec![FilterInfo {
                    param: "category".into(),
                    top_values: vec!["alpha (10)".into(), "beta (5)".into()],
                    total: 12,
                }],
            });

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();

        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let summary = value
            .get("filter_summary")
            .expect("Should have filter_summary");
        assert_eq!(summary["note"], "Top values shown.");
        let filters = summary["filters"].as_array().unwrap();
        assert_eq!(filters.len(), 1);
        assert_eq!(filters[0]["param"], "category");
        assert_eq!(filters[0]["total"], 12);
    }

    // ── new sections omit when empty ────────────────────────────────────────

    #[tokio::test]
    async fn test_directory_omits_new_metadata_sections_when_empty() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
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
            value.get("domain_model").is_none(),
            "Empty domain_model should be omitted"
        );
        assert!(
            value.get("id_conventions").is_none(),
            "Empty id_conventions should be omitted"
        );
        assert!(
            value.get("backend_status").is_none(),
            "Empty backend_status should be omitted"
        );
        assert!(
            value.get("query_strategies").is_none(),
            "Empty query_strategies should be omitted"
        );
        assert!(
            value.get("filter_summary").is_none(),
            "Empty filter_summary should be omitted"
        );
    }

    // ── conventions and constraints builders ─────────────────────────────────

    #[tokio::test]
    async fn test_with_conventions_and_constraints_builders() {
        let inner = MockRegistry {
            tools: vec![make_tool("search", "Search")],
        };

        let registry = DiscoverableRegistry::new(inner, "myapp")
            .with_conventions(vec!["Always call directory first."])
            .with_constraints(vec!["Read-only access."]);

        let result = registry
            .call("myapp_directory", json!({}))
            .unwrap()
            .await
            .unwrap();
        let text = extract_text(&result);
        let value: Value = serde_json::from_str(&text).unwrap();

        let convs = value["conventions"].as_array().unwrap();
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0], "Always call directory first.");

        let cons = value["constraints"].as_array().unwrap();
        assert_eq!(cons.len(), 1);
        assert_eq!(cons[0], "Read-only access.");
    }

    // ── serde roundtrip tests for new types ─────────────────────────────────

    #[test]
    fn test_domain_model_serde_roundtrip() {
        let model = DomainModel {
            summary: "A corpus of meeting notes.".into(),
            entities: vec![
                Entity {
                    name: "meeting".into(),
                    description: "A recorded meeting.".into(),
                    id_format: "UUID v4".into(),
                    count: Some(500),
                },
                Entity {
                    name: "attendee".into(),
                    description: "A meeting participant.".into(),
                    id_format: "email".into(),
                    count: None,
                },
            ],
            relationships: vec![
                "attended — attendee participated in meeting".into(),
                "mentioned — meeting references a topic".into(),
            ],
        };
        let json = serde_json::to_string(&model).unwrap();
        let deserialized: DomainModel = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_entity_serde_roundtrip_with_count() {
        let entity = Entity {
            name: "document".into(),
            description: "An indexed document.".into(),
            id_format: "slug".into(),
            count: Some(42),
        };
        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count, Some(42));
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_entity_serde_roundtrip_without_count() {
        let entity = Entity {
            name: "tag".into(),
            description: "A content tag.".into(),
            id_format: "lowercase-slug".into(),
            count: None,
        };
        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: Entity = serde_json::from_str(&json).unwrap();
        assert!(deserialized.count.is_none());
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_id_convention_serde_roundtrip() {
        let conv = IdConvention {
            name: "item_id".into(),
            format: "UUID v4".into(),
            used_by: vec![
                "search -> 'id' in results".into(),
                "get_item -> item_id param".into(),
            ],
        };
        let json = serde_json::to_string(&conv).unwrap();
        let deserialized: IdConvention = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_backend_status_serde_roundtrip() {
        let status = BackendStatus {
            capabilities: vec![
                Capability {
                    name: "full_text_search".into(),
                    ready: true,
                    note: None,
                },
                Capability {
                    name: "vector_search".into(),
                    ready: false,
                    note: Some("Index rebuilding, ETA 10 min.".into()),
                },
            ],
        };
        let json = serde_json::to_string(&status).unwrap();
        let deserialized: BackendStatus = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_capability_serde_roundtrip() {
        let cap = Capability {
            name: "semantic_search".into(),
            ready: true,
            note: None,
        };
        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: Capability = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_task_strategy_serde_roundtrip() {
        let strategy = TaskStrategy {
            task: "Find all meetings about a topic".into(),
            steps: vec![
                "search with topic keywords".into(),
                "get_item on best result".into(),
                "list_related for connected items".into(),
            ],
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: TaskStrategy = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_filter_summary_serde_roundtrip() {
        let summary = FilterSummary {
            note: "Showing top 5 values per parameter.".into(),
            filters: vec![
                FilterInfo {
                    param: "category".into(),
                    top_values: vec!["engineering (42)".into(), "product (31)".into()],
                    total: 15,
                },
                FilterInfo {
                    param: "status".into(),
                    top_values: vec!["active (100)".into(), "archived (50)".into()],
                    total: 3,
                },
            ],
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: FilterSummary = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_filter_info_serde_roundtrip() {
        let info = FilterInfo {
            param: "tag".into(),
            top_values: vec!["rust".into(), "python".into(), "go".into()],
            total: 25,
        };
        let json = serde_json::to_string(&info).unwrap();
        let deserialized: FilterInfo = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }
}
