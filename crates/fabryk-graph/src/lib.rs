//! Knowledge graph infrastructure for Fabryk.
//!
//! This crate provides graph storage, traversal algorithms, and
//! persistence using petgraph and optional rkyv caching.
//!
//! # Features
//!
//! - `graph-rkyv-cache`: Enable rkyv-based graph persistence with
//!   content-hash cache validation
//! - `test-utils`: Export mock types for testing in downstream crates
//!
//! # Key Abstractions
//!
//! - `GraphExtractor` trait: Domain implementations provide this to
//!   extract nodes and edges from content files
//! - `Relationship` enum: Common relationship types with `Custom(String)`
//!   for domain-specific relationships
//! - `NodeType` enum: Distinguishes domain vs user-query nodes
//! - `GraphData`: Core graph structure with runtime mutation support

pub mod algorithms;
pub mod builder;
pub mod concept_card_extractor;
pub mod extractor;
pub mod persistence;
pub mod query;
pub mod stats;
pub mod types;
pub mod validation;

// Re-exports — algorithms
pub use algorithms::{
    CentralityScore, LearningPathResult, LearningStep, NeighborhoodResult, PathResult,
    PrerequisitesResult, bridge_between_categories, calculate_centrality, concept_sources,
    concept_variants, dependents, find_bridges, get_related, learning_path, neighborhood,
    prerequisites_sorted, shortest_path, source_coverage,
};

// Re-exports — builder
pub use builder::{BuildError, BuildStats, ErrorHandling, GraphBuilder, ManualEdge};

// Re-exports — concept card extractor
pub use concept_card_extractor::{
    ConceptCardEdgeData, ConceptCardGraphExtractor, ConceptCardNodeData,
};

// Re-exports — extractor
pub use extractor::GraphExtractor;

// Re-exports — persistence
pub use persistence::{
    GraphMetadata, SerializableGraph, is_cache_fresh, load_graph, load_graph_from_str, save_graph,
};

// Re-exports — query
pub use query::{
    CategoryCount, EdgeInfo, GraphInfoResponse, NeighborInfo, NeighborhoodResponse, NodeDetail,
    NodeEdgesResult, NodeSummary, PathResponse, PathStep, PrerequisiteInfo, PrerequisitesResponse,
    RelatedConceptsResponse, RelatedGroup, RelationshipCount, get_node_detail, get_node_edges,
};

// Re-exports — stats
pub use stats::{DegreeDirection, GraphStats, compute_stats, quick_summary, top_nodes_by_degree};

// Re-exports — types
pub use types::{Edge, EdgeOrigin, GraphData, LoadedGraph, Node, NodeType, Relationship};

// Re-exports — validation
pub use validation::{ValidationIssue, ValidationResult, is_valid, validate_graph};

#[cfg(any(test, feature = "test-utils"))]
pub use extractor::mock::{MockEdgeData, MockExtractor, MockNodeData};

#[cfg(feature = "graph-rkyv-cache")]
pub use persistence::rkyv_cache;
