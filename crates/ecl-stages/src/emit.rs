//! Emit stage: writes pipeline item content to the output directory.

use std::path::PathBuf;

use async_trait::async_trait;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Emit stage that writes pipeline items to files in the output directory.
///
/// Each item is written to `{output_dir}/{item.id}`. Parent directories
/// are created automatically. The item passes through unchanged after
/// writing, so downstream stages (if any) still receive it.
#[derive(Debug)]
pub struct EmitStage;

impl EmitStage {
    /// Create a new emit stage.
    pub fn new() -> Self {
        Self
    }

    /// Compute the output path for an item.
    fn output_path(output_dir: &std::path::Path, item_id: &str) -> PathBuf {
        output_dir.join(item_id)
    }
}

impl Default for EmitStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Stage for EmitStage {
    fn name(&self) -> &str {
        "emit"
    }

    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let output_path = Self::output_path(&ctx.output_dir, &item.id);

        // Create parent directories
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| StageError::Permanent {
                    stage: "emit".to_string(),
                    item_id: item.id.clone(),
                    message: format!("failed to create directory {}: {e}", parent.display()),
                })?;
        }

        // Write content
        tokio::fs::write(&output_path, item.content.as_ref())
            .await
            .map_err(|e| StageError::Permanent {
                stage: "emit".to_string(),
                item_id: item.id.clone(),
                message: format!("failed to write {}: {e}", output_path.display()),
            })?;

        debug!(
            item_id = %item.id,
            path = %output_path.display(),
            bytes = item.content.len(),
            "emit: wrote file"
        );

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
    use std::sync::Arc;
    use tempfile::TempDir;

    fn make_item(id: &str, content: &[u8]) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            content: Arc::from(content),
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

    fn make_context(output_dir: PathBuf) -> StageContext {
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
            output_dir,
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    #[test]
    fn test_emit_stage_name() {
        let stage = EmitStage::new();
        assert_eq!(stage.name(), "emit");
    }

    #[test]
    fn test_emit_stage_default() {
        let stage = EmitStage;
        assert_eq!(stage.name(), "emit");
    }

    #[test]
    fn test_output_path_simple() {
        let path = EmitStage::output_path(std::path::Path::new("/out"), "readme.md");
        assert_eq!(path, PathBuf::from("/out/readme.md"));
    }

    #[test]
    fn test_output_path_nested() {
        let path = EmitStage::output_path(std::path::Path::new("/out"), "sub/dir/file.txt");
        assert_eq!(path, PathBuf::from("/out/sub/dir/file.txt"));
    }

    #[tokio::test]
    async fn test_emit_writes_file() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let item = make_item("test.txt", b"hello world");
        let ctx = make_context(tmp.path().to_path_buf());

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);

        let written = std::fs::read_to_string(tmp.path().join("test.txt")).unwrap();
        assert_eq!(written, "hello world");
    }

    #[tokio::test]
    async fn test_emit_creates_parent_directories() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let item = make_item("sub/dir/nested.md", b"# Nested");
        let ctx = make_context(tmp.path().to_path_buf());

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);

        let written = std::fs::read_to_string(tmp.path().join("sub/dir/nested.md")).unwrap();
        assert_eq!(written, "# Nested");
    }

    #[tokio::test]
    async fn test_emit_passes_item_through() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let item = make_item("doc.md", b"content");
        let ctx = make_context(tmp.path().to_path_buf());

        let result = stage.process(item, &ctx).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "doc.md");
        assert_eq!(result[0].content.as_ref(), b"content");
    }

    #[tokio::test]
    async fn test_emit_overwrites_existing_file() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let ctx = make_context(tmp.path().to_path_buf());

        // Write first version
        let item1 = make_item("test.txt", b"version 1");
        stage.process(item1, &ctx).await.unwrap();

        // Write second version
        let item2 = make_item("test.txt", b"version 2");
        stage.process(item2, &ctx).await.unwrap();

        let written = std::fs::read_to_string(tmp.path().join("test.txt")).unwrap();
        assert_eq!(written, "version 2");
    }

    #[tokio::test]
    async fn test_emit_handles_binary_content() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let binary = vec![0x00, 0x01, 0x02, 0xFF, 0xFE];
        let item = make_item("binary.bin", &binary);
        let ctx = make_context(tmp.path().to_path_buf());

        stage.process(item, &ctx).await.unwrap();

        let written = std::fs::read(tmp.path().join("binary.bin")).unwrap();
        assert_eq!(written, binary);
    }

    #[tokio::test]
    async fn test_emit_handles_empty_content() {
        let tmp = TempDir::new().unwrap();
        let stage = EmitStage::new();
        let item = make_item("empty.txt", b"");
        let ctx = make_context(tmp.path().to_path_buf());

        stage.process(item, &ctx).await.unwrap();

        let written = std::fs::read(tmp.path().join("empty.txt")).unwrap();
        assert!(written.is_empty());
    }
}
