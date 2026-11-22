# Slice15 closing report — vector construction contracts

Status: CC author-proposed bounded completion; independent CDC review pending.
Source branch `codex/foundation-audit` remained read-only at
`719c58f245fe4a9115c7c0391d3e268c99157e2f`. Planning began at
`364cffe2eb792cded3b2667ce8bb5d7a21457ee7`.

## Result and limits

The six assigned files were read completely (2,471 lines), followed by the
required source-cited readback before edits. Static source tracing supports one
proposed Low finding, FA-RS-039: zero batch size is accepted by the public setter
and reaches the documented `chunks(0)` panic precondition on successful cold
build/append paths. Missing inputs, earlier errors and a successful cache hit
can exit first. No execution reproduced the panic.

Append-provider embedding-space equality, cache partitioning by provider/model,
and custom batch cardinality are retained as questions because these source files
do not establish a supported cross-provider contract or built-in cardinality
failure. Same-provider append, same-provider cache hit, content-change miss,
skip-cache rebuild, positive batches, and default/mock one-result-per-input are
contrary controls. Different dimensions are shape-incompatible; equal dimensions do not establish
the same provider, model or embedding space. No vector was generated and no
retrieval behavior was observed.

This review made no product/source changes and did not run Rust, Cargo, build,
tests, executable fixtures, providers/models, services, security, CI, or any
interrupted SEARCH work. It does not accept any original coverage owner, graph/
vector/MCP gate, SEARCH criterion, or whole-arc composition. Finding039 remains
author-proposed pending CDC review; earlier findings/status bytes are preserved.
VEC-05 attribution is UNMET/EXCEPTION-PENDING for Operator disposition: the later
valid trailer block does not repair the immutable initial commit.

## Preservation and exact scope

The Rust report and index retain their exact pre-slice15 bytes as prefixes; the
original history and both author commits remain preserved. Their exact pre-364cffe
SHA256 fingerprints are report
`98f947c97a18644737b64a45838270ce00408fd5f549f298463cdb8407a39689` and index
`9b83b3477b5cfbad892a4196880cf474136f2a35300c28b24435205809b8a7ea`. The audit
coverage JSON remains unchanged at SHA256
`4c278c0c8cf2289970c114c692ff3b40df63aa1f6c8cea3c3c4fcb7b11600c58`.

### Preservation provenance correction01

The first four paths below are the original author evidence files. The prior
hash-check history applies to this author set only; they are not the unfinished
CDC preparation artifacts. Current hashes match that recorded author evidence:

| Original author evidence artifact | SHA256 |
| --- | --- |
| `2026.11.22-content-search-probe.rs` | `f08fac7db35435c3f23d0ef449547ab8232ba711bae938e6367638ae5459a039` |
| `2026.11.22-content-search-probe.toml` | `6c241f2fa1aa4d6c320db035b68a60ee81d8162e6e78075bb755f100d24e4185` |
| `2026.11.22-content-search-probe.lock` | `5b60133ce1714418470e314b21c252113a4e5c5fa75c8de13c65b1420c39b19d` |
| `2026.11.22-content-search-run.log` | `8d8feb5aac36e7bca07cbb42248f137dc72b96fdff8d5fdf465b06ceadda5e6e` |

The separately named CDC preparation set was hashed only for preservation. All
four current hashes match the values in the immutable correction. CDC reports
that it independently compared both sets byte-for-byte with planning base
`364cffe2eb792cded3b2667ce8bb5d7a21457ee7`; that is CDC's verification, not a
claim that these files were opened or parsed here. These CDC files remain
UNVERIFIED, and a hash is not run attestation:

| Unfinished CDC preparation artifact | SHA256 |
| --- | --- |
| `2026.11.22-content-search-cdc-probe.rs` | `e04f1a0d0c185ff429180536e64a828de69e857e4ffb29d2e2de6bc5afff088a` |
| `2026.11.22-content-search-cdc-probe.toml` | `a7471fdc14c83fce774101076cabb03911bb6886fd6308d329023f462336a6d4` |
| `2026.11.22-content-search-cdc-probe.lock` | `5c1ba213248a216cea25ee00e1f89691504b1433d8d2baa953cab39372cda48b` |
| `2026.11.22-content-search-cdc-run.log` | `0189c14b29d4fa5bdcd8b495be2474c2d1f0b426f6aca95771a6524be785b24c` |

No content from either four-file set was opened in this correction. Coverage JSON
and all eight files remain unchanged. No other source or planning path changed.
The original five-path content commit and its separate closeout-only follow-up
remain immutable; the next correction commit is scoped to the same five owned
outputs/index.

## Actual bookkeeping and privacy screen

All listed intake and bookkeeping commands completed with exit0: absolute-path
`cat`/`sed` source and planning reads; `wc -l`; source/planning
`git status --short --branch` and `git rev-parse HEAD`; targeted `rg` over
planning records; and `sha256sum` over coverage and both four-file preservation sets. CDC artifacts
were not opened, parsed, reconciled or executed. No product code executed.

The prior counts-only privacy screen covered the five owned paths before this
correction. Correction01 ran the same screen after edits: content matches=0; path matches=0.
No matched text was printed. Examples use only Acme Inc. and anonymized
technical labels.

## Commit and ownership return

The initial five-path content commit is preserved as
`bf193ab9d3f26fb7a858e54e84e92aa97d692542`. Post-commit review found that
`git interpret-trailers --parse` recognized only its Billo AI line because the
two co-author lines were separated by a blank line. The close-report-only commit
`7133ede9adc5eb500ec9479505193550940dd3eb` has both adjacent trailers parsed,
but it does NOT repair `bf193ab`. No amend or history rewrite was used.
VEC-05 attribution therefore remains UNMET/EXCEPTION-PENDING for Operator
disposition; preservation and scoped-path checks are recorded separately.

For the correction commit, the complete subject/body and adjacent trailers are
written to one temporary message file; both trailers are parsed before commit
and checked on the created object afterward. The final handback records the
commit ID and parsed lines. CC then explicitly returns ownership of the five
named planning paths and planning index to CDC. No mutable command remains
active at handback.
