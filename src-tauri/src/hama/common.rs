// common.rs

use anyhow::{Result, anyhow};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::{Client};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use log::{info, debug, error};
use unicode_normalization::UnicodeNormalization; // For Unicode normalization

//
// Global Constants and Globals
//
lazy_static! {
    /// PlexRoot: In production, calculate based on platform.
    pub static ref PLEX_ROOT: String = "/path/to/plex/root".to_string();
    /// CachePath: where cached files are stored.
    pub static ref CACHE_PATH: String = {
        let mut p = PathBuf::from(&*PLEX_ROOT);
        p.push("Plug-in Support");
        p.push("Data");
        p.push("com.plexapp.agents.hama");
        p.push("DataItems");
        p.to_string_lossy().to_string()
    };
    /// downloaded: a counter for each type.
    pub static ref DOWNLOADED: Mutex<BTreeMap<&'static str, i32>> = Mutex::new(BTreeMap::from([
        ("posters", 0),
        ("art", 0),
        ("seasons", 0),
        ("banners", 0),
        ("themes", 0),
        ("thumbs", 0),
    ]));
    /// netLocked: used for simple network locking.
    pub static ref NET_LOCKED: Mutex<BTreeMap<String, (bool, u64)>> = Mutex::new(BTreeMap::new());
    /// WEB_LINK format string.
    pub static ref WEB_LINK: String = "<a href='{}' target='_blank'>{}</a>".to_string();
    /// TVDB_SERIE_URL and ANIDB_SERIE_URL.
    pub static ref TVDB_SERIE_URL: String = "https://thetvdb.com/?tab=series&id=".to_string();
    pub static ref ANIDB_SERIE_URL: String = "https://anidb.net/anime/".to_string();
    /// Default preferences, fields, and source lists.
    pub static ref DEFAULT_PREFS: Vec<&'static str> = vec![
        "SerieLanguagePriority", "EpisodeLanguagePriority", "PosterLanguagePriority",
        "AnidbGenresAddWeights", "MinimumWeight", "adult", "OMDbApiKey"
    ];
    pub static ref FIELD_LIST_MOVIES: Vec<&'static str> = vec![
        "original_title", "title", "title_sort", "roles", "studio", "year", "originally_available_at",
        "tagline", "summary", "content_rating", "content_rating_age", "producers", "directors", "writers",
        "countries", "posters", "art", "themes", "rating", "quotes", "trivia", "genres", "collections"
    ];
    pub static ref FIELD_LIST_SERIES: Vec<&'static str> = vec![
        "title", "title_sort", "originally_available_at", "duration", "rating", "reviews", "collections",
        "genres", "tags", "summary", "extras", "countries", "rating_count", "content_rating", "studio",
        "countries", "posters", "banners", "art", "themes", "roles", "original_title",
        "rating_image", "audience_rating", "audience_rating_image"
    ];
    pub static ref FIELD_LIST_SEASONS: Vec<&'static str> = vec!["summary", "posters", "art"];
    pub static ref FIELD_LIST_EPISODES: Vec<&'static str> = vec![
        "title", "summary", "originally_available_at", "writers", "directors", "producers",
        "guest_stars", "rating", "thumbs", "duration", "content_rating", "content_rating_age", "absolute_index"
    ];
    pub static ref SOURCE_LIST: Vec<&'static str> = vec![
        "AniDB", "MyAnimeList", "FanartTV", "OMDb", "TheTVDB", "TheMovieDb", "Plex", "AnimeLists", "tvdb4", "TVTunes", "Local", "AniList"
    ];
    pub static ref MOVIE_TO_SERIE_US_RATING: BTreeMap<&'static str, &'static str> = {
        let mut m = BTreeMap::new();
        m.insert("G", "TV-Y7");
        m.insert("PG", "TV-G");
        m.insert("PG-13", "TV-PG");
        m.insert("R", "TV-14");
        m.insert("R+", "TV-MA");
        m.insert("Rx", "NC-17");
        m
    };
    pub static ref COMMON_HEADERS: BTreeMap<&'static str, &'static str> = {
        let mut m = BTreeMap::new();
        m.insert("User-agent", "Plex/HAMA");
        m.insert("Content-type", "application/json");
        m
    };
    pub static ref THROTTLE: Mutex<BTreeMap<String, Vec<u64>>> = Mutex::new(BTreeMap::new());
}

//
// Utility Functions
//

/// Pretty-print a JSON object.
pub fn dict_string(value: &Value, indent: usize) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
}

/// Traverse a JSON object by keys.
pub fn dict(value: &Value, keys: &[&str]) -> Value {
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

/// Set a nested value in a JSON object (creating intermediate objects as needed).
pub fn set_nested(target: &mut Value, keys: &[&str], new_value: Value) {
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

/// Save a value into a JSON object (alias for set_nested).
pub fn save_dict(target: &mut Value, keys: &[&str], value: Value) -> Value {
    set_nested(target, keys, value.clone());
    value
}

/// Update a JSON object with another JSON object (merging keys).
pub fn update_dict(dst: &mut Value, src: &Value) {
    if let (Some(dst_obj), Some(src_obj)) = (dst.as_object_mut(), src.as_object()) {
        for (k, v) in src_obj.iter() {
            dst_obj.insert(k.clone(), v.clone());
        }
    }
}

/// Natural sort key: splits a string into non-digit and digit parts.
pub fn natural_sort_key(s: &str) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"([0-9]+)").unwrap();
    }
    RE.split(s)
        .map(|x| x.to_lowercase())
        .collect()
}

/// Replace substrings in a string given lists of strings to search and their replacements.
pub fn replace_list(string: &str, a: &[&str], b: &[&str]) -> String {
    let mut result = string.to_string();
    for (old, new) in a.iter().zip(b.iter()) {
        result = result.replace(old, new);
    }
    result
}

/// Compute the Levenshtein distance between two strings.
pub fn levenshtein_distance(first: &str, second: &str) -> usize {
    let mut costs: Vec<usize> = (0..=second.len()).collect();
    for (i, c1) in first.chars().enumerate() {
        let mut last_cost = i;
        costs[0] = i + 1;
        for (j, c2) in second.chars().enumerate() {
            let new_cost = if c1 == c2 { last_cost } else { last_cost + 1 };
            last_cost = costs[j + 1];
            costs[j + 1] = std::cmp::min(std::cmp::min(costs[j] + 1, costs[j + 1] + 1), new_cost);
        }
    }
    *costs.last().unwrap()
}

/// Compute the Levenshtein ratio as a percentage.
pub fn levenshtein_ratio(first: &str, second: &str) -> usize {
    if first.is_empty() || second.is_empty() {
        return 0;
    }
    let dist = levenshtein_distance(first, second);
    let max_len = first.len().max(second.len());
    if max_len == 0 { 0 } else { 100 - (100 * dist / max_len) }
}

/// Return the element at index in a slice as a string, or empty string if not found.
pub fn is_index<T: ToString>(var: &[T], index: usize) -> String {
    var.get(index).map(|x| x.to_string()).unwrap_or_else(|| "".to_string())
}

/// Cleanse a title: remove diacritics, remove parenthesized/bracketed text, lowercase, and replace punctuation.
pub fn cleanse_title(string: &str) -> String {
    // Normalize using NFC.
    let normalized: String = string.nfc().collect();
    // Remove parenthesized text.
    let re_paren = Regex::new(r"\([^\(\)]*?\)").unwrap();
    let mut cleaned = re_paren.replace_all(&normalized, " ").to_string();
    // Remove bracketed text.
    let re_bracket = Regex::new(r"\[[^\[\]]*?\]").unwrap();
    cleaned = re_bracket.replace_all(&cleaned, " ").to_string();
    // Replace punctuation with spaces.
    let replace_chars = "`:/*?-.,;_";
    for ch in replace_chars.chars() {
        cleaned = cleaned.replace(ch, " ");
    }
    // Collapse multiple spaces.
    cleaned = cleaned.split_whitespace().collect::<Vec<&str>>().join(" ");
    cleaned.to_lowercase()
}

/// SSL open: download content from a URL without verifying certificates.
pub fn ssl_open(url: &str, headers: &BTreeMap<String, String>, timeout: u64) -> Result<String> {
    // Using reqwest with certificate verification disabled.
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(Duration::from_secs(timeout))
        .build()?;
    let mut req = client.get(url);
    for (k, v) in headers {
        req = req.header(k, v);
    }
    let resp = req.send()?.text()?;
    Ok(resp)
}

/// Get the status code for a URL using an HTTP HEAD request.
pub fn get_status_code(url: &str) -> Result<u16> {
    let client = reqwest::blocking::Client::new();
    let resp = client.head(url).send()?;
    Ok(resp.status().as_u16())
}

/// Save a file to the cache directory.
pub fn save_file(filename: &str, file: &str, relative_directory: &str) -> Result<()> {
    let relative_filename = Path::new(relative_directory).join(filename);
    let fullpath_directory = Path::new(&*CACHE_PATH).join(relative_directory);
    if !fullpath_directory.exists() {
        fs::create_dir_all(&fullpath_directory)?;
    }
    let full_path = fullpath_directory.join(filename);
    fs::write(&full_path, file)?;
    info!("common.save_file() - Saved file: {:?}", full_path);
    Ok(())
}

/// Decompress a gzip–compressed byte slice.
pub fn decompress(file: &[u8]) -> Result<Vec<u8>> {
    use flate2::read::GzDecoder;
    use std::io::Read;
    let mut decoder = GzDecoder::new(file);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed)?;
    Ok(decompressed)
}

/// Convert a file’s bytes into a JSON, XML string, or plain string.
pub fn object_from_file(file: &[u8]) -> Result<Value> {
    let file_str = String::from_utf8_lossy(file);
    if file_str.starts_with("<?xml") {
        // Return XML as a string (for further XML parsing later).
        Ok(json!(file_str.to_string()))
    } else if file_str.trim_start().starts_with("{") {
        let parsed: Value = serde_json::from_str(&file_str)?;
        Ok(parsed)
    } else if file_str.trim().is_empty() {
        info!("Empty file");
        Ok(json!({}))
    } else {
        Ok(json!(file_str.to_string()))
    }
}

/// Load a file from cache. Returns (file_object, file_age in seconds).
pub fn load_file_cache(filename: &str, relative_directory: &str) -> Result<(Value, f64)> {
    let relative_filename = Path::new(relative_directory).join(filename);
    let full_path = Path::new(&*CACHE_PATH).join(relative_directory).join(filename);
    let mut file_object = json!({});
    let mut file_age = 0.0;
    if full_path.exists() {
        let file_bytes = fs::read(&full_path)?;
        file_object = object_from_file(&file_bytes)?;
        let metadata = fs::metadata(&full_path)?;
        file_age = SystemTime::now().duration_since(metadata.modified()?)?.as_secs_f64();
    }
    Ok((file_object, file_age))
}

/// Throttle: remove timestamps older than a given duration from the throttle list.
pub fn throttle_count(index: &str, duration: u64) -> usize {
    let mut throttle = THROTTLE.lock().unwrap();
    if !throttle.contains_key(index) {
        return 0;
    }
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let entries = throttle.get_mut(index).unwrap();
    entries.retain(|&t| t >= now - duration);
    entries.len()
}

/// Throttle: add the current time (in seconds) to the throttle list for the given index.
pub fn throttle_add(index: &str) {
    let mut throttle = THROTTLE.lock().unwrap();
    throttle.entry(index.to_string()).or_insert_with(Vec::new).push(
        SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()
    );
}

/// Load a file from network with caching and throttling.
pub async fn load_file(
    filename: &str,
    relative_directory: &str,
    url: &str,
    headers: Option<BTreeMap<String, String>>,
    data: Option<Value>,
    cache: u64,
    sleep_duration: u64,
    throttle: Option<(&str, u64, usize)>,
) -> Result<Value> {
    let mut merged_headers = COMMON_HEADERS
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<BTreeMap<String, String>>();
    if let Some(h) = headers {
        for (k, v) in h {
            merged_headers.insert(k, v);
        }
    }
    let filename = if filename.ends_with(".gz") {
        &filename[..filename.len()-3]
    } else {
        filename
    };
    let (cached_file, file_age) = load_file_cache(filename, relative_directory)?;
    if !cached_file.is_null() && file_age < cache as f64 {
        debug!("common.load_file() - Loaded cached file: {} (age: {}s)", filename, file_age);
        return Ok(cached_file);
    }
    if let Some((index, duration, max_count)) = throttle {
        while throttle_count(index, duration) >= max_count {
            info!("Throttle '{}' max hit. Waiting...", index);
            sleep(Duration::from_secs(60)).await;
        }
        throttle_add(index);
    }
    let client = Client::new();
    let mut req = client.get(url);
    for (k, v) in merged_headers.iter() {
        req = req.header(k, v);
    }
    if let Some(data) = data {
        req = req.json(&data);
    }
    let response = req.send().await?;
    if !response.status().is_success() {
        return Err(anyhow!("Failed to load file from {}: {}", url, response.status()));
    }
    let mut bytes = response.bytes().await?.to_vec();
    if url.ends_with(".gz") {
        bytes = decompress(&bytes)?;
    }
    sleep(Duration::from_secs(sleep_duration)).await;
    let file_str = String::from_utf8_lossy(&bytes);
    save_file(filename, &file_str, relative_directory)?;
    Ok(object_from_file(&bytes)?)
}

/// Download metadata (e.g. images, themes) and save into the given metadata field.
pub fn metadata_download(
    metadata_root: &mut Value,
    metatype: &mut Value,
    url: &str,
    filename: &str,
    num: i32,
    url_thumbnail: Option<&str>,
) {
    let field = if metatype == metadata_root.get("posters").unwrap_or(&json!({})) {
        "posters"
    } else if metatype == metadata_root.get("art").unwrap_or(&json!({})) {
        "art"
    } else if metatype == metadata_root.get("banners").unwrap_or(&json!({})) {
        "banners"
    } else if metatype == metadata_root.get("themes").unwrap_or(&json!({})) {
        "themes"
    } else if filename.starts_with("TVDB/episodes/") {
        "thumbs"
    } else {
        "seasons"
    };
    if metatype.get(url).is_some() {
        info!("url: '{}', num: '{}', filename: '{}' already present", url, num, filename);
    } else {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(get_status_code(url));
        let result_str = match result {
            Ok(code) => code.to_string(),
            Err(e) => format!("{}", e),
        };
        info!("Return code: '{}', url: '{}'", result_str, url);
        if result_str == "200" || result_str == "*" {
            let local_file = format!("TelevisionTunes/{}", filename);
            save_dict(metatype, &[url], json!((local_file, 1, Value::Null)));
            info!("[ ] theme: {}", dict_string(metatype, 1));
        }
    }
    let mut downloaded = DOWNLOADED.lock().unwrap();
    if let Some(counter) = downloaded.get_mut(field) {
        *counter += 1;
    }
}

/// Write log messages to a cache file (dummy implementation).
pub fn write_logs(media: &Value, movie: bool, error_log: &mut Value, source: &str, AniDBid: &str, TVDBid: &str) {
    info!("{}", "=== common.write_logs() ===".repeat(1));
    info!("Writing logs for source: {}", source);
    info!("Error log: {}", dict_string(error_log, 1));
    // In production, this function would lock a file, load previous entries, update them, and then write back.
}

/// Return additional tags for a media file (e.g. extension, dubbed/subbed).
pub fn other_tags(media: &Value, movie: bool, status: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if movie {
        if let Some(file) = media.get("file").and_then(|v| v.as_str()) {
            if let Some(ext) = Path::new(file).extension().and_then(|s| s.to_str()) {
                tags.push(ext.to_string());
            }
        }
    } else {
        if status == "Ended" || status == "Continuing" {
            tags.push(status.to_string());
        }
    }
    tags
}

/// Update a metadata field if new data differs from old data (dummy implementation).
pub fn update_meta_field(
    metadata: &mut Value,
    meta_root: &Value,
    field_list: &[&str],
    field: &str,
    source: &str,
    movie: bool,
    source_list: &[String],
) {
    info!("[UpdateMetaField] Updating field '{}' from source '{}'", field, source);
    // For demonstration, we simply replace metadata[field] with meta_root[field].
    if let Some(new_value) = meta_root.get(field) {
        metadata[field] = new_value.clone();
    }
}

/// Update all metadata fields according to source priorities (dummy implementation).
pub fn update_meta(metadata: &mut Value, _media: &Value, movie: bool, meta_sources: &Value, mapping_list: &mut Value) {
    info!("{}", "=== common.UpdateMeta() ===".repeat(1));
    let now = chrono::Utc::now().to_rfc3339();
    metadata["updated"] = json!(now);
    info!("Metadata updated at {}", now);
}

/// Sort a title by removing common leading articles based on language.
pub fn sort_title(title: &str, language: &str) -> String {
    let dict_sort: BTreeMap<&str, Vec<&str>> = BTreeMap::from([
        ("en", vec!["The", "A", "An"]),
        ("fr", vec!["Le", "La", "Les", "L", "Un", "Une", "Des"]),
        ("sp", vec!["El", "La", "Las", "Lo", "Los", "Uno", "Una"]),
    ]);
    let parts: Vec<&str> = title.splitn(2, ' ').collect();
    if let Some(prefixes) = dict_sort.get(language) {
        if parts.len() > 1 && prefixes.contains(&parts[0]) {
            return parts[1].to_string();
        }
    }
    title.to_string()
}

/// Poster rank: compute a rank for an image given the source, type, language, and an adjustment.
pub fn poster_rank_common(source: &str, image_type: &str, language: &str, rank_adjustment: i32) -> i32 {
    let max_rank = 100;
    let language_posters: Vec<&str> = prefs().get("PosterLanguagePriority")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim())
        .collect();
    let priority_posters: Vec<&str> = prefs().get(image_type)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim())
        .collect();

    let lp_len = language_posters.len() as i32;
    let pp_len = priority_posters.len() as i32;
    let lp_pos = language_posters.iter().position(|&x| x == language).map(|x| x as i32).unwrap_or(lp_len);
    let pp_pos = priority_posters.iter().position(|&x| x == source).map(|x| x as i32).unwrap_or(pp_len);
    let lp_block_size = max_rank / lp_len;
    let pp_block_size = lp_block_size / pp_len;
    let mut rank = (lp_pos * lp_block_size) + (pp_pos * pp_block_size) + 1 + rank_adjustment;
    if rank > 100 { rank = 100; }
    if rank < 1 { rank = 1; }
    rank
}
