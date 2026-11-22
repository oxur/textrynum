---
arc: arc05-ci-efficiency
status: planned
depends-on: ["arc01-foundation-audit"]
related: ["arc02-foundation-repair", "arc04-release-validation"]
---

# Measured CI efficiency and preserved confidence

## Current authorization — v1.0, 2026-11-22

Duncan requested an additional project05 arc for analysing, brainstorming and
implementing a series of CI efficiencies/improvements because runs take over
30 minutes and recent runs reportedly took 45–46 minutes. This record adds the
arc while arc01 remains the active Autonomous Mode endpoint. It authorizes this
planning addition now; it does not dispatch this arc, change CI/source, run an
extra native build or workflow, or authorize publication. Current selected roles
remain CDC gpt-6.1-sol High and CC gpt-6-luna High, without substitution.

Outcome: measurably shorter useful CI feedback and green critical-path runtime,
with every original correctness, freshness, feature, MSRV, platform and packaging
gate preserved where applicable. Define numeric targets and the concrete comparison
scope after measurement and explicit agreement; none is invented here.

## Dependency and execution order

Arc01's complete accepted diagnosis supplies correctness constraints and repair
inputs. Identify the particular repair prerequisites for each CI proposal. One
option is to pull a first bounded, measured improvement forward after audit
acceptance so bulk foundation repairs benefit, if findings and dependencies
permit. Another is to complete necessary repairs first. Duncan has not selected
an exact priority; do not interpret the arc number as a requirement to wait for
every foundation repair. Arc02 and this arc must coordinate source ownership and
integration. Arc04 release validation consumes the final accepted CI arrangement
and actual evidence. No established arc path is renamed or renumbered.

## Evidence already available

The unchanged source pin is `719c58f245fe4a9115c7c0391d3e268c99157e2f`.
Push-triggered [run44986949945](https://github.com/oxur/textrynum/actions/runs/44986949945)
is successful; its exact retained [metadata](../artifacts/2026.11.22-ci-efficiency-run44986949945.json)
and [integration summary/log hashes](../artifacts/2026.11.21-release-ref-ci-summary.json)
are descriptive baseline evidence, not an efficiency experiment. Primary complete
logs remain at `/tmp/textrynum-release05-44986949945-35mzoa39/` with the hashes in
that summary. Arc00 also retains successful run44982217154 at the same source;
compare conditions before treating it as a matched control.

| Measure from captured timestamps | Observed value | Meaning |
| --- | --- | --- |
| Run created to updated | 2766s / 46m06s | Includes orchestration; distinguish queue time in later analysis |
| Release job wall time | 2763s / 46m03s | Current critical job; build919s, lint327s, Run tests1493s |
| Debug job wall time | 1909s / 31m49s | Dependencies109s, build451s, lint208s, Run tests1089s |
| Actual Rust1.94 MSRV job wall time | 299s / 4m59s | Check step254s |
| Sum of three job wall times | 4971s / 82m51s | Neither run elapsed time nor a billing measurement |

All three lanes reported `No cache found.` and `save-if: false`; current workflow
saves only for `refs/heads/main`. Shared keys differ by debug/release/MSRV lane.
This is an observed cache-miss/no-save condition, not proof that it explains the
whole runtime or a decision to change trusted-branch policy. Release test-step
Cargo compilation receipt reports24m05; debug reports17m53. Therefore `Run tests`
step duration must not be labelled test execution time. The existing dev, test
and release profiles, feature sets and native dependency populations differ;
quantify reuse and repeated compilation before selecting an intervention.

## Planned work and readiness gates

1. **Reconstruct the baseline.** Inventory actual workflow/job/step timing, queue
   and setup overhead, useful first-failure feedback, critical path, dependency
   graph, artifacts, caches and hit/miss/save eligibility. Separate cold/warm
   cache and unchanged-SHA effects. Reconcile Cargo profiles, feature/target
   combinations, test compilation versus execution, native build duplication,
   toolchain changes, runner hardware and lock/source revisions. Preserve raw
   primary evidence and parser assumptions. Job sums are not billing claims.
2. **Brainstorm from evidence.** Compare alternative cache scope/key/save and
   trust policies, job ordering or decomposition, native/artifact reuse,
   feature/profile scheduling, instrumentation and runner choices as candidates.
   Record expected mechanism, costs, trust/security constraints, invalidation,
   failure visibility, platform implications and cases where each will not help.
   No candidate is a proven fix before comparison.
3. **Make a bounded implementation contract ready.** Choose one coherent change
   with its prerequisites, exact source paths and mutation owner, rollback,
   retained-gate mapping, compatibility decisions, discriminating oracle and
   comparison design. Use scientific-methods to specify conditions, repeated
   observations where needed, outcomes, confounders, stopping/re-entry and
   evidence limits. Agree trial scope/targets before commissioning trials.
   Issue implementation slice prompts only after this readiness gate and current
   execution authorization; do not preissue them from this planning addition.
4. **Implement and compare in real CI.** Use the approved roles, isolated task
   branch, preserved prompts/iterations, full logs and true exit status. Compare
   meaningful before/after conditions, report cold and warm results separately,
   and retain failures and unsuccessful ideas. Local checks support but do not
   establish hosted CI runtime improvement.
5. **Independently accept and hand back.** Verify the measured mechanism and
   runtime/feedback result with unchanged correctness. Map every original gate
   to the new arrangement and prove any proposed equivalence. Update operational
   guidance, repair/release dependencies and ledgers; write a retrospective.

No skipping tests, weakening dependency freshness, reducing feature coverage,
hiding failures, blanket warning suppression or claiming unmeasured cache wins.
Any proposed rearrangement must retain and demonstrate the original gates.
No external messages, service deployment or release publication are included.

## Records and references

Canonical slice plans, ledgers, immutable prompts, closes and independent reviews
will be created as bounded work becomes ready. CI analysis/results/evidence use
project05 `artifacts/`; do not place them in workbench or invent completed slices.
Current [project plan](../project-plan.md) and [ledger](ledger.md) govern acceptance.
Primary technical references for later revalidation: [GitHub cache reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching)
and [the exact current rust-cache v2.9.3 documentation](https://raw.githubusercontent.com/Swatinem/rust-cache/v2.9.3/README.md).

## History

- 2026-11-22 v1.0 — Added from Duncan's explicit request during active arc01;
  reserved arc05 without changing existing paths or the audit endpoint. Captured
  an actual descriptive46m06 baseline and optional post-audit early improvement.
