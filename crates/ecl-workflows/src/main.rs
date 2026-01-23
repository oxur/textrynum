//! ECL Workflows service entry point.
//!
//! Note: Full Restate HTTP server integration deferred pending SDK 0.7 API verification.
//! This main creates the workflow services and demonstrates the setup pattern.

use ecl_core::llm::{ClaudeProvider, MockLlmProvider, RetryWrapper};
use std::sync::Arc;

mod critique_loop;
mod simple;

use critique_loop::CritiqueLoopWorkflow;
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

    // Determine which workflow to run
    let workflow_type = std::env::var("WORKFLOW_TYPE").unwrap_or_else(|_| "simple".to_string());

    match workflow_type.as_str() {
        "simple" => {
            tracing::info!("Running simple 2-step workflow");
            let service = SimpleWorkflowService::new(llm);
            let input = simple::SimpleWorkflowInput::new("Benefits of Rust programming");

            match service.run_simple(input).await {
                Ok(output) => {
                    tracing::info!("Simple workflow completed successfully");
                    tracing::info!("Generated: {}", output.generated_text);
                    tracing::info!("Critique: {}", output.critique);
                }
                Err(e) => {
                    tracing::error!("Simple workflow failed: {}", e);
                    return Err(e.into());
                }
            }
        }
        "critique_loop" => {
            tracing::info!("Running critique-revise loop workflow");
            let workflow = CritiqueLoopWorkflow::new(llm);
            let input = critique_loop::CritiqueLoopInput::new("Benefits of Rust programming")
                .with_max_revisions(2);

            match workflow.run(input).await {
                Ok(output) => {
                    tracing::info!("Critique loop workflow completed successfully");
                    tracing::info!("Revisions: {}", output.revision_count);
                    tracing::info!("Final text: {}", output.final_text);
                    tracing::info!("Critiques: {}", output.critiques.len());
                }
                Err(e) => {
                    tracing::error!("Critique loop workflow failed: {}", e);
                    return Err(e.into());
                }
            }
        }
        other => {
            tracing::error!("Unknown workflow type: {}", other);
            return Err(anyhow::anyhow!(
                "Unknown WORKFLOW_TYPE: {}. Use 'simple' or 'critique_loop'",
                other
            ));
        }
    }

    tracing::info!("ECL Workflows service completed");
    Ok(())
}
