//! Handler functions for vectordb CLI commands.
//!
//! Provides the shared `get-model` handler that downloads and caches an
//! embedding model. Domain applications resolve model name and cache directory
//! from their own config before calling [`cmd_vectordb_get_model`].

use crate::cli::VectordbAction;
use fabryk_core::Result;
use fabryk_vector::{EmbeddingProvider, FastEmbedProvider};

// ============================================================================
// Command dispatch
// ============================================================================

/// Handle a vectordb subcommand using CLI args only.
///
/// When called from [`FabrykCli::run()`], the model name and cache directory
/// come from CLI args only. Domain applications that need config-based defaults
/// should call [`cmd_vectordb_get_model`] directly.
pub fn handle_vectordb_command(action: VectordbAction) -> Result<()> {
    match action {
        VectordbAction::GetModel { model, cache_dir } => {
            let model_name = model.as_deref().unwrap_or("bge-small-en-v1.5");
            cmd_vectordb_get_model(model_name, cache_dir.as_deref())
        }
    }
}

// ============================================================================
// Generic command handlers
// ============================================================================

/// Download and cache an embedding model.
///
/// Callers resolve `model_name` and `cache_dir` from their own config
/// before calling this function. This keeps config loading out of the
/// shared handler, matching the pattern in [`crate::config_handlers`].
pub fn cmd_vectordb_get_model(model_name: &str, cache_dir: Option<&str>) -> Result<()> {
    println!("Downloading embedding model '{model_name}' ...");

    let provider = FastEmbedProvider::new(model_name, cache_dir)?;

    let cache_display = cache_dir.unwrap_or("(fastembed default)");
    println!(
        "Model ready: name={model_name}, dimension={}, cache={cache_display}",
        provider.dimension()
    );
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Downloads model — run manually or in integration tests
    fn test_cmd_vectordb_get_model_default() {
        let result = cmd_vectordb_get_model("bge-small-en-v1.5", None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_vectordb_get_model_invalid_model() {
        let result = cmd_vectordb_get_model("nonexistent-model-xyz", None);
        assert!(result.is_err());
    }
}
