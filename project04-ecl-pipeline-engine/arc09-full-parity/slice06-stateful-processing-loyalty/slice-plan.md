# 7. Milestone 10.6: Stateful Processing (Loyalty)

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0029-phase-4-detailed-implementation-plan-full-parity.md, lines 918-1121. Original bytes are recoverable from import commit 2d006a7. -->

## 7. Milestone 10.6: Stateful Processing (Loyalty)

### 7.1 Scope

Support Acme Inc. loyalty pipeline: database-backed state for diffing consumer tokens across runs. This is the most complex Phase 4 milestone.

### 7.2 Design

The loyalty pipeline needs to:
1. Load loyalty records from CSV
2. Compare against previously-known state (in a database)
3. Compute diffs: new, changed, deleted, unchanged
4. Emit appropriate Kafka messages (insert, update, delete)
5. Update the database state

**New concept: `StatefulStage`** — a batch stage with access to a persistent key-value store that survives across pipeline runs.

### 7.3 State Store Extension

#### File: `crates/ecl-pipeline-state/src/store.rs`

Add a key-value interface to the `StateStore` trait:

```rust
#[async_trait]
pub trait StateStore: Send + Sync {
    // ... existing checkpoint/hash methods ...

    /// Get a value from the named state table.
    async fn state_get(&self, table: &str, key: &str) -> Result<Option<Vec<u8>>, StateError>;

    /// Set a value in the named state table.
    async fn state_set(&self, table: &str, key: &str, value: &[u8]) -> Result<(), StateError>;

    /// Delete a key from the named state table.
    async fn state_delete(&self, table: &str, key: &str) -> Result<(), StateError>;

    /// Iterate all keys in a named state table.
    async fn state_keys(&self, table: &str) -> Result<Vec<String>, StateError>;

    /// Batch set multiple keys atomically.
    async fn state_batch_set(
        &self,
        table: &str,
        entries: &BTreeMap<String, Vec<u8>>,
    ) -> Result<(), StateError>;
}
```

The `RedbStateStore` implements these using additional redb tables (one per named state table).

### 7.4 Stateful Diff Stage

#### File: `crates/ecl-stages/src/stateful_diff.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct StatefulDiffConfig {
    /// State table name (persisted across runs).
    pub table: String,
    /// Key field in the record (e.g., "loyalty_id_sha").
    pub key_field: String,
    /// Value field to compare (e.g., "computed_cat").
    pub value_field: String,
    /// Output field indicating the diff action: "insert", "update", "delete", "unchanged".
    #[serde(default = "default_diff_action")]
    pub action_field: String,
    /// Field to store the previous value for updates.
    #[serde(default)]
    pub previous_value_field: Option<String>,
}

#[derive(Debug)]
pub struct StatefulDiffStage {
    config: StatefulDiffConfig,
    store: Arc<dyn StateStore>,
}

#[async_trait]
impl Stage for StatefulDiffStage {
    fn name(&self) -> &str { "stateful_diff" }
    fn requires_batch(&self) -> bool { true }

    async fn process_batch(
        &self,
        items: Vec<PipelineItem>,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        // 1. Load all existing keys from state table.
        let existing_keys = self.store.state_keys(&self.config.table).await
            .map_err(|e| StageError::Permanent { /* ... */ })?;
        let existing_set: HashSet<String> = existing_keys.into_iter().collect();

        // 2. Process each incoming record.
        let mut results = Vec::new();
        let mut seen_keys = HashSet::new();
        let mut updates: BTreeMap<String, Vec<u8>> = BTreeMap::new();

        for mut item in items {
            let record = item.record.as_mut()
                .ok_or_else(|| StageError::Permanent { /* ... */ })?;

            let key = record.get(&self.config.key_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let new_value = record.get(&self.config.value_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            seen_keys.insert(key.clone());

            // Look up existing value.
            let existing_value = self.store.state_get(&self.config.table, &key).await
                .map_err(|e| StageError::Permanent { /* ... */ })?
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string());

            let action = match (&existing_value, new_value.is_empty()) {
                (None, false) => "insert",
                (None, true) => {
                    // No existing, no new → skip
                    continue;
                }
                (Some(_), true) => "delete",
                (Some(existing), false) if existing == &new_value => {
                    // Unchanged → skip
                    continue;
                }
                (Some(_), false) => "update",
            };

            record.insert(self.config.action_field.clone(), json!(action));
            if let Some(ref prev_field) = self.config.previous_value_field {
                record.insert(prev_field.clone(), json!(existing_value));
            }

            // Update state store.
            if action == "delete" {
                // Will delete after processing.
            } else {
                updates.insert(key.clone(), new_value.into_bytes());
            }

            results.push(item);
        }

        // 3. Find keys that exist in state but not in current batch → deletes.
        for existing_key in &existing_set {
            if !seen_keys.contains(existing_key) {
                // Key disappeared → generate delete record.
                let existing_value = self.store.state_get(&self.config.table, existing_key).await
                    .map_err(|e| StageError::Permanent { /* ... */ })?;

                let mut record = Record::new();
                record.insert(self.config.key_field.clone(), json!(existing_key));
                record.insert(self.config.action_field.clone(), json!("delete"));
                if let Some(ref prev_field) = self.config.previous_value_field {
                    record.insert(prev_field.clone(), json!(
                        existing_value.map(|b| String::from_utf8_lossy(&b).to_string())
                    ));
                }

                results.push(PipelineItem {
                    id: format!("diff:delete:{existing_key}"),
                    display_name: format!("Delete {existing_key}"),
                    record: Some(record),
                    content: Arc::from(Vec::new().as_slice()),
                    // ... other fields
                });
            }
        }

        // 4. Persist state updates atomically.
        self.store.state_batch_set(&self.config.table, &updates).await
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        // 5. Delete removed keys.
        for key in &existing_set {
            if !seen_keys.contains(key) {
                self.store.state_delete(&self.config.table, key).await
                    .map_err(|e| StageError::Permanent { /* ... */ })?;
            }
        }

        Ok(results)
    }
}
```

### 7.5 Tests

1. `test_stateful_diff_first_run_all_inserts`
2. `test_stateful_diff_no_changes_empty_output`
3. `test_stateful_diff_value_changed_update`
4. `test_stateful_diff_key_disappeared_delete`
5. `test_stateful_diff_new_and_changed_mixed`
6. `test_stateful_diff_empty_value_triggers_delete`
7. `test_stateful_diff_state_persists_across_calls`
8. `test_stateful_diff_acme_inc_loyalty_scenario`

---

