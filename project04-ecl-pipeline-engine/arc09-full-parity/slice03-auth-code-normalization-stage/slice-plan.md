# 4. Milestone 10.3: Auth Code Normalization Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 486-581. Original bytes are recoverable from import commit 3a39aea. -->

## 4. Milestone 10.3: Auth Code Normalization Stage

### 4.1 Scope

Specialized stage for authorization code cleaning — strip leading zeros, left-pad to N digits. Used by Acme Inc. (Medagate codes), Affinity, and Giant Eagle.

### 4.2 Implementation

#### File: `crates/ecl-stages/src/auth_code_norm.rs` (new)

This is simple enough that it could be a `field_map` `pad` operation, but the Acme Inc. auth code has specific logic: strip leading zeros first, THEN left-pad to 6. The `pad` operation doesn't strip first.

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct AuthCodeNormConfig {
    /// Field containing the auth code.
    pub field: String,
    /// Output field. Default: overwrites input.
    #[serde(default)]
    pub output: Option<String>,
    /// Target width for padding. Default: 6.
    #[serde(default = "default_six")]
    pub width: usize,
    /// Whether to strip leading zeros before padding. Default: true.
    #[serde(default = "default_true")]
    pub strip_leading_zeros: bool,
    /// Pad character. Default: '0'.
    #[serde(default = "default_zero")]
    pub pad_char: char,
}

#[derive(Debug)]
pub struct AuthCodeNormStage {
    config: AuthCodeNormConfig,
}

#[async_trait]
impl Stage for AuthCodeNormStage {
    fn name(&self) -> &str { "auth_code_norm" }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut()
            .ok_or_else(|| StageError::Permanent { /* ... */ })?;

        let raw = record.get(&self.config.field)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let output_field = self.config.output.as_deref()
            .unwrap_or(&self.config.field);

        if raw.is_empty() {
            return Ok(vec![item]);
        }

        let mut normalized = raw.clone();

        // Strip leading zeros (e.g., "00123" → "123").
        if self.config.strip_leading_zeros {
            normalized = normalized.trim_start_matches('0').to_string();
            if normalized.is_empty() {
                normalized = "0".to_string();
            }
        }

        // Left-pad to target width.
        if normalized.len() < self.config.width {
            let pad = self.config.pad_char.to_string()
                .repeat(self.config.width - normalized.len());
            normalized = format!("{pad}{normalized}");
        }

        record.insert(output_field.to_string(), json!(normalized));

        Ok(vec![item])
    }
}
```

### 4.3 Tests

1. `test_auth_code_strip_and_pad` — "00123" → "000123" (strip to "123", pad to 6)
2. `test_auth_code_already_correct_length` — "123456" → "123456"
3. `test_auth_code_longer_than_width` — "1234567" → "1234567" (no truncation)
4. `test_auth_code_all_zeros` — "000000" → "000000" (strip to "0", pad to 6)
5. `test_auth_code_no_strip` — strip_leading_zeros=false, "00123" → "00123"
6. `test_auth_code_empty_passthrough`
7. `test_auth_code_acme_inc_medagate` — real Medagate codes

---

