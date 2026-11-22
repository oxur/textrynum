use fabryk_fts::{SearchConfig, StopwordFilter};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let oracle_path = PathBuf::from(std::env::args().nth(1).ok_or("baseline stop-words 0.10.0 ISO JSON path required")?);
    let source: serde_json::Value = serde_json::from_slice(&std::fs::read(oracle_path)?)?;
    let english = source["en"].as_array().ok_or("baseline ISO English array missing")?;
    assert_eq!(english.len(), 1298);
    let filter = StopwordFilter::new(&SearchConfig::default());
    for entry in english {
        let word = entry.as_str().ok_or("baseline ISO entry is not a string")?;
        assert!(filter.is_stopword(word), "product API omitted baseline ISO word {word}");
    }
    for word in ["able", "across", "therefore", "system"] {
        assert!(filter.is_stopword(word), "ISO-only control {word}");
    }
    for word in ["ain", "hadn", "mightn", "mustn", "needn", "shan"] {
        assert!(!filter.is_stopword(word), "NLTK-only control {word}");
    }
    println!("product StopwordFilter matched all 1298 baseline stop-words 0.10.0 ISO English entries and six NLTK-only exclusions");
    Ok(())
}
