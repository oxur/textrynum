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
