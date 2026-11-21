# Privacy rewrite record — 2026-11-21

## Status and authority

Duncan explicitly required anonymization of a private customer throughout source,
fixtures, documents, filenames and Git history, including overwriting affected
refs on every configured remote. Acme Inc. is the prose replacement; code and
path identifiers use anonymized forms. The private original is intentionally
not repeated in this record.

Current status: local rewrite, remote ref replacement and macpro host-side purge
completed; GitHub and Codeberg host-side object/cache purge remains pending. This is privacy-cleanup evidence, not the
planned full code/architecture audit, an independent acceptance, or a 0.5.0 release.

## Preparation and preservation boundaries

Prepared an isolated mirror, fetched all advertised remote refs, and checked
clean ownership of all seven local worktrees. No unsanitized archive or backup
branch is retained. The verified rewrite changes private-name payload/path
occurrences and associated Git identities. Exact tree comparisons against the
anonymization covered all 30 original local refs and 1,219 distinct source blobs,
including the Codex snapshot tree refs. Read-only PR refs were separately checked:
their original reachable objects contained no private-name matches.

All seven local worktrees were refreshed from the rewritten refs. The ignored
first-party reference list was also anonymized. The import/source commit IDs,
migration headers, hashes and sizes were reconciled against sanitized history.
All 138 selected original documents still have complete source-line coverage.
The original 214-source inventory and 76 preferred-version decisions remain
accounted for. Pre-rewrite byte-preservation claims are superseded by the explicit
privacy sanitization; historical mtimes remain capture metadata.

## Remote ref replacement and fresh verification

Updates used exact old-ref leases and atomic transactions per remote, with
successful dry-runs first. Existing advertised branches/tags were replaced;
no new planning/release branches were published and no source cleanup merge or
0.5.0 release was performed. Remote baselines were preserved independently:
Codeberg/macpro did not receive unrelated later source commits.

| Remote | Existing refs replaced | Advertised refs read back | Result |
| --- | ---: | ---: | --- |
| github | 10 | 20 | Exact expected identities matched |
| codeberg | 6 | 12 | Exact expected identities matched |
| macpro | 8 | 16 | Exact expected identities matched |

Freshly fetched object populations across all remotes: 270 commit objects, 1729 tree objects, 1649 blob objects, 6 tag objects. No private-name matches were found in those fetched objects. This verifies advertised history, not unadvertised host storage.

## Confirmed host-side residue — still open

After the successful rewrite, a bounded fetch-by-old-hash probe recovered one
known old main commit from each host. Each recovered tree still contained 11
matching files. All disposable probe repositories were destroyed immediately;
no old sensitive payload was retained as a new archive.

At the initial probe, GitHub, Codeberg and macpro still needed host-side
object/cache cleanup. The macpro follow-up below supersedes its initial result.
A normal fresh clone is clean, but the complete privacy-removal requirement is
not closed. The configured macpro endpoint is a restricted Git service and
rejected shell-style maintenance commands; its admin route is requested from
Duncan. Do not claim that force-pushing alone erased host-retained objects.
No support messages or requests to other people have been sent.

Provider guidance:

- [GitHub sensitive-data removal](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository): cached views and retained references may require provider-side cleanup.
- [Codeberg history replacement and collection](https://docs.codeberg.org/git/reduce-size/): collection is a separate host operation, normally run periodically.

The next owner is Duncan with the required Git-service admin/provider access.
Re-entry is a host-side purge followed by repeating the old-hash and fresh-fetch
checks. No repository deletion, account-wide maintenance, support message, or
changes to other clones are authorized by this record. Existing clones must be
recloned or carefully moved to the rewritten history before they push again.

## Focused implementation validation

`cargo test -p ecl-stages -p ecl-pipeline-spec` passed: 50 configuration/spec tests
and 142 stage tests; 192 total, zero failures. This includes anonymized regex and
fixture cases. The first offline attempts were blocked by missing cached crates;
the online rerun fetched dependencies and passed without changing Cargo.lock.
The full workspace audit has not started and no broader test-coverage claim is made.

The configured GitHub workflow was inspected before publication: pushes trigger
build, lint and test jobs; no release/publishing job was present. Remote CI outcomes
are not claimed by these local test results.

## Hashes and signatures

History rewriting changed commit/tag identities. The translation map is preserved
in `migration-map.json` and the local `.git/filter-repo/commit-map`; it contains
identities rather than sensitive payloads. Historical signatures were not re-signed.

Observed signature metadata: {"old_signed_commits": 3, "rewritten_signed_commits": 0}.

## Local object purge

Local reflogs and obsolete objects are to be expired/pruned after committing this
record, followed by a scan of every remaining object. The final result is appended
here after the purge; this statement does not itself claim completion.

Final local purge check: reflogs expired and obsolete objects pruned. Scanned every remaining local Git object: 270 commit, 6 tag, 2049 tree, 2225 blob; zero private-name matches. First-party live files/paths across all worktrees also had zero matches. Release-tip relationships, clean release worktrees and orphan planning ancestry passed. These are author self-checks.

## Concrete host-purge targets

The following known old commits were still recoverable at the retention check. The current refs are clean; old identities below are purge targets, not backup refs. Complete before/after ref identities are recorded under `history_rewrite.remote_ref_updates` in the source map.

| Host | Old main commit confirmed recoverable | Current main |
| --- | --- | --- |
| github | `f19bb3b9bc00295dea8eabb40247b6eb9e6fabdf` | `856af87561c7e53379ec9094c69a01211d3e5283` |
| codeberg | `7a9dead8b608d959d592d393e62094872a4e2a8e` | `87af29b820e495afe2dbe1b0ad5ce5173d59fa43` |
| macpro | `7a9dead8b608d959d592d393e62094872a4e2a8e` | `87af29b820e495afe2dbe1b0ad5ce5173d59fa43` |

## Completed macpro follow-up

The port-23231 endpoint is Soft Serve, which has its own SSH command interface.
Normal host SSH on port 22 was available. The running service exposed its data
location at `/Users/oubiwann/lab/.git-server/data`; the exact bare repository was
`repos/oxur/textrynum.git` beneath it. No web admin interface was required.

Verified the intended bare repository and expected rewritten main, checked for
active lock files, expired its reflogs and ran `git gc --prune=now` only there.
All 10 refs were unchanged. Scanned every stored object: 259 commits, 5 tags,
1,720 trees and 1,646 blobs; zero private-name matches. A fresh old-hash fetch
through Soft Serve was rejected. Macpro's cleanup is complete by these checks.
The old-hash probes against GitHub and Codeberg still recovered sensitive content.

## Operator hold and pending integration instructions

Duncan explicitly selected **wait for all hosted purges** before applying the
AGENTS.md changes or fast-forwarding the source-cleanup branch. Do not treat clean
advertised refs, completed macpro maintenance, or a provider acknowledgement as
proof that the two remaining hosted purges have completed. Repeat the old-object
checks and inspect the relevant cached views after provider cleanup.

When that gate passes, in this order:

1. Update both source and planning AGENTS.md with the policies below.
2. Fast-forward `codex/planning-migration` into `release/0.5.x` using `--ff-only`.
   Keep the orphan `planning` branch separate; never integrate it into source.
3. Push `release/0.1.x` through `release/0.5.x` to github, codeberg and macpro.
   Recheck remote state, preserve concurrent work, and use normal pushes where
   possible. No further privacy-history rewrite is planned or authorized here.

Pending integration policy:

> Integrate branches only with commit-preserving rebase or `git merge --ff-only`.
> Do not create ordinary merge commits or use squash merges, rebase squash/fixup,
> autosquash, or commit-dropping operations to collapse work history.

Pending commit-attribution policy, copied from Cayrn's convention and adopted
by Duncan on 2026-11-21: every future assistant-authored commit on source or
planning branches must include both trailers, separated from the body by a blank
line. The requested file edit remains held; subsequent commits already follow
the explicit user instruction.

```text
Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>
```

Private provider-purge request drafts are in
[host-purge-requests.md](host-purge-requests.md). They have not been sent; explicit
approval is required before messaging the providers.
