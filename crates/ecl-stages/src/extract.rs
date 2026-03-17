//! Extract stage: delegates to a `SourceAdapter` to fetch document content.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, SourceAdapter, SourceItem, Stage, StageContext};

/// Extract stage that fetches content from a source adapter.
///
/// For each `PipelineItem` that arrives (typically created by the runner from
/// `SourceItem` enumeration), this stage calls `adapter.fetch()` to retrieve
/// the full document content. The result replaces the item's content and
/// metadata.
#[derive(Debug)]
pub struct ExtractStage {
    /// The source adapter to fetch from.
    adapter: Arc<dyn SourceAdapter>,
    /// Source name for error reporting.
    source_name: String,
}

impl ExtractStage {
    /// Create a new extract stage backed by the given source adapter.
    pub fn new(adapter: Arc<dyn SourceAdapter>, source_name: &str) -> Self {
        Self {
            adapter,
            source_name: source_name.to_string(),
        }
    }
}

#[async_trait]
impl Stage for ExtractStage {
    fn name(&self) -> &str {
        "extract"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        debug!(item_id = %item.id, source = %self.source_name, "extracting content");

        // Build a SourceItem from the PipelineItem for the adapter
        let source_item = SourceItem {
            id: item.id.clone(),
            display_name: item.display_name.clone(),
            mime_type: item.mime_type.clone(),
            path: item.id.clone(),
            modified_at: item.provenance.source_modified,
            source_hash: None,
        };

        let doc = self
            .adapter
            .fetch(&source_item)
            .await
            .map_err(|e| StageError::Permanent {
                stage: "extract".to_string(),
                item_id: item.id.clone(),
                message: format!("source fetch failed: {e}"),
            })?;

        let extracted = PipelineItem {
            id: doc.id,
            display_name: doc.display_name,
            content: Arc::from(doc.content),
            mime_type: doc.mime_type,
            source_name: self.source_name.clone(),
            source_content_hash: doc.content_hash,
            provenance: doc.provenance,
            metadata: BTreeMap::new(),
            record: None,
        };

        Ok(vec![extracted])
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use ecl_pipeline_topo::ExtractedDocument;
    use ecl_pipeline_topo::error::SourceError;
    use std::path::PathBuf;

    #[derive(Debug)]
    struct MockAdapter {
        content: Vec<u8>,
    }

    #[async_trait]
    impl SourceAdapter for MockAdapter {
        fn source_kind(&self) -> &str {
            "mock"
        }
        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }
        async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Ok(ExtractedDocument {
                id: item.id.clone(),
                display_name: item.display_name.clone(),
                content: self.content.clone(),
                mime_type: "text/plain".to_string(),
                provenance: ItemProvenance {
                    source_kind: "mock".to_string(),
                    metadata: BTreeMap::new(),
                    source_modified: None,
                    extracted_at: chrono::Utc::now(),
                },
                content_hash: Blake3Hash::new("abc123"),
            })
        }
    }

    #[derive(Debug)]
    struct FailingAdapter;

    #[async_trait]
    impl SourceAdapter for FailingAdapter {
        fn source_kind(&self) -> &str {
            "failing"
        }
        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }
        async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Err(SourceError::NotFound {
                source_name: "failing".to_string(),
                item_id: item.id.clone(),
            })
        }
    }

    fn make_pipeline_item() -> PipelineItem {
        PipelineItem {
            id: "test-file.md".to_string(),
            display_name: "test-file.md".to_string(),
            content: Arc::from(b"placeholder" as &[u8]),
            mime_type: "text/markdown".to_string(),
            source_name: "local".to_string(),
            source_content_hash: Blake3Hash::new("0000"),
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
    fn test_extract_stage_name() {
        let stage = ExtractStage::new(Arc::new(MockAdapter { content: vec![] }), "local");
        assert_eq!(stage.name(), "extract");
    }

    #[tokio::test]
    async fn test_extract_stage_process_success() {
        let adapter = Arc::new(MockAdapter {
            content: b"fetched content".to_vec(),
        });
        let stage = ExtractStage::new(adapter, "local");
        let item = make_pipeline_item();
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content.as_ref(), b"fetched content");
        assert_eq!(result[0].source_name, "local");
        assert_eq!(result[0].source_content_hash.as_str(), "abc123");
    }

    #[tokio::test]
    async fn test_extract_stage_process_preserves_id() {
        let adapter = Arc::new(MockAdapter {
            content: b"data".to_vec(),
        });
        let stage = ExtractStage::new(adapter, "local");
        let item = make_pipeline_item();
        let ctx = make_context();

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result[0].id, "test-file.md");
    }

    #[tokio::test]
    async fn test_extract_stage_process_failure() {
        let adapter = Arc::new(FailingAdapter);
        let stage = ExtractStage::new(adapter, "local");
        let item = make_pipeline_item();
        let ctx = make_context();

        let result = stage.process(item, &ctx).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StageError::Permanent { .. }));
    }
}
