//! Configurable metadata-based node filters for graph queries.
//!
//! This module provides [`MetadataNodeFilter`], a generic filter that checks
//! graph node metadata against values from MCP tool call arguments. It supports
//! two filter modes:
//!
//! - **Exact match**: the argument value must equal the node metadata value.
//! - **Ordered match**: the node metadata value must be at or above the
//!   argument value in a defined ordering (e.g., `["low", "medium", "high"]`).
//!
//! # Example
//!
//! ```rust
//! use fabryk_mcp_graph::MetadataNodeFilter;
//! use fabryk_mcp_graph::GraphNodeFilter;
//! use fabryk_graph::Node;
//!
//! let filter = MetadataNodeFilter::new()
//!     .with_exact("tier", "tier")
//!     .with_ordered("min_confidence", "extraction_confidence", &["low", "medium", "high"]);
//!
//! let node = Node::new("test", "Test")
//!     .with_metadata("tier", serde_json::Value::String("foundational".into()))
//!     .with_metadata("extraction_confidence", serde_json::Value::String("high".into()));
//!
//! let args = serde_json::json!({ "tier": "foundational", "min_confidence": "medium" });
//! assert!(filter.matches(&node, &args));
//! ```

use fabryk_graph::Node;

use crate::GraphNodeFilter;

// ============================================================================
// Types
// ============================================================================

/// How a single metadata filter compares values.
#[derive(Clone, Debug)]
enum FilterMode {
    /// Arg value must exactly match node metadata value.
    Exact,
    /// Node metadata value must be >= arg value in the given ordering.
    /// The ordering is lowest-first: `["low", "medium", "high"]`.
    Ordered(Vec<String>),
}

/// A single metadata filter specification.
#[derive(Clone, Debug)]
struct MetadataFilter {
    /// The key to look for in the MCP tool call `extra_args` JSON.
    arg_key: String,
    /// The key to check in `node.metadata`.
    metadata_key: String,
    /// How to compare the two values.
    mode: FilterMode,
}

/// A configurable metadata-based node filter.
///
/// Filters graph nodes by checking metadata fields against values from
/// MCP tool call arguments. Supports two filter modes:
///
/// - **Exact match**: arg value must equal node metadata value.
/// - **Ordered match**: node metadata value must be >= arg value in a defined ordering.
///
/// Multiple filters are combined with AND logic: all must pass for a node to match.
/// If no filters are configured, every node passes.
///
/// # Example
///
/// ```rust
/// use fabryk_mcp_graph::MetadataNodeFilter;
/// use fabryk_mcp_graph::GraphNodeFilter;
/// use fabryk_graph::Node;
///
/// let filter = MetadataNodeFilter::new()
///     .with_exact("tier", "tier")
///     .with_ordered("min_confidence", "extraction_confidence", &["low", "medium", "high"]);
///
/// let node = Node::new("test", "Test")
///     .with_metadata("tier", serde_json::Value::String("foundational".into()))
///     .with_metadata("extraction_confidence", serde_json::Value::String("high".into()));
///
/// let args = serde_json::json!({ "tier": "foundational", "min_confidence": "medium" });
/// assert!(filter.matches(&node, &args));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MetadataNodeFilter {
    filters: Vec<MetadataFilter>,
}

// ============================================================================
// Builder
// ============================================================================

impl MetadataNodeFilter {
    /// Creates a new empty filter that passes all nodes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an exact-match filter.
    ///
    /// When the MCP tool call arguments contain `arg_key`, the node's
    /// `metadata[metadata_key]` must exactly equal the argument value.
    /// If the argument is absent, this filter is skipped (passes).
    pub fn with_exact(mut self, arg_key: &str, metadata_key: &str) -> Self {
        self.filters.push(MetadataFilter {
            arg_key: arg_key.to_owned(),
            metadata_key: metadata_key.to_owned(),
            mode: FilterMode::Exact,
        });
        self
    }

    /// Adds an ordered filter with a defined level ordering (lowest first).
    ///
    /// When the MCP tool call arguments contain `arg_key`, the node's
    /// `metadata[metadata_key]` must be at or above the argument value
    /// in the given ordering. For example, with `ordering = ["low", "medium", "high"]`,
    /// a node with `"high"` passes a filter requesting `"medium"`.
    ///
    /// If the argument is absent, this filter is skipped (passes).
    pub fn with_ordered(mut self, arg_key: &str, metadata_key: &str, ordering: &[&str]) -> Self {
        self.filters.push(MetadataFilter {
            arg_key: arg_key.to_owned(),
            metadata_key: metadata_key.to_owned(),
            mode: FilterMode::Ordered(ordering.iter().map(|s| (*s).to_owned()).collect()),
        });
        self
    }
}

// ============================================================================
// GraphNodeFilter implementation
// ============================================================================

impl GraphNodeFilter for MetadataNodeFilter {
    fn matches(&self, node: &Node, extra_args: &serde_json::Value) -> bool {
        for filter in &self.filters {
            // Get the argument value; if absent, skip this filter.
            let arg_value = match extra_args.get(&filter.arg_key).and_then(|v| v.as_str()) {
                Some(v) => v,
                None => continue,
            };

            // Get the node metadata value; if absent, the node fails.
            let node_value = match node
                .metadata
                .get(&filter.metadata_key)
                .and_then(|v| v.as_str())
            {
                Some(v) => v,
                None => return false,
            };

            match &filter.mode {
                FilterMode::Exact => {
                    if arg_value != node_value {
                        return false;
                    }
                }
                FilterMode::Ordered(ordering) => {
                    let arg_pos = ordering.iter().position(|s| s == arg_value);
                    let node_pos = ordering.iter().position(|s| s == node_value);

                    match (arg_pos, node_pos) {
                        (Some(ap), Some(np)) => {
                            if np < ap {
                                return false;
                            }
                        }
                        // If either value is not in the ordering, fail.
                        _ => return false,
                    }
                }
            }
        }

        true
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Helper: create a node with tier and extraction_confidence metadata.
    fn make_node(tier: &str, confidence: &str) -> Node {
        Node::new("test", "Test")
            .with_metadata("tier", serde_json::Value::String(tier.into()))
            .with_metadata(
                "extraction_confidence",
                serde_json::Value::String(confidence.into()),
            )
    }

    // 1. Empty filter passes everything.
    #[test]
    fn test_empty_filter_passes_all() {
        let filter = MetadataNodeFilter::new();
        let node = Node::new("test", "Test");
        let args = json!({});
        assert!(filter.matches(&node, &args));
    }

    // 2. Exact match — passes when metadata matches arg.
    #[test]
    fn test_exact_match_passes_when_equal() {
        let filter = MetadataNodeFilter::new().with_exact("tier", "tier");
        let node = make_node("foundational", "high");
        let args = json!({ "tier": "foundational" });
        assert!(filter.matches(&node, &args));
    }

    // 3. Exact match — fails when metadata doesn't match.
    #[test]
    fn test_exact_match_fails_when_not_equal() {
        let filter = MetadataNodeFilter::new().with_exact("tier", "tier");
        let node = make_node("foundational", "high");
        let args = json!({ "tier": "advanced" });
        assert!(!filter.matches(&node, &args));
    }

    // 4. Exact match — passes when arg is absent (not filtering).
    #[test]
    fn test_exact_match_passes_when_arg_absent() {
        let filter = MetadataNodeFilter::new().with_exact("tier", "tier");
        let node = make_node("foundational", "high");
        let args = json!({});
        assert!(filter.matches(&node, &args));
    }

    // 5. Exact match — fails when node metadata is absent.
    #[test]
    fn test_exact_match_fails_when_metadata_absent() {
        let filter = MetadataNodeFilter::new().with_exact("tier", "tier");
        let node = Node::new("test", "Test"); // no metadata
        let args = json!({ "tier": "foundational" });
        assert!(!filter.matches(&node, &args));
    }

    // 6. Ordered filter — passes when node >= arg.
    #[test]
    fn test_ordered_passes_when_node_gte_arg() {
        let filter = MetadataNodeFilter::new().with_ordered(
            "min_confidence",
            "extraction_confidence",
            &["low", "medium", "high"],
        );
        let node = make_node("foundational", "high");
        let args = json!({ "min_confidence": "medium" });
        assert!(filter.matches(&node, &args));

        // Equal case.
        let args_eq = json!({ "min_confidence": "high" });
        assert!(filter.matches(&node, &args_eq));
    }

    // 7. Ordered filter — fails when node < arg.
    #[test]
    fn test_ordered_fails_when_node_lt_arg() {
        let filter = MetadataNodeFilter::new().with_ordered(
            "min_confidence",
            "extraction_confidence",
            &["low", "medium", "high"],
        );
        let node = make_node("foundational", "low");
        let args = json!({ "min_confidence": "high" });
        assert!(!filter.matches(&node, &args));
    }

    // 8. Ordered filter — passes when arg absent.
    #[test]
    fn test_ordered_passes_when_arg_absent() {
        let filter = MetadataNodeFilter::new().with_ordered(
            "min_confidence",
            "extraction_confidence",
            &["low", "medium", "high"],
        );
        let node = make_node("foundational", "low");
        let args = json!({});
        assert!(filter.matches(&node, &args));
    }

    // 9. Multiple filters with AND logic.
    #[test]
    fn test_multiple_filters_and_logic() {
        let filter = MetadataNodeFilter::new()
            .with_exact("tier", "tier")
            .with_ordered(
                "min_confidence",
                "extraction_confidence",
                &["low", "medium", "high"],
            );

        let node = make_node("foundational", "high");

        // Both pass.
        let args_both = json!({ "tier": "foundational", "min_confidence": "medium" });
        assert!(filter.matches(&node, &args_both));

        // Exact fails, ordered passes.
        let args_exact_fail = json!({ "tier": "advanced", "min_confidence": "medium" });
        assert!(!filter.matches(&node, &args_exact_fail));

        // Exact passes, ordered fails.
        let args_ordered_fail = json!({ "tier": "foundational", "min_confidence": "high" });
        // node is "high", arg is "high" => equal => passes
        assert!(filter.matches(&node, &args_ordered_fail));

        // Actually make ordered fail.
        let low_node = make_node("foundational", "low");
        let args_ordered_fail2 = json!({ "tier": "foundational", "min_confidence": "high" });
        assert!(!filter.matches(&low_node, &args_ordered_fail2));
    }

    // 10. Ordered filter with unknown values (not in ordering list).
    #[test]
    fn test_ordered_unknown_values_fail() {
        let filter = MetadataNodeFilter::new().with_ordered(
            "min_confidence",
            "extraction_confidence",
            &["low", "medium", "high"],
        );

        // Unknown arg value.
        let node = make_node("foundational", "high");
        let args = json!({ "min_confidence": "ultra" });
        assert!(!filter.matches(&node, &args));

        // Unknown node value.
        let node_unknown = Node::new("test", "Test").with_metadata(
            "extraction_confidence",
            serde_json::Value::String("unknown".into()),
        );
        let args2 = json!({ "min_confidence": "low" });
        assert!(!filter.matches(&node_unknown, &args2));

        // Both unknown.
        let args3 = json!({ "min_confidence": "ultra" });
        assert!(!filter.matches(&node_unknown, &args3));
    }
}
