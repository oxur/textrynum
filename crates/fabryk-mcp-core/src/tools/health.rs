//! Health check tool for MCP servers.
//!
//! Provides a built-in `health` tool that reports server status,
//! tool count, version information, and optional backend diagnostics.
//!
//! # Backward Compatibility
//!
//! The existing [`HealthTools::new()`] constructor continues to work
//! unchanged. New fields (`backends`, `search_config`) are optional
//! and only appear when configured via builder methods.

use std::sync::Arc;

use crate::registry::{ToolRegistry, ToolResult};
use fabryk_core::BackendProbe;
use rmcp::model::{CallToolResult, Content, ErrorData, Tool};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Health check response.
///
/// Contains core server metadata plus optional backend diagnostics
/// and search configuration. Fields added in later versions use
/// `skip_serializing_if` to maintain backward-compatible JSON output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Server status ("healthy").
    pub status: String,
    /// Server name.
    pub server_name: String,
    /// Server version.
    pub version: String,
    /// Number of registered tools.
    pub tool_count: usize,
    /// Backend service diagnostics (omitted when no probes are registered).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backends: Option<BackendsInfo>,
    /// Search configuration summary (omitted when not provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_config: Option<SearchConfigInfo>,
}

/// Collection of backend service diagnostics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendsInfo {
    /// Individual backend service reports.
    pub services: Vec<BackendInfo>,
}

/// Diagnostic report for a single backend service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BackendInfo {
    /// Backend identifier (e.g., "tantivy", "simple", "lancedb").
    pub name: String,
    /// Backend kind for grouping (e.g., "fts", "vector").
    pub kind: String,
    /// Whether the backend is ready to handle requests.
    pub ready: bool,
    /// Number of indexed documents (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_count: Option<usize>,
    /// When the backend was last indexed (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_indexed: Option<String>,
}

/// Search configuration summary for health output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchConfigInfo {
    /// Active query mode (e.g., "smart", "and", "or").
    pub query_mode: String,
    /// Whether stopword filtering is enabled.
    pub stopwords_enabled: bool,
    /// Whether fuzzy search is enabled.
    pub fuzzy_search: bool,
    /// Field boost weights (if configured).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_boosts: Option<FieldBoosts>,
}

/// Field boost weights for search scoring.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldBoosts {
    /// Title field boost weight.
    pub title: f32,
    /// Description field boost weight.
    pub description: f32,
    /// Content field boost weight.
    pub content: f32,
}

/// A tool registry that provides the `health` tool.
///
/// Captures server metadata at construction time and reports it
/// when the tool is called. Optionally includes backend diagnostics
/// and search configuration when configured via builder methods.
///
/// # Backward Compatibility
///
/// [`HealthTools::new()`] creates an instance with no backends and no
/// search config, producing the same JSON output as before enrichment
/// was added.
pub struct HealthTools {
    server_name: String,
    version: String,
    total_tool_count: usize,
    backends: Vec<Arc<dyn BackendProbe>>,
    search_config: Option<SearchConfigInfo>,
}

impl HealthTools {
    /// Create health tools with server metadata.
    ///
    /// `total_tool_count` should include the health tool itself.
    ///
    /// This constructor produces a backward-compatible instance with
    /// no backend probes and no search configuration.
    pub fn new(
        server_name: impl Into<String>,
        version: impl Into<String>,
        total_tool_count: usize,
    ) -> Self {
        Self {
            server_name: server_name.into(),
            version: version.into(),
            total_tool_count,
            backends: Vec::new(),
            search_config: None,
        }
    }

    /// Attach backend probes for health diagnostics.
    ///
    /// Each probe is queried at call time to build the `backends`
    /// section of the health response.
    pub fn with_backends(mut self, backends: Vec<Arc<dyn BackendProbe>>) -> Self {
        self.backends = backends;
        self
    }

    /// Attach search configuration for health diagnostics.
    pub fn with_search_config(mut self, config: SearchConfigInfo) -> Self {
        self.search_config = Some(config);
        self
    }

    /// Build the health response by querying all registered probes.
    fn build_response(&self) -> HealthResponse {
        let backends_info = if self.backends.is_empty() {
            None
        } else {
            Some(BackendsInfo {
                services: self
                    .backends
                    .iter()
                    .map(|b| BackendInfo {
                        name: b.probe_name().to_string(),
                        kind: b.probe_kind().to_string(),
                        ready: b.probe_ready(),
                        document_count: b.probe_document_count(),
                        last_indexed: b.probe_last_indexed(),
                    })
                    .collect(),
            })
        };

        HealthResponse {
            status: "healthy".to_string(),
            server_name: self.server_name.clone(),
            version: self.version.clone(),
            tool_count: self.total_tool_count,
            backends: backends_info,
            search_config: self.search_config.clone(),
        }
    }
}

impl ToolRegistry for HealthTools {
    fn tools(&self) -> Vec<Tool> {
        vec![Tool::new(
            "health",
            "Check server health and status",
            crate::empty_input_schema(),
        )]
    }

    fn call(&self, name: &str, _args: Value) -> Option<ToolResult> {
        if name != "health" {
            return None;
        }

        let response = self.build_response();

        Some(Box::pin(async move {
            let json = serde_json::to_string_pretty(&response)
                .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
            Ok(CallToolResult::success(vec![Content::text(json)]))
        }))
    }
}

/// Handle a health check request directly (without registry).
///
/// Useful for testing or when building custom tool dispatchers.
/// This is the backward-compatible version without backend probes.
pub async fn handle_health(
    server_name: &str,
    version: &str,
    tool_count: usize,
) -> Result<CallToolResult, ErrorData> {
    let response = HealthResponse {
        status: "healthy".to_string(),
        server_name: server_name.to_string(),
        version: version.to_string(),
        tool_count,
        backends: None,
        search_config: None,
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Handle an enriched health check request with backend probes.
///
/// Queries each probe to build the full health response including
/// backend diagnostics and optional search configuration.
pub async fn handle_health_enriched(
    server_name: &str,
    version: &str,
    tool_count: usize,
    backends: &[Arc<dyn BackendProbe>],
    search_config: Option<&SearchConfigInfo>,
) -> Result<CallToolResult, ErrorData> {
    let backends_info = if backends.is_empty() {
        None
    } else {
        Some(BackendsInfo {
            services: backends
                .iter()
                .map(|b| BackendInfo {
                    name: b.probe_name().to_string(),
                    kind: b.probe_kind().to_string(),
                    ready: b.probe_ready(),
                    document_count: b.probe_document_count(),
                    last_indexed: b.probe_last_indexed(),
                })
                .collect(),
        })
    };

    let response = HealthResponse {
        status: "healthy".to_string(),
        server_name: server_name.to_string(),
        version: version.to_string(),
        tool_count,
        backends: backends_info,
        search_config: search_config.cloned(),
    };

    let json = serde_json::to_string_pretty(&response)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Mock probe for testing health tools without real backends.
    struct MockProbe {
        name: &'static str,
        kind: &'static str,
        ready: bool,
        doc_count: Option<usize>,
        last_indexed: Option<String>,
    }

    impl MockProbe {
        fn new(name: &'static str, kind: &'static str) -> Self {
            Self {
                name,
                kind,
                ready: true,
                doc_count: None,
                last_indexed: None,
            }
        }

        fn with_ready(mut self, ready: bool) -> Self {
            self.ready = ready;
            self
        }

        fn with_document_count(mut self, count: usize) -> Self {
            self.doc_count = Some(count);
            self
        }

        fn with_last_indexed(mut self, ts: &'static str) -> Self {
            self.last_indexed = Some(ts.to_string());
            self
        }
    }

    impl BackendProbe for MockProbe {
        fn probe_name(&self) -> &str {
            self.name
        }
        fn probe_ready(&self) -> bool {
            self.ready
        }
        fn probe_document_count(&self) -> Option<usize> {
            self.doc_count
        }
        fn probe_last_indexed(&self) -> Option<String> {
            self.last_indexed.clone()
        }
        fn probe_kind(&self) -> &str {
            self.kind
        }
    }

    // ---- Existing tests (backward compatibility) ----

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            server_name: "test-server".to_string(),
            version: "0.1.0".to_string(),
            tool_count: 5,
            backends: None,
            search_config: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("healthy"));
        assert!(json.contains("test-server"));
        assert!(json.contains("0.1.0"));
        assert!(json.contains("5"));
    }

    #[test]
    fn test_health_response_deserialization() {
        let json = r#"{"status":"healthy","server_name":"test","version":"1.0","tool_count":3}"#;
        let response: HealthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.status, "healthy");
        assert_eq!(response.tool_count, 3);
        assert!(response.backends.is_none());
        assert!(response.search_config.is_none());
    }

    #[test]
    fn test_health_tools_creation() {
        let tools = HealthTools::new("server", "1.0", 10);
        assert_eq!(tools.tool_count(), 1);
        assert!(tools.has_tool("health"));
        assert!(!tools.has_tool("other"));
    }

    #[tokio::test]
    async fn test_health_tools_call() {
        let tools = HealthTools::new("test-server", "0.1.0", 5);
        let future = tools.call("health", json!({})).unwrap();
        let result = future.await.unwrap();

        // Should be a success result
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_health_tools_unknown_tool() {
        let tools = HealthTools::new("server", "1.0", 1);
        assert!(tools.call("unknown", json!({})).is_none());
    }

    #[tokio::test]
    async fn test_handle_health_direct() {
        let result = handle_health("my-server", "2.0.0", 10).await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[test]
    fn test_health_tool_info() {
        let tools = HealthTools::new("server", "1.0", 1);
        let tool_list = tools.tools();
        assert_eq!(tool_list.len(), 1);
        assert_eq!(tool_list[0].name, "health");
        assert!(tool_list[0].description.is_some());
    }

    // ---- New tests: backend probes ----

    #[tokio::test]
    async fn test_health_with_backends() {
        let probes: Vec<Arc<dyn BackendProbe>> = vec![
            Arc::new(
                MockProbe::new("tantivy", "fts")
                    .with_document_count(150)
                    .with_last_indexed("2026-04-01T12:00:00Z"),
            ),
            Arc::new(MockProbe::new("simple-vector", "vector").with_document_count(42)),
        ];

        let tools = HealthTools::new("test-server", "0.2.0", 5).with_backends(probes);
        let future = tools.call("health", json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        // Parse the response JSON to verify backend info
        let content_text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        let response: HealthResponse = serde_json::from_str(&content_text).unwrap();

        let backends = response.backends.unwrap();
        assert_eq!(backends.services.len(), 2);

        let fts = &backends.services[0];
        assert_eq!(fts.name, "tantivy");
        assert_eq!(fts.kind, "fts");
        assert!(fts.ready);
        assert_eq!(fts.document_count, Some(150));
        assert_eq!(fts.last_indexed, Some("2026-04-01T12:00:00Z".to_string()));

        let vector = &backends.services[1];
        assert_eq!(vector.name, "simple-vector");
        assert_eq!(vector.kind, "vector");
        assert_eq!(vector.document_count, Some(42));
        assert!(vector.last_indexed.is_none());
    }

    #[tokio::test]
    async fn test_health_with_search_config() {
        let config = SearchConfigInfo {
            query_mode: "smart".to_string(),
            stopwords_enabled: true,
            fuzzy_search: false,
            field_boosts: Some(FieldBoosts {
                title: 3.0,
                description: 2.0,
                content: 1.0,
            }),
        };

        let tools = HealthTools::new("test-server", "0.2.0", 5).with_search_config(config);
        let future = tools.call("health", json!({})).unwrap();
        let result = future.await.unwrap();

        let content_text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        let response: HealthResponse = serde_json::from_str(&content_text).unwrap();

        let search_config = response.search_config.unwrap();
        assert_eq!(search_config.query_mode, "smart");
        assert!(search_config.stopwords_enabled);
        assert!(!search_config.fuzzy_search);

        let boosts = search_config.field_boosts.unwrap();
        assert!((boosts.title - 3.0).abs() < f32::EPSILON);
        assert!((boosts.description - 2.0).abs() < f32::EPSILON);
        assert!((boosts.content - 1.0).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_health_backward_compatible() {
        // No backends, no config -- should produce the same shape as before
        let tools = HealthTools::new("legacy-server", "1.0.0", 3);
        let future = tools.call("health", json!({})).unwrap();
        let result = future.await.unwrap();

        let content_text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        let response: HealthResponse = serde_json::from_str(&content_text).unwrap();

        assert_eq!(response.status, "healthy");
        assert_eq!(response.server_name, "legacy-server");
        assert_eq!(response.version, "1.0.0");
        assert_eq!(response.tool_count, 3);
        assert!(response.backends.is_none());
        assert!(response.search_config.is_none());
    }

    #[test]
    fn test_health_response_skips_none_fields() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            server_name: "test".to_string(),
            version: "1.0".to_string(),
            tool_count: 1,
            backends: None,
            search_config: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("backends"));
        assert!(!json.contains("search_config"));
    }

    #[test]
    fn test_backend_info_skips_none_fields() {
        let info = BackendInfo {
            name: "test".to_string(),
            kind: "fts".to_string(),
            ready: true,
            document_count: None,
            last_indexed: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(!json.contains("document_count"));
        assert!(!json.contains("last_indexed"));
    }

    #[test]
    fn test_search_config_info_skips_none_boosts() {
        let config = SearchConfigInfo {
            query_mode: "and".to_string(),
            stopwords_enabled: false,
            fuzzy_search: true,
            field_boosts: None,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(!json.contains("field_boosts"));
    }

    #[tokio::test]
    async fn test_health_with_unready_backend() {
        let probes: Vec<Arc<dyn BackendProbe>> = vec![Arc::new(
            MockProbe::new("failing-backend", "fts").with_ready(false),
        )];

        let tools = HealthTools::new("test-server", "0.2.0", 5).with_backends(probes);
        let response = tools.build_response();

        let backends = response.backends.unwrap();
        assert_eq!(backends.services.len(), 1);
        assert!(!backends.services[0].ready);
    }

    #[tokio::test]
    async fn test_handle_health_enriched() {
        let probes: Vec<Arc<dyn BackendProbe>> = vec![Arc::new(
            MockProbe::new("tantivy", "fts").with_document_count(100),
        )];

        let config = SearchConfigInfo {
            query_mode: "smart".to_string(),
            stopwords_enabled: true,
            fuzzy_search: false,
            field_boosts: None,
        };

        let result = handle_health_enriched("enriched-server", "0.3.0", 7, &probes, Some(&config))
            .await
            .unwrap();
        assert_eq!(result.is_error, Some(false));

        let content_text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        let response: HealthResponse = serde_json::from_str(&content_text).unwrap();

        assert_eq!(response.server_name, "enriched-server");
        assert!(response.backends.is_some());
        assert!(response.search_config.is_some());
    }

    #[tokio::test]
    async fn test_handle_health_enriched_no_backends() {
        let result = handle_health_enriched("server", "1.0", 5, &[], None)
            .await
            .unwrap();

        let content_text = match &result.content[0].raw {
            rmcp::model::RawContent::Text(t) => t.text.clone(),
            _ => panic!("Expected text content"),
        };
        let response: HealthResponse = serde_json::from_str(&content_text).unwrap();

        assert!(response.backends.is_none());
        assert!(response.search_config.is_none());
    }

    #[test]
    fn test_health_response_full_roundtrip() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            server_name: "roundtrip-server".to_string(),
            version: "0.5.0".to_string(),
            tool_count: 8,
            backends: Some(BackendsInfo {
                services: vec![
                    BackendInfo {
                        name: "tantivy".to_string(),
                        kind: "fts".to_string(),
                        ready: true,
                        document_count: Some(200),
                        last_indexed: Some("2026-04-02T10:00:00Z".to_string()),
                    },
                    BackendInfo {
                        name: "lancedb".to_string(),
                        kind: "vector".to_string(),
                        ready: true,
                        document_count: Some(200),
                        last_indexed: None,
                    },
                ],
            }),
            search_config: Some(SearchConfigInfo {
                query_mode: "smart".to_string(),
                stopwords_enabled: true,
                fuzzy_search: true,
                field_boosts: Some(FieldBoosts {
                    title: 3.0,
                    description: 2.0,
                    content: 1.0,
                }),
            }),
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        let deserialized: HealthResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.server_name, "roundtrip-server");
        assert_eq!(deserialized.backends.as_ref().unwrap().services.len(), 2);
        assert_eq!(
            deserialized.search_config.as_ref().unwrap().query_mode,
            "smart"
        );
    }

    #[test]
    fn test_build_response_no_backends() {
        let tools = HealthTools::new("server", "1.0", 3);
        let response = tools.build_response();
        assert_eq!(response.status, "healthy");
        assert!(response.backends.is_none());
        assert!(response.search_config.is_none());
    }

    #[test]
    fn test_build_response_with_backends() {
        let probes: Vec<Arc<dyn BackendProbe>> = vec![
            Arc::new(MockProbe::new("tantivy", "fts")),
            Arc::new(MockProbe::new("simple", "vector").with_document_count(10)),
        ];

        let tools = HealthTools::new("server", "1.0", 5).with_backends(probes);
        let response = tools.build_response();

        let backends = response.backends.unwrap();
        assert_eq!(backends.services.len(), 2);
        assert_eq!(backends.services[0].name, "tantivy");
        assert_eq!(backends.services[1].document_count, Some(10));
    }
}
