// src/api_providers/omdb.rs

use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, debug, error};

//
// Constants
//
const OMDB_HTTP_API_URL: &str = "https://www.omdbapi.com/?apikey={api_key}&i=";

//
// Configuration retrieval: replace this with your own configuration logic.
//
fn get_pref(key: &str) -> Option<String> {
    match key {
        "OMDbApiKey" => Some("YOUR_OMDB_API_KEY".to_string()), // Replace with your actual OMDb API key
        _ => None,
    }
}

/// Returns a map of default HTTP headers.
fn common_headers() -> BTreeMap<&'static str, &'static str> {
    let mut headers = BTreeMap::new();
    headers.insert("User-Agent", "OMDbRustClient/1.0");
    headers
}

/// Helper: Traverse a JSON object by keys and return an Option reference.
fn get_nested<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current)
}

/// Helper: Set a nested JSON value, creating intermediate objects if needed.
fn set_nested(value: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &keys[..keys.len()-1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len()-1]] = new_value;
}

/// Alias for set_nested, mimicking a “SaveDict” helper.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) {
    set_nested(target, keys, value);
}

/// Helper: Retrieve a nested value from a JSON object.
/// Returns an empty JSON object if any key is missing.
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

/// Pretty-print a JSON value.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Asynchronous function that loads JSON from a given URL.
/// (In a full implementation you might check for a cached local copy.)
async fn load_file(filename: &str, relative_directory: &str, url: &str) -> Result<Option<Value>> {
    debug!(
        "Loading file: filename='{}', relative_directory='{}', url='{}'",
        filename, relative_directory, url
    );
    let client = Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let json: Value = response.json().await?;
        // Optionally: Save the JSON to a file using filename and relative_directory.
        Ok(Some(json))
    } else {
        error!("Failed to load file from {}: {}", url, response.status());
        Ok(None)
    }
}

/// Fetch metadata from the OMDb API.
/// This function performs a real HTTP GET call to OMDb using the provided IMDb id.
/// It then extracts key fields such as title, plot, released date, countries, director,
/// genres, writers, rating, content_rating, poster URL, and duration.
///
/// # Parameters
///
/// - `movie`: a boolean flag (true if the media is a movie)
/// - `imdb_id`: the IMDb id (expected to start with "tt")
///
/// # Returns
///
/// A JSON object (Value) containing the metadata.
pub async fn get_metadata(movie: bool, imdb_id: &str) -> Result<Value> {
    info!("OMDb: Fetching metadata for imdb_id: {} (movie: {})", imdb_id, movie);

    let api_key = match get_pref("OMDbApiKey") {
        Some(k) if !k.trim().is_empty() && k != "None" && k != "N/A" => k,
        _ => {
            info!("No API key found - Prefs['OMDbApiKey'] is missing or invalid");
            return Ok(json!({}));
        }
    };

    let url = format!("{}{}", OMDB_HTTP_API_URL.replace("{api_key}", &api_key), imdb_id);
    let client = Client::new();
    let req_headers = common_headers()
        .into_iter()
        .map(|(k, v)| {
            let header_name = reqwest::header::HeaderName::from_static(k);
            let header_value = reqwest::header::HeaderValue::from_str(v).unwrap();
            (header_name, header_value)
        })
        .collect::<reqwest::header::HeaderMap>();

    let resp = client.get(&url)
        .headers(req_headers)
        .send()
        .await?;

    if !resp.status().is_success() {
        error!("Failed to load file from {}: {}", url, resp.status());
        return Ok(json!({}));
    }

    let json_val: Value = resp.json().await?;
    let mut omdb_dict = json!({});

    info!("IMDbid: '{}'", imdb_id);

    // Process only if imdb_id starts with "tt"
    if imdb_id.starts_with("tt") {
        // In case imdb_id contains multiple comma-separated IDs, iterate over them.
        for imdbid_single in imdb_id.split(',') {
            info!("{}", format!("--- {}.series ---", imdbid_single)
                .chars()
                .cycle()
                .take(157)
                .collect::<String>());
            let url_single = format!("{}{}", OMDB_HTTP_API_URL.replace("{api_key}", &api_key), imdbid_single);
            let json_opt = load_file(imdbid_single, "OMDb/json", &url_single).await?;
            if let Some(json_val) = json_opt {
                // Title
                if let Some(title) = json_val.get("Title") {
                    save_dict(&mut omdb_dict, &["title"], title.clone());
                    info!("[ ] title: {}", title);
                }
                // Summary (Plot) – remove HTML tags.
                if let Some(plot) = json_val.get("Plot").and_then(|v| v.as_str()) {
                    let re = Regex::new(r"<.*?>").unwrap();
                    let clean_plot = re.replace_all(plot, "").to_string();
                    save_dict(&mut omdb_dict, &["summary"], json!(clean_plot));
                    info!("[ ] summary: {}", plot);
                }
                // Released date
                if let Some(released) = json_val.get("Released") {
                    save_dict(&mut omdb_dict, &["originally_available_at"], released.clone());
                    info!("[ ] originally_available_at: {}", released);
                }
                // Countries
                if let Some(country) = json_val.get("Country") {
                    save_dict(&mut omdb_dict, &["countries"], country.clone());
                    info!("[ ] countries: {}", country);
                }
                // Directors
                if let Some(director) = json_val.get("Director") {
                    save_dict(&mut omdb_dict, &["directors"], director.clone());
                    info!("[ ] directors: {}", director);
                }
                // Genres – split by comma and trim whitespace.
                if let Some(genre) = json_val.get("Genre").and_then(|v| v.as_str()) {
                    let mut genres: Vec<String> = genre.split(',')
                        .map(|s| s.trim().to_string())
                        .collect();
                    genres.sort();
                    save_dict(&mut omdb_dict, &["genres"], json!(genres));
                    info!("[ ] genres: {}", genre);
                }
                // Writers
                if let Some(writer) = json_val.get("Writer") {
                    save_dict(&mut omdb_dict, &["writers"], writer.clone());
                    info!("[ ] writers: {}", writer);
                }
                // Rating: use imdbRating field.
                if let Some(imdb_rating) = json_val.get("imdbRating") {
                    save_dict(&mut omdb_dict, &["rating"], imdb_rating.clone());
                }
                // If rating is empty and Metascore is numeric, use Metascore / 10.
                if let Some(rating_val) = omdb_dict.get("rating").and_then(|v| v.as_str()) {
                    if rating_val.is_empty() {
                        if let Some(metascore) = json_val.get("Metascore").and_then(|v| v.as_str()) {
                            if metascore.chars().all(|c| c.is_digit(10)) {
                                if let Ok(ms) = metascore.parse::<f64>() {
                                    save_dict(&mut omdb_dict, &["rating"], json!(ms / 10.0));
                                }
                            }
                        }
                    }
                }
                info!("[ ] rating: {:?}", omdb_dict.get("rating"));
                // Content Rating: from Rated field.
                if let Some(rated) = json_val.get("Rated") {
                    save_dict(&mut omdb_dict, &["content_rating"], rated.clone());
                    info!("[ ] content_rating: {}", rated);
                }
                // Poster
                if let Some(poster) = json_val.get("Poster").and_then(|v| v.as_str()) {
                    info!("[ ] poster: {}", poster);
                    let poster_path = format!("OMDb/poster/{}.jpg", imdbid_single);
                    save_dict(&mut omdb_dict, &["posters", poster], json!((poster_path, poster_rank("OMDb", "posters"), Value::Null)));
                }
                // Duration: from Runtime field (e.g., "140 min").
                if let Some(runtime) = json_val.get("Runtime").and_then(|v| v.as_str()) {
                    let runtime_clean = runtime.replace(" min", "");
                    if let Ok(rt) = runtime_clean.parse::<i32>() {
                        let duration = rt * 60 * 1000;
                        save_dict(&mut omdb_dict, &["duration"], json!(duration));
                        info!("[ ] duration: {}", duration);
                    }
                }
            }
        }
    }

    info!("{}", "--- return ---".repeat(1));
    info!("OMDb_dict: {}", dict_string(&omdb_dict, 4));
    Ok(omdb_dict)
}
