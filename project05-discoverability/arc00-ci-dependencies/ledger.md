# CI/dependency prerequisite ledger

All rows are open; no implementation or acceptance is claimed.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CI-01 | Fresh CI baseline passes on pinned sanitized source | Actual complete debug/release CI and preserved failure/repair evidence | correctness-grade | Operator 2026-11-21 | open | [Run 44943360336](https://github.com/oxur/textrynum/actions/runs/44943360336), source `856af875`; both jobs failed | Debug: Redis freshness check. Release: ethnum E0512. Repairs and independent acceptance pending. |
| DEP-01 | All external dependency surfaces inventoried and updated to latest stable, with explicit upstream constraints | Current authoritative version evidence, manifests, lockfile and migration diffs | correctness-grade | Operator 2026-11-21 | open | | |
| CI-02 | Upgraded candidate passes meaningful local gates and fresh CI | Current feature/target/package checks and CI run identities | correctness-grade | Operator 2026-11-21 | open | | |
| REVIEW-01 | Independent review, composition check and retrospective complete | CDC review of CC's actual candidate, dispositions and closure records | correctness-grade | Framework and Operator | open | | |
