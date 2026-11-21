# ECL Phase 4 — Hardening (historical proposal)

> Historical arc plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/docs/10-superseded/0004-ecl-project-plan.md, lines 1340-1674. Original bytes are recoverable from import commit 3a39aea. -->

## Phase 4: Hardening

**Objective**: Production readiness
**Duration**: ~2 weeks
**Deliverable**: Production-ready v1 with comprehensive error handling, testing, and deployment documentation

### Stage 4.1: Error Handling Audit

**Goal**: Comprehensive error handling throughout the codebase.

**Tasks**:

1. Audit all `unwrap()` and `expect()` calls:
   - Replace with proper error handling
   - Add context to errors using `anyhow::Context`
2. Ensure all errors are:
   - Logged with appropriate level
   - Traceable to source
   - User-friendly in API responses
3. Add error codes for all API errors
4. Implement graceful degradation:
   - LLM timeout → retry with backoff
   - Database unavailable → queue requests
   - Restate unavailable → clear error message
5. Add panic handling:
   - Catch panics in async tasks
   - Log panic details
   - Prevent cascade failures
6. Review all `?` propagation for appropriate context

**Acceptance Criteria**:

- [ ] No `unwrap()` on fallible operations
- [ ] All errors have context
- [ ] API errors have consistent format
- [ ] Panics don't crash the server

**Files to Modify**: All source files (audit)

---

### Stage 4.2: Circuit Breaker for Claude API

**Goal**: Protect against Claude API failures and prevent cascade failures.

**Tasks**:

1. Implement circuit breaker using `failsafe`:

   ```rust
   pub struct LlmCircuitBreaker {
       circuit: CircuitBreaker<LlmError>,
   }

   impl LlmCircuitBreaker {
       pub async fn call<F, T>(&self, f: F) -> Result<T, LlmError>
       where
           F: Future<Output = Result<T, LlmError>>;
   }
   ```

2. Configure circuit breaker:
   - Failure threshold: 5 failures in 60 seconds
   - Open duration: 30 seconds
   - Half-open: Allow 1 request to test recovery
3. Add metrics for circuit state:
   - Circuit open/closed/half-open events
   - Failure counts
   - Recovery time
4. Wrap `ClaudeProvider` with circuit breaker
5. Add health check endpoint that reports circuit state
6. Configure alerts for circuit open (documentation)

**Acceptance Criteria**:

- [ ] Circuit opens after threshold failures
- [ ] Circuit recovers after timeout
- [ ] Metrics accurately reflect state
- [ ] Health check reports circuit state

**Files to Create**:

```
ecl-core/src/
├── resilience/
│   ├── mod.rs
│   └── circuit_breaker.rs
```

---

### Stage 4.3: Comprehensive Test Suite

**Goal**: High confidence in system correctness.

**Tasks**:

1. Unit tests for all modules:
   - Types and serialization
   - Validation logic
   - Configuration parsing
   - Error handling
2. Integration tests:
   - Full workflow execution
   - API endpoints
   - Database operations
   - Artifact storage
3. Property-based tests using `proptest`:
   - Serialization roundtrips
   - Validation edge cases
   - ID generation uniqueness
4. Chaos tests:
   - Kill process mid-workflow
   - Simulate LLM failures
   - Database disconnection
   - Network partitions (where applicable)
5. Performance tests:
   - Workflow throughput
   - API latency
   - Memory usage
   - Database query performance
6. Add test coverage reporting

**Acceptance Criteria**:

- [ ] >80% test coverage
- [ ] All tests pass reliably
- [ ] Chaos tests verify recovery
- [ ] Performance baselines established

**Files to Create**:

```
tests/
├── unit/
│   └── (per-module tests)
├── integration/
│   ├── api/
│   ├── workflows/
│   └── database/
├── property/
│   └── types.rs
├── chaos/
│   └── recovery.rs
└── performance/
    └── benchmarks.rs
```

---

### Stage 4.4: Performance Optimization

**Goal**: Ensure system performs well under expected load.

**Tasks**:

1. Profile with `cargo flamegraph`:
   - Identify hot paths
   - Find unnecessary allocations
   - Locate blocking operations
2. Optimize database queries:
   - Add indexes for common queries
   - Use batch operations where possible
   - Optimize JSON serialization
3. Optimize LLM calls:
   - Prompt caching (if supported)
   - Response streaming
   - Parallel step execution (where dependencies allow)
4. Add connection pooling:
   - Database connection pool sizing
   - HTTP client connection reuse
5. Memory optimization:
   - Avoid cloning large structures
   - Use `Arc` for shared data
   - Stream large artifacts
6. Document performance characteristics

**Acceptance Criteria**:

- [ ] Workflow latency meets targets
- [ ] Memory usage is bounded
- [ ] No blocking operations in async code
- [ ] Performance documentation complete

**Files to Modify**: Various (optimization)

---

### Stage 4.5: Security Review

**Goal**: Ensure system is secure for production use.

**Tasks**:

1. Audit authentication:
   - API key storage (not in logs)
   - API key validation
   - Rate limiting
2. Audit authorization:
   - Workflow access control
   - Artifact access control
3. Input validation:
   - All API inputs validated
   - SQL injection prevention (parameterized queries)
   - Path traversal prevention
4. Secret management:
   - Claude API key handling
   - Database credentials
   - Config file permissions
5. Dependency audit:
   - `cargo audit` for vulnerabilities
   - Review dependency licenses
6. Add security documentation:
   - Deployment security checklist
   - Secret rotation procedures

**Acceptance Criteria**:

- [ ] No secrets in logs
- [ ] All inputs validated
- [ ] `cargo audit` passes
- [ ] Security documentation complete

**Files to Create**:

```
docs/
└── security/
    ├── deployment-checklist.md
    └── secret-management.md
```

---

### Stage 4.6: Deployment Packaging

**Goal**: Easy deployment to production environments.

**Tasks**:

1. Create Dockerfile:

   ```dockerfile
   FROM rust:1.XX as builder
   # Build steps

   FROM debian:bookworm-slim
   # Runtime image
   ```

2. Create docker-compose for full stack:
   - ECL application
   - Restate server
   - PostgreSQL
3. Create Kubernetes manifests:
   - Deployment
   - Service
   - ConfigMap
   - Secret
   - PersistentVolumeClaim (for artifacts)
4. Add health check endpoints:
   - `/health/live` - Process is running
   - `/health/ready` - Dependencies available
5. Create deployment documentation:
   - Docker deployment
   - Kubernetes deployment
   - Manual deployment
6. Add environment-specific configs

**Acceptance Criteria**:

- [ ] Docker image builds and runs
- [ ] docker-compose stack works
- [ ] K8s manifests are valid
- [ ] Deployment docs are complete

**Files to Create**:

```
Dockerfile
docker-compose.yml
docker-compose.prod.yml

deploy/
├── kubernetes/
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   └── pvc.yaml
└── docs/
    ├── docker.md
    ├── kubernetes.md
    └── manual.md
```

---

### Stage 4.7: Release Preparation

**Goal**: Prepare for v1.0 release.

**Tasks**:

1. Version all crates at 1.0.0
2. Create CHANGELOG.md with all changes
3. Update all documentation for release
4. Create release checklist:
   - [ ] All tests pass
   - [ ] Documentation complete
   - [ ] CHANGELOG updated
   - [ ] Version bumped
   - [ ] Docker image tagged
   - [ ] Release notes written
5. Tag release in git
6. Publish crates to crates.io (if public)
7. Create GitHub release with binaries
8. Announce release (internal/external)

**Acceptance Criteria**:

- [ ] All checklist items complete
- [ ] Release is tagged
- [ ] Artifacts are published
- [ ] Documentation matches release

**Files to Create**:

```
CHANGELOG.md
RELEASE.md (checklist)
```

---

