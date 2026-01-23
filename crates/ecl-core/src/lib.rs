#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Core Library
//!
//! Core types, traits, and utilities for the ECL workflow orchestration system.

pub mod error;
pub mod llm;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use llm::{CompletionRequest, CompletionResponse, LlmProvider, Message};
pub use types::{CritiqueDecision, StepId, StepMetadata, StepResult, WorkflowId, WorkflowState};
