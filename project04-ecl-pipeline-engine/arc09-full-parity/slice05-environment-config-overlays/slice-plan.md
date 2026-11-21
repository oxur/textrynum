# 6. Milestone 10.5: Environment Config Overlays

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 799-917. Original bytes are recoverable from import commit 2d006a7. -->

## 6. Milestone 10.5: Environment Config Overlays

### 6.1 Scope

Support running one base pipeline spec across multiple environments (sandbox, testing, production) with per-environment overrides for bucket names, topics, credentials, etc.

### 6.2 Design

**Approach: TOML inheritance with override files.**

```
pipelines/
├── acme-inc-327.toml         # Base spec
├── acme-inc-327.sand.toml    # Sandbox overrides
├── acme-inc-327.test.toml    # Testing overrides
└── acme-inc-327.prod.toml    # Production overrides
```

The CLI command:
```bash
ecl pipeline run acme-inc-327.toml --env prod
```

Loads the base spec, then deep-merges the environment override file on top.

### 6.3 Override File Format

Override files contain only the fields that differ:

```toml
# acme-inc-327.prod.toml
[sources.encrypted-files.config]
bucket = "byn-prod-merchant-327-acme-inc"

[stages.kafka-receipts.params]
topic = "by_production_327_acme_inc_receipt-preprocess_canonical_batch_avro_0"
bootstrap_servers = "${KAFKA_BROKERS_PROD}"

[secrets]
provider = "gcp_secret_manager"
project = "pipeline-production-deploy-env"
```

### 6.4 Implementation

#### File: `crates/ecl-pipeline-spec/src/overlay.rs` (new)

```rust
use serde_json::Value;

/// Deep-merge two JSON values. `overlay` values take precedence.
/// Objects are recursively merged. Non-object values are replaced entirely.
pub fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_val) in overlay_map {
                let base_val = base_map.entry(key.clone()).or_insert(Value::Null);
                deep_merge(base_val, overlay_val);
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// Load a PipelineSpec with optional environment overlay.
pub fn load_with_overlay(
    base_toml: &str,
    overlay_toml: Option<&str>,
) -> Result<PipelineSpec> {
    let mut base: Value = toml::from_str(base_toml)?;

    if let Some(overlay_str) = overlay_toml {
        let overlay: Value = toml::from_str(overlay_str)?;
        deep_merge(&mut base, &overlay);
    }

    let spec: PipelineSpec = serde_json::from_value(base)?;
    spec.validate()?;
    Ok(spec)
}
```

#### CLI integration:

```rust
// In run.rs:
pub async fn execute(config_path: PathBuf, env: Option<String>) -> Result<()> {
    let base_toml = tokio::fs::read_to_string(&config_path).await?;

    let overlay_toml = if let Some(env_name) = &env {
        let overlay_path = config_path.with_extension(format!("{env_name}.toml"));
        if overlay_path.exists() {
            Some(tokio::fs::read_to_string(&overlay_path).await?)
        } else {
            None
        }
    } else {
        None
    };

    let spec = overlay::load_with_overlay(&base_toml, overlay_toml.as_deref())?;
    // ... rest of run logic ...
}
```

### 6.5 Tests

1. `test_deep_merge_simple_override` — string value replaced
2. `test_deep_merge_nested_object` — only changed fields replaced
3. `test_deep_merge_add_new_field` — overlay adds field not in base
4. `test_deep_merge_array_replaced` — arrays replaced entirely (not merged)
5. `test_load_with_overlay_prod`
6. `test_load_with_overlay_none` — no overlay = base spec
7. `test_load_with_overlay_invalid_toml`

---

