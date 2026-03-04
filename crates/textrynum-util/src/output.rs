//! Output formatting for text, JSON, and TOML.

use crate::cli::OutputFormat;
use crate::crate_info::{CrateInfo, VersionMismatch};
use anyhow::Result;

/// Format a list of crates in the requested output format.
pub fn format_crate_list(crates: &[CrateInfo], format: &OutputFormat) -> Result<String> {
    match format {
        OutputFormat::Text => Ok(format_crates_text(crates)),
        OutputFormat::Json => Ok(serde_json::to_string_pretty(crates)?),
        OutputFormat::Toml => Ok(toml::to_string_pretty(crates)?),
    }
}

fn format_crates_text(crates: &[CrateInfo]) -> String {
    let mut out = String::new();
    for info in crates {
        out.push_str(&format!("{} ({})\n", info.name, info.version));
        if info.internal_deps.is_empty() {
            out.push_str("  (no internal deps)\n");
        } else {
            for dep in &info.internal_deps {
                let optional_tag = if dep.optional { " [optional]" } else { "" };
                let section_tag = if dep.section != "dependencies" {
                    format!(" ({})", dep.section)
                } else {
                    String::new()
                };
                out.push_str(&format!(
                    "  {} = \"{}\"{}{}\n",
                    dep.name, dep.declared_version, optional_tag, section_tag
                ));
            }
        }
    }
    out
}

/// Format version mismatches as human-readable text.
pub fn format_mismatches(mismatches: &[VersionMismatch]) -> String {
    let mut out = String::new();
    for m in mismatches {
        out.push_str(&format!(
            "  {} -> {} = \"{}\" (expected \"{}\")\n",
            m.crate_name, m.dep_name, m.declared_version, m.expected_version
        ));
    }
    out
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crate_info::DepInfo;
    use std::path::PathBuf;

    fn sample_crates() -> Vec<CrateInfo> {
        vec![
            CrateInfo {
                name: "alpha".to_string(),
                path: PathBuf::from("alpha"),
                version: "0.1.1".to_string(),
                publish: true,
                internal_deps: vec![],
            },
            CrateInfo {
                name: "beta".to_string(),
                path: PathBuf::from("beta"),
                version: "0.1.1".to_string(),
                publish: true,
                internal_deps: vec![
                    DepInfo {
                        name: "alpha".to_string(),
                        declared_version: "0.1.0".to_string(),
                        path: "../alpha".to_string(),
                        section: "dependencies".to_string(),
                        optional: false,
                    },
                    DepInfo {
                        name: "alpha".to_string(),
                        declared_version: "0.1.0".to_string(),
                        path: "../alpha".to_string(),
                        section: "dev-dependencies".to_string(),
                        optional: true,
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_format_crate_list_text_includes_all_info() {
        let text = format_crate_list(&sample_crates(), &OutputFormat::Text).expect("format");
        assert!(text.contains("alpha (0.1.1)"));
        assert!(text.contains("beta (0.1.1)"));
        assert!(text.contains("(no internal deps)"));
        assert!(text.contains(r#"alpha = "0.1.0""#));
        assert!(text.contains("[optional]"));
        assert!(text.contains("(dev-dependencies)"));
    }

    #[test]
    fn test_format_crate_list_json_roundtrips() {
        let crates = sample_crates();
        let json = format_crate_list(&crates, &OutputFormat::Json).expect("format");
        let parsed: Vec<CrateInfo> = serde_json::from_str(&json).expect("parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "alpha");
        assert_eq!(parsed[1].internal_deps.len(), 2);
    }

    #[test]
    fn test_format_mismatches_shows_all_fields() {
        let mismatches = vec![VersionMismatch {
            crate_name: "beta".to_string(),
            dep_name: "alpha".to_string(),
            declared_version: "0.1.0".to_string(),
            expected_version: "0.1.1".to_string(),
            section: "dependencies".to_string(),
        }];
        let text = format_mismatches(&mismatches);
        assert!(text.contains("beta"));
        assert!(text.contains("alpha"));
        assert!(text.contains("0.1.0"));
        assert!(text.contains("0.1.1"));
    }
}
