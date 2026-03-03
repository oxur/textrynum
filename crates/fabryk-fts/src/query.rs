//! Query building with weighted multi-field search.
//!
//! This module provides `QueryBuilder` for constructing Tantivy queries with:
//! - Field-specific boost weights
//! - Phrase query support
//! - Query mode selection (AND, OR, Smart)
//! - Optional fuzzy matching
//! - Stopword filtering
//!
//! # Query Modes
//!
//! - **Smart** (default): AND for 1-2 terms, OR with minimum match for 3+
//! - **And**: All terms must match
//! - **Or**: Any term can match
//! - **MinimumMatch**: At least N terms must match
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_fts::{QueryBuilder, SearchSchema, SearchConfig};
//!
//! let schema = SearchSchema::build();
//! let config = SearchConfig::default();
//! let builder = QueryBuilder::new(&schema, &config);
//!
//! let query = builder.build_query("functional harmony")?;
//! ```

use fabryk_core::Result;
use tantivy::Term;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery};
use tantivy::schema::IndexRecordOption;
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, Stemmer, TextAnalyzer, TokenStream};

use crate::schema::SearchSchema;
use crate::stopwords::StopwordFilter;
use crate::types::{QueryMode, SearchConfig};

/// Query builder for constructing Tantivy queries.
pub struct QueryBuilder<'a> {
    schema: &'a SearchSchema,
    config: &'a SearchConfig,
    stopword_filter: StopwordFilter,
}

impl<'a> QueryBuilder<'a> {
    /// Create a new query builder.
    pub fn new(schema: &'a SearchSchema, config: &'a SearchConfig) -> Self {
        let stopword_filter = StopwordFilter::new(config);
        Self {
            schema,
            config,
            stopword_filter,
        }
    }

    /// Build a query from a search string.
    ///
    /// Handles:
    /// - Quoted phrases ("exact phrase")
    /// - Multiple terms with configurable AND/OR logic
    /// - Field-specific boost weights
    /// - Optional fuzzy matching
    pub fn build_query(&self, query_str: &str) -> Result<Box<dyn Query>> {
        let query_str = query_str.trim();

        // Handle empty/wildcard queries
        if query_str.is_empty() || query_str == "*" {
            return Ok(Box::new(tantivy::query::AllQuery));
        }

        // Filter stopwords
        let filtered = self.stopword_filter.filter(query_str);

        // Extract phrases
        let (phrases, remaining) = parse_phrases(&filtered);

        // Parse remaining terms
        let terms: Vec<&str> = remaining.split_whitespace().collect();

        // Build subqueries for each field with boost
        let mut field_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for (field, boost) in self.schema.full_text_fields() {
            let mut term_queries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

            // Add phrase queries
            for phrase in &phrases {
                if let Some(pq) = self.create_phrase_query(field, phrase) {
                    term_queries.push((Occur::Should, Box::new(BoostQuery::new(pq, boost))));
                }
            }

            // Add term queries
            let occur = self.determine_occur_mode(&terms);
            for term in &terms {
                let tq = self.create_term_query(field, term);
                term_queries.push((occur, Box::new(BoostQuery::new(tq, boost))));
            }

            if !term_queries.is_empty() {
                let field_query = BooleanQuery::new(term_queries);
                field_queries.push((Occur::Should, Box::new(field_query)));
            }
        }

        if field_queries.is_empty() {
            return Ok(Box::new(tantivy::query::AllQuery));
        }

        Ok(Box::new(BooleanQuery::new(field_queries)))
    }

    /// Determine the occur mode based on config and term count.
    fn determine_occur_mode(&self, terms: &[&str]) -> Occur {
        match self.config.query_mode {
            QueryMode::And => Occur::Must,
            QueryMode::Or => Occur::Should,
            QueryMode::Smart => {
                if terms.len() <= 2 {
                    Occur::Must // AND for short queries
                } else {
                    Occur::Should // OR for longer queries
                }
            }
            QueryMode::MinimumMatch => Occur::Should,
        }
    }

    /// Tokenize text through the same analyzer used for indexing.
    ///
    /// Returns stemmed/lowercased tokens (e.g., "harmony" → "harmoni").
    fn analyze(&self, text: &str) -> Vec<String> {
        let mut analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(tantivy::tokenizer::Language::English))
            .build();
        let mut tokens = Vec::new();
        let mut stream = analyzer.token_stream(text);
        while let Some(token) = stream.next() {
            tokens.push(token.text.clone());
        }
        tokens
    }

    /// Create a phrase query for exact matching.
    fn create_phrase_query(
        &self,
        field: tantivy::schema::Field,
        phrase: &str,
    ) -> Option<Box<dyn Query>> {
        let terms: Vec<Term> = self
            .analyze(phrase)
            .into_iter()
            .map(|tok| Term::from_field_text(field, &tok))
            .collect();

        if terms.is_empty() {
            return None;
        }

        if terms.len() == 1 {
            return Some(Box::new(TermQuery::new(
                terms[0].clone(),
                IndexRecordOption::WithFreqs,
            )));
        }

        Some(Box::new(tantivy::query::PhraseQuery::new(terms)))
    }

    /// Create a term query (optionally fuzzy).
    fn create_term_query(&self, field: tantivy::schema::Field, term: &str) -> Box<dyn Query> {
        // Analyze through the same tokenizer used for indexing
        let analyzed = self.analyze(term);
        let token = analyzed.first().map(|s| s.as_str()).unwrap_or(term);
        let term_obj = Term::from_field_text(field, token);

        if self.config.fuzzy_enabled && term.len() >= 4 {
            Box::new(tantivy::query::FuzzyTermQuery::new(
                term_obj,
                self.config.fuzzy_distance,
                true, // transposition
            ))
        } else {
            Box::new(TermQuery::new(term_obj, IndexRecordOption::WithFreqs))
        }
    }
}

/// Parse quoted phrases from a query string.
///
/// Returns (phrases, remaining text without quotes).
fn parse_phrases(query: &str) -> (Vec<String>, String) {
    let mut phrases = Vec::new();
    let mut remaining = query.to_string();

    while let Some(start) = remaining.find('"') {
        if let Some(end) = remaining[start + 1..].find('"') {
            let phrase = remaining[start + 1..start + 1 + end].trim().to_string();
            if !phrase.is_empty() {
                phrases.push(phrase);
            }
            remaining = format!("{}{}", &remaining[..start], &remaining[start + end + 2..]);
        } else {
            break;
        }
    }

    (phrases, remaining)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schema() -> SearchSchema {
        SearchSchema::build()
    }

    fn test_builder() -> QueryBuilder<'static> {
        static SCHEMA: std::sync::OnceLock<SearchSchema> = std::sync::OnceLock::new();
        static CONFIG: std::sync::OnceLock<SearchConfig> = std::sync::OnceLock::new();

        let schema = SCHEMA.get_or_init(SearchSchema::build);
        let config = CONFIG.get_or_init(SearchConfig::default);

        QueryBuilder::new(schema, config)
    }

    #[test]
    fn test_build_simple_query() {
        let builder = test_builder();
        let query = builder.build_query("harmony");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_multi_term_query() {
        let builder = test_builder();
        let query = builder.build_query("functional harmony");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_phrase_query() {
        let builder = test_builder();
        let query = builder.build_query("\"functional harmony\"");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_empty_query() {
        let builder = test_builder();
        let query = builder.build_query("");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_wildcard_query() {
        let builder = test_builder();
        let query = builder.build_query("*");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_whitespace_only_query() {
        let builder = test_builder();
        let query = builder.build_query("   ");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_query_with_fuzzy() {
        let config = SearchConfig {
            fuzzy_enabled: true,
            fuzzy_distance: 1,
            ..Default::default()
        };
        let schema = test_schema();
        let builder = QueryBuilder::new(&schema, &config);
        let query = builder.build_query("harmonics");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_query_short_term_no_fuzzy() {
        // Short terms (<4 chars) should not use fuzzy even when enabled
        let config = SearchConfig {
            fuzzy_enabled: true,
            ..Default::default()
        };
        let schema = test_schema();
        let builder = QueryBuilder::new(&schema, &config);
        let query = builder.build_query("key");
        assert!(query.is_ok());
    }

    #[test]
    fn test_parse_phrases_single() {
        let (phrases, remaining) = parse_phrases("\"exact phrase\" other");
        assert_eq!(phrases, vec!["exact phrase"]);
        assert!(remaining.contains("other"));
    }

    #[test]
    fn test_parse_phrases_multiple() {
        let (phrases, remaining) = parse_phrases("\"one\" word \"two\"");
        assert_eq!(phrases.len(), 2);
        assert!(remaining.contains("word"));
    }

    #[test]
    fn test_parse_phrases_none() {
        let (phrases, remaining) = parse_phrases("no phrases here");
        assert!(phrases.is_empty());
        assert_eq!(remaining.trim(), "no phrases here");
    }

    #[test]
    fn test_parse_phrases_empty_quotes() {
        let (phrases, remaining) = parse_phrases("\"\" something");
        assert!(phrases.is_empty());
        assert!(remaining.contains("something"));
    }

    #[test]
    fn test_parse_phrases_unclosed_quote() {
        let (phrases, remaining) = parse_phrases("\"unclosed phrase");
        assert!(phrases.is_empty());
        assert!(remaining.contains("unclosed"));
    }

    #[test]
    fn test_determine_occur_mode_smart() {
        let builder = test_builder();

        // Short query: AND
        let occur = builder.determine_occur_mode(&["one", "two"]);
        assert_eq!(occur, Occur::Must);

        // Long query: OR
        let occur = builder.determine_occur_mode(&["one", "two", "three"]);
        assert_eq!(occur, Occur::Should);
    }

    #[test]
    fn test_determine_occur_mode_and() {
        let config = SearchConfig {
            query_mode: QueryMode::And,
            ..Default::default()
        };
        let schema = test_schema();
        let builder = QueryBuilder::new(&schema, &config);

        let occur = builder.determine_occur_mode(&["one", "two", "three"]);
        assert_eq!(occur, Occur::Must);
    }

    #[test]
    fn test_determine_occur_mode_or() {
        let config = SearchConfig {
            query_mode: QueryMode::Or,
            ..Default::default()
        };
        let schema = test_schema();
        let builder = QueryBuilder::new(&schema, &config);

        let occur = builder.determine_occur_mode(&["one"]);
        assert_eq!(occur, Occur::Should);
    }

    #[test]
    fn test_determine_occur_mode_minimum_match() {
        let config = SearchConfig {
            query_mode: QueryMode::MinimumMatch,
            ..Default::default()
        };
        let schema = test_schema();
        let builder = QueryBuilder::new(&schema, &config);

        let occur = builder.determine_occur_mode(&["one", "two"]);
        assert_eq!(occur, Occur::Should);
    }

    #[test]
    fn test_build_mixed_phrase_and_terms() {
        let builder = test_builder();
        let query = builder.build_query("\"chord progression\" functional harmony");
        assert!(query.is_ok());
    }

    #[test]
    fn test_build_query_with_stopwords() {
        let builder = test_builder();
        // "what is a" should be filtered, leaving "cadence"
        let query = builder.build_query("what is a cadence");
        assert!(query.is_ok());
    }
}
