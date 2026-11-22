# Slice02 closing report — core/auth foundation diagnosis

**Status:** CC proposes done; independent CDC review and acceptance pending.
**Execution date:** 2026-11-22 America/Chicago
**Assignment:** immutable `slice02-core-auth/cc-prompt.md`, issued planning commit `fb28833b824d0fd2401e2cb0de7e5c4e51725e37`
**Source:** `codex/foundation-audit` at `719c58f245fe4a9115c7c0391d3e268c99157e2f`, clean at end of work
**Planning at assignment:** orphan `planning` at `3fb6386906b2c50bb8ec0cd5cc1951d199de55a5`; this close is committed on that branch by the returned commit.
**Refinement01:** immutable `cc-prompt-iteration01.md`, issued/current planning HEAD `c6095aa225ccb571fdb0db4222dbe47edbb7cbed`. This correction changes only the six assigned existing result/close paths; it does not rerun suites or providers.

## CORE row walk

| ID | CC disposition | Evidence and limit |
| --- | --- | --- |
| CORE-01 | **Proposed done.** The full Rust-guidelines intake was completed before the initial semantic source work; this refinement retains that receipt. All 38 assigned paths were individually read and reconciled. | [Core/auth evidence](../../artifacts/2026.11.22-core-auth-evidence.md) records the full36-guide inventory, absent CLI09 guide caveat, 38-path table, reverse consumers and map preservation. CDC acceptance remains pending. |
| CORE-02 | **Proposed done.** All seven scales were assessed. Shared root causes remain deduplicated. The corrected record has ten confirmed findings (FA-RS-001..010) and two separate hypotheses. | [Audit results](../../artifacts/2026.11.22-audit-results-rust.md) contains source citations, disposition, owners, compatibility decisions and repair oracles. Wider codebase composition remains pending. |
| CORE-03 | **Proposed done with limits.** Original actual-entrypoint probes and locked-suite outputs remain the author evidence; CDC's independent local-only witnesses are cited separately. Seven-crate test count is 233 by arithmetic correction, not new tests or a rerun; consumer `fabryk-mcp-content` remains a separate 136-unit/10-ignored-doc witness. | [Author probe](../../artifacts/2026.11.22-core-auth-probe.rs), [author raw run log](../../artifacts/2026.11.22-core-auth-run.log), [CDC replay/evidence](../../artifacts/2026.11.22-core-auth-cdc-evidence.md), and [full CDC replay log](../../artifacts/2026.11.22-core-auth-cdc-replay.log). CDC observed unknown-kid request counts 1 then 2, a stalled request pending at a 500 ms outer guard, and corrected ADC directory resolution. No Google provider/Redis server/MCP wire/client/deployed-app/ChatGPT usability evidence is claimed. |
| CORE-04 | **Proposed done.** Ten confirmed, actionable findings retain IDs FA-RS-001..008 and add FA-RS-009 opaque-token audience gap and FA-RS-010 case-sensitive Bearer scheme rejection. Each has exact 719 source lines, severity, scale, evidence strength, later repair owner, fix direction, compatibility decision and oracle. Two hypotheses remain outside totals. | [Audit results](../../artifacts/2026.11.22-audit-results-rust.md). H-RS-001 remains domain-policy sensitive; H-RS-002 remains dependent on deployment/client requirements. |
| CORE-05 | **Proposed done.** Only the 38 owned path dispositions and seven pass01 scale cells carry CC-proposed review; no independent acceptance is claimed. The other 322 path rows and 42 scale cells remain pending. Source tracked bytes/index stayed unchanged and clean719 was restored after the retained two-file scratch incident. | [Index](../../artifacts/2026.11.21-audit-index.md), [coverage JSON](../../artifacts/2026.11.21-audit-coverage.json), and this close. Map acceptance, CDC slice acceptance, other passes and repair work remain separate. |

## Exact output inventory and refinement scope

Owned new project artifacts, all under `project05-discoverability/artifacts/`:

- `2026.11.22-audit-results-rust.md`
- `2026.11.22-core-auth-evidence.md`
- `2026.11.22-core-auth-probe.rs`
- `2026.11.22-core-auth-probe.toml`
- `2026.11.22-core-auth-run.log`

Updated owned paths:

- Existing `project05-discoverability/artifacts/2026.11.21-audit-index.md`
- Existing `project05-discoverability/artifacts/2026.11.21-audit-coverage.json`
- `project05-discoverability/arc01-foundation-audit/slice02-core-auth/ledger.md`
- This `closing-report.md`

The report, evidence, probe, manifest and raw log above are pre-existing outputs from the initial assignment. This refinement changed only the two report/evidence files, the index, coverage JSON, ledger and this close; the probe, manifest, raw author log, issued prompts and CDC evidence/failure records are preserved unchanged. No source, native output, index outside the named JSON/Markdown, probe, provider or test was run or edited for this correction.

The earlier guessed prose-base64 privacy scan was not an accepted oracle. The exact parent-supplied recipe in `cc-prompt-iteration02.md` screened the current bytes of all six absolute owned paths and returned `owned_paths 6 private_pattern_matches 0`; the evidence record preserves counts only.

Build/dependency/test scratch remains from the prior assignment under `/private/tmp/arc01-slice02-probe`, `/private/tmp/arc01-slice02-adc`, and `/private/tmp/arc01-slice02-cargo-target`; none is staged. The owned probe manifest/source are the replay recipe. Its local cargo lock is temporary and not claimed to be the product workspace lock. Product suites used `--locked` in the original assignment only.

## Scope delivered and outstanding

Delivered: complete assigned intake; all 38 file dispositions; all seven scale dispositions; actual provider and tool dispatch filesystem cases; HTTP auth, retry/state and discovery behavior; Redis mock semantics; ADC subprocess precedence; ten confirmed findings and two hypotheses; exact author evidence plus separately attributed CDC local witnesses; corrected index/count/source-incident chronology; diagnosis-only close. FA-RS-009 explicitly records the static configured-audience gap between JWT and opaque userinfo paths. FA-RS-010 records lowercase Bearer rejection against RFC 9110 §11.1.

Not covered or not claimed: external Google/JWKS/userinfo behavior or real tokens, production Redis parity due missing server, ChatGPT client trials, MCP wire/server behavior, deployed auth wiring, non-default feature/native/full-CI rerun, other pass coverage, repairs, release readiness or independent acceptance. The original author loopback EPERM remains preserved; CDC later independently observed same-unknown-kid counts 1 then 2 and one stalled request pending at a 500 ms outer guard before cancellation, which is not an internal deadline or proof of infinite duration. CDC also corrected an ADC replay binary collision before its successful directory observation. The Google domain-policy distinction and resource-metadata challenge configuration remain questions for repair planning.

Bubble-up to arc01: route FA-RS-001..010 to a later repair handoff with their named logical owners and validation oracles. In particular, decide the supported opaque-token compatibility contract before changing FA-RS-009; preserve token-byte/empty/invalid controls for FA-RS-010. Preserve H-RS-001 and H-RS-002 as questions, not confirmed totals. Do not mark the arc, project or later slices accepted from CC attestation.

## Return

Refinement01's six-path correction is complete. I explicitly return ownership of those six planning paths and their exact planning commit to CDC for independent review; CDC retains source/native authority. Source remains protected at pinned clean719 with tracked bytes and index unchanged and the clean tree restored after the documented transient scratch incident. All findings remain CC proposed-done only; CDC review, other passes and arc acceptance remain pending. No active mutable process remains.
