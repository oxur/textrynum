# 6. Milestone 8.5: Date/Time & Timezone Stages

> Historical slice plan, reorganized on 2026-11-21. The source text and its completion claims are historical evidence, not current instructions or independently verified acceptance. Original paths inside the text are intentionally unchanged.

<!-- Historical source: crates/design/dev/ecl/0027-phase-2-detailed-implementation-plan-make-giant-eagle-work.md, lines 1201-1288. Original bytes are recoverable from import commit 2d006a7. -->

## 6. Milestone 8.5: Date/Time & Timezone Stages

### 6.1 Scope

Two stages: `DateParseStage` (string → RFC3339) and `TimezoneStage` (local → UTC by ZIP code).

### 6.2 DateParseStage

#### File: `crates/ecl-stages/src/date_parse.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DateParseConfig {
    pub conversions: Vec<DateConversion>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DateConversion {
    /// Source field containing the date string.
    pub field: String,
    /// Output field for the parsed RFC3339 datetime.
    pub output: String,
    /// strftime format string (e.g., "%m/%d/%Y", "%Y-%m-%dT%H:%M:%S").
    pub format: String,
    /// Assumed timezone if the input has no timezone.
    /// "UTC", "US/Eastern", or IANA timezone name.
    #[serde(default = "default_utc")]
    pub assume_timezone: String,
}
```

**Implementation:** Parse date string with `chrono::NaiveDateTime::parse_from_str` (or `NaiveDate` for date-only formats), attach timezone, convert to UTC, format as RFC3339.

### 6.3 TimezoneStage

#### File: `crates/ecl-stages/src/timezone.rs` (new)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct TimezoneConfig {
    /// Field containing the datetime string (already parsed).
    pub datetime_field: String,
    /// Field containing the ZIP code for timezone lookup.
    pub zipcode_field: String,
    /// Output field for UTC datetime.
    pub output: String,
    /// Fallback timezone if ZIP lookup fails.
    #[serde(default = "default_us_eastern")]
    pub fallback_timezone: String,
    /// Special overrides: { "5995": "UTC" } for online stores.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Override key field (e.g., "store_id" to match against overrides).
    #[serde(default)]
    pub override_key_field: Option<String>,
}
```

**Implementation:** Use an embedded US ZIP → timezone lookup table (compile-time or loaded from a CSV resource). The `uszipcode` equivalent in Rust is `zipcode` crate or an embedded table.

**Simpler approach for Phase 2:** Ship a static `HashMap<String, String>` of US ZIP prefixes → timezone strings (the first 3 digits of a ZIP determine the timezone with high accuracy). ~1000 entries covers all US ZIPs.

**Dependencies:**

```toml
chrono-tz = "0.10"  # IANA timezone support
```

### 6.4 Tests

**DateParse:**

1. `test_date_parse_mm_dd_yyyy`
2. `test_date_parse_iso8601`
3. `test_date_parse_iso8601_millis` — digital file timestamps
4. `test_date_parse_invalid_returns_null`
5. `test_date_parse_assume_timezone`

**Timezone:**

1. `test_timezone_us_eastern_to_utc`
2. `test_timezone_us_pacific_to_utc`
3. `test_timezone_override_store_5995` — store ID → UTC
4. `test_timezone_fallback_on_unknown_zip`
5. `test_timezone_zip_prefix_lookup`

---

