//! Error conversion between fabryk-core and MCP error types.
//!
//! Bridges `fabryk_core::Error` to `rmcp::model::ErrorData` so that
//! tool implementations can use `?` with fabryk operations and have
//! errors automatically converted to MCP-compliant error responses.

use rmcp::model::ErrorData;

/// Extension trait for converting fabryk errors to MCP errors.
pub trait McpErrorExt {
    /// Convert to an MCP ErrorData.
    fn to_mcp_error(&self) -> ErrorData;
}

impl McpErrorExt for fabryk_core::Error {
    fn to_mcp_error(&self) -> ErrorData {
        match self {
            fabryk_core::Error::NotFound { .. } | fabryk_core::Error::FileNotFound { .. } => {
                ErrorData::resource_not_found(self.to_string(), None)
            }
            fabryk_core::Error::Parse(_) | fabryk_core::Error::Yaml(_) => {
                ErrorData::parse_error(self.to_string(), None)
            }
            fabryk_core::Error::Config(_) => ErrorData::invalid_params(self.to_string(), None),
            fabryk_core::Error::Json(_) => ErrorData::parse_error(self.to_string(), None),
            _ => ErrorData::internal_error(self.to_string(), None),
        }
    }
}

/// Extension trait for converting errors to MCP ErrorData with a descriptive
/// context string.
///
/// Extends [`McpErrorExt`] (which provides no-context `to_mcp_error()`) with a
/// context-aware variant used by tool handlers that want to include a
/// human-readable description of what was happening when the error occurred.
///
/// Maps error types to MCP protocol error codes:
/// - NotFound/FileNotFound → RESOURCE_NOT_FOUND
/// - Config → INVALID_PARAMS
/// - All other errors → INTERNAL_ERROR (with context prefix)
pub trait McpErrorContextExt {
    /// Convert to MCP ErrorData with a descriptive context string.
    fn to_mcp_error_with_context(&self, context: &str) -> ErrorData;
}

impl McpErrorContextExt for fabryk_core::Error {
    fn to_mcp_error_with_context(&self, context: &str) -> ErrorData {
        if self.is_not_found() {
            ErrorData::resource_not_found(format!("Not found: {}", self), None)
        } else if self.is_config() {
            ErrorData::invalid_params(format!("Configuration error: {}", self), None)
        } else {
            ErrorData::internal_error(format!("{}: {}", context, self), None)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_not_found_error_conversion() {
        let err = fabryk_core::Error::not_found("Concept", "major-triad");
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("not found"));
    }

    #[test]
    fn test_file_not_found_error_conversion() {
        let err = fabryk_core::Error::file_not_found(PathBuf::from("/missing.md"));
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("File not found"));
    }

    #[test]
    fn test_config_error_conversion() {
        let err = fabryk_core::Error::config("bad config");
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("bad config"));
    }

    #[test]
    fn test_parse_error_conversion() {
        let err = fabryk_core::Error::parse("syntax error");
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("syntax error"));
    }

    #[test]
    fn test_operation_error_conversion() {
        let err = fabryk_core::Error::operation("something failed");
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("something failed"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = fabryk_core::Error::io(io_err);
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("access denied"));
    }

    #[test]
    fn test_to_mcp_error() {
        let err = fabryk_core::Error::not_found("Item", "xyz");
        let mcp_err = err.to_mcp_error();
        assert!(mcp_err.message.contains("not found"));
    }

    // McpErrorContextExt tests

    #[test]
    fn test_context_not_found() {
        let err = fabryk_core::Error::file_not_found(PathBuf::from("/test.txt"));
        let mcp_err = err.to_mcp_error_with_context("Error reading file");
        assert!(mcp_err.message.contains("Not found"));
    }

    #[test]
    fn test_context_config() {
        let err = fabryk_core::Error::config("missing field");
        let mcp_err = err.to_mcp_error_with_context("Error loading config");
        assert!(mcp_err.message.contains("Configuration error"));
    }

    #[test]
    fn test_context_internal() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err = fabryk_core::Error::io(io_err);
        let mcp_err = err.to_mcp_error_with_context("Error accessing file");
        assert!(mcp_err.message.contains("Error accessing file"));
    }
}
