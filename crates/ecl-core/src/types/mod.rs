//! Core types for ECL workflows.

mod ids;
mod proptests;
mod step_result;
mod workflow_state;

pub use ids::{StepId, WorkflowId};
pub use step_result::StepResult;
pub use workflow_state::{StepMetadata, WorkflowState};
