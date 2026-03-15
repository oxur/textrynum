//! MCP tool schema validation.
//!
//! Validates that tool definitions conform to the MCP specification.
//! Use [`validate_tools`] at startup to catch schema issues early,
//! or [`assert_tools_valid`] in tests.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp::validate::assert_tools_valid;
//!
//! #[test]
//! fn test_all_tools_have_valid_schemas() {
//!     let registry = build_my_registry();
//!     assert_tools_valid(&registry);
//! }
//! ```

use rmcp::model::Tool;

/// A single validation issue found in a tool definition.
#[derive(Debug, Clone)]
pub struct ToolIssue {
    /// The tool name (or "<unnamed>" if missing).
    pub tool_name: String,
    /// What's wrong.
    pub message: String,
}

impl std::fmt::Display for ToolIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tool '{}': {}", self.tool_name, self.message)
    }
}

/// Validate a single tool definition against MCP spec requirements.
///
/// Checks:
/// - Name is 1–128 characters
/// - Name uses only allowed characters (A-Z, a-z, 0-9, _, -, .)
/// - Name contains no spaces, commas, or special characters
/// - `inputSchema` has `"type": "object"`
/// - Description is present and non-empty
pub fn validate_tool(tool: &Tool) -> Vec<ToolIssue> {
    let name = tool.name.to_string();
    let display_name = if name.is_empty() {
        "<unnamed>".to_string()
    } else {
        name.clone()
    };

    let mut issues = Vec::new();

    // --- Name checks (SHOULD per spec) ---
    if name.is_empty() {
        issues.push(ToolIssue {
            tool_name: display_name.clone(),
            message: "name is empty".into(),
        });
    } else if name.len() > 128 {
        issues.push(ToolIssue {
            tool_name: display_name.clone(),
            message: format!("name exceeds 128 characters ({})", name.len()),
        });
    }

    if !name.is_empty()
        && !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    {
        issues.push(ToolIssue {
            tool_name: display_name.clone(),
            message: format!(
                "name contains invalid characters (allowed: A-Z, a-z, 0-9, _, -, .): '{name}'"
            ),
        });
    }

    // --- inputSchema checks (MUST per spec) ---
    let schema = tool.input_schema.as_ref();

    if schema.is_empty() {
        issues.push(ToolIssue {
            tool_name: display_name.clone(),
            message: "inputSchema is empty ({}); MUST have at least {\"type\": \"object\"}".into(),
        });
    } else {
        match schema.get("type") {
            None => {
                issues.push(ToolIssue {
                    tool_name: display_name.clone(),
                    message: "inputSchema missing \"type\" field; MUST have \"type\": \"object\""
                        .into(),
                });
            }
            Some(serde_json::Value::String(t)) if t != "object" => {
                issues.push(ToolIssue {
                    tool_name: display_name.clone(),
                    message: format!(
                        "inputSchema \"type\" is \"{t}\"; MUST be \"object\" for MCP tools"
                    ),
                });
            }
            Some(serde_json::Value::String(_)) => { /* "object" — OK */ }
            Some(other) => {
                issues.push(ToolIssue {
                    tool_name: display_name.clone(),
                    message: format!(
                        "inputSchema \"type\" is not a string: {other}; MUST be \"object\""
                    ),
                });
            }
        }

        // If "properties" is present, check it's an object
        if let Some(props) = schema.get("properties")
            && !props.is_object()
        {
            issues.push(ToolIssue {
                tool_name: display_name.clone(),
                message: format!("inputSchema \"properties\" is not an object: {}", props),
            });
        }

        // If "required" is present, check it's an array of strings
        if let Some(required) = schema.get("required") {
            match required {
                serde_json::Value::Array(arr) => {
                    for (i, item) in arr.iter().enumerate() {
                        if !item.is_string() {
                            issues.push(ToolIssue {
                                tool_name: display_name.clone(),
                                message: format!(
                                    "inputSchema \"required\"[{i}] is not a string: {item}"
                                ),
                            });
                        }
                    }
                }
                _ => {
                    issues.push(ToolIssue {
                        tool_name: display_name.clone(),
                        message: "inputSchema \"required\" is not an array".into(),
                    });
                }
            }
        }
    }

    // --- Description check (SHOULD per spec) ---
    match &tool.description {
        None => {
            issues.push(ToolIssue {
                tool_name: display_name.clone(),
                message: "description is missing".into(),
            });
        }
        Some(d) if d.is_empty() => {
            issues.push(ToolIssue {
                tool_name: display_name,
                message: "description is empty".into(),
            });
        }
        _ => {}
    }

    issues
}

/// Validate all tools from a [`ToolRegistry`](crate::ToolRegistry).
///
/// Returns a list of all issues found across all tools. An empty vec means
/// all tools are valid.
pub fn validate_tools(registry: &dyn crate::ToolRegistry) -> Vec<ToolIssue> {
    let tools = registry.tools();
    let mut all_issues = Vec::new();

    // Per-tool validation
    for tool in &tools {
        all_issues.extend(validate_tool(tool));
    }

    // Cross-tool: check for duplicate names
    let mut seen = std::collections::HashSet::new();
    for tool in &tools {
        let name = tool.name.to_string();
        if !name.is_empty() && !seen.insert(name.clone()) {
            all_issues.push(ToolIssue {
                tool_name: name.clone(),
                message: format!("duplicate tool name: '{name}'"),
            });
        }
    }

    all_issues
}

/// Assert that all tools in a registry pass MCP validation.
///
/// Panics with a detailed message listing all issues if any tool is invalid.
/// Intended for use in tests.
///
/// # Example
///
/// ```rust,ignore
/// #[test]
/// fn test_tools_are_mcp_compliant() {
///     let registry = CompositeRegistry::new()
///         .add(MyTools::new());
///     fabryk_mcp::validate::assert_tools_valid(&registry);
/// }
/// ```
pub fn assert_tools_valid(registry: &dyn crate::ToolRegistry) {
    let issues = validate_tools(registry);
    if !issues.is_empty() {
        let report = issues
            .iter()
            .map(|i| format!("  - {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        panic!(
            "MCP tool validation failed ({} issue{}):\n{report}",
            issues.len(),
            if issues.len() == 1 { "" } else { "s" },
        );
    }
}

/// Log warnings for any tool validation issues.
///
/// Call at server startup to surface problems without crashing.
/// Returns the number of issues found.
pub fn warn_on_invalid_tools(registry: &dyn crate::ToolRegistry) -> usize {
    let issues = validate_tools(registry);
    for issue in &issues {
        log::warn!("MCP schema issue: {issue}");
    }
    issues.len()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool_with_schema(
        name: &str,
        description: Option<&str>,
        schema: serde_json::Map<String, serde_json::Value>,
    ) -> Tool {
        Tool::new_with_raw(
            name.to_string(),
            description.map(|d| d.to_string().into()),
            schema,
        )
    }

    fn valid_schema() -> serde_json::Map<String, serde_json::Value> {
        crate::empty_input_schema()
    }

    fn schema_with_props() -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        let mut props = serde_json::Map::new();
        props.insert(
            "query".to_string(),
            serde_json::json!({"type": "string", "description": "Search query"}),
        );
        m.insert("properties".to_string(), serde_json::Value::Object(props));
        m.insert("required".to_string(), serde_json::json!(["query"]));
        m
    }

    // --- Valid tools ---

    #[test]
    fn test_valid_no_params_tool() {
        let tool = make_tool_with_schema("health", Some("Check health"), valid_schema());
        let issues = validate_tool(&tool);
        assert!(issues.is_empty(), "Expected no issues, got: {issues:?}");
    }

    #[test]
    fn test_valid_tool_with_properties() {
        let tool = make_tool_with_schema("search", Some("Search things"), schema_with_props());
        let issues = validate_tool(&tool);
        assert!(issues.is_empty(), "Expected no issues, got: {issues:?}");
    }

    #[test]
    fn test_valid_name_with_allowed_chars() {
        for name in ["get_user", "DATA_EXPORT_v2", "admin.tools.list", "my-tool"] {
            let tool = make_tool_with_schema(name, Some("A tool"), valid_schema());
            let issues = validate_tool(&tool);
            assert!(
                issues.is_empty(),
                "Name '{name}' should be valid, got: {issues:?}"
            );
        }
    }

    // --- inputSchema issues ---

    #[test]
    fn test_empty_schema_is_invalid() {
        let tool = make_tool_with_schema("bad", Some("Bad tool"), serde_json::Map::new());
        let issues = validate_tool(&tool);
        assert!(
            issues.iter().any(|i| i.message.contains("empty")),
            "Expected empty schema error, got: {issues:?}"
        );
    }

    #[test]
    fn test_missing_type_field() {
        let mut schema = serde_json::Map::new();
        schema.insert("properties".to_string(), serde_json::json!({}));
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("missing \"type\"")),
            "Expected missing type error, got: {issues:?}"
        );
    }

    #[test]
    fn test_wrong_type_value() {
        let mut schema = serde_json::Map::new();
        schema.insert(
            "type".to_string(),
            serde_json::Value::String("array".to_string()),
        );
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("MUST be \"object\"")),
            "Expected wrong type error, got: {issues:?}"
        );
    }

    #[test]
    fn test_type_not_a_string() {
        let mut schema = serde_json::Map::new();
        schema.insert("type".to_string(), serde_json::json!(42));
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues.iter().any(|i| i.message.contains("not a string")),
            "Expected non-string type error, got: {issues:?}"
        );
    }

    #[test]
    fn test_properties_not_an_object() {
        let mut schema = valid_schema();
        schema.insert("properties".to_string(), serde_json::json!("bad"));
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("\"properties\" is not an object")),
            "Expected bad properties error, got: {issues:?}"
        );
    }

    #[test]
    fn test_required_not_an_array() {
        let mut schema = valid_schema();
        schema.insert("required".to_string(), serde_json::json!("query"));
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("\"required\" is not an array")),
            "Expected bad required error, got: {issues:?}"
        );
    }

    #[test]
    fn test_required_contains_non_string() {
        let mut schema = valid_schema();
        schema.insert("required".to_string(), serde_json::json!(["ok", 42]));
        let tool = make_tool_with_schema("bad", Some("Bad tool"), schema);
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("required\"[1] is not a string")),
            "Expected non-string required entry, got: {issues:?}"
        );
    }

    // --- Name issues ---

    #[test]
    fn test_empty_name() {
        let tool = make_tool_with_schema("", Some("No name"), valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues.iter().any(|i| i.message.contains("name is empty")),
            "Expected empty name error, got: {issues:?}"
        );
    }

    #[test]
    fn test_name_too_long() {
        let long_name = "a".repeat(129);
        let tool = make_tool_with_schema(&long_name, Some("Too long"), valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues.iter().any(|i| i.message.contains("exceeds 128")),
            "Expected name length error, got: {issues:?}"
        );
    }

    #[test]
    fn test_name_with_spaces() {
        let tool = make_tool_with_schema("my tool", Some("Spacey"), valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("invalid characters")),
            "Expected invalid char error, got: {issues:?}"
        );
    }

    #[test]
    fn test_name_with_special_chars() {
        let tool = make_tool_with_schema("tool@v2!", Some("Special"), valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("invalid characters")),
            "Expected invalid char error, got: {issues:?}"
        );
    }

    // --- Description issues ---

    #[test]
    fn test_missing_description() {
        let tool = make_tool_with_schema("nodesc", None, valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("description is missing")),
            "Expected missing description, got: {issues:?}"
        );
    }

    #[test]
    fn test_empty_description() {
        let tool = make_tool_with_schema("emptydesc", Some(""), valid_schema());
        let issues = validate_tool(&tool);
        assert!(
            issues
                .iter()
                .any(|i| i.message.contains("description is empty")),
            "Expected empty description, got: {issues:?}"
        );
    }

    // --- Cross-tool validation ---

    #[test]
    fn test_duplicate_names() {
        use crate::ToolRegistry;

        struct DupeRegistry;
        impl ToolRegistry for DupeRegistry {
            fn tools(&self) -> Vec<Tool> {
                vec![
                    make_tool_with_schema("search", Some("First"), valid_schema()),
                    make_tool_with_schema("search", Some("Duplicate"), valid_schema()),
                ]
            }
            fn call(&self, _name: &str, _args: serde_json::Value) -> Option<crate::ToolResult> {
                None
            }
        }

        let issues = validate_tools(&DupeRegistry);
        assert!(
            issues.iter().any(|i| i.message.contains("duplicate")),
            "Expected duplicate name error, got: {issues:?}"
        );
    }

    // --- assert_tools_valid ---

    #[test]
    fn test_assert_tools_valid_passes_for_valid_registry() {
        use crate::ToolRegistry;

        struct GoodRegistry;
        impl ToolRegistry for GoodRegistry {
            fn tools(&self) -> Vec<Tool> {
                vec![
                    make_tool_with_schema("health", Some("Health check"), valid_schema()),
                    make_tool_with_schema("search", Some("Search"), schema_with_props()),
                ]
            }
            fn call(&self, _name: &str, _args: serde_json::Value) -> Option<crate::ToolResult> {
                None
            }
        }

        assert_tools_valid(&GoodRegistry); // should not panic
    }

    #[test]
    #[should_panic(expected = "MCP tool validation failed")]
    fn test_assert_tools_valid_panics_for_invalid() {
        use crate::ToolRegistry;

        struct BadRegistry;
        impl ToolRegistry for BadRegistry {
            fn tools(&self) -> Vec<Tool> {
                vec![make_tool_with_schema(
                    "bad",
                    Some("Bad"),
                    serde_json::Map::new(),
                )]
            }
            fn call(&self, _name: &str, _args: serde_json::Value) -> Option<crate::ToolResult> {
                None
            }
        }

        assert_tools_valid(&BadRegistry); // should panic
    }
}
