// src/api_providers/tvdb.rs

use anyhow::{Result, anyhow};
use reqwest::{Client, header};
use serde_json::{json, Value};
use log::info;
use std::time::{Duration, SystemTime};
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use urlencoding::encode;

// --- Global Token Caching ---
// We cache the TVDB bearer token for 12 hours.
static TVDB_TOKEN: Lazy<Mutex<Option<(String, SystemTime)>>> = Lazy::new(|| Mutex::new(None));

// --- TVDB API Constants ---
const TVDB_API_KEY: &str = "A27AD9BE0DA63333";
const TVDB_BASE_URL: &str = "https://api.thetvdb.com";
const TVDB_LOGIN_URL: &str = "https://api.thetvdb.com/login";
const TVDB_SERIES_URL: &str = "https://api.thetvdb.com/series/{}";
const TVDB_SEARCH_URL: &str = "https://api.thetvdb.com/search/series?name={}&language={}";
const TVDB_CALENDAR_URL: &str = "https://api.thetvdb.com/calendar";

/// Authenticate with TVDB and return a bearer token.
/// The token is cached for 12 hours.
async fn get_token(client: &Client) -> Result<String> {
    // Acquire the token cache lock.
    let mut token_lock = TVDB_TOKEN.lock().await;
    let valid_duration = Duration::from_secs(12 * 3600);
    if let Some((token, timestamp)) = token_lock.as_ref() {
        if timestamp.elapsed().unwrap_or(Duration::ZERO) < valid_duration {
            return Ok(token.clone());
        }
    }
    // Token is missing or expired; perform login.
    let login_payload = json!({ "apikey": TVDB_API_KEY });
    let resp = client.post(TVDB_LOGIN_URL)
        .json(&login_payload)
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("TVDB login failed with status: {}", resp.status()));
    }
    let resp_json: Value = resp.json().await?;
    let token = resp_json.get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("TVDB login response missing token"))?
        .to_string();
    // Cache the token with the current time.
    *token_lock = Some((token.clone(), SystemTime::now()));
    info!("TVDB login successful; token obtained.");
    Ok(token)
}

/// Fetch metadata from TheTVDB for a given series ID.
///
/// This function performs the following:
/// 1. Obtains (or refreshes) a TVDB bearer token.
/// 2. Sends a GET request to the series endpoint with the token in the Authorization header.
/// 3. Returns the parsed JSON response.
///
/// You can later extend this function to handle language parameters or additional fields.
pub async fn get_metadata(anime_id: &str, movie: bool) -> Result<Value> {
    info!(
        "TheTVDB: Fetching metadata for anime_id: {} (movie: {})",
        anime_id, movie
    );
    let client = Client::new();
    let token = get_token(&client).await?;
    let url = format!(TVDB_SERIES_URL, anime_id);
    // Append query parameter "language=en" (modify as needed).
    let request_url = reqwest::Url::parse_with_params(&url, &[("language", "en")])?;
    let resp = client.get(request_url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("Failed to fetch TVDB metadata, status: {}", resp.status()));
    }
    let metadata: Value = resp.json().await?;
    Ok(metadata)
}

/// Search TheTVDB for a series using a query string.
///
/// This function:
/// 1. Authenticates with TVDB to get a token.
/// 2. Constructs a search URL with the query and language parameters.
/// 3. Returns a vector of JSON objects representing the search results.
pub async fn search(query: &str, lang: &str, _manual: bool, _movie: bool) -> Result<Vec<Value>> {
    info!(
        "TheTVDB: Searching for query: '{}' (lang: '{}')",
        query, lang
    );
    let client = Client::new();
    let token = get_token(&client).await?;
    let encoded_query = encode(query);
    let url = format!(TVDB_SEARCH_URL, encoded_query, lang);
    let request_url = reqwest::Url::parse(&url)?;
    let resp = client.get(request_url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("TVDB search failed, status: {}", resp.status()));
    }
    let resp_json: Value = resp.json().await?;
    // TVDB search response should contain a "data" field with an array.
    let results = resp_json.get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(results)
}

/// Fetch the airing schedule from TheTVDB.
///
/// This function authenticates with TVDB and then sends a GET request to the
/// calendar endpoint. The returned JSON is expected to contain the schedule data.
pub async fn fetch_schedule() -> Result<Value> {
    info!("TheTVDB: Fetching airing schedule");
    let client = Client::new();
    let token = get_token(&client).await?;
    let request_url = reqwest::Url::parse(TVDB_CALENDAR_URL)?;
    let resp = client.get(request_url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(anyhow!("TVDB fetch schedule failed, status: {}", resp.status()));
    }
    let schedule: Value = resp.json().await?;
    Ok(schedule)
}
