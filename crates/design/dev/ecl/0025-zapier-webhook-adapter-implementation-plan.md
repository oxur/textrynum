# Zapier Webhook Adapter — Implementation Plan

## Context

The ECL pipeline currently supports **pull-based** source adapters (filesystem, Google Drive, Slack) where the runner calls `enumerate()` then `fetch()`. Zapier inverts this model — data is **pushed** via HTTP webhooks. When a Zap triggers (Granola meeting note, Gmail, etc.), Zapier POSTs JSON to our endpoint.

This plan covers Milestones 7.1 (trait + webhook receiver + runner changes) and 7.2 (typed payload schemas). Milestone 7.3 (live Zap testing) is a test plan only, no code.

The CEO is excited to demo this. The goal is to get a working webhook receiver that integrates cleanly with the existing pipeline architecture.

---

## Architecture Decision

**Separate `push_sources` field** (not an enum wrapper around sources). Rationale:

- Every existing consumer of `topology.sources` (runner's `enumerate_sources()`, `collect_items_for_stage()`, etc.) stays untouched
- Additive change — no match-arm pollution across existing code
- Easy to collapse into a unified enum later if needed

---

## Implementation Steps (dependency order)

### Step 1: Workspace deps + crate registration

**File: `/Users/oubiwann/lab/oxur/ecl/Cargo.toml`**

- Add `tokio-stream = "0.1"` to `[workspace.dependencies]`
- Add `base64 = "0.22"` to `[workspace.dependencies]`
- Add `"crates/ecl-adapter-zapier"` to workspace `members`

### Step 2: Add `ZapierSourceSpec` to spec layer

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline-spec/src/source.rs`**

- Add variant to `SourceSpec` enum:

  ```rust
  #[serde(rename = "zapier")]
  Zapier(ZapierSourceSpec),
  ```

- Add `ZapierSourceSpec` struct:

  ```rust
  pub struct ZapierSourceSpec {
      pub bind_addr: String,
      /// Username for Basic Auth (plain string — not sensitive).
      pub auth_username: String,
      /// Secret (password or Bearer token) via CredentialRef — no secrets in TOML.
      pub credentials: CredentialRef,
      #[serde(default = "default_batch_max_items")]
      pub batch_max_items: usize,
      #[serde(default = "default_batch_timeout_secs")]
      pub batch_timeout_secs: u64,
      #[serde(default = "default_channel_capacity")]
      pub channel_capacity: usize,
      pub default_source_hint: Option<String>,
  }
  ```

  Auth supports both Basic Auth (username + CredentialRef secret) and Bearer token (just the CredentialRef secret). Handler checks for both.
- Add default helper functions + serde roundtrip test

### Step 3: Add `PushSourceAdapter` trait to topo layer

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline-topo/src/traits.rs`**

- Add new trait after existing `SourceAdapter`:

  ```rust
  #[async_trait]
  pub trait PushSourceAdapter: Send + Sync + std::fmt::Debug {
      fn source_kind(&self) -> &str;
      async fn start(&self) -> Result<tokio::sync::mpsc::Receiver<ExtractedDocument>, SourceError>;
      async fn shutdown(&self) -> Result<(), SourceError>;
  }
  ```

- Design: returns concrete `mpsc::Receiver` (not `Pin<Box<dyn Stream>>`) — simpler, object-safe, natural backpressure via bounded channel
- Add object-safety test (mock impl, store as `Arc<dyn PushSourceAdapter>`)

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline-topo/src/lib.rs`**

- Export `PushSourceAdapter` from `pub use traits::...`
- Add field to `PipelineTopology`:

  ```rust
  pub push_sources: BTreeMap<String, Arc<dyn PushSourceAdapter>>,
  ```

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline-topo/src/resolve.rs`**

- Add `SourceSpec::Zapier(_) => "zapier"` arm to `source_kind()` function
- In `resolve()`, skip Zapier sources during pull-adapter resolution (they'll be resolved separately):

  ```rust
  for (name, source_spec) in &spec.sources {
      if matches!(source_spec, SourceSpec::Zapier(_)) {
          continue; // Push sources resolved separately
      }
      // ... existing adapter_lookup code ...
  }
  ```

- Initialize `push_sources: BTreeMap::new()` in the returned `PipelineTopology`
- Update `mock_adapter_lookup` in tests to handle `SourceSpec::Zapier`

### Step 4: Create `ecl-adapter-zapier` crate

**New directory: `crates/ecl-adapter-zapier/`**

Structure:

```
crates/ecl-adapter-zapier/
├── Cargo.toml
└── src/
    ├── lib.rs          # ZapierAdapter + PushSourceAdapter impl
    ├── error.rs        # ZapierAdapterError enum
    ├── server.rs       # axum webhook handler
    └── schemas/
        ├── mod.rs      # Schema dispatch (source_hint -> typed deserializer)
        ├── granola.rs  # GranolaMeetingNote (priority)
        ├── gmail.rs    # GmailMessage
        ├── slack.rs    # SlackMessage
        └── gdrive.rs   # GDriveFileChange
```

**`Cargo.toml` deps:**

```toml
[dependencies]
ecl-pipeline-topo = { path = "../ecl-pipeline-topo" }
ecl-pipeline-spec = { path = "../ecl-pipeline-spec" }
ecl-pipeline-state = { path = "../ecl-pipeline-state" }
axum = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
chrono = { workspace = true }
blake3 = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }
async-trait = { workspace = true }
base64 = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
reqwest = { workspace = true }
tempfile = { workspace = true }
```

**`src/lib.rs` — Core adapter:**

```rust
#[derive(Debug)]
pub struct ZapierAdapter {
    source_name: String,
    spec: ZapierSourceSpec,
    shutdown: Arc<Notify>,
    sender: mpsc::Sender<ExtractedDocument>,
    receiver: tokio::sync::Mutex<Option<mpsc::Receiver<ExtractedDocument>>>,
}
```

- `from_spec(name, &SourceSpec) -> Result<Self, ResolveError>` constructor
- `start()`: spawns axum server task, returns receiver
- `shutdown()`: signals Notify, server gracefully stops

**`src/server.rs` — Webhook handler:**

- `POST /webhook` endpoint
- Validates auth — supports **both** Basic Auth and Bearer token:
  - Basic Auth: base64-decode `Authorization: Basic <b64>`, compare username + secret
  - Bearer token: compare `Authorization: Bearer <token>` against secret
  - Zapier natively supports Basic Auth; Bearer is there for flexibility
- Parses JSON body
- Identifies source via `X-Zapier-Source` header or `default_source_hint`
- Dispatches to typed schema deserializer (Milestone 7.2)
- Computes blake3 hash of raw body bytes
- Builds `ExtractedDocument` with `ItemProvenance` (source_kind: "zapier", metadata: source_hint, webhook headers)
- Sends to bounded mpsc channel; returns 429 if channel full (backpressure)
- Returns 200 immediately
- Server uses `axum::serve()` with `with_graceful_shutdown(shutdown.notified())`

**`src/error.rs`:**

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ZapierAdapterError {
    #[error("server bind failed on {bind_addr}: {message}")]
    BindFailed { bind_addr: String, message: String },
    #[error("channel closed unexpectedly")]
    ChannelClosed,
    #[error("invalid payload from {source_hint}: {message}")]
    InvalidPayload { source_hint: String, message: String },
    #[error("authentication failed")]
    AuthFailed,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

**`src/schemas/` — All four typed payloads (Milestone 7.2):**

All four schemas implemented upfront for demo completeness.

Schema resolution: `source_hint` string -> typed deserializer -> `ExtractedDocument`

- `"granola"` -> `GranolaMeetingNote` (title, creator, attendees, calendar_event, my_notes, summary, transcript, link) -> content = summary + transcript as markdown, metadata = attendees/calendar/link
- `"gmail"` -> `GmailMessage` (from, to, subject, body_plain, body_html, date, labels, thread_id, message_id) -> content = body_plain or body_html, metadata = from/to/subject/labels
- `"slack"` -> `SlackMessage` (channel, user, text, ts, thread_ts, team) -> content = text, metadata = channel/user/thread_ts
- `"gdrive"` -> `GDriveFileChange` (file_id, file_name, mime_type, modified_time, web_view_link) -> content = raw JSON (file metadata only), metadata = file_id/mime_type/link
- Fallback: raw `serde_json::Value` stored as JSON bytes with `mime_type = "application/json"`

### Step 5: Runner changes for push sources

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline/src/runner.rs`**

Add to `PipelineRunner`:

```rust
/// External shutdown signal (for push-source long-running mode).
shutdown: Arc<tokio::sync::Notify>,
```

Modify `PipelineRunner::new()` to accept a shutdown signal:

```rust
pub async fn new(
    topology: PipelineTopology,
    store: Box<dyn StateStore>,
    shutdown: Arc<tokio::sync::Notify>,
) -> Result<Self>
```

Modify `run()` to handle push sources after pull sources complete:

```rust
pub async fn run(&mut self) -> Result<&PipelineState> {
    // Phase 1: Enumerate pull sources (existing)
    // Phase 2: Execute batches for pull sources (existing)
    // Phase 3: Push source loop (NEW)
    if !self.topology.push_sources.is_empty() {
        self.run_push_sources().await?;
    }
    // Phase 4: Finalize (existing)
}
```

New method `run_push_sources()`:

- Calls `adapter.start()` for each push source
- Enters a loop: `tokio::select!` between receiver items and shutdown signal
- Batches incoming `ExtractedDocument`s (up to batch_max_items or batch_timeout_secs)
- Converts each doc into `ItemState` + `PipelineItem`
- Feeds batch through stage pipeline using existing `execute_batch()` machinery
- Checkpoints after each batch
- On shutdown: drains remaining buffered items, processes final batch, exits

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline/Cargo.toml`**

- Add `tokio-stream = { workspace = true }` (for potential future use; the mpsc receiver approach may not need it immediately)

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-pipeline/src/error.rs`**

- Add variant if needed: `PushSourceError { source_name, detail }`

### Step 6: Registry wiring

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-cli/src/pipeline/registry.rs`**

- Add import: `use ecl_adapter_zapier::ZapierAdapter;`
- In `resolve_adapters()`, handle `Zapier` variant:

  ```rust
  SourceSpec::Zapier(_) => continue, // Push sources resolved separately
  ```

- Add new function `resolve_push_adapters()`:

  ```rust
  pub fn resolve_push_adapters(
      spec: &PipelineSpec,
  ) -> Result<BTreeMap<String, Arc<dyn PushSourceAdapter>>, ResolveError> {
      let mut adapters = BTreeMap::new();
      for (name, source_spec) in &spec.sources {
          if let SourceSpec::Zapier(_) = source_spec {
              adapters.insert(name.clone(),
                  Arc::new(ZapierAdapter::from_spec(name, source_spec)?) as Arc<dyn PushSourceAdapter>);
          }
      }
      Ok(adapters)
  }
  ```

- Wire into the CLI pipeline build path: call `resolve_push_adapters()` and set `topology.push_sources`

**File: `/Users/oubiwann/lab/oxur/ecl/crates/ecl-cli/Cargo.toml`**

- Add `ecl-adapter-zapier = { version = "0.4.0", path = "../ecl-adapter-zapier" }`

---

## Testing Strategy

### Unit Tests (per crate)

1. **ecl-pipeline-spec**: Serde roundtrip for `ZapierSourceSpec`, TOML parsing with `kind = "zapier"`, default values
2. **ecl-pipeline-topo**: Object-safety test for `PushSourceAdapter`, `source_kind()` returns `"zapier"` for Zapier variant
3. **ecl-adapter-zapier**:
   - `from_spec()` with valid/invalid specs
   - Webhook handler: POST valid JSON -> 200, wrong auth -> 401, channel full -> 429, invalid JSON -> 400
   - Typed schema deserialization from fixture JSON files (one per source type)
   - Shutdown: start server, signal shutdown, verify stream ends cleanly
   - Blake3 hash consistency (same payload -> same hash)
   - Backpressure: fill channel, verify handler blocks/returns 429
4. **ecl-pipeline runner**: Mock `PushSourceAdapter` yields N docs then closes. Verify docs flow through stages, get checkpointed, state is correct.

### Integration Tests

1. **ecl-adapter-zapier**: Full flow — start adapter on `127.0.0.1:0`, POST via `reqwest`, verify `ExtractedDocument` arrives on receiver with correct fields
2. **ecl-pipeline**: Pipeline with both a filesystem source (pull) and a mock push source — verify both flows complete and state merges correctly
3. **Regression**: Existing pull adapter tests (FS, GDrive, Slack) must pass unchanged

### Test Patterns to Follow

- `#[cfg(test)] #[allow(clippy::unwrap_used)]`
- Naming: `test_<unit>_<scenario>_<expectation>`
- Fixtures in test data files
- `wiremock` / `reqwest` for HTTP testing
- `127.0.0.1:0` for OS-assigned ports in tests

---

## Verification

After implementation:

1. `make test` — all tests pass
2. `make lint` — no clippy warnings
3. `make format` — code formatted
4. `make coverage` — target >= 95%
5. Manual smoke test:

   ```bash
   # Terminal 1: Start pipeline with Zapier source
   ecl pipeline run --config examples/zapier-demo.toml

   # Terminal 2: Send test webhook
   curl -X POST -u ecl-webhook:demo-secret \
     -H "Content-Type: application/json" \
     -H "X-Zapier-Source: granola" \
     -d '{"title":"Test Meeting","summary":"...","transcript":"..."}' \
     http://127.0.0.1:9090/webhook
   # Expect: 200 OK

   # Terminal 1: Verify item processed
   ecl pipeline status ./output
   ```

---

## Files Summary

### New files (1 crate, 7 files)

- `crates/ecl-adapter-zapier/Cargo.toml`
- `crates/ecl-adapter-zapier/src/lib.rs`
- `crates/ecl-adapter-zapier/src/error.rs`
- `crates/ecl-adapter-zapier/src/server.rs`
- `crates/ecl-adapter-zapier/src/schemas/mod.rs`
- `crates/ecl-adapter-zapier/src/schemas/granola.rs`
- `crates/ecl-adapter-zapier/src/schemas/gmail.rs`
- `crates/ecl-adapter-zapier/src/schemas/slack.rs`
- `crates/ecl-adapter-zapier/src/schemas/gdrive.rs`

### Modified files (in dependency order)

1. `Cargo.toml` (root) — workspace deps + member
2. `crates/ecl-pipeline-spec/src/source.rs` — Zapier variant + ZapierSourceSpec
3. `crates/ecl-pipeline-topo/src/traits.rs` — PushSourceAdapter trait
4. `crates/ecl-pipeline-topo/src/lib.rs` — export trait, add push_sources field
5. `crates/ecl-pipeline-topo/src/resolve.rs` — skip Zapier in pull resolution, add source_kind arm, init push_sources empty
6. `crates/ecl-pipeline/Cargo.toml` — tokio-stream dep
7. `crates/ecl-pipeline/src/runner.rs` — shutdown signal, run_push_sources() method
8. `crates/ecl-pipeline/src/error.rs` — push source error variant
9. `crates/ecl-cli/Cargo.toml` — ecl-adapter-zapier dep
10. `crates/ecl-cli/src/pipeline/registry.rs` — resolve_push_adapters(), skip Zapier in resolve_adapters()

### Key patterns to reuse

- `SourceAdapter` trait pattern from `ecl-pipeline-topo/src/traits.rs:54-70`
- Error handling pattern from `ecl-adapter-gdrive/src/error.rs`
- Constructor pattern: `from_spec(name, &SourceSpec)` from all existing adapters
- `CredentialRef` resolution from `ecl-adapter-gdrive/src/auth.rs`
- Lint attrs: `#![forbid(unsafe_code)]`, `#![deny(clippy::unwrap_used)]`, etc.

---

## Risks

1. **Runner complexity** (medium): Adding push-source loop to the runner is the riskiest change. Mitigated by keeping it isolated in `run_push_sources()` and reusing existing `execute_batch()`.
2. **Push source durability** (low for now): If pipeline crashes between webhook receipt and checkpoint, events are lost. This is inherent to push architectures. Future: add WAL/durable queue.
3. **Port binding in CI** (low): Use `127.0.0.1:0` for OS-assigned ports in all tests.
4. **SourceSpec enum expansion** (low): Mechanical change — every `match` on `SourceSpec` needs a new arm. Small blast radius.
