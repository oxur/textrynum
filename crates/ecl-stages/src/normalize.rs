//! Normalize stage: passthrough placeholder for future format conversion.
//!
//! In the current implementation, this stage passes items through unchanged.
//! Future versions will handle format conversion (PDF→markdown, HTML→markdown, etc.).

use async_trait::async_trait;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Normalize stage that passes items through unchanged.
///
/// This is a placeholder for future format conversion logic. When fully
/// implemented, it will convert various input formats (PDF, HTML, DOCX)
/// into a normalized markdown representation.
#[derive(Debug)]
pub struct NormalizeStage;

impl NormalizeStage {
    /// Create a new normalize stage.
    pub fn new() -> Self {
        Self
    }
}

impl Default for NormalizeStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for NormalizeStage {
    fn name(&self) -> &str {
        "normalize"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        debug!(item_id = %item.id, "normalize: passthrough");
        Ok(vec![item])
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

    fn make_item() -> PipelineItem {
        PipelineItem {
            id: "doc.md".to_string(),
            display_name: "doc.md".to_string(),
            content: Arc::from(b"# Hello" as &[u8]),
            mime_type: "text/markdown".to_string(),
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
    fn test_normalize_stage_name() {
        let stage = NormalizeStage::new();
        assert_eq!(stage.name(), "normalize");
    }

    #[test]
    fn test_normalize_stage_default() {
        let stage = NormalizeStage;
        assert_eq!(stage.name(), "normalize");
    }

    #[tokio::test]
    async fn test_normalize_passthrough_preserves_content() {
        let stage = NormalizeStage::new();
        let item = make_item();
        let ctx = make_context();
        let original_content = item.content.clone();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content.as_ref(), original_content.as_ref());
    }

    #[tokio::test]
    async fn test_normalize_passthrough_preserves_metadata() {
        let stage = NormalizeStage::new();
        let mut item = make_item();
        item.metadata.insert(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(
            result[0].metadata.get("key"),
            Some(&serde_json::Value::String("value".to_string()))
        );
    }

    #[tokio::test]
    async fn test_normalize_passthrough_preserves_id() {
        let stage = NormalizeStage::new();
        let item = make_item();
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result[0].id, "doc.md");
    }

    #[tokio::test]
    async fn test_normalize_returns_exactly_one_item() {
        let stage = NormalizeStage::new();
        let item = make_item();
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
    }
}
