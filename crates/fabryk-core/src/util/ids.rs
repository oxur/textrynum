//! ID normalization utilities.
//!
//! Provides functions for normalizing string identifiers to consistent
//! kebab-case format. Used by graph builders, content loaders, and
//! anywhere stable IDs are needed.

use std::path::Path;

/// Normalize an identifier to lowercase kebab-case.
///
/// Performs the following transformations:
/// 1. Trims leading/trailing whitespace
/// 2. Converts to lowercase
/// 3. Replaces underscores with hyphens
/// 4. Collapses multiple whitespace into single hyphens
///
/// # Examples
///
/// ```
/// use fabryk_core::util::ids::normalize_id;
///
/// assert_eq!(normalize_id("Voice Leading"), "voice-leading");
/// assert_eq!(normalize_id("non_chord_tone"), "non-chord-tone");
/// assert_eq!(normalize_id("  Mixed   Case  "), "mixed-case");
/// assert_eq!(normalize_id("UPPERCASE"), "uppercase");
/// ```
pub fn normalize_id(id: &str) -> String {
    id.trim()
        .to_lowercase()
        .replace('_', " ") // Convert underscores to spaces first
        .split_whitespace() // Split on any whitespace, collapsing multiples
        .collect::<Vec<&str>>()
        .join("-")
}

/// Compute an ID from a file path's stem.
///
/// Extracts the file stem (filename without extension) and normalizes it.
/// Returns `None` if the path has no file stem.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use fabryk_core::util::ids::id_from_path;
///
/// assert_eq!(
///     id_from_path(Path::new("/data/concepts/Voice_Leading.md")),
///     Some("voice-leading".to_string())
/// );
/// assert_eq!(
///     id_from_path(Path::new("/data/Major Scale.md")),
///     Some("major-scale".to_string())
/// );
/// assert_eq!(id_from_path(Path::new("/")), None);
/// ```
pub fn id_from_path(path: &Path) -> Option<String> {
    path.file_stem().and_then(|s| s.to_str()).map(normalize_id)
}

/// Convert a kebab-case or snake_case identifier to Title Case.
///
/// Inverse of [`normalize_id`]. Splits on hyphens and underscores, capitalizes
/// the first letter of each word, and joins with spaces.
///
/// # Examples
///
/// ```
/// use fabryk_core::util::ids::humanize_id;
///
/// assert_eq!(humanize_id("voice-leading"), "Voice Leading");
/// assert_eq!(humanize_id("jazz_theory_book"), "Jazz Theory Book");
/// assert_eq!(humanize_id("single"), "Single");
/// assert_eq!(humanize_id(""), "");
/// ```
pub fn humanize_id(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars);
                    s
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // normalize_id tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_normalize_id_simple() {
        assert_eq!(normalize_id("dissonance"), "dissonance");
    }

    #[test]
    fn test_normalize_id_with_spaces() {
        assert_eq!(normalize_id("Voice Leading"), "voice-leading");
    }

    #[test]
    fn test_normalize_id_with_underscores() {
        assert_eq!(normalize_id("non_chord_tone"), "non-chord-tone");
    }

    #[test]
    fn test_normalize_id_mixed_case() {
        assert_eq!(normalize_id("PicardyThird"), "picardythird");
    }

    #[test]
    fn test_normalize_id_with_whitespace() {
        assert_eq!(normalize_id("  Mixed   Case  "), "mixed-case");
    }

    #[test]
    fn test_normalize_id_already_normalized() {
        assert_eq!(normalize_id("voice-leading"), "voice-leading");
    }

    #[test]
    fn test_normalize_id_uppercase() {
        assert_eq!(normalize_id("UPPERCASE"), "uppercase");
    }

    #[test]
    fn test_normalize_id_empty() {
        assert_eq!(normalize_id(""), "");
        assert_eq!(normalize_id("   "), "");
    }

    #[test]
    fn test_normalize_id_mixed_separators() {
        assert_eq!(normalize_id("foo_bar baz"), "foo-bar-baz");
    }

    // -------------------------------------------------------------------------
    // id_from_path tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_id_from_path_simple() {
        let path = Path::new("/data/concepts/dissonance.md");
        assert_eq!(id_from_path(path), Some("dissonance".to_string()));
    }

    #[test]
    fn test_id_from_path_with_underscores() {
        let path = Path::new("/data/Voice_Leading.md");
        assert_eq!(id_from_path(path), Some("voice-leading".to_string()));
    }

    #[test]
    fn test_id_from_path_nested() {
        let path = Path::new("/data/harmony/chord-progressions/ii-V-I.md");
        assert_eq!(id_from_path(path), Some("ii-v-i".to_string()));
    }

    #[test]
    fn test_id_from_path_no_extension() {
        let path = Path::new("/data/README");
        assert_eq!(id_from_path(path), Some("readme".to_string()));
    }

    #[test]
    fn test_id_from_path_no_stem() {
        let path = Path::new("/");
        assert_eq!(id_from_path(path), None);
    }

    #[test]
    fn test_id_from_path_hidden_file() {
        let path = Path::new("/data/.hidden");
        assert_eq!(id_from_path(path), Some(".hidden".to_string()));
    }

    // -------------------------------------------------------------------------
    // humanize_id tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_humanize_id_kebab_case() {
        assert_eq!(humanize_id("voice-leading"), "Voice Leading");
    }

    #[test]
    fn test_humanize_id_snake_case() {
        assert_eq!(humanize_id("jazz_theory_book"), "Jazz Theory Book");
    }

    #[test]
    fn test_humanize_id_single_word() {
        assert_eq!(humanize_id("single"), "Single");
    }

    #[test]
    fn test_humanize_id_empty() {
        assert_eq!(humanize_id(""), "");
    }

    #[test]
    fn test_humanize_id_double_separator() {
        assert_eq!(humanize_id("some--double-dash"), "Some Double Dash");
    }

    #[test]
    fn test_humanize_id_roundtrip_with_normalize() {
        let original = "voice-leading";
        let humanized = humanize_id(original);
        let normalized = normalize_id(&humanized);
        assert_eq!(normalized, original);
    }
}
