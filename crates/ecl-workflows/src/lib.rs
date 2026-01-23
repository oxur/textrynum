#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! ECL Workflows Library
//!
//! Restate workflow definitions for ECL.

// Re-export core types
pub use ecl_core::{Error, Result};

#[cfg(test)]
mod tests {
    #[test]
    fn test_crate_compiles() {
        // This test ensures the crate compiles
        // More substantive tests come in later stages
    }
}
