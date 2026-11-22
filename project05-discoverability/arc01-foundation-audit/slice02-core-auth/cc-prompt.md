# CC assignment — slice02 core/auth foundation diagnosis

You are the selected exact gpt-6-luna High CC on Codex/OpenAI CLI, same session
01a2019f-73de-7de1-831f-53d55b3da247. CDC is independent gpt-6.1-sol High. The
verified own-session turn_context receipt remains:
/Users/oubiwann/Library/Application Support/ChatGPT-Profiles/personal/codex-home/sessions/2026/11/21/rollout-2026-11-21T22-59-17-01a2019f-73de-7de1-831f-53d55b3da247.jsonl
Requested model/effort, on-request approval and workspace-write were verified;
hidden backend identity is unreported. No repeated introspection or substitution.
Physical source /Users/oubiwann/lab/oxur/textrynum, clean codex/foundation-audit,
exact 719c58f245fe4a9115c7c0391d3e268c99157e2f. Physical planning is
/Users/oubiwann/lab/oxur/textrynum/.worktrees/planning, orphan planning. Accepted
map checkpoint: 98b3c18fe68afa8145c49122af669a5fd90fd0fc plus this open-set commit.

## Endpoint and ownership

Complete pass01 diagnosis, all 38 assigned files and all seven review scales,
actual meaningful evidence, honest limitations and proposed close. Do not stop at
a plan, receipt or initial suspicious grep. No product repairs, feature work,
arc02, held projects, new migration, release, deployment or product-provider
requests. All new customer examples use Acme Inc. Source/index/existing build
outputs stay protected. No temporary edits to product files to make a probe work.

During this assignment you are the sole planning-file/index mutation owner.
Owned paths are exactly: existing project05-discoverability/artifacts/
2026.11.21-audit-index.md and 2026.11.21-audit-coverage.json; one actual Chicago
creation-date YYYY.MM.DD-audit-results-rust.md and YYYY.MM.DD-core-auth-evidence.md;
if needed actual-date core-auth-probe.rs, core-auth-probe.toml and core-auth-run.log
in that SAME artifacts directory; this slice ledger.md and closing-report.md.
No other planning/arc/project/README/source files, no issued-prompt edits. Report
exact new names and isolated temporary build tree ownership. No workbench or
per-slice artifacts. Preserve unknown outputs. CDC is read-only until return.

## Required reading and intake before semantic work

Read this prompt, slice plan/ledger, current planning AGENTS, project plan/ledger,
arc plan/ledger, accepted slice01 CDC verification/retro and current audit index.
Query all 38 path rows owned by slice02, full package/target/features for its seven
crates, and all relevant cross-consumer edges from coverage JSON. Do not dump
unrelated full structured data. Full normal text must actually be read; hashes,
counts, grep summaries, manifests or an attestation do not replace text intake.

Installed skill root:
/Users/oubiwann/Library/Application Support/ChatGPT-Profiles/personal/codex-home/skills
Read the full collaboration-framework posture set if not already loaded:
knowledge/collaboration-framework/guides/01-posture-and-ethics.md,
02-structural-pulls.md and03-collaborative-rights.md under its skill directory.
Read agent-coordination/guides/04-anti-patterns.md if not already loaded.
Retain the same-session full collaboration-framework/code-auditing guide set,
work-verification, coordination and testing/intake contracts already loaded for
slice01; reload relevant uncertain sections. Read engineering-methods/SKILL.md,
full governing AUM/shared-invariant sections of guides/01-engineering-methodology.md,
project-management wayfinder plus planning/closing contracts and methodology-
stewardship SKILL/retrospective guide if not yet loaded. No new model/budget gate.

MANDATORY before Rust semantic audit: read rust-guidelines/SKILL.md completely,
then full guides/11-anti-patterns.md FIRST; then every other existing Markdown
guide under rust-guidelines/guides. Inventory is 36 files: root chapters01–13,
16–17 plus README and MERGE_REPORT; CLI14 README and files01–08; Cargo15 README,
CHECKPOINT and files01–08. Include complete CLI/Cargo collections, not only the
currently most relevant chapter. The CLI index's missing09-common-pitfalls file
is absent in the actual inventory: disclose, do not pretend to read or clone it.
Use bounded contiguous full-text chunks (roughly 300–400 lines), recover actual
truncations and continue through compaction with useful source-cited notes. Do
not silently shrink the corpus. Preserve actual read ranges/coverage and applied
rule notes in core-auth evidence; full reading is distinct from semantic source
coverage. If a concrete input/context conflict prevents a required read, state
the exact conflict and remaining unread text early; size alone grants no waiver.

After intake, give source-cited contract readback BEFORE dependent edits/probes:
explain the complete population, source/result boundary, all seven scales,
strongest invalidators and planned discriminating oracles. Preserve useful
intake through later same-session slices; do not reread the entire corpus solely
for a new slice name. Reconsult affected rules as needed. Guide examples can be
outdated or flawed: exact source/locked dependency and primary guidance control.

## Full source population and method

Use all 38 map rows for slice02-core-auth: every tracked body/test/doc/manifest in
fabryk-core, fabryk-auth, fabryk-auth-google, fabryk-mcp-auth, fabryk-acl,
fabryk-redis and fabryk-gcp. Read complete files in bounded chunks. Include
correct/negative results and every file's actual disposition. Trace consumers
for function, file/module, logical unit, crate/target, application/service,
whole-codebase and workspace/system boundaries; keep broader composition pending
where another pass owns it. Identify shared root causes rather than count the
same helper defect again at each consumer. Hunt all categories in arc plan,
including competing abstractions, incomplete behavior, ownership, unsafe/error/
async/resource/secret handling and documentation promises. Source digests/module
scans are navigation only. Identity is not authorization; ACL is an explicitly
deferred placeholder and project06 is held, so do not invent its implementation.

CDC inspected all 38 paths and the following real interaction points. They are
leads and methods, not preaccepted findings. Challenge assumptions; a documented
caller precondition or intended deferred contract is not automatically a defect.

1. File boundary: fabryk-core/src/util/files.rs:71/158 implements pattern/simple/
   recursive lookup. Inspect absolute IDs, `../`, symlink escape, directory matches,
   ambiguous prefixes, max_depth and I/O-error behavior. Concrete fixture: an
   owned root with content/public.md and sibling outside.md, plus a directory
   named directory.md; call actual find_file_by_id and provider get_item/get_guide
   in fabryk-mcp-content (get_item:216, get_guide:167). Verify returned/read path
   and content, not a recreated algorithm. A containment/type finding needs actual
   controlling-source and entrypoint evidence; disclose whether MCP dispatch/client
   was exercised. Do not claim an unauthenticated network exploit from a helper.
2. Lifecycle: core service.rs:102/260/300, slot.rs:86, orchestration/probe/state.
   Probe max_attempts0, exhausted failures, successful retry, panic/abort and
   wait_ready timeout/failure. Check state versus backend storage, transitions,
   lock poisoning/order, cancellation, backend replacement and actual fallback
   consumer search_fallback.rs. Observe terminal state after JoinHandle completion;
   distinguish caller misuse/explicit assumptions from a violated guarantee.
   Redundant unsafe Send/Sync in state.rs:169–170 needs bound/soundness inspection,
   not an automatic UB assertion. Verify ConfigProvider/ConfigManager ownership.
3. HTTP identity: auth/middleware.rs:93/144 and AuthError::is_client_error. Use
   actual AuthLayer/ServiceExt oneshot with a validator that yields a known valid
   user, invalid token and JwksFetchError. Observe actual status/body/challenge and
   whether inner service is called; distinguish dependency outage from client
   invalidity, disabled dev mode, missing/empty tokens and valid scheme casing.
   Inspect identity propagation and hardcoded configuration namespaces. Compare
   documented behavior before grading interoperability or fail-open claims.
4. Google validation: auth-google/lib.rs:99/163/227/253 and public TokenValidator.
   Trace audience/issuer/domain/email verification for BOTH JWT and opaque token
   paths. Do not obtain or send real tokens or provider requests. A local owned
   JWKS fixture can exercise public validate with validly formed test JWT headers
   and unknown kids, counting refresh calls under fresh cache and stalled/non-2xx
   response behavior. Use an outer bound solely to keep a stalled probe finite;
   report internal deadline absence separately from measured external cutoff.
   Verify exact cached reqwest0.13.5 async client default timeout (client.rs:1444–
   1451 states none), not blocking-client defaults. Opaque validation hardcodes
   userinfo URL, so unsupported live claims stay unavailable/static with precise
   controlling-source evidence; do not alter product endpoints to fake proof.
   Primary Google guidance: developers.google.com/identity/openid-connect/
   openid-connect and identity/sign-in/web/backend-auth; RFC9700 audience checks
   where relevant. Check exact claims before conclusions; no speculative severity.
5. Discovery: mcp-auth/lib.rs:27/70/86 promises configurable routes. Invoke actual
   returned Router through oneshot for BOTH root and /mcp metadata paths using
   an Acme test issuer/resource URL; compare issuer and endpoint fields. Probe
   trailing slash/path handling and challenge/discovery composition. No external
   OAuth service calls; do not confuse resource metadata and authorization server
   metadata or promise arbitrary-provider support without source-contract evidence.
6. Redis: compare actual RedisOps trait/RedisClient vs MockRedis. Probe invalid
   numeric strings, signed/overflow boundaries, missing keys, glob metacharacters
   and MGET/JSON parse failure policy. If installed, an owned loopback Redis with
   ephemeral port/no persistence may provide production evidence; otherwise use
   exact redis1.8.2 primary-source/server protocol evidence and label runtime gaps.
   Never present mock-only success as Redis proof. Preserve intentionally skipped
   missing/parse entries versus undocumented failures; no new external service.
7. Config/utilities: inspect ADC path precedence/file-versus-directory/malformed/
   unsupported credential types, and real consumers without reading real credential
   bytes. Spawn owned processes with task-specific temporary environment for
   resolution probes; do not mutate global HOME or system options. Inspect ID/time/
   error-chain semantics and duplicate config/identity abstractions with exact
   consumers. No generic style quota or speculative performance claim.

These recipes do not replace complete review of the remaining code in every
assigned file. At least five concrete meaningful negative checks belong in the
Rust language report, with actual method/result and contrary evidence. There is
no finding quota; retain sound behavior and label hypotheses separately.

## Evidence, reports and closure

Begin the cumulative actual-date audit-results-rust.md. Confirmed IDs start
FA-RS-001 and never get reused; hypotheses use H-RS- identifiers and are not in
confirmed totals. Every finding: severity Blocker/High/Medium/Low, exact 719
file:line, affected scale, observed failure mode, evidence strength and actual
entrypoint, fix direction, compatibility-sensitive decisions and validation oracle.
No source repair or repair acceptance. Source-only architectural/control-flow
proof can be explicit static evidence; runtime claims require actual public/
protocol entrypoint observations. Historical/arc00 results retain original dates,
source pins and limitations; source 719 unchanged does not make them fresh runs.

Keep full commands, actual exits and raw diagnostic output in the owned evidence/
run log, including failures/retries. No false tee exit, weakened test, skip added
or blanket warning suppression. Isolated probes use exact pinned product paths
and locked dependencies where possible. Record any isolated Cargo.lock/resolution,
features/toolchain divergence and compare it against product Cargo.lock; never
label scratch resolution an exact locked-product run. Prefer existing applicable
cached builds; no redundant full all-feature/native suite. Name and own temporary
build outputs; do not mutate existing source target/cache artifacts. If a native
check is unavailable, record exact command/blocker/re-entry condition honestly.

Update only the 38 per-path coverage dispositions with actual methods/evidence;
preserve all other path rows/digests/owners, map counts and feature inventories.
Pass01 scale cells may record bounded reviewed evidence; wider final composition
remains pending. Index carries findings/coverage/limitations and exact report links.
Evidence must include complete 38-path receipt and per-scale assessment so CDC
can reproduce coverage. Preserve consequential questions for CDC; do not expand
authority to repair, live-provider requests or ChatGPT usability trials.

Before return, self-review source/diff/evidence and walk all five CORE rows in
closing-report.md with attested dispositions, exact assignment/source/planning
state, actual output inventory, scope-as-specified/delivered, uncovered cases and
bubble-up to arc. Do not edit CDC acceptance or open later slices. Stage/commit
ONLY exact owned paths. Verify actual message/trailers before commit:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

No amend/squash/fixup/ordinary merge/push/tag. Return actual commit, confirmed
findings/hypotheses/limits, checks/exits and exact artifacts, no active mutable
process, and explicit planning/index ownership return. CDC independently reviews
actual source/probes, not just this return. Default five refinements; none used.
