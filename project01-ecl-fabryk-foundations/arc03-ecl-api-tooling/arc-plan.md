# ECL Phase 3 — API and tooling (historical proposal)

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 951-1339. Original bytes are recoverable from import commit 2d006a7. -->

## Phase 3: API & Tooling

**Objective**: Make ECL usable for real workflows
**Duration**: ~2 weeks
**Deliverable**: HTTP API, CLI improvements, configuration system, documentation

### Stage 3.1: HTTP API Design

**Goal**: Define RESTful API for workflow management.

**Tasks**:

1. Design API endpoints:

   ```
   POST   /workflows                    - Submit new workflow
   GET    /workflows                    - List workflows (with filtering)
   GET    /workflows/{id}               - Get workflow details
   GET    /workflows/{id}/steps         - Get step executions
   GET    /workflows/{id}/artifacts     - List workflow artifacts
   DELETE /workflows/{id}               - Cancel workflow

   POST   /workflows/{id}/signal        - Send signal to workflow

   GET    /definitions                  - List workflow definitions
   GET    /definitions/{id}             - Get definition details

   GET    /health                       - Health check
   GET    /metrics                      - Prometheus metrics
   ```

2. Define request/response schemas (OpenAPI spec)
3. Choose HTTP framework (recommend `axum` for Tokio ecosystem alignment)
4. Design error response format:

   ```json
   {
     "error": {
       "code": "WORKFLOW_NOT_FOUND",
       "message": "Workflow with ID xyz not found",
       "details": {}
     }
   }
   ```

5. Plan authentication approach (API key for v1, JWT for future)

**Acceptance Criteria**:

- [ ] OpenAPI spec is complete and valid
- [ ] All endpoints have clear semantics
- [ ] Error codes are documented
- [ ] Pagination design for list endpoints

**Files to Create**:

```
docs/
└── api/
    └── openapi.yaml

ecl-api/
├── Cargo.toml
└── src/
    └── lib.rs (placeholder)
```

---

### Stage 3.2: HTTP API Implementation

**Goal**: Implement the HTTP API using axum.

**Tasks**:

1. Create `ecl-api` crate
2. Implement router with all endpoints
3. Implement request handlers:

   ```rust
   async fn submit_workflow(
       State(app): State<AppState>,
       Json(request): Json<SubmitWorkflowRequest>,
   ) -> Result<Json<WorkflowResponse>, ApiError> { ... }
   ```

4. Add request validation with `validator` crate
5. Implement pagination for list endpoints:
   - Cursor-based pagination
   - Configurable page size (default 20, max 100)
6. Add API key authentication middleware:
   - Header: `X-API-Key`
   - Validate against config
7. Add request logging (correlate with workflow logs)
8. Add graceful shutdown handling

**Acceptance Criteria**:

- [ ] All endpoints return correct responses
- [ ] Validation errors return 400 with details
- [ ] Authentication works correctly
- [ ] Requests are traced

**Files to Create**:

```
ecl-api/src/
├── lib.rs
├── router.rs
├── handlers/
│   ├── mod.rs
│   ├── workflows.rs
│   ├── definitions.rs
│   └── health.rs
├── middleware/
│   ├── mod.rs
│   ├── auth.rs
│   └── logging.rs
├── error.rs
└── models/
    ├── mod.rs
    ├── requests.rs
    └── responses.rs
```

---

### Stage 3.3: Configuration System

**Goal**: Comprehensive configuration using confyg with multiple sources.

**Tasks**:

1. Define configuration structure:

   ```rust
   #[derive(Deserialize)]
   pub struct Config {
       pub server: ServerConfig,
       pub database: DatabaseConfig,
       pub llm: LlmConfig,
       pub restate: RestateConfig,
       pub logging: LoggingConfig,
   }

   #[derive(Deserialize)]
   pub struct LlmConfig {
       pub provider: String,  // "claude"
       pub api_key: Secret<String>,
       pub model: String,     // "claude-sonnet-4-20250514"
       pub max_tokens: u32,
       pub timeout_secs: u64,
       pub retry: RetryConfig,
   }
   ```

2. Configure confyg to load from (in priority order):
   1. Environment variables (highest priority)
   2. XDG dir
   3. `config/local.toml` (gitignored)
   4. `config/{environment}.toml`
   5. `config/default.toml`
3. Add `Secret<T>` wrapper that redacts in logs/debug
4. Validate configuration at startup:
   - Required fields present
   - URLs are valid
   - Numeric ranges are sensible
5. Create example configuration files
6. Add `--config` CLI flag to specify config path

**Acceptance Criteria**:

- [ ] Configuration loads correctly from all sources
- [ ] Environment variables override file config
- [ ] Secrets are never logged
- [ ] Invalid config produces helpful error messages

**Files to Create**:

```
ecl-core/src/
├── config/
│   ├── mod.rs
│   ├── types.rs
│   ├── loader.rs
│   └── validation.rs

config/
├── default.toml
├── development.toml
├── production.toml
└── local.toml.example
```

---

### Stage 3.4: CLI Enhancement

**Goal**: Full-featured CLI for workflow management.

**Tasks**:

1. Use `clap` with derive macros for CLI definition
2. Implement commands:

   ```
   ecl run <definition> [--input <json>] [--input-file <path>]
   ecl status <workflow-id> [--watch]
   ecl list [--state <state>] [--limit <n>]
   ecl cancel <workflow-id>
   ecl logs <workflow-id> [--step <step-id>] [--follow]
   ecl definitions list
   ecl definitions show <definition-id>
   ecl server start [--port <port>]
   ecl migrate [--target <version>]
   ```

3. Add output format options:
   - `--format json` - JSON output for scripting
   - `--format table` - Pretty table (default)
   - `--format quiet` - Minimal output (just IDs)
4. Implement `--watch` mode for status:
   - Poll every 2 seconds
   - Show step progression
   - Exit when workflow completes
5. Add shell completion generation:

   ```
   ecl completions bash > /etc/bash_completion.d/ecl
   ```

6. Create man page generation

**Acceptance Criteria**:

- [ ] All commands work correctly
- [ ] Help text is clear and complete
- [ ] Output formats work as expected
- [ ] Shell completion works

**Files to Modify/Create**:

```
ecl-cli/src/
├── main.rs
├── commands/
│   ├── mod.rs
│   ├── run.rs
│   ├── status.rs
│   ├── list.rs
│   ├── cancel.rs
│   ├── logs.rs
│   ├── definitions.rs
│   ├── server.rs
│   └── migrate.rs
├── output/
│   ├── mod.rs
│   ├── json.rs
│   ├── table.rs
│   └── quiet.rs
└── completions.rs
```

---

### Stage 3.5: Artifact Storage

**Goal**: File-based artifact storage with abstraction for future S3 support.

**Tasks**:

1. Define `ArtifactStore` trait:

   ```rust
   #[async_trait]
   pub trait ArtifactStore: Send + Sync {
       async fn store(&self, artifact: &Artifact, content: &[u8]) -> Result<ArtifactRef, StoreError>;
       async fn retrieve(&self, reference: &ArtifactRef) -> Result<Vec<u8>, StoreError>;
       async fn delete(&self, reference: &ArtifactRef) -> Result<(), StoreError>;
       async fn list(&self, workflow_id: &WorkflowId) -> Result<Vec<ArtifactRef>, StoreError>;
   }
   ```

2. Define `Artifact` struct:

   ```rust
   pub struct Artifact {
       pub id: Uuid,
       pub workflow_id: WorkflowId,
       pub step_id: StepId,
       pub name: String,
       pub content_type: String,
       pub size: u64,
       pub created_at: DateTime<Utc>,
   }
   ```

3. Implement `LocalArtifactStore`:
   - Base path from config
   - Directory structure: `{base}/{workflow_id}/{step_id}/{artifact_id}`
   - Metadata in database, content on filesystem
4. Add artifact support to `StepContext`:

   ```rust
   impl StepContext {
       pub async fn store_artifact(&self, name: &str, content: &[u8], content_type: &str) -> Result<ArtifactRef, StepError>;
       pub async fn retrieve_artifact(&self, reference: &ArtifactRef) -> Result<Vec<u8>, StepError>;
   }
   ```

5. Add API endpoints for artifact access:
   - `GET /workflows/{id}/artifacts/{artifact_id}` - Download artifact
   - `GET /workflows/{id}/artifacts/{artifact_id}/metadata` - Get metadata only

**Acceptance Criteria**:

- [ ] Artifacts store and retrieve correctly
- [ ] Metadata persists in database
- [ ] Content persists on filesystem
- [ ] API serves artifact downloads

**Files to Create**:

```
ecl-core/src/
├── artifacts/
│   ├── mod.rs
│   ├── store.rs       (ArtifactStore trait)
│   ├── local.rs       (LocalArtifactStore)
│   └── types.rs       (Artifact, ArtifactRef)
```

---

### Stage 3.6: Documentation

**Goal**: Comprehensive documentation for users and developers.

**Tasks**:

1. Create user documentation:
   - Getting Started guide
   - Installation instructions
   - Configuration reference
   - CLI reference
   - API reference
2. Create developer documentation:
   - Architecture overview
   - Creating custom steps
   - Creating custom workflows
   - Contributing guidelines
3. Add rustdoc comments to all public APIs
4. Generate API documentation:
   - `cargo doc` for Rust docs
   - OpenAPI spec rendered as HTML
5. Create example workflows:
   - Critique-Revise (toy)
   - Document summarizer (simple real-world)
   - Code review (complex)
6. Add inline code examples in documentation

**Acceptance Criteria**:

- [ ] All public APIs have rustdoc
- [ ] Getting Started works for new users
- [ ] Examples compile and run
- [ ] Documentation builds without warnings

**Files to Create**:

```
docs/
├── getting-started.md
├── installation.md
├── configuration.md
├── cli-reference.md
├── api-reference.md
├── architecture.md
├── creating-steps.md
├── creating-workflows.md
├── CONTRIBUTING.md
└── examples/
    ├── critique-revise.md
    ├── document-summarizer.md
    └── code-review.md
```

---


## Operator disposition — 2026-11-21

Cancelled as prospective work in favor of project06-pipeline. Preserve this historical record; do not issue or resume its old assignments.
