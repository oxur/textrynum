use fabryk_fts::{SearchBackend, SearchConfig, SearchParams, TantivySearch};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(std::env::args().nth(1).ok_or("copied index directory required")?);
    let config = SearchConfig {
        backend: "tantivy".to_string(),
        index_path: Some(path.to_string_lossy().into_owned()),
        ..Default::default()
    };
    let search = TantivySearch::new(&config)?;

    let stemming = search
        .search(SearchParams {
            query: "harmonics".to_string(),
            limit: Some(10),
            ..Default::default()
        })
        .await?;
    assert_eq!(stemming.total, 2);
    assert_eq!(stemming.items.len(), 2);
    let ids: std::collections::BTreeSet<&str> = stemming.items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, ["doc-cadence", "doc-harmony"].into_iter().collect());
    let harmony = stemming.items.iter().find(|item| item.id == "doc-harmony").unwrap();
    assert_eq!(harmony.title, "Harmonics and harmony");
    assert_eq!(harmony.description.as_deref(), Some("Golden description for doc-harmony"));
    assert_eq!(harmony.category, "harmony");
    assert_eq!(harmony.source.as_deref(), Some("Acme Inc. fixture"));
    assert_eq!(harmony.path.as_deref(), Some("fixture/doc-harmony.md"));
    assert_eq!(harmony.content_type.as_deref(), Some("concept"));
    assert_eq!(harmony.chapter.as_deref(), Some("chapter-7"));
    assert_eq!(harmony.section.as_deref(), Some("stored-section"));
    assert!(harmony.snippet.as_deref().is_some_and(|snippet| snippet.contains("Harmonics")));
    assert!(harmony.relevance.is_finite() && harmony.relevance > 0.0);
    let cadence = stemming.items.iter().find(|item| item.id == "doc-cadence").unwrap();
    assert_eq!(cadence.title, "Cadence resolution");
    assert_eq!(cadence.description.as_deref(), Some("Golden description for doc-cadence"));
    assert_eq!(cadence.category, "harmony");
    assert_eq!(cadence.source.as_deref(), Some("Acme Inc. fixture"));
    assert_eq!(cadence.path.as_deref(), Some("fixture/doc-cadence.md"));
    assert_eq!(cadence.content_type.as_deref(), Some("concept"));
    assert_eq!(cadence.chapter.as_deref(), Some("chapter-7"));
    assert_eq!(cadence.section.as_deref(), Some("stored-section"));
    assert_eq!(cadence.snippet.as_deref(), Some("Golden description for doc-cadence"));
    assert!(cadence.relevance.is_finite() && cadence.relevance > 0.0);

    let rhythm = search.search(SearchParams {
        query: "rhythmic".to_string(),
        limit: Some(10),
        ..Default::default()
    }).await?;
    assert_eq!(rhythm.total, 1);
    assert_eq!(rhythm.items.len(), 1);
    let rhythm = &rhythm.items[0];
    assert_eq!(rhythm.id, "doc-rhythm");
    assert_eq!(rhythm.title, "Rhythmic patterns");
    assert_eq!(rhythm.description.as_deref(), Some("Golden description for doc-rhythm"));
    assert_eq!(rhythm.category, "rhythm");
    assert_eq!(rhythm.source.as_deref(), Some("Acme Inc. fixture"));
    assert_eq!(rhythm.path.as_deref(), Some("fixture/doc-rhythm.md"));
    assert_eq!(rhythm.content_type.as_deref(), Some("concept"));
    assert_eq!(rhythm.chapter.as_deref(), Some("chapter-7"));
    assert_eq!(rhythm.section.as_deref(), Some("stored-section"));
    assert_eq!(rhythm.snippet.as_deref(), Some("A rhythmic pulse organizes measures"));
    assert!(rhythm.relevance.is_finite() && rhythm.relevance > 0.0);

    let category = search
        .search(SearchParams {
            query: "harmonic".to_string(),
            category: Some("harmony".to_string()),
            limit: Some(10),
            ..Default::default()
        })
        .await?;
    assert_eq!(category.items.len(), 2);
    assert!(category.items.iter().all(|item| item.category == "harmony"));
    assert!(category.items.iter().any(|item| item.id == "doc-cadence"));

    drop(search);
    println!("upgraded Tantivy product reader retrieved each of three old documents exactly once across harmonics/rhythmic searches, with every SearchResult field asserted, plus stemming and category filtering");
    Ok(())
}
