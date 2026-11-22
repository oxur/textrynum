//! Independent review oracle for literal MAX with more than two metadata batches.
use async_trait::async_trait;
use fabryk_core::Result as FabrykResult;
use fabryk_vector::{EmbeddedDocument, EmbeddingProvider, LancedbBackend, VectorBackend, VectorDocument, VectorSearchParams};
use std::{collections::HashSet, sync::Arc};

struct ZeroProvider;
#[async_trait]
impl EmbeddingProvider for ZeroProvider {
    async fn embed(&self, _: &str) -> FabrykResult<Vec<f32>> { Ok(vec![0.0; 4]) }
    fn dimension(&self) -> usize { 4 }
    fn name(&self) -> &str { "independent-max-oracle" }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("owned empty database path required")?;
    let documents = (0..260).map(|index| {
        EmbeddedDocument::new(
            VectorDocument::new(format!("row-{index:03}"), "Independent MAX fixture")
                .with_category("reports").with_metadata("owner", "acme"),
            if index == 259 { vec![0.0; 4] } else { vec![1.0; 4] },
        )
    }).collect();
    let backend = LancedbBackend::build(&path, "max_review", Arc::new(ZeroProvider), documents).await?;
    let results = backend.search(
        VectorSearchParams::new("zero query").with_category("reports")
            .with_filter("owner", "acme").with_limit(usize::MAX),
    ).await?;
    assert_eq!(results.total, 260);
    assert_eq!(results.items.len(), 260);
    let ids = results.items.iter().map(|item| item.id.clone()).collect::<HashSet<_>>();
    assert_eq!(ids, (0..260).map(|index| format!("row-{index:03}")).collect::<HashSet<_>>());
    assert_eq!(results.items[0].id, "row-259");
    for item in &results.items {
        let distance = if item.id == "row-259" { 0.0 } else { 4.0 };
        assert!((item.distance - distance).abs() < 1e-5);
        assert!((item.score - 1.0 / (1.0 + distance)).abs() < 1e-5);
        assert_eq!(item.metadata["owner"], "acme");
    }
    assert!(results.items.windows(2).all(|pair| pair[0].score >= pair[1].score));
    let absent = backend.search(VectorSearchParams::new("zero query")
        .with_filter("absent", "value").with_limit(usize::MAX)).await?;
    assert_eq!(absent.total, 0);
    assert!(absent.items.is_empty());
    let zero = backend.search(VectorSearchParams::new("zero query")
        .with_filter("owner", "acme").with_limit(0)).await?;
    assert_eq!(zero.total, 0);
    assert!(zero.items.is_empty());
    println!("literal usize::MAX returned all260 exact matching IDs across128-row batches, native L2 scores/order and late winner; MAX/no-match and zero-limit also passed");
    Ok(())
}
