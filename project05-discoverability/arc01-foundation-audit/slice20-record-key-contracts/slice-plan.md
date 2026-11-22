# Record grouping and assembly key contracts

Status: separate bounded static scope recorded before author work; initial 0/5
refinements. Same retained Luna High author and Sol6.1 High CDC. Source/index719
read-only; existing Rust-library API assessment, no pipeline feature integration.

## Exact population and own readiness

Full author primary files: ecl-stages aggregate.rs665, assemble.rs616, lib.rs53:
1,334 lines including all tests. Fresh companion topo/traits.rs1–25/165–282 for
Record/PipelineItem/Stage/StageContext. Reuse actual18 companion dispatch receipt
if referring to runner/batch; recover relevant full units where uncertain. This
is not full ECL/runner/pass acceptance or another reviewer's borrowed intake.

CDC own primary reads are complete: aggregate1–220/215–390/390–665;
assemble1–215/215–370/370–616; lib1–53. Own topo traits175–310 plus retained18
Stage/Record definitions and generic StageRegistry90–190. A scoped CLI reference
query found no Aggregate/Assemble references; tracked md/toml query found no stage
contract, so no CLI integration or external documentation promise is claimed.
Current public module/config/method contracts, not historical plans, control.

## Bounded cases and calibration

Aggregate helper75–82 uses raw as_str values joined with '|'; it does not JSON-
serialize/quote strings. Trace exact encoding and actual public admissibility.
Manual two-field tuples ('AcmeA|AcmeB','AcmeC') versus ('AcmeA','AcmeB|AcmeC'),
all fields present/string, group_by left/right, amounts1/2 and sum: trace key,
group membership, copied fields and totals. No execution or observed output.
Check current restrictions/validation before classifying. Controls: equal tuples,
separator-free tuples, one grouping field containing a literal separator, empty
input/global grouping policy. Numeric/missing key types and count semantics are
not presumed promised; keep questions separate instead of expanding the scope.

Assemble index keys use stream alone; investigate repeated stream/different
foreign_key definitions, but Vec/deserialization alone does NOT establish supported
repetition. Inspect current docs/tests/validation, local/foreign/output identity,
aliases and caller responsibilities. Distinguish table overwrite, source-derived
nesting and observed effects (none). Keep same stream/same foreign key with distinct
local keys/aliases, single join, distinct streams, first-match/collect and missing
secondary behavior as controls. Retain a question if repeated-stream support is
indeterminate; do not commission it as a project06 feature.

Current public Rust Stage implementations are included in full assessment; original
workflow cancelled and project06 features remain held. Any accepted current-contract
problem may be resolved by explicit supported preconditions/rejection or a later
compatible correction, not mandatory new integration. Separate root mechanisms;
no finding-count target. Stable044+ only for supported diagnoses, questions allowed.
All results project05artifacts; prior001–043, coverage38/31/291,49cells7/7/35,
SEARCH/fullpass/finalcomposition unchanged. No fixtures, behavior program or repair.


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


## Independent CDC acceptance — 2026-11-22

CDC accepts KEY-01–05 and Medium FA-RS-044 within this static existing-library
contract facet after correctionbedf656 and independent preservation resolution.
Q20-01 remains ungraded. See [CDC verification](cdc-verification.md) and
[retrospective](cdc-retrospective.md). Original b8d0/bedf assertions and failed
checks remain chronological witnesses. Budget1/5 used,4 remaining. No runtime,
whole-owner/pass/scale/SEARCH or final-arc acceptance follows. Source719 unchanged;
CDC owns planning/index for next approved bounded scope, no active mutable child.
