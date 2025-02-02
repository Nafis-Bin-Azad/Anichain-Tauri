// src/utils.rs

use regex::Regex;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// Checks if the file at `path` has a known video extension.
pub fn is_video_ext(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
        let ext = ext.to_lowercase();
        let video_exts = vec![
            "3g2", "3gp", "asf", "asx", "avi", "mkv", "mp4", "mov", "mpeg", "mpg", "wmv", "webm",
        ];
        video_exts.contains(&ext.as_str())
    } else {
        false
    }
}

/// Generates a natural sort key by splitting the string into non-digit and digit chunks.
/// For example, "Episode 12" becomes ["episode ", "12", ""]. This can be used to sort strings
/// in a way that numeric parts are compared as numbers.
pub fn natural_sort_key(s: &str) -> Vec<String> {
    let re = Regex::new(r"(\d+)").unwrap();
    re.split(s)
        .map(|x| x.to_lowercase())
        .collect()
}

/// Cleanses a title string by normalizing Unicode, removing parenthesized/bracketed text,
/// and replacing certain punctuation with spaces.
pub fn cleanse_title(s: &str) -> String {
    // Normalize the string using NFC normalization.
    let normalized: String = s.nfc().collect();
    // Remove text inside parentheses.
    let re_paren = Regex::new(r"\([^)]*\)").unwrap();
    let without_paren = re_paren.replace_all(&normalized, " ");
    // Remove text inside square brackets.
    let re_bracket = Regex::new(r"\[[^\]]*\]").unwrap();
    let without_brackets = re_bracket.replace_all(&without_paren, " ");
    // Replace specific punctuation characters with spaces.
    let punctuation = &[':', '/', '\\', '*', '?', '<', '>', '|'][..];
    let replaced = without_brackets
        .replace(punctuation, " ");
    // Collapse multiple spaces and convert to lowercase.
    replaced
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .to_lowercase()
}

/// Replaces characters found in `filter_chars` with spaces.
pub fn filter_chars(s: &str) -> String {
    let filter_chars = "\\/:*?<>|;";
    let mut result = s.to_string();
    for ch in filter_chars.chars() {
        result = result.replace(ch, " ");
    }
    result
}

/// Logs a simple info message. For now, it just prints to stdout.
/// In a more sophisticated setup, you might integrate with the `log` crate.
pub fn log_info(message: &str) {
    println!("{}", message);
}
