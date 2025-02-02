use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use log::{info, debug, error};

//
// API Constants
//
const FTV_API_KEY: &str = "cfa9dc054d221b8d107f8411cd20b13f";
const FTV_API_MOVIES_URL: &str =
    "https://webservice.fanart.tv/v3/movies/{id}?api_key=cfa9dc054d221b8d107f8411cd20b13f";
const FTV_API_TV_URL: &str =
    "https://webservice.fanart.tv/v3/tv/{id}?api_key=cfa9dc054d221b8d107f8411cd20b13f";

//
// Helper functions that mimic your Python dynamic dictionary utilities
//

/// Traverse a nested JSON value given a list of keys.
fn get_nested<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current)
}

/// Set a nested value in a JSON object. Intermediate objects are created as needed.
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

/// Save a value into a nested dictionary (alias for set_nested).
fn save_dict(target: &mut Value, keys: &[&str], value: Value) {
    set_nested(target, keys, value);
}

/// Get a JSON array from a Value by key.
fn get_json_array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key)?.as_array()
}

/// Get a JSON string from a Value by key.
fn get_json_string(value: &Value, key: &str) -> Option<&str> {
    value.get(key)?.as_str()
}

/// Dummy implementation for poster ranking (replace with your own logic).
fn poster_rank(service: &str, typ: &str) -> i32 {
    // For demonstration, always return 0.
    debug!("poster_rank({}, {}) called", service, typ);
    0
}

/// Dummy helper to simulate a web link (for error reporting).
fn common_web_link(anidb_id: &str) -> Option<String> {
    Some(format!(
        "https://anidb.net/perl-bin/animedb.pl?show=anime&aid={}",
        anidb_id
    ))
}

//
// Main Function: GetMetadata
//
// This function mimics your Python GetMetadata for FanartTV. It takes as input:
/// - `movie`: a bool indicating whether we are processing a movie.
/// - `TVDBid`: the TVDB id (if available).
/// - `tmdbid`: the TMDB id (if available).
/// - `imdbid`: the IMDB id (if available).
/// - `season`: an integer season number (unused in this example).
///
/// It returns a JSON object (serde_json::Value) containing metadata (posters, art, banners, etc.)
///
pub async fn get_metadata(
    movie: bool,
    TVDBid: &str,
    tmdbid: &str,
    imdbid: &str,
    season: i32,
) -> Result<Value> {
    info!("{}", "=== FanartTv.GetMetadata() ===".repeat(1));
    let mut fanarttv_dict = json!({});

    info!(
        "movie: '{}', TVDBid: '{}', tmdbid: '{}', imdbid: '{}', season: '{}'",
        movie, TVDBid, tmdbid, imdbid, season
    );

    // If imdbid (or tmdbid) contains a comma, split and recursively call get_metadata.
    if !imdbid.is_empty() && imdbid.contains(',') {
        for imdbid_unique in imdbid.split(',') {
            // Use tmdbid if available, else the current imdbid_unique.
            let id_val = if !tmdbid.is_empty() {
                tmdbid
            } else {
                imdbid_unique
            };
            // We ignore the returned value.
            let _ = get_metadata(movie, "", "", id_val, season).await?;
        }
        return Ok(fanarttv_dict);
    }
    if !tmdbid.is_empty() && tmdbid.contains(',') {
        for tmdbid_unique in tmdbid.split(',') {
            let _ = get_metadata(movie, "", tmdbid_unique, "", season).await?;
        }
        return Ok(fanarttv_dict);
    }

    // Determine which id to use and build the relative directory and URL.
    let (id, relative_directory, url) = if !movie && TVDBid.chars().all(|c| c.is_digit(10)) {
        (
            TVDBid.to_string(),
            format!("FanartTV/tv/{}", TVDBid),
            FTV_API_TV_URL.replace("{id}", TVDBid),
        )
    } else if movie && (!imdbid.is_empty() || !tmdbid.is_empty()) {
        let id_val = if !imdbid.is_empty() { imdbid } else { tmdbid };
        (
            id_val.to_string(),
            format!("FanartTV/movie/{}", id_val),
            FTV_API_MOVIES_URL.replace("{id}", id_val),
        )
    } else {
        return Ok(fanarttv_dict);
    };

    // Fetch JSON from FanartTV.
    info!("{}", format!("--- {}.images ---", id).chars().take(157).collect::<String>());
    let json_opt = load_file(&(format!("{}.json", id)), &relative_directory, &url).await?;
    if let Some(json_val) = json_opt {
        // Process movies
        if movie && (!imdbid.is_empty() || !tmdbid.is_empty()) {
            if let Some(movieposter_arr) = get_json_array(&json_val, "movieposter") {
                for item in movieposter_arr {
                    if let Some(poster_url) = get_json_string(item, "url") {
                        info!("[ ] poster: {}", poster_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let poster_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "movieposter", filename_item);
                        let rank = poster_rank("FanartTV", "posters");
                        save_dict(&mut fanarttv_dict, &["posters", poster_url], json!((poster_path, rank, null)));
                    }
                }
            }
            if let Some(moviebg_arr) = get_json_array(&json_val, "moviebackground") {
                for item in moviebg_arr {
                    if let Some(bg_url) = get_json_string(item, "url") {
                        info!("[ ] art: {}", bg_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let art_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "moviebackground", filename_item);
                        let rank = poster_rank("FanartTV", "art");
                        save_dict(&mut fanarttv_dict, &["art", bg_url], json!((art_path, rank, null)));
                    }
                }
            }
        }
        // Process series (TV)
        if !movie && TVDBid.chars().all(|c| c.is_digit(10)) {
            if let Some(tvposter_arr) = get_json_array(&json_val, "tvposter") {
                for item in tvposter_arr {
                    if let Some(poster_url) = get_json_string(item, "url") {
                        info!("[ ] poster: {}", poster_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let poster_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "tvposter", filename_item);
                        let rank = poster_rank("FanartTV", "posters");
                        save_dict(&mut fanarttv_dict, &["posters", poster_url], json!((poster_path, rank, null)));
                    }
                }
            }
            if let Some(showbg_arr) = get_json_array(&json_val, "showbackground") {
                for item in showbg_arr {
                    if let Some(bg_url) = get_json_string(item, "url") {
                        info!("[ ] art: {}", bg_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let art_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "showbackground", filename_item);
                        let rank = poster_rank("FanartTV", "art");
                        save_dict(&mut fanarttv_dict, &["art", bg_url], json!((art_path, rank, null)));
                    }
                }
            }
            if let Some(tvbanner_arr) = get_json_array(&json_val, "tvbanner") {
                for item in tvbanner_arr {
                    if let Some(banner_url) = get_json_string(item, "url") {
                        info!("[ ] banner: {}", banner_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let banner_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "tvbanner", filename_item);
                        let rank = poster_rank("FanartTV", "banners");
                        save_dict(&mut fanarttv_dict, &["banners", banner_url], json!((banner_path, rank, null)));
                    }
                }
            }
            if let Some(seasonposter_arr) = get_json_array(&json_val, "seasonposter") {
                for item in seasonposter_arr {
                    if let Some(sp_url) = get_json_string(item, "url") {
                        info!("[ ] season poster: {}", sp_url);
                        let filename_item = get_json_string(item, "id").unwrap_or("unknown");
                        let season_val = get_json_string(item, "season").unwrap_or("0");
                        let sp_path = format!("{}/{}/{}/{}.jpg", relative_directory, id, "seasonposter", filename_item);
                        let rank = poster_rank("FanartTV", "posters");
                        // Save nested under "seasons" -> season value -> "posters" -> url.
                        save_dict(&mut fanarttv_dict, &["seasons", season_val, "posters", sp_url], json!((sp_path, rank, null)));
                    }
                }
            }
        }
    }

    // Remove key "all" from fanarttv_dict["seasons"] if it exists.
    if let Some(seasons) = fanarttv_dict.get_mut("seasons") {
        if let Some(obj) = seasons.as_object_mut() {
            obj.remove("all");
        }
    }
    info!("{}", "-".repeat(157));
    info!("FanartTV_dict: {:?}", fanarttv_dict);
    Ok(fanarttv_dict)
}

/// Asynchronous loader: fetches JSON from the given URL.
/// Optionally, you might cache this locally.
async fn load_file(filename: &str, relative_directory: &str, url: &str) -> Result<Option<Value>> {
    // (In a full implementation you might check for a local file first.)
    let client = Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let json: Value = response.json().await?;
        // (Optionally write to local file here.)
        Ok(Some(json))
    } else {
        error!("Failed to load file from {}: {}", url, response.status());
        Ok(None)
    }
}
