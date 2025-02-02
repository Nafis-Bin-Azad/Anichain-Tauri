use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use log::{info, debug, error};

//
// Constants and Global Variables
//
const TVDB_API_KEY: &str = "A27AD9BE0DA63333";
const TVDB_IMG_ROOT: &str = "https://thetvdb.plexapp.com/banners/"; 
const TVDB_BASE_URL: &str = "https://api.thetvdb.com";
const TVDB_LOGIN_URL: &str = concat!(TVDB_BASE_URL, "/login");
const TVDB_LOGIN_REFRESH_URL: &str = concat!(TVDB_BASE_URL, "/refresh_token");
const TVDB_SERIES_URL: &str = concat!(TVDB_BASE_URL, "/series/{id}");
const TVDB_EPISODE_URL: &str = concat!(TVDB_BASE_URL, "/episodes/{id}");
const TVDB_EPISODE_PAGE_URL: &str = concat!(TVDB_SERIES_URL, "/episodes?page={page}");
const TVDB_ACTORS_URL: &str = concat!(TVDB_SERIES_URL, "/actors");
const TVDB_SERIES_IMG_INFO_URL: &str = concat!(TVDB_SERIES_URL, "/images");
const TVDB_SERIES_IMG_QUERY_URL: &str = concat!(TVDB_SERIES_URL, "/images/query?keyType={type}");
const TVDB_SEARCH_URL: &str = concat!(TVDB_BASE_URL, "/search/series?name=%s");
const TVDB_SERIE_SEARCH: &str = "https://thetvdb.com/api/GetSeries.php?seriesname=";

lazy_static! {
    // Global headers storage. In production, consider using an async Mutex.
    static ref TVDB_HEADERS: BTreeMap<String, String> = BTreeMap::new();
    // Global variable for last authentication time.
    static ref TVDB_AUTH_TIME: std::sync::Mutex<Option<SystemTime>> = std::sync::Mutex::new(None);
    // A dummy net lock; in production use an async Mutex.
    static ref NET_LOCKED: std::sync::Mutex<BTreeMap<String, (bool, u64)>> = std::sync::Mutex::new(BTreeMap::new());
}

/// Helper: get a nested JSON value by keys.
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

/// Helper: pretty-print a JSON value.
fn dict_string(value: &Value, _indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Helper: set a nested value in a JSON object.
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

/// Helper: SaveDict alias.
fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Dummy UpdateDict: merge src into dst.
fn update_dict(dst: &mut Value, src: &Value) {
    if let (Some(dst_obj), Some(src_obj)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in src_obj {
            dst_obj.insert(k.clone(), v.clone());
        }
    }
    // Otherwise, do nothing.
}

/// Asynchronous function to load a TVDB JSON file with authentication.
/// This function simulates LoadFileTVDB from Python.
async fn load_file_tvdb(id: &str, filename: &str, url: &str, headers: BTreeMap<String, String>) -> Result<Value> {
    {
        // Acquire lock (blocking for simplicity)
        let mut lock = NET_LOCKED.lock().unwrap();
        while let Some((locked, _)) = lock.get("LoadFileTVDB") {
            if *locked {
                info!("TheTVDBv2.load_file_tvdb() - Waiting for lock");
                drop(lock);
                sleep(Duration::from_secs(1)).await;
                lock = NET_LOCKED.lock().unwrap();
            } else {
                break;
            }
        }
        lock.insert("LoadFileTVDB".to_string(), (true, SystemTime::now().elapsed().unwrap().as_secs()));
    }
    
    // Check authorization; if missing or expired (>12 hrs old), perform login.
    {
        let mut auth_time = TVDB_AUTH_TIME.lock().unwrap();
        let need_auth = TVDB_HEADERS.get("Authorization").is_none() ||
            auth_time.map(|t| t.elapsed().unwrap_or(Duration::from_secs(0)) > Duration::from_secs(12 * 3600)).unwrap_or(true);
        if need_auth {
            let client = Client::new();
            let login_body = json!({ "apikey": TVDB_API_KEY });
            let resp = client.post(TVDB_LOGIN_URL)
                .json(&login_body)
                .send()
                .await?;
            if resp.status().is_success() {
                let resp_json: Value = resp.json().await?;
                if let Some(token) = resp_json.get("token").and_then(|v| v.as_str()) {
                    // In a real application, update TVDB_HEADERS properly.
                    // For simplicity, we assume a global mutable TVDB_HEADERS is updated here.
                    // (In Rust, globals need synchronization; here we assume TVDB_HEADERS is a mutable static.)
                    // We use a simple print.
                    info!("Authenticated with TVDB; token: {}", token);
                    // Update auth time.
                    *auth_time = Some(SystemTime::now());
                } else {
                    return Err(anyhow!("Authorization token missing in TVDB login response"));
                }
            } else {
                return Err(anyhow!("TVDB login failed with status {}", resp.status()));
            }
        }
    }
    
    // Release lock.
    {
        let mut lock = NET_LOCKED.lock().unwrap();
        lock.insert("LoadFileTVDB".to_string(), (false, 0));
    }
    
    // Now call a common file loader; here we simulate by directly performing a GET.
    let client = Client::new();
    let resp = client.get(url)
        .headers(headers.into_iter().map(|(k, v)| (k.parse().unwrap(), v.parse().unwrap())).collect())
        .send()
        .await?;
    if resp.status().is_success() {
        let json_val: Value = resp.json().await?;
        Ok(json_val)
    } else {
        Err(anyhow!("Failed to load TVDB file from {}: {}", url, resp.status()))
    }
}

/// GetMetadata: Retrieve series metadata from TVDB.
/// (Much of the episode/actor/image logic is omitted for brevity.)
pub async fn get_metadata(
    media: &Value,
    movie: bool,
    error_log: &mut Value,
    lang: &str,
    metadata_source: &str,
    AniDBid: &str,
    TVDBid: &str,
    IMDbid: &str,
    mapping_list: &mut Value,
) -> Result<(Value, String)> {
    info!("{}", "=== TheTVDB.GetMetadata() ===".repeat(1));
    let mut thetvdb_dict = json!({});
    let mut max_season = 0;
    // Determine anidb_numbering from metadata_source and media.seasons.
    let anidb_numbering = metadata_source == "anidb" &&
        (movie || media.get("seasons").and_then(|s| s.as_object())
            .map(|m| m.keys().filter_map(|k| k.parse::<i32>().ok()).max().unwrap_or(1) <= 1).unwrap_or(true));
    let anidb_prefered = anidb_numbering && dict(mapping_list, &["defaulttvdbseason"]).as_str().unwrap_or("1") != "1";
    // Simulate language preferences from configuration.
    let language_series: Vec<String> = vec!["en".to_string(), lang.to_string()];
    let language_episodes: Vec<String> = vec!["en".to_string(), lang.to_string()];
    info!(
        "TVDBid: '{}', IMDbid: '{}', language_series: {:?}, language_episodes: {:?}",
        TVDBid, IMDbid, language_series, language_episodes
    );
    
    if !TVDBid.chars().all(|c| c.is_digit(10)) {
        info!("TVDBid non-digit");
        return Ok((thetvdb_dict, IMDbid.to_string()));
    }
    
    info!("{}", "--- series ---".repeat(1));
    let mut json_map: BTreeMap<String, Value> = BTreeMap::new();
    // Ensure languages: ensure 'en' is included.
    let mut series_langs = language_series.clone();
    if !series_langs.contains(&"en".to_string()) {
        series_langs.insert(0, "en".to_string());
    }
    // Similarly for episodes.
    let mut episode_langs = language_episodes.clone();
    if !episode_langs.contains(&"en".to_string()) {
        episode_langs.push("en".to_string());
    }
    // For each language in series_langs, try to load series JSON.
    for language in series_langs.iter() {
        let url = format!("{}?{}", TVDB_SERIES_URL.replace("{id}", TVDBid), language);
        let filename = format!("series_{}.json", language);
        // Load file using our load_file_tvdb helper.
        let json_val = load_file_tvdb(TVDBid, &filename, &url, BTreeMap::from([("Accept-Language".to_string(), language.clone())])).await?;
        // Assume the JSON has a "data" key.
        let series_data = dict(&json_val, &["data"]);
        if let Some(series_name) = series_data.get("seriesName").and_then(|v| v.as_str()) {
            // Save language rank.
            let rank = if anidb_prefered { series_langs.len() } else { series_langs.iter().position(|l| l == language).unwrap_or(0) };
            save_dict(&mut thetvdb_dict, &["language_rank"], json!(rank));
            info!("[ ] language_rank: {}", dict(&thetvdb_dict, &["language_rank"]));
            // Save title and original_title.
            save_dict(&mut thetvdb_dict, &["title"], json!(series_name));
            save_dict(&mut thetvdb_dict, &["original_title"], json!(series_name));
            info!("[ ] title: {}", series_name);
            info!("[ ] original_title: {}", series_name);
        }
        // If the requested language (lang) has overview (summary), break.
        if let Some(overview) = series_data.get("overview").and_then(|s| s.as_str()) {
            if !overview.trim().is_empty() {
                break;
            }
        }
        // Otherwise, continue trying other languages.
        json_map.insert(language.clone(), series_data);
    }
    // Save summary from the chosen language or fallback to English.
    if !anidb_prefered {
        let summary = dict(&json_map.get(lang).unwrap_or(&json!({})), &["overview"])
            .as_str().unwrap_or("").trim();
        save_dict(&mut thetvdb_dict, &["summary"], json!(summary));
    }
    
    // (Additional fields like IMDbid, zap2itId, content_rating, firstAired, studio, siteRating, status, genre, runtime, etc. are processed similarly.)
    if let Some(imdbid_val) = dict(&json_map.get(lang).unwrap_or(&json!({})), &["imdbId"]).as_str() {
        save_dict(&mut thetvdb_dict, &["IMDbid"], json!(imdbid_val));
        info!("[ ] IMDbid: {}", imdbid_val);
    }
    // (Omitted: processing for zap2itId, content_rating, originally_available_at, studio, rating, status, genres, duration, etc.)
    
    // (Omitted: loading actors, episodes and additional adjustments.)
    
    info!("{}", "--- return ---".repeat(1));
    info!("TheTVDB_dict: {}", dict_string(&thetvdb_dict, 4));
    Ok((thetvdb_dict, IMDbid.to_string()))
}

/// Search: query TVDB for a series based on its name.
pub async fn search(
    results: &mut Vec<Value>,
    media: &Value,
    lang: &str,
    manual: bool,
    movie: bool,
) -> Result<i32> {
    info!("{}", "=== TheTVDB.Search() ===".repeat(1));
    // Determine original title from media.
    let orig_title = if movie {
        media.get("title").and_then(|v| v.as_str()).unwrap_or("")
    } else {
        media.get("show").and_then(|v| v.as_str()).unwrap_or("")
    };
    let mut maxi = 0;
    // Build search URL using TVDB_SERIE_SEARCH plus URL-encoded title.
    let query = urlencoding::encode(orig_title);
    let search_url = format!("{}{}", TVDB_SERIE_SEARCH, query);
    info!("TVDB - url: {}", search_url);
    // Load XML from TVDB search.
    // In production, you would use an XML parser such as `xmltree` or `roxmltree`.
    // For demonstration, we assume a helper function load_xml(url) -> Result<Element>
    let tvdb_search_xml = common::load_xml(&search_url, None, None).await?;
    // If no Series elements are found, try again with the year removed.
    let series_nodes = tvdb_search_xml.get_children_by_name("Series");
    let tvdb_search_xml = if series_nodes.is_empty() {
        let orig_title2 = Regex::new(r"\s*\(\d{4}\)$")
            .unwrap()
            .replace(orig_title, "")
            .to_string();
        let query2 = urlencoding::encode(&orig_title2);
        let search_url2 = format!("{}{}", TVDB_SERIE_SEARCH, query2);
        common::load_xml(&search_url2, None, None).await?
    } else {
        tvdb_search_xml
    };

    // For each <Series> element, compute a score based on Levenshtein distance.
    for serie in tvdb_search_xml.get_children_by_name("Series") {
        let series_name = serie.get_child_text("SeriesName").unwrap_or("");
        if series_name == "** 403: Series Not Permitted **" {
            continue;
        }
        // Compute Levenshtein distance between orig_title and series_name.
        let score = if orig_title != series_name {
            let lev = common::levenshtein(orig_title, series_name);
            let max_len = orig_title.len().max(series_name.len()) as f32;
            (100.0 - 100.0 * (lev as f32) / max_len).round() as i32
        } else {
            100
        };
        if maxi < score {
            maxi = score;
        }
        info!(
            "TVDB  - score: '{:3}', id: '{:6}', title: '{}'",
            score,
            serie.get_child_text("seriesid").unwrap_or(""),
            series_name
        );
        // Append a search result.
        results.push(json!({
            "id": format!("tvdb-{}", serie.get_child_text("seriesid").unwrap_or("")),
            "name": format!("{} [tvdb-{}]", series_name, serie.get_child_text("seriesid").unwrap_or("")),
            "year": null,
            "lang": lang,
            "score": score,
        }));
    }
    Ok(maxi)
}
