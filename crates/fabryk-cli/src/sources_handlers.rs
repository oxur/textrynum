//! Handler functions for sources CLI commands.
//!
//! Provides reusable source management commands (scan, validate, alias) that
//! domain applications can integrate into their CLI. The handlers accept
//! explicit parameters (`content_path`, `categories`, `config_path`) rather
//! than a project-specific config, so callers can supply values from their
//! own configuration.

use std::collections::HashMap;
use std::path::Path;

use clap::{Parser, Subcommand};

use fabryk_content::sources::{
    ScanStats, SourceCategory, SourceReference, ValidationMode, ValidationReport,
    scan_content_for_sources_with_stats, validate_sources,
};
use fabryk_core::{Error, Result};

// ============================================================================
// Command types
// ============================================================================

/// Source management commands.
#[derive(Parser, Debug)]
pub struct SourcesCommand {
    /// Sources subcommand to execute.
    #[command(subcommand)]
    pub command: SourcesSubcommand,
}

/// Available sources subcommands.
#[derive(Subcommand, Debug)]
pub enum SourcesSubcommand {
    /// Scan content files for source references.
    Scan {
        /// Output format: table or json.
        #[arg(long, default_value = "table")]
        output: String,
        /// Show list of cards referencing each source.
        #[arg(long)]
        show_cards: bool,
    },
    /// Validate sources against config and filesystem.
    Validate {
        /// Validation mode: all, cards-config, cards-fs, config-fs.
        #[arg(long, default_value = "all")]
        mode: String,
        /// Show fuzzy match suggestions.
        #[arg(long)]
        suggest_matches: bool,
        /// Similarity threshold (0.0-1.0).
        #[arg(long, default_value = "0.7")]
        threshold: f32,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Manage source title aliases.
    Alias(AliasCommand),
}

/// Alias-specific subcommands.
#[derive(Parser, Debug)]
pub struct AliasCommand {
    /// Alias subcommand to execute.
    #[command(subcommand)]
    pub command: AliasSubcommand,
}

/// Available alias subcommands.
#[derive(Subcommand, Debug)]
pub enum AliasSubcommand {
    /// List all configured aliases.
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Add an alias for a source.
    Add {
        /// Source ID (e.g., "general-lewin-gmit").
        source_id: String,
        /// Alias string to add.
        alias: String,
    },
    /// Remove an alias.
    Remove {
        /// Source ID (e.g., "general-lewin-gmit").
        source_id: String,
        /// Alias string to remove.
        alias: String,
    },
}

// ============================================================================
// Command dispatch
// ============================================================================

/// Handle a sources subcommand.
///
/// The caller provides content and config paths plus the loaded category
/// configuration from its own config file. This keeps config loading out
/// of the shared handler.
///
/// # Arguments
///
/// * `cmd` - The parsed sources command
/// * `content_path` - Path to the concept cards directory
/// * `categories` - Source category configuration
/// * `config_path` - Path to the TOML config file (needed for alias edits)
///
/// # Errors
///
/// Returns `Err` on I/O failures, invalid arguments, or config parse errors.
pub async fn handle_sources(
    cmd: SourcesCommand,
    content_path: &Path,
    categories: &HashMap<String, SourceCategory>,
    config_path: Option<&Path>,
) -> Result<()> {
    match cmd.command {
        SourcesSubcommand::Scan {
            output,
            show_cards,
        } => handle_scan(content_path, &output, show_cards).await,
        SourcesSubcommand::Validate {
            mode,
            suggest_matches,
            threshold,
            json,
        } => {
            let validation_mode = parse_validation_mode(&mode)?;
            handle_validate(
                content_path,
                categories,
                validation_mode,
                suggest_matches,
                threshold,
                json,
            )
            .await
        }
        SourcesSubcommand::Alias(alias_cmd) => {
            let cfg_path = config_path.ok_or_else(|| {
                Error::config("No config path provided; alias commands require a config file")
            })?;
            handle_alias(alias_cmd, cfg_path, categories).await
        }
    }
}

// ============================================================================
// Scan handler
// ============================================================================

/// Scan content for source references and display results.
async fn handle_scan(content_path: &Path, output_format: &str, show_cards: bool) -> Result<()> {
    let (sources, stats) = scan_content_for_sources_with_stats(content_path).await?;

    match output_format {
        "json" => print_scan_json(&sources, &stats),
        _ => print_scan_table(&sources, &stats, show_cards),
    }

    Ok(())
}

/// Format scan results as a table.
fn print_scan_table(
    sources: &HashMap<String, SourceReference>,
    stats: &ScanStats,
    show_cards: bool,
) {
    println!("Source Scan Results");
    println!("===================");
    println!(
        "Cards scanned: {}  |  With sources: {}  |  Unique sources: {}",
        stats.total_cards, stats.cards_with_sources, stats.unique_sources
    );
    println!();

    if sources.is_empty() {
        println!("  (no source references found)");
        return;
    }

    // Sort by card count descending, then alphabetically.
    let mut entries: Vec<_> = sources.iter().collect();
    entries.sort_by(|a, b| b.1.card_count().cmp(&a.1.card_count()).then(a.0.cmp(b.0)));

    // Column widths.
    let title_width = 50;
    println!(
        "  {:<width$}  {:>5}",
        "Source Title",
        "Cards",
        width = title_width
    );
    println!("  {:-<width$}  {:->5}", "", "", width = title_width);

    for (title, reference) in &entries {
        println!(
            "  {:<width$}  {:>5}",
            truncate_string(title, title_width),
            reference.card_count(),
            width = title_width
        );
        if show_cards {
            for card_id in &reference.card_ids {
                println!("    - {card_id}");
            }
        }
    }
}

/// Format scan results as JSON.
fn print_scan_json(sources: &HashMap<String, SourceReference>, stats: &ScanStats) {
    let json_sources: Vec<serde_json::Value> = {
        let mut entries: Vec<_> = sources.iter().collect();
        entries.sort_by(|a, b| b.1.card_count().cmp(&a.1.card_count()).then(a.0.cmp(b.0)));
        entries
            .iter()
            .map(|(title, reference)| {
                serde_json::json!({
                    "title": title,
                    "card_count": reference.card_count(),
                    "card_ids": reference.card_ids,
                })
            })
            .collect()
    };

    let output = serde_json::json!({
        "stats": {
            "total_cards": stats.total_cards,
            "cards_with_sources": stats.cards_with_sources,
            "unique_sources": stats.unique_sources,
        },
        "sources": json_sources,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

// ============================================================================
// Validate handler
// ============================================================================

/// Parse a validation mode string into a `ValidationMode`.
fn parse_validation_mode(mode: &str) -> Result<ValidationMode> {
    match mode {
        "all" => Ok(ValidationMode::All),
        "cards-config" => Ok(ValidationMode::CardsConfig),
        "cards-fs" => Ok(ValidationMode::CardsFilesystem),
        "config-fs" => Ok(ValidationMode::ConfigFilesystem),
        other => Err(Error::config(format!(
            "Unknown validation mode: '{other}'. Expected: all, cards-config, cards-fs, config-fs"
        ))),
    }
}

/// Validate sources and print a report.
async fn handle_validate(
    content_path: &Path,
    categories: &HashMap<String, SourceCategory>,
    mode: ValidationMode,
    suggest_matches: bool,
    threshold: f32,
    json: bool,
) -> Result<()> {
    let report = validate_sources(content_path, categories, mode, suggest_matches, threshold).await?;

    if json {
        print_validation_json(&report);
    } else {
        print_validation_table(&report);
    }

    if report.is_valid() {
        Ok(())
    } else {
        Err(Error::operation(format!(
            "Validation found {} issue(s)",
            report.missing_from_config.len() + report.missing_from_filesystem.len()
        )))
    }
}

/// Print validation report as a table.
fn print_validation_table(report: &ValidationReport) {
    println!("Source Validation Report");
    println!("========================");
    println!("  Cards scanned:       {}", report.stats.total_cards_scanned);
    println!(
        "  Unique sources:      {}",
        report.stats.unique_sources_found
    );
    println!(
        "  Sources in config:   {}",
        report.stats.sources_in_config
    );
    println!("  Resolved:            {}", report.stats.sources_resolved);
    println!("  On disk:             {}", report.stats.sources_on_disk);
    println!(
        "  Missing from config: {}",
        report.stats.missing_from_config
    );
    println!(
        "  Missing from disk:   {}",
        report.stats.missing_from_disk
    );

    if !report.missing_from_config.is_empty() {
        println!("\nSources in cards but NOT in config:");
        for missing in &report.missing_from_config {
            println!(
                "  - \"{}\" ({} card(s))",
                missing.title, missing.card_count
            );
            if !missing.sample_card_ids.is_empty() {
                let ids = missing.sample_card_ids.join(", ");
                println!("    Cards: {ids}");
            }
            for suggestion in &missing.suggestions {
                println!(
                    "    -> Did you mean \"{}\"? (similarity: {:.0}%)",
                    suggestion.title,
                    suggestion.similarity * 100.0
                );
            }
        }
    }

    if !report.missing_from_filesystem.is_empty() {
        println!("\nSources in config but NOT on filesystem:");
        for missing in &report.missing_from_filesystem {
            println!(
                "  - {} [{}]: {}",
                missing.config_id,
                missing.category,
                missing.expected_path.display()
            );
        }
    }

    if report.is_valid() {
        println!("\nAll sources validated successfully.");
    }
}

/// Print validation report as JSON.
fn print_validation_json(report: &ValidationReport) {
    let missing_config: Vec<serde_json::Value> = report
        .missing_from_config
        .iter()
        .map(|m| {
            let suggestions: Vec<serde_json::Value> = m
                .suggestions
                .iter()
                .map(|s| {
                    serde_json::json!({
                        "config_id": s.config_id,
                        "title": s.title,
                        "similarity": s.similarity,
                    })
                })
                .collect();
            serde_json::json!({
                "title": m.title,
                "card_count": m.card_count,
                "sample_card_ids": m.sample_card_ids,
                "suggestions": suggestions,
            })
        })
        .collect();

    let missing_fs: Vec<serde_json::Value> = report
        .missing_from_filesystem
        .iter()
        .map(|m| {
            serde_json::json!({
                "config_id": m.config_id,
                "category": m.category,
                "expected_path": m.expected_path.display().to_string(),
            })
        })
        .collect();

    let output = serde_json::json!({
        "valid": report.is_valid(),
        "stats": report.stats,
        "missing_from_config": missing_config,
        "missing_from_filesystem": missing_fs,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

// ============================================================================
// Alias handler
// ============================================================================

/// Handle alias subcommands by reading/editing a TOML config file.
async fn handle_alias(
    cmd: AliasCommand,
    config_path: &Path,
    categories: &HashMap<String, SourceCategory>,
) -> Result<()> {
    match cmd.command {
        AliasSubcommand::List { json } => handle_alias_list(categories, json),
        AliasSubcommand::Add { source_id, alias } => {
            handle_alias_add(config_path, categories, &source_id, &alias).await
        }
        AliasSubcommand::Remove { source_id, alias } => {
            handle_alias_remove(config_path, categories, &source_id, &alias).await
        }
    }
}

/// List all configured aliases.
fn handle_alias_list(categories: &HashMap<String, SourceCategory>, json: bool) -> Result<()> {
    if json {
        let mut all_aliases: HashMap<String, &Vec<String>> = HashMap::new();
        for (category, cat) in categories {
            for (file_id, aliases) in &cat.aliases {
                if !aliases.is_empty() {
                    let source_id = format!("{category}-{file_id}");
                    all_aliases.insert(source_id, aliases);
                }
            }
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&all_aliases).unwrap_or_default()
        );
    } else {
        let mut found_any = false;
        let mut cat_keys: Vec<_> = categories.keys().collect();
        cat_keys.sort();

        for category in cat_keys {
            let cat = &categories[category];
            let mut file_ids: Vec<_> = cat.aliases.keys().collect();
            file_ids.sort();

            for file_id in file_ids {
                let aliases = &cat.aliases[file_id];
                if !aliases.is_empty() {
                    found_any = true;
                    let source_id = format!("{category}-{file_id}");
                    println!("{source_id}:");
                    for alias in aliases {
                        println!("  - {alias}");
                    }
                }
            }
        }

        if !found_any {
            println!("No aliases configured.");
        }
    }

    Ok(())
}

/// Add an alias to the TOML config file.
async fn handle_alias_add(
    config_path: &Path,
    categories: &HashMap<String, SourceCategory>,
    source_id: &str,
    alias: &str,
) -> Result<()> {
    let known_categories: Vec<&str> = categories.keys().map(String::as_str).collect();
    let (category, file_id) = parse_source_id(source_id, &known_categories)?;

    // Verify the file_id exists in the category.
    let cat = categories.get(category).ok_or_else(|| {
        Error::config(format!("Category '{category}' not found in configuration"))
    })?;
    if !cat.files.contains_key(file_id) {
        return Err(Error::config(format!(
            "File ID '{file_id}' not found in category '{category}'"
        )));
    }

    // Read and parse the TOML config.
    let content = tokio::fs::read_to_string(config_path)
        .await
        .map_err(|e| Error::io_with_path(e, config_path))?;
    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| Error::config(format!("Failed to parse config TOML: {e}")))?;

    // Navigate to sources.<category>.aliases.<file_id>.
    let aliases_array = ensure_alias_array(&mut doc, category, file_id);

    // Check for duplicates.
    let already_exists = aliases_array
        .iter()
        .any(|v| v.as_str() == Some(alias));
    if already_exists {
        println!("Alias '{alias}' already exists for '{source_id}'.");
        return Ok(());
    }

    aliases_array.push(alias);

    tokio::fs::write(config_path, doc.to_string())
        .await
        .map_err(|e| Error::io_with_path(e, config_path))?;

    println!("Added alias '{alias}' for '{source_id}'.");
    Ok(())
}

/// Remove an alias from the TOML config file.
async fn handle_alias_remove(
    config_path: &Path,
    categories: &HashMap<String, SourceCategory>,
    source_id: &str,
    alias: &str,
) -> Result<()> {
    let known_categories: Vec<&str> = categories.keys().map(String::as_str).collect();
    let (category, file_id) = parse_source_id(source_id, &known_categories)?;

    // Read and parse the TOML config.
    let content = tokio::fs::read_to_string(config_path)
        .await
        .map_err(|e| Error::io_with_path(e, config_path))?;
    let mut doc: toml_edit::DocumentMut = content
        .parse()
        .map_err(|e| Error::config(format!("Failed to parse config TOML: {e}")))?;

    // Navigate to sources.<category>.aliases.<file_id>.
    let removed = {
        let sources = doc.get_mut("sources").and_then(|v| v.as_table_mut());
        let cat_table = sources
            .and_then(|s| s.get_mut(category))
            .and_then(|v| v.as_table_mut());
        let aliases_table = cat_table
            .and_then(|c| c.get_mut("aliases"))
            .and_then(|v| v.as_table_mut());
        let alias_array = aliases_table
            .and_then(|a| a.get_mut(file_id))
            .and_then(|v| v.as_value_mut())
            .and_then(|v| v.as_array_mut());

        if let Some(arr) = alias_array {
            let before_len = arr.len();
            arr.retain(|v| v.as_str() != Some(alias));
            arr.len() < before_len
        } else {
            false
        }
    };

    if removed {
        tokio::fs::write(config_path, doc.to_string())
            .await
            .map_err(|e| Error::io_with_path(e, config_path))?;
        println!("Removed alias '{alias}' from '{source_id}'.");
    } else {
        println!("Alias '{alias}' not found for '{source_id}'.");
    }

    Ok(())
}

/// Ensure the TOML path `sources.<category>.aliases.<file_id>` exists as an
/// array, creating intermediate tables as needed.
fn ensure_alias_array<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    category: &str,
    file_id: &str,
) -> &'a mut toml_edit::Array {
    // Ensure [sources]
    if !doc.contains_key("sources") {
        doc["sources"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let sources = doc["sources"].as_table_mut().expect("sources is a table");

    // Ensure [sources.<category>]
    if !sources.contains_key(category) {
        sources[category] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let cat = sources[category].as_table_mut().expect("category is a table");

    // Ensure [sources.<category>.aliases]
    if !cat.contains_key("aliases") {
        cat["aliases"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let aliases = cat["aliases"].as_table_mut().expect("aliases is a table");

    // Ensure sources.<category>.aliases.<file_id> = []
    if !aliases.contains_key(file_id) {
        aliases[file_id] = toml_edit::value(toml_edit::Array::new());
    }
    aliases[file_id]
        .as_value_mut()
        .expect("alias entry is a value")
        .as_array_mut()
        .expect("alias entry is an array")
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse a source ID into `(category, file_id)` using known category names.
///
/// The source ID format is `"{category}-{file_id}"`. Because both the category
/// and file ID may contain hyphens, we try each known category as a prefix
/// (longest first) and split on the first match.
///
/// # Examples
///
/// ```ignore
/// let known = &["general", "oxford"];
/// assert_eq!(parse_source_id("general-lewin-gmit", &known)?, ("general", "lewin-gmit"));
/// assert_eq!(parse_source_id("oxford-harmony", &known)?, ("oxford", "harmony"));
/// ```
///
/// # Errors
///
/// Returns `Err` if no known category matches the prefix.
fn parse_source_id<'a>(
    source_id: &'a str,
    known_categories: &[&str],
) -> Result<(&'a str, &'a str)> {
    // Sort by length descending so longer category names match first.
    let mut sorted: Vec<&str> = known_categories.to_vec();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));

    for category in &sorted {
        let prefix = format!("{category}-");
        if let Some(file_id) = source_id.strip_prefix(&prefix) {
            if !file_id.is_empty() {
                return Ok((&source_id[..category.len()], file_id));
            }
        }
    }

    Err(Error::config(format!(
        "Cannot parse source ID '{source_id}': no known category prefix matches. \
         Known categories: {}",
        known_categories.join(", ")
    )))
}

/// Truncate a string to `max_len` characters, appending "..." if truncated.
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        ".".repeat(max_len)
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    // ------------------------------------------------------------------------
    // parse_source_id
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_source_id_simple_category() {
        let known = &["general", "oxford"];
        let (cat, file_id) = parse_source_id("general-lewin-gmit", known).unwrap();
        assert_eq!(cat, "general");
        assert_eq!(file_id, "lewin-gmit");
    }

    #[test]
    fn test_parse_source_id_single_segment_file_id() {
        let known = &["oxford"];
        let (cat, file_id) = parse_source_id("oxford-harmony", known).unwrap();
        assert_eq!(cat, "oxford");
        assert_eq!(file_id, "harmony");
    }

    #[test]
    fn test_parse_source_id_longer_category_wins() {
        // "general-ref" is longer than "general", so it should match first.
        let known = &["general", "general-ref"];
        let (cat, file_id) = parse_source_id("general-ref-some-file", known).unwrap();
        assert_eq!(cat, "general-ref");
        assert_eq!(file_id, "some-file");
    }

    #[test]
    fn test_parse_source_id_no_match() {
        let known = &["oxford", "general"];
        let result = parse_source_id("unknown-source", known);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_source_id_empty_file_id() {
        let known = &["general"];
        let result = parse_source_id("general-", known);
        // strip_prefix yields "" which is empty, so this should fail.
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_source_id_empty_categories() {
        let known: &[&str] = &[];
        let result = parse_source_id("general-test", known);
        assert!(result.is_err());
    }

    // ------------------------------------------------------------------------
    // truncate_string
    // ------------------------------------------------------------------------

    #[test]
    fn test_truncate_string_short() {
        assert_eq!(truncate_string("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_string_exact() {
        assert_eq!(truncate_string("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_string_truncated() {
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }

    #[test]
    fn test_truncate_string_very_short_max() {
        assert_eq!(truncate_string("hello", 2), "..");
    }

    #[test]
    fn test_truncate_string_max_three() {
        assert_eq!(truncate_string("hello", 3), "...");
    }

    #[test]
    fn test_truncate_string_max_four() {
        assert_eq!(truncate_string("hello", 4), "h...");
    }

    #[test]
    fn test_truncate_string_empty() {
        assert_eq!(truncate_string("", 5), "");
    }

    // ------------------------------------------------------------------------
    // parse_validation_mode
    // ------------------------------------------------------------------------

    #[test]
    fn test_parse_validation_mode_all() {
        assert_eq!(parse_validation_mode("all").unwrap(), ValidationMode::All);
    }

    #[test]
    fn test_parse_validation_mode_cards_config() {
        assert_eq!(
            parse_validation_mode("cards-config").unwrap(),
            ValidationMode::CardsConfig
        );
    }

    #[test]
    fn test_parse_validation_mode_cards_fs() {
        assert_eq!(
            parse_validation_mode("cards-fs").unwrap(),
            ValidationMode::CardsFilesystem
        );
    }

    #[test]
    fn test_parse_validation_mode_config_fs() {
        assert_eq!(
            parse_validation_mode("config-fs").unwrap(),
            ValidationMode::ConfigFilesystem
        );
    }

    #[test]
    fn test_parse_validation_mode_invalid() {
        assert!(parse_validation_mode("invalid").is_err());
    }

    // ------------------------------------------------------------------------
    // Clap parsing: sources scan
    // ------------------------------------------------------------------------

    /// Helper to parse a sources command from CLI args.
    fn parse_sources(args: &[&str]) -> SourcesCommand {
        #[derive(Parser, Debug)]
        struct Wrapper {
            #[command(subcommand)]
            cmd: WrapperCmd,
        }

        #[derive(Subcommand, Debug)]
        enum WrapperCmd {
            Sources(SourcesCommand),
        }

        let mut full_args = vec!["test", "sources"];
        full_args.extend_from_slice(args);
        let wrapper = Wrapper::parse_from(full_args);
        match wrapper.cmd {
            WrapperCmd::Sources(s) => s,
        }
    }

    #[test]
    fn test_clap_scan_defaults() {
        let cmd = parse_sources(&["scan"]);
        match cmd.command {
            SourcesSubcommand::Scan {
                output,
                show_cards,
            } => {
                assert_eq!(output, "table");
                assert!(!show_cards);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    #[test]
    fn test_clap_scan_json_with_cards() {
        let cmd = parse_sources(&["scan", "--output", "json", "--show-cards"]);
        match cmd.command {
            SourcesSubcommand::Scan {
                output,
                show_cards,
            } => {
                assert_eq!(output, "json");
                assert!(show_cards);
            }
            _ => panic!("Expected Scan command"),
        }
    }

    // ------------------------------------------------------------------------
    // Clap parsing: sources validate
    // ------------------------------------------------------------------------

    #[test]
    fn test_clap_validate_defaults() {
        let cmd = parse_sources(&["validate"]);
        match cmd.command {
            SourcesSubcommand::Validate {
                mode,
                suggest_matches,
                threshold,
                json,
            } => {
                assert_eq!(mode, "all");
                assert!(!suggest_matches);
                assert!((threshold - 0.7).abs() < f32::EPSILON);
                assert!(!json);
            }
            _ => panic!("Expected Validate command"),
        }
    }

    #[test]
    fn test_clap_validate_custom() {
        let cmd = parse_sources(&[
            "validate",
            "--mode",
            "cards-config",
            "--suggest-matches",
            "--threshold",
            "0.85",
            "--json",
        ]);
        match cmd.command {
            SourcesSubcommand::Validate {
                mode,
                suggest_matches,
                threshold,
                json,
            } => {
                assert_eq!(mode, "cards-config");
                assert!(suggest_matches);
                assert!((threshold - 0.85).abs() < f32::EPSILON);
                assert!(json);
            }
            _ => panic!("Expected Validate command"),
        }
    }

    // ------------------------------------------------------------------------
    // Clap parsing: sources alias
    // ------------------------------------------------------------------------

    #[test]
    fn test_clap_alias_list() {
        let cmd = parse_sources(&["alias", "list"]);
        match cmd.command {
            SourcesSubcommand::Alias(AliasCommand {
                command: AliasSubcommand::List { json },
            }) => {
                assert!(!json);
            }
            _ => panic!("Expected Alias List command"),
        }
    }

    #[test]
    fn test_clap_alias_list_json() {
        let cmd = parse_sources(&["alias", "list", "--json"]);
        match cmd.command {
            SourcesSubcommand::Alias(AliasCommand {
                command: AliasSubcommand::List { json },
            }) => {
                assert!(json);
            }
            _ => panic!("Expected Alias List command with json"),
        }
    }

    #[test]
    fn test_clap_alias_add() {
        let cmd = parse_sources(&["alias", "add", "general-lewin-gmit", "GMIT"]);
        match cmd.command {
            SourcesSubcommand::Alias(AliasCommand {
                command: AliasSubcommand::Add { source_id, alias },
            }) => {
                assert_eq!(source_id, "general-lewin-gmit");
                assert_eq!(alias, "GMIT");
            }
            _ => panic!("Expected Alias Add command"),
        }
    }

    #[test]
    fn test_clap_alias_remove() {
        let cmd = parse_sources(&["alias", "remove", "general-lewin-gmit", "GMIT"]);
        match cmd.command {
            SourcesSubcommand::Alias(AliasCommand {
                command: AliasSubcommand::Remove { source_id, alias },
            }) => {
                assert_eq!(source_id, "general-lewin-gmit");
                assert_eq!(alias, "GMIT");
            }
            _ => panic!("Expected Alias Remove command"),
        }
    }

    // ------------------------------------------------------------------------
    // Scan formatting (with mock data)
    // ------------------------------------------------------------------------

    #[test]
    fn test_print_scan_table_empty() {
        // Smoke test: should not panic with empty data.
        let sources = HashMap::new();
        let stats = ScanStats::default();
        print_scan_table(&sources, &stats, false);
    }

    #[test]
    fn test_print_scan_table_with_data() {
        let mut sources = HashMap::new();
        let mut ref1 = SourceReference::new("Open Music Theory");
        ref1.add_card("card-1");
        ref1.add_card("card-2");
        sources.insert("Open Music Theory".to_string(), ref1);

        let stats = ScanStats::new(10, 1, 5);
        // Smoke test: should not panic.
        print_scan_table(&sources, &stats, true);
    }

    #[test]
    fn test_print_scan_json_with_data() {
        let mut sources = HashMap::new();
        let mut ref1 = SourceReference::new("Test Source");
        ref1.add_card("card-a");
        sources.insert("Test Source".to_string(), ref1);

        let stats = ScanStats::new(5, 1, 3);
        // Smoke test: should not panic.
        print_scan_json(&sources, &stats);
    }

    // ------------------------------------------------------------------------
    // Alias list formatting
    // ------------------------------------------------------------------------

    #[test]
    fn test_handle_alias_list_empty() {
        let categories = HashMap::new();
        let result = handle_alias_list(&categories, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_alias_list_with_data() {
        let mut aliases = HashMap::new();
        aliases.insert(
            "lewin-gmit".to_string(),
            vec!["GMIT".to_string(), "Lewin".to_string()],
        );

        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources".to_string(),
                files: HashMap::new(),
                aliases,
            },
        );

        let result = handle_alias_list(&categories, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_alias_list_json() {
        let categories = HashMap::new();
        let result = handle_alias_list(&categories, true);
        assert!(result.is_ok());
    }

    // ------------------------------------------------------------------------
    // Alias add/remove with temp config
    // ------------------------------------------------------------------------

    #[tokio::test]
    async fn test_alias_add_to_config() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        // Minimal config with a source.
        let initial = r#"
[sources.general]
path = "/sources/general"

[sources.general.files]
lewin-gmit = "[1987] Lewin - GMIT.pdf"

[sources.general.aliases]
"#;
        tokio::fs::write(&config_path, initial).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "lewin-gmit".to_string(),
            "[1987] Lewin - GMIT.pdf".to_string(),
        );
        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources/general".to_string(),
                files,
                aliases: HashMap::new(),
            },
        );

        let result =
            handle_alias_add(&config_path, &categories, "general-lewin-gmit", "GMIT").await;
        assert!(result.is_ok());

        // Verify it was written.
        let content = tokio::fs::read_to_string(&config_path).await.unwrap();
        assert!(content.contains("GMIT"));
    }

    #[tokio::test]
    async fn test_alias_add_duplicate() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let initial = r#"
[sources.general]
path = "/sources/general"

[sources.general.files]
lewin-gmit = "[1987] Lewin - GMIT.pdf"

[sources.general.aliases]
lewin-gmit = ["GMIT"]
"#;
        tokio::fs::write(&config_path, initial).await.unwrap();

        let mut files = HashMap::new();
        files.insert(
            "lewin-gmit".to_string(),
            "[1987] Lewin - GMIT.pdf".to_string(),
        );
        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources/general".to_string(),
                files,
                aliases: HashMap::new(),
            },
        );

        let result =
            handle_alias_add(&config_path, &categories, "general-lewin-gmit", "GMIT").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_alias_add_unknown_file_id() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");
        tokio::fs::write(&config_path, "").await.unwrap();

        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources".to_string(),
                files: HashMap::new(),
                aliases: HashMap::new(),
            },
        );

        let result =
            handle_alias_add(&config_path, &categories, "general-nonexistent", "alias").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_alias_remove_existing() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let initial = r#"
[sources.general]
path = "/sources/general"

[sources.general.files]
lewin-gmit = "[1987] Lewin - GMIT.pdf"

[sources.general.aliases]
lewin-gmit = ["GMIT", "Lewin"]
"#;
        tokio::fs::write(&config_path, initial).await.unwrap();

        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources/general".to_string(),
                files: HashMap::new(),
                aliases: HashMap::new(),
            },
        );

        let result =
            handle_alias_remove(&config_path, &categories, "general-lewin-gmit", "GMIT").await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&config_path).await.unwrap();
        assert!(!content.contains("\"GMIT\""));
        assert!(content.contains("Lewin"));
    }

    #[tokio::test]
    async fn test_alias_remove_nonexistent() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.toml");

        let initial = r#"
[sources.general]
path = "/sources/general"

[sources.general.aliases]
lewin-gmit = ["GMIT"]
"#;
        tokio::fs::write(&config_path, initial).await.unwrap();

        let mut categories = HashMap::new();
        categories.insert(
            "general".to_string(),
            SourceCategory {
                path: "/sources/general".to_string(),
                files: HashMap::new(),
                aliases: HashMap::new(),
            },
        );

        let result = handle_alias_remove(
            &config_path,
            &categories,
            "general-lewin-gmit",
            "NonExistent",
        )
        .await;
        assert!(result.is_ok());
    }
}
