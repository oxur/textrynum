# 5. Milestone 8.4: Lookup Table Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1089-1200. Original bytes are recoverable from import commit 2d006a7. -->

## 5. Milestone 8.4: Lookup Table Stage

### 5.1 Scope

Static value mapping for payment types, schemes, and other enumerated fields.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/lookup.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct LookupConfig {
    pub lookups: Vec<LookupOp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LookupOp {
    /// Source field to look up.
    pub field: String,
    /// Output field for the mapped value.
    pub output: String,
    /// Lookup table: { input_value: output_value }
    pub table: BTreeMap<String, String>,
    /// Default value if input not found in table.
    #[serde(default)]
    pub default: Option<String>,
    /// Case-insensitive matching. Default: false.
    #[serde(default)]
    pub case_insensitive: bool,
}

#[derive(Debug)]
pub struct LookupStage {
    config: LookupConfig,
    /// Pre-built lookup tables (optionally lowercased keys for case-insensitive).
    tables: Vec<HashMap<String, String>>,
}

impl LookupStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: LookupConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let tables = config.lookups.iter().map(|op| {
            if op.case_insensitive {
                op.table.iter()
                    .map(|(k, v)| (k.to_lowercase(), v.clone()))
                    .collect()
            } else {
                op.table.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            }
        }).collect();

        Ok(Self { config, tables })
    }
}

#[async_trait]
impl Stage for LookupStage {
    fn name(&self) -> &str { "lookup" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        for (i, op) in self.config.lookups.iter().enumerate() {
            let input_value = record.get(&op.field)
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let lookup_key = if op.case_insensitive {
                input_value.to_lowercase()
            } else {
                input_value.to_string()
            };

            let output_value = self.tables[i].get(&lookup_key)
                .cloned()
                .or_else(|| op.default.clone());

            if let Some(val) = output_value {
                record.insert(op.output.clone(), Value::String(val));
            } else {
                record.insert(op.output.clone(), Value::Null);
            }
        }

        Ok(vec![item])
    }
}
```

### 5.3 Tests

1. `test_lookup_basic_match`
2. `test_lookup_no_match_default`
3. `test_lookup_no_match_no_default_null`
4. `test_lookup_case_insensitive`
5. `test_lookup_multiple_lookups_in_one_stage`
6. `test_lookup_ge_payment_types` — full Giant Eagle payment mapping
7. `test_lookup_ge_payment_schemes` — Visa→VI, MC→MC, etc.
8. `test_lookup_missing_field_uses_empty_string`

---

