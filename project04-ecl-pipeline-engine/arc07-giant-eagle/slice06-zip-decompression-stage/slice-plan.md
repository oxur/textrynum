# 7. Milestone 8.6: ZIP Decompression Stage

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1289-1401. Original bytes are recoverable from import commit 3a39aea. -->

## 7. Milestone 8.6: ZIP Decompression Stage

### 7.1 Scope

Decompress ZIP archives, producing one PipelineItem per extracted file (fan-out).

### 7.2 Implementation

#### File: `crates/ecl-stages/src/decompress.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DecompressConfig {
    /// Supported formats: "zip", "gzip". Default: "zip"
    #[serde(default = "default_zip")]
    pub format: String,
    /// File extension filter for extracted files (empty = all).
    #[serde(default)]
    pub extensions: Vec<String>,
}

#[derive(Debug)]
pub struct DecompressStage {
    config: DecompressConfig,
}

#[async_trait]
impl Stage for DecompressStage {
    fn name(&self) -> &str { "decompress" }

    async fn process(
        &self,
        item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        match self.config.format.as_str() {
            "zip" => self.decompress_zip(item),
            "gzip" => self.decompress_gzip(item),
            _ => Err(StageError::Permanent { /* unsupported format */ }),
        }
    }
}

impl DecompressStage {
    fn decompress_zip(&self, item: PipelineItem) -> Result<Vec<PipelineItem>, StageError> {
        use std::io::Cursor;

        let cursor = Cursor::new(item.content.as_ref());
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| StageError::Permanent { /* ... */ })?;

        let mut results = Vec::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| StageError::Permanent { /* ... */ })?;

            if file.is_dir() { continue; }

            let name = file.name().to_string();

            // Extension filter.
            if !self.config.extensions.is_empty() {
                let ext = std::path::Path::new(&name)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if !self.config.extensions.iter().any(|e| e == ext) {
                    continue;
                }
            }

            let mut content = Vec::new();
            std::io::Read::read_to_end(&mut file, &mut content)
                .map_err(|e| StageError::Permanent { /* ... */ })?;

            results.push(PipelineItem {
                id: format!("{}:{}", item.id, name),
                display_name: name.clone(),
                content: Arc::from(content.as_slice()),
                mime_type: mime_from_extension(&name),
                stream: item.stream.clone(),
                record: None,
                ..item.clone()
            });
        }

        Ok(results)
    }
}
```

**Dependencies:**

```toml
zip = "2"
flate2 = "1"  # for gzip
```

**Note:** `zip::ZipArchive::new` is synchronous but operates on in-memory `Cursor<&[u8]>`, so it's CPU-bound not I/O-bound. For large archives, consider `spawn_blocking`. For typical Giant Eagle files (few MB), inline is fine.

### 7.3 Tests

1. `test_decompress_zip_basic` — 3 files in archive → 3 items
2. `test_decompress_zip_extension_filter` — only .csv extracted
3. `test_decompress_zip_skips_directories`
4. `test_decompress_zip_preserves_stream_tag`
5. `test_decompress_zip_fan_out_ids` — `{parent_id}:{filename}`
6. `test_decompress_zip_empty_archive` — returns empty vec
7. `test_decompress_zip_invalid_archive` — returns Permanent error
8. `test_decompress_gzip_basic`

---

