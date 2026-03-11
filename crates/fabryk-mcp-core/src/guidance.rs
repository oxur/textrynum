//! Composable server guidance for MCP servers.
//!
//! [`ServerGuidance`] unifies all server instruction mechanisms into a single
//! composable builder. It replaces scattered `with_description` /
//! `with_discoverable_instructions` / `with_query_strategy` calls with a
//! single declarative structure.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::ServerGuidance;
//!
//! let guidance = ServerGuidance::for_domain("nms")
//!     .context("Galactic copilot for No Man's Sky")
//!     .workflow("Call where_am_i to establish location")
//!     .convention("Distances are in light-years")
//!     .subscribe("nms://player/location", "Live warp tracking");
//! ```

use crate::discoverable::{ExternalConnector, ToolMeta};
use std::collections::HashMap;

/// Composable server guidance that generates instructions and directory metadata.
#[derive(Clone, Debug, Default)]
pub struct ServerGuidance {
    /// The domain name (e.g., "nms"). Used for directory tool naming.
    pub domain: String,
    /// High-level context about what this server does.
    pub context: Option<String>,
    /// Ordered workflow steps (replaces `query_strategy`).
    pub workflow: Vec<String>,
    /// Domain conventions the AI should follow.
    pub conventions: Vec<String>,
    /// Hard constraints the AI must obey.
    pub constraints: Vec<String>,
    /// Structured metadata per tool.
    pub tool_metas: HashMap<String, ToolMeta>,
    /// External connectors the server can reach.
    pub external_connectors: Vec<ExternalConnector>,
    /// Data freshness per source.
    pub data_freshness: HashMap<String, String>,
    /// Recommended resource subscriptions: `(uri, reason)`.
    pub recommended_subscriptions: Vec<(String, String)>,
}

impl ServerGuidance {
    /// Create guidance for a named domain.
    pub fn for_domain(domain: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            ..Default::default()
        }
    }

    /// Set high-level context about this server.
    pub fn context(mut self, text: impl Into<String>) -> Self {
        self.context = Some(text.into());
        self
    }

    /// Add a recommended resource subscription.
    pub fn subscribe(mut self, uri: impl Into<String>, reason: impl Into<String>) -> Self {
        self.recommended_subscriptions
            .push((uri.into(), reason.into()));
        self
    }

    /// Add a workflow step.
    pub fn workflow(mut self, step: impl Into<String>) -> Self {
        self.workflow.push(step.into());
        self
    }

    /// Add a domain convention.
    pub fn convention(mut self, text: impl Into<String>) -> Self {
        self.conventions.push(text.into());
        self
    }

    /// Add a hard constraint.
    pub fn constraint(mut self, text: impl Into<String>) -> Self {
        self.constraints.push(text.into());
        self
    }

    /// Register metadata for a single tool.
    pub fn tool_meta(mut self, name: impl Into<String>, meta: ToolMeta) -> Self {
        self.tool_metas.insert(name.into(), meta);
        self
    }

    /// Register metadata for multiple tools.
    pub fn tool_metas<S: Into<String>>(mut self, metas: Vec<(S, ToolMeta)>) -> Self {
        for (name, meta) in metas {
            self.tool_metas.insert(name.into(), meta);
        }
        self
    }

    /// Add an external connector.
    pub fn connector(mut self, connector: ExternalConnector) -> Self {
        self.external_connectors.push(connector);
        self
    }

    /// Add a data freshness entry.
    pub fn data_freshness(mut self, source: impl Into<String>, info: impl Into<String>) -> Self {
        self.data_freshness.insert(source.into(), info.into());
        self
    }

    /// Generate the `ServerInfo.instructions` text.
    ///
    /// Includes the directory-first directive, context, subscription
    /// recommendations, and conventions.
    pub fn to_instructions(&self) -> String {
        let mut parts = Vec::new();

        // Directory-first directive
        parts.push(format!(
            "ALWAYS call {}_directory first \u{2014} it maps all available tools, \
             valid filter values, and the optimal query strategy for this session.",
            self.domain
        ));

        // Context
        if let Some(ctx) = &self.context {
            parts.push(ctx.clone());
        }

        // Subscription recommendations
        if !self.recommended_subscriptions.is_empty() {
            let subs: Vec<String> = self
                .recommended_subscriptions
                .iter()
                .map(|(uri, reason)| format!("- {uri}: {reason}"))
                .collect();
            parts.push(format!("Recommended subscriptions:\n{}", subs.join("\n")));
        }

        // Conventions
        if !self.conventions.is_empty() {
            let convs: Vec<String> = self.conventions.iter().map(|c| format!("- {c}")).collect();
            parts.push(format!("Conventions:\n{}", convs.join("\n")));
        }

        // Constraints
        if !self.constraints.is_empty() {
            let cons: Vec<String> = self.constraints.iter().map(|c| format!("- {c}")).collect();
            parts.push(format!("Constraints:\n{}", cons.join("\n")));
        }

        parts.join("\n\n")
    }

    /// Return the directory tool name for this domain.
    pub fn directory_tool_name(&self) -> String {
        format!("{}_directory", self.domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_domain() {
        let g = ServerGuidance::for_domain("nms");
        assert_eq!(g.domain, "nms");
        assert!(g.context.is_none());
        assert!(g.workflow.is_empty());
    }

    #[test]
    fn test_builder_chaining() {
        let g = ServerGuidance::for_domain("myapp")
            .context("A test app")
            .workflow("Step 1")
            .workflow("Step 2")
            .convention("Use metric units")
            .constraint("Never delete data")
            .subscribe("myapp://status", "Live updates");

        assert_eq!(g.context.as_deref(), Some("A test app"));
        assert_eq!(g.workflow.len(), 2);
        assert_eq!(g.conventions.len(), 1);
        assert_eq!(g.constraints.len(), 1);
        assert_eq!(g.recommended_subscriptions.len(), 1);
    }

    #[test]
    fn test_directory_tool_name() {
        let g = ServerGuidance::for_domain("nms");
        assert_eq!(g.directory_tool_name(), "nms_directory");
    }

    #[test]
    fn test_to_instructions_includes_directive() {
        let g = ServerGuidance::for_domain("nms");
        let text = g.to_instructions();
        assert!(text.contains("ALWAYS call nms_directory first"));
    }

    #[test]
    fn test_to_instructions_includes_context() {
        let g = ServerGuidance::for_domain("nms").context("Galactic copilot");
        let text = g.to_instructions();
        assert!(text.contains("Galactic copilot"));
    }

    #[test]
    fn test_to_instructions_includes_subscriptions() {
        let g = ServerGuidance::for_domain("nms").subscribe("nms://player", "Live tracking");
        let text = g.to_instructions();
        assert!(text.contains("Recommended subscriptions:"));
        assert!(text.contains("nms://player: Live tracking"));
    }

    #[test]
    fn test_to_instructions_includes_conventions() {
        let g = ServerGuidance::for_domain("nms").convention("Distances in light-years");
        let text = g.to_instructions();
        assert!(text.contains("Conventions:"));
        assert!(text.contains("Distances in light-years"));
    }

    #[test]
    fn test_to_instructions_includes_constraints() {
        let g = ServerGuidance::for_domain("nms").constraint("Read-only access");
        let text = g.to_instructions();
        assert!(text.contains("Constraints:"));
        assert!(text.contains("Read-only access"));
    }

    #[test]
    fn test_to_instructions_omits_empty_sections() {
        let g = ServerGuidance::for_domain("nms");
        let text = g.to_instructions();
        assert!(!text.contains("Recommended subscriptions:"));
        assert!(!text.contains("Conventions:"));
        assert!(!text.contains("Constraints:"));
    }

    #[test]
    fn test_tool_meta_single() {
        let g = ServerGuidance::for_domain("nms").tool_meta(
            "search",
            ToolMeta {
                summary: "Search things".into(),
                ..Default::default()
            },
        );
        assert!(g.tool_metas.contains_key("search"));
    }

    #[test]
    fn test_tool_metas_multiple() {
        let g = ServerGuidance::for_domain("nms").tool_metas(vec![
            (
                "search",
                ToolMeta {
                    summary: "Search".into(),
                    ..Default::default()
                },
            ),
            (
                "route",
                ToolMeta {
                    summary: "Route".into(),
                    ..Default::default()
                },
            ),
        ]);
        assert_eq!(g.tool_metas.len(), 2);
    }

    #[test]
    fn test_connector() {
        let g = ServerGuidance::for_domain("nms").connector(ExternalConnector {
            name: "External".into(),
            when_to_use: "Always".into(),
            description: "An external service".into(),
        });
        assert_eq!(g.external_connectors.len(), 1);
    }

    #[test]
    fn test_data_freshness() {
        let g = ServerGuidance::for_domain("nms")
            .data_freshness("save_file", "Real-time via file watcher");
        assert_eq!(g.data_freshness.len(), 1);
        assert_eq!(
            g.data_freshness.get("save_file").unwrap(),
            "Real-time via file watcher"
        );
    }
}
