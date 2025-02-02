use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use log::{info, debug, error};

//
// Dummy helper functions simulating your common module
//

/// Simulate common.GetMediaDir(media, movie)
fn get_media_dir(media: &Value, _movie: bool) -> String {
    // For demonstration, assume media is a JSON object with a "dir" field.
    media.get("dir").and_then(|v| v.as_str()).unwrap_or("").to_string()
}

/// Simulate common.LoadFile(filename, relativeDirectory, url)
/// In a real implementation, you would check for a local file and/or cache.
fn load_file(filename: &str, relative_directory: &str, url: &str) -> Result<Value> {
    // For now, we attempt to load from the URL.
    // (Replace with reqwest calls and caching if needed.)
    Err(anyhow!("load_file not implemented"))
}

/// Simulate common.GetXml: given a string containing XML data and an XPath expression,
/// return the matching text. (For simplicity, this dummy version just returns the input.)
fn get_xml(xml: &str, xpath: &str) -> String {
    // In production you would use an XML parser and run an XPath query.
    // Here, we simply return the xml string if it is nonempty.
    if !xml.trim().is_empty() {
        xml.to_string()
    } else {
        "".to_string()
    }
}

/// Simulate SaveDict: update a JSON object at a nested key path.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Set a nested value in a JSON object, creating intermediate objects if necessary.
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

/// Simulate Dict: retrieve a nested value from a JSON object.
fn dict(value: &Value, keys: &[&str]) -> Value {
    let mut current = value;
    for key in keys {
        if let Some(v) = current.get(*key) {
            current = v;
        } else {
            return json!({});
        }
    }
    current.clone()
}

/// Simulate DictString: produce a pretty-printed string of a JSON value.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Dummy poster_rank: returns a rank based on a rank_adjustment.
fn poster_rank(service: &str, typ: &str, rank_adjustment: i32) -> i32 {
    debug!("poster_rank({}, {}, {}) called", service, typ, rank_adjustment);
    rank_adjustment // For demonstration, simply return the adjustment.
}

//
// tvdb4 Module Implementation
//

// These URLs are defined as in your Python code.
const TVDB4_MAPPING_URL: &str =
    "https://raw.githubusercontent.com/ZeroQI/Absolute-Series-Scanner/master/tvdb4.mapping.xml";
const TVDB4_POSTERS_URL: &str =
    "https://raw.githubusercontent.com/ZeroQI/Absolute-Series-Scanner/master/tvdb4.posters.xml";

/// find_tvdb4_file: searches upward from the media directory for a file named `file_to_find`.
/// Returns the file content as a String. If not found, returns an empty string.
fn find_tvdb4_file(file_to_find: &str, media: &Value, movie: bool) -> Result<String> {
    let mut folder = get_media_dir(media, movie);
    while !folder.is_empty() {
        let path = Path::new(&folder).join(file_to_find);
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            return Ok(content);
        }
        // Go to parent directory.
        if let Some(parent) = Path::new(&folder).parent() {
            folder = parent.to_string_lossy().to_string();
        } else {
            break;
        }
    }
    info!("No '{}' file detected locally", file_to_find);
    Ok(String::new())
}

/// get_metadata: loads tvdb4 mapping and posters files and updates mapping_list accordingly.
/// Returns a JSON object (TVDB4_dict) containing the processed data.
pub fn get_metadata(
    media: &Value,
    movie: bool,
    source: &str,
    TVDBid: &str,
    mapping_list: &mut Value,
    _num: Option<usize>,
) -> Result<Value> {
    info!("{}", "=== tvdb4.GetMetadata() ===".repeat(1));
    let mut tvdb4_dict = json!({});

    // If movie or source is not "tvdb4", do not process.
    if movie || source != "tvdb4" {
        info!("not tvdb4 mode");
        return Ok(tvdb4_dict);
    }
    info!("tvdb4 mode");

    // Load tvdb4.mapping.xml.
    info!("{}", "--- tvdb4.mapping.xml ---".repeat(1));
    let tvdb4_mapping = find_tvdb4_file("tvdb4.mapping", media, movie)?;
    let mut mapping_entry = String::new();
    if !tvdb4_mapping.is_empty() {
        // If tvdb4_mapping is an XML string, use get_xml to extract the entry.
        mapping_entry = get_xml(&tvdb4_mapping, &format!("/tvdb4entries/anime[@tvdbid='{}']", TVDBid));
        if mapping_entry.is_empty() {
            error!("TVDBid '{}' is not found in mapping file", TVDBid);
        }
    } else {
        // Otherwise, attempt to load from remote.
        // (For brevity, this branch is not fully implemented.)
    }
    // Process each line of the mapping entry.
    if !mapping_entry.is_empty() {
        for line in mapping_entry.trim().lines().filter(|l| !l.trim().is_empty()) {
            let parts: Vec<&str> = line.trim().split("|").collect();
            if parts.len() < 4 {
                continue;
            }
            // parts[0]: season label, parts[1]: starting episode, parts[2]: ending episode, parts[3]: label.
            let season_label = parts[0].trim();
            let start_ep: i32 = parts[1].trim().parse().unwrap_or(0);
            let end_ep: i32 = parts[2].trim().parse().unwrap_or(0);
            // For each absolute episode in the range [start_ep, end_ep]:
            for abs_ep in start_ep..=end_ep {
                save_dict(
                    mapping_list,
                    &["absolute_map", &abs_ep.to_string()],
                    json!((season_label, abs_ep.to_string())),
                );
            }
            let unknown = parts[3].contains("(unknown length)") || parts[1].trim() == parts[2].trim();
            save_dict(mapping_list, &["absolute_map", "unknown_series_length"], json!(unknown));
            // Save max season as season_label parsed to int and then converted back to string.
            let max_season = season_label.parse::<i32>().unwrap_or(0).to_string();
            save_dict(mapping_list, &["absolute_map", "max_season"], json!(max_season));
            info!(
                "[ ] season: {}, starting episode: {}, ending episode: {}, label: {}",
                season_label,
                parts[1].trim(),
                parts[2].trim(),
                parts[3].trim()
            );
        }
    }

    // Process tvdb4.posters.xml.
    info!("{}", "--- tvdb4.posters.xml ---".repeat(1));
    let tvdb4_posters = find_tvdb4_file("tvdb.posters", media, movie)?;
    let mut posters_entry = String::new();
    if !tvdb4_posters.is_empty() {
        posters_entry = get_xml(&tvdb4_posters, &format!("/tvdb4entries/posters[@tvdbid='{}']", TVDBid));
        if posters_entry.is_empty() {
            error!("TVDBid '{}' is not found in posters file", TVDBid);
        }
    }
    // If both posters file and entry exist, process each line.
    if !tvdb4_posters.is_empty() && !posters_entry.is_empty() {
        let mut season_posters: HashMap<String, i32> = HashMap::new();
        for line in posters_entry.trim().lines().filter(|l| !l.trim().is_empty()) {
            let parts: Vec<&str> = line.trim().splitn(2, "|").collect();
            if parts.len() < 2 {
                continue;
            }
            let mut season = parts[0].trim().trim_start_matches('0').to_string();
            if season.is_empty() {
                season = "0".to_string();
            }
            // Update the count for this season.
            let count = season_posters.entry(season.clone()).or_insert(0);
            *count += 1;
            let rank = poster_rank("tvdb4", "posters", *count - 1);
            let url = parts[1].trim();
            let basename = Path::new(url)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let local_filename = format!("TheTVDB/seasons/{}-{}-{}", TVDBid, season, basename);
            save_dict(
                &mut tvdb4_dict,
                &["seasons", &season, "posters", url],
                json!((local_filename, rank, Value::Null)),
            );
            info!("[ ] season: {:>2}, rank: {:>3}, filename: {}", season, rank, url);
        }
    }

    info!("{}", "--- return ---".repeat(1));
    info!("absolute_map: {}", dict_string(&dict(mapping_list, &["absolute_map"]), 0));
    info!("TVDB4_dict: {}", dict_string(&tvdb4_dict, 4));
    Ok(tvdb4_dict)
}
