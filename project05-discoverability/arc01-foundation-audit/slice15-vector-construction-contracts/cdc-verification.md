# CDC verification — slice15 vector construction contracts

## Independent verdict and boundary

VEC-01–04 and Low039 independently accepted for this static construction/API
facet at corrected candidate98fca6cb344b7efc0c0efd28e1529fda5f947cf7. VEC-05
preservation/owned-path checks pass; attribution is explicitly UNMET/EXCEPTION-
PENDING. Full slice completion is not accepted. Source diagnosis does not waive
that requirement. Original full-pass/SEARCH/whole-arc gates remain pending.
CDC Sol6.1High independently reviews the retained LunaHigh author; no extra
composition reviewer was available or claimed.

## Controlling source and contract interpretation

CDC directly read complete builder construction/build/append/hash bodies1–448,
positive/cache controls600–633/645–753; embedding1–194; backend1–317/455–535;
types1–185/285–320; extractor1–207 and lib1–108. Correction rechecked exact
numbered loci. These are own logical-unit/range reads, not another reviewer's
full-source receipt. Actual author commands were independently reconciled:
builder1–250/251–520/521–753; embedding full; backend1–240/241–480/481–626;
types1–220/221–440/441–583 plus complete extractor/lib, no source truncation.
Six full author files2471lines and source-cited pre-edit readbacks are supported.
Initial combined metadata truncation was recovered; no corpus restart required.

Setter96–100 stores any usize; build220 and append357 call chunks(size).
Official [Rust1.100 standard-library documentation](https://doc.rust-lang.org/std/primitive.slice.html#method.chunks)
read2026-11-22 defines zero as a panic precondition. Build130–145/missing inputs,
147–173/cache hit and discovery/fail-fast paths can return earlier. If batching
is reached, even an empty vector evaluates chunks(0). This is a static language/
call-path conclusion, not an observed Rust run. Low is proportionate because the
caller explicitly selects0, default64 is valid and no application config wiring
was established. Result alone never guarantees no panic. The current public docs
do not state the nonzero invariant, so039 concerns that undocumented contract;
owner may document it or select validation/fallible/type policy. No fix chosen.

Cache fields85–96, loading167–186 and freshness189–203 preserve old documents
and attach a supplied provider, keyed by content hash. Append301–390 uses its
own provider while the existing backend keeps another. Those source paths expose
compatibility questions, but do not establish a supported cross-provider pairing
or cache-partition responsibility; no separate defect/severity is accepted.
Same-provider append/cache examples and content-change/skip controls remain
contrary evidence. Provider.name44–45 is diagnostic, not a proven model key.

Different dimensions prove shape incompatibility; equal dimensions alone do not
prove a shared model/space. Cosine205–219 returns0 for mismatched lengths; backend
request fallback0 at232 can retain that score while positive thresholds filter
it. This is branch reasoning only. It is not VectorConfig-to-request wiring,
executed retrieval or data-loss proof. Default embedding33–39 and mock97–102
produce one vector per input on success; zip cardinality validation remains a
question, with no built-in defect inferred. Three contract questions stay
separate; no finding-count target or unsupported shared root was imposed.

## Criterion walk

| ID | Actual independent evidence | Disposition |
| --- | --- | --- |
| VEC-01 | Complete author source command extents/no truncated source; own controlling reads; retained guidance and pre-edit readbacks | Accepted bounded intake |
| VEC-02 | Corrected numbered precondition/provider/cache ownership map; explicit intended same-provider use/caller uncertainty | Accepted static map |
| VEC-03 | Conditional zero/cache/early exits; dimension-vs-space and default/mock controls; no executed behavior | Accepted source derivation |
| VEC-04 | One Low039 with stable ID, pinned source, Scale/owner/fix/compatibility/oracle; three separate questions | Accepted static diagnosis |
| VEC-05 | Exact owned paths, source clean, prefixes/coverage/both artifact sets preserved, but bf193 attribution malformed | Preservation verified; attribution UNMET/EXCEPTION-PENDING |

## Exact attribution evidence and policy

[Planning AGENTS55–60](../../../AGENTS.md#L55) says every assistant-authored commit
“must include both trailers”;59–60 supplies Codex and Billo. History/integration
policy50–53 and current assignment do not authorize rewriting bf193 to fix it.
The distinction below is actual parsed Git object state, not string-presence
inference:

| Object | Literal text | git parsed Co-authored-by values | Disposition |
| --- | --- | --- | --- |
| bf193ab9d3f26fb7a858e54e84e92aa97d692542 | Both lines, blank between them | Billo AI only | Mandatory attribution unmet; exception pending |
| 7133ede9adc5eb500ec9479505193550940dd3eb | Adjacent correct lines | Codex and Billo AI | Its own block valid; no retroactive repair |
| 98fca6cb344b7efc0c0efd28e1529fda5f947cf7 | Adjacent correct lines | Codex and Billo AI | Correction block valid; original remains unmet |

CDC independently ran git interpret-trailers --parse and actual-object
%(trailers:key=Co-authored-by,valueonly) checks. Root separately bounded the
initial resumption defect to bf193 among13objects17c6ecb..653a394; that is root's
read-only witness, not a claim about all earlier history. Later correction98 was
independently checked above. No exception has been granted. VEC-05 cannot be
counted met from both literal strings or a later valid block. No amend/rebase/
rewrite was performed. Root receives this final evidence for Operator disposition.
Future messages use one complete owned file, adjacent trailers parsed before
commit and actual object checked afterward; separate -m paragraphs caused the
original layout problem.

## Preservation, correction history and actual exits

At initial7133handback, author had hashed the four original author proof files
and mislabelled them as unfinished CDC preparations. CDC checked both sets byte-
equal against364cffe, then correction01 supplied exact CDC names/hashes. Author
later hashed the CDC set separately. All eight files and coverageJSON remain
byte-equal; only preservation metadata/bytes were handled, no content interpreted
or executed. The unfinished CDC quartet remains unverified, with old unresolved
pre-run assertion and no build/test/run witness unchanged. This work never
reconciled that driver or lock and supplies no new SEARCH proof.

Both complete pre-364cffe report/index prefixes and issued initial/correction
prompts were preserved. Candidate98 contains exactly five authorized outputs,
source719 and planning were clean at handback, and exact path/content privacy
counts0. CC13602/15598 each completed0; no mutable child/native process remains.
Actual runner witnesses:

- /private/tmp/textrynum-vector-construction-slice15-run.jsonl —356604bytes, SHA2565abe1b218a1a3f0ac821d5268d4e94dc2e07e784126f5a95566a70b4f34ab359
- /private/tmp/textrynum-vector-construction-slice15-iteration01-run.jsonl —285810bytes, SHA256db7cadd25e339582c5ca438a1e8a6e6056dabf54acc53f88ca95dc6242e7b146

Both frozen coordination logs screened0 for the encoded private pattern. Local
text-replacement anchors failed on wrapped prose and were corrected; these are
bookkeeping observations, not product or behavioral failures. Successful final
source/ID/link/prefix/hash/privacy and actual-trailer checks carry this verdict.
No Rust/product build/test/provider/model/service/SEARCH execution occurred.
Refinements1/5used,4remaining;03 remains2/5used. Original coverage38/7 and all
remaining arc gates unchanged. Only15full closure waits for attribution; the
whole arc is active, without a user pause or automatic hold on independent work.
