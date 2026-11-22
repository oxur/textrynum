# Slice21 close — relationship names, parsers, and typed consumers

**Assignment:** `slice21-relationship-parser-contracts/cc-prompt.md`. **Planning readiness:** `4d8e233fc19cd5f8151648229915d4b3e6ab7aa0`. **Source:** `719c58f245fe4a9115c7c0391d3e268c99157e2f`, clean/read-only. **Current author assessment:** CC proposes REL-01–05 met after scoped commit checks; independent CDC review is pending. This is a bounded static API consistency review, not a behavioral or client-acceptance claim.

## Exact delivered paths

1. `project05-discoverability/artifacts/2026.11.22-relationship-parser-contracts.md`
2. `project05-discoverability/artifacts/2026.11.22-audit-results-rust.md`
3. `project05-discoverability/artifacts/2026.11.21-audit-index.md`
4. `project05-discoverability/arc01-foundation-audit/slice21-relationship-parser-contracts/ledger.md`
5. `project05-discoverability/arc01-foundation-audit/slice21-relationship-parser-contracts/closing-report.md`

## Result and limits

- Reused same-session slice14/17 complete reads totaling 5,276 lines for `types.rs`, `algorithms.rs`, MCP graph `tools.rs`, and graph `builder.rs`; read `concept_card_extractor.rs` and `query.rs` completely (1,362 lines, including all tests), plus refreshed affected complete logical units. Receipt paths and the actual slice17 digest are in the companion record.
- CC proposes one Medium finding, FA-RS-045: the MCP relationship filter converts canonical `contrasts_with` to `Custom("contrasts_with")`, which fails exact typed comparison with a stock `ContrastsWith` edge. This is manually derived from source, not an observed MCP result.
- ManualEdge support for `SameAs`/`Cites` and unknown custom-name casing remain ungraded Q21-01 and Q21-02. Legacy aliases and custom/persisted identity remain compatibility controls; no parser policy or migration is selected.
- No tests, program, fixture, Rust/Cargo/build, MCP call, graph execution, service/provider/RPC, SEARCH operation, source/CI edit, repair, migration, or external action occurred. Source stayed at719.
- Findings001–044, Q19/Q20 questions, original coverage38/31/291 and49cells7/7/35, SEARCH, full-pass, and final-arc gates remain unchanged. Slice21 refinement budget is0/5.

## Ledger walk

| Criterion | CC disposition | Evidence and remaining gate |
| --- | --- | --- |
| REL-01 — intake/readback | CC proposes met | Retained complete same-session 14/17 reads plus fresh complete extractor/query and cited pre-edit readback; CDC review pending. |
| REL-02 — names/parsers/consumers | CC proposes met | Eleven-variant map, both private parsers, exact typed consumers, stock producer and canonical response name; CDC review pending. |
| REL-03 — manual cases/controls | CC proposes met | Manual parser/default-weight derivation, aliases, endpoint/dedup/cache boundaries and stock filter controls; static only. |
| REL-04 — calibration | CC proposes met | FA-RS-045 proposed; Q21-01/02 remain ungraded; CDC reviews finding/question calibration. |
| REL-05 — preservation/commit | CC proposes met after scoped checks | Fixed-baseline raw bytes, nine protected files, source state, privacy counts, exact paths and actual parsed authors checked; CDC review pending. |

## Artifact inventory

Produced `project05-discoverability/artifacts/2026.11.22-relationship-parser-contracts.md`; appended one full FA-RS-045 diagnosis to the cumulative Rust report and one concise index note. The slice ledger and close are the other owned outputs. No executable artifact was produced.

## Bubble-up to the arc

1. **Contribution:** a typed stock relationship and a canonical string response expose one cross-boundary name that the MCP filter parser currently maps to a different typed variant.
2. **Arc implications:** the MCP and graph API owners should decide the supported relationship-name contract across filters and manual edge input, including custom-case and legacy-alias compatibility. Q21-01/02 leave manual built-in subset and custom casing unresolved; no parser unification or cache migration is commissioned.
3. **Silent-drop check:** the all-eleven variant map, current serde/FromStr names, both private parser subsets, typed equality, stock `ContrastsWith` producer, response naming, no-filter/working/custom controls, manual weight and loader exits, and unresolved cases are recorded. No `SameAs`/`Cites` finding, actual client effect, or whole-graph acceptance is inferred.

## Independent review boundary

This is CC author attestation only. CDC independently determines finding acceptance, question calibration, and criterion status. The report and index preserve the complete byte prefixes from fixed baseline `56d9f3d79f07d68114ebfb24c576679b446408a1`. The exact nine protected paths were parsed from the canonical manifest at immutable `78847d67ec54f1617bb3f734ceaf60f86a32f99a` and compared as raw bytes to the fixed baseline; all nine matched. Unfinished CDC preparations remain unverified and were not textually inspected or executed.

Commit verification and exact five-path/planning-index ownership return are reported with the final handback. No mutable child or process remains after the scoped commit.


## Independent CDC acceptance — 2026-11-22

REL-01–05/Medium FA-RS-045 independently accepted statically after b0a3735.
Q21-01/02 remain ungraded, with graph builder/Relationship/MCP API owners and
policy-dependent future oracles. CDC clarified approved support narrowing versus
today's contract, actual per-tool response shapes, and captured wrapper versus
uncaptured individual statuses; candidate history retained. See [CDC verification](cdc-verification.md)
and [retrospective](cdc-retrospective.md). Budget0/5, source719 unchanged. All
original owner/scale/runtime/SEARCH/final-arc gates remain; CDC owns planning/index
for original04's source-complete static phase, no active mutable process.
