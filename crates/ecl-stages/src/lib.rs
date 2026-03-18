//! Built-in stage implementations for the ECL pipeline runner.
//!
//! Provides five stages:
//! - [`ExtractStage`] — delegates to a `SourceAdapter` to fetch content
//! - [`CsvParseStage`] — parses CSV content into structured records (fan-out)
//! - [`NormalizeStage`] — passthrough (placeholder for future format conversion)
//! - [`FilterStage`] — glob-based include/exclude filtering
//! - [`EmitStage`] — writes pipeline items to the output directory

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod csv_parse;
pub mod emit;
pub mod extract;
pub mod field_map;
pub mod filter;
pub mod normalize;
pub mod validate;

pub use csv_parse::CsvParseStage;
pub use emit::EmitStage;
pub use extract::ExtractStage;
pub use field_map::FieldMapStage;
pub use filter::FilterStage;
pub use normalize::NormalizeStage;
pub use validate::ValidateStage;
