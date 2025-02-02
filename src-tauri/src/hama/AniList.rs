use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use log::{info, debug, error};
use lazy_static::lazy_static;

// Constants for the AniList module
const ARM_SERVER_URL: &str = "https://arm.haglund.dev/api/v2/ids?source=anidb&include=anilist&id={id}";
const GRAPHQL_API_URL: &str = "https://graphql.anilist.co";
const ANIME_DATA_DOCUMENT: &str = r#"
query($id: Int, $malId: Int) {
  anime: Media(type: ANIME, id: $id, idMal: $malId) {
    coverImage {
      url: extraLarge
    }
    bannerImage
  }
}
"#;

/// Performs a GraphQL query against AniList.
/// 
/// Logs the query and variables, sends a POST request with a JSON payload, and returns
/// the `"data"` field from the JSON response if no errors occur.
async fn make_graphql_query(document: &str, variables: &Value) -> Result<Value> {
    info!("Query: {}", document);
    info!("Variables: {}", variables);
    let client = Client::new();
    let payload = json!({
        "query": document,
        "variables": variables
    });
    let response = client
        .post(GRAPHQL_API_URL)
        .json(&payload)
        .send()
        .await?
        .json::<Value>()
        .await?;
        
    if let Some(errors) = response.get("errors") {
        if errors.as_array().map(|arr| !arr.is_empty()).unwrap_or(false) {
            error!("Got error: {:?}", errors.get(0));
            return Err(anyhow!("GraphQL query error: {:?}", errors.get(0)));
        }
    }
    
    response.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("No 'data' field in GraphQL response"))
}

/// Maps an AniDB ID to an AniList ID by querying an external ARM service.
/// 
/// If successful, returns Some(anilist_id); otherwise returns None.
async fn get_alist_id(anidbid: &str) -> Result<Option<String>> {
    let url = ARM_SERVER_URL.replace("{id}", anidbid);
    let client = Client::new();
    let response = client.get(&url).send().await?;
    let response_json = response.json::<Value>().await?;
    // Expect the JSON to have an "anilist" field
    if let Some(alid) = response_json.get("anilist") {
        if alid.is_string() {
            Ok(Some(alid.as_str().unwrap().to_string()))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Helper: Retrieve a nested value from a JSON object given a list of keys.
fn get_nested<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current)
}

/// Helper: Sets a nested value in a JSON object. Creates intermediate objects as needed.
/// This mimics the behavior of your Python SaveDict.
fn set_nested(value: &mut Value, keys: &[&str], new_value: Value) {
    if keys.is_empty() {
        *value = new_value;
        return;
    }
    let mut current = value;
    for key in &keys[..keys.len() - 1] {
        if current.get(key).is_none() {
            current[key] = json!({});
        }
        current = current.get_mut(key).unwrap();
    }
    current[keys[keys.len() - 1]] = new_value;
}

/// Implements AniList.GetMetadata() as described in your Python code.
///
/// It first attempts to map an AniDB ID to an AniList ID via `get_alist_id`. If neither an
/// AniList ID nor a valid MAL ID (MyAnimeList) is available, it returns an empty JSON object.
/// Otherwise, it builds GraphQL variables, sends the query, and, if successful, extracts
/// poster and banner image URLs, storing them in the returned JSON object.
pub async fn get_metadata(anidbid: Option<&str>, malid: Option<&str>) -> Result<Value> {
    info!("{}", "=".repeat(157));
    info!("=== AniList.GetMetadata() ===");
    let mut anilist_dict = json!({});

    // Try to obtain an AniList id from AniDB id.
    let alid: Option<String> = if let Some(id) = anidbid {
        get_alist_id(id).await?
    } else {
        None
    };

    info!("AniDBid={:?}, MALid={:?}, ALid={:?}", anidbid, malid, alid);
    
    // If we don't have an AniList id and the MAL id is not a digit, return an empty dict.
    let malid_valid = if let Some(malid) = malid {
        malid.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    };
    if alid.is_none() && (!malid_valid) {
        return Ok(anilist_dict);
    }
    
    info!("{}", "-".repeat(157));
    info!("--- series ---");
    
    // Build variables for the GraphQL query.
    let mut variables = json!({});
    if let Some(alid_val) = alid {
        // AniList API expects an integer. Try to parse the AL id.
        if let Ok(num) = alid_val.parse::<i32>() {
            set_nested(&mut variables, &["id"], json!(num));
        } else if let Some(malid_val) = malid {
            if let Ok(num) = malid_val.parse::<i32>() {
                set_nested(&mut variables, &["malId"], json!(num));
            }
        }
    } else if let Some(malid_val) = malid {
        if let Ok(num) = malid_val.parse::<i32>() {
            set_nested(&mut variables, &["malId"], json!(num));
        }
    }
    
    // Fetch data from AniList using the GraphQL API.
    let data = make_graphql_query(ANIME_DATA_DOCUMENT, &variables).await?;
    
    if !data.is_null() {
        info!("{}", "-".repeat(157));
        info!("--- images ---");
        if let Some(poster_url) = get_nested(&data, &["anime", "coverImage", "url"])
            .and_then(|v| v.as_str()) {
            info!("[ ] poster: {}", poster_url);
            // For demonstration, we save a tuple (path, rank, null). Replace poster_rank logic as needed.
            let poster_path = format!("AniList/poster/{}", poster_url.split('/').last().unwrap_or(""));
            let poster_rank = 0; // Dummy value
            set_nested(&mut anilist_dict, &["posters", poster_url], json!((poster_path, poster_rank, null)));
        }
        if let Some(banner_url) = get_nested(&data, &["anime", "bannerImage"])
            .and_then(|v| v.as_str()) {
            info!("[ ] banner: {}", banner_url);
            let banner_path = format!("AniList/banners/{}", banner_url.split('/').last().unwrap_or(""));
            let banner_rank = 0; // Dummy value
            set_nested(&mut anilist_dict, &["banners", banner_url], json!((banner_path, banner_rank, null)));
        }
    }
    
    info!("{}", "-".repeat(157));
    info!("AniList_dict: {:#?}", anilist_dict);
    Ok(anilist_dict)
}
