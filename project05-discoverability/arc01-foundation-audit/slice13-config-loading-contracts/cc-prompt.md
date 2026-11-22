# Slice13 — configuration loading contracts

Read loader options and current public signatures, map caller responsibilities,
and compare exact configuration-guide entrypoints. Produce one small static
assessment; return ownership after the scoped commit. Source719 is immutable.
Initial assignment0/5 refinements; slice03 remains2/5used/3remaining.

## Terminology and stop contract

Use accurate ordinary software engineering terms in new prose/prompts—“code quality assessment”, “behavioral check”, or “API consistency review”—when equally precise. Keep necessary technical facts, exact code identifiers and historical records unchanged. Do not construct keyword blacklists, obscure or relabel the work, or use wording/context/model changes to evade platform restrictions; we do not know which words triggered prior flags. If a task is platform-flagged, stop that affected work and report its evidence without variants or retries.

Propagate this paragraph to any later authorized contributor assignment.

## Roles, baseline and ownership

Same retained CC gpt-6-luna High session01a2019f-73de-7de1-831f-53d55b3da247;
CDC gpt-6.1-sol High independently reviews. Requested model/reasoning receipt was
already checked in your own-session metadata; acknowledge retained receipt, do
not repeat a metadata hunt. Hidden backend identity is unreported. Disclose an
unavailable retained session; no silent model substitution.

Source /Users/oubiwann/lab/oxur/textrynum is read-only, including index, at
719c58f245fe4a9115c7c0391d3e268c99157e2f on codex/foundation-audit.
Planning /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning is orphanplanning.
Use ABSOLUTE paths for ALL reads/writes and physical planning cwd/Git -C for
mutations; retained tool cwd may still be source despite launch --cd.

You own ONLY these five paths:
1. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-config-loading-contracts.md
2. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md
3. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md
4. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice13-config-loading-contracts/ledger.md
5. /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice13-config-loading-contracts/closing-report.md

Source inspection and scoped Markdown bookkeeping only. Keep all existing report
bytes and coverageJSON unchanged; append only this result and one index note.
Keep all SEARCH records/partial files untouched: no inspection/reconciliation/
execution or reassignment of that interrupted driver. No Cargo, build, test,
runtime, provider/service, repair, CI edit, push, publication, Daybreak or external
message. Parent already submitted feedback; do not resubmit. Whole-arc scope and
all pending gates remain intact.

## Required intake and author readback

Read current arc-plan/ledger, this slice plan/ledger/prompt and applicable current
AGENTS/project instructions. Reuse completed unchanged framework, PM wayfinder/
operation guides, Autonomous contract, work-verification, full code-auditing set,
engineering prompt/intake and methodology-stewardship. Retain complete Rust skill
and36existing guides, anti-patterns first, including API/ownership/Cargo/rustdoc;
missing CLI09 is a documented gap. Re-read uncertain relevant rules as needed.

Read COMPLETE these source files using absolute source paths:
- crates/fabryk-cli/src/config_loader.rs208lines
- crates/fabryk-cli/src/config.rs496lines
- crates/fabryk-core/src/traits.rs330lines
- docs/howtos/mcp-configuration.md353lines
Four files/1387lines. Follow exact direct configuration traces app.rs1–42/75–106,
config_handlers.rs1–175 and cli.rs80–130 in fabryk-cli; compare building-a-fabryk-
mcp-server.md193–198 as a current signature contrary example. Cite actual ranges;
range-only traces are not full-file semantic coverage.

Before dependent edits provide compact source-cited readback: population, relevant
Rust ownership/API rules, methods/options/responsibilities, exact output limits,
negative controls and retained terminology/stop contract. No hash-only substitute
for actual reading.

## Concrete source-guided work

1. Map all builder methods and fields: prefix/section consumption, resolver input
   and returned path, build ownership and returned config, port registration and
   helper. Map all ConfigManager method signatures and concrete FabrykConfig
   delegation/loading/path precedence. Record duplication by responsibility and
   policy, not a demand to consolidate intentional abstraction boundaries.
2. Calibrate port_env_override: build takes self and does not read port_env_var;
   check_port_override borrows self and says call after build. However63–70
   explicitly makes application caller-owned, checking BEFORE build then applying
   afterward is possible, and the guide118–123 independently reads/applies PORT.
   Guide313–314 implies registration supplies the override. Separate an actual
   option/call-order/documentation inconsistency from claims that every caller
   cannot apply PORT. No environment mutation or execution is needed/authorized.
3. Compare guide115/140 resolve_config_path() and214 Config::load() with shown
   trait signatures and actual definitions. Account for any shown inherent helper
   instead of inventing one; distinguish incomplete illustrative context from
   contradicted advertised entrypoints. Read correct concrete delegations and the
   other guide's load(config_path) example as contrary controls. Do not claim an
   actual compile error was executed.
4. FabrykCli<C:ConfigProvider> config dispatch is concrete FabrykConfig; handlers
   explicitly document that generic consumers call cmd_config_* directly. This
   intentional policy and ConfigProvider versus ConfigManager bounds are contrary
   evidence, not automatically a defect. Record any remaining design question
   separately. Formatting/export roundtrips, general CLI placeholders, path-
   utility internals and every other subsystem are outside this bounded scope.

## Outputs, preservation and handback

New companion artifact: full read receipt, source-pinned option/method/call matrix,
manual contract interpretation, at least five concrete negative checks, questions,
static bookkeeping commands/exits and limitations. Detailed new findings belong
in one appended Rust report section, sequential after035. Every supported entry
needs stable ID, severity/rationale, pinned file:line, Scale, observed static
mechanism/consumer impact, confidence, logical repair owner, issue-specific fix/
compatibility direction and discriminating oracle. Zero findings is valid.
CC ledger/close mark author-proposed, not independently accepted.

Coverage units are configuration responsibilities and documented calls, not four
whole-file owners or a full pass. Do not change360path owners/statuses,49cells or
pending SEARCH criteria. Preserve accepted001–010/034–035 and all011–033 bytes.
All results directly project05/artifacts; no per-slice result directory.

Before staging screen exact five owned paths/content in memory with Python:

```python
import re
pattern = re.compile(bytes([87,97,108,103,114,101,101,110,115]), re.I)
```

Output COUNTS ONLY; Acme Inc. examples only. Check source719/clean state, exact
owned diff, prior report/index prefixes and coverageJSON/old partial bytes.
Reconcile actual methods/IDs/status/source links, not cosmetic prose assertions.
No extra executable artifact or passing baseline rerun for this static task.

Ordinary commit ONLY five owned paths with actual trailers after blank body:

Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>

No amend/squash/fixup/drop/merge/push. Verify actual paths/trailers; return commit,
source-backed results/questions/limits and explicit five-path/planning-index
ownership handback with no active mutable command. CDC independently reviews.
