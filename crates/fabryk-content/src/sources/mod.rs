//! Source reference validation.
//!
//! Cross-validates source references across concept cards, configuration,
//! and the filesystem. Categories are fully generic -- callers provide a
//! `HashMap<String, SourceCategory>` rather than hardcoding specific
//! category names.
//!
//! # Modules
//!
//! - [`types`]: Data types for validation results
//! - [`resolver`]: Title / alias / fuzzy resolution
//! - [`scanner`]: Filesystem scanning for source references in frontmatter
//! - [`validator`]: Orchestrates cross-validation

pub mod resolver;
pub mod scanner;
pub mod types;
pub mod validator;

pub use resolver::{SourceCategory, SourceResolver, extract_title_from_filename};
pub use scanner::{ScanStats, scan_content_for_sources, scan_content_for_sources_with_stats};
pub use types::*;
pub use validator::{ValidationMode, validate_sources};
