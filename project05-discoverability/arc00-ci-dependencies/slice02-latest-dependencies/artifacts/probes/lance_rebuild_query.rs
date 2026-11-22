use async_trait::async_trait;
use fabryk_core::Result as FabrykResult;
use fabryk_vector::{
    EmbeddedDocument, EmbeddingProvider, LancedbBackend, VectorBackend, VectorDocument,
    VectorSearchParams,
};
use futures::TryStreamExt;
use lancedb::query::ExecutableQuery;
use std::path::PathBuf;
use std::sync::Arc;

struct ZeroProvider;

#[async_trait]
impl EmbeddingProvider for ZeroProvider {
    async fn embed(&self, _text: &str) -> FabrykResult<Vec<f32>> {
        Ok(vec![0.0; 4])
    }

    fn dimension(&self) -> usize { 4 }
    fn name(&self) -> &str { "cc-zero-oracle" }
}

fn documents() -> Vec<EmbeddedDocument> {
    let mut nullable = VectorDocument::new("doc-null", "Document with nullable category")
        .with_metadata("owner", "acme");
    nullable.category = None;
    vec![
        EmbeddedDocument::new(nullable, vec![0.0; 4]),
        EmbeddedDocument::new(
            VectorDocument::new("doc-one", "One vector row")
                .with_category("reports")
                .with_metadata("owner", "acme"),
            vec![1.0; 4],
        ),
        EmbeddedDocument::new(
            VectorDocument::new("doc-two", "Two vector row")
                .with_category("archive")
                .with_metadata("owner", "example"),
            vec![2.0; 4],
        ),
    ]
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("separate copied fixture directory required")?);
    let database_path = root.join("lance-db");
    let uri = database_path.to_str().ok_or("UTF-8 database path")?;

    let connection = lancedb::connect(uri).execute().await?;
    let unrelated_schema = Arc::new(arrow_schema::Schema::new(vec![
        arrow_schema::Field::new("marker", arrow_schema::DataType::Utf8, false),
    ]));
    connection.create_empty_table("unrelated", unrelated_schema).execute().await?;
    drop(connection);

    let backend = LancedbBackend::build(
        uri,
        "cc_vectors",
        Arc::new(ZeroProvider),
        documents(),
    )
    .await?;
    let results = backend
        .search(VectorSearchParams::new("known zero query").with_limit(10))
        .await?;
    assert_eq!(results.items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(),
        ["doc-null", "doc-one", "doc-two"]);
    for ((item, distance), score) in results.items.iter()
        .zip([0.0_f32, 4.0, 16.0])
        .zip([1.0_f32, 1.0 / 5.0, 1.0 / 17.0])
    {
        assert!((item.distance - distance).abs() < 1e-5);
        assert!((item.score - score).abs() < 1e-5);
    }

    let category = backend.search(
        VectorSearchParams::new("known zero query").with_category("reports")
    ).await?;
    assert_eq!(category.items.len(), 1);
    assert_eq!(category.items[0].id, "doc-one");

    let threshold = backend.search(
        VectorSearchParams::new("known zero query").with_threshold(0.5)
    ).await?;
    assert_eq!(threshold.items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["doc-null"]);

    let metadata = backend.search(
        VectorSearchParams::new("known zero query").with_filter("owner", "acme")
    ).await?;
    assert_eq!(metadata.items.len(), 2);
    assert!(metadata.items.iter().all(|item| item.metadata["owner"] == "acme"));

    let farther_only = backend.search(
        VectorSearchParams::new("known zero query")
            .with_filter("owner", "example")
            .with_limit(1),
    ).await?;
    assert_eq!(farther_only.items.len(), 1);
    assert_eq!(farther_only.items[0].id, "doc-two");
    assert!((farther_only.items[0].distance - 16.0).abs() < 1e-5);
    assert!((farther_only.items[0].score - 1.0 / 17.0).abs() < 1e-5);
    let conjunction = backend.search(
        VectorSearchParams::new("known zero query")
            .with_category("archive")
            .with_filter("owner", "example"),
    ).await?;
    assert_eq!(conjunction.items.iter().map(|item| item.id.as_str()).collect::<Vec<_>>(), ["doc-two"]);
    let absent = backend.search(
        VectorSearchParams::new("known zero query").with_filter("missing-key", "value"),
    ).await?;
    assert!(absent.items.is_empty());

    drop(backend);
    let connection = lancedb::connect(uri).execute().await?;
    let tables = connection.table_names().execute().await?;
    assert!(tables.iter().any(|table| table == "cc_vectors"));
    assert!(tables.iter().any(|table| table == "unrelated"));

    // Ensure LanceDB can still query the rebuilt table through its public SDK.
    let table = connection.open_table("cc_vectors").execute().await?;
    let batches = table.query().execute().await?.try_collect::<Vec<_>>().await?;
    assert_eq!(batches.iter().map(|batch| batch.num_rows()).sum::<usize>(), 3);
    println!("product rebuild/query returned oracle distances and preserved the unrelated table on an old-fixture copy");
    Ok(())
}
