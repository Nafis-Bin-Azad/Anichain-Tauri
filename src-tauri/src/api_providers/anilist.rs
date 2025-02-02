// src/api_providers/anilist.rs

use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::{json, Value};
use log::{info, debug, error};
use lazy_static::lazy_static;
use std::collections::BTreeMap;
use tokio::time::{sleep, Duration};

//
// Constants and GraphQL Document
//
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

//
// Helper functions
//

/// Performs a GraphQL query against AniList.
/// Logs the query and variables, sends a POST request with a JSON payload, and returns
/// the `"data"` field from the JSON response if no errors occur.
async fn make_graphql_query(document: &str, variables: &Value) -> Result<Value> {
    info!("GraphQL Query:\n{}", document);
    info!("Variables:\n{}", variables);
    let client = Client::new();
    let payload = json!({
        "query": document,
        "variables": variables
    });
    let response = client
        .post(GRAPHQL_API_URL)
        .json(&payload)
        .send()
        .await
        .context("Failed to send GraphQL query")?;
        
    let response_json = response
        .json::<Value>()
        .await
        .context("Failed to parse GraphQL response as JSON")?;
        
    if let Some(errors) = response_json.get("errors") {
        if errors.as_array().map(|arr| !arr.is_empty()).unwrap_or(false) {
            error!("GraphQL error: {:?}", errors.get(0));
            return Err(anyhow!("GraphQL query error: {:?}", errors.get(0)));
        }
    }
        
    response_json.get("data")
        .cloned()
        .ok_or_else(|| anyhow!("No 'data' field in GraphQL response"))
}

/// Maps an AniDB ID to an AniList ID by querying an external ARM service.
/// If successful, returns Some(anilist_id); otherwise returns None.
async fn get_alist_id(anidbid: &str) -> Result<Option<String>> {
    let url = ARM_SERVER_URL.replace("{id}", anidbid);
    let client = Client::new();
    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send ARM request")?;
    let response_json = response
        .json::<Value>()
        .await
        .context("Failed to parse ARM response")?;
    // Expect the JSON to have an "anilist" field.
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

/// Helper function: Retrieve a nested value from a JSON object given a list of keys.
fn get_nested(value: &Value, keys: &[&str]) -> Option<Value> {
    let mut current = value;
    for key in keys {
        current = current.get(*key)?;
    }
    Some(current.clone())
}

/// Helper function: Sets a nested value in a JSON object. Creates intermediate objects as needed.
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

/// Dummy poster_rank function: returns a ranking value for images.
/// In a full implementation, this might compute a rank based on language and quality.
fn poster_rank(service: &str, typ: &str) -> i32 {
    debug!("poster_rank({}, {}) called", service, typ);
    0 // For demonstration, always return 0.
}

//
// AniList API: Get Metadata
//

/// Fetch metadata from AniList API using GraphQL. This function accepts an optional AniDB ID
/// and an optional MyAnimeList ID. It first attempts to retrieve an AniList ID from the ARM service
/// (using the AniDB ID) and then builds GraphQL variables accordingly. It then sends the query,
/// extracts poster and banner URLs, and returns a JSON object with these details.
///
/// # Arguments
/// - `anidbid`: Optional AniDB ID as &str.
/// - `malid`: Optional MyAnimeList ID as &str.
///
/// # Returns
/// A JSON object containing AniList metadata (e.g. poster and banner information).
pub async fn get_metadata(anidbid: Option<&str>, malid: Option<&str>) -> Result<Value> {
    info!("{}", "=".repeat(157));
    info!("=== AniList.GetMetadata() ===");
    let mut anilist_dict = json!({});

    // Try to obtain an AniList ID from AniDB ID, if provided.
    let alid: Option<String> = if let Some(id) = anidbid {
        get_alist_id(id).await?
    } else {
        None
    };

    info!("AniDBid={:?}, MALid={:?}, ALid={:?}", anidbid, malid, alid);

    // Validate the MAL id: it must be entirely numeric.
    let malid_valid = if let Some(malid) = malid {
        malid.chars().all(|c| c.is_ascii_digit())
    } else {
        false
    };

    // If we don't have an AniList id and the MAL id is not valid, return an empty dict.
    if alid.is_none() && (!malid_valid) {
        return Ok(anilist_dict);
    }

    info!("{}", "-".repeat(157));
    info!("--- series ---");

    // Build variables for the GraphQL query.
    let mut variables = json!({});
    if let Some(alid_val) = alid {
        // AniList API expects an integer ID.
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
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            info!("[ ] poster: {}", poster_url);
            let poster_path = format!("AniList/poster/{}", poster_url.split('/').last().unwrap_or("unknown.jpg"));
            let rank = poster_rank("AniList", "posters");
            set_nested(&mut anilist_dict, &["posters", &poster_url], json!((poster_path, rank, Value::Null)));
        }
        if let Some(banner_url) = get_nested(&data, &["anime", "bannerImage"])
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            info!("[ ] banner: {}", banner_url);
            let banner_path = format!("AniList/banners/{}", banner_url.split('/').last().unwrap_or("unknown.jpg"));
            let rank = poster_rank("AniList", "banners");
            set_nested(&mut anilist_dict, &["banners", &banner_url], json!((banner_path, rank, Value::Null)));
        }
    }

    info!("{}", "-".repeat(157));
    info!("AniList_dict: {:#?}", anilist_dict);
    Ok(anilist_dict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata() -> Result<()> {
        // For testing, we supply dummy AniDB and MAL IDs.
        let anidbid = Some("12345");
        let malid = Some("67890");
        let result = get_metadata(anidbid, malid).await?;
        println!("AniList Metadata: {}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
}
