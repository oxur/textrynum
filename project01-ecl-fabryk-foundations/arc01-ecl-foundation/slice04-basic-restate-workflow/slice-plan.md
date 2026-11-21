# Stage 1.4: Basic Restate Workflow

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 2217-2536. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.4: Basic Restate Workflow

**Goal:** Create minimal 2-step workflow demonstrating Restate integration.

**Dependencies:** Stage 1.3 (LLM abstraction layer)

### Detailed Implementation Steps

#### Step 1: Create Workflow Input/Output Types

**`crates/ecl-workflows/src/simple.rs`**

```rust
//! Simple 2-step workflow for Phase 1 validation.

use serde::{Deserialize, Serialize};
use ecl_core::{WorkflowId, Result};

/// Input for the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowInput {
    /// Unique ID for this workflow instance
    pub workflow_id: WorkflowId,

    /// Topic to generate content about
    pub topic: String,
}

impl SimpleWorkflowInput {
    /// Creates a new workflow input.
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            workflow_id: WorkflowId::new(),
            topic: topic.into(),
        }
    }
}

/// Output from the simple workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleWorkflowOutput {
    /// Workflow ID that produced this output
    pub workflow_id: WorkflowId,

    /// Generated text from step 1
    pub generated_text: String,

    /// Critique of generated text from step 2
    pub critique: String,
}
```

#### Step 2: Implement Restate Service

**`crates/ecl-workflows/src/simple.rs`** (continued)

```rust
use restate_sdk::prelude::*;
use std::sync::Arc;
use ecl_core::llm::{LlmProvider, Message, CompletionRequest};

/// Simple workflow service demonstrating Restate integration.
pub struct SimpleWorkflowService {
    llm: Arc<dyn LlmProvider>,
}

impl SimpleWorkflowService {
    /// Creates a new simple workflow service.
    pub fn new(llm: Arc<dyn LlmProvider>) -> Self {
        Self { llm }
    }

    /// Runs the simple 2-step workflow.
    pub async fn run_simple(
        &self,
        ctx: Context<'_>,
        input: SimpleWorkflowInput,
    ) -> Result<SimpleWorkflowOutput> {
        // Step 1: Generate content
        let generated_text = ctx
            .run("generate", || async {
                self.generate_step(&input.topic).await
            })
            .await?;

        // Step 2: Critique the generated content
        let critique = ctx
            .run("critique", || async {
                self.critique_step(&generated_text).await
            })
            .await?;

        Ok(SimpleWorkflowOutput {
            workflow_id: input.workflow_id,
            generated_text,
            critique,
        })
    }

    /// Step 1: Generate content on a topic.
    async fn generate_step(&self, topic: &str) -> Result<String> {
        tracing::info!(topic = %topic, "Generating content");

        let request = CompletionRequest::new(vec![
            Message::user(format!("Write a short paragraph about: {}", topic))
        ])
        .with_system_prompt("You are a helpful content generator. Write clear, concise paragraphs.")
        .with_max_tokens(500);

        let response = self.llm.complete(request).await?;

        tracing::info!(
            tokens = response.tokens_used.total(),
            "Content generated"
        );

        Ok(response.content)
    }

    /// Step 2: Critique generated content.
    async fn critique_step(&self, content: &str) -> Result<String> {
        tracing::info!("Critiquing generated content");

        let request = CompletionRequest::new(vec![
            Message::user(format!(
                "Please provide constructive criticism of the following text:\n\n{}",
                content
            ))
        ])
        .with_system_prompt("You are a helpful writing critic. Provide specific, actionable feedback.")
        .with_max_tokens(300);

        let response = self.llm.complete(request).await?;

        tracing::info!(
            tokens = response.tokens_used.total(),
            "Critique completed"
        );

        Ok(response.content)
    }
}
```

#### Step 3: Register Service with Restate

**`crates/ecl-workflows/src/main.rs`**

```rust
//! ECL Workflows service entry point.

use std::sync::Arc;
use ecl_core::llm::{ClaudeProvider, RetryWrapper};
use restate_sdk::prelude::*;

mod simple;
use simple::SimpleWorkflowService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,ecl=debug".into())
        )
        .init();

    tracing::info!("Starting ECL Workflows service");

    // Load configuration
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");
    let model = std::env::var("ANTHROPIC_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());

    // Create LLM provider with retry wrapper
    let claude = ClaudeProvider::new(api_key, model);
    let llm = Arc::new(RetryWrapper::new(Arc::new(claude)));

    // Create workflow service
    let service = SimpleWorkflowService::new(llm);

    // Build Restate endpoint
    let endpoint = Endpoint::builder()
        .bind(service)
        .build();

    // Start HTTP server
    let bind_address = std::env::var("BIND_ADDRESS")
        .unwrap_or_else(|_| "0.0.0.0:9080".to_string());

    tracing::info!(address = %bind_address, "Starting Restate endpoint");

    HttpServer::new(endpoint)
        .listen_and_serve(bind_address.parse()?, tokio::signal::ctrl_c())
        .await?;

    tracing::info!("ECL Workflows service stopped");
    Ok(())
}
```

#### Step 4: Create Invocation Script

**`scripts/run-simple-workflow.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail

RESTATE_URL="${RESTATE_URL:-http://localhost:8080}"
WORKFLOW_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')
TOPIC="${1:-Benefits of Rust programming}"

echo "=== ECL Simple Workflow Test ==="
echo "Workflow ID: $WORKFLOW_ID"
echo "Topic: $TOPIC"
echo

# Invoke workflow
curl -X POST \
  "$RESTATE_URL/SimpleWorkflowService/run_simple" \
  -H "Content-Type: application/json" \
  -d "{
    \"workflow_id\": \"$WORKFLOW_ID\",
    \"topic\": \"$TOPIC\"
  }" | jq '.'
```

#### Step 5: Update Docker Compose

Add to `docker-compose.yml`:

```yaml
  ecl-workflows:
    build:
      context: .
      dockerfile: Dockerfile
    depends_on:
      - restate
    environment:
      - ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY}
      - RUST_LOG=info,ecl=debug
    ports:
      - "9080:9080"
```

**`Dockerfile`**

```dockerfile
FROM rust:1.75-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin ecl-workflows

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ecl-workflows /usr/local/bin/
EXPOSE 9080
CMD ["ecl-workflows"]
```

### Testing Strategy

**Test coverage goal:** ≥95%

**Tests needed:**

1. **Workflow execution**:
   - Happy path (both steps complete)
   - Workflow survives restart mid-execution
   - Tracing logs capture execution

2. **Input/output validation**:
   - Valid inputs accepted
   - Invalid inputs rejected
   - Output serialization works

3. **Restate integration**:
   - Service registration works
   - Durable execution survives restarts
   - State persists correctly

### Potential Blockers

1. **Restate SDK API changes**
   - **Mitigation**: Carefully review Restate SDK 0.7 documentation
   - **Mitigation**: Test service registration early

2. **Docker networking issues**
   - **Mitigation**: Use docker-compose networking
   - **Mitigation**: Document port mappings clearly

3. **Serialization with Restate**
   - **Mitigation**: Ensure all types are Serialize + Deserialize
   - **Mitigation**: Test serialization early

### Acceptance Criteria

- [ ] Workflow completes successfully end-to-end
- [ ] Killing process mid-workflow and restarting resumes from last step
- [ ] Workflow output contains both generated text and critique
- [ ] Tracing logs show step execution with structured fields
- [ ] Docker Compose brings up full stack
- [ ] Service registers with Restate successfully
- [ ] Test coverage ≥90% (integration tests may be lower)

### Estimated Effort

**Time:** 5-6 hours

**Breakdown:**
- Restate service setup: 2 hours
- Workflow implementation: 2 hours
- Docker setup: 1 hour
- Testing and debugging: 2 hours

---

