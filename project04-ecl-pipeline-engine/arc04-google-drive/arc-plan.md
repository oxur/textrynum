# Pipeline Phase 4 — Google Drive

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/05-active/0018-ecl-pipeline-runner-multi-phase-project-plan.md, lines 399-469. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 4: Real Adapter (Google Drive)

### Milestone 4.1: Google Drive Adapter — Auth & Enumerate

**Goal:** Authenticate with Google Drive API and enumerate files in a
folder. No fetching yet.

**Crate:** `crates/ecl-adapter-gdrive/`

**Key files to create:**

- `Cargo.toml` — depends on ecl-pipeline-topo (for `SourceAdapter` trait), plus reqwest, serde, serde_json, tokio, yup-oauth2 (or google-authz)
- `src/lib.rs` — `GoogleDriveAdapter` struct, SourceAdapter impl
- `src/auth.rs` — OAuth2 token management (service account + user credentials)
- `src/enumerate.rs` — Drive Files.list API, pagination, folder traversal
- `src/types.rs` — Drive API response types

**Key deliverables:**

- [ ] `CredentialRef::File` resolves to service account JSON
- [ ] `CredentialRef::EnvVar` resolves to token from environment
- [ ] `CredentialRef::ApplicationDefault` uses ADC
- [ ] `enumerate()` calls Drive Files.list with folder filter
- [ ] Pagination: handles `nextPageToken` for large folders
- [ ] Recursive folder traversal (respects root_folders config)
- [ ] Filter application: file_types, glob patterns, modified_after
- [ ] `SourceItem.source_hash` populated from Drive's `md5Checksum`
- [ ] Test: mock HTTP responses for enumerate (use wiremock or similar)
- [ ] Register crate as workspace member

**Test strategy:**

- Mock Drive API responses with wiremock
- Test pagination (multiple pages)
- Test filter application (file type, glob, modified_after)
- Test auth credential resolution (file, env, ADC)
- Error handling: invalid credentials, API errors, rate limits

---

### Milestone 4.2: Google Drive Adapter — Fetch & Full Pipeline

**Goal:** Fetch document content from Drive. Run a full pipeline:
Drive folder -> extract -> normalize -> emit to local files.

**Files to modify:**

- `ecl-adapter-gdrive/src/fetch.rs` — content download, format detection
- `ecl-stages/src/normalize.rs` — extend with basic format handling

**Key deliverables:**

- [ ] `fetch()` downloads file content via Drive Files.get with `alt=media`
- [ ] Google Docs export: `application/vnd.google-apps.document` -> markdown via export API
- [ ] PDF handling: extract text (basic, via external tool or crate)
- [ ] Compute blake3 hash of fetched content
- [ ] Rate limiting: respect Drive API quotas (429 handling via retry)
- [ ] `NormalizeStage` handles plain text and markdown passthrough
- [ ] Test: mock HTTP for fetch, verify content and hash
- [ ] Integration test: filesystem adapter simulating Drive output structure

**Test strategy:**

- Mock HTTP for document download
- Test Google Docs export MIME type mapping
- Test content hash computation
- Rate limit handling (429 -> retry with backoff)
- End-to-end with mocked HTTP server

---


## Reconstructed slice locations

- [Milestone 4.1: Google Drive Auth & Enumerate (`ecl-adapter-gdrive`)](slice01-google-drive-auth-enumerate/slice-plan.md)
- [Milestone 4.2: Google Drive Fetch & Full Pipeline (`ecl-adapter-gdrive` + `ecl-stages`)](slice02-google-drive-fetch-full-pipeline/slice-plan.md)
