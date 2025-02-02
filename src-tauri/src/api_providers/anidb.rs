// src/api_providers/anidb.rs

use anyhow::{Result, Context};
use reqwest::Client;
use serde_json::{json, Value};
use log::{info, error, debug};
use quick_xml::Reader;
use quick_xml::events::Event;

/// AniDB API endpoint for fetching anime data.
/// For example, for anime id "6481", the complete URL is:
/// http://api.anidb.net:9001/httpapi?request=anime&client=hama&clientver=1&protover=1&aid=6481
const ANIDB_HTTP_API_URL: &str = "http://api.anidb.net:9001/httpapi?request=anime&client=hama&clientver=1&protover=1&aid=";

/// Fetch metadata from the AniDB API given an anime_id.
/// If `movie` is true, this metadata is for a movie; otherwise for a series.
///
/// This function sends an HTTP GET request to AniDB, then parses the returned XML
/// to extract a few key fields. (For a production system you might want to expand this
/// to include more fields and more robust error handling.)
pub async fn get_metadata(anime_id: &str, movie: bool) -> Result<Value> {
    info!("AniDB: Fetching metadata for anime_id: {} (movie: {})", anime_id, movie);
    
    // Build the URL.
    let url = format!("{}{}", ANIDB_HTTP_API_URL, anime_id);
    let client = Client::new();
    
    // Send the request.
    let response = client.get(&url)
        .send()
        .await
        .context("Failed to send request to AniDB")?;
    
    let text = response.text().await.context("Failed to read AniDB response text")?;
    
    // Check if the response indicates a ban.
    if text.to_lowercase().contains("banned") {
        info!("AniDB response indicates a ban for anime_id: {}", anime_id);
        return Ok(json!({"Banned": true}));
    }
    
    // Parse the XML response.
    // (For simplicity, we re-create a reader for each element extraction.)
    let title = extract_first(&text, "title").unwrap_or_else(|| "".to_string());
    let startdate = extract_first(&text, "startdate").unwrap_or_else(|| "".to_string());
    let rating = extract_first(&text, "ratings/permanent").unwrap_or_else(|| "".to_string());
    let summary = extract_first(&text, "description").unwrap_or_else(|| "".to_string());
    let picture = extract_first(&text, "picture").unwrap_or_else(|| "".to_string());
    
    // For AniDB the picture URL is built from a base URL.
    let poster_url = if picture.is_empty() {
        Value::Null
    } else {
        // In this example we assume the picture element contains just a filename.
        // In production, you might adjust this to suit the actual data.
        json!(format!("https://cdn.anidb.net/images/main/{}", picture))
    };
    
    // Build a JSON object with the extracted metadata.
    let result = json!({
        "id": anime_id,
        "title": title.trim(),
        "startdate": startdate.trim(),
        "rating": rating.trim(),
        "summary": summary.trim(),
        "picture": poster_url,
        "movie": movie,
    });
    
    Ok(result)
}

/// Helper function that extracts the text content of the first occurrence of a given XML element.
/// The `element` parameter may be a nested path separated by slashes (e.g. "ratings/permanent").
/// This function uses quick_xml’s Reader to iterate over events and returns the text when found.
fn extract_first(xml: &str, element: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.trim_text(true);
    let mut buf = Vec::new();

    // Split the target path by '/'
    let parts: Vec<&str> = element.split('/').collect();
    let mut current_depth = 0;
    let mut found = false;
    let mut result = String::new();

    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = std::str::from_utf8(e.name()).unwrap_or("");
                if current_depth < parts.len() && tag.eq_ignore_ascii_case(parts[current_depth]) {
                    current_depth += 1;
                    if current_depth == parts.len() {
                        // We have matched the entire nested path.
                        found = true;
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if found {
                    result.push_str(&e.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = std::str::from_utf8(e.name()).unwrap_or("");
                if found && current_depth == parts.len() && tag.eq_ignore_ascii_case(parts[current_depth - 1]) {
                    // End tag of our target element.
                    return Some(result);
                }
                if current_depth > 0 && tag.eq_ignore_ascii_case(parts[current_depth - 1]) {
                    current_depth -= 1;
                    if found && current_depth < parts.len() {
                        return Some(result);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                error!("Error reading AniDB XML: {:?}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio;

    #[tokio::test]
    async fn test_get_metadata() -> Result<()> {
        // For testing purposes, use a known anime id (replace "6481" with a valid id if needed)
        let anime_id = "6481";
        let movie = false;
        let result = get_metadata(anime_id, movie).await?;
        println!("AniDB Metadata: {}", serde_json::to_string_pretty(&result)?);
        Ok(())
    }
}
