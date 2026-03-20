//! Decompress stage: extracts files from ZIP and GZIP archives (fan-out).
//!
//! Produces one `PipelineItem` per extracted file. Supports optional extension
//! filtering and preserves stream tags from the parent item.

use std::io::{Cursor, Read as _};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

/// Configuration for the decompress stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct DecompressConfig {
    /// Supported formats: "zip", "gzip". Default: "zip".
    #[serde(default = "default_zip")]
    pub format: String,
    /// File extension filter for extracted files (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
}

fn default_zip() -> String {
    "zip".to_string()
}

/// Decompress stage that extracts files from archives.
#[derive(Debug)]
pub struct DecompressStage {
    config: DecompressConfig,
}

impl DecompressStage {
    /// Create a decompress stage from JSON params.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: DecompressConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "decompress".into(),
                item_id: String::new(),
                message: format!("invalid decompress config: {e}"),
            })?;

        Ok(Self { config })
    }

    /// Extract files from a ZIP archive.
    fn decompress_zip(&self, item: PipelineItem) -> Result<Vec<PipelineItem>, StageError> {
        let cursor = Cursor::new(item.content.as_ref());
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| StageError::Permanent {
            stage: "decompress".into(),
            item_id: item.id.clone(),
            message: format!("invalid ZIP archive: {e}"),
        })?;

        let mut results = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| StageError::Permanent {
                stage: "decompress".into(),
                item_id: item.id.clone(),
                message: format!("failed to read ZIP entry {i}: {e}"),
            })?;

            if file.is_dir() {
                continue;
            }

            let name = file.name().to_string();

            // Extension filter.
            if !self.config.extensions.is_empty() {
                let ext = std::path::Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if !self.config.extensions.iter().any(|e| e == ext) {
                    continue;
                }
            }

            let mut content = Vec::new();
            file.read_to_end(&mut content)
                .map_err(|e| StageError::Permanent {
                    stage: "decompress".into(),
                    item_id: item.id.clone(),
                    message: format!("failed to read ZIP entry '{name}': {e}"),
                })?;

            results.push(PipelineItem {
                id: format!("{}:{}", item.id, name),
                display_name: name.clone(),
                content: Arc::from(content.as_slice()),
                mime_type: mime_from_extension(&name),
                source_name: item.source_name.clone(),
                source_content_hash: item.source_content_hash.clone(),
                provenance: item.provenance.clone(),
                metadata: item.metadata.clone(),
                record: None,
                stream: item.stream.clone(),
            });
        }

        debug!(
            item_id = %item.id,
            extracted = results.len(),
            "decompressed ZIP archive"
        );

        Ok(results)
    }

    /// Decompress a GZIP file (single file output).
    fn decompress_gzip(&self, item: PipelineItem) -> Result<Vec<PipelineItem>, StageError> {
        let cursor = Cursor::new(item.content.as_ref());
        let mut decoder = flate2::read::GzDecoder::new(cursor);
        let mut content = Vec::new();
        decoder
            .read_to_end(&mut content)
            .map_err(|e| StageError::Permanent {
                stage: "decompress".into(),
                item_id: item.id.clone(),
                message: format!("failed to decompress GZIP: {e}"),
            })?;

        // Strip .gz extension from the display name for the output.
        let output_name = item
            .display_name
            .strip_suffix(".gz")
            .unwrap_or(&item.display_name)
            .to_string();

        let output_item = PipelineItem {
            id: format!("{}:gunzipped", item.id),
            display_name: output_name.clone(),
            content: Arc::from(content.as_slice()),
            mime_type: mime_from_extension(&output_name),
            source_name: item.source_name.clone(),
            source_content_hash: item.source_content_hash.clone(),
            provenance: item.provenance.clone(),
            metadata: item.metadata.clone(),
            record: None,
            stream: item.stream.clone(),
        };

        debug!(
            item_id = %item.id,
            output_size = content.len(),
            "decompressed GZIP file"
        );

        Ok(vec![output_item])
    }
}

/// Guess MIME type from file extension.
fn mime_from_extension(name: &str) -> String {
    match std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some("csv") => "text/csv".to_string(),
        Some("json") => "application/json".to_string(),
        Some("txt") => "text/plain".to_string(),
        Some("xml") => "application/xml".to_string(),
        Some("html") | Some("htm") => "text/html".to_string(),
        Some("md") => "text/markdown".to_string(),
        _ => "application/octet-stream".to_string(),
    }
}

#[async_trait]
impl Stage for DecompressStage {
    fn name(&self) -> &str {
        "decompress"
    }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        match self.config.format.as_str() {
            "zip" => self.decompress_zip(item),
            "gzip" => self.decompress_gzip(item),
            other => Err(StageError::Permanent {
                stage: "decompress".into(),
                item_id: String::new(),
                message: format!("unsupported format: {other}"),
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;
    use std::collections::BTreeMap;
    use std::io::Write as _;

    fn make_item(id: &str, content: &[u8]) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            content: Arc::from(content),
            mime_type: "application/zip".to_string(),
            source_name: "test".to_string(),
            source_content_hash: Blake3Hash::new("test"),
            provenance: ItemProvenance {
                source_kind: "test".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: None,
            stream: None,
        }
    }

    fn make_item_with_stream(id: &str, content: &[u8], stream: &str) -> PipelineItem {
        let mut item = make_item(id, content);
        item.stream = Some(stream.to_string());
        item
    }

    fn ctx() -> StageContext {
        StageContext {
            spec: Arc::new(PipelineSpec {
                name: "test".to_string(),
                version: 1,
                output_dir: std::path::PathBuf::from("./out"),
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                defaults: ecl_pipeline_spec::DefaultsSpec::default(),
                lifecycle: None,
                secrets: Default::default(),
                triggers: None,
                schedule: None,
            }),
            output_dir: std::path::PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    /// Create a ZIP archive in memory with the given files.
    fn create_test_zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            for (name, content) in files {
                writer.start_file(*name, options).unwrap();
                writer.write_all(content).unwrap();
            }
            writer.finish().unwrap();
        }
        buf
    }

    /// Create a GZIP compressed buffer.
    fn create_test_gzip(content: &[u8]) -> Vec<u8> {
        use flate2::Compression;
        use flate2::write::GzEncoder;
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(content).unwrap();
        encoder.finish().unwrap()
    }

    #[tokio::test]
    async fn test_decompress_zip_basic() {
        let zip_data = create_test_zip(&[
            ("file1.csv", b"a,b,c"),
            ("file2.csv", b"d,e,f"),
            ("file3.txt", b"hello"),
        ]);
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("archive.zip", &zip_data);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, "archive.zip:file1.csv");
        assert_eq!(result[0].display_name, "file1.csv");
        assert_eq!(result[0].mime_type, "text/csv");
        assert_eq!(result[0].content.as_ref(), b"a,b,c");
        assert_eq!(result[2].mime_type, "text/plain");
    }

    #[tokio::test]
    async fn test_decompress_zip_extension_filter() {
        let zip_data = create_test_zip(&[
            ("data.csv", b"a,b"),
            ("readme.txt", b"docs"),
            ("more.csv", b"c,d"),
        ]);
        let params = json!({ "format": "zip", "extensions": ["csv"] });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("archive.zip", &zip_data);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|i| i.mime_type == "text/csv"));
    }

    #[tokio::test]
    async fn test_decompress_zip_skips_directories() {
        // ZIP entries ending with / are directories
        let mut buf = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut buf));
            let dir_options = zip::write::SimpleFileOptions::default();
            writer.add_directory("subdir/", dir_options).unwrap();
            let file_options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file("subdir/file.csv", file_options).unwrap();
            writer.write_all(b"data").unwrap();
            writer.finish().unwrap();
        }
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("archive.zip", &buf);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].display_name, "subdir/file.csv");
    }

    #[tokio::test]
    async fn test_decompress_zip_preserves_stream_tag() {
        let zip_data = create_test_zip(&[("file.csv", b"data")]);
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item_with_stream("archive.zip", &zip_data, "raw_files");

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result[0].stream.as_deref(), Some("raw_files"));
    }

    #[tokio::test]
    async fn test_decompress_zip_fan_out_ids() {
        let zip_data = create_test_zip(&[("stores.csv", b"s"), ("transactions.csv", b"t")]);
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("ge_data.zip", &zip_data);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result[0].id, "ge_data.zip:stores.csv");
        assert_eq!(result[1].id, "ge_data.zip:transactions.csv");
    }

    #[tokio::test]
    async fn test_decompress_zip_empty_archive() {
        let zip_data = create_test_zip(&[]);
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("empty.zip", &zip_data);

        let result = stage.process(item, &ctx()).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_decompress_zip_invalid_archive() {
        let params = json!({ "format": "zip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let item = make_item("bad.zip", b"this is not a zip file");

        let result = stage.process(item, &ctx()).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            StageError::Permanent { message, .. } => {
                assert!(message.contains("invalid ZIP archive"));
            }
            other => panic!("expected Permanent error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_decompress_gzip_basic() {
        let gzip_data = create_test_gzip(b"hello world compressed");
        let params = json!({ "format": "gzip" });
        let stage = DecompressStage::from_params(&params).unwrap();
        let mut item = make_item("data.csv.gz", &gzip_data);
        item.display_name = "data.csv.gz".to_string();

        let result = stage.process(item, &ctx()).await.unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, "data.csv.gz:gunzipped");
        assert_eq!(result[0].display_name, "data.csv");
        assert_eq!(result[0].mime_type, "text/csv");
        assert_eq!(result[0].content.as_ref(), b"hello world compressed");
    }
}
