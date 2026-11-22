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
- Findings001–043 and Q19-01–04, original coverage38/31/291 and49cells7/7/35, SEARCH, full ECL/pass-scale and final-arc gates remain unchanged. At the initial close the refinement budget was0/5; correction01 now uses1/5, with4 remaining. The initial candidate/review history is preserved.

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


## Correction01 — 2026-11-22

Corrected FA-RS-044 routing to the actual `ecl-stages` package / `ecl_stages::aggregate` namespace and made its future oracle conditional on an explicit approved policy. The current source continues to accept delimiter-bearing string values; documentation alone does not make the demonstrated inputs invalid. Under a future approved policy that supports these values, verify distinct groups/sums1 and2, copied group fields, and ID compatibility. Under an explicitly approved narrowing policy, verify predictable rejection before coalescing, without simultaneously requiring successful two-group output. Controls follow the selected policy. No policy or repair was chosen. FA-RS-044 remains CC-proposed pending CDC review; Q20-01 remains ungraded. No tests, programs, fixtures, source edits, or repairs occurred.

Pre-commit checks: the cumulative Rust report retained its exact 107,669-byte historical prefix; the 20,603-byte audit index matched its correction-baseline SHA-256 and its earlier 20,053-byte prefix; `git diff --check` passed; source remained clean at719; the encoded privacy-pattern count was zero in each of the five owned paths. Hash-only checks matched seven of nine inherited protected-file fingerprints. The 541-byte `2026.11.22-content-search-cdc-probe.toml` and 541,517-byte `2026.11.21-audit-coverage.json` matched their recorded sizes but not their inherited hashes; neither content was inspected, and the unfinished CDC evidence remains UNVERIFIED. No assertion is made that those two fingerprints were unchanged from the inherited manifest. The index artifact is an authorized handback path and remained byte-identical.


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
