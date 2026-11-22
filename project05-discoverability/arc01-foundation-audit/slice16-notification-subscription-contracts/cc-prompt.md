# Slice16 — notification/subscription composition

Read the public notifier and server callback composition. Map shared subscription
state and source-level update/pruning calls, then assess the two concrete manual
sequences below. This is separate static code-quality work; source719 read-only,
no runtime/service/RPC execution. Initial0/5 refinements;15 stays1/5used and its
VEC-05 attribution is UNMET/EXCEPTION-PENDING. No Operator approval has arrived.
Elapsed time is not approval.03 remains2/5used; original SEARCH is untouched.

## Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Propagate this paragraph to later authorized assignments.

## Same roles and exact five-path ownership

Retained CC gpt-6-luna High session01a2019f-73de-7de1-831f-53d55b3da247; CDC
Sol6.1High independent slice review. Acknowledge checked requested receipt; hidden
backend unreported. No repeated metadata hunt or silent substitution. Source
/Users/oubiwann/lab/oxur/textrynum at719c58f245fe4a9115c7c0391d3e268c99157e2f,
codex/foundation-audit, is read-only INCLUDING index. Planning is exact physical
/Users/oubiwann/lab/oxur/textrynum/.worktrees/planning, orphanplanning. Use ABSOLUTE
paths and Git -C that worktree for mutations; --cd does not prove retained tool
cwd changed. Do not guess .worktrees/source.

Only these five paths/planning index pass to you:
1. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-notification-subscription-contracts.md
2. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md
3. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md
4. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice16-notification-subscription-contracts/ledger.md
5. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice16-notification-subscription-contracts/closing-report.md


Static file/text/manual reasoning and scoped Markdown records only. No product
build/test/behavioral program, transport/peer fixture, notification/RPC/provider/
service/model operation, source repair, CI edit, push/publication, Daybreak or
external message. No blocked SEARCH preparation inspection, reconciliation,
execution/reassignment or lock work; preservation metadata only. No protocol
migration or held project06 feature is commissioned. Original owner/cell/full-
pass and final arc gates remain pending. Root stays read-only while you own index.

## Required intake and source-cited readback

Read this slice plan/ledger/prompt, current arc-plan/ledger/current AGENTS and
slice15 CDC verification/closing report for the exact pending attribution issue.
Retain completed unchanged framework, PM wayfinder/operation/Autonomous contract,
work-verification, full code-auditing guides, engineering prompt/intake and
methodology-stewardship. Retain complete Rust skill/all36existingguides with
anti-patterns first; CLI09 missing remains known. Re-read relevant uncertain
Arc/async/collection/public-API/protocol rules; no corpus restart for a slice name.

Read COMPLETE source files under the absolute product root:
- crates/fabryk-mcp-core/src/notifier.rs255lines
- crates/fabryk-mcp-core/src/server.rs1243
- crates/fabryk-mcp-core/src/lib.rs162
Three full files1660lines, including all bodies/tests. Recover any truncated
range. Full intake supports only this notification/subscription facet, not all
server behavior. CDC own current controlling ranges are recorded in plan.

Read selected PRIMARY dependency references under absolute
/Users/oubiwann/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-3.11.0/:
- src/model.rs150–205/225–232/2185–2270: versions and legacy resource types
- src/handler/server.rs76–145/213–233/530–583: lifecycle gate/dispatch/defaults
- src/service.rs241–247: uses_legacy_lifecycle predicate
- src/service/server.rs790–814/945–960: peer_not macro and update method mapping
These are narrow API/dispatch references, not full third-party implementation
coverage. CDC compared these four entire files with the cached rmcp3.11archive;
archive SHA256 matches product Cargo.lock391f153431ab68664d946513ea0cb3edbca78eaf99c5a983036cacab6220d85b.
Provided receipt (no Cargo/registry/build run): model.rsSHA256f4de606fa0a455a4de77067fae792271f9c079d5a59cbc835ea00931559e6111;
handler/server.rscbf2708ec42e50bc98d24575ed01e6347769aa15db7a636f6e1757ea7bd0ec67;
service/server.rsb87a9561d539d44768c90f3b77ae847d0d2f3429d237e9e7c6c2e588b000f595;
service.rs5d7491a2d169daae244d1db8d14123699a6345bfc7956477a9718cd16cb092e8.
No need duplicate the archive/check or inspect unrelated dependency code.

Primary protocol input already read by CDC2026-11-22:
https://modelcontextprotocol.io/specification/2025-11-25/server/resources
https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/schema/2025-11-25/schema.ts
Paraphrased contract: client resource subscriptions request subsequent updates;
unsubscribe cancels them. ResourceUpdated guidance targets prior subscribers.
ResourceListChanged separately permits unsolicited notifications. Preserve the
SHOULD-level guidance rather than inventing an exact-once SLA or conflating the
two notification types. Quote/paraphrase sparingly; cite exact versioned sources.
No new browser/provider/service operation is required to receive this input.

Give source-cited readback BEFORE edits: complete intake, actual reachable
version/callback boundary, shared state/delivery attempts, deliberate policies,
manual case assumptions, protected populations and pending attribution status.

## Concrete source-guided cases

1. Version/ownership map: server get_info enables resource subscriptions only
   with resource registry (server349–377), on_initialized registers the context
   peer (379–392); server Clone and notifier Arc state are shared (server76–95,
   notifier29–41). HTTP factory clones self (server248–260). SDKLATEST is2026-
   07-28, LATEST_WITH_INITIALIZE2025-11-25. Legacy callbacks are dispatched only
   when uses_legacy_lifecycle is true. Assess those initialized compatibility
   sessions explicitly; do not claim every current client uses this path or
   grade optional2026 subscriptions/listen absence automatically as a defect.
2. Mixed prune sequence: assume explicit URI subscribersA/B, A's method sends
   succeed and B's fail in both passes; no concurrent subscription changes.
   resource_updated112–141 sends to the snapshot once, then on any failure the
   pruning loop creates another real ResourceUpdated param and calls send again
   for current subscribers. Count METHOD ATTEMPTS per peer and first-pass success
   separately; no actual delivery or client read was observed. Compare all-success
   branch, no-subscriber fallback142–158 and log/list_changed one-pass failure-
   index pruning. No exact-once wire guarantee or measured UX impact is assumed.
3. Peer lifecycle composition: subscribe handler uses context.peer (server437–448),
   unsubscribe ignores context and calls URI-wide removal (452–461). The helper
   expressly documents removing ALL peers because Peer lacks PartialEq
   (notifier86–93); that standalone bulk policy is contrary evidence, not itself
   an undocumented helper defect. Trace server composition against the client
   subscription contract, using two initialized live peers sharing one notifier.
   Sequence: A subscribesURI; B subscribes sameURI; A unsubscribes; A subscribes
   again; updateURI. Track map/global-peer state and selected recipients after
   each operation. At the intermediate no-map state, documented broadcast fallback
   may still target B, so do NOT falsely claim immediate notification loss there.
   The later explicit map distinguishes whether B's still-active registration
   survived. This is manual source-state reasoning, not an actual RPC test.
4. Keep deliberate policies/units precise: public unsubscribe helper is bulk;
   no-entry fallback is documented backwards compatibility; returned success is
   clients, not total send attempts. Any server-specific incompatibility must
   cite the actual legacy callback/contract, not recommission tenant/auth policy.
   Repeated-subscribe dedup semantics may remain a question absent a supported
   idempotence promise. Don't infer a new mandatory feature from an optional API.
5. Propose stable FA-RS-040+ only for supported roots; no count target. Keep prune
   resend and per-peer lifecycle mechanisms distinct unless evidence establishes
   one genuinely shared root/fix/oracle. Fix direction must account for concurrent
   map changes and identity: do not blindly remove snapshot indices from a later
   mutable vector or assume Peer gains PartialEq. Preserve compatibility of the
   documented bulk/broadcast API in any future repair decision. No repair now.

## Deliverables and protected prior work

Companion artifact: full intake/selected-reference receipt, version/ownership map,
manual state/attempt tables, at least five concrete contrary controls, open
questions and actual static bookkeeping exits/limits. Detailed diagnoses ONCE in
new cumulative Rust section, with stable ID/severity/rationale/pinnedsource719
file:line/Scale/confidence/consumer/repair owner/fix/compatibility/oracle. No runtime,
external client or actual ChatGPT usability claim. Zero findings is valid. CC
ledger/close author-proposed; CDC independently reviews. Index one new note.

Retain exact original report/index bytes, append with only needed separator
bytes, verify exact retained prefix. No cosmetic suffix/status-prose assertions.
Do not change prior001–039 dispositions, VEC-05 exception, original360owner/49cell
coverage or pending SEARCH/full-pass/composition gates. NTF-05 concerns THIS
assignment's own commits and preserving the prior unmet issue, not declaring all
historical commits compliant. Operator question is pending; never assume approval.

Protected artifact populations are explicitly distinct under ABSOLUTE planning/
project05-discoverability/artifacts/; hash only, no interpretation/execution:
- Original author evidence (each exact basename):
  2026.11.22-content-search-probe.rs;
  2026.11.22-content-search-probe.toml;
  2026.11.22-content-search-probe.lock;
  2026.11.22-content-search-run.log.
- Unfinished/unverified CDC preparations (each exact basename):
  2026.11.22-content-search-cdc-probe.rs;
  2026.11.22-content-search-cdc-probe.toml;
  2026.11.22-content-search-cdc-probe.lock;
  2026.11.22-content-search-cdc-run.log.
- 2026.11.21-audit-coverage.json.
Exact names/hashes and distinct roles are in REQUIRED slice15closing report;
use that manifest, not a generic four-file count. Preserve all nine files.

## Privacy, own commit compliance and handback

Screen exact five owned path/content bytes in memory with:

```python
import re
pattern = re.compile(bytes([87,97,108,103,114,101,101,110,115]), re.I)
```

COUNTS ONLY; Acme Inc. examples. Verify exact five outputs, source719 clean,
pre-assignment report/index prefixes, protected manifest hashes, real source
anchors and no old status changes. Record actual checks; no stale pending-check
prose after completion. Correct minor document issues proportionately.

MANDATORY: write complete commit subject/body to ONE owned temp message file,
blank line then BOTH ADJACENT footer lines:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

Verify file with git interpret-trailers --parse BEFORE commit, requiring both;
commit --file <ownedpath>, then git show -s --format='%(trailers:key=Co-authored-by,valueonly)' <actualcommit>.
Separate -m paragraphs split trailer blocks; do not do that. No amend/rebase/
rewrite/squash/fixup/drop/ordinarymerge/push. bf193 exception stays pending.

Return exact commit/five paths, actual parsed trailers, source-backed conclusions/
questions/static limits/privacycounts, source719clean,0/5refinement budget and
explicit five-path/planning-index handback with no active mutable command.
