# CI/dependency prerequisite ledger

CI-01 is independently accepted; DEP-01, CI-02 and final REVIEW-01 remain open. No whole-arc acceptance is claimed.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CI-01 | Fresh CI baseline passes on pinned sanitized source | Actual complete debug/release CI and preserved failure/repair evidence | correctness-grade | Operator 2026-11-21 | done | [Slice01 independent acceptance](slice01-green-baseline/cdc-verification.md), [run44957414892](https://github.com/oxur/textrynum/actions/runs/44957414892), source edc8db69 | Both complete Linux jobs passed; minimal two-file repair; nonmerged task branch. |
| DEP-01 | All external dependency surfaces inventoried and updated to latest stable, with explicit upstream constraints | Current authoritative version evidence, manifests, lockfile and migration diffs | correctness-grade | Operator 2026-11-21 | open | | |
| CI-02 | Upgraded candidate passes meaningful local gates and fresh CI | Current feature/target/package checks and CI run identities | correctness-grade | Operator 2026-11-21 | open | | |
| REVIEW-01 | Independent review, composition check and retrospective complete | CDC review of CC's actual candidate, dispositions and closure records | correctness-grade | Framework and Operator | open | | |
