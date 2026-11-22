# Slice20 close — record grouping and assembly key contracts

**Assignment:** `slice20-record-key-contracts/cc-prompt.md`. **Source:** `719c58f245fe4a9115c7c0391d3e268c99157e2f`, clean/read-only. **Current author assessment:** CC proposes KEY-01–05 met after scoped commit checks; independent CDC review is pending. The assessment is diagnosis-only and does not choose a new key representation or repeated-join feature policy.

## Exact delivered paths

1. `project05-discoverability/artifacts/2026.11.22-record-key-contracts.md`
2. `project05-discoverability/artifacts/2026.11.22-audit-results-rust.md`
3. `project05-discoverability/artifacts/2026.11.21-audit-index.md`
4. `project05-discoverability/arc01-foundation-audit/slice20-record-key-contracts/ledger.md`
5. `project05-discoverability/arc01-foundation-audit/slice20-record-key-contracts/closing-report.md`

## Result and limits

- Retained same-session complete intake of aggregate.rs, assemble.rs and ecl-stages/lib.rs (1,334 lines including tests); freshly read Record/PipelineItem/Stage companion units and scoped StageRegistry/caller-reference evidence. Source-cited readback preceded edits.
- CC proposes one Medium aggregate finding, FA-RS-044, for two valid composite string tuples that map to the same unescaped `|`-joined key and therefore aggregate together. Full source, severity, owner, fix direction, ID compatibility, and later oracle are recorded once in the cumulative Rust report.
- Assemble's per-stream index replacement for repeated definitions with different foreign keys is retained as Q20-01, ungraded. The public config is deserializable, but current tests/docs do not establish same-stream repetition as supported.
- No product, record-processing program, fixture, test, Rust/Cargo/build, RPC/service/provider, SEARCH, source/CI repair, pipeline integration, or external action occurred. Source stayed at719.
- Findings001–043 and Q19-01–04, original coverage38/31/291 and49cells7/7/35, SEARCH, full ECL/pass-scale and final-arc gates remain unchanged. Slice20 refinement budget remains0/5.

## Ledger walk

| Criterion | CC disposition | Evidence and remaining gate |
| --- | --- | --- |
| KEY-01 — intake/readback | CC proposes met | Complete 1,334-line retained primary intake, fresh traits unit, cited pre-edit readback; CDC review pending. |
| KEY-02 — encoding/admissibility | CC proposes met | Exact aggregate string encoding and assembly identity map/validation boundary; CDC review pending. |
| KEY-03 — manual cases/controls | CC proposes met | Valid aggregate collision, both join-definition orders, and required controls; hand-derived only. |
| KEY-04 — calibration | CC proposes met | FA-RS-044 proposed; Q20-01 kept ungraded because repetition support is indeterminate. CDC reviews both. |
| KEY-05 — preservation/commit | CC proposes met after actual scoped checks | Exact paths, byte prefixes, protected fingerprints, source state, privacy counts and actual commit authors recorded at handback; CDC review pending. |

## Artifact inventory

Produced `project05-discoverability/artifacts/2026.11.22-record-key-contracts.md`; appended one FA-RS-044 diagnosis to the cumulative Rust report and one bounded index note. The slice ledger and close are the other two owned outputs. No executable artifact was produced.

## Bubble-up to the arc

1. **Contribution:** source-based distinction between aggregate tuple identity and its current delimiter string encoding, plus the separate assemble stream-index identity and a bounded repeated-stream question.
2. **Arc implications:** the aggregate owner may choose an unambiguous encoding or an explicit enforced separator constraint, with generated item-ID compatibility considered. The assemble owner should clarify whether repeated stream/different foreign-key joins are supported or constrained. This assessment selects neither representation nor feature policy and does not revive held project06 work.
3. **Silent-drop check:** all five KEY criteria, exact source population, Record/Stage contracts, public admissibility, tuple collision arithmetic, both join orders, aliases, matching/missing/collect/empty controls, actual stage exits, Q20-01, IDs/compatibility, protected populations, and unchanged prior gates are recorded. No unrelated numeric/missing-key or count policy is graded.

## Independent review boundary

This is CC author attestation only. CDC independently determines finding acceptance, question calibration, and criterion status. No mutable command or child process remains. I return exactly the five listed paths and planning index to CDC for independent review.
