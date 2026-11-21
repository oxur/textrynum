# Prompts

## CD Prompt - Generate Milestone Docs

I'd like for us to generate Phase N implementation prompts (as .md artefacts) for CC (Claude Code) -- one for each milestone. We can address any discrepancies as we get to a milestone that deals directly with those.

You should first (re)read the the following audit sections:

- §4.1: fabryk-core extraction details
- §7: migration steps

so that the prompts are grounded in the full specifics. But be sure to cross-reference those against the newer docs that may provide corrections or additional context.

---

## CC Prep - odm add

`for FILE in $(find workbench/*.md); do odm add --dev --subdir fabryk $FILE; done`

---

## CC Dev Plan Review

Your current working directory should be `~/lab/oxur/ecl`. If it isn't, you need to make sure you `cd` there now.

Claude Desktop (CD) has built a series of prompts for you to work on in this next phase of development. Here are the documents he used to come up with these plans:

| # | Document | Filename | ECL No. | Music Theory No. | Why It's Needed |
|---|----------|----------|---------|------------------|-----------------|
| 1 | **The bootstrap doc** | `./crates/design/dev/0012-fabryk-extraction-session-bootstrap-phases-1-7.md` | dev 0012 | dev 0018 | The prompt and orientation |
| 2 | Project Plan Overview | `./crates/design/docs/05-active/0013-fabryk-extraction-project-plan-overview.md` | 0013 | 0013 | Milestone breakdown for all phases |
| 3 | **Extraction Audit** | `./crates/design/docs/06-final/0011-fabryk-extraction-audit-music-theory-mcp-server.md` | 0011 | 0009 | File-level inventory, classifications, trait designs, migration phases |
| 4 | **Audit Amendment** | `./crates/design/docs/06-final/0012-fabryk-extraction-audit-amendment-refinements.md` | 0012 | 0012 | Six refinements that override parts of the audit |
| 5 | **Unified Ecosystem Vision v2** | `./crates/design/docs/01-draft/0009-unified-ecosystem-vision-ecl-fabryk-skill-framework.md` | 0009 | NA | Architecture, crate structure, component responsibilities |
| 6 | **Git-tracked files** | `git ls-files` (executed in `./` and `./workbench/music-theory-mcp-server`; see below, for notes on the latter) | NA | NA | Current codebase file listing |

CD has done an amazing job pulling together a big picture and high-level planning for this effort. I have reviewed his plans carefully, and the chance of us needing to make significant changes, is low; we may not, in fact, need to make any changes to the big picture at all.

However, unlike CD, you have something else special: access to all the files of both repos in questions _and_ their git history. Even more importantly, though, you have access to the AI Rust `SKILL.md` I've created for you.

You are going to be not only evaluating the technical merits of the low-level details of each phase, its milestones, and how they connect to each other; you're also going to be evalutating the Rust examples on how closely they adhere to the Rust best practices. Therefore, you will be responsible for ensuring the code truly adheres to the highest levels of excellence. You will also be responsible for filling in gaps (sometimes considerable gaps) that CD was unable to provide direcotion for.

In preparation for this, please read `CLAUDE.md` and follow the path to the `SKILL.md` mention in that file, noting that the path `assets/ai/ai-rust` is a symlink, and you will need to use a trailing `/` on it to see the contents of the linked directory, i.e., `assets/ai/ai-rust/`.

As a convenience for you, I have provided a symlink to the music theory MCP server code mentioned in the above docs:

`./workbench/music-theory-mcp-server`

I created that with the following:

```bash
$ cd ./workbench
$ ln -s ../../../music-comp/ai-music-theory/mcp-server \
    music-theory-mcp-server
```

Or, more clearly, the MCP server code lives here:

```
~/lab/music-comp/ai-music-theory/mcp-server
```

To access the contents of the direcotory via the symlink, you will need to use a trailing `/`, i.e. `./workbench/music-theory-mcp-server/`. You will be making changes both in that repo as well as the repo here, at `~/lab/oxur/ecl`. You will need to track that work VERY carefully, not mixing up the two.

The milestone documents you be will be reviewing are here:

```
crates/design/dev/fabryk/0001-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0002-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0003-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0004-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0005-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0006-cc-prompt-fabryk-1.md
crates/design/dev/fabryk/0007-cc-prompt-fabryk-1.md
```

You will process these in order, one at a time. After analysing each one, you will pause and share your findings with me, asking any questions you have, discussing options with me. Once we have discussed to our mutual satisfaction, you will make recommendations for edits to the document in question. Once approved, you will apply the agreed upon edits and commit the changes to the document in question.

---

## Duncan's instructions for cleanup

### Phase 1, Milestone 1

1. Agreed, the document should be rewritten to use Option 2 (within ECL workspace). Please do the following:
    - Update all paths to use ~/lab/oxur/ecl/crates/fabryk-*
    - We will keep the following old stubs: fabryk, fabryk-acl, fabryk-cli, fabryk-core, fabryk-mcp
    - We will delete the following old stubs: fabryk-storage, fabryk-query, fabryk-api, fabryk-client
    - Add the missing crates: fabryk-content, fabryk-fts, fabryk-graph, fabryk-mcp-content, fabryk-mcp-fts, fabryk-mcp-graph
2. Use the same values as ECL:
    edition = "2021"
    rust-version = "1.75"
3. For dependencies, we want to use the most recent verions for all deps, as long as they don't cause conflicts with other deps. For now, you will leave the deps versions as they are in music-theory. Once we have ensured that the moved code is working, we will then update all deps, one at a time, fixing issues encountered as we go. If music-theory uses more than one version, we will use the most recent of the two.
4. Agreed, as stipulated in #1. Follow these explictiy steps:
    1. Deprecate/remove old stubs that don't fit the extraction plan
    2. Gut existing stubs (fabryk-core, fabryk-acl, fabryk-mcp, fabryk-cli) to match extraction requirements
    3. Create new stubs for missing crates
5. Update the doc to add missing deps.
6. Agree 100% -- please fix all:
    1. Feature naming: The document uses tantivy and rkyv-cache as feature names. Per best practices, consider more
    descriptive names like fts-tantivy and graph-rkyv-cache to avoid confusion with the crate names.
    2. lib.rs stubs: The document says "minimal src/lib.rs stub per crate" — should specify the stub should include #![doc =
    include_str!("../README.md")] pattern if READMEs are created.
    3. Error handling: The document doesn't mention that fabryk-core should define a proper Error enum and Result type alias
    from day one.
7. More questions:
    1. Umbrella crate fabryk: Should this be kept as a re-export convenience crate? Yes, keep it.
    2. Old stub cleanup: Should we create a separate milestone 1.0 for cleaning up old stubs? Yes!
    3. Feature flag naming: Prefer more explicit fts-tantivy and
    graph-rkyv-cache, per best practices!
    4. ECL workspace version: Should we update version = "0.0.1" to version = "0.1.0-alpha.0" to align with the Fabryk
    versioning plan? Yes!

### Phase 1, Milestone 2, 3

Agree 100%, do as recommended.

---

## CC Prompt

You've done a great job reviewing the docs and proposing changes -- You ready to get started on Phase 1, Milestone ??!?!!

First step, brush up on `CLAUDE.md`, the referenced `SKILL.md` and various guides. For each document you work on, check the guides to see which best practices you should read up on.

Once you've caught up on the Rust expertise, go ahead and (re)read `crates/design/dev/fabryk/0001-cc-prompt-fabryk-1.md`, following its instructions, and making the necessary changes in the repos as indicated.
