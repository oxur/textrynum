# Slice02 latest-dependencies ledger

Seven opening rows; all open, no implementation/acceptance claimed.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DEP2-01 | All external declaration/tooling surfaces inventoried and selected latest stable with explicit real constraints | Whole42-manifest/64-package inventory, registry/upstream versions, coherent generated lock; internal0.5.0 preserved | correctness-grade | DEP-01 | open | | |
| DEP2-02 | Existing persistent data remains usable across the selected dependency upgrades | Actual pinned old writer/new reader on immutable-owned fixture copies for Redb, Lance/vector metadata and Tantivy; current roundtrips | correctness-grade | DEP-01 | open | | |
| DEP2-03 | SDK/API migration preserves supported behavior and documents approved public changes | Source review; real MCP wire/host behavior, vector distances, Avro decode, existing real JWT/storage tests; downstream notes | correctness-grade | DEP-01 | open | | |
| DEP2-04 | Declared Rust1.94 is true for actual candidate features/targets | Actual locked1.94.0 all-feature all-target workspace check plus default/17 isolated feature selections; MSRV CI job | correctness-grade | CI-02 | open | | |
| DEP2-05 | Latest-stable local workspace/package/provider gates pass meaningfully | 1.100 format/freshness/debug+release build/strict-clippy/tests, packaging and actual ignored Fastembed checks/limits | correctness-grade | CI-02 | open | | |
| DEP2-06 | Actual upgraded candidate CI passes complete required jobs | Exact SHA, every debug/release/MSRV job and step, full logs; unchanged gate strength/no publication | correctness-grade | CI-02 | open | | |
| DEP2-07 | Independent slice review, full row/artifact accounting and retrospective bubble up | CC canonical close, CDC reproduced checks/review/corrections, accepted ledger, retrospective/arc-plan update | correctness-grade | REVIEW-01 | open | | |
