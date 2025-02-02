use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use reqwest::Client;
use serde_json::{json, Value};
use std::path::Path;
use std::collections::BTreeMap;
use log::{info, debug, error};
use tokio::time::{sleep, Duration};
use urlencoding::encode;

//
// Dummy common functions and configuration – replace with your own implementations
//

/// Simulate a global preferences object.
fn prefs() -> Value {
    // For demonstration, assume the themes list contains "TVTunes"
    json!({
        "themes": ["TVTunes"]
    })
}

/// Check if a file exists locally (dummy implementation).
fn file_exists(filename: &str) -> bool {
    Path::new(filename).exists()
}

/// Performs an HTTP HEAD request to get the status code for a URL.
async fn get_status_code(url: &str) -> Result<u16> {
    let client = Client::new();
    let resp = client.head(url).send().await?;
    Ok(resp.status().as_u16())
}

/// Set a nested value in a JSON object (creating intermediate objects as needed).
fn set_nested(target: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *target = new_value;
        return;
    }
    let mut current = target;
    for key in &keys[..keys.len()-1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len()-1]] = new_value;
}

/// Mimic SaveDict: update a target dictionary at a nested key.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Mimic Dict: get a nested value from a JSON object.
fn dict(value: &Value, keys: &[&str]) -> Value {
    let mut current = value;
    for key in keys {
        if let Some(next) = current.get(*key) {
            current = next;
        } else {
            return json!({});
        }
    }
    current.clone()
}

/// Mimic DictString: pretty-print a JSON value.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Dummy poster_rank: returns a fixed rank (here, always 1).
fn poster_rank(_service: &str, _typ: &str) -> i32 {
    1
}

/// Dummy update_dict: merge src into dst.
fn update_dict(dst: &mut Value, src: &Value) {
    if let (Some(dst_obj), Some(src_obj)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in src_obj.iter() {
            dst_obj.insert(k.clone(), v.clone());
        }
    }
}

/// Simulate a Data.Exists check from Python.
/// Here we check whether a file with the given filename exists.
fn data_exists(filename: &str) -> bool {
    // In your application you might check in a specific data folder.
    Path::new(filename).exists()
}

//
// Main Function: GetMetadata for TVTunes
//

/// Asynchronous function that mimics TVTunes.GetMetadata() from Python.
///
/// - `metadata`: a JSON object that may contain a key "themes" (an array of URLs)
/// - `title1` and `title2`: two candidate title strings
///
/// For each nonempty title, the function builds a URL by quoting the title and substituting
/// into THEME_URL. Then it checks if the URL is already present in `metadata["themes"]` or if a
/// local file exists (using `data_exists()`). If so, the result is set to "*" (indicating success);
/// otherwise, it performs an HTTP HEAD request to obtain a status code. If the result is 200 or "*",
/// a tuple (local filename, rank, null) is saved into a TVTunes dictionary.
pub async fn get_metadata(metadata: &Value, title1: &str, title2: &str) -> Result<Value> {
    info!("{}", "=== TVTunes.GetMetadata() ===".repeat(1));
    let mut tvtunes_dict = json!({});

    info!(
        "Prefs['themes']: '{:?}', title: '{}', title2: '{}'",
        prefs().get("themes"),
        title1,
        title2
    );
    info!("{}", "--- themes ---".repeat(1));

    // Check if "TVTunes" is in prefs["themes"] and at least one title is provided.
    let themes_pref = prefs().get("themes").and_then(|v| v.as_array());
    if let Some(themes) = themes_pref {
        if themes.iter().any(|v| v.as_str() == Some("TVTunes")) && (!title1.is_empty() || !title2.is_empty()) {
            // For each title (title1 and title2) that is nonempty:
            for t in [title1, title2].iter().filter(|&&t| !t.is_empty()) {
                // URL-quote the title.
                let quoted = encode(t);
                let url = format!("https://www.televisiontunes.com/uploads/audio/{}.mp3", quoted);
                // Check if the URL is already in metadata.themes OR if a file exists locally.
                let result = if metadata.get("themes")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().any(|v| v.as_str() == Some(&url)))
                    .unwrap_or(false)
                    || data_exists(url.split('/').last().unwrap_or("")) {
                    "*".to_string()
                } else {
                    // Otherwise, call get_status_code.
                    match get_status_code(&url).await {
                        Ok(code) => code.to_string(),
                        Err(e) => {
                            error!("Error getting status code for {}: {}", url, e);
                            "0".to_string()
                        }
                    }
                };
                info!("Return code: '{}', url: '{}'", result, url);
                if result == "200" || result == "*" {
                    let filename = url.split('/').last().unwrap_or("");
                    let local_filename = format!("TelevisionTunes/{}", filename);
                    save_dict(&mut tvtunes_dict, &["themes", url.as_str()], json!((local_filename, 1, Value::Null)));
                    info!("[ ] theme: {}", dict_string(&tvtunes_dict, 1));
                }
            }
        }
    } else {
        info!(
            "Not pulling meta - 'TVTunes' in Prefs['themes']: {:?}",
            prefs().get("themes")
        );
    }

    info!("{}", "--- return ---".repeat(1));
    info!("TVTunes_dict: {}", dict_string(&tvtunes_dict, 1));
    Ok(tvtunes_dict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata() {
        // Set up a dummy metadata JSON.
        let metadata = json!({
            "themes": ["https://www.televisiontunes.com/uploads/audio/existing.mp3"]
        });
        // Call get_metadata with two titles.
        let result = get_metadata(&metadata, "Test Show", "Alternate Title").await.unwrap();
        println!("Result: {}", dict_string(&result, 2));
    }
}
