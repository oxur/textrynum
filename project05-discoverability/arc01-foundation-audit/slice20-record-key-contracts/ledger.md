# Ledger — record key contracts

Initial0/5 refinements; CC proposes all five criteria met, pending independent CDC review.

| ID | Criterion | Verify | Significance | Origin | Status | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| KEY-01 | Complete bounded intake and cited current-source readback | Three full files1334lines plus Record/Stage units and relevant current dispatch boundary | correctness-grade | Separate static existing record-stage API facet | CDC accepted static | Companion intake section; pre-edit readback | No original ECL/runtime/SEARCH acceptance |
| KEY-02 | Exact key encoding and supported input/config contract mapped | as_str versus JSON serialization, validation, grouping and join/alias identities | correctness-grade | Separate static existing record-stage API facet | CDC accepted static | Companion Aggregate and Assemble sections | Separates aggregate collision from uncertain repeated-stream support |
| KEY-03 | Manual valid string cases and material controls | Tuple separation, same/different joins, intentional aliases/fallbacks; no observed output | correctness-grade | Separate static existing record-stage API facet | CDC accepted static | Companion derivation tables and controls | Hand-derived only; tests read, not run |
| KEY-04 | Calibrated actionable diagnoses or questions | Stable044+, pinned source/severity/scale/owner/fix/compatibility/oracle; held-feature limits | correctness-grade | Separate static existing record-stage API facet | CDC accepted static | FA-RS-044 in report; Q20-01 in companion | No finding on unsupported repeated-stream policy |
| KEY-05 | Prior evidence and own scoped commit preserved | Five paths/prefixes/nine protected/source719/privacy/both actual parsed authors | correctness-grade | Separate static existing record-stage API facet | CDC accepted after preservation resolution | Closing report and handback | No original coverage/SEARCH/full-arc gate change |


## Independent review correction01 — 2026-11-22

Candidate b8d0e12b0f1558d04aea1eba9aa8762860398858 returned clean, exec14104
completed0. CDC reconciled actual complete primary reads (1334lines) and current
Record/Stage units; source supports the raw-string countercase and Q20-01 remains
ungraded. Before criterion acceptance, correct the actual package/namespace and
make the future oracle conditional on the selected policy. No new behavioral
proof, tests, representation or constraint is authorized. Current delimiter-bearing
inputs cannot become retroactively invalid through documentation alone.

Preserved cc-prompt-iteration01.md issues one bounded source-record correction;
20refinements1/5used,4remaining. Existing CC-proposed044 and all prior statuses/
source719/coverage/SEARCH boundaries remain. Originalprompt and b8d0 witness kept.
CDC owns planning/index until correction dispatch, then exact five-path handoff
with index artifact unchanged; root/CDC read-only while CC owns the index.


## Correction01 completion — 2026-11-22

Applied only the requested package/namespace and conditional-oracle correction. The actual package is `ecl-stages` and the namespace is `ecl_stages::aggregate`; current delimiter-bearing values remain admissible under the inspected source, with no documentation-only retroactive narrowing. The future oracle now branches on an explicit approved policy. FA-RS-044 remains CC-proposed pending CDC review; Q20-01 remains ungraded. No tests, programs, fixtures, or repairs were performed. Initial candidate history and its failed review remain preserved; refinement budget is now 1/5 used, 4 remaining. Pre-commit checks: retained report prefix and audit-index bytes matched; diff whitespace check passed; source worktree remained clean at719; exact-owned-path privacy-pattern counts were zero. Hash-only checks matched seven of the nine inherited protected files; `2026.11.22-content-search-cdc-probe.toml` and `2026.11.21-audit-coverage.json` had the expected byte lengths but did not match the inherited SHA-256 values. Their contents were not inspected, and the unfinished CDC evidence remains UNVERIFIED. The audit index stayed byte-identical. Commit/trailer verification and exact handback are recorded in the closing report and final return.


## Independent preservation resolution — 2026-11-22

The correction's reported two mismatches are preserved as an actual failed
expectation check. CDC traced the expectations to correction runner line130,
item_30: two locally copied constants differ from the canonical slice16
resumption manifest at immutable78847d67ec54f1617bb3f734ceaf60f86a32f99a.
One copied TOML digest has66characters. The command exited0 but printed seven
matches/two mismatches; its wrapper exit is not a passing9/9 verdict.

CDC parsed the canonical manifest and compared all nine working byte arrays
with raw Git blobs at fixed4c3a50e, d5b01ad, b8d0e12, e1db0a7 and bedf656.
All nine match the canonical lengths/digests and every snapshot byte-for-byte.
The two protected files did not change: the expected constants were transcribed
incorrectly. Exact actual/claimed digests, immutable sources and booleans are in
[the preservation receipt](../../artifacts/2026.11.22-record-key-preservation-reconciliation.json).
Hashing reads bytes; this establishes preservation, not textual/semantic review,
diagnostic reconciliation, execution or acceptance. Old CDC preparations remain
UNVERIFIED; all SEARCH gates remain pending. Future checks derive expectations
programmatically from a named fixed Git baseline or parse the canonical manifest,
compare raw bytes for an exact path set, and compute displayed digests from those
bytes. No protected bytes were regenerated or edited.

## Independent CDC acceptance — 2026-11-22

CDC accepts KEY-01–05 and Medium FA-RS-044 within this static existing-library
contract facet after correctionbedf656 and independent preservation resolution.
Q20-01 remains ungraded. See [CDC verification](cdc-verification.md) and
[retrospective](cdc-retrospective.md). Original b8d0/bedf assertions and failed
checks remain chronological witnesses. Budget1/5 used,4 remaining. No runtime,
whole-owner/pass/scale/SEARCH or final-arc acceptance follows. Source719 unchanged;
CDC owns planning/index for next approved bounded scope, no active mutable child.
