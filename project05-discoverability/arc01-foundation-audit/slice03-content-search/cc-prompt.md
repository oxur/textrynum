# CC assignment — slice03 complete content/search diagnosis

Complete this bounded pass autonomously, then return for independent CDC review.
Do not stop after intake/plan. User-authorized endpoint remains full acceptedarc01
foundation diagnosis and actionable repair handoff, not repairs or wholeproject.
This slice follows acceptedcore/authfd496fa;0of5 semantic refinements used here.

## Role, current pin and exact receipt

Unchanged selected CC gpt-6-luna High through localCodexCLI, session
01a2019f-73de-7de1-831f-53d55b3da247. CDC is independent gpt-6.1-sol High.
Coordinator verified your own-session turn_context at:
/Users/oubiwann/Library/Application Support/ChatGPT-Profiles/personal/codex-home/sessions/2026/11/21/rollout-2026-11-21T22-59-17-01a2019f-73de-7de1-831f-53d55b3da247.jsonl
It records requestedmodelgpt-6-luna/efforthigh, physicalsourcecwd, on-request and
workspace-write with planning writable. Acknowledge receipt; hiddenbackend remains
unreported. Do not infer mismatch from generic self-description or create another
metadata-location/model-selection gate. No silent substitution.

Source physical /Users/oubiwann/lab/oxur/textrynum, codex/foundation-audit at exact
719c58f245fe4a9115c7c0391d3e268c99157e2f. Tracked product bytes/index protected;
sourceclean at handoff. Planning physical
/Users/oubiwann/lab/oxur/textrynum/.worktrees/planning on orphanplanning. All audit
RESULTS directly project05-discoverability/artifacts/, no workbench/sliceartifacts.
Use ABSOLUTE physical paths for every output/workingdirectory. Preserve allunknown
outputs. No source changes/staging/commit, fixes, feature work, liveprovider call,
external messages, release/publication/deployment or held06/07/deferred08 execution.

## Required reading and source-cited readback before probes

Read completely current README/AGENTS, project05project-plan/ledger, arc01arc-plan/
ledger, thissliceplan/ledger, cumulativeindex/coverageJSON (scopequeries forinventory,
not unrelateddumps), accepted slice02CDCverification/retro and currentRustreport.
Retain the already completed fullRustskill+all36existingguides/anti-patternsfirst,
complete code-auditing set and collaboration/project/AUM/intake/prompt-authoring/
verification/testing/stewardship contracts from thissame session. Re-read relevant
uncertain rules when applying them; do not rerun fullcorpusintake merely due to a
new slice name. AbsentCLI09common-pitfalls remains a realgap, not an inventedread.

Read ALL31 assigned files below, every implementation/test body and doc/manifest.
Parse the completeJSON and validate allterms/uniqueness/types/licenseprovenance;
no need dump1298terms or repeat arc00passing1298-term runtimeoracle absentchange.
Trace real consumers for cross-scale contracts; they do not enter this denominator
or close their later owners. Give a compactreadback citing exactsource/rules and
actualAPI/oracles, read-manifest ranges, scope/pin/ownership/outputpaths before
runningdiagnostic probes. Receipt/hashes/mapping alone are notsemantic coverage.

## Complete population

- crates/fabryk-content/Cargo.toml (37 lines)
- crates/fabryk-content/README.md (17 lines)
- crates/fabryk-content/src/concept_card.rs (238 lines)
- crates/fabryk-content/src/lib.rs (67 lines)
- crates/fabryk-content/src/markdown/frontmatter.rs (481 lines)
- crates/fabryk-content/src/markdown/helpers.rs (599 lines)
- crates/fabryk-content/src/markdown/mod.rs (48 lines)
- crates/fabryk-content/src/markdown/parser.rs (416 lines)
- crates/fabryk-content/src/metadata.rs (518 lines)
- crates/fabryk-content/src/sources/mod.rs (23 lines)
- crates/fabryk-content/src/sources/resolver.rs (367 lines)
- crates/fabryk-content/src/sources/scanner.rs (315 lines)
- crates/fabryk-content/src/sources/types.rs (295 lines)
- crates/fabryk-content/src/sources/validator.rs (429 lines)
- crates/fabryk-fts/Cargo.toml (43 lines)
- crates/fabryk-fts/README.md (56 lines)
- crates/fabryk-fts/STOPWORDS-LICENSE.md (57 lines)
- crates/fabryk-fts/src/backend.rs (686 lines)
- crates/fabryk-fts/src/builder.rs (1034 lines)
- crates/fabryk-fts/src/concept_card_extractor.rs (396 lines)
- crates/fabryk-fts/src/document.rs (576 lines)
- crates/fabryk-fts/src/freshness.rs (647 lines)
- crates/fabryk-fts/src/indexer.rs (299 lines)
- crates/fabryk-fts/src/lib.rs (206 lines)
- crates/fabryk-fts/src/probe.rs (133 lines)
- crates/fabryk-fts/src/query.rs (404 lines)
- crates/fabryk-fts/src/schema.rs (319 lines)
- crates/fabryk-fts/src/stopwords-en-iso.json (1 lines)
- crates/fabryk-fts/src/stopwords.rs (347 lines)
- crates/fabryk-fts/src/tantivy_search.rs (587 lines)
- crates/fabryk-fts/src/types.rs (261 lines)

## Domain-source investigations and discriminating falsifiers

These are source-grounded leads/questions, not preaccepted findings or a ceiling.
Read allother code too; clear/falsify unsupported leads explicitly. Use actual
public entrypoints and authoritative source contracts, not copied algorithms.

1. **Unicode/bounds and parsing.** markdown/parser.rs222–227 byte-slices a limit
   documented ascharacters. Exercise publicextract_first_paragraph on multibyte
   cut, zero/exact/ASCII controls; trace realFsGuideProvider max200consumer119.
   SearchDocument::extract_snippet/document.rs228–246 and Tantivy's near-duplicate
   snippet helper238+ adjust byteoffset after rfindwhitespace with +1; test an
   em-space boundary (e.g.foo\u{2003}bar baz/querybaz/limit4), non-ASCII case
   expansion (İ), empty/ASCII/longdescription controls. Distinguish truepanic,
   wrongexcerpt and intentionally unboundedmetadata. Frontmatter opener/closer
   prefix matching vs exactdelimiters, malformed/emptyYAML, headings/codefences:
   evaluate actual documented grammar, preserve explicitgracefulinvalid-YAML policy.
2. **Identity/metadata/provenance competing policies.** Compare publiccoreids
   normalize_id27 vs contenthelpers265 on repeatedwhitespace/tabs/underscores;
   trace real consumers ratherthan invent a missingproductionID. Compare actual
   extract_metadata andConceptCardExtractor on the same frontmatter-bearing body:
   heading fallback with nofrontmattertitle, categorypath depth, ContentType path
   conventions, source/author/date/chapter/section and aliases. metadataextractor
   usesfullcontent for fallback while FTSusesbody; check a validfm plus actual#title.
   Different intentionalextractor policies are not automatically defects. Preserve
   current readercompatibility, not a newconcept-cardmethodology/schema programme.
3. **Source resolution/validation.** SourceResolver::from_categories creates IDs
   via category-file concatenation; aliases/titles overwrite inHashMaps. Test two
   distinct definitions colliding as a-b/c versus a/b-c and ambiguousalias/title;
   inspect actualstableID/disposition, not onlysynthetichelperstrings. Public
   validate_sources modeConfigFilesystem still scanscontent first and computes
   sources_on_disk asconfigcount minusmissingfiles; controlledcollision/missing
   fixtures can discriminate underflow/countfailure. Cover everyValidationMode,
   missingcontentroot, valid/absent/directorysource and malformedcard skip policy.
   Do not demand held ingestion features or misgrade documentedskip behavior.
4. **Advertised query semantics/configuration.** query.rsbuild_query creates
   per-field Boolean groups, phrasesShould and mode-basedtermoccurrence; types
   documentsAnd/Or/Smart/MinimumMatch and configboosts/minimum_match_percent.
   Use realIndexer+TantivySearch (and appropriateQueryBuilder publiccontract)
   with a tiny discriminatingcorpus: And terms split across title/body; And mixed
   quotedphrase+otherterm withphraseabsent; quotedstopword phrase (circle of fifths)
   versus alteredadjacency; hyphen tokenization; MinimumMatch100% with onlyoneof3
   terms; configuredfieldboost reversal; per-requestquery_mode override; default
   fuzzy/stopwordspositivecontrols. Verify actualmatchingIDs/scores, not onlyDebug
   of aquery. Preserve the explicitly approved1298ISOcorpus; fixingquerygrammar
   does not authorize replacing it. Contrary: all-stopwordfilter fallback returns
   originalquery, so do not claim all-stopwords automatically becomeAllQuery.
5. **Filtering/counts/limits/snapshots.** TantivySearch.search executesTopDocs
   limit beforecategory/source/content-type filtering and takes total=docs.len.
   Probe wrongcategoryhighrank +eligiblelower-rank atlimit1, unfilteredranking
   positivecontrol, actualtotalvsallmatchingIDs, source/type/case controls. Check
   requestsnippet_length/query_mode forwarding. Zero-limit reachesTopDocs::with_limit
   assert_ne0 in exactcachedtantivy0.27top_score_collector93–94; compare public
   simple/Tantivy entrypoints with caughtpanic/error, no producttest injection.
   Do not allocateMAX/hazardous sizes merely to crash aprocess. Reconcileexecute
   andconvert_results using separatelyacquiredSearcher snapshots; a concurrent-
   generation suspicion needs exactSDKsemantics and discriminating publicevidence
   or explicitstatic/hypothesisconfidence, not an invented reproducedrace.
6. **Index generations/append/freshness.** Actual IndexBuilder build clearsindex
   but doesnotinvalidateAppendMetadata; build_append skips per-sourcecachedhash.
   Exercise A-build/B-append/A-forcerebuild/B-appendunchanged, reopenactualsearch
   and assertallIDs/content, not juststats. Exercise sourceupdate/delete/reappend
   and sameIDs across roots, oldterm disappearance and uniqueness. Freshness178
   hashes mtimeintegerseconds: controlled same-secondchangedbytes/distinctnanos
   with actualobservedmtime evidence should distinguish stale reuse from rebuild.
   Keep cache/index outsideownedcontent fixtures. Contrary: Indexer.clear119–123
   itself commits, so empty/allskiprebuild lacking finalcommit is NOT a defect.
   Schema version IS included in FTShash; don't claim omittedschema. Errorhandling,
   missingindex/create/queryreadiness and partialskipmetadata also require explicit
   presentcontract assessment, not assuming every skippedfile invalidates success.
7. **Fallback/API composition and architecture.** Review wholeSimpleSearch contract
   againstTantivy, config/schema ownership, publicsearch/result/metadata fields,
   duplicatedhelpers and extractor boundaries. Show actualsameinputs and consumer
   implications before grading competingpolicy abstractions. Assess invalidpublic
   input and test weakness without adding repairtests toproduct. Current passing
   unit tests can encode the wrongcontract; originaltestnames alone are notproof.

## Verification discipline and negative checks

Use at leastfive concrete negative/counterexample checks for thislanguage report,
covering the actualnewpopulations above (not just reusing earlierpass rows). Each
confirmedfinding must have actualentrypoint evidence when feasible; staticproof
is allowed only explicitly identified with its controlling source and futureoracle.
Keep hypothesis/clearedlead dispositions and genuinelyunavailable checks explicit.
Runtimeclaims require exactcommands, trueexitstatuses and fullrawlogs. Don't use
tee's exitstatus, add skips/weakenassertions or suppresswarnings to manufacturegreen.
Do not rerun unchanged fullworkspace/native/CI gates already acceptedatarc00/719.
Targeted meaningful tests/probes are allowed; existingseven-core+MCPconsumerpassing
suites are not a prerequisite to repeat. For relevantTantivy feature, record actual
selecteddefault/feature cases and build resolution, not blanketallfeatures claims.

Only CC-owned scratch under/private/tmp, with a UNIQUE package/binaryname such as
foundation-content-search-probe. Existing ownedtarget
/private/tmp/arc01-slice02-cargo-target is transferred for safe cachedreuse during
thisassignment; don't delete its earlieroutputs, mutate producttarget/cache or run
competingCargo/native processes. Keep new scratchmanifest/source/lock/evidence in
flat projectartifacts and copy to a namedtmpbuildtree. Compare scratch dependency
selection/features/toolchain to productlockedbaseline; a subset lock is not a
productlockrun. Preserve any failedcompilation/oracle and its correction. Never
labelunrelated scratchbinary output as an intendedentrypoint's witness.

## Exact mutable outputs

During execution you alone own these existing absolutepaths:
- /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.22-audit-results-rust.md (append pass02after accepted001–010; next stableID011; preserve existingcontent/history)
- /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-index.md
- /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/2026.11.21-audit-coverage.json (only31slice03rows,7pass02cells/statusnote; preserve38acceptedpass01rows,7acceptedcells and allother populations/counts/features/hashes)
- /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice03-content-search/ledger.md
- /Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/arc01-foundation-audit/slice03-content-search/closing-report.md

New ownedproject artifacts at actual2026.11.22 creationdate, all absolute under
/Users/oubiwann/lab/oxur/textrynum/.worktrees/planning/project05-discoverability/artifacts/:
- 2026.11.22-content-search-evidence.md (per31file/sevenscale dispositions, commands/limits/clearedleads)
- 2026.11.22-content-search-probe.rs and2026.11.22-content-search-probe.toml
- 2026.11.22-content-search-probe.lock (actualscratchlock, if used)
- 2026.11.22-content-search-run.log (completeactualrunoutput/exits, no hugeunusedarchives)

Use actualChicago dateif creationcrossesday; explicitlyreport anyprefixadjustment
without rewritingthisissuedprompt. No extra per-observationfiles, workbench output,
per-sliceartifacts, sourceedits or CDC verification/retro/plan/arc/project edits.
Currentindex should distinguish38accepted,31CCproposed,291pending and pass01accepted/
pass02proposed/otherspending. Do not turn this into wholeRust orarc acceptance.

## Finding contract, privacy, close and return

Every confirmedfinding: stableID, severity(Critical/High/Medium/Low), exact719file:line,
seven-scale relevance, observedfailuremechanism, static/runtimeconfidence, logical
repairowner+affectedconsumers, fixdirection, compatibilitydecisions and discriminating
validationoracle. Separatehypotheses/clearedleads, root-deduplicate, don't silently
recommissionheld/cancelledfeatures or callactualChatGPTusabilityproved. No repairs.

Use onlyAcme Inc.newexamples. Exact authoritative counts-onlyscreen, with no
identityprinting/decoding or oldobjectlookup:
```python
pattern = re.compile(bytes([87, 97, 108, 103, 114, 101, 101, 110, 115]), re.I)
content_hits = sum(len(pattern.findall(path.read_bytes())) for path in owned_paths)
path_hits = sum(len(pattern.findall(str(path).encode())) for path in owned_paths)
print('owned_paths', len(owned_paths), 'content_hits', content_hits, 'path_hits', path_hits)
assert content_hits == path_hits == 0
```
owned_paths is the exactexisting/newpaths youactuallywrite; no guessed/base64derived
pattern is an oracle. Screen before dumps if anynewinputmightcontainprivatedata.
No hostedpurge/historyrewrite/externalmessage is authorized.

Walk allfiveSEARCHrows in canonicalclose, proposed-doneonly with actualevidence.
Validate exactownedmapping/diff/links/privacy, sourceclean719, trueprobeexits and
allrequiredfindingfields. Beforecommit writeexactmessage to tasktemporaryfile,
inspectrequiredtrailers, gitaddONLYownedpaths, compareexactstagedallowlist andcommit
onplanning. Bothtrailers afterblankline, adjacentactualnewlines:
Co-authored-by: Codex <noreply@openai.com>
Co-authored-by: Billo AI <ai-engineering@billo.systems>
Verify actualmessage/trailers/filelist aftercommit. Preserveallcommits; noamend/
squash/fixup/drop/ordinarymerge/push. ReturncommitID, coverage/findings/limits and
explicitplanning/index/ownedtargethandback with noactive mutableprocess. CDC then
independentlyreviews; diagnosisacceptance is not repairacceptance. Completework,
not just a launch/plan, within this boundedassignment.
