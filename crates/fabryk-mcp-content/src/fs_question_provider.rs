//! Filesystem-backed question search provider.
//!
//! [`FsQuestionSearchProvider`] scans markdown concept cards for
//! `answers_questions` frontmatter and implements fuzzy question matching
//! via [`QuestionSearchProvider`].
//!
//! # How it Works
//!
//! Each concept card can declare questions it answers in its YAML frontmatter:
//!
//! ```yaml
//! answers_questions:
//!   - What is voice leading?
//!   - How do I connect chords smoothly?
//! ```
//!
//! When a user asks a question, this provider fuzzy-matches the query against
//! all declared questions using normalized Damerau-Levenshtein distance, with
//! an optional boost for substring containment.
//!
//! # Example
//!
//! ```rust,ignore
//! use fabryk_mcp_content::FsQuestionSearchProvider;
//!
//! let provider = FsQuestionSearchProvider::new("/path/to/concepts")
//!     .with_threshold(0.3)
//!     .with_substring_boost(0.3);
//!
//! let results = provider.search_by_question("What is voice leading?", 10).await?;
//! ```

use async_trait::async_trait;
use std::path::PathBuf;

use fabryk_content::{ConceptCardFrontmatter, extract_frontmatter};
use fabryk_core::Result;
use fabryk_core::util::files::{FindOptions, find_all_files, read_file};

use crate::traits::{QuestionMatch, QuestionSearchProvider, QuestionSearchResponse};

// ============================================================================
// Provider
// ============================================================================

/// Filesystem-backed implementation of [`QuestionSearchProvider`].
///
/// Scans markdown files for `answers_questions` frontmatter fields and
/// performs fuzzy matching against user queries.
pub struct FsQuestionSearchProvider {
    /// Root path to content files.
    content_path: PathBuf,
    /// Minimum similarity score to include in results.
    threshold: f64,
    /// Bonus added when the query is a substring of the question or vice versa.
    substring_boost: f64,
}

impl FsQuestionSearchProvider {
    /// Create a new provider rooted at the given content directory.
    pub fn new(content_path: impl Into<PathBuf>) -> Self {
        Self {
            content_path: content_path.into(),
            threshold: 0.3,
            substring_boost: 0.3,
        }
    }

    /// Set the minimum similarity threshold (0.0 to 1.0).
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.threshold = threshold;
        self
    }

    /// Set the substring containment boost (0.0 to 1.0).
    pub fn with_substring_boost(mut self, boost: f64) -> Self {
        self.substring_boost = boost;
        self
    }
}

#[async_trait]
impl QuestionSearchProvider for FsQuestionSearchProvider {
    async fn search_by_question(
        &self,
        question: &str,
        limit: usize,
    ) -> Result<QuestionSearchResponse> {
        let files = find_all_files(&self.content_path, FindOptions::markdown()).await?;
        let query_lower = question.to_lowercase();
        let mut matches = Vec::new();

        for file_info in &files {
            let content = read_file(&file_info.path).await?;

            // Parse frontmatter
            let fm = match extract_frontmatter(&content) {
                Ok(result) => match result.deserialize::<ConceptCardFrontmatter>() {
                    Ok(Some(fm)) => fm,
                    _ => continue,
                },
                Err(_) => continue,
            };

            // Skip files without questions
            if fm.answers_questions.is_empty() {
                continue;
            }

            let item_id = file_info.stem.clone();
            let item_title = fm
                .title
                .or(fm.concept)
                .unwrap_or_else(|| file_info.stem.clone());
            let category = fm
                .category
                .or_else(|| {
                    file_info
                        .relative_path
                        .parent()
                        .and_then(|p| p.to_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "uncategorized".to_string());
            let tier = fm.tier;

            for q in &fm.answers_questions {
                let q_lower = q.to_lowercase();
                let mut similarity = strsim::normalized_damerau_levenshtein(&query_lower, &q_lower);

                // Apply substring boost
                if query_lower.contains(&q_lower) || q_lower.contains(&query_lower) {
                    similarity = (similarity + self.substring_boost).min(1.0);
                }

                if similarity > self.threshold {
                    matches.push(QuestionMatch {
                        item_id: item_id.clone(),
                        item_title: item_title.clone(),
                        matched_question: q.clone(),
                        category: category.clone(),
                        tier: tier.clone(),
                        similarity,
                    });
                }
            }
        }

        // Sort by similarity descending
        matches.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = matches.len();
        matches.truncate(limit);

        Ok(QuestionSearchResponse {
            matches,
            total,
            query: question.to_string(),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    /// Write a concept card with answers_questions frontmatter.
    async fn write_card(
        dir: &std::path::Path,
        filename: &str,
        title: &str,
        category: &str,
        tier: Option<&str>,
        questions: &[&str],
    ) {
        let tier_line = tier.map(|t| format!("tier: {t}\n")).unwrap_or_default();
        let questions_yaml = if questions.is_empty() {
            String::new()
        } else {
            let items: Vec<String> = questions.iter().map(|q| format!("  - {q}")).collect();
            format!("answers_questions:\n{}\n", items.join("\n"))
        };

        let content = format!(
            "---\ntitle: {title}\ncategory: {category}\n{tier_line}{questions_yaml}---\n\nContent for {title}.",
        );
        fs::write(dir.join(filename), content).await.unwrap();
    }

    /// Build a populated temp directory with concept cards.
    async fn setup_content_dir() -> TempDir {
        let temp = TempDir::new().unwrap();

        let harmony = temp.path().join("harmony");
        fs::create_dir(&harmony).await.unwrap();

        write_card(
            &harmony,
            "voice-leading.md",
            "Voice Leading",
            "harmony",
            Some("foundational"),
            &[
                "What is voice leading?",
                "How do I connect chords smoothly?",
            ],
        )
        .await;

        write_card(
            &harmony,
            "tritone-sub.md",
            "Tritone Substitution",
            "harmony",
            Some("advanced"),
            &["What is a tritone substitution?"],
        )
        .await;

        let rhythm = temp.path().join("rhythm");
        fs::create_dir(&rhythm).await.unwrap();

        write_card(
            &rhythm,
            "syncopation.md",
            "Syncopation",
            "rhythm",
            None,
            &["What is syncopation?", "How does syncopation work?"],
        )
        .await;

        // Card with no questions (should be skipped)
        write_card(&rhythm, "tempo.md", "Tempo", "rhythm", None, &[]).await;

        temp
    }

    #[tokio::test]
    async fn test_exact_match_returns_high_similarity() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path());

        let response = provider
            .search_by_question("What is voice leading?", 10)
            .await
            .unwrap();

        assert!(!response.matches.is_empty());
        let top = &response.matches[0];
        assert_eq!(top.item_id, "voice-leading");
        // Exact match (case-insensitive) + substring boost should be very high
        assert!(top.similarity > 0.9);
    }

    #[tokio::test]
    async fn test_substring_match_gets_boost() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path()).with_substring_boost(0.3);

        let response = provider
            .search_by_question("voice leading", 10)
            .await
            .unwrap();

        // "voice leading" is a substring of "What is voice leading?"
        let vl_match = response
            .matches
            .iter()
            .find(|m| m.item_id == "voice-leading" && m.matched_question.contains("What is voice"));

        assert!(
            vl_match.is_some(),
            "Should find voice leading via substring"
        );
    }

    #[tokio::test]
    async fn test_below_threshold_filtered() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path()).with_threshold(0.99);

        let response = provider
            .search_by_question("completely unrelated query about cooking", 10)
            .await
            .unwrap();

        assert!(
            response.matches.is_empty(),
            "Very different query with high threshold should yield no matches"
        );
    }

    #[tokio::test]
    async fn test_results_sorted_descending() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path()).with_threshold(0.1);

        let response = provider
            .search_by_question("What is syncopation?", 100)
            .await
            .unwrap();

        assert!(response.matches.len() > 1);
        for window in response.matches.windows(2) {
            assert!(
                window[0].similarity >= window[1].similarity,
                "Results should be sorted by descending similarity"
            );
        }
    }

    #[tokio::test]
    async fn test_limit_respected() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path()).with_threshold(0.1);

        let response = provider.search_by_question("What is", 2).await.unwrap();

        assert!(response.matches.len() <= 2);
        // Total may be larger than the returned matches
        assert!(response.total >= response.matches.len());
    }

    #[tokio::test]
    async fn test_empty_answers_questions_skipped() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path());

        let response = provider.search_by_question("tempo", 100).await.unwrap();

        // "tempo" card has no answers_questions, so it should never appear
        assert!(
            !response.matches.iter().any(|m| m.item_id == "tempo"),
            "Cards without answers_questions should not appear in results"
        );
    }

    #[tokio::test]
    async fn test_response_query_preserved() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path());

        let query = "What is voice leading?";
        let response = provider.search_by_question(query, 10).await.unwrap();

        assert_eq!(response.query, query);
    }

    #[tokio::test]
    async fn test_empty_content_directory() {
        let temp = TempDir::new().unwrap();
        let provider = FsQuestionSearchProvider::new(temp.path());

        let response = provider.search_by_question("anything", 10).await.unwrap();

        assert!(response.matches.is_empty());
        assert_eq!(response.total, 0);
    }

    #[tokio::test]
    async fn test_tier_included_in_match() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path());

        let response = provider
            .search_by_question("What is a tritone substitution?", 10)
            .await
            .unwrap();

        let top = &response.matches[0];
        assert_eq!(top.tier, Some("advanced".to_string()));
    }

    #[tokio::test]
    async fn test_no_tier_returns_none() {
        let temp = setup_content_dir().await;
        let provider = FsQuestionSearchProvider::new(temp.path());

        let response = provider
            .search_by_question("What is syncopation?", 10)
            .await
            .unwrap();

        let sync_match = response
            .matches
            .iter()
            .find(|m| m.item_id == "syncopation")
            .unwrap();

        assert!(sync_match.tier.is_none());
    }

    #[tokio::test]
    async fn test_custom_threshold() {
        let temp = setup_content_dir().await;

        // Very high threshold
        let strict = FsQuestionSearchProvider::new(temp.path()).with_threshold(0.95);
        let response = strict
            .search_by_question("What is voice leading?", 100)
            .await
            .unwrap();

        let strict_count = response.total;

        // Low threshold
        let lenient = FsQuestionSearchProvider::new(temp.path()).with_threshold(0.1);
        let response = lenient
            .search_by_question("What is voice leading?", 100)
            .await
            .unwrap();

        let lenient_count = response.total;

        assert!(
            lenient_count >= strict_count,
            "Lower threshold should yield at least as many results"
        );
    }
}
