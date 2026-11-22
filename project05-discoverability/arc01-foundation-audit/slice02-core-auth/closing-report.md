# Slice02 closing report — core/auth foundation diagnosis

**Status:** CC proposes done; independent CDC review and acceptance pending.
**Execution date:** 2026-11-22 America/Chicago
**Assignment:** immutable `slice02-core-auth/cc-prompt.md`, issued planning commit `fb28833b824d0fd2401e2cb0de7e5c4e51725e37`
**Source:** `codex/foundation-audit` at `719c58f245fe4a9115c7c0391d3e268c99157e2f`, clean at end of work
**Planning at assignment:** orphan `planning` at `3fb6386906b2c50bb8ec0cd5cc1951d199de55a5`; this close is committed on that branch by the returned commit.

## CORE row walk

| ID | CC disposition | Evidence and limit |
| --- | --- | --- |
| CORE-01 | **Proposed done.** The complete Rust-guidelines intake preceded semantic source work. All 38 assigned paths were read and reconciled individually with the accepted coverage map. | [Core/auth evidence](../../artifacts/2026.11.22-core-auth-evidence.md) records guidance inventory, full 38-path disposition table, reverse consumers and map preservation checks. CDC must independently replay. |
| CORE-02 | **Proposed done.** Function, file/module, logical unit, crate/target, application/service/executable, whole-codebase and workspace/system scales were assessed. Shared root causes were not counted again at every consumer. | [Audit results](../../artifacts/2026.11.22-audit-results-rust.md) contains the scale table, findings and cleared negative leads. Broader pass composition remains pending. |
| CORE-03 | **Proposed done with limits.** Meaningful actual-entrypoint checks covered filesystem provider and tool dispatch, AuthLayer, service lifecycle, discovery Router, Redis mock, and isolated ADC resolution. Locked suites passed. | [Probe source](../../artifacts/2026.11.22-core-auth-probe.rs), [manifest](../../artifacts/2026.11.22-core-auth-probe.toml), [raw run log](../../artifacts/2026.11.22-core-auth-run.log). Local JWKS listener bind was denied by sandbox; no provider call occurred. No Redis server was installed. No MCP wire/client/deployed app was exercised. |
| CORE-04 | **Proposed done.** Eight confirmed, actionable findings use stable IDs FA-RS-001 through FA-RS-008 and include severity, exact 719 source lines, scale, failure evidence, fix direction, compatibility decisions and a validation oracle. Two hypotheses are separate and excluded from confirmed counts. | [Audit results](../../artifacts/2026.11.22-audit-results-rust.md). H-RS-001 needs a domain-policy decision; H-RS-002 needs intended deployment/client validation. |
| CORE-05 | **Proposed done.** Existing index and coverage map now reflect only slice02's 38 dispositions and seven assigned pass01 scale cells. The other rows, owners, hashes, counts and feature inventory were preservation-checked. Diagnosis stayed separate from repair. | [Index](../../artifacts/2026.11.21-audit-index.md), [coverage JSON](../../artifacts/2026.11.21-audit-coverage.json), and the scoped planning commit identified in the CC return. Source is clean at 719. CDC acceptance, other passes and repair work remain open. |

## Exact output inventory

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

Build/dependency/test scratch remains CC-owned under `/private/tmp/arc01-slice02-probe`, `/private/tmp/arc01-slice02-adc`, and `/private/tmp/arc01-slice02-cargo-target`; none is staged. The owned probe manifest/source are the replay recipe. Its local cargo lock is temporary and not claimed to be the product workspace lock. Product suites used `--locked`.

## Scope delivered and outstanding

Delivered: complete assigned intake; all 38 file dispositions; all seven scale dispositions; actual provider and tool dispatch filesystem cases; HTTP auth, retry/state and discovery behavior; Redis mock semantics; ADC subprocess precedence; eight findings and two hypotheses; exact test/run evidence; map/index update; diagnosis-only close.

Not covered or not claimed: external Google/JWKS/userinfo behavior, local JWKS fixture due sandbox socket denial, production Redis parity due missing server, ChatGPT client trials, MCP wire/server behavior, deployed auth wiring, non-default feature/native/full-CI rerun, other pass coverage, repairs, release readiness or independent acceptance. The Google domain-policy distinction and resource-metadata challenge configuration should be reviewed before repair planning.

Bubble-up to arc01: route FA-RS-001 (path containment), FA-RS-002 (provider outage classification), FA-RS-003/004 (JWKS refresh and deadline) and FA-RS-007/008 (mock/production Redis contract) to the later repair handoff with their validation oracles. Preserve H-RS-001 and H-RS-002 as questions, not confirmed totals. Do not mark the arc, project or later slices accepted from CC attestation.

## Return

I have completed the assigned planning result and exact owned commit below. Planning/index mutation ownership is explicitly returned to CDC for independent review. CDC remains read-only to the claimed evidence until it begins its review; source stays protected at the pinned clean revision. No active mutable process remains.
