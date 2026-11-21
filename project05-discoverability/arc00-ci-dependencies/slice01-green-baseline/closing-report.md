# Slice01 closing report — CC performed work, CDC acceptance pending

Date: 2026-11-21

Current close assignment: [`cc-prompt-iteration02.md`](cc-prompt-iteration02.md)

Implementation assignment: [`cc-prompt-iteration01.md`](cc-prompt-iteration01.md)

Predecessor and original records: [`cc-prompt.md`](cc-prompt.md), [`cc-prompt-iteration01.md`](cc-prompt-iteration01.md)

This is CC's performed-work report for the documentation-only close handoff. It is not independent acceptance. CDC must verify this report, complete the retrospective and bubble-up review, and decide the slice verdict. The candidate remains frozen.

## Reviewed source pin and delivery

- Source checkout: `/Users/oubiwann/lab/oxur/textrynum`, branch `codex/ci-dependencies`.
- Candidate: `edc8db69a2ef45d49e1c369ad5cf489dc58d161b`, parent `92c72d7058dea91c0d475959e8c5b51b65f577ab`.
- Current source status: clean. Commit changed exactly `Cargo.toml` and `Cargo.lock`.
- Planning commit: none in this close packet; the permitted planning edits are this report and the dated append to `artifacts/cc-execution.md`. No commit was created.

The root Redis workspace requirement changed from `1.2` to `1.8.2`; `default-features = false` and features `tokio-comp` and `connection-manager` remain. Cargo generated these lock changes:

| Package | Before | After | Reason |
| --- | --- | --- | --- |
| `redis` | 1.2.0 | 1.8.2 | Satisfies the current compatible Redis requirement and CI freshness check; sole workspace consumer is `fabryk-redis`. |
| `ethnum` | 1.5.2 | 1.5.3 | Replaces the observed invalid layout-dependent `transmute` with safe upstream error construction. |
| `combine` | 4.6.7 | 4.6.9 | Necessary resolution consequence: Redis 1.8.2 requires `combine ^4.6.8`. |

No other locked package changed. There was no source/API, feature, test, CI workflow, toolchain or workspace MSRV change; MSRV remains 1.91. No broad dependency sweep, vendoring, local patch or compiler downgrade was made. The checked-in source diff and all resolution consequences were independently inspected by CDC.

## Validation evidence

### Local gates, supplementary

The implementation iteration recorded the original required local commands on macOS aarch64 (`aarch64-apple-darwin`), Rust/Cargo 1.95.0. Every command exited 0:

| Command | Result |
| --- | --- |
| `make check-deps-cicd` | Pass; compatible dependencies current. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo build --locked --all-features --workspace` | Pass. |
| `cargo clippy --locked --all-features --workspace -- -D warnings` | Pass. |
| `cargo test --locked --all-features --workspace` | 85 successful summaries; 2,437 passed, 0 failed, 61 ignored, 0 measured, 0 filtered. |
| `cargo build --locked --all-features --workspace --release` | Pass. |
| `cargo clippy --locked --all-features --workspace --release -- -D warnings` | Pass. |
| `cargo test --locked --all-features --workspace --release` | 85 successful summaries; 2,437 passed, 0 failed, 61 ignored, 0 measured, 0 filtered. |
| `git diff --check HEAD^ HEAD` | Pass on the exact committed diff. |
| `cargo tree --locked -i combine` | Pass; confirms `combine → redis 1.8.2 → fabryk-redis 0.5.0`. |

The local test logs are `/private/tmp/textrynum-slice01-debug-tests.log` and `/private/tmp/textrynum-slice01-release-tests.log`. This local evidence is supplementary to Linux CI. CDC separately reproduced formatting, dependency freshness and the affected Redis tests: 13 passed. This close iteration did not rerun any gate.

### Actual complete Linux CI

Actual [GitHub Actions run 44957414892](https://github.com/oxur/textrynum/actions/runs/44957414892) is a completed successful `push` run at the exact candidate SHA, updated `2026-11-21T23:06:23Z`. The complete read-only run query returned head SHA `edc8db69a2ef45d49e1c369ad5cf489dc58d161b`, status `completed`, conclusion `success`.

| Job | Conclusion | Every step disposition |
| --- | --- | --- |
| `Build and Test (debug)` (`137008481299`) | Success | Steps 1–5 setup/checkout/toolchain/system dependencies/cache: success; 6 Check dependencies: success; 7 formatting, 8 build, 9 lint, 10 tests: success; 19–21 post-cache, post-checkout, job completion: success. |
| `Build and Test (release)` (`137008481067`) | Success | Steps 1–5 setup/checkout/toolchain/system dependencies/cache: success; 6 Check dependencies: skipped by the unchanged debug-only predicate; 7 formatting, 8 build, 9 lint, 10 tests: success; 19–21 post-cache, post-checkout, job completion: success. |

Both jobs used Linux and Rust 1.100.0. Both completed formatting, locked all-feature workspace build, strict clippy and full tests. The only skipped step was the release dependency-freshness check, intentionally excluded by the unchanged workflow condition; no required build, lint or test step was skipped. The check was run and passed in debug. Complete saved CI logs are `/private/tmp/textrynum-slice01-ci-debug.log` and `/private/tmp/textrynum-slice01-ci-release.log`.

All four complete local and CI test logs were parsed in full. Each contained 85 successful test-result summaries, totaling 2,437 passed, 0 failed, 61 ignored, 0 measured and 0 filtered. Each had zero failed-result or compiler-error markers. The 61 ignored tests are existing ignored cases; this report makes no claim that their behaviors were exercised. Rust 1.100.0 CI and local Rust 1.95.0 do not establish the declared Rust 1.91 MSRV beyond this scoped compatible dependency repair.

## Ledger walk — 3 opening rows, all accounted for

These are proposed dispositions at CC return. The ledger remains open and unchanged until CDC makes its independent decision.

| ID | Proposed disposition | Evidence and remaining condition |
| --- | --- | --- |
| BASE-01 | **Proposed done** | Exact two-file source diff, targeted Cargo resolution, upstream safe ethnum repair and required combine consequence are recorded above and independently inspected by CDC. CDC's final row closure remains pending. |
| BASE-02 | **Proposed done** | Actual full debug and release CI passed at the candidate SHA, with all steps accounted for above. Debug freshness passed; the release-only freshness skip follows the unchanged workflow predicate. CDC final row closure remains pending. |
| BASE-03 | **Open at CC return** | Intake, implementation evidence, scope accounting and this closing report are supplied. CDC still must verify the report, close rows as warranted, complete the arc bubble-up/retrospective, and decide acceptance. No final `done`, `deferred` or `no-op` disposition is claimed for this row. |

All three opening rows are represented; none is silently dropped, deferred or treated as a no-op. The initial runtime blocker and incomplete intake remain preserved in `artifacts/cc-execution.md`; the actual runtime projection resolved the blocker, and iteration01 records completion of the required intake before source edits. This report is CC attestation plus saved execution and CI evidence; reading/attestation alone is not behavioral proof.

## Artifact inventory

Durable supporting artifacts present under the slice `artifacts/` directory:

- [`artifacts/cc-execution.md`](artifacts/cc-execution.md) — append-only intake, execution evidence, process observations and returns.
- [`artifacts/cc-last-message.md`](artifacts/cc-last-message.md) — preserved initial blocked handoff.
- [`artifacts/cc-runner.jsonl`](artifacts/cc-runner.jsonl) — runner transcript.
- [`artifacts/cc-runner-stderr.log`](artifacts/cc-runner-stderr.log) — runner standard-error log.

The prompts, slice plan, ledger, CDC verification and this closing report are first-class slice-root records, excluded from the supporting-artifact inventory. The forthcoming [`cdc-retrospective.md`](cdc-retrospective.md) is a pending CDC-owned duty and does not yet exist; its contents are not presumed here. No other durable supporting artifact is present in the actual inventory.

## Bubble-up to the arc

1. **Assigned capability and integration state.** Arc00 assigns slice01 to establish the first useful, real green CI increment before the broader latest-dependency sweep. This slice delivered a minimal Redis/ethnum baseline repair at `edc8db69a2ef45d49e1c369ad5cf489dc58d161b` on the nonmerged `codex/ci-dependencies` task branch. Actual complete debug/release CI verifies the improvement. The task branch has been pushed by CDC for CI; it is not merged to main or a release branch. No publication or release action occurred.
2. **Arc-plan findings.** Redis 1.8.2 necessarily moved `combine` to 4.6.9 because of its `^4.6.8` requirement; this is fully accounted for and does not require changing slice order or scope. The preserved duplicate-build/replay and commit-message amendment observations are process findings for CDC's retrospective and future prompting. I found no new dependency compatibility or architecture decision that changes the arc breakdown. CDC should confirm whether any arc-plan update is needed before slice02 is planned against it.
3. **Scope-as-specified versus scope-as-delivered.** The authorized baseline requirement, targeted lock updates, unchanged local gates, exact two-file commit, actual complete green CI and evidence return were delivered. No Rust source change was needed, no additional package update or feature work was introduced, and no specified slice01 item is silently dropped. The all-dependency sweep, actual Rust 1.94 proof, and baseline-writer/new-reader persistence compatibility work belong to prospective slice02; they were not part of this baseline delivery and remain gated on independent slice01 acceptance. Full audit, project06/07 features and publication remain out of scope.

CDC owns final row closure, review verdict, retrospective, final bubble-up confirmation and acceptance. Slice01/arc00 is not declared complete here; slice02 must not start until the recorded acceptance gate is met.
