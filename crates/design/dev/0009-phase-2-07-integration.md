# Stage 2.7: Toy Workflow Integration

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2 - Step Abstraction Framework
**Stage:** 2.7 - Integration
**Status:** Ready for Implementation

---

## Goal

Wire everything together into a working Critique-Revise workflow that demonstrates the full capabilities of the Step Abstraction Framework built in Stages 2.1-2.6.

**Deliverables:**
- Complete `CritiqueReviseWorkflow` definition using the orchestrator
- Full Restate integration with workflow orchestrator
- CLI commands for running and monitoring workflows
- Comprehensive integration tests covering all execution paths
- End-to-end validation of the complete system

---

## Dependencies

**Required Stages (must be complete):**
- ✅ Stage 2.1: Step Trait Definition
- ✅ Stage 2.2: Step Executor
- ✅ Stage 2.3: Toy Workflow Step Implementations
- ✅ Stage 2.4: Workflow Orchestrator
- ✅ Stage 2.5: SQLx Persistence Layer
- ✅ Stage 2.6: Observability Infrastructure

**External Dependencies:**
- Restate runtime (local development)
- SQLite/PostgreSQL for persistence
- LLM provider (configured in environment)

---

## Detailed Implementation Steps

### Step 1: Create CritiqueReviseWorkflow Definition

**Before writing code:**
1. Review `assets/ai/ai-rust/guides/11-anti-patterns.md`
2. Review orchestrator API from Stage 2.4
3. Review step implementations from Stage 2.3

**`crates/ecl-workflows/src/definitions/critique_revise.rs`**

```rust
//! Critique-Revise workflow definition.
//!
//! This is the toy workflow that demonstrates the ECL framework capabilities.
//! It implements a simple feedback loop:
//!
//! 1. **Generate**: Create initial content on a topic
//! 2. **Critique**: Analyze the content and decide if it needs revision
//! 3. **Revise**: If needed, improve the content based on critique feedback
//!
//! The workflow continues revising until either:
//! - The critique step approves the content (passes validation)
//! - Maximum revision iterations are reached (default: 3)

use crate::orchestrator::{WorkflowDefinition, StepDefinition, Transition};

/// Creates the Critique-Revise workflow definition.
///
/// # Workflow Structure
///
/// ```text
/// Generate → Critique ⇄ Revise
///            (revision loop, max 3 iterations)
/// ```
///
/// # Example
///
/// ```rust
/// use ecl_workflows::definitions::critique_revise_workflow;
/// use ecl_workflows::orchestrator::WorkflowOrchestrator;
///
/// let workflow = critique_revise_workflow();
/// let orchestrator = WorkflowOrchestrator::new(/* ... */);
/// orchestrator.execute_workflow(&workflow, input).await?;
/// ```
pub fn critique_revise_workflow() -> WorkflowDefinition {
    WorkflowDefinition {
        id: "critique-revise".into(),
        name: "Critique and Revise Loop".into(),
        description: Some(
            "A feedback loop workflow that generates content, critiques it, \
             and revises until quality standards are met or max iterations reached."
                .into(),
        ),
        steps: vec![
            StepDefinition {
                step_id: "generate".into(),
                depends_on: vec![],
                revision_source: None,
                metadata: None,
            },
            StepDefinition {
                step_id: "critique".into(),
                depends_on: vec!["generate".into()],
                revision_source: None,
                metadata: None,
            },
            StepDefinition {
                step_id: "revise".into(),
                depends_on: vec!["critique".into()],
                revision_source: Some("critique".into()),
                metadata: None,
            },
        ],
        transitions: vec![
            // Sequential flow from generate to critique
            Transition::Sequential {
                from: "generate".into(),
                to: "critique".into(),
            },
            // Revision loop between critique and revise
            Transition::RevisionLoop {
                reviser: "revise".into(),
                validator: "critique".into(),
                max_iterations: 3,
            },
        ],
        metadata: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_definition() {
        let workflow = critique_revise_workflow();

        assert_eq!(workflow.id.as_str(), "critique-revise");
        assert_eq!(workflow.name, "Critique and Revise Loop");
        assert_eq!(workflow.steps.len(), 3);
        assert_eq!(workflow.transitions.len(), 2);
    }

    #[test]
    fn test_step_dependencies() {
        let workflow = critique_revise_workflow();

        let generate = workflow.steps.iter().find(|s| s.step_id.as_str() == "generate").unwrap();
        assert!(generate.depends_on.is_empty());

        let critique = workflow.steps.iter().find(|s| s.step_id.as_str() == "critique").unwrap();
        assert_eq!(critique.depends_on, vec!["generate".to_string()]);

        let revise = workflow.steps.iter().find(|s| s.step_id.as_str() == "revise").unwrap();
        assert_eq!(revise.depends_on, vec!["critique".to_string()]);
    }

    #[test]
    fn test_revision_source() {
        let workflow = critique_revise_workflow();

        let revise = workflow.steps.iter().find(|s| s.step_id.as_str() == "revise").unwrap();
        assert_eq!(revise.revision_source.as_deref(), Some("critique"));
    }

    #[test]
    fn test_transitions() {
        let workflow = critique_revise_workflow();

        // Check sequential transition
        let sequential = workflow.transitions.iter().find(|t| {
            matches!(t, Transition::Sequential { from, to } if from.as_str() == "generate" && to.as_str() == "critique")
        });
        assert!(sequential.is_some());

        // Check revision loop
        let revision_loop = workflow.transitions.iter().find(|t| {
            matches!(t, Transition::RevisionLoop { reviser, validator, max_iterations }
                if reviser.as_str() == "revise" && validator.as_str() == "critique" && *max_iterations == 3)
        });
        assert!(revision_loop.is_some());
    }
}
```

### Step 2: Create Workflow Module Definition

**`crates/ecl-workflows/src/definitions/mod.rs`**

```rust
//! Workflow definitions.
//!
//! This module contains pre-built workflow definitions that can be used
//! out of the box or as templates for custom workflows.

mod critique_revise;

pub use critique_revise::critique_revise_workflow;
```

**Update `crates/ecl-workflows/src/lib.rs`:**

```rust
pub mod definitions;
pub mod orchestrator;

// Re-export commonly used items
pub use definitions::critique_revise_workflow;
pub use orchestrator::{
    WorkflowOrchestrator, WorkflowDefinition, StepDefinition, Transition,
};
```

### Step 3: Integrate with Restate Workflow

**`crates/ecl-workflows/src/restate/workflow_service.rs`**

```rust
//! Restate service implementation for workflow orchestration.
//!
//! This service provides durable workflow execution using Restate's
//! Virtual Objects pattern. Each workflow instance is a virtual object
//! with its own state.

use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn, error};

use crate::orchestrator::{WorkflowOrchestrator, WorkflowDefinition};
use ecl_core::{Result, Error, WorkflowId, StepId};

/// Input for starting a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowInput {
    /// Workflow definition to execute
    pub workflow_definition: WorkflowDefinition,

    /// Initial input data (JSON-encoded)
    pub input: serde_json::Value,
}

/// Status of a workflow execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkflowStatus {
    /// Workflow is currently executing
    Running {
        current_step: StepId,
        attempt: u32,
    },

    /// Workflow completed successfully
    Completed {
        output: serde_json::Value,
        total_steps: usize,
    },

    /// Workflow failed
    Failed {
        error: String,
        failed_step: StepId,
    },

    /// Workflow was paused
    Paused {
        current_step: StepId,
    },
}

/// Restate workflow service.
///
/// This is a Virtual Object service where each workflow instance
/// has its own state stored in Restate's K/V store.
#[restate_sdk::service]
trait WorkflowService {
    /// Start a new workflow execution.
    ///
    /// This is the entry point for workflow execution. It:
    /// 1. Creates a new workflow instance
    /// 2. Stores initial state in Restate K/V
    /// 3. Begins orchestrated execution
    /// 4. Uses Durable Promises for revision loops
    async fn start(
        &self,
        ctx: Context<'_>,
        input: StartWorkflowInput,
    ) -> Result<WorkflowId>;

    /// Get the current status of a workflow.
    async fn get_status(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<WorkflowStatus>;

    /// Pause a running workflow.
    async fn pause(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<()>;

    /// Resume a paused workflow.
    async fn resume(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<()>;
}

/// Implementation of the workflow service.
pub struct WorkflowServiceImpl {
    orchestrator: Arc<WorkflowOrchestrator>,
}

impl WorkflowServiceImpl {
    /// Creates a new workflow service.
    pub fn new(orchestrator: Arc<WorkflowOrchestrator>) -> Self {
        Self { orchestrator }
    }
}

#[restate_sdk::service]
impl WorkflowService for WorkflowServiceImpl {
    async fn start(
        &self,
        ctx: Context<'_>,
        input: StartWorkflowInput,
    ) -> Result<WorkflowId> {
        let workflow_id = WorkflowId::new();

        info!(
            workflow_id = %workflow_id,
            workflow_name = %input.workflow_definition.name,
            "Starting workflow execution"
        );

        // Store workflow definition in Restate K/V
        ctx.set("workflow_definition", &input.workflow_definition).await;
        ctx.set("input", &input.input).await;
        ctx.set("status", &WorkflowStatus::Running {
            current_step: input.workflow_definition.steps[0].step_id.clone(),
            attempt: 1,
        }).await;

        // Execute workflow using orchestrator
        // Each step execution is wrapped in ctx.run() for durability
        match self.execute_workflow_durable(&ctx, workflow_id, &input.workflow_definition, input.input).await {
            Ok(output) => {
                ctx.set("status", &WorkflowStatus::Completed {
                    output,
                    total_steps: input.workflow_definition.steps.len(),
                }).await;

                info!(workflow_id = %workflow_id, "Workflow completed successfully");
                Ok(workflow_id)
            }
            Err(e) => {
                let failed_step = self.get_current_step(&ctx).await;
                ctx.set("status", &WorkflowStatus::Failed {
                    error: e.to_string(),
                    failed_step,
                }).await;

                error!(workflow_id = %workflow_id, error = %e, "Workflow failed");
                Err(e)
            }
        }
    }

    async fn get_status(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<WorkflowStatus> {
        ctx.get::<WorkflowStatus>("status")
            .await
            .ok_or_else(|| Error::workflow_not_found(workflow_id))
    }

    async fn pause(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<()> {
        let current_step = self.get_current_step(&ctx).await;
        ctx.set("status", &WorkflowStatus::Paused { current_step }).await;

        info!(workflow_id = %workflow_id, "Workflow paused");
        Ok(())
    }

    async fn resume(
        &self,
        ctx: Context<'_>,
        workflow_id: WorkflowId,
    ) -> Result<()> {
        let status = self.get_status(ctx, workflow_id).await?;

        match status {
            WorkflowStatus::Paused { current_step } => {
                ctx.set("status", &WorkflowStatus::Running {
                    current_step: current_step.clone(),
                    attempt: 1,
                }).await;

                info!(workflow_id = %workflow_id, "Workflow resumed");
                Ok(())
            }
            _ => Err(Error::invalid_state("Workflow is not paused")),
        }
    }
}

impl WorkflowServiceImpl {
    /// Execute workflow with durable guarantees.
    ///
    /// Each step execution is wrapped in `ctx.run()` which provides:
    /// - Exactly-once execution semantics
    /// - Automatic retry on transient failures
    /// - State persistence across restarts
    async fn execute_workflow_durable(
        &self,
        ctx: &Context<'_>,
        workflow_id: WorkflowId,
        definition: &WorkflowDefinition,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Delegate to orchestrator, wrapping each step in ctx.run()
        // This provides durable execution guarantees

        // For revision loops, use Durable Promises to track iteration state
        // across potential restarts

        self.orchestrator
            .execute_workflow_with_restate(ctx, workflow_id, definition, input)
            .await
    }

    /// Get the current step from context.
    async fn get_current_step(&self, ctx: &Context<'_>) -> StepId {
        ctx.get::<WorkflowStatus>("status")
            .await
            .and_then(|status| match status {
                WorkflowStatus::Running { current_step, .. } => Some(current_step),
                WorkflowStatus::Paused { current_step } => Some(current_step),
                WorkflowStatus::Failed { failed_step, .. } => Some(failed_step),
                _ => None,
            })
            .unwrap_or_else(|| StepId::new("unknown"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_status_serialization() {
        let status = WorkflowStatus::Running {
            current_step: StepId::new("test"),
            attempt: 1,
        };

        let json = serde_json::to_string(&status).unwrap();
        let deserialized: WorkflowStatus = serde_json::from_str(&json).unwrap();

        match deserialized {
            WorkflowStatus::Running { current_step, attempt } => {
                assert_eq!(current_step.as_str(), "test");
                assert_eq!(attempt, 1);
            }
            _ => panic!("Wrong variant"),
        }
    }
}
```

**`crates/ecl-workflows/src/restate/mod.rs`**

```rust
//! Restate integration for durable workflow execution.

mod workflow_service;

pub use workflow_service::{
    WorkflowService, WorkflowServiceImpl, StartWorkflowInput, WorkflowStatus,
};
```

**Update `crates/ecl-workflows/src/lib.rs`:**

```rust
pub mod definitions;
pub mod orchestrator;
pub mod restate;

pub use definitions::critique_revise_workflow;
pub use orchestrator::{
    WorkflowOrchestrator, WorkflowDefinition, StepDefinition, Transition,
};
pub use restate::{WorkflowService, WorkflowServiceImpl, StartWorkflowInput, WorkflowStatus};
```

### Step 4: Add CLI Commands

#### Step 4.1: CLI Structure

**`crates/ecl-cli/src/main.rs`**

```rust
//! ECL CLI - Command-line interface for workflow execution.

use clap::{Parser, Subcommand};
use ecl_core::Result;
use tracing_subscriber::EnvFilter;

mod commands;

#[derive(Debug, Parser)]
#[command(name = "ecl")]
#[command(about = "ECL - Emergent Cognitive Loops CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Log format (json, pretty, compact)
    #[arg(long, global = true, default_value = "pretty")]
    log_format: String,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a workflow
    Run(commands::run::RunCommand),

    /// Check workflow status
    Status(commands::status::StatusCommand),

    /// List available workflows
    List(commands::list::ListCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    init_logging(cli.verbose, &cli.log_format)?;

    // Execute command
    match cli.command {
        Commands::Run(cmd) => cmd.execute().await,
        Commands::Status(cmd) => cmd.execute().await,
        Commands::List(cmd) => cmd.execute().await,
    }
}

fn init_logging(verbose: bool, format: &str) -> Result<()> {
    let filter = if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false);

    match format {
        "json" => subscriber.json().init(),
        "compact" => subscriber.compact().init(),
        "pretty" | _ => subscriber.pretty().init(),
    }

    Ok(())
}
```

#### Step 4.2: Run Command

**`crates/ecl-cli/src/commands/run.rs`**

```rust
//! CLI command to run a workflow.

use clap::Parser;
use ecl_core::{Result, Error};
use ecl_workflows::{
    critique_revise_workflow,
    restate::{WorkflowServiceImpl, StartWorkflowInput},
};
use serde_json::json;
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Debug, Parser)]
pub struct RunCommand {
    /// Workflow name to run
    #[arg(value_name = "WORKFLOW")]
    workflow: String,

    /// Topic for content generation (for critique-revise workflow)
    #[arg(long)]
    topic: Option<String>,

    /// Input file (JSON)
    #[arg(long, short = 'i')]
    input_file: Option<PathBuf>,

    /// Restate endpoint
    #[arg(long, env = "RESTATE_ENDPOINT", default_value = "http://localhost:8080")]
    restate_endpoint: String,

    /// Wait for completion
    #[arg(long, short = 'w')]
    wait: bool,
}

impl RunCommand {
    pub async fn execute(self) -> Result<()> {
        info!("Running workflow: {}", self.workflow);

        // Get workflow definition
        let workflow_def = match self.workflow.as_str() {
            "critique-revise" => critique_revise_workflow(),
            _ => {
                error!("Unknown workflow: {}", self.workflow);
                return Err(Error::invalid_input(format!(
                    "Unknown workflow: {}. Available workflows: critique-revise",
                    self.workflow
                )));
            }
        };

        // Prepare input
        let input = if let Some(input_file) = &self.input_file {
            let content = tokio::fs::read_to_string(input_file).await?;
            serde_json::from_str(&content)?
        } else if let Some(topic) = &self.topic {
            json!({ "topic": topic })
        } else {
            error!("Either --topic or --input-file must be provided");
            return Err(Error::invalid_input(
                "Either --topic or --input-file must be provided"
            ));
        };

        // Connect to Restate and start workflow
        let client = restate_sdk::http_client::RestateHttpClient::new(&self.restate_endpoint)?;

        let start_input = StartWorkflowInput {
            workflow_definition: workflow_def,
            input,
        };

        let workflow_id = client
            .call_service::<WorkflowServiceImpl>("workflow", "start", start_input)
            .await?;

        println!("Workflow started: {}", workflow_id);
        println!("Check status with: ecl status {}", workflow_id);

        if self.wait {
            println!("Waiting for completion...");
            self.wait_for_completion(&client, workflow_id).await?;
        }

        Ok(())
    }

    async fn wait_for_completion(
        &self,
        client: &restate_sdk::http_client::RestateHttpClient,
        workflow_id: ecl_core::WorkflowId,
    ) -> Result<()> {
        use ecl_workflows::restate::WorkflowStatus;
        use tokio::time::{sleep, Duration};

        loop {
            let status = client
                .call_service::<WorkflowServiceImpl>("workflow", "get_status", workflow_id)
                .await?;

            match status {
                WorkflowStatus::Completed { output, total_steps } => {
                    println!("\n✓ Workflow completed successfully!");
                    println!("Total steps: {}", total_steps);
                    println!("\nOutput:");
                    println!("{}", serde_json::to_string_pretty(&output)?);
                    return Ok(());
                }
                WorkflowStatus::Failed { error, failed_step } => {
                    eprintln!("\n✗ Workflow failed at step: {}", failed_step);
                    eprintln!("Error: {}", error);
                    return Err(Error::workflow_failed(error));
                }
                WorkflowStatus::Running { current_step, attempt } => {
                    print!("\rRunning: {} (attempt {})...", current_step, attempt);
                    sleep(Duration::from_secs(2)).await;
                }
                WorkflowStatus::Paused { current_step } => {
                    println!("\n⏸ Workflow paused at step: {}", current_step);
                    return Ok(());
                }
            }
        }
    }
}
```

#### Step 4.3: Status Command

**`crates/ecl-cli/src/commands/status.rs`**

```rust
//! CLI command to check workflow status.

use clap::Parser;
use ecl_core::{Result, WorkflowId};
use ecl_workflows::restate::{WorkflowServiceImpl, WorkflowStatus};
use tracing::info;

#[derive(Debug, Parser)]
pub struct StatusCommand {
    /// Workflow ID to check
    #[arg(value_name = "WORKFLOW_ID")]
    workflow_id: String,

    /// Restate endpoint
    #[arg(long, env = "RESTATE_ENDPOINT", default_value = "http://localhost:8080")]
    restate_endpoint: String,

    /// Output format (text, json)
    #[arg(long, short = 'o', default_value = "text")]
    output: String,
}

impl StatusCommand {
    pub async fn execute(self) -> Result<()> {
        info!("Checking status for workflow: {}", self.workflow_id);

        let workflow_id = WorkflowId::from_string(&self.workflow_id)?;

        // Connect to Restate and get status
        let client = restate_sdk::http_client::RestateHttpClient::new(&self.restate_endpoint)?;

        let status = client
            .call_service::<WorkflowServiceImpl>("workflow", "get_status", workflow_id)
            .await?;

        match self.output.as_str() {
            "json" => {
                println!("{}", serde_json::to_string_pretty(&status)?);
            }
            "text" | _ => {
                self.print_status_text(&status);
            }
        }

        Ok(())
    }

    fn print_status_text(&self, status: &WorkflowStatus) {
        println!("Workflow ID: {}", self.workflow_id);
        println!();

        match status {
            WorkflowStatus::Running { current_step, attempt } => {
                println!("Status: ⏳ Running");
                println!("Current Step: {}", current_step);
                println!("Attempt: {}", attempt);
            }
            WorkflowStatus::Completed { output, total_steps } => {
                println!("Status: ✓ Completed");
                println!("Total Steps: {}", total_steps);
                println!();
                println!("Output:");
                println!("{}", serde_json::to_string_pretty(output).unwrap());
            }
            WorkflowStatus::Failed { error, failed_step } => {
                println!("Status: ✗ Failed");
                println!("Failed Step: {}", failed_step);
                println!();
                println!("Error:");
                println!("{}", error);
            }
            WorkflowStatus::Paused { current_step } => {
                println!("Status: ⏸ Paused");
                println!("Current Step: {}", current_step);
            }
        }
    }
}
```

#### Step 4.4: List Command

**`crates/ecl-cli/src/commands/list.rs`**

```rust
//! CLI command to list available workflows.

use clap::Parser;
use ecl_core::Result;

#[derive(Debug, Parser)]
pub struct ListCommand {
    /// Show detailed information
    #[arg(long, short = 'd')]
    detailed: bool,
}

impl ListCommand {
    pub async fn execute(self) -> Result<()> {
        println!("Available Workflows:");
        println!();

        // For now, we only have one workflow
        // In the future, this would query the workflow registry
        println!("  critique-revise");

        if self.detailed {
            println!("    Description: A feedback loop workflow that generates content,");
            println!("                 critiques it, and revises until quality standards");
            println!("                 are met or max iterations reached.");
            println!("    Steps:");
            println!("      1. generate  - Create initial content");
            println!("      2. critique  - Analyze and validate content");
            println!("      3. revise    - Improve based on feedback");
            println!("    Max Iterations: 3");
            println!();
            println!("    Usage:");
            println!("      ecl run critique-revise --topic \"Benefits of Rust\"");
        }

        Ok(())
    }
}
```

#### Step 4.5: Commands Module

**`crates/ecl-cli/src/commands/mod.rs`**

```rust
//! CLI command implementations.

pub mod list;
pub mod run;
pub mod status;
```

### Step 5: Comprehensive Integration Tests

**`crates/ecl-workflows/tests/integration/mod.rs`**

```rust
//! Integration tests for the complete workflow system.

mod happy_path;
mod revision_path;
mod max_revisions;
mod recovery;

// Common test utilities
pub mod utils;
```

#### Step 5.1: Happy Path Test

**`crates/ecl-workflows/tests/integration/happy_path.rs`**

```rust
//! Integration test: Happy path (generates, critiques, passes).
//!
//! This test verifies that when content is generated and immediately
//! passes critique validation, the workflow completes successfully
//! without requiring any revisions.

use ecl_core::{Result, WorkflowId};
use ecl_workflows::{
    critique_revise_workflow,
    orchestrator::WorkflowOrchestrator,
};
use serde_json::json;

use super::utils::{setup_test_orchestrator, mock_llm_with_responses};

#[tokio::test]
async fn test_critique_revise_happy_path() -> Result<()> {
    // Setup
    let llm = mock_llm_with_responses(vec![
        // Generate step response
        json!({
            "content": "Rust is an excellent systems programming language that provides \
                       memory safety without garbage collection through its ownership system.",
            "word_count": 15,
        }),
        // Critique step response (passes immediately)
        json!({
            "decision": "pass",
            "feedback": "Content is well-structured and accurate.",
            "quality_score": 95,
        }),
    ]);

    let orchestrator = setup_test_orchestrator(llm).await?;
    let workflow = critique_revise_workflow();

    // Execute
    let workflow_id = WorkflowId::new();
    let input = json!({
        "topic": "Benefits of Rust",
        "min_words": 10,
    });

    let result = orchestrator
        .execute_workflow(workflow_id, &workflow, input)
        .await?;

    // Verify
    assert!(result.is_object());

    let output = result.as_object().unwrap();
    assert!(output.contains_key("content"));
    assert!(output.contains_key("word_count"));

    // Verify workflow metadata
    let metadata = orchestrator.get_workflow_metadata(workflow_id).await?;
    assert_eq!(metadata.total_steps, 2); // generate + critique (no revise)
    assert_eq!(metadata.status, "completed");

    Ok(())
}

#[tokio::test]
async fn test_workflow_creates_database_records() -> Result<()> {
    let llm = mock_llm_with_responses(vec![
        json!({"content": "Test content", "word_count": 2}),
        json!({"decision": "pass", "feedback": "Good", "quality_score": 90}),
    ]);

    let orchestrator = setup_test_orchestrator(llm).await?;
    let workflow = critique_revise_workflow();

    let workflow_id = WorkflowId::new();
    let input = json!({"topic": "Test"});

    orchestrator.execute_workflow(workflow_id, &workflow, input).await?;

    // Verify database records were created
    let repo = orchestrator.workflow_repository();

    let workflow_record = repo.get_workflow(workflow_id).await?;
    assert_eq!(workflow_record.status, "completed");

    let step_executions = repo.get_step_executions(workflow_id).await?;
    assert_eq!(step_executions.len(), 2); // generate + critique

    Ok(())
}
```

#### Step 5.2: Revision Path Test

**`crates/ecl-workflows/tests/integration/revision_path.rs`**

```rust
//! Integration test: Revision path (generates, critiques, revises, passes).
//!
//! This test verifies that when content fails initial critique,
//! the workflow correctly enters the revision loop and can
//! successfully improve the content.

use ecl_core::Result;
use ecl_workflows::critique_revise_workflow;
use serde_json::json;

use super::utils::{setup_test_orchestrator, mock_llm_with_responses};

#[tokio::test]
async fn test_single_revision_then_pass() -> Result<()> {
    let llm = mock_llm_with_responses(vec![
        // Generate step
        json!({
            "content": "Rust is good.",
            "word_count": 3,
        }),
        // First critique (fail - too short)
        json!({
            "decision": "revise",
            "feedback": "Content is too brief. Please expand with more details.",
            "quality_score": 40,
        }),
        // Revise step
        json!({
            "content": "Rust is an excellent systems programming language with \
                       memory safety guarantees and zero-cost abstractions.",
            "word_count": 14,
        }),
        // Second critique (pass)
        json!({
            "decision": "pass",
            "feedback": "Much better! Good detail and accuracy.",
            "quality_score": 85,
        }),
    ]);

    let orchestrator = setup_test_orchestrator(llm).await?;
    let workflow = critique_revise_workflow();

    let workflow_id = ecl_core::WorkflowId::new();
    let input = json!({
        "topic": "Benefits of Rust",
        "min_words": 10,
    });

    let result = orchestrator
        .execute_workflow(workflow_id, &workflow, input)
        .await?;

    // Verify final content is the revised version
    let content = result["content"].as_str().unwrap();
    assert!(content.len() > "Rust is good.".len());
    assert!(content.contains("memory safety"));

    // Verify workflow went through revision
    let metadata = orchestrator.get_workflow_metadata(workflow_id).await?;
    assert_eq!(metadata.total_steps, 4); // generate + critique + revise + critique
    assert_eq!(metadata.revision_count, 1);

    Ok(())
}

#[tokio::test]
async fn test_multiple_revisions_then_pass() -> Result<()> {
    let llm = mock_llm_with_responses(vec![
        // Generate
        json!({"content": "Rust.", "word_count": 1}),
        // Critique 1 (fail)
        json!({"decision": "revise", "feedback": "Too short", "quality_score": 20}),
        // Revise 1
        json!({"content": "Rust is a language.", "word_count": 4}),
        // Critique 2 (fail)
        json!({"decision": "revise", "feedback": "Still too brief", "quality_score": 50}),
        // Revise 2
        json!({"content": "Rust is a systems programming language with safety.", "word_count": 9}),
        // Critique 3 (pass)
        json!({"decision": "pass", "feedback": "Good enough", "quality_score": 80}),
    ]);

    let orchestrator = setup_test_orchestrator(llm).await?;
    let workflow = critique_revise_workflow();

    let workflow_id = ecl_core::WorkflowId::new();
    let result = orchestrator
        .execute_workflow(workflow_id, &workflow, json!({"topic": "Rust"}))
        .await?;

    assert!(result.is_object());

    let metadata = orchestrator.get_workflow_metadata(workflow_id).await?;
    assert_eq!(metadata.revision_count, 2);
    assert_eq!(metadata.status, "completed");

    Ok(())
}
```

#### Step 5.3: Max Revisions Test

**`crates/ecl-workflows/tests/integration/max_revisions.rs`**

```rust
//! Integration test: Max revisions (generates, critiques, revises x3, fails).
//!
//! This test verifies that the workflow correctly enforces the maximum
//! revision limit and fails gracefully when content cannot be improved
//! within the allowed iterations.

use ecl_core::Result;
use ecl_workflows::critique_revise_workflow;
use serde_json::json;

use super::utils::{setup_test_orchestrator, mock_llm_with_responses};

#[tokio::test]
async fn test_max_revisions_reached() -> Result<()> {
    let llm = mock_llm_with_responses(vec![
        // Generate
        json!({"content": "Bad content", "word_count": 2}),
        // Critique 1 (fail)
        json!({"decision": "revise", "feedback": "Not good", "quality_score": 30}),
        // Revise 1
        json!({"content": "Still bad", "word_count": 2}),
        // Critique 2 (fail)
        json!({"decision": "revise", "feedback": "Still not good", "quality_score": 35}),
        // Revise 2
        json!({"content": "Getting worse", "word_count": 2}),
        // Critique 3 (fail)
        json!({"decision": "revise", "feedback": "Worse", "quality_score": 25}),
        // Revise 3
        json!({"content": "Final attempt", "word_count": 2}),
        // Critique 4 (fail - but max revisions reached)
        json!({"decision": "revise", "feedback": "Still bad", "quality_score": 30}),
    ]);

    let orchestrator = setup_test_orchestrator(llm).await?;
    let workflow = critique_revise_workflow();

    let workflow_id = ecl_core::WorkflowId::new();
    let result = orchestrator
        .execute_workflow(workflow_id, &workflow, json!({"topic": "Test"}))
        .await;

    // Should fail with max revisions error
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("max"));
    assert!(err.to_string().contains("revision"));

    // Verify metadata
    let metadata = orchestrator.get_workflow_metadata(workflow_id).await?;
    assert_eq!(metadata.revision_count, 3);
    assert_eq!(metadata.status, "failed");
    assert!(metadata.error_message.is_some());

    Ok(())
}

#[tokio::test]
async fn test_max_revisions_configured_correctly() {
    let workflow = critique_revise_workflow();

    // Find the revision loop transition
    let revision_loop = workflow.transitions.iter().find(|t| {
        matches!(t, ecl_workflows::orchestrator::Transition::RevisionLoop { .. })
    });

    assert!(revision_loop.is_some());

    if let Some(ecl_workflows::orchestrator::Transition::RevisionLoop {
        max_iterations, ..
    }) = revision_loop {
        assert_eq!(*max_iterations, 3);
    }
}
```

#### Step 5.4: Recovery Test

**`crates/ecl-workflows/tests/integration/recovery.rs`**

```rust
//! Integration test: Recovery (kill mid-execution, verify resume).
//!
//! This test verifies that workflows can recover from crashes or
//! interruptions, resuming from their last persisted state.

use ecl_core::Result;
use ecl_workflows::{
    critique_revise_workflow,
    restate::{WorkflowServiceImpl, WorkflowStatus},
};
use serde_json::json;
use std::sync::Arc;

use super::utils::{
    setup_test_orchestrator,
    setup_test_restate_service,
    mock_llm_with_responses,
};

#[tokio::test]
async fn test_workflow_recovery_after_crash() -> Result<()> {
    // This test simulates a crash during workflow execution
    // and verifies that the workflow can resume from the persisted state

    let llm = mock_llm_with_responses(vec![
        // Generate step
        json!({"content": "Initial content", "word_count": 2}),
        // Critique step (will crash here)
        json!({"decision": "revise", "feedback": "Needs work", "quality_score": 50}),
    ]);

    let orchestrator = Arc::new(setup_test_orchestrator(llm.clone()).await?);
    let service = setup_test_restate_service(orchestrator.clone()).await?;

    let workflow_id = ecl_core::WorkflowId::new();
    let workflow = critique_revise_workflow();

    // Start workflow (will execute generate + critique)
    let start_input = ecl_workflows::restate::StartWorkflowInput {
        workflow_definition: workflow.clone(),
        input: json!({"topic": "Test"}),
    };

    // Simulate crash by dropping service mid-execution
    drop(service);

    // Verify state was persisted
    let repo = orchestrator.workflow_repository();
    let workflow_record = repo.get_workflow(workflow_id).await?;
    assert!(workflow_record.status == "running" || workflow_record.status == "paused");

    // Create new service instance (simulating restart)
    let llm_resumed = mock_llm_with_responses(vec![
        // Revise step (continues from where it left off)
        json!({"content": "Improved content with more detail", "word_count": 5}),
        // Critique step (pass)
        json!({"decision": "pass", "feedback": "Good", "quality_score": 85}),
    ]);

    let orchestrator_resumed = Arc::new(setup_test_orchestrator(llm_resumed).await?);
    let service_resumed = setup_test_restate_service(orchestrator_resumed.clone()).await?;

    // Resume workflow
    service_resumed.resume(workflow_id).await?;

    // Wait for completion
    let final_status = service_resumed.get_status(workflow_id).await?;

    match final_status {
        WorkflowStatus::Completed { .. } => {
            // Success!
        }
        _ => panic!("Expected completed status, got: {:?}", final_status),
    }

    Ok(())
}

#[tokio::test]
async fn test_pause_and_resume() -> Result<()> {
    let llm = mock_llm_with_responses(vec![
        json!({"content": "Test", "word_count": 1}),
        json!({"decision": "revise", "feedback": "More", "quality_score": 40}),
        json!({"content": "Test with more", "word_count": 3}),
        json!({"decision": "pass", "feedback": "Good", "quality_score": 80}),
    ]);

    let orchestrator = Arc::new(setup_test_orchestrator(llm).await?);
    let service = setup_test_restate_service(orchestrator).await?;

    let workflow_id = ecl_core::WorkflowId::new();

    // Start workflow
    let start_input = ecl_workflows::restate::StartWorkflowInput {
        workflow_definition: critique_revise_workflow(),
        input: json!({"topic": "Test"}),
    };

    service.start(start_input).await?;

    // Pause after first step
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    service.pause(workflow_id).await?;

    // Verify paused
    let status = service.get_status(workflow_id).await?;
    assert!(matches!(status, WorkflowStatus::Paused { .. }));

    // Resume
    service.resume(workflow_id).await?;

    // Wait for completion
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let final_status = service.get_status(workflow_id).await?;

    assert!(matches!(final_status, WorkflowStatus::Completed { .. }));

    Ok(())
}
```

#### Step 5.5: Test Utilities

**`crates/ecl-workflows/tests/integration/utils.rs`**

```rust
//! Test utilities for integration tests.

use ecl_core::{
    Result,
    llm::{LlmProvider, LlmRequest, LlmResponse},
};
use ecl_workflows::{
    orchestrator::WorkflowOrchestrator,
    restate::WorkflowServiceImpl,
};
use async_trait::async_trait;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Mock LLM provider for testing.
#[derive(Clone)]
pub struct MockLlmProvider {
    responses: Arc<Mutex<Vec<Value>>>,
    current_index: Arc<Mutex<usize>>,
}

impl MockLlmProvider {
    pub fn new(responses: Vec<Value>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            current_index: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl LlmProvider for MockLlmProvider {
    async fn generate(&self, _request: LlmRequest) -> Result<LlmResponse> {
        let mut index = self.current_index.lock().unwrap();
        let responses = self.responses.lock().unwrap();

        if *index >= responses.len() {
            panic!("Not enough mock responses provided");
        }

        let response = responses[*index].clone();
        *index += 1;

        Ok(LlmResponse {
            content: serde_json::to_string(&response)?,
            model: "mock".to_string(),
            usage: None,
        })
    }

    fn model_name(&self) -> &str {
        "mock-llm"
    }
}

/// Creates a mock LLM provider with pre-defined responses.
pub fn mock_llm_with_responses(responses: Vec<Value>) -> Arc<dyn LlmProvider> {
    Arc::new(MockLlmProvider::new(responses))
}

/// Sets up a test workflow orchestrator with in-memory database.
pub async fn setup_test_orchestrator(
    llm: Arc<dyn LlmProvider>,
) -> Result<WorkflowOrchestrator> {
    // Use in-memory SQLite for testing
    let database_url = ":memory:";

    WorkflowOrchestrator::builder()
        .llm_provider(llm)
        .database_url(database_url)
        .build()
        .await
}

/// Sets up a test Restate service.
pub async fn setup_test_restate_service(
    orchestrator: Arc<WorkflowOrchestrator>,
) -> Result<WorkflowServiceImpl> {
    Ok(WorkflowServiceImpl::new(orchestrator))
}
```

### Step 6: End-to-End Validation

**`crates/ecl-workflows/tests/e2e_test.rs`**

```rust
//! End-to-end test that validates the complete system.
//!
//! This test runs the full Critique-Revise workflow through:
//! - CLI command invocation
//! - Restate service
//! - Workflow orchestration
//! - Database persistence
//! - Observability logging
//!
//! Requires:
//! - Restate runtime running locally
//! - Database available
//! - LLM provider configured (or mock)

use assert_cmd::Command;
use predicates::prelude::*;
use ecl_core::Result;

#[test]
#[ignore] // Requires Restate runtime
fn test_e2e_critique_revise_workflow() -> Result<()> {
    // Start workflow via CLI
    let mut cmd = Command::cargo_bin("ecl")?;
    cmd.arg("run")
        .arg("critique-revise")
        .arg("--topic")
        .arg("Benefits of Rust")
        .arg("--wait");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Workflow started"))
        .stdout(predicate::str::contains("completed successfully"));

    Ok(())
}

#[test]
#[ignore]
fn test_e2e_status_command() -> Result<()> {
    // First, start a workflow
    let mut run_cmd = Command::cargo_bin("ecl")?;
    run_cmd.arg("run")
        .arg("critique-revise")
        .arg("--topic")
        .arg("Test");

    let output = run_cmd.output()?;
    let stdout = String::from_utf8(output.stdout)?;

    // Extract workflow ID from output
    let workflow_id = stdout
        .lines()
        .find(|line| line.contains("Workflow started:"))
        .and_then(|line| line.split(':').nth(1))
        .map(|id| id.trim())
        .expect("Could not find workflow ID");

    // Check status
    let mut status_cmd = Command::cargo_bin("ecl")?;
    status_cmd.arg("status").arg(workflow_id);

    status_cmd.assert()
        .success()
        .stdout(predicate::str::contains("Workflow ID"))
        .stdout(predicate::str::contains("Status:"));

    Ok(())
}

#[test]
fn test_e2e_list_workflows() -> Result<()> {
    let mut cmd = Command::cargo_bin("ecl")?;
    cmd.arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available Workflows"))
        .stdout(predicate::str::contains("critique-revise"));

    Ok(())
}

#[test]
fn test_e2e_list_workflows_detailed() -> Result<()> {
    let mut cmd = Command::cargo_bin("ecl")?;
    cmd.arg("list").arg("--detailed");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Description:"))
        .stdout(predicate::str::contains("Steps:"))
        .stdout(predicate::str::contains("generate"))
        .stdout(predicate::str::contains("critique"))
        .stdout(predicate::str::contains("revise"));

    Ok(())
}
```

---

## Testing Strategy

**Test coverage goal:** ≥95%

### Unit Tests

1. **Workflow Definition Tests** (`critique_revise.rs`):
   - Workflow structure validation
   - Step dependencies
   - Transition configuration
   - Revision source configuration

2. **Restate Service Tests** (`workflow_service.rs`):
   - Workflow status serialization
   - State transitions
   - Error handling

3. **CLI Command Tests**:
   - Argument parsing
   - Input validation
   - Output formatting

### Integration Tests

1. **Happy Path**:
   - Generate → Critique → Pass
   - Minimal execution time
   - Correct database records
   - Proper logging

2. **Revision Path**:
   - Single revision cycle
   - Multiple revision cycles
   - Revision count tracking
   - Feedback propagation

3. **Max Revisions**:
   - Enforces iteration limit
   - Fails gracefully
   - Error message clarity
   - Metadata accuracy

4. **Recovery**:
   - Crash recovery
   - State persistence
   - Pause/resume functionality
   - Idempotent execution

### End-to-End Tests

1. **CLI Integration**:
   - Run command works
   - Status command shows accurate state
   - List command displays workflows
   - Output formatting

2. **Full System**:
   - Restate integration
   - Database persistence
   - Observability logs
   - Error propagation

### Property-Based Tests

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_workflow_always_terminates(
        topic in "\\PC{1,100}",
        max_revisions in 1u32..10,
    ) {
        // Test that workflow always terminates within
        // reasonable bounds regardless of input
    }

    #[test]
    fn test_revision_count_never_exceeds_max(
        max_revisions in 1u32..10,
    ) {
        // Verify revision count constraint is always enforced
    }
}
```

---

## Potential Blockers

### 1. Restate SDK Integration Complexity

**Risk:** Restate's Virtual Objects pattern may require significant API changes.

**Mitigation:**
- Start with minimal Restate integration
- Use in-memory state for initial testing
- Gradually add durability features
- Reference Restate SDK examples

**Fallback:** Implement simpler state machine without Restate for initial version.

### 2. Type Erasure in Workflow Orchestrator

**Risk:** Passing different step input/output types through orchestrator.

**Mitigation:**
- Use `serde_json::Value` for workflow-level data
- Let steps handle type conversion
- Provide clear error messages for type mismatches

**Solution:** Already addressed in Stage 2.4 design.

### 3. CLI Error Handling

**Risk:** Poor user experience with unclear error messages.

**Mitigation:**
- Use `anyhow` for CLI-level errors with context
- Provide actionable error messages
- Include suggestions for common issues
- Add `--verbose` flag for debugging

### 4. Integration Test Flakiness

**Risk:** Tests may be flaky due to timing or external dependencies.

**Mitigation:**
- Use deterministic mock LLM provider
- In-memory database for tests
- Avoid actual Restate calls in unit tests
- Mark E2E tests as `#[ignore]` by default

### 5. Database Migration State

**Risk:** Tests may fail if database schema is out of sync.

**Mitigation:**
- Run migrations in test setup
- Use separate test database
- Clean up between tests
- Document migration process

---

## Acceptance Criteria

### Functionality
- [ ] Complete Critique-Revise workflow executes end-to-end
- [ ] Happy path works (generate → critique → pass)
- [ ] Revision path works (generate → critique → revise → critique → pass)
- [ ] Max revisions enforced (fails after 3 revisions)
- [ ] Recovery works (can resume after crash/pause)

### CLI
- [ ] `ecl run critique-revise --topic "X"` starts workflow
- [ ] `ecl status <workflow-id>` shows current state
- [ ] `ecl list` shows available workflows
- [ ] Clear, actionable error messages
- [ ] Progress indicators for long-running operations

### Integration
- [ ] Restate service correctly manages workflow state
- [ ] Database records created for workflows and steps
- [ ] Observability logs capture all execution details
- [ ] Error handling propagates through all layers

### Testing
- [ ] All integration tests pass
- [ ] Test coverage ≥95%
- [ ] E2E tests work with Restate runtime
- [ ] Mock tests run without external dependencies
- [ ] Property tests verify invariants

### Code Quality
- [ ] No `unwrap()` in library code
- [ ] All public APIs have rustdoc
- [ ] Follows Rust anti-patterns guide
- [ ] No compiler warnings
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo fmt` check passes

### Documentation
- [ ] README includes usage examples
- [ ] CLI help text is clear
- [ ] API documentation is complete
- [ ] Examples compile and run

---

## Estimated Effort

**Total Time:** 6-8 hours

### Breakdown

1. **Workflow Definition** (1 hour)
   - Create `critique_revise_workflow()`
   - Write unit tests
   - Documentation

2. **Restate Integration** (2 hours)
   - Implement `WorkflowService`
   - State management
   - Durable execution patterns
   - Testing

3. **CLI Commands** (2 hours)
   - Run command
   - Status command
   - List command
   - Error handling
   - Output formatting

4. **Integration Tests** (2 hours)
   - Happy path test
   - Revision path test
   - Max revisions test
   - Recovery test
   - Test utilities

5. **E2E Validation** (1 hour)
   - E2E test suite
   - Manual testing
   - Documentation

6. **Polish and Documentation** (1 hour)
   - README updates
   - Usage examples
   - Final testing
   - Code cleanup

---

## Files to Create/Modify

### New Files

```
crates/ecl-workflows/src/
├── definitions/
│   ├── mod.rs
│   └── critique_revise.rs
└── restate/
    ├── mod.rs
    └── workflow_service.rs

crates/ecl-cli/src/
├── main.rs
└── commands/
    ├── mod.rs
    ├── run.rs
    ├── status.rs
    └── list.rs

crates/ecl-workflows/tests/
├── e2e_test.rs
└── integration/
    ├── mod.rs
    ├── happy_path.rs
    ├── revision_path.rs
    ├── max_revisions.rs
    ├── recovery.rs
    └── utils.rs
```

### Modified Files

```
crates/ecl-workflows/src/lib.rs
crates/ecl-cli/Cargo.toml (add dependencies: clap, assert_cmd, predicates)
```

---

## Dependencies to Add

**`crates/ecl-cli/Cargo.toml`:**

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
serde_json = { workspace = true }
anyhow = "1"
ecl-core = { path = "../ecl-core" }
ecl-workflows = { path = "../ecl-workflows" }

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

---

## Next Steps

After completing Stage 2.7:

1. **Validate Phase 2 Completion**:
   - Review [Phase 2 Checklist](./0010-phase-2-08-checklist.md)
   - Ensure all acceptance criteria met
   - Verify test coverage ≥95%
   - Check all stages complete

2. **System Testing**:
   - Run full E2E test suite
   - Manual testing with real LLM
   - Performance baseline measurement
   - Load testing (optional)

3. **Documentation**:
   - Update main README
   - Create user guide
   - API documentation review
   - Examples verification

4. **Prepare for Phase 3**:
   - Review Phase 3 requirements
   - Identify refactoring needs
   - Plan architecture enhancements
   - Stakeholder demo (optional)

---

**Phase 2 Complete!** Review the [Phase 2 Checklist](./0010-phase-2-08-checklist.md) before moving to Phase 3.

---

**Document Version:** 1.0
**Last Updated:** 2026-01-23
**Status:** Ready for Implementation
