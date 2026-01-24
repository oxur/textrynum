//! # fabryk-mcp
//!
//! Model Context Protocol server for Fabryk knowledge fabric.
//!
//! This crate exposes Fabryk to AI agents via MCP:
//! - MCP tools for query, get, list, store, relate
//! - MCP resources for context injection
//! - MCP prompts for common workflows
//! - OAuth authentication flow
//! - Permission enforcement via Fabryk ACL

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod server;
pub mod handlers;
pub mod session;
pub mod formatting;

pub use error::{Error, Result};
pub use server::McpServer;
