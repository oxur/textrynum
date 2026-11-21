# Ledger

All rows are open. Planning and historical records are not acceptance evidence.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 01 | Private-name cleanup covers reachable Git history, filenames, live source and all affected configured remote refs | Sanitized object/path scan, remote ref read-back and fresh fetch verification | correctness-grade | Operator decision 2026-11-21 | open | [Privacy record](artifacts/privacy-rewrite-record.md), author-attested | Controlled refs and local objects cleaned; hosted purge remains open but Duncan explicitly made it nonblocking on 2026-11-21. No deletion/recreation authorized. |
| 09 | Fresh CI baseline passes before the audit | Complete current CI run on pinned sanitized source | correctness-grade | Operator 2026-11-21 | open | | |
| 10 | All dependencies updated to latest stable and checks/CI green again before audit | Version inventory, explicit compatibility constraints, lockfile and current checks | correctness-grade | Operator 2026-11-21 | open | | |
| 11 | ChatGPT UX delivers natural, efficient discovery and completed grounded tasks | Real ChatGPT trials with task success, discovery/request/payload/useful-information evidence | correctness-grade | Operator 2026-11-21 | open | | |
| 02 | Full audit is independently accepted across all workspace members and scales | Audit index, per-language reports, coverage accounting and independent review | correctness-grade | Operator decision 2026-11-21 | open | | |
| 03 | All audit findings repaired or explicitly dispositioned by Duncan before feature work | Finding-to-repair traceability, discriminating checks and independent composition review | correctness-grade | Operator decision 2026-11-21 | open | | |
| 04 | Shared server assembly, startup and backend state provide a consistent usable foundation | A real downstream migration/new-server demonstration against packaged APIs | correctness-grade | Operator decision 2026-11-21 | open | | |
| 05 | Duplicate infrastructure and competing policy implementations consolidated where justified | Audit findings, public API/behavior preservation and cross-crate tests | correctness-grade | Operator decision 2026-11-21 | open | | |
| 06 | Tentative vector improvements receive an explicit release-scope decision | Recorded include/defer/cancel decisions after audit | correctness-grade | Operator decision 2026-11-21 | open | | |
| 07 | 0.5.0 packages, feature/target combinations and release documentation match actual behavior | Repository-native build/test/lint/package checks plus downstream integration evidence | correctness-grade | Operator decision 2026-11-21 | open | | |
| 08 | Operator accepts 0.5.0 release readiness | Explicit release acceptance; publishing remains a separate authorized action | correctness-grade | Operator decision 2026-11-21 | open | | |
