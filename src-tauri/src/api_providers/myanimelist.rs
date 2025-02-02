// src/api_providers/myanimelist.rs

use anyhow::{Result, anyhow, Context};
use log::{info, debug, error};
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use tokio::time::{sleep, Duration};
use lazy_static::lazy_static;
use regex::Regex;

/// URL template for fetching anime details from MyAnimeList.
const MYANIMELIST_URL_DETAILS_TEMPLATE: &str = 
    "https://api.myanimelist.net/v2/anime/{id}?fields=id,title,pictures,start_date,synopsis,mean,genres,rating,studios,media_type";

/// For caching purposes (in seconds); here we set one week.
const MYANIMELIST_CACHE_TIME: u64 = 60 * 60 * 24 * 7;

lazy_static! {
    /// Mapping for rating values.
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

/// Fetch metadata from MyAnimeList API given a (possibly comma‐separated) list of MAL IDs,
/// a flag indicating whether the media is a movie, and a JSON object from AniDB for comparison.
/// 
/// Returns a tuple containing the JSON metadata of the best-matching entry and the chosen main MAL id.
pub async fn get_metadata(anime_id: &str, movie: bool, dict_anidb: &Value) -> Result<(Value, String)> {
    info!(
        "MyAnimeList: Fetching metadata for anime_id: {} (movie: {})",
        anime_id, movie
    );

    // Split the provided anime_id string by commas.
    let mal_ids: Vec<&str> = anime_id
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if mal_ids.is_empty() {
        return Err(anyhow!("No MAL IDs provided"));
    }

    // Retrieve MAL API client id from environment variable (simulate Prefs).
    let mal_api_client_id = std::env::var("MAL_API_CLIENT_ID").unwrap_or_default();
    if mal_api_client_id.is_empty() || mal_api_client_id == "None" || mal_api_client_id == "N/A" {
        info!("No API key found - MAL_API_CLIENT_ID is not set");
        // If dict_anidb has seasons, choose the first MAL id from the first season (simulation).
        if let Some(seasons) = dict_anidb.get("seasons").and_then(|v| v.as_object()) {
            if !seasons.is_empty() {
                let main_mal_id = mal_ids[0].to_string();
                return Ok((json!({}), main_mal_id));
            }
        }
        return Ok((json!({}), "".to_string()));
    }

    let client = Client::new();
    let mut best_score: i32 = -1;
    let mut best_match = json!({});
    let mut main_mal_id = String::new();

    // Iterate over each MAL id, fetch details, and compute a matching score.
    for &id in &mal_ids {
        let url = MYANIMELIST_URL_DETAILS_TEMPLATE.replace("{id}", id);
        info!("Fetching details for MAL ID: {} from URL: {}", id, url);
        // Sleep 2 seconds between requests to avoid rate limits.
        sleep(Duration::from_secs(2)).await;
        let response = client
            .get(&url)
            .header("X-MAL-CLIENT-ID", &mal_api_client_id)
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .with_context(|| format!("Failed to fetch MAL details for id {}", id))?;

        if !response.status().is_success() {
            error!(
                "Failed to fetch MAL details for id {}: HTTP {}",
                id,
                response.status()
            );
            continue;
        }

        let json_resp: Value = response
            .json()
            .await
            .with_context(|| format!("Failed to parse MAL JSON response for id {}", id))?;

        let mut current_score = 0;
        // Compare the title field.
        if let Some(mal_title) = json_resp.get("title").and_then(|v| v.as_str()) {
            if let Some(orig_title) = dict_anidb.get("original_title").and_then(|v| v.as_str()) {
                if mal_title == orig_title {
                    current_score += 2;
                } else if mal_title.contains(orig_title) {
                    current_score += 1;
                }
            }
        }
        // Compare the start date.
        if let Some(mal_start_date) = json_resp.get("start_date").and_then(|v| v.as_str()) {
            if let Some(anidb_date) = dict_anidb.get("originally_available_at").and_then(|v| v.as_str())
            {
                if mal_start_date.contains(anidb_date) {
                    current_score += 1;
                }
            }
        }
        // (Additional fields such as rating, genres, etc. could contribute to the score here.)

        debug!("MAL ID: {} has score: {}", id, current_score);
        if current_score > best_score {
            best_score = current_score;
            best_match = json_resp.clone();
            main_mal_id = id.to_string();
        }
    }

    info!(
        "Selected MAL metadata (score {}): {}",
        best_score,
        serde_json::to_string_pretty(&best_match)?
    );
    Ok((best_match, main_mal_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata() {
        // Simulate an AniDB metadata JSON.
        let dict_anidb = json!({
            "original_title": "Test Anime",
            "originally_available_at": "2020-01-01",
            "seasons": {
                "1": []
            }
        });
        // Set a dummy MAL API client id.
        std::env::set_var("MAL_API_CLIENT_ID", "dummy_api_key");
        
        // Call get_metadata with a dummy MAL id list.
        let result = get_metadata("12345,67890", false, &dict_anidb).await;
        match result {
            Ok((metadata, main_id)) => {
                println!("Metadata: {}", serde_json::to_string_pretty(&metadata).unwrap());
                println!("Main MAL ID: {}", main_id);
            },
            Err(e) => panic!("Error: {:?}", e),
        }
    }
}
