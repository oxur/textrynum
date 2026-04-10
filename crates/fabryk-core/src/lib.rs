//! Fabryk Core — shared types, traits, errors, and utilities.
//!
//! This crate provides the foundational types used across all Fabryk crates.
//! It has no internal Fabryk dependencies (dependency level 0).
//!
//! # Modules
//!
//! - [`error`]: Error types and Result alias
//! - [`probe`]: Backend diagnostic probe trait
//! - [`state`]: Generic application state container
//! - [`traits`]: Core traits for domain abstraction
//! - [`util`]: File, path, and ID utilities

#![doc = include_str!("../README.md")]

pub mod deploy;
pub mod error;
pub mod orchestration;
pub mod probe;
pub mod service;
pub mod slot;
pub mod state;
pub mod traits;
pub mod util;

// Re-export key types at crate root for convenience
pub use error::{Error, Result, log_error_chain};
pub use orchestration::{ManagedService, ServiceOrchestrator, ServiceStatus};
pub use probe::BackendProbe;
pub use service::{
    RetryConfig, ServiceHandle, ServiceState, Transition, spawn_with_retry, wait_all_ready,
};
pub use slot::BackendSlot;
pub use state::AppState;
pub use traits::ConfigManager;
pub use traits::ConfigProvider;

// Convenience re-exports from util
pub use util::ids::{id_from_path, normalize_id};
pub use util::resolver::PathResolver;
