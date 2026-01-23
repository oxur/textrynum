//! Integration test suite for ECL workflows.
//!
//! Tests the complete workflow execution paths with mock LLM providers,
//! verifying the interaction between workflow orchestration, LLM calls,
//! and decision handling.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

mod common;
mod integration;
