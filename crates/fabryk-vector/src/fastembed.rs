//! FastEmbed embedding provider.
//!
//! Wraps the `fastembed` crate to provide local embedding generation
//! via pre-trained models (e.g., BGE-small, AllMiniLM).
//!
//! # Thread Safety
//!
//! `fastembed::TextEmbedding` is not `Send + Sync`, so we wrap it in
//! `Arc<Mutex<>>` and use `tokio::task::spawn_blocking` for embedding calls.
//!
//! # Feature Gate
//!
//! This module requires the `vector-fastembed` feature.

use crate::embedding::EmbeddingProvider;
use async_trait::async_trait;
use fabryk_core::{Error, Result};
use std::sync::{Arc, Mutex};

/// Map a model name string to a fastembed `EmbeddingModel` enum variant.
fn resolve_model(name: &str) -> Result<fastembed::EmbeddingModel> {
    match name {
        "bge-small-en-v1.5" | "BGESmallENV15" => Ok(fastembed::EmbeddingModel::BGESmallENV15),
        "all-minilm-l6-v2" | "AllMiniLML6V2" => Ok(fastembed::EmbeddingModel::AllMiniLML6V2),
        "bge-base-en-v1.5" | "BGEBaseENV15" => Ok(fastembed::EmbeddingModel::BGEBaseENV15),
        "bge-large-en-v1.5" | "BGELargeENV15" => Ok(fastembed::EmbeddingModel::BGELargeENV15),
        other => Err(Error::config(format!(
            "Unknown embedding model: '{other}'. Supported: bge-small-en-v1.5, all-minilm-l6-v2, bge-base-en-v1.5, bge-large-en-v1.5"
        ))),
    }
}

/// FastEmbed-based embedding provider.
///
/// Uses locally-downloaded transformer models for embedding generation.
/// The model is loaded once and reused for all subsequent calls.
///
/// # Supported Models
///
/// | Name | Dimension | Size |
/// |------|-----------|------|
/// | `bge-small-en-v1.5` | 384 | ~50MB |
/// | `all-minilm-l6-v2` | 384 | ~80MB |
/// | `bge-base-en-v1.5` | 768 | ~130MB |
/// | `bge-large-en-v1.5` | 1024 | ~335MB |
pub struct FastEmbedProvider {
    model: Arc<Mutex<fastembed::TextEmbedding>>,
    dimension: usize,
    model_name: String,
}

impl FastEmbedProvider {
    /// Create a new FastEmbed provider with the given model name.
    ///
    /// Downloads the model if not cached locally. The cache directory
    /// can be configured via the `cache_path` parameter.
    ///
    /// # Arguments
    ///
    /// * `model_name` - Model identifier (e.g., "bge-small-en-v1.5")
    /// * `cache_path` - Optional directory for model file caching
    pub fn new(model_name: &str, cache_path: Option<&str>) -> Result<Self> {
        let model_enum = resolve_model(model_name)?;

        let mut init = fastembed::InitOptions::new(model_enum);
        if let Some(path) = cache_path {
            init = init.with_cache_dir(std::path::PathBuf::from(path));
        }

        let mut text_embedding = fastembed::TextEmbedding::try_new(init)
            .map_err(|e| Error::operation(format!("Failed to initialize fastembed model: {e}")))?;

        // Probe dimension via a test embedding
        let probe = text_embedding
            .embed(vec!["dimension probe"], None)
            .map_err(|e| Error::operation(format!("Failed to probe embedding dimension: {e}")))?;

        let dimension = probe
            .first()
            .map(|v| v.len())
            .ok_or_else(|| Error::operation("Empty probe embedding"))?;

        Ok(Self {
            model: Arc::new(Mutex::new(text_embedding)),
            dimension,
            model_name: model_name.to_string(),
        })
    }
}

#[async_trait]
impl EmbeddingProvider for FastEmbedProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let model = self.model.clone();
        let text = text.to_string();

        tokio::task::spawn_blocking(move || {
            let mut model = model
                .lock()
                .map_err(|e| Error::operation(format!("Mutex poisoned: {e}")))?;
            let results = model
                .embed(vec![text], None)
                .map_err(|e| Error::operation(format!("Embedding failed: {e}")))?;
            results
                .into_iter()
                .next()
                .ok_or_else(|| Error::operation("No embedding returned"))
        })
        .await
        .map_err(|e| Error::operation(format!("spawn_blocking failed: {e}")))?
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let model = self.model.clone();
        let texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();

        tokio::task::spawn_blocking(move || {
            let mut model = model
                .lock()
                .map_err(|e| Error::operation(format!("Mutex poisoned: {e}")))?;
            model
                .embed(texts, None)
                .map_err(|e| Error::operation(format!("Batch embedding failed: {e}")))
        })
        .await
        .map_err(|e| Error::operation(format!("spawn_blocking failed: {e}")))?
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn name(&self) -> &str {
        &self.model_name
    }
}

impl std::fmt::Debug for FastEmbedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastEmbedProvider")
            .field("model", &self.model_name)
            .field("dimension", &self.dimension)
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model_known() {
        assert!(resolve_model("bge-small-en-v1.5").is_ok());
        assert!(resolve_model("all-minilm-l6-v2").is_ok());
        assert!(resolve_model("bge-base-en-v1.5").is_ok());
        assert!(resolve_model("bge-large-en-v1.5").is_ok());
    }

    #[test]
    fn test_resolve_model_aliases() {
        assert!(resolve_model("BGESmallENV15").is_ok());
        assert!(resolve_model("AllMiniLML6V2").is_ok());
    }

    #[test]
    fn test_resolve_model_unknown() {
        let err = resolve_model("nonexistent-model").unwrap_err();
        assert!(err.to_string().contains("Unknown embedding model"));
    }

    // Integration tests requiring model download are gated with #[ignore]
    #[tokio::test]
    #[ignore = "requires model download (~50MB)"]
    async fn test_fastembed_provider_creation() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        assert_eq!(provider.dimension(), 384);
        assert_eq!(provider.name(), "bge-small-en-v1.5");
    }

    #[tokio::test]
    #[ignore = "requires model download (~50MB)"]
    async fn test_fastembed_embed_single() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let embedding = provider.embed("Hello world").await.unwrap();
        assert_eq!(embedding.len(), 384);

        // Should be normalized
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.1);
    }

    #[tokio::test]
    #[ignore = "requires model download (~50MB)"]
    async fn test_fastembed_embed_batch() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let texts = vec!["Hello", "World", "Test"];
        let embeddings = provider.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for emb in &embeddings {
            assert_eq!(emb.len(), 384);
        }
    }

    #[tokio::test]
    #[ignore = "requires model download (~50MB)"]
    async fn test_fastembed_deterministic() {
        let provider = FastEmbedProvider::new("bge-small-en-v1.5", None).unwrap();
        let e1 = provider.embed("same text").await.unwrap();
        let e2 = provider.embed("same text").await.unwrap();
        assert_eq!(e1, e2);
    }
}
