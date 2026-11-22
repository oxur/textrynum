use async_trait::async_trait;
use fabryk_core::Result as FabrykResult;
use fabryk_vector::persistence::{save_metadata, IndexMetadata};
use fabryk_vector::{EmbeddedDocument, EmbeddingProvider, LancedbBackend, VectorDocument};
use std::path::PathBuf;
use std::sync::Arc;

struct ZeroProvider;

#[async_trait]
impl EmbeddingProvider for ZeroProvider {
    async fn embed(&self, _text: &str) -> FabrykResult<Vec<f32>> { Ok(vec![0.0; 4]) }
    fn dimension(&self) -> usize { 4 }
    fn name(&self) -> &str { "cc-zero-oracle" }
}

fn docs() -> Vec<EmbeddedDocument> {
    let mut nullable = VectorDocument::new("doc-null", "Document with nullable category")
        .with_metadata("owner", "acme");
    nullable.category = None;
    vec![
        EmbeddedDocument::new(nullable, vec![0.0, 0.0, 0.0, 0.0]),
        EmbeddedDocument::new(
            VectorDocument::new("doc-one", "One vector row")
                .with_category("reports")
                .with_metadata("owner", "acme"),
            vec![1.0, 1.0, 1.0, 1.0],
        ),
        EmbeddedDocument::new(
            VectorDocument::new("doc-two", "Two vector row")
                .with_category("archive")
                .with_metadata("owner", "example"),
            vec![2.0, 2.0, 2.0, 2.0],
        ),
    ]
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or_else(|| std::io::Error::other("fixture directory required"))?);
    std::fs::create_dir_all(&root)?;
    let db = root.join("lance-db");
    let provider: Arc<dyn EmbeddingProvider> = Arc::new(ZeroProvider);
    let backend = LancedbBackend::build(
        db.to_str().ok_or_else(|| std::io::Error::other("UTF-8 fixture path"))?,
        "cc_vectors",
        provider,
        docs(),
    ).await?;
    drop(backend);
    let metadata = IndexMetadata {
        content_hash: "golden-lance-content-hash-42".to_string(),
        document_count: 3,
        embedding_dimension: 4,
        built_at: "2026-03-13T10:00:00Z".to_string(),
        provider: "cc-zero-oracle".to_string(),
        model: "fixed-4d-fixture-v1".to_string(),
    };
    save_metadata(&root.join("index_metadata.json"), &metadata)?;
    println!("wrote baseline Lance table=cc_vectors rows=3 dims=4 path={}", root.display());
    Ok(())
}
