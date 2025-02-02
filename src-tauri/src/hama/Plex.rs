use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, debug, error};

//
// Constants
//
const THEME_URL: &str = "https://tvthemes.plexapp.com/{}.mp3";

//
// Dummy preferences and common helper functions
//
fn prefs() -> Value {
    // Simulate global configuration; adjust as needed.
    // For example, themes is an array that should include "Plex" if you want to pull theme metadata.
    json!({
        "themes": ["Plex"]
    })
}

/// Dummy implementation for common_web_link – returns a formatted link.
fn common_web_link(link: &str) -> String {
    // In your application, this might return an HTML link.
    format!("WEB_LINK({})", link)
}

/// Performs an HTTP GET request (using HEAD method for speed) and returns the status code.
/// (You might wish to cache these calls in your production code.)
async fn get_status_code(url: &str) -> Result<u16> {
    let client = Client::new();
    // For efficiency, use HEAD if supported; here we use GET.
    let resp = client.get(url).timeout(Duration::from_secs(30)).send().await?;
    Ok(resp.status().as_u16())
}

/// Append an error message to the array in `error_log` under the given key.
/// If the key does not exist, create an array.
fn append_error(error_log: &mut Value, key: &str, message: &str) {
    if let Some(arr) = error_log.get_mut(key) {
        if let Some(vec) = arr.as_array_mut() {
            vec.push(json!(message));
        }
    } else {
        // Create a new array with the message.
        error_log[key] = json!([message]);
    }
}

/// Save a nested value into a JSON object (similar to your SaveDict).
/// Here the value is stored at the nested keys in the target JSON object.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) {
    // Create intermediate objects if needed.
    let mut current = target;
    for key in &keys[..keys.len()-1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len()-1]] = value;
}

/// Pretty-print a JSON object (similar to DictString).
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

//
// Main Function: get_metadata
//
// Parameters:
//   - metadata: a JSON object that is expected to have a field "themes" which is an array of URLs
//   - error_log: a mutable JSON object where error messages will be appended
//   - tvdbid: the TVDB id as a string (should be numeric)
//   - title: an optional title string; if None, tvdbid is used as title
//
// Returns a JSON object (Plex_dict) built from theme data.
pub async fn get_metadata(metadata: &Value, error_log: &mut Value, tvdbid: &str, title: Option<&str>) -> Result<Value> {
    info!("{}", "=== Plex.GetMetadata() ===".repeat(1));
    
    // Build the theme URL.
    let url = format!(THEME_URL, tvdbid);
    let mut plex_dict = json!({});

    // Log preferences and tvdbid.
    let themes_pref = prefs().get("themes").cloned().unwrap_or(json!([]));
    info!("Prefs['themes']: '{:?}', TVDBid: '{}'", themes_pref, tvdbid);
    info!("{}", "--- themes ---".repeat(1));
    
    // Check that the preferences include "Plex" and that tvdbid is all digits.
    if themes_pref.as_array().map(|arr| {
        arr.iter().any(|v| v.as_str() == Some("Plex"))
    }).unwrap_or(false) && tvdbid.chars().all(|c| c.is_digit(10)) {
        let title_str = title.unwrap_or(tvdbid);
        // Check if the URL is already in metadata.themes.
        // Here we assume that metadata["themes"] is an array of URLs.
        let result: String = if let Some(themes_arr) = metadata.get("themes").and_then(|v| v.as_array()) {
            if themes_arr.iter().any(|v| v.as_str() == Some(&url)) {
                "*".to_string()
            } else {
                // If not present, call get_status_code.
                match get_status_code(&url).await {
                    Ok(code) => code.to_string(),
                    Err(e) => {
                        error!("Error getting status code: {}", e);
                        "0".to_string()
                    }
                }
            }
        } else {
            // If no themes field exists, simply get the status code.
            match get_status_code(&url).await {
                Ok(code) => code.to_string(),
                Err(e) => {
                    error!("Error getting status code: {}", e);
                    "0".to_string()
                }
            }
        };
        info!("result code: '{{plex}}', url: '{}'", url);
        // If result is "200" or "*", then save the theme.
        if result == "200" || result == "*" {
            // Save a tuple (as JSON array) into plex_dict["themes"][url]:
            // For example: ("Plex/{TVDBid}.mp3", 2, null)
            save_dict(&mut plex_dict, &["themes", &url], json!([format!("Plex/{}.mp3", tvdbid), 2, Value::Null]));
            info!("[ ] theme: {}", dict_string(&plex_dict, 1));
        } else {
            // Otherwise, append an error message.
            let mailto = format!(
                "mailto:themes@plexapp.com?cc=&subject=Missing theme song - '{} - {}.mp3'",
                title_str, tvdbid
            );
            let link = common_web_link(&format!("Upload {}", mailto));
            let msg = format!("TVDBid: '{}' | Title: '{}' | {}", tvdbid, title_str, link);
            append_error(error_log, "Plex themes missing", &msg);
        }
    } else {
        info!(
            "Not pulling meta - 'Plex' in Prefs['themes']: '{:?}', TVDBid: '{}'",
            themes_pref, tvdbid
        );
    }
    
    info!("{}", "--- return ---".repeat(1));
    info!("Plex_dict: {}", dict_string(&plex_dict, 1));
    Ok(plex_dict)
}

/// Helper: Append an error message to a JSON array in error_log under the given key.
fn append_error(error_log: &mut Value, key: &str, message: &str) {
    if let Some(arr) = error_log.get_mut(key) {
        if let Some(vec) = arr.as_array_mut() {
            vec.push(json!(message));
        }
    } else {
        // Create a new array with the message.
        *error_log = json!({ key: [message] });
    }
}

/// Dummy implementation for common_web_link.
fn common_web_link(s: &str) -> String {
    format!("WEB_LINK({})", s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata_success() {
        // Setup a dummy metadata JSON that has a "themes" field.
        let metadata = json!({
            "themes": ["https://example.com/existing.mp3"]
        });
        // Dummy error_log.
        let mut error_log = json!({});
        // For testing, choose a numeric TVDBid.
        let tvdbid = "12345";
        // For this test, we simulate that get_status_code returns 200.
        // (In a real test, you might use a mock HTTP client.)
        let result = get_metadata(&metadata, &mut error_log, tvdbid, Some("Test Show")).await.unwrap();
        println!("Plex_dict: {}", dict_string(&result, 2));
    }
}
