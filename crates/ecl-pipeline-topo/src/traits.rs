//! Core pipeline traits: SourceAdapter, Stage, and supporting types.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use ecl_pipeline_spec::PipelineSpec;
use ecl_pipeline_state::{Blake3Hash, ItemProvenance};

use crate::error::{SourceError, StageError};

/// Custom serde module for `Arc<[u8]>` using serde_bytes for efficient
/// binary serialization. `serde_bytes` doesn't natively support `Arc<[u8]>`,
/// so we serialize via `&[u8]` and deserialize via `Vec<u8>` then convert.
mod arc_bytes {
    use serde::{Deserializer, Serializer};
    use std::sync::Arc;

    /// Serialize `Arc<[u8]>` as bytes.
    pub fn serialize<S>(data: &Arc<[u8]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_bytes::serialize(data.as_ref(), serializer)
    }

    /// Deserialize bytes into `Arc<[u8]>`.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<[u8]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde_bytes::deserialize(deserializer)?;
        Ok(Arc::from(bytes))
    }
}

/// A source adapter handles all interaction with an external data service.
///
/// Implementors handle: authentication, enumeration, filtering, pagination,
/// rate limiting, and fetching. The pipeline runner sees only the trait.
///
/// Object-safe by design: adapters are resolved from TOML config at runtime
/// and stored as `Arc<dyn SourceAdapter>`.
///
/// Note: `async_trait` is required here despite Rust 1.85+ supporting native
/// `async fn` in traits. Native async trait methods are not object-safe:
/// `dyn SourceAdapter` requires the future to be boxed, which `async_trait`
/// handles automatically.
#[async_trait]
pub trait SourceAdapter: Send + Sync + std::fmt::Debug {
    /// Human-readable name of the source type (e.g., "Google Drive").
    fn source_kind(&self) -> &str;

    /// Enumerate items available from this source.
    /// Returns lightweight descriptors (no content) for filtering and
    /// hash comparison. This is the "what's there?" step.
    ///
    /// The adapter applies source-level filters (folder IDs, file types,
    /// modified_after) during enumeration.
    async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError>;

    /// Fetch the full content of a single item.
    /// Separate from `enumerate()` because fetching is expensive and we
    /// want to skip unchanged items before paying this cost.
    async fn fetch(&self, item: &SourceItem) -> Result<ExtractedDocument, SourceError>;
}

/// Lightweight item descriptor returned by `SourceAdapter::enumerate()`.
/// Contains enough metadata for filtering and hash comparison,
/// but does NOT contain the actual content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceItem {
    /// Source-specific unique identifier.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// MIME type (for filtering by file type).
    pub mime_type: String,

    /// Path within the source (for glob-based filtering).
    pub path: String,

    /// Last modified timestamp (for incremental sync).
    pub modified_at: Option<DateTime<Utc>>,

    /// Content hash if cheaply available from the source API.
    /// Google Drive provides md5Checksum; Slack provides message hash.
    /// If None, the pipeline fetches content and computes blake3.
    pub source_hash: Option<String>,
}

/// A document extracted from a source, in its original format.
/// This is the raw material before any normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    /// Unique identifier within this source.
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// The raw content bytes.
    #[serde(with = "serde_bytes")]
    pub content: Vec<u8>,

    /// MIME type of the content, as reported by the source.
    pub mime_type: String,

    /// Provenance metadata.
    pub provenance: ItemProvenance,

    /// Content hash (blake3 of content bytes).
    pub content_hash: Blake3Hash,
}

/// The intermediate representation flowing between stages.
/// Starts life as an `ExtractedDocument`, accumulates transformations.
///
/// Uses `Arc<[u8]>` for content to enable zero-copy cloning in hot paths.
/// `PipelineItem` is cloned when fanning out to concurrent tasks and when
/// building retry attempts — `Arc<[u8]>` makes these O(1) instead of O(n).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineItem {
    /// The item's unique identifier (stable across stages).
    pub id: String,

    /// Human-readable name.
    pub display_name: String,

    /// Current content (may be transformed by prior stages).
    /// Wrapped in `Arc` for zero-copy cloning in concurrent pipelines.
    #[serde(with = "arc_bytes")]
    pub content: Arc<[u8]>,

    /// Current MIME type (changes as content is transformed,
    /// e.g., "application/pdf" -> "text/markdown").
    pub mime_type: String,

    /// Which source this item came from.
    pub source_name: String,

    /// Content hash of the original source content (for incrementality).
    pub source_content_hash: Blake3Hash,

    /// Provenance chain.
    pub provenance: ItemProvenance,

    /// Metadata accumulated by stages. Each stage can add key-value pairs.
    /// Structured as `serde_json::Value` for flexibility without losing
    /// serializability.
    pub metadata: BTreeMap<String, serde_json::Value>,
}

/// A pipeline stage transforms items.
///
/// Stages are intentionally simple: one item in, zero or more out.
/// The runner handles orchestration, retries, checkpointing, and
/// concurrency. The stage handles only transformation logic.
///
/// Object-safe by design: stages are resolved from TOML config at runtime
/// and stored as `Arc<dyn Stage>`.
#[async_trait]
pub trait Stage: Send + Sync + std::fmt::Debug {
    /// Human-readable name of this stage type.
    fn name(&self) -> &str;

    /// Process a single item. Returns:
    /// - `Ok(vec![item])` — item transformed successfully (common case)
    /// - `Ok(vec![item1, item2, ...])` — item split into multiple (fan-out)
    /// - `Ok(vec![])` — item filtered out / consumed
    /// - `Err(e)` — processing failed
    async fn process(
        &self,
        item: PipelineItem,
        ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError>;
}

/// Read-only context provided to stages during execution.
/// Immutable — stages cannot mutate prior outputs or pipeline state.
/// (Addresses erio-workflow's `&mut WorkflowContext` anti-pattern.)
#[derive(Debug, Clone)]
pub struct StageContext {
    /// The original pipeline specification.
    pub spec: Arc<PipelineSpec>,

    /// The output directory for this pipeline run.
    pub output_dir: PathBuf,

    /// Stage-specific parameters from the pipeline config.
    pub params: serde_json::Value,

    /// Tracing span for structured logging within this stage.
    pub span: tracing::Span,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;

    fn test_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 3, 13, 10, 0, 0).unwrap()
    }

    fn make_source_item() -> SourceItem {
        SourceItem {
            id: "file-123".to_string(),
            display_name: "doc.pdf".to_string(),
            mime_type: "application/pdf".to_string(),
            path: "/Engineering/doc.pdf".to_string(),
            modified_at: Some(test_time()),
            source_hash: Some("md5abc".to_string()),
        }
    }

    fn make_provenance() -> ItemProvenance {
        ItemProvenance {
            source_kind: "filesystem".to_string(),
            metadata: BTreeMap::new(),
            source_modified: None,
            extracted_at: test_time(),
        }
    }

    #[test]
    fn test_source_item_serde_roundtrip() {
        let item = make_source_item();
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: SourceItem = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_extracted_document_serde_roundtrip() {
        let doc = ExtractedDocument {
            id: "file-123".to_string(),
            display_name: "doc.pdf".to_string(),
            content: b"hello world".to_vec(),
            mime_type: "application/pdf".to_string(),
            provenance: make_provenance(),
            content_hash: Blake3Hash::new("aabbccdd"),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: ExtractedDocument = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_pipeline_item_serde_roundtrip() {
        let item = PipelineItem {
            id: "item-001".to_string(),
            display_name: "doc.pdf".to_string(),
            content: Arc::from(b"content bytes" as &[u8]),
            mime_type: "text/markdown".to_string(),
            source_name: "local".to_string(),
            source_content_hash: Blake3Hash::new("aabb"),
            provenance: make_provenance(),
            metadata: BTreeMap::new(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let deserialized: PipelineItem = serde_json::from_str(&json).unwrap();
        let json2 = serde_json::to_string(&deserialized).unwrap();
        assert_eq!(json, json2);
    }

    #[test]
    fn test_pipeline_item_arc_content_clone_is_shallow() {
        let item = PipelineItem {
            id: "item-001".to_string(),
            display_name: "doc.pdf".to_string(),
            content: Arc::from(b"shared content" as &[u8]),
            mime_type: "text/plain".to_string(),
            source_name: "local".to_string(),
            source_content_hash: Blake3Hash::new("aabb"),
            provenance: make_provenance(),
            metadata: BTreeMap::new(),
        };
        let cloned = item.clone();
        assert!(Arc::ptr_eq(&item.content, &cloned.content));
    }

    #[derive(Debug)]
    struct MockSourceAdapter;

    #[async_trait]
    impl SourceAdapter for MockSourceAdapter {
        fn source_kind(&self) -> &str {
            "mock"
        }

        async fn enumerate(&self) -> Result<Vec<SourceItem>, SourceError> {
            Ok(vec![])
        }

        async fn fetch(&self, _item: &SourceItem) -> Result<ExtractedDocument, SourceError> {
            Err(SourceError::NotFound {
                source_name: "mock".to_string(),
                item_id: "none".to_string(),
            })
        }
    }

    #[derive(Debug)]
    struct MockStage;

    #[async_trait]
    impl Stage for MockStage {
        fn name(&self) -> &str {
            "mock"
        }

        async fn process(
            &self,
            item: PipelineItem,
            _ctx: &StageContext,
        ) -> Result<Vec<PipelineItem>, StageError> {
            Ok(vec![item])
        }
    }

    #[test]
    fn test_source_adapter_is_object_safe() {
        let adapter: Arc<dyn SourceAdapter> = Arc::new(MockSourceAdapter);
        assert_eq!(adapter.source_kind(), "mock");
    }

    #[test]
    fn test_stage_is_object_safe() {
        let stage: Arc<dyn Stage> = Arc::new(MockStage);
        assert_eq!(stage.name(), "mock");
    }

    #[test]
    fn test_stage_context_is_clone() {
        let ctx = StageContext {
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
            output_dir: PathBuf::from("./output"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        };
        let _cloned = ctx.clone();
    }
}
