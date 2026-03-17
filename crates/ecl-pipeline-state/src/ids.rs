//! Newtype identifiers for pipeline state.
//!
//! All IDs are name-based strings (not auto-incremented integers) for
//! human readability and checkpoint stability across runs.

use serde::{Deserialize, Serialize};

/// Deterministic, name-based stage identifier.
/// NOT an auto-incremented integer (dagrs anti-pattern).
/// Stable across runs for checkpoint compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StageId(String);

impl StageId {
    /// Create a new stage ID from a name.
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the stage ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for StageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Blake3 content hash, stored as hex string for JSON readability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blake3Hash(String);

impl Blake3Hash {
    /// Create a new Blake3Hash from a hex string.
    pub fn new(hex: impl Into<String>) -> Self {
        Self(hex.into())
    }

    /// Get the hash as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if the hash is empty (not yet computed).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl std::fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Unique identifier for a pipeline run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(String);

impl RunId {
    /// Create a new run ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the run ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_id_new_and_as_str() {
        let id = StageId::new("extract");
        assert_eq!(id.as_str(), "extract");
    }

    #[test]
    fn test_stage_id_display() {
        let id = StageId::new("normalize");
        assert_eq!(format!("{id}"), "normalize");
    }

    #[test]
    fn test_stage_id_ord() {
        let a = StageId::new("alpha");
        let b = StageId::new("beta");
        let c = StageId::new("charlie");
        assert!(a < b);
        assert!(b < c);
    }

    #[test]
    fn test_stage_id_serde_roundtrip() {
        let id = StageId::new("fetch-gdrive");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: StageId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_run_id_new_and_as_str() {
        let id = RunId::new("run-2026-03-13-001");
        assert_eq!(id.as_str(), "run-2026-03-13-001");
    }

    #[test]
    fn test_run_id_display() {
        let id = RunId::new("run-42");
        assert_eq!(format!("{id}"), "run-42");
    }

    #[test]
    fn test_run_id_serde_roundtrip() {
        let id = RunId::new("run-abc");
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: RunId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_blake3_hash_new_and_as_str() {
        let hash = Blake3Hash::new("aabbccdd");
        assert_eq!(hash.as_str(), "aabbccdd");
    }

    #[test]
    fn test_blake3_hash_is_empty() {
        assert!(Blake3Hash::new("").is_empty());
        assert!(!Blake3Hash::new("abc").is_empty());
    }

    #[test]
    fn test_blake3_hash_display() {
        let hash = Blake3Hash::new("deadbeef");
        assert_eq!(format!("{hash}"), "deadbeef");
    }

    #[test]
    fn test_blake3_hash_equality() {
        let a = Blake3Hash::new("abc123");
        let b = Blake3Hash::new("abc123");
        let c = Blake3Hash::new("def456");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_blake3_hash_serde_roundtrip() {
        let hash = Blake3Hash::new("a7f3b2c8d9e0");
        let json = serde_json::to_string(&hash).unwrap();
        let deserialized: Blake3Hash = serde_json::from_str(&json).unwrap();
        assert_eq!(hash, deserialized);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_run_id_proptest_roundtrip(s in "\\PC{1,100}") {
            let id = RunId::new(s);
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: RunId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(id, deserialized);
        }

        #[test]
        fn test_stage_id_proptest_roundtrip(s in "\\PC{1,100}") {
            let id = StageId::new(s);
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: StageId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(id, deserialized);
        }

        #[test]
        fn test_blake3_hash_proptest_roundtrip(s in "[0-9a-f]{0,64}") {
            let hash = Blake3Hash::new(s);
            let json = serde_json::to_string(&hash).unwrap();
            let deserialized: Blake3Hash = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(hash, deserialized);
        }
    }
}
