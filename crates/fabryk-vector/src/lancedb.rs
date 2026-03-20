//! LanceDB vector backend.
//!
//! Provides approximate nearest neighbor (ANN) search via LanceDB,
//! an embedded vector database built on Apache Arrow and Lance format.
//!
//! # Schema
//!
//! The generic Arrow schema:
//!
//! | Column | Type | Purpose |
//! |--------|------|---------|
//! | `id` | Utf8 | Unique document identifier |
//! | `text` | Utf8 | Original text (stored, not searched) |
//! | `category` | Utf8 (nullable) | Category for filtering |
//! | `metadata` | Utf8 | JSON-serialized metadata |
//! | `vector` | FixedSizeList<Float32> | Embedding vector |
//!
//! # Feature Gate
//!
//! This module requires the `vector-lancedb` feature.

use crate::backend::VectorBackend;
use crate::embedding::EmbeddingProvider;
use crate::types::{EmbeddedDocument, VectorSearchParams, VectorSearchResult, VectorSearchResults};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use async_trait::async_trait;
use fabryk_core::{Error, Result};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

/// LanceDB-backed vector search backend.
///
/// Stores embeddings in a LanceDB table with Arrow schema, providing
/// fast approximate nearest neighbor search with optional metadata filtering.
pub struct LancedbBackend {
    connection: lancedb::Connection,
    table_name: String,
    provider: Arc<dyn EmbeddingProvider>,
    document_count: usize,
}

impl LancedbBackend {
    /// Build a new LanceDB backend from embedded documents.
    ///
    /// Creates (or replaces) a LanceDB table with the given documents.
    ///
    /// # Arguments
    ///
    /// * `db_path` - Path to the LanceDB database directory
    /// * `table_name` - Name for the vector table
    /// * `provider` - Embedding provider for query embedding
    /// * `documents` - Pre-embedded documents to index
    pub async fn build(
        db_path: &str,
        table_name: &str,
        provider: Arc<dyn EmbeddingProvider>,
        documents: Vec<EmbeddedDocument>,
    ) -> Result<Self> {
        let connection = lancedb::connect(db_path)
            .execute()
            .await
            .map_err(|e| Error::operation(format!("Failed to connect to LanceDB: {e}")))?;

        let dimension = provider.dimension() as i32;
        let doc_count = documents.len();

        if !documents.is_empty() {
            let batch = build_record_batch(&documents, dimension)?;
            let schema = batch.schema();

            let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

            // Create or overwrite the table
            connection
                .create_table(
                    table_name,
                    Box::new(batches) as Box<dyn RecordBatchReader + Send>,
                )
                .mode(lancedb::database::CreateTableMode::Overwrite)
                .execute()
                .await
                .map_err(|e| Error::operation(format!("Failed to create LanceDB table: {e}")))?;
        }

        Ok(Self {
            connection,
            table_name: table_name.to_string(),
            provider,
            document_count: doc_count,
        })
    }
}

#[async_trait]
impl VectorBackend for LancedbBackend {
    async fn search(&self, params: VectorSearchParams) -> Result<VectorSearchResults> {
        if self.document_count == 0 {
            return Ok(VectorSearchResults::empty(self.name()));
        }

        let query_embedding = self.provider.embed(&params.query).await?;
        let limit = params.limit.unwrap_or(10);
        let threshold = params.similarity_threshold.unwrap_or(0.0);

        let table = self
            .connection
            .open_table(&self.table_name)
            .execute()
            .await
            .map_err(|e| Error::operation(format!("Failed to open table: {e}")))?;

        let mut query = table
            .vector_search(query_embedding)
            .map_err(|e| Error::operation(format!("Failed to create vector search: {e}")))?
            .limit(limit);

        // Apply category filter
        if let Some(ref category) = params.category {
            query = query.only_if(format!("category = '{}'", category.replace('\'', "''")));
        }

        // Apply metadata filters
        for (key, value) in &params.metadata_filters {
            query = query.only_if(format!(
                "json_extract(metadata, '$.{}') = '{}'",
                key.replace('\'', "''"),
                value.replace('\'', "''")
            ));
        }

        let results = query
            .execute()
            .await
            .map_err(|e| Error::operation(format!("Vector search failed: {e}")))?;

        let batches: Vec<RecordBatch> = results
            .try_collect()
            .await
            .map_err(|e| Error::operation(format!("Failed to collect results: {e}")))?;

        let mut items = Vec::new();
        for batch in &batches {
            let parsed = parse_search_results(batch)?;
            items.extend(parsed);
        }

        // Filter by threshold and sort
        items.retain(|r| r.score >= threshold);
        items.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        items.truncate(limit);

        let total = items.len();
        Ok(VectorSearchResults {
            items,
            total,
            backend: self.name().to_string(),
        })
    }

    fn name(&self) -> &str {
        "lancedb"
    }

    fn document_count(&self) -> Result<usize> {
        Ok(self.document_count)
    }
}

impl std::fmt::Debug for LancedbBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LancedbBackend")
            .field("table", &self.table_name)
            .field("documents", &self.document_count)
            .finish()
    }
}

// ============================================================================
// Arrow schema and batch construction
// ============================================================================

/// Create the Arrow schema for the vector table.
fn make_schema(dimension: i32) -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("id", DataType::Utf8, false),
        Field::new("text", DataType::Utf8, false),
        Field::new("category", DataType::Utf8, true),
        Field::new("metadata", DataType::Utf8, false),
        Field::new(
            "vector",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                dimension,
            ),
            false,
        ),
    ]))
}

/// Build an Arrow RecordBatch from embedded documents.
fn build_record_batch(documents: &[EmbeddedDocument], dimension: i32) -> Result<RecordBatch> {
    let schema = make_schema(dimension);

    let ids: Vec<&str> = documents.iter().map(|d| d.document.id.as_str()).collect();
    let texts: Vec<&str> = documents.iter().map(|d| d.document.text.as_str()).collect();
    let categories: Vec<Option<&str>> = documents
        .iter()
        .map(|d| d.document.category.as_deref())
        .collect();
    let metadata_strings: Vec<String> = documents
        .iter()
        .map(|d| serde_json::to_string(&d.document.metadata).unwrap_or_else(|_| "{}".to_string()))
        .collect();
    let metadata_refs: Vec<&str> = metadata_strings.iter().map(|s| s.as_str()).collect();

    // Flatten embeddings into a single Vec<f32>
    let all_values: Vec<f32> = documents
        .iter()
        .flat_map(|d| d.embedding.iter().copied())
        .collect();

    let values_array = Float32Array::from(all_values);
    let vector_array = FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        dimension,
        Arc::new(values_array),
        None,
    )
    .map_err(|e| Error::operation(format!("Failed to create vector array: {e}")))?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(ids)),
            Arc::new(StringArray::from(texts)),
            Arc::new(StringArray::from(categories)),
            Arc::new(StringArray::from(metadata_refs)),
            Arc::new(vector_array),
        ],
    )
    .map_err(|e| Error::operation(format!("Failed to create RecordBatch: {e}")))
}

/// Parse search results from a RecordBatch.
fn parse_search_results(batch: &RecordBatch) -> Result<Vec<VectorSearchResult>> {
    let id_col = batch
        .column_by_name("id")
        .ok_or_else(|| Error::operation("Missing 'id' column in results"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| Error::operation("'id' column is not StringArray"))?;

    let metadata_col = batch
        .column_by_name("metadata")
        .ok_or_else(|| Error::operation("Missing 'metadata' column in results"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| Error::operation("'metadata' column is not StringArray"))?;

    let distance_col = batch
        .column_by_name("_distance")
        .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

    let mut results = Vec::new();
    for i in 0..batch.num_rows() {
        let id = id_col.value(i).to_string();

        let metadata: std::collections::HashMap<String, String> = metadata_col
            .value(i)
            .parse::<serde_json::Value>()
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();

        let distance = distance_col.map(|c| c.value(i)).unwrap_or(0.0);
        // Distance-to-score normalization: 1/(1 + distance)
        let score = 1.0 / (1.0 + distance);

        results.push(VectorSearchResult {
            id,
            score,
            distance,
            metadata,
        });
    }

    Ok(results)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::VectorDocument;

    fn make_test_documents(dimension: usize) -> Vec<EmbeddedDocument> {
        vec![
            EmbeddedDocument::new(
                VectorDocument::new("doc-1", "harmony concepts")
                    .with_category("harmony")
                    .with_metadata("tier", "beginner"),
                vec![0.1; dimension],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-2", "rhythm patterns")
                    .with_category("rhythm")
                    .with_metadata("tier", "advanced"),
                vec![0.2; dimension],
            ),
            EmbeddedDocument::new(
                VectorDocument::new("doc-3", "melody writing").with_category("melody"),
                vec![0.3; dimension],
            ),
        ]
    }

    #[test]
    fn test_make_schema() {
        let schema = make_schema(384);
        assert_eq!(schema.fields().len(), 5);
        assert_eq!(schema.field(0).name(), "id");
        assert_eq!(schema.field(4).name(), "vector");

        match schema.field(4).data_type() {
            DataType::FixedSizeList(_, size) => assert_eq!(*size, 384),
            other => panic!("Expected FixedSizeList, got {:?}", other),
        }
    }

    #[test]
    fn test_build_record_batch() {
        let docs = make_test_documents(8);
        let batch = build_record_batch(&docs, 8).unwrap();

        assert_eq!(batch.num_rows(), 3);
        assert_eq!(batch.num_columns(), 5);
    }

    #[test]
    fn test_build_record_batch_ids() {
        let docs = make_test_documents(4);
        let batch = build_record_batch(&docs, 4).unwrap();

        let ids = batch
            .column_by_name("id")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        assert_eq!(ids.value(0), "doc-1");
        assert_eq!(ids.value(1), "doc-2");
        assert_eq!(ids.value(2), "doc-3");
    }

    #[test]
    fn test_build_record_batch_categories() {
        let docs = make_test_documents(4);
        let batch = build_record_batch(&docs, 4).unwrap();

        let categories = batch
            .column_by_name("category")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        assert_eq!(categories.value(0), "harmony");
        assert_eq!(categories.value(1), "rhythm");
    }

    #[test]
    fn test_build_record_batch_metadata() {
        let docs = make_test_documents(4);
        let batch = build_record_batch(&docs, 4).unwrap();

        let metadata = batch
            .column_by_name("metadata")
            .unwrap()
            .as_any()
            .downcast_ref::<StringArray>()
            .unwrap();

        let parsed: serde_json::Value = serde_json::from_str(metadata.value(0)).unwrap();
        assert_eq!(parsed["tier"], "beginner");
    }

    #[test]
    fn test_parse_search_results_basic() {
        let docs = make_test_documents(4);
        let batch = build_record_batch(&docs, 4).unwrap();

        let results = parse_search_results(&batch).unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].id, "doc-1");
        // Without _distance column, distance defaults to 0 → score = 1.0
        assert_eq!(results[0].score, 1.0);
    }

    #[test]
    fn test_distance_to_score_normalization() {
        // score = 1/(1 + distance)
        assert_eq!(1.0_f32 / (1.0 + 0.0), 1.0); // distance 0 → score 1.0
        assert!((1.0_f32 / (1.0 + 1.0) - 0.5).abs() < 1e-5); // distance 1 → score 0.5
        assert!((1.0_f32 / (1.0 + 0.176) - 0.85).abs() < 0.01); // distance 0.176 → score ~0.85
    }

    #[test]
    fn test_build_record_batch_empty() {
        let docs: Vec<EmbeddedDocument> = vec![];
        // Empty docs with dimension 4 - should still create valid schema
        // but build_record_batch with empty vec creates 0-row batch
        let batch = build_record_batch(&docs, 4).unwrap();
        assert_eq!(batch.num_rows(), 0);
    }

    #[tokio::test]
    async fn test_lancedb_backend_build_and_search() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test_db");

        let provider = Arc::new(crate::embedding::MockEmbeddingProvider::new(4));
        let docs = make_test_documents(4);

        let backend =
            LancedbBackend::build(db_path.to_str().unwrap(), "test_vectors", provider, docs)
                .await
                .unwrap();

        assert_eq!(backend.name(), "lancedb");
        assert_eq!(backend.document_count().unwrap(), 3);

        // Search
        let params = VectorSearchParams {
            query: "harmony".to_string(),
            limit: Some(10),
            ..Default::default()
        };

        let results = backend.search(params).await.unwrap();
        assert!(!results.items.is_empty());
        assert_eq!(results.backend, "lancedb");
    }

    #[tokio::test]
    async fn test_lancedb_backend_empty() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("empty_db");

        let provider = Arc::new(crate::embedding::MockEmbeddingProvider::new(4));
        let docs: Vec<EmbeddedDocument> = vec![];

        let backend =
            LancedbBackend::build(db_path.to_str().unwrap(), "empty_table", provider, docs)
                .await
                .unwrap();

        assert_eq!(backend.document_count().unwrap(), 0);

        let params = VectorSearchParams::new("test");
        let results = backend.search(params).await.unwrap();
        assert!(results.items.is_empty());
    }

    #[tokio::test]
    async fn test_lancedb_backend_score_normalization() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("score_db");

        let provider = Arc::new(crate::embedding::MockEmbeddingProvider::new(4));
        let docs = make_test_documents(4);

        let backend =
            LancedbBackend::build(db_path.to_str().unwrap(), "score_table", provider, docs)
                .await
                .unwrap();

        let params = VectorSearchParams::new("test query").with_limit(10);
        let results = backend.search(params).await.unwrap();

        // All scores should be in [0, 1]
        for item in &results.items {
            assert!(item.score >= 0.0 && item.score <= 1.0);
        }
    }

    #[tokio::test]
    async fn test_lancedb_backend_with_category_filter() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("filter_db");

        let provider = Arc::new(crate::embedding::MockEmbeddingProvider::new(4));
        let docs = make_test_documents(4);

        let backend =
            LancedbBackend::build(db_path.to_str().unwrap(), "filter_table", provider, docs)
                .await
                .unwrap();

        let params = VectorSearchParams::new("test")
            .with_limit(10)
            .with_category("harmony");

        let results = backend.search(params).await.unwrap();
        // Should only get harmony results
        for item in &results.items {
            // The category filter is applied at the LanceDB level
            assert_eq!(item.id, "doc-1");
        }
    }

    #[test]
    fn test_lancedb_debug() {
        // Can't easily construct without async, so just test schema/batch helpers
        let schema = make_schema(4);
        assert_eq!(schema.fields().len(), 5);
    }
}
