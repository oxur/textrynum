//! Integration tests for the critique-revise loop workflow.

use ecl_core::Error;
use ecl_workflows::critique_loop::{CritiqueLoopInput, CritiqueLoopWorkflow};
use std::sync::Arc;

use crate::common::{mock_always_revise, mock_pass_immediately, mock_revise_once, TestHarness};

#[tokio::test]
async fn test_critique_loop_passes_immediately() {
    let harness = TestHarness::with_llm(mock_pass_immediately());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let input = CritiqueLoopInput::new("Test topic");

    let output = workflow
        .run(input.clone())
        .await
        .expect("Workflow should complete successfully");

    assert_eq!(output.workflow_id, input.workflow_id);
    assert_eq!(output.final_text, "Generated content.");
    assert_eq!(output.revision_count, 0, "Should have zero revisions");
    assert_eq!(output.critiques.len(), 1, "Should have one critique");
    assert!(output.critiques[0].contains("Looks good!"));
}

#[tokio::test]
async fn test_critique_loop_with_one_revision() {
    let harness = TestHarness::with_llm(mock_revise_once());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let input = CritiqueLoopInput::new("Test topic");

    let output = workflow
        .run(input.clone())
        .await
        .expect("Workflow should complete after one revision");

    assert_eq!(output.workflow_id, input.workflow_id);
    assert_eq!(
        output.final_text, "Improved draft with more detail.",
        "Should have revised text"
    );
    assert_eq!(output.revision_count, 1, "Should have one revision");
    assert_eq!(output.critiques.len(), 2, "Should have two critiques");
    assert!(output.critiques[0].contains("Needs work"));
    assert!(output.critiques[1].contains("Much better!"));
}

#[tokio::test]
async fn test_critique_loop_max_revisions_exceeded() {
    let harness = TestHarness::with_llm(mock_always_revise());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let input = CritiqueLoopInput::new("Test topic");

    let result = workflow.run(input).await;

    assert!(result.is_err(), "Should fail with max revisions exceeded");

    let Error::MaxRevisionsExceeded { attempts } = result.unwrap_err() else {
        unreachable!("Expected MaxRevisionsExceeded error");
    };

    assert_eq!(attempts, 3, "Should fail after 3 revisions (default MAX)");
}

#[tokio::test]
async fn test_critique_loop_custom_max_revisions() {
    let harness = TestHarness::with_llm(mock_always_revise());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let input = CritiqueLoopInput::new("Test topic").with_max_revisions(1);

    let result = workflow.run(input).await;

    assert!(
        result.is_err(),
        "Should fail with custom max revisions exceeded"
    );

    let Error::MaxRevisionsExceeded { attempts } = result.unwrap_err() else {
        unreachable!("Expected MaxRevisionsExceeded error");
    };

    assert_eq!(attempts, 1, "Should fail after 1 revision (custom max)");
}

#[tokio::test]
async fn test_critique_loop_multiple_revisions() {
    // Mock that revises twice then passes
    let mock = Arc::new(ecl_core::llm::MockLlmProvider::new(vec![
        "Draft 1.".to_string(),
        r#"{"decision": "revise", "critique": "First critique", "feedback": "Add introduction"}"#
            .to_string(),
        "Draft 2 with intro.".to_string(),
        r#"{"decision": "revise", "critique": "Second critique", "feedback": "Add conclusion"}"#
            .to_string(),
        "Draft 3 with intro and conclusion.".to_string(),
        r#"{"decision": "pass", "critique": "Perfect!"}"#.to_string(),
    ]));

    let workflow = CritiqueLoopWorkflow::new(mock);
    let input = CritiqueLoopInput::new("Test topic");

    let output = workflow
        .run(input)
        .await
        .expect("Workflow should complete after two revisions");

    assert_eq!(
        output.final_text, "Draft 3 with intro and conclusion.",
        "Should have final revised text"
    );
    assert_eq!(output.revision_count, 2, "Should have two revisions");
    assert_eq!(output.critiques.len(), 3, "Should have three critiques");
    assert!(output.critiques[0].contains("First critique"));
    assert!(output.critiques[1].contains("Second critique"));
    assert!(output.critiques[2].contains("Perfect!"));
}

#[tokio::test]
async fn test_critique_loop_preserves_all_critiques() {
    let harness = TestHarness::with_llm(mock_revise_once());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let input = CritiqueLoopInput::new("Test topic");

    let output = workflow.run(input).await.expect("Workflow should complete");

    // Verify all critiques are preserved
    assert_eq!(output.critiques.len(), 2);
    for critique in &output.critiques {
        assert!(!critique.is_empty(), "All critiques should have content");
    }
}

#[tokio::test]
async fn test_critique_loop_different_topics() {
    let harness = TestHarness::with_llm(mock_pass_immediately());
    let workflow = CritiqueLoopWorkflow::new(harness.llm.clone());

    let topics = vec!["AI Safety", "Rust Performance", "Distributed Systems"];

    for topic in topics {
        let input = CritiqueLoopInput::new(topic);
        let output = workflow
            .run(input.clone())
            .await
            .expect("Each workflow should succeed");

        assert_eq!(output.workflow_id, input.workflow_id);
        assert!(!output.final_text.is_empty());
    }
}
