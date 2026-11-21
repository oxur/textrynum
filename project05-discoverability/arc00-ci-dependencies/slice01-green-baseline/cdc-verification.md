# CDC verification — slice01 baseline

Current verdict: **accepted — 2026-11-21** at edc8db69a2ef45d49e1c369ad5cf489dc58d161b. Actual complete Linux CI, canonical CC close, independent bubble-up review and CDC retrospective are complete. This closes slice01 only; arc00 is not complete. This is a performed independent review record; earlier pending states remain identifiable in Git history and dated execution records.

## Identity and reviewed state

Reviewer: CDC `/root/ci_dependency_cdc`, parent-dispatched requested `gpt-6.1-sol` High, Codex/OpenAI runner. Implementer: separate CC CLI session `01a2000f-9f2d-7660-97b4-f54373f19a94`, whose existing turn_context explicitly reports `gpt-6-luna`, effort `high`, physical source cwd, on-request approval and workspace-write sandbox. No unreported backend identity is asserted. Two-Contributor Autonomous arrangement, arc00 only.

Current assignment reviewed: preserved `cc-prompt-iteration01.md`, predecessor `cc-prompt.md`; source `edc8db69a2ef45d49e1c369ad5cf489dc58d161b`, direct parent `92c72d7058dea91c0d475959e8c5b51b65f577ab`, branch `codex/ci-dependencies`. Source was independently checked clean at handoff. CC returned ownership and its runner exited0 before CDC source checks. CDC now owns the frozen source/index and planning.

## Actual source and intake review

The complete source diff contains exactly Cargo.toml and Cargo.lock. Root manifest changes only Redis requirement1.2→1.8.2; existing feature flags/default disabling, workspace version0.5.0 and MSRV1.91 are unchanged. Lock changes only version/checksum pairs for Redis1.2.0→1.8.2, ethnum1.5.2→1.5.3 and combine4.6.7→4.6.9. No source code, tests, features, ignored cases, workflow or freshness predicate changed.

CDC inspected the original/iteration prompts, governing source/planning instructions, complete CC execution record, required-text readback and the actual runner observations. Initial truncated/incomplete intake was preserved as incomplete; the resumed intake recovers required extents and whole-set queries before source edits. The readback correctly distinguishes a local candidate from actual CI and independent acceptance. Reading evidence remains attestation; the diff and execution checks provide the behavior evidence.

Authoritative Redis1.8.2 dependency metadata declares combine^4.6.8, so old4.6.7 cannot satisfy it; Cargo's4.6.9 resolution is a necessary compatible consequence. CDC inspected tagged ethnum1.5.2/1.5.3 error.rs and the1.5.3 release notes: the new release replaces the failing layout-dependent transmute with standard safe error construction. No vendored shim or compiler downgrade was introduced. This applies Rust Cargo basics CG-B-05/07/10/11 and AP-31 in their actual contexts; no unrelated API redesign is claimed.

## Independently reproduced checks

At the exact returned source SHA on macOS aarch64, Rust/Cargo1.95.0:

| Check personally executed by CDC | Result |
| --- | --- |
| `cargo fmt --all -- --check` | exit0 |
| `make check-deps-cicd` | exit0; compatible dependencies up to date, unchanged real predicate |
| `cargo test --locked -p fabryk-redis --all-features` | exit0;13 passed,0 failed/ignored/filtered, plus empty doctest phase |
| Source branch, clean state, direct parent, exact diff scope and both parsed trailers | verified |

CC's complete local debug/release build, strict clippy and full test results were inspected directly in the runner/logs: each full suite has85 result summaries totaling2,437 passed,0 failed,61 pre-existing ignored,0 filtered. These full local gates are doer-attested observations, not CDC reruns. The61 ignored cases were unchanged; they are not accepted evidence of the behavior they skip. The later audit/release obligations remain open.

The real workflow on Linux uses Rust1.100.0, distinct from local1.95.0. After checking the exact unchanged sole workflow and active state, CDC pushed only the authorized task branch. Fresh [CI run44957414892](https://github.com/oxur/textrynum/actions/runs/44957414892) is tied to edc8db69. CDC retrieved every job/step conclusion and both complete job logs after final completion at2026-11-21T23:06:23Z. Debug job137008481299 and release job137008481067 both succeeded. Each Linux Rust1.100.0 job passed formatting, locked all-feature workspace build, strict clippy and full tests; debug passed actual freshness. Release freshness alone was intentionally skipped by the unchanged debug-only predicate. Each job has85 test summaries totaling2,437 passed,0 failed,61 unchanged ignored,0 measured/filtered; no actual compiler-error or failed-test marker. Complete CI logs inspected at /private/tmp/textrynum-slice01-ci-{debug,release}.log, with GitHub retaining authoritative run logs. No PR, main/release update, merge, tag, publish, deploy or support/purge action occurred.

## Execution observations retained

CC first created unpublished source commit d92f50c1e1e04cb31a34871e85aeb332b1086012 with a blank line between its two co-author lines; git's trailer parser recognized only the last block. CC amended that unpublished message to current edc8db69, whose two exact trailers now parse together. CDC verified both tree IDs equal `3d60b80bfa545132f0cf371a36b780e645806e77`. This record does not claim no amendment occurred. The current valid candidate is not being undone or rewritten. Future prompts require message-file byte/trailer verification before committing and ordinary follow-up commits; no housekeeping amendments, squash, fixup or drop operations.

CC launched a queued duplicate debug build while the original owned build held the artifact lock. Both exited0. CDC identified original PID70995 with active compiler children and queued PID71352 without children; an identity recheck before any proposed cleanup found71352 already exited, so no signal was sent. CC later repeated the already-passing debug suite to capture a simpler log; original complete output already supported the same counts. These are process observations for the retrospective, not product failures. Preserve the distinct executions in existing runner/evidence; future work should query saved output instead of rerunning passed gates for bookkeeping.

## Row walk and remaining gate

Opening ledger has3 rows; all3 are accounted for (CI evidence updated after completion):

| Row | Current evidence and disposition |
| --- | --- |
| BASE-01 | Minimal repair is independently inspected/reproduced with no blocking source finding; final row closure joins the full slice verdict. |
| BASE-02 | Actual complete debug/release CI passed at the exact candidate; final row closure joins the slice verdict. |
| BASE-03 | Intake/delivery reviewed; canonical CC closing report, final bubble-up check and CDC retrospective remain open. |

Do not advance the dependency sweep until BASE-02 and BASE-03 are complete and this verdict is updated. The Operator has now explicitly approved Rust1.94 and current RMCP facade migration for the subsequent sweep; those choices do not waive this baseline gate. Full audit and later features remain unopened.


## Final independent close and bubble-up — 2026-11-21

CC iteration02 runner13019 returned exit0 and documentation ownership before final CDC edits. New actual turn_context again reported gpt-6-luna/high with physical source cwd and normal on-request/workspace-write policy. CDC read the complete closing-report.md and dated execution appendix, verified the actual filenames/diff and retained the independently reproduced source/behavior evidence above. No source change or gate rerun occurred during the close handoff. Source was rechecked clean on codex/ci-dependencies at edc8db69; no native/CC mutable process remains owned or detached.

All3 opening BASE rows close done, with no deferral/no-op or silent drop: BASE-01 exact minimal repair/upstream consequence and affected behavior; BASE-02 both complete actual CI jobs at the candidate; BASE-03 full intake/CC delivery, this independent review, canonical closing report, cdc-retrospective.md and the verified bubble-up. The CC report's proposed/pending labels accurately describe its return before this final reviewer decision; this paragraph supplies the current acceptance rather than inventing CC self-acceptance.

Artifact inventory matches all4 durable files in artifacts/: cc-execution.md, cc-last-message.md, cc-runner.jsonl and cc-runner-stderr.log. All3 issued prompts remain in the slice root; current documentation-close assignment is cc-prompt-iteration02.md, implementation assignment iteration01. Root first-class close/verification/retrospective records are excluded from supporting-artifact inventory. Complete raw CI logs remain available from the exact GitHub jobs; inspected temporary copies supplement that authoritative record.

Against arc-plan slice breakdown, the delivered capability is the minimal actual-green baseline on a nonmerged, pushed task branch. Existing features/tests/freshness/workflow behavior are preserved; no wider latest sweep or declared MSRV proof is claimed. The silent-drop diff is complete: no assigned baseline capability missing, and larger audit/release obligations remain open rather than silently disposed. The necessary combine update does not change decomposition. Arc-plan was updated before the next slice to record this accepted state and the parent-strengthened, approved prospective persistence/MSRV contract. CI-01 and project05 row09 close; DEP-01/CI-02 and final arc composition REVIEW-01 remain open. Final retrospective is linked from the CC close and completed at cdc-retrospective.md. Slice02 may now be planned/executed; no audit, feature, merge or publication authority follows.
