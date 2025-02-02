// src/api_providers/themoviedb.rs

use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use log::{info, debug, error};
use urlencoding::encode;

// -----------------------------------------------------------------------------
// Constants – Adapted from your Python code.
const TMDB_API_KEY: &str = "7f4a0bd0bd3315bb832e17feda70b5cd"; 

lazy_static! {
    // Search for movies using query.
    static ref TMDB_MOVIE_SEARCH: String = format!(
        "https://api.tmdb.org/3/search/movie?api_key={}&query={{query}}&year=&language=en&include_adult=true",
        TMDB_API_KEY
    );
    // Fetch details by TMDb id.
    static ref TMDB_MOVIE_SEARCH_BY_TMDBID: String = format!(
        "https://api.tmdb.org/3/movie/{{id}}?api_key={}&append_to_response=releases,credits,trailers,external_ids&language=en",
        TMDB_API_KEY
    );
    // For series, we use the “find” endpoint with TVDB id.
    static ref TMDB_SERIE_SEARCH_BY_TVDBID: String = format!(
        "https://api.themoviedb.org/3/find/{{id}}?api_key={}&external_source=tvdb_id&append_to_response=releases,credits,trailers,external_ids&language=en",
        TMDB_API_KEY
    );
    // Configuration URL for images.
    static ref TMDB_CONFIG_URL: String = format!(
        "https://api.tmdb.org/3/configuration?api_key={}",
        TMDB_API_KEY
    );
    // URL to fetch images (we will use this to construct poster and backdrop URLs)
    static ref TMDB_MOVIE_IMAGES_URL: String = format!(
        "https://api.tmdb.org/3/{{mode}}/{{id}}/images?api_key={}",
        TMDB_API_KEY
    );
}

// -----------------------------------------------------------------------------
// Helper functions

/// Traverses a JSON value by the list of keys and returns a cloned value or an empty object.
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

/// Pretty-print a JSON value with indentation.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Inserts a value into a JSON object at a nested key path, creating intermediate objects as needed.
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

/// Mimics SaveDict: updates a JSON object at the nested key path and returns the inserted value.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Dummy poster_rank function – returns 0 (you can update with your own ranking logic).
fn poster_rank(service: &str, typ: &str) -> i32 {
    debug!("poster_rank({}, {}) called", service, typ);
    0
}

/// Loads a JSON file from a URL. (In production, you might implement caching and local file checking.)
async fn load_file(filename: &str, relative_directory: &str, url: &str) -> Result<Option<Value>> {
    debug!(
        "Loading file: filename='{}', relative_directory='{}', url='{}'",
        filename, relative_directory, url
    );
    let client = Client::new();
    let resp = client.get(url).send().await?;
    if resp.status().is_success() {
        let json: Value = resp.json().await?;
        Ok(Some(json))
    } else {
        error!("Failed to load file from {}: {}", url, resp.status());
        Ok(None)
    }
}

// -----------------------------------------------------------------------------
// Public API functions

/// Fetch metadata from TheMovieDb API.
///
/// For movies the function calls the movie details endpoint;
/// for series, it calls the “find” endpoint using TVDB id.
///
/// Returns a unified JSON object with keys such as title, rating, summary, duration, poster URL, art URL, etc.
pub async fn get_metadata(anime_id: &str, movie: bool) -> Result<Value> {
    info!("TheMovieDb: Fetching metadata for anime_id: {} (movie: {})", anime_id, movie);
    
    // Decide which endpoint to use based on the ID format.
    // (In our implementation, we assume that if anime_id consists solely of digits,
    // it is a TVDB id for series search; otherwise, if it starts with "tt" or is non‐digit,
    // we try to treat it as a movie via TMDb.)
    let (url, filename) = if movie {
        // For movies we expect a TMDb id or IMDb id.
        // Here we assume anime_id is a TMDb id if it’s all digits, or an IMDb id if it starts with "tt".
        if anime_id.starts_with("tt") {
            (
                TMDB_MOVIE_SEARCH_BY_TMDBID.replace("{id}", anime_id),
                format!("IMDb-{}.json", anime_id),
            )
        } else if anime_id.chars().all(|c| c.is_digit(10)) {
            (
                TMDB_MOVIE_SEARCH_BY_TMDBID.replace("{id}", anime_id),
                format!("TMDB-{}.json", anime_id),
            )
        } else {
            return Err(anyhow!("Invalid movie id format: {}", anime_id));
        }
    } else {
        // For series, we assume anime_id is a TVDB id (all digits).
        if anime_id.chars().all(|c| c.is_digit(10)) {
            (
                TMDB_SERIE_SEARCH_BY_TVDBID.replace("{id}", anime_id),
                format!("TVDB-{}.json", anime_id),
            )
        } else {
            return Err(anyhow!("Invalid series id format: {}", anime_id));
        }
    };

    let mode = if movie { "movie" } else { "tv" };
    info!("{}", format!("--- {} ---", mode).chars().cycle().take(157).collect::<String>());

    // Load main JSON data from TMDb.
    let json_opt = load_file(&filename, "TheMovieDb/json", &url).await?;
    // Load configuration to obtain the image base URL.
    let config_opt = load_file("TMDB_CONFIG_URL.json", "TheMovieDb", &TMDB_CONFIG_URL).await?;
    let image_base_url = if let Some(config) = config_opt {
        config.get("images")
            .and_then(|v| v.get("secure_base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
    } else {
        ""
    };

    if json_opt.is_none() {
        info!("TMDB - Failed to get JSON from URL: {}", url);
        return Err(anyhow!("Failed to fetch TMDb data"));
    }

    // We expect the returned JSON to contain a results array.
    let mut json_data = json_opt.unwrap();

    // For TV series search the API returns a field "tv_results" (and for movies "movie_results").
    if !movie {
        if let Some(tv_results) = json_data.get("tv_results") {
            if let Some(arr) = tv_results.as_array() {
                if !arr.is_empty() {
                    json_data = arr[0].clone();
                }
            }
        }
    } else {
        if let Some(movie_results) = json_data.get("movie_results") {
            if let Some(arr) = movie_results.as_array() {
                if !arr.is_empty() {
                    json_data = arr[0].clone();
                }
            }
        }
    }

    let mut themoviedb_dict = json!({});

    // Save title: use "title" for movies or "name" for series.
    let title_field = json_data.get("title").or(json_data.get("name")).cloned().unwrap_or(json!(""));
    info!("[ ] title: {}", save_dict(&mut themoviedb_dict, &["title"], title_field.clone()));
    // Save rating: use "vote_average".
    if let Some(vote_average) = json_data.get("vote_average") {
        info!("[ ] rating: {}", save_dict(&mut themoviedb_dict, &["rating"], vote_average.clone()));
    }
    // Save tagline.
    if let Some(tagline) = json_data.get("tagline") {
        info!("[ ] tagline: {}", save_dict(&mut themoviedb_dict, &["tagline"], tagline.clone()));
    }
    // Save summary: use "overview".
    if let Some(overview) = json_data.get("overview") {
        info!("[ ] summary: {}", save_dict(&mut themoviedb_dict, &["summary"], overview.clone()));
    }
    // Save duration: use "runtime" (minutes) converted to milliseconds.
    if let Some(runtime) = json_data.get("runtime").and_then(|v| v.as_i64()) {
        let duration_ms = runtime * 60 * 1000;
        info!("[ ] duration: {}", save_dict(&mut themoviedb_dict, &["duration"], json!(duration_ms)));
    }
    // Save countries: "origin_country".
    if let Some(countries) = json_data.get("origin_country") {
        info!("[ ] countries: {}", save_dict(&mut themoviedb_dict, &["countries"], countries.clone()));
    }
    // Save originally available date: first_air_date or release_date.
    if let Some(date) = json_data.get("first_air_date").or_else(|| json_data.get("release_date")) {
        info!(
            "[ ] originally_available_at: {}",
            save_dict(&mut themoviedb_dict, &["originally_available_at"], date.clone())
        );
    }
    // Save collections: if "belongs_to_collection.name" exists.
    if let Some(collection_obj) = json_data.get("belongs_to_collection").and_then(|v| v.get("name")) {
        info!(
            "[ ] collections: {}",
            save_dict(&mut themoviedb_dict, &["collections"], json!([collection_obj]))
        );
    }
    // Save genres: sort them alphabetically.
    if let Some(genres) = json_data.get("genres").and_then(|v| v.as_array()) {
        let mut genre_names: Vec<String> = genres.iter()
            .filter_map(|g| g.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();
        genre_names.sort();
        info!(
            "[ ] genres: {}",
            save_dict(&mut themoviedb_dict, &["genres"], json!(genre_names))
        );
    }
    // Save poster image: if "poster_path" exists.
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
    // Save backdrop (art): if "backdrop_path" exists.
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

    // For TV series searches, TMDb returns results under "tv_results" and sets an "id" field;
    // we store that value in TSDbid.
    if !movie {
        TSDbid = dict(&json_data, &["id"]).as_str().unwrap_or("").to_string();
    }
    // If TMDb id is empty, update it from the JSON.
    if TMDbid.is_empty() {
        TMDbid = dict(&json_data, &["id"]).as_str().unwrap_or("");
    }
    // If IMDbid is empty, try to get it from "imdb_id".
    if IMDbid.is_empty() {
        IMDbid = dict(&json_data, &["imdb_id"]).as_str().unwrap_or("");
    }

    info!("{}", "--- return ---".repeat(1));
    info!("TheMovieDb_dict: {}", dict_string(&themoviedb_dict, 4));
    // Return a tuple: (TheMovieDb_dict, TSDbid, TMDbid, IMDbid)
    Ok((themoviedb_dict, TSDbid, TMDbid.to_string(), IMDbid.to_string()))
}

/// Search for movies/series on TheMovieDb.
///
/// For movies this function uses TMDb’s search endpoint; for TV series it uses the “find” endpoint
/// with a TVDB id. It returns a vector of JSON objects representing search results.
pub async fn search(query: &str, lang: &str, _manual: bool, movie: bool) -> Result<Vec<Value>> {
    info!("TheMovieDb: Searching for query: '{}'", query);
    let client = Client::new();
    let mode = if movie { "movie" } else { "tv" };
    let search_url = if movie {
        // Replace {query} with URL-encoded query.
        TMDB_MOVIE_SEARCH.replace("{query}", &encode(query))
    } else {
        // For series search, assume query is a TVDB id.
        TMDB_SERIE_SEARCH_BY_TVDBID.replace("{id}", query)
    };
    let request_url = reqwest::Url::parse(&search_url)?;
    let resp = client.get(request_url).send().await?;
    if !resp.status().is_success() {
        return Err(anyhow!("TMDB search failed with status: {}", resp.status()));
    }
    let resp_json: Value = resp.json().await?;
    let results = if movie {
        resp_json.get("results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    } else {
        resp_json.get("tv_results")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    };
    Ok(results)
}
