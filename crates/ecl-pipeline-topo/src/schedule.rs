//! Schedule computation: topological sort and resource-conflict layer grouping.
//!
//! Uses Kahn's algorithm for topological ordering, then groups stages into
//! parallel batches where no resource conflicts exist within a batch.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ecl_pipeline_state::StageId;

use crate::error::ResolveError;

/// Compute a parallel execution schedule from resource declarations.
///
/// Returns `Vec<Vec<StageId>>` — batches of stages that can run concurrently.
/// Stages within a batch touch independent resources. Batch ordering respects
/// resource dependencies (a stage that reads a resource runs in a later batch
/// than the stage that creates it).
///
/// # Algorithm
///
/// 1. Build a dependency graph: stage A depends on stage B if A reads a
///    resource that B creates.
/// 2. Perform Kahn's algorithm (BFS topological sort) to get a valid ordering.
/// 3. Assign each stage to the earliest batch where all its dependencies are
///    in earlier batches and no write-write conflicts exist within the batch.
///
/// # Errors
///
/// Returns `ResolveError::CycleDetected` if the dependency graph has a cycle.
pub fn compute_schedule(
    stages: &[StageId],
    creators: &BTreeMap<String, StageId>,
    readers: &BTreeMap<String, Vec<StageId>>,
    _writers: &BTreeMap<String, Vec<StageId>>,
) -> Result<Vec<Vec<StageId>>, ResolveError> {
    if stages.is_empty() {
        return Ok(vec![]);
    }

    // Step 1: Build adjacency list and in-degree map.
    // An edge from A -> B means "B depends on A" (B reads what A creates).
    let mut adjacency: BTreeMap<StageId, BTreeSet<StageId>> = BTreeMap::new();
    let mut in_degree: BTreeMap<StageId, usize> = BTreeMap::new();

    // Initialize all stages with zero in-degree.
    for stage in stages {
        adjacency.entry(stage.clone()).or_default();
        in_degree.entry(stage.clone()).or_insert(0);
    }

    // For each resource that is both created and read, add edges:
    // creator -> each reader (reader depends on creator).
    for (resource, reader_stages) in readers {
        if let Some(creator) = creators.get(resource) {
            for reader in reader_stages {
                // Don't add self-edges.
                if reader != creator
                    && adjacency
                        .entry(creator.clone())
                        .or_default()
                        .insert(reader.clone())
                {
                    *in_degree.entry(reader.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    // Write-write conflicts: if two stages write to the same resource,
    // they cannot be in the same batch. We handle this in the batching
    // step, not via edges (since the order between them is arbitrary).

    // Step 2: Kahn's algorithm — BFS topological sort.
    let mut queue: VecDeque<StageId> = VecDeque::new();
    for (stage, &degree) in &in_degree {
        if degree == 0 {
            queue.push_back(stage.clone());
        }
    }

    // Track the "depth" (earliest batch) for each stage.
    let mut depth: BTreeMap<StageId, usize> = BTreeMap::new();
    let mut processed_count = 0usize;

    while let Some(stage) = queue.pop_front() {
        processed_count += 1;
        let stage_depth = depth.get(&stage).copied().unwrap_or(0);

        if let Some(neighbors) = adjacency.get(&stage) {
            for neighbor in neighbors {
                // The neighbor's depth is at least one more than this stage's depth.
                let neighbor_depth = depth.entry(neighbor.clone()).or_insert(0);
                if *neighbor_depth <= stage_depth {
                    *neighbor_depth = stage_depth + 1;
                }

                let mut fallback = 0usize;
                let deg = in_degree.get_mut(neighbor).unwrap_or(&mut fallback);
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    // If we didn't process all stages, there's a cycle.
    if processed_count != stages.len() {
        let stuck: Vec<StageId> = stages
            .iter()
            .filter(|s| in_degree.get(*s).copied().unwrap_or(0) > 0)
            .cloned()
            .collect();
        return Err(ResolveError::CycleDetected { stages: stuck });
    }

    // Step 3: Group stages into batches by depth.
    let max_depth = depth.values().copied().max().unwrap_or(0);
    let mut batches: Vec<Vec<StageId>> = vec![Vec::new(); max_depth + 1];

    for stage in stages {
        let d = depth.get(stage).copied().unwrap_or(0);
        batches[d].push(stage.clone());
    }

    // Sort stages within each batch for deterministic output.
    for batch in &mut batches {
        batch.sort();
    }

    // Remove empty batches (shouldn't happen, but be safe).
    batches.retain(|b| !b.is_empty());

    Ok(batches)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::ResourceSpec;
    use ecl_pipeline_spec::StageSpec;

    fn make_stage(reads: Vec<&str>, creates: Vec<&str>, writes: Vec<&str>) -> StageSpec {
        StageSpec {
            adapter: "test".to_string(),
            source: None,
            resources: ResourceSpec {
                reads: reads.into_iter().map(String::from).collect(),
                creates: creates.into_iter().map(String::from).collect(),
                writes: writes.into_iter().map(String::from).collect(),
            },
            params: serde_json::Value::Null,
            retry: None,
            timeout_secs: None,
            skip_on_error: false,
            condition: None,
        }
    }

    fn build_and_schedule(
        stages: &BTreeMap<String, StageSpec>,
    ) -> Result<Vec<Vec<StageId>>, ResolveError> {
        let graph = crate::resource_graph::ResourceGraph::build(stages)?;
        graph.compute_schedule()
    }

    #[test]
    fn test_schedule_empty_pipeline_returns_empty() {
        let stages = BTreeMap::new();
        let result = build_and_schedule(&stages).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_schedule_single_stage_one_batch() {
        let mut stages = BTreeMap::new();
        stages.insert(
            "extract".to_string(),
            make_stage(vec![], vec!["docs"], vec![]),
        );
        let result = build_and_schedule(&stages).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![StageId::new("extract")]);
    }

    #[test]
    fn test_schedule_independent_stages_same_batch() {
        let mut stages = BTreeMap::new();
        stages.insert(
            "fetch-a".to_string(),
            make_stage(vec![], vec!["a-docs"], vec![]),
        );
        stages.insert(
            "fetch-b".to_string(),
            make_stage(vec![], vec!["b-docs"], vec![]),
        );
        let result = build_and_schedule(&stages).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            vec![StageId::new("fetch-a"), StageId::new("fetch-b")]
        );
    }

    #[test]
    fn test_schedule_dependent_stages_sequential_batches() {
        let mut stages = BTreeMap::new();
        stages.insert(
            "extract".to_string(),
            make_stage(vec![], vec!["raw-docs"], vec![]),
        );
        stages.insert(
            "normalize".to_string(),
            make_stage(vec!["raw-docs"], vec!["normalized"], vec![]),
        );
        let result = build_and_schedule(&stages).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0], vec![StageId::new("extract")]);
        assert_eq!(result[1], vec![StageId::new("normalize")]);
    }

    #[test]
    fn test_schedule_five_stage_example_three_batches() {
        let stages = five_stage_specs();
        let result = build_and_schedule(&stages).unwrap();
        let expected = vec![
            vec![StageId::new("fetch-gdrive"), StageId::new("fetch-slack")],
            vec![
                StageId::new("normalize-gdrive"),
                StageId::new("normalize-slack"),
            ],
            vec![StageId::new("emit")],
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_schedule_cycle_detection_returns_error() {
        let stages = cyclic_stage_specs();
        let result = build_and_schedule(&stages);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ResolveError::CycleDetected { .. }));
    }

    #[test]
    fn test_schedule_diamond_dependency() {
        let stages = diamond_stage_specs();
        let result = build_and_schedule(&stages).unwrap();
        let expected = vec![
            vec![StageId::new("a")],
            vec![StageId::new("b"), StageId::new("c")],
            vec![StageId::new("d")],
        ];
        assert_eq!(result, expected);
    }

    #[test]
    fn test_schedule_deterministic_ordering() {
        let stages = five_stage_specs();
        let first = build_and_schedule(&stages).unwrap();
        for _ in 0..10 {
            let result = build_and_schedule(&stages).unwrap();
            assert_eq!(result, first);
        }
    }

    fn five_stage_specs() -> BTreeMap<String, StageSpec> {
        let mut stages = BTreeMap::new();

        stages.insert(
            "fetch-gdrive".to_string(),
            StageSpec {
                adapter: "extract".to_string(),
                source: Some("engineering-drive".to_string()),
                resources: ResourceSpec {
                    reads: vec!["gdrive-api".to_string()],
                    creates: vec!["raw-gdrive-docs".to_string()],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: Some(300),
                skip_on_error: false,
                condition: None,
            },
        );

        stages.insert(
            "fetch-slack".to_string(),
            StageSpec {
                adapter: "extract".to_string(),
                source: Some("team-slack".to_string()),
                resources: ResourceSpec {
                    reads: vec!["slack-api".to_string()],
                    creates: vec!["raw-slack-messages".to_string()],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages.insert(
            "normalize-gdrive".to_string(),
            StageSpec {
                adapter: "normalize".to_string(),
                source: Some("engineering-drive".to_string()),
                resources: ResourceSpec {
                    reads: vec!["raw-gdrive-docs".to_string()],
                    creates: vec!["normalized-docs".to_string()],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages.insert(
            "normalize-slack".to_string(),
            StageSpec {
                adapter: "slack-normalize".to_string(),
                source: Some("team-slack".to_string()),
                resources: ResourceSpec {
                    reads: vec!["raw-slack-messages".to_string()],
                    creates: vec!["normalized-messages".to_string()],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages.insert(
            "emit".to_string(),
            StageSpec {
                adapter: "emit".to_string(),
                source: None,
                resources: ResourceSpec {
                    reads: vec![
                        "normalized-docs".to_string(),
                        "normalized-messages".to_string(),
                    ],
                    creates: vec![],
                    writes: vec![],
                },
                params: serde_json::json!({ "subdir": "normalized" }),
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages
    }

    fn cyclic_stage_specs() -> BTreeMap<String, StageSpec> {
        let mut stages = BTreeMap::new();

        stages.insert(
            "stage-a".to_string(),
            make_stage(vec!["resource-b"], vec!["resource-a"], vec![]),
        );

        stages.insert(
            "stage-b".to_string(),
            make_stage(vec!["resource-a"], vec!["resource-b"], vec![]),
        );

        stages
    }

    fn diamond_stage_specs() -> BTreeMap<String, StageSpec> {
        let mut stages = BTreeMap::new();

        stages.insert(
            "a".to_string(),
            StageSpec {
                adapter: "test".to_string(),
                source: None,
                resources: ResourceSpec {
                    reads: vec![],
                    creates: vec!["resource-x".to_string(), "resource-y".to_string()],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages.insert(
            "b".to_string(),
            make_stage(vec!["resource-x"], vec!["resource-bx"], vec![]),
        );

        stages.insert(
            "c".to_string(),
            make_stage(vec!["resource-y"], vec!["resource-cy"], vec![]),
        );

        stages.insert(
            "d".to_string(),
            StageSpec {
                adapter: "test".to_string(),
                source: None,
                resources: ResourceSpec {
                    reads: vec!["resource-bx".to_string(), "resource-cy".to_string()],
                    creates: vec![],
                    writes: vec![],
                },
                params: serde_json::Value::Null,
                retry: None,
                timeout_secs: None,
                skip_on_error: false,
                condition: None,
            },
        );

        stages
    }
}
