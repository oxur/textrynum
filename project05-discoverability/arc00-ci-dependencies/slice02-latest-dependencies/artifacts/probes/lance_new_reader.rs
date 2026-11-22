use arrow_array::{Array, FixedSizeListArray, Float32Array, StringArray};
use arrow_schema::DataType;
use fabryk_vector::persistence::{is_index_fresh, load_metadata};
use futures::TryStreamExt;
use lancedb::query::ExecutableQuery;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("copied fixture directory required")?);
    let database_path = root.join("lance-db");
    let database = lancedb::connect(database_path.to_str().ok_or("UTF-8 database path")?)
        .execute()
        .await?;
    let table = database.open_table("cc_vectors").execute().await?;
    let schema = table.schema().await?;
    assert_eq!(schema.fields().iter().map(|field| field.name().as_str()).collect::<Vec<_>>(),
        ["id", "text", "category", "metadata", "vector"]);
    assert_eq!(schema.field_with_name("id")?.data_type(), &DataType::Utf8);
    assert!(!schema.field_with_name("id")?.is_nullable());
    assert_eq!(schema.field_with_name("text")?.data_type(), &DataType::Utf8);
    assert!(!schema.field_with_name("text")?.is_nullable());
    assert_eq!(schema.field_with_name("category")?.data_type(), &DataType::Utf8);
    assert!(schema.field_with_name("category")?.is_nullable());
    assert_eq!(schema.field_with_name("metadata")?.data_type(), &DataType::Utf8);
    assert!(!schema.field_with_name("metadata")?.is_nullable());
    match schema.field_with_name("vector")?.data_type() {
        DataType::FixedSizeList(item, 4) => {
            assert_eq!(item.name(), "item");
            assert_eq!(item.data_type(), &DataType::Float32);
            assert!(item.is_nullable());
        }
        other => panic!("unexpected vector schema: {other:?}"),
    }
    assert!(!schema.field_with_name("vector")?.is_nullable());

    let batches = table.query().execute().await?.try_collect::<Vec<_>>().await?;
    let mut observed = HashMap::new();
    let mut total_rows = 0;
    for batch in &batches {
        let ids = batch.column_by_name("id").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
        let texts = batch.column_by_name("text").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
        let categories = batch.column_by_name("category").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
        let metadata = batch.column_by_name("metadata").unwrap().as_any().downcast_ref::<StringArray>().unwrap();
        let vectors = batch.column_by_name("vector").unwrap().as_any().downcast_ref::<FixedSizeListArray>().unwrap();
        for row in 0..batch.num_rows() {
            total_rows += 1;
            let vector_values = vectors.value(row);
            let vector = vector_values.as_any().downcast_ref::<Float32Array>().unwrap();
            let metadata: HashMap<String, String> = serde_json::from_str(metadata.value(row))?;
            assert!(observed.insert(
                ids.value(row).to_string(),
                (
                    texts.value(row).to_string(),
                    (!categories.is_null(row)).then(|| categories.value(row).to_string()),
                    metadata,
                    vector.values().to_vec(),
                ),
            ).is_none(), "duplicate old row id {}", ids.value(row));
        }
    }
    assert_eq!(total_rows, 3, "the old table contains exactly three rows");
    assert_eq!(observed.len(), 3);
    assert_eq!(observed["doc-null"], ("Document with nullable category".into(), None, HashMap::from([("owner".into(), "acme".into())]), vec![0.0; 4]));
    assert_eq!(observed["doc-one"], ("One vector row".into(), Some("reports".into()), HashMap::from([("owner".into(), "acme".into())]), vec![1.0; 4]));
    assert_eq!(observed["doc-two"], ("Two vector row".into(), Some("archive".into()), HashMap::from([("owner".into(), "example".into())]), vec![2.0; 4]));
    drop(table);
    drop(database);

    let metadata_path = root.join("index_metadata.json");
    let metadata = load_metadata(&metadata_path)?;
    assert_eq!(serde_json::to_value(&metadata)?, serde_json::json!({
        "content_hash": "golden-lance-content-hash-42",
        "document_count": 3,
        "embedding_dimension": 4,
        "built_at": "2026-03-13T10:00:00Z",
        "provider": "cc-zero-oracle",
        "model": "fixed-4d-fixture-v1"
    }));
    assert_eq!(metadata.content_hash, "golden-lance-content-hash-42");
    assert_eq!(metadata.document_count, 3);
    assert_eq!(metadata.embedding_dimension, 4);
    assert_eq!(metadata.built_at, "2026-03-13T10:00:00Z");
    assert_eq!(metadata.provider, "cc-zero-oracle");
    assert_eq!(metadata.model, "fixed-4d-fixture-v1");
    assert!(is_index_fresh(&metadata_path, "golden-lance-content-hash-42"));
    assert!(!is_index_fresh(&metadata_path, "different-independent-hash"));

    println!("upgraded Lance SDK read three golden rows and schema; product metadata reader and freshness check passed");
    Ok(())
}
