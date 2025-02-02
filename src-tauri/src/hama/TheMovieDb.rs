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
// Constants (adapted from your Python code)
//
const TMDB_API_KEY: &str = "7f4a0bd0bd3315bb832e17feda70b5cd";

lazy_static! {
    static ref TMDB_MOVIE_SEARCH: String = format!(
        "https://api.tmdb.org/3/search/movie?api_key={}&query={{query}}&year=&language=en&include_adult=true",
        TMDB_API_KEY
    );
    static ref TMDB_MOVIE_SEARCH_BY_TMDBID: String = format!(
        "https://api.tmdb.org/3/movie/{{id}}?api_key={}&append_to_response=releases,credits,trailers,external_ids&language=en",
        TMDB_API_KEY
    );
    static ref TMDB_SERIE_SEARCH_BY_TVDBID: String = format!(
        "https://api.themoviedb.org/3/find/{{id}}?api_key={}&external_source=tvdb_id&append_to_response=releases,credits,trailers,external_ids&language=en",
        TMDB_API_KEY
    );
    static ref TMDB_CONFIG_URL: String = format!(
        "https://api.tmdb.org/3/configuration?api_key={}",
        TMDB_API_KEY
    );
    static ref TMDB_MOVIE_IMAGES_URL: String = format!(
        "https://api.tmdb.org/3/{{mode}}/{{id}}/images?api_key={}",
        TMDB_API_KEY
    );
    // (Assume CACHE_1MONTH is defined elsewhere; for now, we do not implement caching.)
}

/// Helper function to mimic your Python Dict.
/// It traverses nested JSON values by keys and returns a cloned Value or an empty object.
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

/// Pretty-print a JSON value (mimicking DictString).
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Mimics SaveDict by inserting a value into a JSON object at a nested key path.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Set a nested value in a JSON object, creating intermediate objects as needed.
fn set_nested(target: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *target = new_value;
        return;
    }
    let mut current = target;
    for key in &keys[..keys.len() - 1] {
        if current.get(*key).is_none() {
            current[*key] = json!({});
        }
        current = current.get_mut(*key).unwrap();
    }
    current[keys[keys.len() - 1]] = new_value;
}

/// Dummy poster_rank function.
fn poster_rank(service: &str, typ: &str) -> i32 {
    debug!("poster_rank({}, {}) called", service, typ);
    0
}

/// Dummy function to load JSON from a URL (and optionally cache it).
/// In production you might check for a local file first.
async fn load_file(filename: &str, relative_directory: &str, url: &str) -> Result<Option<Value>> {
    debug!("Loading file: filename='{}', relative_directory='{}', url='{}'", filename, relative_directory, url);
    let client = Client::new();
    let response = client.get(url).send().await?;
    if response.status().is_success() {
        let json: Value = response.json().await?;
        Ok(Some(json))
    } else {
        error!("Failed to load file from {}: {}", url, response.status());
        Ok(None)
    }
}

/// Asynchronous function to get TMDB metadata.
///
/// Parameters:
///   - `media`: a JSON value representing media (unused here but provided for consistency)
///   - `movie`: boolean flag: true if movie, false if series
///   - `TVDBid`: TVDB id as a string
///   - `TMDbid`: TMDb id as a string (if available)
///   - `IMDbid`: IMDb id as a string (if available)
///
/// Returns a tuple:
///   (TheMovieDb_dict, TSDbid, TMDbid, IMDbid)
pub async fn get_metadata(
    _media: &Value,
    movie: bool,
    TVDBid: &str,
    mut TMDbid: &str,
    mut IMDbid: &str,
) -> Result<(Value, String, String, String)> {
    info!("{}", "=== TheMovieDb.GetMetadata() ===".repeat(1));
    let mut themoviedb_dict = json!({});
    let mut TSDbid = String::new();
    
    info!(
        "TVDBid: '{}', TMDbid: '{}', IMDbid: '{}'",
        TVDBid, TMDbid, IMDbid
    );

    // Decide which endpoint to use based on provided IDs.
    let (url, filename) = if !TMDbid.is_empty() {
        (
            TMDB_MOVIE_SEARCH_BY_TMDBID.replace("{id}", TMDbid),
            format!("TMDB-{}.json", TMDbid),
        )
    } else if !IMDbid.is_empty() {
        (
            TMDB_MOVIE_SEARCH_BY_TMDBID.replace("{id}", IMDbid),
            format!("IMDb-{}.json", IMDbid),
        )
    } else if TVDBid.chars().all(|c| c.is_digit(10)) {
        (
            TMDB_SERIE_SEARCH_BY_TVDBID.replace("{id}", TVDBid),
            format!("TVDB-{}.json", TVDBid),
        )
    } else {
        return Ok((themoviedb_dict, TSDbid, TMDbid.to_string(), IMDbid.to_string()));
    };

    let mode = if movie { "movie" } else { "tv" };
    info!("{}", format!("--- {} ---", mode).chars().cycle().take(157).collect::<String>());

    // Load main JSON data.
    let json_opt = load_file(&filename, &format!("TheMovieDb/json"), &url).await?;
    // Load configuration (for image base URL).
    let config_dict = load_file("TMDB_CONFIG_URL.json", "TheMovieDb", &TMDB_CONFIG_URL).await?;
    let image_base_url = config_dict
        .as_ref()
        .and_then(|v| v.get("images"))
        .and_then(|v| v.get("secure_base_url"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
        
    if json_opt.is_none() {
        info!("TMDB - url: failed to get json {}", TMDB_MOVIE_SEARCH_BY_TMDBID.replace("{id}", TMDbid));
    } else {
        let mut json_data = json_opt.unwrap();
        // If "tv_results" exists, select the first element and set mode to "tv".
        if let Some(tv_results) = json_data.get("tv_results") {
            if let Some(arr) = tv_results.as_array() {
                if !arr.is_empty() {
                    json_data = arr[0].clone();
                }
            }
        } else if let Some(movie_results) = json_data.get("movie_results") {
            if let Some(arr) = movie_results.as_array() {
                if !arr.is_empty() {
                    json_data = arr[0].clone();
                }
            }
        }
        // Title: use "title" for movies or "name" for tv.
        let title_field = json_data.get("title").or(json_data.get("name")).cloned().unwrap_or(json!(""));
        info!("[ ] title: {}", save_dict(&mut themoviedb_dict, &["title"], title_field.clone()));
        // Rating: use "vote_average"
        if let Some(vote_average) = json_data.get("vote_average") {
            info!("[ ] rating: {}", save_dict(&mut themoviedb_dict, &["rating"], vote_average.clone()));
        }
        // Tagline
        if let Some(tagline) = json_data.get("tagline") {
            info!("[ ] tagline: {}", save_dict(&mut themoviedb_dict, &["tagline"], tagline.clone()));
        }
        // Summary: "overview"
        if let Some(overview) = json_data.get("overview") {
            info!("[ ] summary: {}", save_dict(&mut themoviedb_dict, &["summary"], overview.clone()));
        }
        // Duration: "runtime" (minutes)
        if let Some(runtime) = json_data.get("runtime").and_then(|v| v.as_i64()) {
            // Convert minutes to milliseconds.
            let duration_ms = runtime * 60 * 1000;
            info!("[ ] duration: {}", save_dict(&mut themoviedb_dict, &["duration"], json!(duration_ms)));
        }
        // Countries: "origin_country"
        if let Some(countries) = json_data.get("origin_country") {
            info!("[ ] countries: {}", save_dict(&mut themoviedb_dict, &["countries"], countries.clone()));
        }
        // Originally available date: use first_air_date or release_date.
        if let Some(date) = json_data.get("first_air_date").or_else(|| json_data.get("release_date")) {
            info!(
                "[ ] originally_available_at: {}",
                save_dict(&mut themoviedb_dict, &["originally_available_at"], date.clone())
            );
        }
        // Collections: if "belongs_to_collection.name" exists.
        if let Some(collection_obj) = json_data.get("belongs_to_collection").and_then(|v| v.get("name")) {
            info!(
                "[ ] collections: {}",
                save_dict(&mut themoviedb_dict, &["collections"], json!([collection_obj]))
            );
        }
        // Genres: if "genres" exists; sort them.
        if let Some(genres) = json_data.get("genres").and_then(|v| v.as_array()) {
            let mut genre_names: Vec<String> = genres.iter()
                .filter_map(|g| g.get("name").and_then(|v| v.as_str()).map(|s| s.trim().to_string()))
                .collect();
            genre_names.sort();
            info!(
                "[ ] genres: {}",
                save_dict(&mut themoviedb_dict, &["genres"], json!(genre_names))
            );
        }
        // Poster: if "poster_path" exists.
        if let Some(poster_path) = json_data.get("poster_path").and_then(|v| v.as_str()) {
            let full_poster = format!("{}original{}", image_base_url, poster_path);
            info!("[ ] poster: {}", full_poster);
            let local_path = format!("TheMovieDb/poster/{}", poster_path.trim_start_matches('/'));
            save_dict(
                &mut themoviedb_dict,
                &["posters", &full_poster],
                json!((local_path, poster_rank("TheMovieDb", "posters"), Value::Null)),
            );
        }
        // Art (backdrop): if "backdrop_path" exists.
        if let Some(backdrop_path) = json_data.get("backdrop_path").and_then(|v| v.as_str()) {
            let full_art = format!("{}original{}", image_base_url, backdrop_path);
            info!("[ ] art: {}", full_art);
            let local_path = format!("TheMovieDb/artwork/{}", backdrop_path.trim_start_matches('/'));
            let thumb = format!("{}w300{}", image_base_url, backdrop_path);
            save_dict(
                &mut themoviedb_dict,
                &["art", &full_art],
                json!((local_path, poster_rank("TheMovieDb", "art"), thumb)),
            );
        }
        // (Recalculate duration from saved value, if desired.)
        // Set TSDbid or update TMDbid:
        if mode == "tv" {
            TSDbid = dict(&json_data, &["id"]).as_str().unwrap_or("").to_string();
        } else if TMDbid.is_empty() {
            TMDbid = dict(&json_data, &["id"]).as_str().unwrap_or("");
        }
        // If IMDbid is empty, try to get it from "imdb_id".
        if IMDbid.is_empty() {
            IMDbid = dict(&json_data, &["imdb_id"]).as_str().unwrap_or("");
        }
        // Studios: iterate over "production_companies", pick the one with the smallest id.
        if let Some(production_companies) = json_data.get("production_companies").and_then(|v| v.as_array()) {
            if let Some(first) = production_companies.first() {
                if let Some(first_id) = first.get("id").and_then(|v| v.as_i64()) {
                    for studio in production_companies {
                        if let Some(studio_id) = studio.get("id").and_then(|v| v.as_i64()) {
                            if studio_id <= first_id {
                                if let Some(studio_name) = studio.get("name").and_then(|v| v.as_str()) {
                                    info!("[ ] studio: {}", save_dict(&mut themoviedb_dict, &["studio"], json!(studio_name.trim())));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    info!("{}", "--- return ---".repeat(1));
    info!("TheMovieDb_dict: {}", dict_string(&themoviedb_dict, 4));
    // Return a tuple: (TheMovieDb_dict, TSDbid, TMDbid, IMDbid)
    Ok((themoviedb_dict, TSDbid, TMDbid.to_string(), IMDbid.to_string()))
}
