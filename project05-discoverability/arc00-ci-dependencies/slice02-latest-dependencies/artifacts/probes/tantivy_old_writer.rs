use fabryk_fts::{Indexer, SearchDocument, SearchSchema};
use std::path::PathBuf;

fn document(id: &str, title: &str, content: &str, category: &str, author: &str) -> SearchDocument {
    SearchDocument {
        id: id.to_string(),
        path: format!("fixture/{id}.md"),
        title: title.to_string(),
        description: Some(format!("Golden description for {id}")),
        content: content.to_string(),
        category: category.to_string(),
        source: Some("Acme Inc. fixture".to_string()),
        tags: vec!["compatibility".to_string()],
        chapter: Some("chapter-7".to_string()),
        part: None,
        author: Some(author.to_string()),
        date: Some("2026-03-13".to_string()),
        content_type: Some("concept".to_string()),
        section: Some("stored-section".to_string()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("index directory required")?);
    let schema = SearchSchema::build();
    let mut writer = Indexer::new(&root, &schema)?;
    writer.add_document(&document(
        "doc-harmony", "Harmonics and harmony", "Harmonics support harmonic progressions", "harmony", "A. Example",
    ))?;
    writer.add_document(&document(
        "doc-rhythm", "Rhythmic patterns", "A rhythmic pulse organizes measures", "rhythm", "B. Example",
    ))?;
    writer.add_document(&document(
        "doc-cadence", "Cadence resolution", "Harmonic cadences resolve phrases", "harmony", "C. Example",
    ))?;
    writer.commit()?;
    drop(writer);
    println!("wrote baseline Tantivy index documents=3 path={}", root.display());
    Ok(())
}
