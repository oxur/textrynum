# Slice01 baseline ledger

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BASE-01 | Minimal compatible dependency fixes replace the observed Redis/ethnum blockers without broad sweep | Cargo manifest/lock diff; registry/upstream source; targeted resolution | correctness-grade | CI-01 | open | | |
| BASE-02 | Fresh real debug/release CI passes complete unchanged gates at baseline candidate SHA | GitHub run jobs/steps and locked build/clippy/test | correctness-grade | CI-01 | open | | |
| BASE-03 | Scope, intake, independent review and slice bubble-up/retrospective complete | CC record, CDC reproduced checks and closing records | correctness-grade | REVIEW-01 | open | | |
