# Stage 1.6: Phase 1 Integration Test Suite

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/0001-phase-1-implementation.md, lines 3004-3516. Original bytes are recoverable from import commit 2d006a7. -->

## Stage 1.6: Phase 1 Integration Test Suite

**Goal:** Comprehensive tests proving Phase 1 deliverables work correctly.

**Dependencies:** Stages 1.1-1.5 (all prior stages)

### Detailed Implementation Steps

#### Step 1: Create Test Harness

**`tests/common/mod.rs`**

```rust
//! Common test utilities and harness.

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use ecl_core::llm::MockLlmProvider;

/// Test harness for integration tests.
pub struct TestHarness {
    pub restate_url: String,
    pub llm: Arc<MockLlmProvider>,
}

impl TestHarness {
    /// Creates a new test harness.
    pub fn new() -> Self {
        let restate_url = std::env::var("RESTATE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());

        let llm = Arc::new(MockLlmProvider::with_response("Test response"));

        Self { restate_url, llm }
    }

    /// Waits for Restate to be ready.
    pub async fn wait_for_restate(&self) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let health_url = format!("{}/restate/health", self.restate_url);

        for _ in 0..30 {
            if let Ok(response) = client.get(&health_url).send().await {
                if response.status().is_success() {
                    return Ok(());
                }
            }
            sleep(Duration::from_secs(1)).await;
        }

        anyhow::bail!("Restate did not become healthy in time")
    }

    /// Invokes a workflow and waits for completion.
    pub async fn invoke_workflow<I, O>(
        &self,
        service: &str,
        method: &str,
        input: I,
    ) -> anyhow::Result<O>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let client = reqwest::Client::new();
        let url = format!("{}/{}/{}", self.restate_url, service, method);

        let response = client
            .post(&url)
            .json(&input)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Workflow invocation failed: {}", error_text);
        }

        Ok(response.json().await?)
    }
}

/// Helper to parse test data from files.
pub fn load_test_data(filename: &str) -> String {
    std::fs::read_to_string(format!("tests/data/{}", filename))
        .expect("Failed to load test data")
}
```

#### Step 2: Unit Tests

**`tests/unit/mod.rs`**

```rust
//! Unit tests for ECL core types.

mod types;
mod llm_mock;
mod error_handling;
```

**`tests/unit/types.rs`**

```rust
use ecl_core::{WorkflowId, StepId, StepResult, WorkflowState};
use proptest::prelude::*;

#[test]
fn test_workflow_id_uniqueness() {
    let id1 = WorkflowId::new();
    let id2 = WorkflowId::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_step_result_map() {
    let result = StepResult::Success(10);
    let mapped = result.map(|x| x * 2);
    assert_eq!(mapped.unwrap(), 20);
}

#[test]
fn test_workflow_state_terminal() {
    assert!(WorkflowState::Completed.is_terminal());
    assert!(WorkflowState::Failed.is_terminal());
    assert!(!WorkflowState::Running.is_terminal());
}

// Property-based tests
proptest! {
    #[test]
    fn test_workflow_id_serialization_roundtrip(uuid in any::<u128>()) {
        use uuid::Uuid;
        let uuid = Uuid::from_u128(uuid);
        let id = WorkflowId::from_uuid(uuid);
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: WorkflowId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_step_id_roundtrip(s in "\\PC+") {
        let id = StepId::new(s.clone());
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: StepId = serde_json::from_str(&json).unwrap();
        assert_eq!(id.as_str(), deserialized.as_str());
    }
}
```

**`tests/unit/llm_mock.rs`**

```rust
use ecl_core::llm::{MockLlmProvider, Message, CompletionRequest, LlmProvider};

#[tokio::test]
async fn test_mock_provider_responses() {
    let provider = MockLlmProvider::new(vec![
        "First".to_string(),
        "Second".to_string(),
    ]);

    let request = CompletionRequest::new(vec![Message::user("Test")]);

    let response1 = provider.complete(request.clone()).await.unwrap();
    assert_eq!(response1.content, "First");

    let response2 = provider.complete(request.clone()).await.unwrap();
    assert_eq!(response2.content, "Second");

    // Cycles
    let response3 = provider.complete(request).await.unwrap();
    assert_eq!(response3.content, "First");
}
```

**`tests/unit/error_handling.rs`**

```rust
use ecl_core::Error;

#[test]
fn test_error_retryability() {
    assert!(Error::llm("test").is_retryable());
    assert!(Error::Timeout { seconds: 10 }.is_retryable());
    assert!(!Error::validation("test").is_retryable());
    assert!(!Error::MaxRevisionsExceeded { attempts: 3 }.is_retryable());
}

#[test]
fn test_error_display() {
    let err = Error::MaxRevisionsExceeded { attempts: 5 };
    assert_eq!(err.to_string(), "Maximum revisions exceeded: 5 attempts");
}

#[test]
fn test_validation_error_with_field() {
    let err = Error::validation_field("email", "invalid format");
    match err {
        Error::Validation { field, message } => {
            assert_eq!(field, Some("email".to_string()));
            assert_eq!(message, "invalid format");
        }
        _ => panic!("Expected Validation error"),
    }
}
```

#### Step 3: Integration Tests

**`tests/integration/mod.rs`**

```rust
mod simple_workflow;
mod revision_loop;
mod llm_retry;
```

**`tests/integration/simple_workflow.rs`**

```rust
use ecl_workflows::{SimpleWorkflowInput, SimpleWorkflowOutput};

#[tokio::test]
#[ignore] // Requires Restate server
async fn test_simple_workflow_completes() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    let input = SimpleWorkflowInput::new("Rust programming");

    let output: SimpleWorkflowOutput = harness
        .invoke_workflow("SimpleWorkflowService", "run_simple", input)
        .await
        .unwrap();

    assert!(!output.generated_text.is_empty());
    assert!(!output.critique.is_empty());
}
```

**`tests/integration/revision_loop.rs`**

```rust
use ecl_workflows::{CritiqueLoopInput, CritiqueLoopOutput};
use ecl_core::WorkflowId;

#[tokio::test]
#[ignore] // Requires Restate server
async fn test_revision_loop_terminates() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    let input = CritiqueLoopInput {
        workflow_id: WorkflowId::new(),
        topic: "Test topic".to_string(),
    };

    let output: CritiqueLoopOutput = harness
        .invoke_workflow("CritiqueLoopWorkflow", "run", input)
        .await
        .unwrap();

    assert!(!output.final_text.is_empty());
    assert!(output.revision_count <= 3);
}

#[tokio::test]
#[ignore]
async fn test_max_revisions_exceeded() {
    // Test that workflow fails gracefully after max revisions
    // Use mock that always returns "revise" decision
    todo!("Implement test with mock LLM")
}
```

**`tests/integration/llm_retry.rs`**

```rust
use ecl_core::llm::{RetryWrapper, MockLlmProvider, LlmProvider, Message, CompletionRequest};
use std::sync::Arc;

#[tokio::test]
async fn test_retry_on_transient_failure() {
    // Mock that fails twice then succeeds
    // Note: This requires extending MockLlmProvider to support failure injection
    todo!("Implement retry test with failure injection")
}
```

#### Step 4: Chaos Tests

**`tests/chaos/recovery.rs`**

```rust
//! Tests for workflow recovery after process restart.

use std::process::{Command, Child};
use std::time::Duration;
use tokio::time::sleep;

/// Helper to start ECL workflows service.
fn start_service() -> Child {
    Command::new("cargo")
        .args(&["run", "--bin", "ecl-workflows"])
        .spawn()
        .expect("Failed to start service")
}

#[tokio::test]
#[ignore] // Manual test, requires real Restate
async fn test_workflow_survives_restart() {
    let harness = tests::common::TestHarness::new();
    harness.wait_for_restate().await.unwrap();

    // Start service
    let mut service = start_service();
    sleep(Duration::from_secs(2)).await;

    // Start workflow
    let input = ecl_workflows::SimpleWorkflowInput::new("Test");
    // Don't await - let it run in background
    let _handle = tokio::spawn(async move {
        harness
            .invoke_workflow("SimpleWorkflowService", "run_simple", input)
            .await
    });

    // Wait for first step to complete
    sleep(Duration::from_secs(5)).await;

    // Kill service
    service.kill().unwrap();
    service.wait().unwrap();

    // Restart service
    let mut service = start_service();
    sleep(Duration::from_secs(2)).await;

    // Workflow should resume and complete
    // (Check via Restate API or workflow output)

    service.kill().unwrap();
}
```

#### Step 5: CI Configuration

**`.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Suite
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run unit tests
        run: cargo test --workspace --lib

      - name: Run doc tests
        run: cargo test --workspace --doc

  lint:
    name: Linting
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - name: Run rustfmt
        run: cargo fmt --all -- --check

      - name: Run clippy
        run: cargo clippy --workspace --all-features --all-targets -- -D warnings

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: true
```

#### Step 6: Coverage Report Generation

**`justfile`** (add):

```just
# Generate coverage report and check threshold
coverage-check:
    cargo llvm-cov --workspace --all-features --html
    @echo "Checking coverage threshold..."
    @COVERAGE=$(cargo llvm-cov --workspace --all-features --summary-only | grep -oP '\d+\.\d+(?=%)' | head -1); \
    if (( $(echo "$COVERAGE < 95" | bc -l) )); then \
        echo "Coverage $COVERAGE% is below 95% threshold"; \
        exit 1; \
    else \
        echo "Coverage $COVERAGE% meets 95% threshold ✓"; \
    fi
```

### Testing Strategy

**Test organization:**

```
tests/
├── common/          # Shared test utilities
│   └── mod.rs       # TestHarness, helpers
├── unit/            # Fast unit tests (no I/O)
│   ├── mod.rs
│   ├── types.rs
│   ├── llm_mock.rs
│   └── error_handling.rs
├── integration/     # Integration tests (requires services)
│   ├── mod.rs
│   ├── simple_workflow.rs
│   ├── revision_loop.rs
│   └── llm_retry.rs
└── chaos/           # Chaos/recovery tests
    └── recovery.rs
```

**Running tests:**

```bash
# Unit tests only (fast, no dependencies)
cargo test --workspace --lib

# All tests except ignored
cargo test --workspace

# Integration tests (requires Restate)
docker compose up -d
cargo test --workspace -- --ignored
docker compose down

# Coverage
just coverage-check
```

### Potential Blockers

1. **Integration tests are flaky**
   - **Mitigation**: Proper wait conditions
   - **Mitigation**: Retry logic in tests
   - **Mitigation**: Clear test isolation

2. **Coverage reporting issues**
   - **Mitigation**: Use cargo-llvm-cov as specified
   - **Mitigation**: Exclude generated code if needed

3. **CI timeout issues**
   - **Mitigation**: Run integration tests separately
   - **Mitigation**: Cache dependencies properly

### Acceptance Criteria

- [ ] All unit tests pass
- [ ] Integration tests pass locally with Restate server
- [ ] CI pipeline passes (unit tests + linting)
- [ ] Overall test coverage ≥95%
- [ ] No ignored tests except integration/chaos tests
- [ ] Coverage report generates successfully
- [ ] All error paths are tested
- [ ] Property-based tests exist for core types
- [ ] Chaos tests demonstrate recovery (manual)
- [ ] Test documentation is clear

### Estimated Effort

**Time:** 6-8 hours

**Breakdown:**
- Test harness: 1.5 hours
- Unit tests: 2 hours
- Integration tests: 2 hours
- Chaos tests: 1 hour
- CI configuration: 1 hour
- Coverage verification: 0.5 hours

---

