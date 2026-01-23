//! Comprehensive tests for error handling and coverage.

use ecl_core::Error;

#[test]
fn test_llm_error_creation() {
    let err = Error::llm("API request failed");
    assert_eq!(err.to_string(), "LLM error: API request failed");
    assert!(err.is_retryable(), "LLM errors should be retryable");
}

#[test]
fn test_llm_error_with_source() {
    let source = std::io::Error::other("network error");
    let err = Error::llm_with_source("Request failed", Box::new(source));
    assert!(err.to_string().contains("Request failed"));
    assert!(err.is_retryable());
}

#[test]
fn test_validation_error() {
    let err = Error::validation("Invalid input format");
    assert_eq!(err.to_string(), "Validation error: Invalid input format");
    assert!(!err.is_retryable(), "Validation errors are not retryable");
}

#[test]
fn test_validation_error_with_field() {
    let err = Error::validation_field("email", "must be valid email address");
    match &err {
        Error::Validation { field, message } => {
            assert_eq!(field, &Some("email".to_string()));
            assert_eq!(message, "must be valid email address");
        }
        _ => unreachable!("Expected Validation error"),
    }
    assert!(!err.is_retryable());
}

#[test]
fn test_timeout_error() {
    let err = Error::Timeout { seconds: 30 };
    assert_eq!(err.to_string(), "Step timed out after 30s");
    assert!(err.is_retryable(), "Timeout errors should be retryable");
}

#[test]
fn test_max_revisions_exceeded_error() {
    let err = Error::MaxRevisionsExceeded { attempts: 5 };
    assert_eq!(err.to_string(), "Maximum revisions exceeded: 5 attempts");
    assert!(
        !err.is_retryable(),
        "Max revisions errors are not retryable"
    );
}

#[test]
fn test_io_error_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: Error = io_err.into();
    assert!(err.to_string().contains("file not found"));
    assert!(err.is_retryable(), "I/O errors are retryable");
}

#[test]
fn test_serialization_error_conversion() {
    let json = r#"{"invalid": json}"#;
    let serde_err = serde_json::from_str::<serde_json::Value>(json).unwrap_err();
    let err: Error = serde_err.into();
    assert!(err.to_string().contains("Serialization error"));
    assert!(!err.is_retryable());
}

#[test]
fn test_error_debug_formatting() {
    let err = Error::MaxRevisionsExceeded { attempts: 3 };
    let debug_str = format!("{:?}", err);
    assert!(debug_str.contains("MaxRevisionsExceeded"));
    assert!(debug_str.contains("3"));
}

#[test]
fn test_multiple_error_types_retryability() {
    let errors = vec![
        (Error::llm("test"), true),
        (Error::validation("test"), false),
        (Error::Timeout { seconds: 10 }, true),
        (Error::MaxRevisionsExceeded { attempts: 3 }, false),
        (Error::Io(std::io::Error::other("test")), true),
    ];

    for (err, expected_retryable) in errors {
        assert_eq!(
            err.is_retryable(),
            expected_retryable,
            "Error {:?} retryability mismatch",
            err
        );
    }
}

#[test]
fn test_error_source_chain() {
    let io_err = std::io::Error::other("root cause");
    let err = Error::llm_with_source("wrapper", Box::new(io_err));

    // Verify we can access error as std::error::Error trait
    let std_err: &dyn std::error::Error = &err;
    assert!(std_err.source().is_some(), "Should have error source");
}

#[test]
fn test_validation_error_without_field() {
    let err = Error::validation("generic validation error");
    match &err {
        Error::Validation { field, message } => {
            assert_eq!(field, &None);
            assert_eq!(message, "generic validation error");
        }
        _ => unreachable!("Expected Validation error"),
    }
}
