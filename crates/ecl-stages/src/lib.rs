//! Built-in stage implementations for the ECL pipeline runner.
//!
//! Provides stages for extraction, transformation, and output:
//! - [`ExtractStage`] — delegates to a `SourceAdapter` to fetch content
//! - [`CsvParseStage`] — parses CSV content into structured records (fan-out)
//! - [`NormalizeStage`] — passthrough (placeholder for future format conversion)
//! - [`FilterStage`] — glob-based include/exclude filtering
//! - [`FieldMapStage`] — field renaming, date parsing, padding, regex extraction
//! - [`ValidateStage`] — field-level validation with hard/soft severity
//! - [`JoinStage`] — batch join of two streams by key (inner/left/full)
//! - [`AggregateStage`] — batch grouping with aggregate functions (sum/max/min/count/avg/first/last)
//! - [`LookupStage`] — static value mapping through lookup tables
//! - [`DateParseStage`] — date string parsing to RFC3339 format
//! - [`TimezoneStage`] — local datetime to UTC conversion via ZIP code lookup
//! - [`DecompressStage`] — ZIP/GZIP archive extraction (fan-out)
//! - [`EmitStage`] — writes pipeline items to the output directory

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![deny(clippy::panic)]

pub mod aggregate;
pub mod csv_parse;
pub mod date_parse;
pub mod decompress;
pub mod emit;
pub mod extract;
pub mod field_map;
pub mod filter;
pub mod join;
pub mod lookup;
pub mod normalize;
pub mod timezone;
pub mod validate;

pub use aggregate::AggregateStage;
pub use csv_parse::CsvParseStage;
pub use date_parse::DateParseStage;
pub use decompress::DecompressStage;
pub use emit::EmitStage;
pub use extract::ExtractStage;
pub use field_map::FieldMapStage;
pub use filter::FilterStage;
pub use join::JoinStage;
pub use lookup::LookupStage;
pub use normalize::NormalizeStage;
pub use timezone::TimezoneStage;
pub use validate::ValidateStage;
