# Planning location

Planning artifacts live on orphan branch `planning`, worktree
`.worktrees/planning`, under `projectNN-<slug>/`, following the collaboration
framework. See `.worktrees/planning/README.md` for the release/project map and
`migration-map.json` there for original paths, source history, timestamps,
duplicate dispositions and exact split locations.

Projects 01–04 reconstruct release lines 0.1–0.4. Their historical claims and
checkboxes are not independently verified acceptance. Missing implementation
plans, ledgers and verification records were not invented. Preserved historical
prompts are evidence, not current instructions.

Release checkouts are `.worktrees/release-0.N.x`. The post-0.4 planning sources are now organized under projects05–08.
Project05 targets 0.5.0 after a full audit and repair gate; project06/07
implementation is held and project08 is deferred. Audit results belong in
project05-discoverability/artifacts/. For new slice-produced supporting
artifacts, use the owning slice's `artifacts/` directory unless explicitly
agreed otherwise.

The original agent-workflow framework is cancelled in favor of project06.
Current concept-card development lives in ai-engineering. Sygnum belongs to
project07 and Medys to project08. Customer examples must use Acme Inc. and
suitable anonymized code/path identifiers; do not reintroduce private names.

## Git integration and commit attribution

Integrate branches only with commit-preserving rebase or `git merge --ff-only`.
Do not create ordinary merge commits or use squash merges, rebase squash/fixup,
autosquash or commit-dropping operations to collapse work history. Keep the
orphan planning branch separate from implementation branches.

Every future assistant-authored commit, on implementation or planning branches,
must include both trailers, separated from the message body by a blank line:

```text
Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>
```

## Current prerequisite sequence — 2026-11-21

Duncan accepted the completed sanitization and deferred the remaining hosted
purge as an open, nonblocking task. Do not delete or recreate any repository,
submit further GitHub removal operations or claim the purge is complete.
The Codeberg request was sent; its actual outcome remains pending.
Continue the requested integration and release-branch pushes. Before any full
audit or feature coding, complete project05/arc00: fresh green CI, latest stable
external dependencies, then green checks again, using the selected Autonomous
Mode roles (CDC 6.1 Sol High; CC 6 Luna High). Follow the current canonical plans;
no project06/07 feature implementation or 0.5.0 publication is authorized here.
