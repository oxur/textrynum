//! Reusable MCP helper functions.
//!
//! Common utilities for building MCP tools:
//!
//! - [`make_tool`] — Construct a [`Tool`] from name, description, and JSON schema
//! - [`make_tool_no_params`] — Construct a [`Tool`] that takes no parameters
//! - [`serialize_response`] — Serialize any `T: Serialize` into a [`CallToolResult`]
//! - [`tier_confidence_schema`] — Standard metadata filter schema (tier + confidence)

use rmcp::model::{CallToolResult, Content, ErrorData, Tool};
use serde::Serialize;

/// Construct a [`Tool`] from a name, description, and JSON schema value.
///
/// The `schema` parameter should be a `serde_json::Value::Object` representing
/// the JSON Schema for the tool's input. If it's not an object, an empty schema
/// is used.
///
/// # Examples
///
/// ```
/// use fabryk_mcp_core::helpers::make_tool;
/// use serde_json::json;
///
/// let tool = make_tool("my_tool", "Does something useful", json!({
///     "type": "object",
///     "properties": {
///         "query": { "type": "string" }
///     },
///     "required": ["query"]
/// }));
/// assert_eq!(tool.name, "my_tool");
/// ```
pub fn make_tool(name: &str, description: &str, schema: serde_json::Value) -> Tool {
    let input_schema = match schema {
        serde_json::Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    Tool::new(name.to_string(), description.to_string(), input_schema)
}

/// Construct a [`Tool`] that takes no parameters.
///
/// Uses [`crate::empty_input_schema`] for the schema.
pub fn make_tool_no_params(name: &str, description: &str) -> Tool {
    Tool::new(
        name.to_string(),
        description.to_string(),
        crate::empty_input_schema(),
    )
}

/// Serialize any `T: Serialize` into a successful [`CallToolResult`] with
/// pretty-printed JSON content.
///
/// Returns an `ErrorData` with `INTERNAL_ERROR` code if serialization fails.
pub fn serialize_response<T: Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(format!("Serialization error: {}", e), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Standard metadata filter schema for knowledge-base graph tools.
///
/// Returns a JSON value containing `tier` (foundational/intermediate/advanced)
/// and `min_confidence` (high/medium/low) filter properties. These are generic
/// knowledge-management tiers applicable to any content system with prerequisite
/// depth and extraction confidence metadata.
pub fn tier_confidence_schema() -> serde_json::Value {
    serde_json::json!({
        "tier": {
            "type": "string",
            "enum": ["foundational", "intermediate", "advanced"],
            "description": "Filter results by prerequisite depth tier"
        },
        "min_confidence": {
            "type": "string",
            "enum": ["high", "medium", "low"],
            "description": "Minimum extraction confidence threshold"
        }
    })
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_make_tool_with_schema() {
        let tool = make_tool(
            "test_tool",
            "A test tool",
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
        );
        assert_eq!(tool.name, "test_tool");
        assert_eq!(tool.description.as_deref(), Some("A test tool"));
        assert!(tool.input_schema.contains_key("properties"));
    }

    #[test]
    fn test_make_tool_non_object_schema() {
        let tool = make_tool("test_tool", "A test", json!("not an object"));
        assert_eq!(tool.name, "test_tool");
        // Should get empty schema, not panic
        assert!(tool.input_schema.is_empty() || tool.input_schema.contains_key("type"));
    }

    #[test]
    fn test_make_tool_no_params() {
        let tool = make_tool_no_params("simple_tool", "No params needed");
        assert_eq!(tool.name, "simple_tool");
        assert!(tool.input_schema.contains_key("type"));
    }

    #[test]
    fn test_serialize_response() {
        #[derive(Serialize)]
        struct TestData {
            value: u32,
        }
        let data = TestData { value: 42 };
        let result = serialize_response(&data).expect("serialization should succeed");
        assert!(!result.is_error.unwrap_or(false));
    }

    #[test]
    fn test_tier_confidence_schema_structure() {
        let schema = tier_confidence_schema();
        let obj = schema.as_object().expect("schema should be an object");
        assert!(obj.contains_key("tier"));
        assert!(obj.contains_key("min_confidence"));

        let tier = obj["tier"].as_object().unwrap();
        assert_eq!(tier["type"], "string");
        let tier_enum = tier["enum"].as_array().unwrap();
        assert_eq!(tier_enum.len(), 3);

        let conf = obj["min_confidence"].as_object().unwrap();
        assert_eq!(conf["type"], "string");
        let conf_enum = conf["enum"].as_array().unwrap();
        assert_eq!(conf_enum.len(), 3);
    }
}
