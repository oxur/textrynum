//! Integration tests for the simple 2-step workflow.

use ecl_workflows::simple::{SimpleWorkflowInput, SimpleWorkflowService};

// Re-use the test harness
use crate::common::{mock_pass_immediately, TestHarness};

#[tokio::test]
async fn test_simple_workflow_completes_successfully() {
    let harness = TestHarness::with_llm(mock_pass_immediately());
    let service = SimpleWorkflowService::new(harness.llm.clone());

    let input = SimpleWorkflowInput::new("Rust programming benefits");

    let output = service
        .run_simple(input.clone())
        .await
        .expect("Workflow should complete successfully");

    assert_eq!(output.workflow_id, input.workflow_id);
    assert!(!output.generated_text.is_empty());
    assert!(!output.critique.is_empty());
    assert_eq!(output.generated_text, "Generated content.");
}

#[tokio::test]
async fn test_simple_workflow_with_custom_topic() {
    let harness = TestHarness::with_responses(vec![
        "Content about AI safety.".to_string(),
        "Critique: Well-structured argument.".to_string(),
    ]);
    let service = SimpleWorkflowService::new(harness.llm.clone());

    let input = SimpleWorkflowInput::new("AI safety considerations");

    let output = service
        .run_simple(input)
        .await
        .expect("Workflow should complete successfully");

    assert_eq!(output.generated_text, "Content about AI safety.");
    assert_eq!(output.critique, "Critique: Well-structured argument.");
}

#[tokio::test]
async fn test_simple_workflow_preserves_workflow_id() {
    let harness = TestHarness::new();
    let service = SimpleWorkflowService::new(harness.llm.clone());

    let input1 = SimpleWorkflowInput::new("Topic A");
    let input2 = SimpleWorkflowInput::new("Topic B");

    assert_ne!(
        input1.workflow_id, input2.workflow_id,
        "Each workflow should have unique ID"
    );

    let output1 = service
        .run_simple(input1.clone())
        .await
        .expect("Workflow 1 should complete");
    let output2 = service
        .run_simple(input2.clone())
        .await
        .expect("Workflow 2 should complete");

    assert_eq!(output1.workflow_id, input1.workflow_id);
    assert_eq!(output2.workflow_id, input2.workflow_id);
    assert_ne!(output1.workflow_id, output2.workflow_id);
}

#[tokio::test]
async fn test_simple_workflow_multiple_executions() {
    let harness = TestHarness::with_responses(vec![
        "First response.".to_string(),
        "First critique.".to_string(),
        "Second response.".to_string(),
        "Second critique.".to_string(),
    ]);
    let service = SimpleWorkflowService::new(harness.llm.clone());

    // First execution
    let output1 = service
        .run_simple(SimpleWorkflowInput::new("Topic 1"))
        .await
        .expect("First execution should succeed");

    assert_eq!(output1.generated_text, "First response.");
    assert_eq!(output1.critique, "First critique.");

    // Second execution with same service
    let output2 = service
        .run_simple(SimpleWorkflowInput::new("Topic 2"))
        .await
        .expect("Second execution should succeed");

    assert_eq!(output2.generated_text, "Second response.");
    assert_eq!(output2.critique, "Second critique.");
}
