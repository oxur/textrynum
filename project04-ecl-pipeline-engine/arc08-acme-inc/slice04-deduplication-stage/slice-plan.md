# 5. Milestone 9.4: Deduplication Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0028-phase-3-detailed-implementation-plan-make-acme-inc-work.md, lines 873-1026. Original bytes are recoverable from import commit 2d006a7. -->

## 5. Milestone 9.4: Deduplication Stage

### 5.1 Scope

Batch stage that groups files by pattern-derived keys, compares content hashes, and keeps only the newest unique file per group.

### 5.2 Implementation

#### File: `crates/ecl-stages/src/deduplicate.rs` (new)

**Configuration:**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DeduplicateConfig {
    /// Regex pattern with named groups to extract grouping key from filename.
    /// Example: "wag_banyan_(?P<type>\\w+)_file_(?P<date>\\d{8})(?P<time>\\d{6}).csv"
    pub filename_pattern: String,
    /// Named groups to use as the dedup key.
    /// Example: ["type", "date"]
    pub group_by: Vec<String>,
    /// Hash algorithm: "md5", "blake3". Default: "md5"
    #[serde(default = "default_md5")]
    pub hash_algorithm: String,
    /// Which file to keep when duplicates found: "newest", "oldest". Default: "newest"
    #[serde(default = "default_newest")]
    pub keep: String,
}
```

**Stage implementation:**

```rust
#[derive(Debug)]
pub struct DeduplicateStage {
    config: DeduplicateConfig,
    pattern: regex::Regex,
}

impl DeduplicateStage {
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: DeduplicateConfig = serde_json::from_value(params.clone())
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        let pattern = regex::Regex::new(&config.filename_pattern)
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        Ok(Self { config, pattern })
    }
}

#[async_trait]
impl Stage for DeduplicateStage {
    fn name(&self) -> &str { "deduplicate" }
    fn requires_batch(&self) -> bool { true }

    async fn process(
        &self,
        _item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        Err(StageError::Permanent {
            stage: "deduplicate".to_string(),
            item_id: String::new(),
            message: "deduplicate requires batch mode".to_string(),
        })
    }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Group items by extracted key.
        let mut groups: BTreeMap<String, Vec<PipelineItem>> = BTreeMap::new();
        let mut ungrouped: Vec<PipelineItem> = Vec::new();

        for item in items {
            if let Some(key) = self.extract_group_key(&item.display_name) {
                groups.entry(key).or_default().push(item);
            } else {
                // Files not matching the pattern pass through.
                ungrouped.push(item);
            }
        }

        // 2. Within each group, deduplicate by content hash.
        let mut results: Vec<PipelineItem> = ungrouped;

        for (_key, mut group) in groups {
            // Sort by timestamp extracted from filename (newest first or oldest first).
            if self.config.keep == "newest" {
                group.sort_by(|a, b| b.display_name.cmp(&a.display_name));
            } else {
                group.sort_by(|a, b| a.display_name.cmp(&b.display_name));
            }

            // Keep unique hashes only.
            let mut seen_hashes = std::collections::HashSet::new();
            for item in group {
                let hash = self.compute_hash(&item.content);
                if seen_hashes.insert(hash) {
                    results.push(item);
                } else {
                    tracing::info!(
                        item_id = %item.id,
                        display_name = %item.display_name,
                        "deduplicated (duplicate content hash)"
                    );
                }
            }
        }

        Ok(results)
    }
}

impl DeduplicateStage {
    fn extract_group_key(&self, filename: &str) -> Option<String> {
        let caps = self.pattern.captures(filename)?;
        let key_parts: Vec<&str> = self.config.group_by.iter()
            .filter_map(|name| caps.name(name).map(|m| m.as_str()))
            .collect();
        if key_parts.is_empty() { None } else { Some(key_parts.join("|")) }
    }

    fn compute_hash(&self, content: &[u8]) -> String {
        match self.config.hash_algorithm.as_str() {
            "md5" => format!("{:x}", md5::compute(content)),
            "blake3" => blake3::hash(content).to_hex().to_string(),
            _ => blake3::hash(content).to_hex().to_string(),
        }
    }
}
```

**Dependencies:**

```toml
md5 = "0.7"
```

### 5.3 Tests

1. `test_dedup_no_duplicates` — 3 unique files → 3 output
2. `test_dedup_exact_duplicate_removed` — 2 files same hash → 1 output
3. `test_dedup_keeps_newest` — newest timestamp file kept
4. `test_dedup_keeps_oldest` — config keep="oldest"
5. `test_dedup_groups_by_type_and_date` — transaction + item files grouped separately
6. `test_dedup_unmatched_pattern_passes_through`
7. `test_dedup_acme_inc_naming` — `wag_banyan_transaction_file_20260315143022.csv`
8. `test_dedup_requires_batch`
9. `test_dedup_empty_input`

---

