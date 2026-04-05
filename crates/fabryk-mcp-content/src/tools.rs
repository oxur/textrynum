//! Generic MCP tools for content operations.
//!
//! Provides `ContentTools<P>` and `SourceTools<P>` that implement
//! `ToolRegistry` by delegating to domain-specific providers.

use crate::traits::{ContentItemProvider, GuideProvider, QuestionSearchProvider, SourceProvider};
use fabryk_mcp_core::error::McpErrorExt;
use fabryk_mcp_core::model::{CallToolResult, Content, ErrorData, Tool};
use fabryk_mcp_core::registry::{ToolRegistry, ToolResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Convert a `serde_json::Value::Object` to an `Arc<serde_json::Map>`.
fn json_schema(value: Value) -> Arc<serde_json::Map<String, Value>> {
    match value {
        Value::Object(map) => Arc::new(map),
        _ => Arc::new(serde_json::Map::new()),
    }
}

/// Serialize a value to a successful `CallToolResult`.
fn serialize_response<T: serde::Serialize>(value: &T) -> Result<CallToolResult, ErrorData> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Extract a required string field from a JSON `Value`, returning an MCP error
/// if the field is missing or not a string.
fn extract_string_field(args: &Value, field: &str) -> Result<String, ErrorData> {
    args.get(field)
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| {
            ErrorData::invalid_params(format!("Missing required parameter: {field}"), None)
        })
}

/// Build a `Tool` with a JSON schema.
fn make_tool(name: &str, description: &str, schema: Value) -> Tool {
    Tool::new(
        name.to_string(),
        description.to_string(),
        json_schema(schema),
    )
}

// ---------------------------------------------------------------------------
// Argument types
// ---------------------------------------------------------------------------

/// Arguments for list_items tool.
#[derive(Debug, Deserialize)]
pub struct ListItemsArgs {
    /// Optional category filter.
    pub category: Option<String>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Arguments for get_item tool.
#[derive(Debug, Deserialize)]
pub struct GetItemArgs {
    /// Item identifier.
    pub id: String,
}

/// Arguments for get_chapter tool.
#[derive(Debug, Deserialize)]
pub struct GetChapterArgs {
    /// Source identifier.
    pub source_id: String,
    /// Chapter identifier.
    pub chapter: String,
    /// Optional section within chapter.
    pub section: Option<String>,
}

/// Arguments for list_chapters tool.
#[derive(Debug, Deserialize)]
pub struct ListChaptersArgs {
    /// Source identifier.
    pub source_id: String,
}

/// Arguments for check_availability tool.
#[derive(Debug, Deserialize)]
pub struct CheckAvailabilityArgs {
    /// Source identifier.
    pub source_id: String,
}

/// Arguments for get_path tool.
#[derive(Debug, Deserialize)]
pub struct GetPathArgs {
    /// Source identifier.
    pub source_id: String,
}

/// Arguments for get_guide tool.
#[derive(Debug, Deserialize)]
pub struct GetGuideArgs {
    /// Guide identifier.
    pub id: String,
}

/// Arguments for the search_by_question tool.
#[derive(Debug, Deserialize)]
pub struct SearchByQuestionArgs {
    /// The question to search for.
    pub question: String,
    /// Maximum number of results (default: 10).
    pub limit: Option<usize>,
}

// ---------------------------------------------------------------------------
// ContentTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `ContentItemProvider`.
///
/// Generates three tools:
/// - `{prefix}_list` — list items with optional category filter
/// - `{prefix}_get` — get a specific item by ID
/// - `{prefix}_categories` — list available categories
///
/// # Example
///
/// ```rust,ignore
/// let tools = ContentTools::new(my_provider).with_prefix("concepts");
/// // Generates: concepts_list, concepts_get, concepts_categories
/// ```
pub struct ContentTools<P: ContentItemProvider> {
    provider: Arc<P>,
    tool_prefix: String,
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
    extra_list_schema: Option<serde_json::Value>,
    get_id_field: Option<String>,
}

impl<P: ContentItemProvider + 'static> ContentTools<P> {
    /// Slot key for the list tool.
    pub const SLOT_LIST: &str = "list";
    /// Slot key for the get tool.
    pub const SLOT_GET: &str = "get";
    /// Slot key for the categories tool.
    pub const SLOT_CATEGORIES: &str = "categories";

    /// Create new content tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            tool_prefix: String::new(),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            extra_list_schema: None,
            get_id_field: None,
        }
    }

    /// Create content tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self {
            provider,
            tool_prefix: String::new(),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
            extra_list_schema: None,
            get_id_field: None,
        }
    }

    /// Set a prefix for tool names (e.g., "concepts" → "concepts_list").
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.tool_prefix = prefix.into();
        self
    }

    /// Override individual tool names.
    ///
    /// Keys are slot constants (`SLOT_LIST`, etc.), values are custom
    /// MCP-visible names. Unspecified slots keep their default names.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override individual tool descriptions.
    ///
    /// Keys are slot constants, values are custom descriptions.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    /// Provide extra JSON Schema properties to merge into the "list" tool schema.
    ///
    /// The value should be a JSON object whose keys are property names and
    /// values are JSON Schema definitions (e.g., `{"tier": {"type": "string"}}`).
    /// These are merged into the `properties` of the list tool's input schema
    /// and are forwarded to
    /// [`ContentItemProvider::list_items_filtered`](crate::traits::ContentItemProvider::list_items_filtered)
    /// at call time.
    pub fn with_extra_list_schema(mut self, schema: serde_json::Value) -> Self {
        self.extra_list_schema = Some(schema);
        self
    }

    /// Override the JSON property name used by the "get" tool.
    ///
    /// By default the get tool expects an `"id"` property in its input
    /// schema. Use this to change it to a domain-specific name such as
    /// `"concept_id"`. Both the schema and the call handler will use
    /// this name.
    pub fn with_get_id_field(mut self, field_name: impl Into<String>) -> Self {
        self.get_id_field = Some(field_name.into());
        self
    }

    /// Returns the configured get-tool ID field name, defaulting to `"id"`.
    fn get_id_field_name(&self) -> &str {
        self.get_id_field.as_deref().unwrap_or("id")
    }

    fn tool_name(&self, slot: &str) -> String {
        if let Some(custom) = self.custom_names.get(slot) {
            return custom.clone();
        }
        if self.tool_prefix.is_empty() {
            slot.to_string()
        } else {
            format!("{}_{}", self.tool_prefix, slot)
        }
    }

    fn tool_description(&self, slot: &str, default: &str) -> String {
        self.custom_descriptions
            .get(slot)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl<P: ContentItemProvider + 'static> ToolRegistry for ContentTools<P> {
    fn tools(&self) -> Vec<Tool> {
        let type_name = self.provider.content_type_name();
        let type_plural = self.provider.content_type_name_plural();

        let mut list_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "category": {
                    "type": "string",
                    "description": "Filter by category"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results"
                }
            }
        });

        // Merge extra schema properties into the list tool's properties.
        if let Some(extra) = &self.extra_list_schema
            && let (Some(props), Some(extra_props)) =
                (list_schema.pointer_mut("/properties"), extra.as_object())
            && let Some(map) = props.as_object_mut()
        {
            for (key, value) in extra_props {
                map.insert(key.clone(), value.clone());
            }
        }

        vec![
            make_tool(
                &self.tool_name("list"),
                &self.tool_description(
                    "list",
                    &format!("List all {type_plural} with optional category filter"),
                ),
                list_schema,
            ),
            {
                let id_field = self.get_id_field_name();
                let mut props = serde_json::Map::new();
                props.insert(
                    id_field.to_string(),
                    serde_json::json!({
                        "type": "string",
                        "description": format!(
                            "{type_name} slug identifier, e.g. \"voice-leading\" (use the id values returned by {type_plural}_list)",
                            type_name = type_name,
                            type_plural = type_plural,
                        )
                    }),
                );
                make_tool(
                    &self.tool_name("get"),
                    &self.tool_description("get", &format!(
                        "Get a specific {type_name} by its slug identifier (the {id_field} field from {type_plural}_list results)"
                    )),
                    serde_json::json!({
                        "type": "object",
                        "properties": props,
                        "required": [id_field]
                    }),
                )
            },
            make_tool(
                &self.tool_name("categories"),
                &self.tool_description(
                    "categories",
                    &format!("List available {type_name} categories"),
                ),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name("list") {
            return Some(Box::pin(async move {
                let mut obj = match args {
                    Value::Object(map) => map,
                    _ => serde_json::Map::new(),
                };

                let category = obj
                    .remove("category")
                    .and_then(|v| v.as_str().map(String::from));
                let limit = obj
                    .remove("limit")
                    .and_then(|v| v.as_u64().map(|n| n as usize));

                // Remaining fields become the extra filter map.
                let extra_filters = obj;

                let items = provider
                    .list_items_filtered(category.as_deref(), limit, &extra_filters)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&items)
            }));
        }

        if name == self.tool_name("get") {
            let id_field = self.get_id_field_name().to_string();
            return Some(Box::pin(async move {
                let id = extract_string_field(&args, &id_field)?;
                let item = provider.get_item(&id).await.map_err(|e| e.to_mcp_error())?;
                serialize_response(&item)
            }));
        }

        if name == self.tool_name("categories") {
            return Some(Box::pin(async move {
                let categories = provider
                    .list_categories()
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&categories)
            }));
        }

        None
    }
}

// ---------------------------------------------------------------------------
// Source response types
// ---------------------------------------------------------------------------

/// Response for the check-availability tool.
#[derive(Debug, Serialize)]
struct AvailabilityResponse {
    source_id: String,
    available: bool,
}

/// Response for the get-path tool.
#[derive(Debug, Serialize)]
struct PathResponse {
    source_id: String,
    path: Option<String>,
}

// ---------------------------------------------------------------------------
// SourceTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `SourceProvider`.
///
/// Generates five tools:
/// - `sources_list` — list all source materials
/// - `sources_chapters` — list chapters in a source
/// - `sources_get_chapter` — get content from a source chapter
/// - `sources_check_availability` — check if a source is available
/// - `sources_get_path` — get filesystem path to a source
pub struct SourceTools<P: SourceProvider> {
    provider: Arc<P>,
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
}

impl<P: SourceProvider + 'static> SourceTools<P> {
    /// Slot key for the list sources tool.
    pub const SLOT_LIST: &str = "sources_list";
    /// Slot key for the list chapters tool.
    pub const SLOT_CHAPTERS: &str = "sources_chapters";
    /// Slot key for the get chapter tool.
    pub const SLOT_GET_CHAPTER: &str = "sources_get_chapter";
    /// Slot key for the check availability tool.
    pub const SLOT_CHECK_AVAILABILITY: &str = "sources_check_availability";
    /// Slot key for the get path tool.
    pub const SLOT_GET_PATH: &str = "sources_get_path";

    /// Create new source tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Create source tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self {
            provider,
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Override individual tool names.
    ///
    /// Keys are slot constants (`SLOT_LIST`, etc.), values are custom
    /// MCP-visible names. Unspecified slots keep their default names.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override individual tool descriptions.
    ///
    /// Keys are slot constants, values are custom descriptions.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    fn tool_name(&self, slot: &str) -> String {
        self.custom_names
            .get(slot)
            .cloned()
            .unwrap_or_else(|| slot.to_string())
    }

    fn tool_description(&self, slot: &str, default: &str) -> String {
        self.custom_descriptions
            .get(slot)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl<P: SourceProvider + 'static> ToolRegistry for SourceTools<P> {
    fn tools(&self) -> Vec<Tool> {
        vec![
            make_tool(
                &self.tool_name("sources_list"),
                &self.tool_description("sources_list", "List all source materials"),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            make_tool(
                &self.tool_name("sources_chapters"),
                &self.tool_description("sources_chapters", "List chapters in a source"),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        }
                    },
                    "required": ["source_id"]
                }),
            ),
            make_tool(
                &self.tool_name("sources_get_chapter"),
                &self.tool_description("sources_get_chapter", "Get content from a source chapter"),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        },
                        "chapter": {
                            "type": "string",
                            "description": "Chapter identifier"
                        },
                        "section": {
                            "type": "string",
                            "description": "Optional section within chapter"
                        }
                    },
                    "required": ["source_id", "chapter"]
                }),
            ),
            make_tool(
                &self.tool_name("sources_check_availability"),
                &self.tool_description(
                    "sources_check_availability",
                    "Check if a source is available",
                ),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        }
                    },
                    "required": ["source_id"]
                }),
            ),
            make_tool(
                &self.tool_name("sources_get_path"),
                &self.tool_description("sources_get_path", "Get filesystem path to a source"),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "source_id": {
                            "type": "string",
                            "description": "Source identifier"
                        }
                    },
                    "required": ["source_id"]
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name(Self::SLOT_LIST) {
            return Some(Box::pin(async move {
                let sources = provider
                    .list_sources()
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&sources)
            }));
        }

        if name == self.tool_name(Self::SLOT_CHAPTERS) {
            return Some(Box::pin(async move {
                let args: ListChaptersArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let chapters = provider
                    .list_chapters(&args.source_id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&chapters)
            }));
        }

        if name == self.tool_name(Self::SLOT_GET_CHAPTER) {
            return Some(Box::pin(async move {
                let args: GetChapterArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let content = provider
                    .get_chapter(&args.source_id, &args.chapter, args.section.as_deref())
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }));
        }

        if name == self.tool_name(Self::SLOT_CHECK_AVAILABILITY) {
            return Some(Box::pin(async move {
                let args: CheckAvailabilityArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let available = provider
                    .is_available(&args.source_id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&AvailabilityResponse {
                    source_id: args.source_id,
                    available,
                })
            }));
        }

        if name == self.tool_name(Self::SLOT_GET_PATH) {
            return Some(Box::pin(async move {
                let args: GetPathArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let path = provider
                    .get_source_path(&args.source_id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&PathResponse {
                    source_id: args.source_id,
                    path: path.map(|p| p.display().to_string()),
                })
            }));
        }

        None
    }
}

// ---------------------------------------------------------------------------
// GuideTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `GuideProvider`.
///
/// Generates two tools:
/// - `list_guides` — list all available guides
/// - `get_guide` — get a guide's full markdown content by ID
pub struct GuideTools<P: GuideProvider> {
    provider: Arc<P>,
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
}

impl<P: GuideProvider + 'static> GuideTools<P> {
    /// Slot key for the list guides tool.
    pub const SLOT_LIST: &str = "list_guides";
    /// Slot key for the get guide tool.
    pub const SLOT_GET: &str = "get_guide";

    /// Create new guide tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Create guide tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self {
            provider,
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Override individual tool names.
    ///
    /// Keys are slot constants (`SLOT_LIST`, etc.), values are custom
    /// MCP-visible names. Unspecified slots keep their default names.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override individual tool descriptions.
    ///
    /// Keys are slot constants, values are custom descriptions.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    fn tool_name(&self, slot: &str) -> String {
        self.custom_names
            .get(slot)
            .cloned()
            .unwrap_or_else(|| slot.to_string())
    }

    fn tool_description(&self, slot: &str, default: &str) -> String {
        self.custom_descriptions
            .get(slot)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl<P: GuideProvider + 'static> ToolRegistry for GuideTools<P> {
    fn tools(&self) -> Vec<Tool> {
        let type_name = self.provider.guide_type_name();
        let type_plural = self.provider.guide_type_name_plural();

        vec![
            make_tool(
                &self.tool_name(Self::SLOT_LIST),
                &self.tool_description(
                    Self::SLOT_LIST,
                    &format!("List all available {type_plural}"),
                ),
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            ),
            make_tool(
                &self.tool_name(Self::SLOT_GET),
                &self.tool_description(
                    Self::SLOT_GET,
                    &format!(
                        "Get a {type_name}'s full content by its identifier (the id field from {type_plural} list results)"
                    ),
                ),
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": format!(
                                "{type_name} identifier (use the id values returned by the list {type_plural} tool)",
                            )
                        }
                    },
                    "required": ["id"]
                }),
            ),
        ]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name(Self::SLOT_LIST) {
            return Some(Box::pin(async move {
                let guides = provider.list_guides().await.map_err(|e| e.to_mcp_error())?;
                serialize_response(&guides)
            }));
        }

        if name == self.tool_name(Self::SLOT_GET) {
            return Some(Box::pin(async move {
                let args: GetGuideArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let content = provider
                    .get_guide(&args.id)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                Ok(CallToolResult::success(vec![Content::text(content)]))
            }));
        }

        None
    }
}

// ---------------------------------------------------------------------------
// QuestionSearchTools<P>
// ---------------------------------------------------------------------------

/// MCP tools backed by a `QuestionSearchProvider`.
///
/// Generates one tool:
/// - `search_by_question` — fuzzy-match a query against competency questions
pub struct QuestionSearchTools<P: QuestionSearchProvider> {
    provider: Arc<P>,
    custom_names: HashMap<String, String>,
    custom_descriptions: HashMap<String, String>,
}

impl<P: QuestionSearchProvider + 'static> QuestionSearchTools<P> {
    /// Slot key for the search tool.
    pub const SLOT_SEARCH: &str = "search_by_question";

    /// Create new question search tools with the given provider.
    pub fn new(provider: P) -> Self {
        Self {
            provider: Arc::new(provider),
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Create question search tools with a shared provider reference.
    pub fn with_shared(provider: Arc<P>) -> Self {
        Self {
            provider,
            custom_names: HashMap::new(),
            custom_descriptions: HashMap::new(),
        }
    }

    /// Override individual tool names.
    ///
    /// Keys are slot constants (`SLOT_SEARCH`), values are custom
    /// MCP-visible names. Unspecified slots keep their default names.
    pub fn with_names(mut self, names: HashMap<String, String>) -> Self {
        self.custom_names = names;
        self
    }

    /// Override individual tool descriptions.
    ///
    /// Keys are slot constants, values are custom descriptions.
    pub fn with_descriptions(mut self, descriptions: HashMap<String, String>) -> Self {
        self.custom_descriptions = descriptions;
        self
    }

    fn tool_name(&self, slot: &str) -> String {
        self.custom_names
            .get(slot)
            .cloned()
            .unwrap_or_else(|| slot.to_string())
    }

    fn tool_description(&self, slot: &str, default: &str) -> String {
        self.custom_descriptions
            .get(slot)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

impl<P: QuestionSearchProvider + 'static> ToolRegistry for QuestionSearchTools<P> {
    fn tools(&self) -> Vec<Tool> {
        vec![make_tool(
            &self.tool_name(Self::SLOT_SEARCH),
            &self.tool_description(
                Self::SLOT_SEARCH,
                "Search for content items by matching a query against competency questions",
            ),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to search for"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default: 10)"
                    }
                },
                "required": ["question"]
            }),
        )]
    }

    fn call(&self, name: &str, args: Value) -> Option<ToolResult> {
        let provider = Arc::clone(&self.provider);

        if name == self.tool_name(Self::SLOT_SEARCH) {
            return Some(Box::pin(async move {
                let args: SearchByQuestionArgs = serde_json::from_value(args)
                    .map_err(|e| ErrorData::invalid_params(e.to_string(), None))?;
                let limit = args.limit.unwrap_or(10);
                let response = provider
                    .search_by_question(&args.question, limit)
                    .await
                    .map_err(|e| e.to_mcp_error())?;
                serialize_response(&response)
            }));
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
    use crate::traits::{CategoryInfo, ChapterInfo};
    use async_trait::async_trait;
    use serde::Serialize;
    use std::path::PathBuf;

    /// Serialize a `CallToolResult`'s content to a string for test assertions.
    fn result_to_string(result: &CallToolResult) -> String {
        serde_json::to_string(&result.content).unwrap()
    }

    // -- Mock content types -------------------------------------------------

    #[derive(Clone, Debug, Serialize)]
    struct MockItemSummary {
        id: String,
        title: String,
        category: Option<String>,
    }

    #[derive(Clone, Debug, Serialize)]
    struct MockItemDetail {
        id: String,
        title: String,
        content: String,
    }

    // -- Mock content provider ----------------------------------------------

    struct MockContentProvider;

    #[async_trait]
    impl ContentItemProvider for MockContentProvider {
        type ItemSummary = MockItemSummary;
        type ItemDetail = MockItemDetail;

        async fn list_items(
            &self,
            category: Option<&str>,
            limit: Option<usize>,
        ) -> fabryk_core::Result<Vec<Self::ItemSummary>> {
            let mut items = vec![
                MockItemSummary {
                    id: "item-1".to_string(),
                    title: "First Item".to_string(),
                    category: Some("alpha".to_string()),
                },
                MockItemSummary {
                    id: "item-2".to_string(),
                    title: "Second Item".to_string(),
                    category: Some("beta".to_string()),
                },
                MockItemSummary {
                    id: "item-3".to_string(),
                    title: "Third Item".to_string(),
                    category: Some("alpha".to_string()),
                },
            ];

            if let Some(cat) = category {
                items.retain(|i| i.category.as_deref() == Some(cat));
            }
            if let Some(max) = limit {
                items.truncate(max);
            }
            Ok(items)
        }

        async fn get_item(&self, id: &str) -> fabryk_core::Result<Self::ItemDetail> {
            if id == "item-1" {
                Ok(MockItemDetail {
                    id: "item-1".to_string(),
                    title: "First Item".to_string(),
                    content: "Detailed content here.".to_string(),
                })
            } else {
                Err(fabryk_core::Error::not_found("item", id))
            }
        }

        async fn list_categories(&self) -> fabryk_core::Result<Vec<CategoryInfo>> {
            Ok(vec![
                CategoryInfo {
                    id: "alpha".to_string(),
                    name: "Alpha".to_string(),
                    count: 2,
                    description: None,
                },
                CategoryInfo {
                    id: "beta".to_string(),
                    name: "Beta".to_string(),
                    count: 1,
                    description: None,
                },
            ])
        }

        fn content_type_name(&self) -> &str {
            "concept"
        }

        fn content_type_name_plural(&self) -> &str {
            "concepts"
        }
    }

    // -- Mock source types --------------------------------------------------

    #[derive(Clone, Debug, Serialize)]
    struct MockSourceSummary {
        id: String,
        title: String,
        available: bool,
    }

    // -- Mock source provider -----------------------------------------------

    struct MockSourceProvider;

    #[async_trait]
    impl SourceProvider for MockSourceProvider {
        type SourceSummary = MockSourceSummary;

        async fn list_sources(&self) -> fabryk_core::Result<Vec<Self::SourceSummary>> {
            Ok(vec![MockSourceSummary {
                id: "book-1".to_string(),
                title: "Test Book".to_string(),
                available: true,
            }])
        }

        async fn get_chapter(
            &self,
            source_id: &str,
            chapter: &str,
            _section: Option<&str>,
        ) -> fabryk_core::Result<String> {
            if source_id == "book-1" && chapter == "ch1" {
                Ok("Chapter 1 content.".to_string())
            } else {
                Err(fabryk_core::Error::not_found(
                    "chapter",
                    &format!("{source_id}:{chapter}"),
                ))
            }
        }

        async fn list_chapters(&self, source_id: &str) -> fabryk_core::Result<Vec<ChapterInfo>> {
            if source_id == "book-1" {
                Ok(vec![ChapterInfo {
                    id: "ch1".to_string(),
                    title: "Chapter 1".to_string(),
                    number: Some("1".to_string()),
                    available: true,
                }])
            } else {
                Err(fabryk_core::Error::not_found("source", source_id))
            }
        }

        async fn get_source_path(&self, _source_id: &str) -> fabryk_core::Result<Option<PathBuf>> {
            Ok(None)
        }
    }

    // -- ContentTools tests -------------------------------------------------

    #[test]
    fn test_content_tools_creation() {
        let tools = ContentTools::new(MockContentProvider);
        assert_eq!(tools.tool_count(), 3);
    }

    #[test]
    fn test_content_tools_with_prefix() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let tool_list = tools.tools();
        assert_eq!(tool_list.len(), 3);
        assert_eq!(tool_list[0].name, "concepts_list");
        assert_eq!(tool_list[1].name, "concepts_get");
        assert_eq!(tool_list[2].name, "concepts_categories");
    }

    #[test]
    fn test_content_tools_without_prefix() {
        let tools = ContentTools::new(MockContentProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "list");
        assert_eq!(tool_list[1].name, "get");
        assert_eq!(tool_list[2].name, "categories");
    }

    #[test]
    fn test_content_tools_descriptions() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let tool_list = tools.tools();
        assert!(
            tool_list[0]
                .description
                .as_ref()
                .unwrap()
                .contains("concepts")
        );
        assert!(
            tool_list[1]
                .description
                .as_ref()
                .unwrap()
                .contains("concept")
        );
    }

    #[test]
    fn test_content_tools_has_tool() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        assert!(tools.has_tool("concepts_list"));
        assert!(tools.has_tool("concepts_get"));
        assert!(tools.has_tool("concepts_categories"));
        assert!(!tools.has_tool("concepts_delete"));
    }

    #[tokio::test]
    async fn test_content_tools_list() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools.call("concepts_list", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_content_tools_list_with_category() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_list", serde_json::json!({"category": "alpha"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_list_with_limit() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_list", serde_json::json!({"limit": 1}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_get() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_get", serde_json::json!({"id": "item-1"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_get_not_found() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_get", serde_json::json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_content_tools_categories() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let future = tools
            .call("concepts_categories", serde_json::json!({}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_content_tools_unknown_tool() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        assert!(
            tools
                .call("concepts_delete", serde_json::json!({}))
                .is_none()
        );
    }

    // -- SourceTools tests --------------------------------------------------

    #[test]
    fn test_source_tools_creation() {
        let tools = SourceTools::new(MockSourceProvider);
        assert_eq!(tools.tool_count(), 5);
    }

    #[test]
    fn test_source_tools_names() {
        let tools = SourceTools::new(MockSourceProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "sources_list");
        assert_eq!(tool_list[1].name, "sources_chapters");
        assert_eq!(tool_list[2].name, "sources_get_chapter");
        assert_eq!(tool_list[3].name, "sources_check_availability");
        assert_eq!(tool_list[4].name, "sources_get_path");
    }

    #[test]
    fn test_source_tools_has_tool() {
        let tools = SourceTools::new(MockSourceProvider);
        assert!(tools.has_tool("sources_list"));
        assert!(tools.has_tool("sources_chapters"));
        assert!(tools.has_tool("sources_get_chapter"));
        assert!(tools.has_tool("sources_check_availability"));
        assert!(tools.has_tool("sources_get_path"));
        assert!(!tools.has_tool("sources_delete"));
    }

    #[tokio::test]
    async fn test_source_tools_list() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools.call("sources_list", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_chapters() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_chapters",
                serde_json::json!({"source_id": "book-1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_get_chapter() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_get_chapter",
                serde_json::json!({"source_id": "book-1", "chapter": "ch1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_source_tools_get_chapter_not_found() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_get_chapter",
                serde_json::json!({"source_id": "book-1", "chapter": "missing"}),
            )
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[test]
    fn test_source_tools_has_five_tools() {
        let tools = SourceTools::new(MockSourceProvider);
        assert_eq!(tools.tool_count(), 5);
    }

    #[tokio::test]
    async fn test_source_tools_check_availability() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_check_availability",
                serde_json::json!({"source_id": "book-1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        // MockSourceProvider returns None from get_source_path, so
        // the default is_available returns false.
        let text = result_to_string(&result);
        assert!(text.contains("available"));
    }

    #[tokio::test]
    async fn test_source_tools_get_path() {
        let tools = SourceTools::new(MockSourceProvider);
        let future = tools
            .call(
                "sources_get_path",
                serde_json::json!({"source_id": "book-1"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        // MockSourceProvider returns Ok(None) for get_source_path.
        let text = result_to_string(&result);
        assert!(text.contains("path"));
    }

    #[test]
    fn test_source_tools_check_availability_has_tool() {
        let tools = SourceTools::new(MockSourceProvider);
        assert!(tools.has_tool("sources_check_availability"));
    }

    #[test]
    fn test_source_tools_unknown_tool() {
        let tools = SourceTools::new(MockSourceProvider);
        assert!(
            tools
                .call("sources_delete", serde_json::json!({}))
                .is_none()
        );
    }

    // -- Custom names / descriptions tests ----------------------------------

    #[test]
    fn test_content_tools_with_custom_names() {
        let tools = ContentTools::new(MockContentProvider).with_names(HashMap::from([
            ("list".to_string(), "list_concepts".to_string()),
            ("get".to_string(), "get_concept".to_string()),
            ("categories".to_string(), "list_categories".to_string()),
        ]));
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "list_concepts");
        assert_eq!(tool_list[1].name, "get_concept");
        assert_eq!(tool_list[2].name, "list_categories");
    }

    #[tokio::test]
    async fn test_content_tools_custom_names_dispatch() {
        let tools = ContentTools::new(MockContentProvider).with_names(HashMap::from([(
            "list".to_string(),
            "list_concepts".to_string(),
        )]));
        // Old name should NOT work
        assert!(tools.call("list", serde_json::json!({})).is_none());
        // Custom name should work
        let future = tools.call("list_concepts", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_content_tools_with_custom_descriptions() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_descriptions(HashMap::from([(
                "list".to_string(),
                "My custom list description".to_string(),
            )]));
        let tool_list = tools.tools();
        assert_eq!(
            tool_list[0].description.as_ref().unwrap(),
            "My custom list description"
        );
        // Other tools keep defaults
        assert!(
            tool_list[1]
                .description
                .as_ref()
                .unwrap()
                .contains("concept")
        );
    }

    #[test]
    fn test_source_tools_with_custom_names() {
        let tools = SourceTools::new(MockSourceProvider).with_names(HashMap::from([
            ("sources_list".to_string(), "list_sources".to_string()),
            (
                "sources_chapters".to_string(),
                "get_source_chapters".to_string(),
            ),
            (
                "sources_get_chapter".to_string(),
                "get_chapter_content".to_string(),
            ),
        ]));
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "list_sources");
        assert_eq!(tool_list[1].name, "get_source_chapters");
        assert_eq!(tool_list[2].name, "get_chapter_content");
    }

    #[test]
    fn test_source_tools_custom_names_for_new_slots() {
        let tools = SourceTools::new(MockSourceProvider).with_names(HashMap::from([
            (
                "sources_check_availability".to_string(),
                "check_source_availability".to_string(),
            ),
            (
                "sources_get_path".to_string(),
                "get_source_pdf_path".to_string(),
            ),
        ]));
        let tool_list = tools.tools();
        // Original slots keep defaults.
        assert_eq!(tool_list[0].name, "sources_list");
        assert_eq!(tool_list[1].name, "sources_chapters");
        assert_eq!(tool_list[2].name, "sources_get_chapter");
        // New slots use custom names.
        assert_eq!(tool_list[3].name, "check_source_availability");
        assert_eq!(tool_list[4].name, "get_source_pdf_path");
    }

    #[tokio::test]
    async fn test_source_tools_custom_names_dispatch() {
        let tools = SourceTools::new(MockSourceProvider).with_names(HashMap::from([(
            "sources_list".to_string(),
            "list_sources".to_string(),
        )]));
        // Old name should NOT work
        assert!(tools.call("sources_list", serde_json::json!({})).is_none());
        // Custom name should work
        let future = tools.call("list_sources", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- Extra list schema / filter tests --------------------------------------

    #[test]
    fn test_content_tools_with_extra_list_schema() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_extra_list_schema(serde_json::json!({
                "tier": {"type": "string"}
            }));
        let tool_list = tools.tools();
        let list_tool = &tool_list[0];
        assert_eq!(list_tool.name, "concepts_list");

        // The list tool schema should contain the extra "tier" property.
        let schema_value = serde_json::to_value(&list_tool.input_schema).unwrap();
        let properties = schema_value
            .get("properties")
            .expect("schema should have properties");
        assert!(
            properties.get("category").is_some(),
            "should still have category"
        );
        assert!(properties.get("limit").is_some(), "should still have limit");
        assert!(
            properties.get("tier").is_some(),
            "should have extra tier property"
        );
        let tier = properties.get("tier").unwrap();
        assert_eq!(
            tier.get("type").and_then(|v| v.as_str()),
            Some("string"),
            "tier should be a string type"
        );
    }

    /// Mock provider that records extra filters passed to `list_items_filtered`.
    struct FilterCapturingProvider {
        captured: std::sync::Mutex<Option<crate::traits::FilterMap>>,
    }

    impl FilterCapturingProvider {
        fn new() -> Self {
            Self {
                captured: std::sync::Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl ContentItemProvider for FilterCapturingProvider {
        type ItemSummary = MockItemSummary;
        type ItemDetail = MockItemDetail;

        async fn list_items(
            &self,
            category: Option<&str>,
            limit: Option<usize>,
        ) -> fabryk_core::Result<Vec<Self::ItemSummary>> {
            // Delegate to the same mock data as MockContentProvider.
            let mut items = vec![
                MockItemSummary {
                    id: "item-1".to_string(),
                    title: "First Item".to_string(),
                    category: Some("alpha".to_string()),
                },
                MockItemSummary {
                    id: "item-2".to_string(),
                    title: "Second Item".to_string(),
                    category: Some("beta".to_string()),
                },
            ];
            if let Some(cat) = category {
                items.retain(|i| i.category.as_deref() == Some(cat));
            }
            if let Some(max) = limit {
                items.truncate(max);
            }
            Ok(items)
        }

        async fn get_item(&self, id: &str) -> fabryk_core::Result<Self::ItemDetail> {
            Ok(MockItemDetail {
                id: id.to_string(),
                title: "Detail".to_string(),
                content: "Content".to_string(),
            })
        }

        async fn list_categories(&self) -> fabryk_core::Result<Vec<CategoryInfo>> {
            Ok(vec![])
        }

        async fn list_items_filtered(
            &self,
            category: Option<&str>,
            limit: Option<usize>,
            extra_filters: &crate::traits::FilterMap,
        ) -> fabryk_core::Result<Vec<Self::ItemSummary>> {
            // Capture the extra filters for test assertions.
            *self.captured.lock().unwrap() = Some(extra_filters.clone());
            self.list_items(category, limit).await
        }
    }

    #[tokio::test]
    async fn test_content_tools_list_passes_extra_filters() {
        let provider = Arc::new(FilterCapturingProvider::new());
        let tools = ContentTools::with_shared(Arc::clone(&provider))
            .with_prefix("concepts")
            .with_extra_list_schema(serde_json::json!({
                "tier": {"type": "string"}
            }));

        let future = tools
            .call(
                "concepts_list",
                serde_json::json!({"category": "alpha", "tier": "advanced"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));

        // Verify the provider received the extra filter.
        let captured = provider.captured.lock().unwrap();
        let filters = captured
            .as_ref()
            .expect("filters should have been captured");
        assert_eq!(
            filters.get("tier").and_then(|v| v.as_str()),
            Some("advanced"),
            "tier filter should be passed through"
        );
        // category and limit should NOT appear in extra filters.
        assert!(
            filters.get("category").is_none(),
            "category should be extracted, not in extra filters"
        );
        assert!(
            filters.get("limit").is_none(),
            "limit should be extracted, not in extra filters"
        );
    }

    // -- Mock guide types -----------------------------------------------------

    #[derive(Clone, Debug, Serialize)]
    struct MockGuideSummary {
        id: String,
        title: String,
    }

    // -- Mock guide provider --------------------------------------------------

    struct MockGuideProvider;

    #[async_trait]
    impl GuideProvider for MockGuideProvider {
        type GuideSummary = MockGuideSummary;

        async fn list_guides(&self) -> fabryk_core::Result<Vec<Self::GuideSummary>> {
            Ok(vec![MockGuideSummary {
                id: "intro".to_string(),
                title: "Introduction".to_string(),
            }])
        }

        async fn get_guide(&self, id: &str) -> fabryk_core::Result<String> {
            if id == "intro" {
                Ok("# Introduction\nWelcome to the guide.".to_string())
            } else {
                Err(fabryk_core::Error::not_found("guide", id))
            }
        }
    }

    // -- GuideTools tests -----------------------------------------------------

    #[test]
    fn test_guide_tools_creation() {
        let tools = GuideTools::new(MockGuideProvider);
        assert_eq!(tools.tool_count(), 2);
    }

    #[test]
    fn test_guide_tools_tool_names() {
        let tools = GuideTools::new(MockGuideProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "list_guides");
        assert_eq!(tool_list[1].name, "get_guide");
    }

    #[test]
    fn test_guide_tools_has_tool() {
        let tools = GuideTools::new(MockGuideProvider);
        assert!(tools.has_tool("list_guides"));
        assert!(tools.has_tool("get_guide"));
        assert!(!tools.has_tool("delete_guide"));
    }

    #[tokio::test]
    async fn test_guide_tools_list() {
        let tools = GuideTools::new(MockGuideProvider);
        let future = tools.call("list_guides", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
        let text = result_to_string(&result);
        assert!(text.contains("intro"));
        assert!(text.contains("Introduction"));
    }

    #[tokio::test]
    async fn test_guide_tools_get() {
        let tools = GuideTools::new(MockGuideProvider);
        let future = tools
            .call("get_guide", serde_json::json!({"id": "intro"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = result_to_string(&result);
        assert!(text.contains("Welcome to the guide"));
    }

    #[tokio::test]
    async fn test_guide_tools_get_not_found() {
        let tools = GuideTools::new(MockGuideProvider);
        let future = tools
            .call("get_guide", serde_json::json!({"id": "missing"}))
            .unwrap();
        let result = future.await;
        assert!(result.is_err());
    }

    #[test]
    fn test_guide_tools_unknown_tool() {
        let tools = GuideTools::new(MockGuideProvider);
        assert!(tools.call("delete_guide", serde_json::json!({})).is_none());
    }

    #[test]
    fn test_guide_tools_custom_names() {
        let tools = GuideTools::new(MockGuideProvider).with_names(HashMap::from([
            ("list_guides".to_string(), "guides_list".to_string()),
            ("get_guide".to_string(), "guides_get".to_string()),
        ]));
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "guides_list");
        assert_eq!(tool_list[1].name, "guides_get");
    }

    #[tokio::test]
    async fn test_guide_tools_custom_names_dispatch() {
        let tools = GuideTools::new(MockGuideProvider).with_names(HashMap::from([(
            "list_guides".to_string(),
            "my_list_guides".to_string(),
        )]));
        // Old name should NOT work
        assert!(tools.call("list_guides", serde_json::json!({})).is_none());
        // Custom name should work
        let future = tools.call("my_list_guides", serde_json::json!({})).unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    // -- with_get_id_field tests ----------------------------------------------

    #[test]
    fn test_content_tools_default_get_id_field() {
        let tools = ContentTools::new(MockContentProvider).with_prefix("concepts");
        let tool_list = tools.tools();
        let get_tool = &tool_list[1];

        let schema_value = serde_json::to_value(&get_tool.input_schema).unwrap();
        let properties = schema_value
            .get("properties")
            .expect("schema should have properties");
        assert!(
            properties.get("id").is_some(),
            "default id field should be 'id'"
        );
        let required = schema_value.get("required").unwrap().as_array().unwrap();
        assert_eq!(required[0].as_str(), Some("id"));
    }

    #[test]
    fn test_content_tools_custom_get_id_field_schema() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_get_id_field("concept_id");
        let tool_list = tools.tools();
        let get_tool = &tool_list[1];

        let schema_value = serde_json::to_value(&get_tool.input_schema).unwrap();
        let properties = schema_value
            .get("properties")
            .expect("schema should have properties");
        assert!(
            properties.get("concept_id").is_some(),
            "custom id field should be 'concept_id'"
        );
        assert!(
            properties.get("id").is_none(),
            "default 'id' field should not be present"
        );
        let required = schema_value.get("required").unwrap().as_array().unwrap();
        assert_eq!(required[0].as_str(), Some("concept_id"));
    }

    #[tokio::test]
    async fn test_content_tools_custom_get_id_field_dispatch() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_get_id_field("concept_id");
        let future = tools
            .call("concepts_get", serde_json::json!({"concept_id": "item-1"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }

    #[tokio::test]
    async fn test_content_tools_custom_get_id_field_missing() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_get_id_field("concept_id");
        // Passing "id" instead of "concept_id" should fail.
        let future = tools
            .call("concepts_get", serde_json::json!({"id": "item-1"}))
            .unwrap();
        let result = future.await;
        assert!(
            result.is_err(),
            "should fail when custom id field is missing"
        );
    }

    #[test]
    fn test_content_tools_custom_get_id_field_description() {
        let tools = ContentTools::new(MockContentProvider)
            .with_prefix("concepts")
            .with_get_id_field("concept_id");
        let tool_list = tools.tools();
        let get_tool = &tool_list[1];
        let desc = get_tool.description.as_deref().unwrap_or("");
        assert!(
            desc.contains("concept_id"),
            "description should reference the custom id field name, got: {desc}"
        );
    }

    // -- Mock question provider -----------------------------------------------

    struct MockQuestionProvider;

    #[async_trait]
    impl QuestionSearchProvider for MockQuestionProvider {
        async fn search_by_question(
            &self,
            question: &str,
            limit: usize,
        ) -> fabryk_core::Result<crate::traits::QuestionSearchResponse> {
            use crate::traits::{QuestionMatch, QuestionSearchResponse};

            let matches = if question.contains("test") {
                vec![QuestionMatch {
                    item_id: "item-1".into(),
                    item_title: "Test Item".into(),
                    matched_question: "What is a test?".into(),
                    category: "testing".into(),
                    tier: Some("foundational".into()),
                    similarity: 0.95,
                }]
            } else {
                vec![]
            };
            let total = matches.len();
            Ok(QuestionSearchResponse {
                matches: matches.into_iter().take(limit).collect(),
                total,
                query: question.to_string(),
            })
        }
    }

    // -- QuestionSearchTools tests --------------------------------------------

    #[test]
    fn test_question_search_tools_creation() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        assert_eq!(tools.tool_count(), 1);
    }

    #[test]
    fn test_question_search_tools_tool_name() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "search_by_question");
    }

    #[test]
    fn test_question_search_tools_has_tool() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        assert!(tools.has_tool("search_by_question"));
        assert!(!tools.has_tool("list_questions"));
    }

    #[tokio::test]
    async fn test_question_search_tools_search() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        let future = tools
            .call(
                "search_by_question",
                serde_json::json!({"question": "test query"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        assert!(!result.content.is_empty());
        let text = result_to_string(&result);
        assert!(text.contains("item-1"));
        assert!(text.contains("Test Item"));
        assert!(text.contains("0.95"));
    }

    #[tokio::test]
    async fn test_question_search_tools_no_results() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        let future = tools
            .call(
                "search_by_question",
                serde_json::json!({"question": "nothing"}),
            )
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
        let text = result_to_string(&result);
        assert!(text.contains("matches"), "Expected 'matches' in: {text}");
        assert!(text.contains("total"), "Expected 'total' in: {text}");
    }

    #[test]
    fn test_question_search_tools_unknown_tool() {
        let tools = QuestionSearchTools::new(MockQuestionProvider);
        assert!(tools.call("unknown_tool", serde_json::json!({})).is_none());
    }

    #[test]
    fn test_question_search_tools_custom_names() {
        let tools = QuestionSearchTools::new(MockQuestionProvider).with_names(HashMap::from([(
            "search_by_question".to_string(),
            "find_by_question".to_string(),
        )]));
        let tool_list = tools.tools();
        assert_eq!(tool_list[0].name, "find_by_question");
    }

    #[tokio::test]
    async fn test_question_search_tools_custom_names_dispatch() {
        let tools = QuestionSearchTools::new(MockQuestionProvider).with_names(HashMap::from([(
            "search_by_question".to_string(),
            "my_search".to_string(),
        )]));
        // Old name should NOT work
        assert!(
            tools
                .call("search_by_question", serde_json::json!({}))
                .is_none()
        );
        // Custom name should work
        let future = tools
            .call("my_search", serde_json::json!({"question": "test query"}))
            .unwrap();
        let result = future.await.unwrap();
        assert_eq!(result.is_error, Some(false));
    }
}
