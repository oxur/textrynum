# Stage 2.4: Workflow Orchestrator

**Version:** 1.0
**Date:** 2026-01-23
**Phase:** 2 - Step Abstraction Framework
**Stage:** 2.4 - Workflow Orchestrator
**Duration:** 6-7 hours
**Status:** Ready for Implementation

---

## Goal

Create an orchestrator that composes steps into executable workflows with support for dependency resolution, sequential/conditional transitions, and bounded revision loops.

## Dependencies

This stage requires completion of:

- **Stage 2.1**: Step trait definition (Step, StepContext, RetryPolicy, StepRegistry)
- **Stage 2.2**: Step executor (StepExecutor, StepExecution)
- **Stage 2.3**: Toy workflow step implementations (GenerateStep, CritiqueStep, ReviseStep)

## Overview

The Workflow Orchestrator is the execution engine that:

1. **Declares workflows** through structured definitions (not code)
2. **Resolves dependencies** via topological sorting
3. **Executes steps** in the correct order with proper data flow
4. **Handles revision loops** with bounded iteration limits
5. **Aggregates results** into a comprehensive WorkflowResult

This enables declarative workflow composition where workflows are data structures, not hardcoded logic.

---

## Detailed Implementation Steps

### Step 1: Define Core Workflow Types

**Before writing code:**
1. Load `assets/ai/ai-rust/guides/11-anti-patterns.md`
2. Load `assets/ai/ai-rust/guides/03-error-handling.md`
3. Review Step trait and StepExecutor from previous stages

**File: `crates/ecl-workflows/src/definition.rs`**

```rust
//! Declarative workflow definition types.
//!
//! This module provides data structures for defining workflows as compositions
//! of steps with dependencies and transitions.

use ecl_core::StepId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A declarative workflow definition.
///
/// Workflows are defined as a directed acyclic graph (DAG) of steps with
/// explicit transitions between them.
///
/// # Examples
///
/// ```
/// use ecl_workflows::WorkflowDefinition;
/// use ecl_core::StepId;
///
/// let workflow = WorkflowDefinition {
///     id: "critique-revise".to_string(),
///     name: "Critique-Revise Toy Workflow".to_string(),
///     steps: vec![
///         // Step definitions here
///     ],
///     transitions: vec![
///         // Transitions here
///     ],
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkflowDefinition {
    /// Unique identifier for this workflow
    pub id: String,

    /// Human-readable workflow name
    pub name: String,

    /// Steps that compose this workflow
    pub steps: Vec<StepDefinition>,

    /// Transitions between steps
    pub transitions: Vec<Transition>,
}

/// Definition of a single step within a workflow.
///
/// Steps can depend on other steps, forming a dependency graph that
/// the orchestrator will resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StepDefinition {
    /// The step's unique identifier
    pub step_id: StepId,

    /// Steps that must complete before this step can run
    pub depends_on: Vec<StepId>,

    /// Optional: Which step can request revision of this step's output
    /// Used for revision loops
    pub revision_source: Option<StepId>,

    /// Optional: Configuration data for this step instance
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Types of transitions between workflow steps.
///
/// Transitions define how data flows and control passes between steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Transition {
    /// Simple sequential transition: step A always proceeds to step B
    Sequential {
        /// Source step
        from: StepId,
        /// Destination step
        to: StepId,
    },

    /// Conditional transition: proceed only if condition evaluates true
    Conditional {
        /// Source step
        from: StepId,
        /// Destination step
        to: StepId,
        /// JSONPath or expression to evaluate on step output
        condition: String,
    },

    /// Revision loop: validator can request reviser to improve output
    RevisionLoop {
        /// Step that performs revision (e.g., "revise")
        reviser: StepId,
        /// Step that validates and may request revision (e.g., "critique")
        validator: StepId,
        /// Maximum number of revision iterations
        max_iterations: u32,
    },
}

impl WorkflowDefinition {
    /// Creates a new workflow definition.
    ///
    /// # Examples
    ///
    /// ```
    /// use ecl_workflows::WorkflowDefinition;
    ///
    /// let workflow = WorkflowDefinition::new("my-workflow", "My Workflow");
    /// ```
    pub fn new<S1, S2>(id: S1, name: S2) -> Self
    where
        S1: Into<String>,
        S2: Into<String>,
    {
        Self {
            id: id.into(),
            name: name.into(),
            steps: Vec::new(),
            transitions: Vec::new(),
        }
    }

    /// Adds a step to the workflow.
    pub fn add_step(&mut self, step: StepDefinition) {
        self.steps.push(step);
    }

    /// Adds a transition to the workflow.
    pub fn add_transition(&mut self, transition: Transition) {
        self.transitions.push(transition);
    }

    /// Returns all steps that have no dependencies (entry points).
    pub fn entry_steps(&self) -> Vec<&StepDefinition> {
        self.steps
            .iter()
            .filter(|step| step.depends_on.is_empty())
            .collect()
    }

    /// Validates the workflow definition for structural correctness.
    ///
    /// Checks:
    /// - No duplicate step IDs
    /// - All dependencies reference valid steps
    /// - All transitions reference valid steps
    /// - No circular dependencies (DAG constraint)
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Check for duplicate step IDs
        let mut seen_ids = std::collections::HashSet::new();
        for step in &self.steps {
            if !seen_ids.insert(step.step_id.as_str()) {
                return Err(ValidationError::DuplicateStepId {
                    step_id: step.step_id.clone(),
                });
            }
        }

        // Build step lookup map
        let step_map: HashMap<&str, &StepDefinition> = self
            .steps
            .iter()
            .map(|s| (s.step_id.as_str(), s))
            .collect();

        // Check all dependencies reference valid steps
        for step in &self.steps {
            for dep in &step.depends_on {
                if !step_map.contains_key(dep.as_str()) {
                    return Err(ValidationError::InvalidDependency {
                        step_id: step.step_id.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }

        // Check all transitions reference valid steps
        for transition in &self.transitions {
            match transition {
                Transition::Sequential { from, to }
                | Transition::Conditional { from, to, .. } => {
                    if !step_map.contains_key(from.as_str()) {
                        return Err(ValidationError::InvalidTransition {
                            reason: format!("Unknown source step: {}", from),
                        });
                    }
                    if !step_map.contains_key(to.as_str()) {
                        return Err(ValidationError::InvalidTransition {
                            reason: format!("Unknown destination step: {}", to),
                        });
                    }
                }
                Transition::RevisionLoop {
                    reviser, validator, ..
                } => {
                    if !step_map.contains_key(reviser.as_str()) {
                        return Err(ValidationError::InvalidTransition {
                            reason: format!("Unknown reviser step: {}", reviser),
                        });
                    }
                    if !step_map.contains_key(validator.as_str()) {
                        return Err(ValidationError::InvalidTransition {
                            reason: format!("Unknown validator step: {}", validator),
                        });
                    }
                }
            }
        }

        // Check for cycles using DFS
        self.check_for_cycles()?;

        Ok(())
    }

    /// Checks for circular dependencies using depth-first search.
    fn check_for_cycles(&self) -> Result<(), ValidationError> {
        use std::collections::{HashMap, HashSet};

        let step_map: HashMap<&str, &StepDefinition> = self
            .steps
            .iter()
            .map(|s| (s.step_id.as_str(), s))
            .collect();

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for step in &self.steps {
            if self.has_cycle_dfs(
                step.step_id.as_str(),
                &step_map,
                &mut visited,
                &mut rec_stack,
            )? {
                return Err(ValidationError::CircularDependency {
                    step_id: step.step_id.clone(),
                });
            }
        }

        Ok(())
    }

    fn has_cycle_dfs<'a>(
        &'a self,
        step_id: &str,
        step_map: &HashMap<&str, &'a StepDefinition>,
        visited: &mut HashSet<&'a str>,
        rec_stack: &mut HashSet<&'a str>,
    ) -> Result<bool, ValidationError> {
        if rec_stack.contains(step_id) {
            return Ok(true); // Cycle detected
        }

        if visited.contains(step_id) {
            return Ok(false); // Already visited
        }

        visited.insert(step_id);
        rec_stack.insert(step_id);

        if let Some(step) = step_map.get(step_id) {
            for dep in &step.depends_on {
                if self.has_cycle_dfs(dep.as_str(), step_map, visited, rec_stack)? {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(step_id);
        Ok(false)
    }
}

impl StepDefinition {
    /// Creates a new step definition.
    pub fn new(step_id: StepId) -> Self {
        Self {
            step_id,
            depends_on: Vec::new(),
            revision_source: None,
            config: HashMap::new(),
        }
    }

    /// Adds a dependency to this step.
    pub fn depends_on(mut self, dep: StepId) -> Self {
        self.depends_on.push(dep);
        self
    }

    /// Sets the revision source for this step.
    pub fn with_revision_source(mut self, source: StepId) -> Self {
        self.revision_source = Some(source);
        self
    }

    /// Adds a configuration value.
    pub fn with_config<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Serialize,
    {
        self.config.insert(
            key.into(),
            serde_json::to_value(value).expect("Failed to serialize config value"),
        );
        self
    }
}

/// Errors that can occur during workflow validation.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ValidationError {
    /// Duplicate step ID found
    #[error("Duplicate step ID: {step_id}")]
    DuplicateStepId {
        /// The duplicate step ID
        step_id: StepId,
    },

    /// Invalid dependency reference
    #[error("Step {step_id} depends on non-existent step {dependency}")]
    InvalidDependency {
        /// Step with the invalid dependency
        step_id: StepId,
        /// The non-existent dependency
        dependency: StepId,
    },

    /// Invalid transition
    #[error("Invalid transition: {reason}")]
    InvalidTransition {
        /// Reason for the invalid transition
        reason: String,
    },

    /// Circular dependency detected
    #[error("Circular dependency detected involving step: {step_id}")]
    CircularDependency {
        /// A step involved in the cycle
        step_id: StepId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_definition_new() {
        let workflow = WorkflowDefinition::new("test", "Test Workflow");
        assert_eq!(workflow.id, "test");
        assert_eq!(workflow.name, "Test Workflow");
        assert!(workflow.steps.is_empty());
        assert!(workflow.transitions.is_empty());
    }

    #[test]
    fn test_step_definition_builder() {
        let step = StepDefinition::new(StepId::from("test"))
            .depends_on(StepId::from("dep1"))
            .depends_on(StepId::from("dep2"))
            .with_revision_source(StepId::from("validator"));

        assert_eq!(step.step_id.as_str(), "test");
        assert_eq!(step.depends_on.len(), 2);
        assert_eq!(step.revision_source.unwrap().as_str(), "validator");
    }

    #[test]
    fn test_entry_steps() {
        let mut workflow = WorkflowDefinition::new("test", "Test");
        workflow.add_step(StepDefinition::new(StepId::from("a")));
        workflow.add_step(
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
        );

        let entries = workflow.entry_steps();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].step_id.as_str(), "a");
    }

    #[test]
    fn test_validate_duplicate_step_id() {
        let mut workflow = WorkflowDefinition::new("test", "Test");
        workflow.add_step(StepDefinition::new(StepId::from("a")));
        workflow.add_step(StepDefinition::new(StepId::from("a")));

        let result = workflow.validate();
        assert!(matches!(
            result,
            Err(ValidationError::DuplicateStepId { .. })
        ));
    }

    #[test]
    fn test_validate_invalid_dependency() {
        let mut workflow = WorkflowDefinition::new("test", "Test");
        workflow.add_step(
            StepDefinition::new(StepId::from("a")).depends_on(StepId::from("nonexistent")),
        );

        let result = workflow.validate();
        assert!(matches!(
            result,
            Err(ValidationError::InvalidDependency { .. })
        ));
    }

    #[test]
    fn test_validate_circular_dependency() {
        let mut workflow = WorkflowDefinition::new("test", "Test");
        workflow.add_step(
            StepDefinition::new(StepId::from("a")).depends_on(StepId::from("b")),
        );
        workflow.add_step(
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
        );

        let result = workflow.validate();
        assert!(matches!(
            result,
            Err(ValidationError::CircularDependency { .. })
        ));
    }

    #[test]
    fn test_validate_valid_workflow() {
        let mut workflow = WorkflowDefinition::new("test", "Test");
        workflow.add_step(StepDefinition::new(StepId::from("a")));
        workflow.add_step(
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
        );
        workflow.add_transition(Transition::Sequential {
            from: StepId::from("a"),
            to: StepId::from("b"),
        });

        assert!(workflow.validate().is_ok());
    }
}
```

---

### Step 2: Define Workflow Result Types

**File: `crates/ecl-workflows/src/result.rs`**

```rust
//! Workflow execution result types.

use ecl_core::{StepId, WorkflowId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Result of executing a complete workflow.
///
/// Contains all step outputs, metadata, and execution traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct WorkflowResult {
    /// The workflow instance ID
    pub workflow_id: WorkflowId,

    /// The workflow definition ID
    pub definition_id: String,

    /// Final status of the workflow
    pub status: WorkflowStatus,

    /// Outputs from each step (keyed by step_id)
    pub step_outputs: HashMap<StepId, serde_json::Value>,

    /// Execution metadata for each step
    pub step_metadata: HashMap<StepId, StepMetadata>,

    /// Total workflow execution duration
    pub total_duration: Duration,

    /// Timestamp when workflow started
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// Timestamp when workflow completed
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Any error that terminated the workflow
    pub error: Option<String>,
}

/// Workflow execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WorkflowStatus {
    /// Workflow completed successfully
    Success,

    /// Workflow failed with an error
    Failed,

    /// Workflow is still running
    Running,

    /// Workflow was cancelled
    Cancelled,
}

/// Metadata about a single step execution within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StepMetadata {
    /// When this step started
    pub started_at: chrono::DateTime<chrono::Utc>,

    /// When this step completed
    pub completed_at: chrono::DateTime<chrono::Utc>,

    /// Step execution duration
    pub duration: Duration,

    /// Number of retry attempts
    pub retry_count: u32,

    /// Number of revisions (for steps in revision loops)
    pub revision_count: u32,

    /// Whether the step succeeded
    pub success: bool,

    /// Error message if failed
    pub error: Option<String>,

    /// Additional custom metadata
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl WorkflowResult {
    /// Creates a new workflow result for a running workflow.
    pub fn new(workflow_id: WorkflowId, definition_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            workflow_id,
            definition_id,
            status: WorkflowStatus::Running,
            step_outputs: HashMap::new(),
            step_metadata: HashMap::new(),
            total_duration: Duration::from_secs(0),
            started_at: now,
            completed_at: None,
            error: None,
        }
    }

    /// Records output from a step execution.
    pub fn record_step_output<T>(&mut self, step_id: StepId, output: &T)
    where
        T: Serialize,
    {
        let value = serde_json::to_value(output).expect("Failed to serialize step output");
        self.step_outputs.insert(step_id, value);
    }

    /// Records metadata for a step execution.
    pub fn record_step_metadata(&mut self, step_id: StepId, metadata: StepMetadata) {
        self.step_metadata.insert(step_id, metadata);
    }

    /// Marks the workflow as successfully completed.
    pub fn complete_success(&mut self) {
        let now = chrono::Utc::now();
        self.status = WorkflowStatus::Success;
        self.completed_at = Some(now);
        self.total_duration = (now - self.started_at)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));
    }

    /// Marks the workflow as failed with an error.
    pub fn complete_failure(&mut self, error: String) {
        let now = chrono::Utc::now();
        self.status = WorkflowStatus::Failed;
        self.completed_at = Some(now);
        self.error = Some(error);
        self.total_duration = (now - self.started_at)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));
    }

    /// Gets the output from a specific step.
    pub fn get_step_output<T>(&self, step_id: &StepId) -> Option<Result<T, serde_json::Error>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.step_outputs
            .get(step_id)
            .map(|value| serde_json::from_value(value.clone()))
    }
}

impl StepMetadata {
    /// Creates new step metadata for a step that just started.
    pub fn started() -> Self {
        let now = chrono::Utc::now();
        Self {
            started_at: now,
            completed_at: now,
            duration: Duration::from_secs(0),
            retry_count: 0,
            revision_count: 0,
            success: false,
            error: None,
            custom: HashMap::new(),
        }
    }

    /// Marks the step as completed successfully.
    pub fn complete_success(&mut self) {
        let now = chrono::Utc::now();
        self.completed_at = now;
        self.duration = (now - self.started_at)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));
        self.success = true;
    }

    /// Marks the step as failed.
    pub fn complete_failure(&mut self, error: String) {
        let now = chrono::Utc::now();
        self.completed_at = now;
        self.duration = (now - self.started_at)
            .to_std()
            .unwrap_or_else(|_| Duration::from_secs(0));
        self.success = false;
        self.error = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_result_creation() {
        let workflow_id = WorkflowId::new();
        let result = WorkflowResult::new(workflow_id, "test-workflow".to_string());

        assert_eq!(result.status, WorkflowStatus::Running);
        assert_eq!(result.definition_id, "test-workflow");
        assert!(result.completed_at.is_none());
    }

    #[test]
    fn test_workflow_result_success() {
        let workflow_id = WorkflowId::new();
        let mut result = WorkflowResult::new(workflow_id, "test".to_string());

        std::thread::sleep(std::time::Duration::from_millis(10));
        result.complete_success();

        assert_eq!(result.status, WorkflowStatus::Success);
        assert!(result.completed_at.is_some());
        assert!(result.total_duration.as_millis() >= 10);
    }

    #[test]
    fn test_workflow_result_failure() {
        let workflow_id = WorkflowId::new();
        let mut result = WorkflowResult::new(workflow_id, "test".to_string());

        result.complete_failure("Something went wrong".to_string());

        assert_eq!(result.status, WorkflowStatus::Failed);
        assert_eq!(result.error, Some("Something went wrong".to_string()));
        assert!(result.completed_at.is_some());
    }

    #[test]
    fn test_record_step_output() {
        let workflow_id = WorkflowId::new();
        let mut result = WorkflowResult::new(workflow_id, "test".to_string());

        let step_id = StepId::from("test-step");
        result.record_step_output(step_id.clone(), &"test output");

        let output: String = result.get_step_output(&step_id).unwrap().unwrap();
        assert_eq!(output, "test output");
    }

    #[test]
    fn test_step_metadata_lifecycle() {
        let mut metadata = StepMetadata::started();
        assert!(!metadata.success);

        std::thread::sleep(std::time::Duration::from_millis(10));
        metadata.complete_success();

        assert!(metadata.success);
        assert!(metadata.duration.as_millis() >= 10);
    }
}
```

---

### Step 3: Implement Orchestrator Core

**File: `crates/ecl-workflows/src/orchestrator.rs`**

```rust
//! Workflow orchestration engine.
//!
//! The orchestrator executes workflows by resolving dependencies,
//! running steps in the correct order, and handling control flow.

use crate::definition::{StepDefinition, Transition, WorkflowDefinition};
use crate::result::{StepMetadata, WorkflowResult, WorkflowStatus};
use ecl_core::{Error, Result, StepId, WorkflowId};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// The workflow orchestration engine.
///
/// Responsible for executing workflow definitions by:
/// 1. Topologically sorting steps by dependencies
/// 2. Executing steps in order
/// 3. Handling data flow between steps
/// 4. Managing revision loops
/// 5. Aggregating results
pub struct WorkflowOrchestrator {
    /// Workflow definitions keyed by workflow ID
    definitions: HashMap<String, WorkflowDefinition>,
}

impl WorkflowOrchestrator {
    /// Creates a new workflow orchestrator.
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Registers a workflow definition.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow definition is invalid.
    pub fn register(&mut self, definition: WorkflowDefinition) -> Result<()> {
        definition
            .validate()
            .map_err(|e| Error::config(format!("Invalid workflow definition: {}", e)))?;

        info!(
            workflow_id = definition.id,
            step_count = definition.steps.len(),
            "Registered workflow definition"
        );

        self.definitions.insert(definition.id.clone(), definition);
        Ok(())
    }

    /// Gets a workflow definition by ID.
    pub fn get_definition(&self, workflow_id: &str) -> Option<&WorkflowDefinition> {
        self.definitions.get(workflow_id)
    }

    /// Executes a workflow.
    ///
    /// This is a simplified version that demonstrates the orchestration logic.
    /// In practice, this would integrate with StepExecutor from Stage 2.2.
    ///
    /// # Arguments
    ///
    /// * `definition_id` - The workflow definition to execute
    /// * `input` - Initial input data for the workflow
    ///
    /// # Returns
    ///
    /// Returns a WorkflowResult containing all step outputs and metadata.
    pub async fn run(
        &self,
        definition_id: &str,
        input: serde_json::Value,
    ) -> Result<WorkflowResult> {
        let definition = self
            .definitions
            .get(definition_id)
            .ok_or_else(|| Error::WorkflowNotFound {
                id: definition_id.to_string(),
            })?;

        let workflow_id = WorkflowId::new();
        let mut result = WorkflowResult::new(workflow_id, definition_id.to_string());

        info!(
            %workflow_id,
            definition_id,
            "Starting workflow execution"
        );

        // Sort steps topologically
        let execution_order = match self.topological_sort(&definition.steps) {
            Ok(order) => order,
            Err(e) => {
                error!("Failed to sort steps topologically: {}", e);
                result.complete_failure(format!("Dependency resolution failed: {}", e));
                return Ok(result);
            }
        };

        debug!("Execution order: {:?}", execution_order);

        // Execute steps in order
        let mut step_outputs: HashMap<StepId, serde_json::Value> = HashMap::new();
        step_outputs.insert(StepId::from("__input__"), input);

        for step_id in execution_order {
            let step_def = definition
                .steps
                .iter()
                .find(|s| s.step_id == step_id)
                .expect("Step must exist");

            match self
                .execute_step(step_def, &step_outputs, &mut result)
                .await
            {
                Ok(output) => {
                    step_outputs.insert(step_id.clone(), output);
                }
                Err(e) => {
                    error!(step_id = %step_id, error = %e, "Step execution failed");
                    result.complete_failure(format!("Step {} failed: {}", step_id, e));
                    return Ok(result);
                }
            }
        }

        // Handle revision loops
        if let Err(e) = self
            .handle_revision_loops(definition, &mut step_outputs, &mut result)
            .await
        {
            error!(error = %e, "Revision loop handling failed");
            result.complete_failure(format!("Revision loop failed: {}", e));
            return Ok(result);
        }

        result.complete_success();
        info!(%workflow_id, "Workflow completed successfully");
        Ok(result)
    }

    /// Sorts steps topologically by their dependencies.
    ///
    /// Uses Kahn's algorithm for topological sorting.
    ///
    /// # Errors
    ///
    /// Returns an error if the dependency graph contains cycles.
    fn topological_sort(&self, steps: &[StepDefinition]) -> Result<Vec<StepId>> {
        // Build adjacency list and in-degree map
        let mut in_degree: HashMap<&StepId, usize> = HashMap::new();
        let mut adjacency: HashMap<&StepId, Vec<&StepId>> = HashMap::new();

        // Initialize
        for step in steps {
            in_degree.insert(&step.step_id, 0);
            adjacency.insert(&step.step_id, Vec::new());
        }

        // Build graph
        for step in steps {
            for dep in &step.depends_on {
                if let Some(adj_list) = adjacency.get_mut(dep) {
                    adj_list.push(&step.step_id);
                }
                *in_degree.get_mut(&step.step_id).unwrap() += 1;
            }
        }

        // Kahn's algorithm
        let mut queue: VecDeque<&StepId> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut sorted = Vec::new();

        while let Some(step_id) = queue.pop_front() {
            sorted.push(step_id.clone());

            if let Some(neighbors) = adjacency.get(step_id) {
                for neighbor in neighbors {
                    let degree = in_degree.get_mut(neighbor).unwrap();
                    *degree -= 1;
                    if *degree == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        // Check for cycles
        if sorted.len() != steps.len() {
            return Err(Error::config(
                "Circular dependency detected in workflow definition",
            ));
        }

        Ok(sorted)
    }

    /// Executes a single step.
    ///
    /// This is a placeholder that would integrate with StepExecutor in practice.
    async fn execute_step(
        &self,
        step_def: &StepDefinition,
        step_outputs: &HashMap<StepId, serde_json::Value>,
        result: &mut WorkflowResult,
    ) -> Result<serde_json::Value> {
        let mut metadata = StepMetadata::started();

        info!(step_id = %step_def.step_id, "Executing step");

        // In a real implementation, this would:
        // 1. Gather inputs from dependencies
        // 2. Look up the Step implementation from StepRegistry
        // 3. Call StepExecutor.execute_step()
        // 4. Return the output
        //
        // For now, we simulate with a placeholder
        let output = serde_json::json!({
            "step_id": step_def.step_id.as_str(),
            "status": "completed",
            "placeholder": true
        });

        metadata.complete_success();
        result.record_step_metadata(step_def.step_id.clone(), metadata);
        result.record_step_output(step_def.step_id.clone(), &output);

        Ok(output)
    }

    /// Handles revision loops in the workflow.
    ///
    /// Revision loops allow a validator step to request revisions from a reviser step.
    async fn handle_revision_loops(
        &self,
        definition: &WorkflowDefinition,
        step_outputs: &mut HashMap<StepId, serde_json::Value>,
        result: &mut WorkflowResult,
    ) -> Result<()> {
        for transition in &definition.transitions {
            if let Transition::RevisionLoop {
                reviser,
                validator,
                max_iterations,
            } = transition
            {
                info!(
                    %reviser,
                    %validator,
                    max_iterations,
                    "Processing revision loop"
                );

                let mut iteration = 0;

                loop {
                    iteration += 1;

                    if iteration > *max_iterations {
                        warn!(
                            %reviser,
                            %validator,
                            iteration,
                            "Maximum revision iterations exceeded"
                        );
                        return Err(Error::MaxRevisionsExceeded {
                            attempts: iteration - 1,
                        });
                    }

                    // In a real implementation:
                    // 1. Get validator output and check decision
                    // 2. If "Pass", break the loop
                    // 3. If "Revise", execute reviser step again
                    // 4. Update step outputs
                    //
                    // For now, simulate validation passing after 1 iteration
                    debug!(iteration, "Revision loop iteration");

                    // Simulate: validator always passes on first check
                    break;
                }

                info!(%reviser, %validator, iterations = iteration, "Revision loop completed");
            }
        }

        Ok(())
    }
}

impl Default for WorkflowOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::definition::StepDefinition;

    #[test]
    fn test_orchestrator_creation() {
        let orchestrator = WorkflowOrchestrator::new();
        assert_eq!(orchestrator.definitions.len(), 0);
    }

    #[test]
    fn test_register_workflow() {
        let mut orchestrator = WorkflowOrchestrator::new();
        let workflow = WorkflowDefinition::new("test", "Test Workflow");

        let result = orchestrator.register(workflow);
        assert!(result.is_ok());
        assert!(orchestrator.get_definition("test").is_some());
    }

    #[test]
    fn test_topological_sort_simple() {
        let orchestrator = WorkflowOrchestrator::new();

        let steps = vec![
            StepDefinition::new(StepId::from("a")),
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
            StepDefinition::new(StepId::from("c")).depends_on(StepId::from("b")),
        ];

        let sorted = orchestrator.topological_sort(&steps).unwrap();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].as_str(), "a");
        assert_eq!(sorted[1].as_str(), "b");
        assert_eq!(sorted[2].as_str(), "c");
    }

    #[test]
    fn test_topological_sort_diamond() {
        let orchestrator = WorkflowOrchestrator::new();

        let steps = vec![
            StepDefinition::new(StepId::from("a")),
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
            StepDefinition::new(StepId::from("c")).depends_on(StepId::from("a")),
            StepDefinition::new(StepId::from("d"))
                .depends_on(StepId::from("b"))
                .depends_on(StepId::from("c")),
        ];

        let sorted = orchestrator.topological_sort(&steps).unwrap();
        assert_eq!(sorted.len(), 4);
        assert_eq!(sorted[0].as_str(), "a");
        // b and c can be in any order
        assert_eq!(sorted[3].as_str(), "d");
    }

    #[test]
    fn test_topological_sort_circular() {
        let orchestrator = WorkflowOrchestrator::new();

        let steps = vec![
            StepDefinition::new(StepId::from("a")).depends_on(StepId::from("b")),
            StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")),
        ];

        let result = orchestrator.topological_sort(&steps);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_run_workflow_not_found() {
        let orchestrator = WorkflowOrchestrator::new();

        let result = orchestrator
            .run("nonexistent", serde_json::json!({}))
            .await;

        assert!(matches!(result, Err(Error::WorkflowNotFound { .. })));
    }

    #[tokio::test]
    async fn test_run_simple_workflow() {
        let mut orchestrator = WorkflowOrchestrator::new();

        let mut workflow = WorkflowDefinition::new("simple", "Simple Workflow");
        workflow.add_step(StepDefinition::new(StepId::from("step1")));

        orchestrator.register(workflow).unwrap();

        let result = orchestrator.run("simple", serde_json::json!({})).await;
        assert!(result.is_ok());

        let workflow_result = result.unwrap();
        assert_eq!(workflow_result.status, WorkflowStatus::Success);
    }
}
```

---

### Step 4: Wire Up Module Structure

**File: `crates/ecl-workflows/src/lib.rs`**

```rust
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Workflows Library
//!
//! Provides workflow orchestration capabilities for the ECL system.

pub mod definition;
pub mod orchestrator;
pub mod result;

// Re-exports for convenience
pub use definition::{StepDefinition, Transition, WorkflowDefinition};
pub use orchestrator::WorkflowOrchestrator;
pub use result::{StepMetadata, WorkflowResult, WorkflowStatus};
```

**File: `crates/ecl-workflows/README.md`**

```markdown
# ecl-workflows

Workflow orchestration engine for ECL.

## Features

- Declarative workflow definitions
- Dependency resolution via topological sorting
- Sequential, conditional, and revision loop transitions
- Comprehensive result aggregation
```

---

## Testing Strategy

### Unit Tests

Each module includes comprehensive unit tests (see `#[cfg(test)]` sections above).

**Test coverage targets:**
- `definition.rs`: ≥95% (validation, builders, cycle detection)
- `result.rs`: ≥95% (lifecycle, data recording)
- `orchestrator.rs`: ≥95% (topological sort, execution flow)

### Integration Tests

**File: `crates/ecl-workflows/tests/integration_test.rs`**

```rust
use ecl_core::StepId;
use ecl_workflows::{
    StepDefinition, Transition, WorkflowDefinition, WorkflowOrchestrator, WorkflowStatus,
};

#[tokio::test]
async fn test_complete_workflow_execution() {
    let mut orchestrator = WorkflowOrchestrator::new();

    // Define a simple three-step workflow
    let mut workflow = WorkflowDefinition::new("integration-test", "Integration Test Workflow");

    workflow.add_step(StepDefinition::new(StepId::from("generate")));
    workflow.add_step(
        StepDefinition::new(StepId::from("critique")).depends_on(StepId::from("generate")),
    );
    workflow.add_step(
        StepDefinition::new(StepId::from("revise"))
            .depends_on(StepId::from("critique"))
            .with_revision_source(StepId::from("critique")),
    );

    workflow.add_transition(Transition::Sequential {
        from: StepId::from("generate"),
        to: StepId::from("critique"),
    });
    workflow.add_transition(Transition::RevisionLoop {
        reviser: StepId::from("revise"),
        validator: StepId::from("critique"),
        max_iterations: 3,
    });

    orchestrator.register(workflow).unwrap();

    let input = serde_json::json!({
        "topic": "Rust async programming"
    });

    let result = orchestrator.run("integration-test", input).await.unwrap();

    assert_eq!(result.status, WorkflowStatus::Success);
    assert!(result.step_outputs.contains_key(&StepId::from("generate")));
    assert!(result.step_outputs.contains_key(&StepId::from("critique")));
}

#[test]
fn test_workflow_validation() {
    // Test duplicate step IDs
    let mut workflow = WorkflowDefinition::new("test", "Test");
    workflow.add_step(StepDefinition::new(StepId::from("duplicate")));
    workflow.add_step(StepDefinition::new(StepId::from("duplicate")));

    assert!(workflow.validate().is_err());
}

#[test]
fn test_complex_dependency_graph() {
    let mut workflow = WorkflowDefinition::new("complex", "Complex Dependencies");

    // Create a diamond dependency pattern:
    //     A
    //    / \
    //   B   C
    //    \ /
    //     D
    workflow.add_step(StepDefinition::new(StepId::from("a")));
    workflow.add_step(StepDefinition::new(StepId::from("b")).depends_on(StepId::from("a")));
    workflow.add_step(StepDefinition::new(StepId::from("c")).depends_on(StepId::from("a")));
    workflow.add_step(
        StepDefinition::new(StepId::from("d"))
            .depends_on(StepId::from("b"))
            .depends_on(StepId::from("c")),
    );

    assert!(workflow.validate().is_ok());

    let orchestrator = WorkflowOrchestrator::new();
    let sorted = orchestrator
        .topological_sort(&workflow.steps)
        .unwrap();

    // Verify valid topological order
    let positions: std::collections::HashMap<_, _> = sorted
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    assert!(positions["a"] < positions["b"]);
    assert!(positions["a"] < positions["c"]);
    assert!(positions["b"] < positions["d"]);
    assert!(positions["c"] < positions["d"]);
}
```

### Property Tests

**File: `crates/ecl-workflows/tests/property_test.rs`**

```rust
use ecl_core::StepId;
use ecl_workflows::{StepDefinition, WorkflowDefinition, WorkflowOrchestrator};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_topological_sort_preserves_dependencies(step_count in 1usize..=10) {
        let mut workflow = WorkflowDefinition::new("prop-test", "Property Test");

        // Create a chain: step0 -> step1 -> step2 -> ...
        workflow.add_step(StepDefinition::new(StepId::from("step0")));

        for i in 1..step_count {
            workflow.add_step(
                StepDefinition::new(StepId::from(format!("step{}", i)))
                    .depends_on(StepId::from(format!("step{}", i - 1)))
            );
        }

        let orchestrator = WorkflowOrchestrator::new();
        let sorted = orchestrator.topological_sort(&workflow.steps).unwrap();

        // Verify order is preserved
        for i in 0..step_count {
            assert_eq!(sorted[i].as_str(), format!("step{}", i));
        }
    }
}
```

### Test Data

None required for this stage - orchestrator works with abstract definitions.

---

## Potential Blockers

### 1. Topological Sort Complexity

**Issue**: Complex dependency graphs may have performance implications.

**Mitigation**:
- Kahn's algorithm is O(V + E), efficient for typical workflow sizes
- Add validation to reject workflows with >1000 steps
- Consider caching sorted order in production

### 2. Revision Loop Termination

**Issue**: Poorly configured max_iterations could cause excessive LLM calls.

**Mitigation**:
- Enforce reasonable bounds (e.g., max_iterations ≤ 10)
- Add timeout at orchestrator level
- Log warnings when approaching iteration limits

### 3. Step Output Data Flow

**Issue**: Passing outputs between steps requires careful type handling.

**Mitigation**:
- Use `serde_json::Value` as intermediate representation
- Let steps handle deserialization of their inputs
- Provide clear error messages for type mismatches

### 4. Restate Integration Complexity

**Issue**: Integrating with Restate's durable execution model is non-trivial.

**Mitigation**:
- This stage provides the orchestration logic as a library
- Stage 2.7 (Integration) will handle Restate-specific details
- Keep orchestrator logic stateless and testable

---

## Acceptance Criteria

- [ ] `WorkflowDefinition` can express multi-step workflows with dependencies
- [ ] `WorkflowDefinition::validate()` catches structural errors
- [ ] Topological sort correctly orders steps by dependencies
- [ ] Topological sort detects circular dependencies
- [ ] `WorkflowOrchestrator` executes steps in correct order
- [ ] Revision loops have bounded iteration with `max_iterations`
- [ ] `WorkflowResult` captures all step outputs and metadata
- [ ] `WorkflowResult` records success/failure status
- [ ] All unit tests pass with ≥95% coverage
- [ ] Integration tests demonstrate end-to-end orchestration
- [ ] Property tests validate topological sort invariants
- [ ] No `unwrap()` in library code
- [ ] All public items have rustdoc comments
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test --package ecl-workflows` passes

---

## Estimated Effort

**Total Time:** 6-7 hours

**Breakdown:**
- Workflow definition types: 1.5 hours
- Workflow result types: 1 hour
- Topological sort implementation: 1.5 hours
- Orchestrator execution logic: 1.5 hours
- Revision loop handling: 1 hour
- Testing (unit + integration + property): 1-1.5 hours

---

## Files Created

```
ecl-workflows/src/
├── lib.rs               (Module root with re-exports)
├── definition.rs        (WorkflowDefinition, StepDefinition, Transition)
├── orchestrator.rs      (WorkflowOrchestrator with execution engine)
└── result.rs            (WorkflowResult, StepMetadata)

ecl-workflows/tests/
├── integration_test.rs  (End-to-end orchestration tests)
└── property_test.rs     (Property-based tests for topological sort)

ecl-workflows/
└── README.md            (Crate documentation)
```

---

## Notes

1. **Topological Sorting**: Uses Kahn's algorithm, which is both efficient and produces a stable ordering.

2. **Revision Loops**: Represented as a special Transition type. The orchestrator will check validator output and conditionally re-execute the reviser step.

3. **Data Flow**: This stage uses `serde_json::Value` for step I/O to keep the orchestrator generic. Stage 2.7 (Integration) will connect this to concrete Step implementations.

4. **Restate Integration**: Deferred to Stage 2.7. This stage focuses on the core orchestration logic as a testable library.

5. **Validation**: Comprehensive validation ensures workflows are structurally sound before execution.

6. **Testing**: Includes unit tests, integration tests, and property tests to ensure correctness.

---

**Next:** [Stage 2.5: SQLx Persistence Layer](./0007-phase-2-05-sqlx-persistence.md)
