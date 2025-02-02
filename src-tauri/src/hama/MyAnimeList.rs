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
const MYANIMELIST_URL_SEARCH: &str = "https://api.myanimelist.net/v2/anime?q={title}&limit=10";
const MYANIMELIST_URL_DETAILS: &str = "https://api.myanimelist.net/v2/anime/{id}?fields=id,title,pictures,start_date,synopsis,mean,genres,rating,studios,media_type";
// For caching purposes, we define MYANIMELIST_CACHE_TIME as seconds (here 1 week)
const MYANIMELIST_CACHE_TIME: u64 = 60 * 60 * 24 * 7;

// Rating values mapping
lazy_static! {
    static ref RATING_VALUES: BTreeMap<&'static str, &'static str> = {
        let mut m = BTreeMap::new();
        m.insert("g", "G - All Ages");
        m.insert("pg", "PG - Children");
        m.insert("pg_13", "PG-13 - Teens 13 and Older");
        m.insert("r", "R - 17+ (violence & profanity)");
        m.insert("r+", "R+ - Profanity & Mild Nudity");
        m.insert("rx", "Rx - Hentai");
        m
    };
}

/// Dummy global configuration retrieval (simulate Prefs from Python).
fn get_pref(key: &str) -> Option<String> {
    // In your application, replace this with a proper configuration retrieval.
    // For example, here we hardcode a sample MAL API Client ID.
    match key {
        "MalApiClientID" => Some("YOUR_MAL_API_CLIENT_ID".to_string()),
        _ => None,
    }
}

/// COMMON_HEADERS constant (simulate common headers).
fn common_headers() -> BTreeMap<&'static str, &'static str> {
    let mut headers = BTreeMap::new();
    headers.insert("User-Agent", "MyAnimeListRustClient/1.0");
    headers
}

/// A dummy natural sort key. (For simplicity, we use lexicographical order.)
fn natural_sort_key(s: &str) -> String {
    s.to_string()
}

/// Dummy poster_rank function.
fn poster_rank(service: &str, typ: &str) -> i32 {
    debug!("poster_rank({}, {}) called", service, typ);
    0 // always return 0 for demonstration
}

/// Merge two JSON dictionaries by updating (simulating UpdateDict).
/// Here we merge `src` into `dst`.
fn update_dict(dst: &mut Value, src: &Value) {
    if let (Some(dst_obj), Some(src_obj)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in src_obj.iter() {
            dst_obj.insert(k.clone(), v.clone());
        }
    }
}

/// Set a nested value in a JSON object. Creates intermediate objects as needed.
fn set_nested(value: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &keys[..keys.len() - 1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len() - 1]] = new_value;
}

/// Dummy SaveDict, a thin wrapper around set_nested.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) {
    set_nested(target, keys, value);
}

/// Dummy Dict: get a nested value from a JSON object given keys.
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

/// Dummy DictString: return a pretty-printed string representation.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Asynchronous function to get metadata from MyAnimeList.
/// Given:
/// - `myanimelist_ids`: a JSON object with a key "seasons" mapping season keys to arrays of MAL IDs.
/// - `media_type`: a string specifying the media type (for comparison)
/// - `dict_anidb`: a JSON object representing AniDB metadata (to compare original_title, release date, etc.)
///
/// This function returns a tuple of (return_result, MainMALid) where return_result is a JSON object
/// built from the best-matching MAL entry (selected by a computed score).
pub async fn get_metadata(
    myanimelist_ids: &Value,
    media_type: &str,
    dict_anidb: &Value,
) -> Result<(Value, String)> {
    info!("{}", "=== MyAnimeList.GetMetadata() ===".repeat(1));
    let mut return_result = json!({});
    let mut main_mal_id = String::new();

    // Check if the MAL API client id is set
    let api_client_id = match get_pref("MalApiClientID") {
        Some(s) if !s.is_empty() && s != "None" && s != "N/A" => s,
        _ => {
            info!("No API key found - MalApiClientID is not set");
            // If myanimelist_ids has seasons, return the first MAL id from the first season.
            if let Some(seasons) = myanimelist_ids.get("seasons").and_then(|v| v.as_object()) {
                // Sort season keys naturally.
                let mut season_keys: Vec<&String> = seasons.keys().collect();
                season_keys.sort_by_key(|s| natural_sort_key(s));
                if let Some(first_season) = season_keys.first() {
                    if let Some(array) = seasons.get(*first_season).and_then(|v| v.as_array()) {
                        if let Some(first_id) = array.first().and_then(|v| v.as_str()) {
                            main_mal_id = first_id.to_string();
                            info!("Selected MainMALid: '{}'", main_mal_id);
                        }
                    }
                }
            }
            return Ok((json!({}), main_mal_id));
        }
    };

    // Set up HTTP client and headers.
    let client = Client::new();
    let mut headers = reqwest::header::HeaderMap::new();
    for (k, v) in common_headers() {
        headers.insert(
            reqwest::header::HeaderName::from_static(k),
            reqwest::header::HeaderValue::from_str(v).unwrap(),
        );
    }
    headers.insert("X-MAL-CLIENT-ID", reqwest::header::HeaderValue::from_str(&api_client_id)?);

    // For each season in myanimelist_ids["seasons"], iterate and select best match.
    if let Some(seasons) = myanimelist_ids.get("seasons").and_then(|v| v.as_object()) {
        // Sort season keys naturally.
        let mut season_keys: Vec<&String> = seasons.keys().collect();
        season_keys.sort_by_key(|s| natural_sort_key(s));
        for season in season_keys {
            let season_mal_id_list = seasons.get(season)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            info!("Season: {}, MAL id list: {}", season, dict_string(&json!(season_mal_id_list), 4));
            let mut best_match = json!({});
            let mut best_score: i32 = -1;
            // For each MAL id in sorted order.
            let mut sorted_ids = season_mal_id_list.clone();
            sorted_ids.sort_by_key(|v| {
                if let Some(s) = v.as_str() {
                    natural_sort_key(s)
                } else {
                    "".to_string()
                }
            });
            for mal_id_value in sorted_ids {
                let mal_id = mal_id_value.as_str().unwrap_or("");
                let detail_url = MYANIMELIST_URL_DETAILS.replace("{id}", mal_id);
                info!("{}", format!("=== MAL ID: {} ===", mal_id).repeat(1));
                info!("URL: {}", detail_url);
                // Perform the HTTP request (with a 2-second delay between calls).
                let json_response: Value = match client
                    .get(&detail_url)
                    .headers(headers.clone())
                    .timeout(Duration::from_secs(60))
                    .send()
                    .await
                {
                    Ok(resp) => resp.json().await.unwrap_or(json!({})),
                    Err(e) => {
                        error!("No detail information available: {}", e);
                        continue;
                    }
                };
                let mut parsed_response = json!({});
                let mut current_score = 0;
                // Parse JSON fields.
                if let Some(parsed_id) = json_response.get("id") {
                    save_dict(&mut parsed_response, &["id"], parsed_id.clone());
                    debug!("ID: {:?}", parsed_id);
                }
                if let Some(parsed_title) = json_response.get("title") {
                    save_dict(&mut parsed_response, &["title"], parsed_title.clone());
                    debug!("Title: {:?}", parsed_title);
                    // Increase score if dict_anidb["original_title"] equals parsed_title,
                    // or if dict_anidb["original_title"] is a substring.
                    if let Some(orig_title) = dict_anidb.get("original_title").and_then(|v| v.as_str()) {
                        let pt = parsed_title.as_str().unwrap_or("");
                        if orig_title == pt {
                            current_score += 2;
                        } else if pt.contains(orig_title) {
                            current_score += 1;
                        }
                    }
                }
                if let Some(parsed_summary) = json_response.get("synopsis") {
                    // Remove HTML tags.
                    let re = Regex::new(r"<.*?>").unwrap();
                    let clean_summary = re.replace_all(parsed_summary.as_str().unwrap_or(""), "");
                    save_dict(&mut parsed_response, &["summary"], json!(clean_summary.to_string()));
                    debug!("Summary: {:?}", parsed_summary);
                }
                if let Some(parsed_rating) = json_response.get("mean") {
                    if let Some(rating_f) = parsed_rating.as_f64() {
                        save_dict(&mut parsed_response, &["rating"], json!(rating_f));
                        debug!("Rating: {:?}", parsed_rating);
                    }
                }
                if let Some(parsed_content_rating) = json_response.get("rating") {
                    if let Some(rating_str) = parsed_content_rating.as_str() {
                        let content_rating_value = RATING_VALUES.get(rating_str).unwrap_or(&"");
                        save_dict(&mut parsed_response, &["content_rating"], json!(content_rating_value.to_string()));
                        debug!("Content Rating: {:?}", content_rating_value);
                    }
                }
                if let Some(parsed_start_date) = json_response.get("start_date") {
                    save_dict(&mut parsed_response, &["originally_available_at"], parsed_start_date.clone());
                    debug!("Release date: {:?}", parsed_start_date);
                    if let Some(orig_date) = dict_anidb.get("originally_available_at").and_then(|v| v.as_str()) {
                        if parsed_start_date.as_str().unwrap_or("").contains(orig_date) {
                            current_score += 1;
                        }
                    }
                }
                if let Some(parsed_pictures) = json_response.get("pictures").and_then(|v| v.as_array()) {
                    for picture_entry in parsed_pictures {
                        if let Some(poster_file_url) = picture_entry.get("medium").and_then(|v| v.as_str()) {
                            let parts: Vec<&str> = poster_file_url.split('/').collect();
                            let poster_file_name = parts.last().unwrap_or(&"unknown");
                            let poster_entry_value = json!((
                                format!("MyAnimeList/poster/{}.jpg", poster_file_name),
                                poster_rank("MyAnimeList", "posters"),
                                Value::Null
                            ));
                            save_dict(&mut parsed_response, &["posters", poster_file_url], poster_entry_value);
                            debug!("Cover: {}", poster_file_name);
                        }
                    }
                }
                if let Some(parsed_studios) = json_response.get("studios").and_then(|v| v.as_array()) {
                    let studio_names: Vec<String> = parsed_studios.iter()
                        .filter_map(|x| x.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
                        .collect();
                    let joined = studio_names.join(", ");
                    save_dict(&mut parsed_response, &["studio"], json!(joined));
                    debug!("Studios: {}", joined);
                }
                if let Some(parsed_genres) = json_response.get("genres").and_then(|v| v.as_array()) {
                    for genre in parsed_genres {
                        if let Some(genre_name) = genre.get("name").and_then(|v| v.as_str()) {
                            save_dict(&mut parsed_response, &["genres"], json!([genre_name.to_string()]));
                            debug!("Genres: {}", genre_name);
                        }
                    }
                }
                if let Some(parsed_media_type) = json_response.get("media_type").and_then(|v| v.as_str()) {
                    if parsed_media_type == media_type {
                        current_score += 2;
                    } else if parsed_media_type == "ova" {
                        current_score += 1;
                    }
                }
                debug!("MAL id: {}, Compare score: {}", mal_id, current_score);
                if current_score > best_score {
                    best_score = current_score;
                    best_match = parsed_response.clone();
                }
                // If there are more than 2 ids in this season, sleep 1 second to avoid overloading the MAL API.
                if season_mal_id_list.len() > 2 {
                    sleep(Duration::from_secs(1)).await;
                }
            }
            if best_match.get("id").is_some() {
                // Save best match data at the season level.
                save_dict(&mut return_result, &["seasons", season, "summary"], best_match.get("summary").cloned().unwrap_or(json!("")));
                save_dict(&mut return_result, &["seasons", season, "title"], best_match.get("title").cloned().unwrap_or(json!("")));
                save_dict(&mut return_result, &["seasons", season, "posters"], best_match.get("posters").cloned().unwrap_or(json!({})));
                // Save first found best match data at series level if not already set.
                if return_result.get("id").is_none() {
                    update_dict(&mut return_result, &best_match);
                    if let Some(id_val) = best_match.get("id").and_then(|v| v.as_str()) {
                        main_mal_id = id_val.to_string();
                        debug!("Selected MainMALid: '{}'", main_mal_id);
                    }
                }
            }
        }
    }

    info!("MyAnimeList_dict: {}", dict_string(&return_result, 4));
    info!("{}", "--- return ---".repeat(1));
    Ok((return_result, main_mal_id))
}

/// Update a JSON object in-place with another JSON object.
/// (This mimics your Python UpdateDict.)
fn update_dict(dst: &mut Value, src: &Value) {
    if let (Some(dst_obj), Some(src_obj)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in src_obj {
            dst_obj.insert(k.clone(), v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata_no_api_key() {
        // When no API key is set, get_metadata should return an empty dict and a MainMALid
        // from the myanimelist_ids if available.
        let myanimelist_ids = json!({
            "seasons": {
                "1": ["12345", "67890"]
            }
        });
        let dict_anidb = json!({
            "original_title": "Test Anime",
            "originally_available_at": "2020-01-01"
        });
        // Since our get_pref returns a dummy client id only if key equals "MalApiClientID" and here we can simulate absence
        // (For testing, you may modify get_pref to return None.)
        let (result, main_id) = get_metadata(&myanimelist_ids, "TV", &dict_anidb).await.unwrap();
        println!("Result: {}", dict_string(&result, 2));
        println!("MainMALid: {}", main_id);
    }
}
