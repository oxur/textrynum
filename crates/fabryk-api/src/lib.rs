//! # fabryk-api
//!
//! HTTP API server for Fabryk knowledge fabric.
//!
//! This crate provides the HTTP API server:
//! - RESTful API endpoints for knowledge operations
//! - Authentication and authorization middleware
//! - Request validation and error handling
//! - OpenAPI/Swagger documentation
//! - Rate limiting and monitoring

#![warn(missing_docs)]
#![warn(clippy::all)]
#![forbid(unsafe_code)]

pub mod error;
pub mod routes;
pub mod middleware;
pub mod server;

pub use error::{Error, Result};
pub use server::Server;
