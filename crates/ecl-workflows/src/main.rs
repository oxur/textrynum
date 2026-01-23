//! ECL Workflows service entry point.
//!
//! Note: Full Restate HTTP server integration deferred pending SDK 0.7 API verification.
//! This main creates the workflow service and demonstrates the setup pattern.

use ecl_core::llm::{ClaudeProvider, MockLlmProvider, RetryWrapper};
use std::sync::Arc;

mod simple;
use simple::SimpleWorkflowService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,ecl=debug".into()),
        )
        .init();

    tracing::info!("ECL Workflows service (Phase 1 - pre-Restate)");

    // Load configuration
    let use_mock = std::env::var("USE_MOCK_LLM").unwrap_or_else(|_| "true".to_string()) == "true";

    let llm: Arc<dyn ecl_core::llm::LlmProvider> = if use_mock {
        tracing::info!("Using mock LLM provider");
        Arc::new(MockLlmProvider::with_response(
            "This is a mock response for testing.",
        ))
    } else {
        let api_key = match std::env::var("ANTHROPIC_API_KEY") {
            Ok(key) => key,
            Err(_) => {
                tracing::error!("ANTHROPIC_API_KEY environment variable must be set");
                return Err(anyhow::anyhow!(
                    "ANTHROPIC_API_KEY environment variable required when USE_MOCK_LLM=false"
                ));
            }
        };
        let model = std::env::var("ANTHROPIC_MODEL")
            .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

        tracing::info!(model = %model, "Using Claude LLM provider");
        let claude = ClaudeProvider::new(api_key, model);
        Arc::new(RetryWrapper::new(Arc::new(claude)))
    };

    // Create workflow service
    let service = SimpleWorkflowService::new(llm);

    // Run a test workflow
    let input = simple::SimpleWorkflowInput::new("Benefits of Rust programming");

    tracing::info!("Running test workflow");
    match service.run_simple(input).await {
        Ok(output) => {
            tracing::info!("Workflow completed successfully");
            tracing::info!("Generated: {}", output.generated_text);
            tracing::info!("Critique: {}", output.critique);
        }
        Err(e) => {
            tracing::error!("Workflow failed: {}", e);
            return Err(e.into());
        }
    }

    tracing::info!("ECL Workflows service completed");
    Ok(())
}
