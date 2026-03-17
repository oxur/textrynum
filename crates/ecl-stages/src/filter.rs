//! Filter stage: glob-based include/exclude filtering of pipeline items.

use async_trait::async_trait;
use glob::Pattern;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Filter stage that applies glob include/exclude rules to pipeline items.
///
/// Configuration is read from `StageContext.params`:
/// ```json
/// {
///   "include": ["**/*.md", "**/*.txt"],
///   "exclude": ["**/draft/**"]
/// }
/// ```
///
/// Items are matched against their `id` (which is the relative path for
/// filesystem sources). If `include` patterns are specified, the item must
/// match at least one. If `exclude` patterns are specified, the item must
/// not match any. Exclude takes precedence over include.
///
/// If no patterns are configured, all items pass through.
#[derive(Debug)]
pub struct FilterStage {
    /// Pre-compiled include patterns.
    include: Vec<Pattern>,
    /// Pre-compiled exclude patterns.
    exclude: Vec<Pattern>,
}

impl FilterStage {
    /// Create a filter stage from JSON params.
    ///
    /// Expects `params` to optionally contain `"include"` and `"exclude"` arrays
    /// of glob pattern strings.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if a glob pattern is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let include = compile_patterns(params, "include")?;
        let exclude = compile_patterns(params, "exclude")?;
        Ok(Self { include, exclude })
    }

    /// Create a filter stage with no patterns (passes everything through).
    pub fn passthrough() -> Self {
        Self {
            include: vec![],
            exclude: vec![],
        }
    }

    /// Check whether an item passes the filter.
    fn matches(&self, path: &str) -> bool {
        // If exclude patterns are set and any matches, reject
        if self.exclude.iter().any(|p| p.matches(path)) {
            return false;
        }

        // If include patterns are set, at least one must match
        if self.include.is_empty() {
            return true;
        }

        self.include.iter().any(|p| p.matches(path))
    }
}

/// Compile glob patterns from a JSON field.
fn compile_patterns(params: &serde_json::Value, field: &str) -> Result<Vec<Pattern>, StageError> {
    let Some(arr) = params.get(field) else {
        return Ok(vec![]);
    };
    let Some(arr) = arr.as_array() else {
        return Ok(vec![]);
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .map(|s| {
            Pattern::new(s).map_err(|e| StageError::Permanent {
                stage: "filter".to_string(),
                item_id: String::new(),
                message: format!("invalid glob pattern '{s}': {e}"),
            })
        })
        .collect()
}

#[async_trait]
impl Stage for FilterStage {
    fn name(&self) -> &str {
        "filter"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        if self.matches(&item.id) {
            debug!(item_id = %item.id, "filter: included");
            Ok(vec![item])
        } else {
            debug!(item_id = %item.id, "filter: excluded");
            Ok(vec![])
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn make_item(id: &str) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            content: Arc::from(b"content" as &[u8]),
            mime_type: "text/plain".to_string(),
            source_name: "local".to_string(),
            source_content_hash: Blake3Hash::new("aabb"),
            provenance: ItemProvenance {
                source_kind: "filesystem".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: None,
        }
    }

    fn make_context() -> StageContext {
        StageContext {
            spec: Arc::new(
                PipelineSpec::from_toml(
                    r#"
name = "test"
version = 1
output_dir = "./out"

[sources.local]
kind = "filesystem"
root = "/tmp"

[stages.extract]
adapter = "extract"
source = "local"
resources = { creates = ["docs"] }
"#,
                )
                .unwrap(),
            ),
            output_dir: PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    #[test]
    fn test_filter_stage_name() {
        let stage = FilterStage::passthrough();
        assert_eq!(stage.name(), "filter");
    }

    #[test]
    fn test_from_params_empty_passes_all() {
        let params = serde_json::json!({});
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.include.is_empty());
        assert!(stage.exclude.is_empty());
    }

    #[test]
    fn test_from_params_with_include() {
        let params = serde_json::json!({
            "include": ["**/*.md", "**/*.txt"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        assert_eq!(stage.include.len(), 2);
        assert!(stage.exclude.is_empty());
    }

    #[test]
    fn test_from_params_with_exclude() {
        let params = serde_json::json!({
            "exclude": ["**/draft/**"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.include.is_empty());
        assert_eq!(stage.exclude.len(), 1);
    }

    #[test]
    fn test_from_params_invalid_pattern() {
        let params = serde_json::json!({
            "include": ["[bad"]
        });
        let result = FilterStage::from_params(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_params_null_value() {
        let params = serde_json::Value::Null;
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.matches("anything"));
    }

    #[test]
    fn test_matches_no_patterns_includes_all() {
        let stage = FilterStage::passthrough();
        assert!(stage.matches("any/file.md"));
        assert!(stage.matches("any/file.rs"));
    }

    #[test]
    fn test_matches_include_only() {
        let params = serde_json::json!({
            "include": ["**/*.md"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.matches("docs/readme.md"));
        assert!(!stage.matches("data/file.json"));
    }

    #[test]
    fn test_matches_exclude_only() {
        let params = serde_json::json!({
            "exclude": ["**/draft/**"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.matches("docs/readme.md"));
        assert!(!stage.matches("draft/wip.md"));
    }

    #[test]
    fn test_matches_exclude_takes_precedence() {
        let params = serde_json::json!({
            "include": ["**/*.md"],
            "exclude": ["**/draft/**"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        assert!(stage.matches("docs/readme.md"));
        assert!(!stage.matches("draft/readme.md"));
        assert!(!stage.matches("data/file.json")); // not in include
    }

    #[tokio::test]
    async fn test_process_includes_matching_item() {
        let params = serde_json::json!({
            "include": ["**/*.md"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        let item = make_item("docs/readme.md");
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "docs/readme.md");
    }

    #[tokio::test]
    async fn test_process_excludes_non_matching_item() {
        let params = serde_json::json!({
            "include": ["**/*.md"]
        });
        let stage = FilterStage::from_params(&params).unwrap();
        let item = make_item("data/file.json");
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_process_passthrough_includes_everything() {
        let stage = FilterStage::passthrough();
        let item = make_item("any/file.xyz");
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
    }
}
