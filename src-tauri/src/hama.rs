use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs;
use tracing;
use reqwest;
use urlencoding;
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;

// Constants from HAMA
const ANIDB_API_DOMAIN: &str = "http://api.anidb.net:9001";
const ANIDB_HTTP_API_URL: &str = "http://api.anidb.net:9001/httpapi?request=anime&client=hama&clientver=1&protover=1&aid=";
const ANIDB_IMAGE_DOMAIN: &str = "https://cdn.anidb.net";
const ANIDB_PIC_BASE_URL: &str = "https://cdn.anidb.net/images/main/";
const ANILIST_API_URL: &str = "https://graphql.anilist.co";
const MAL_API_URL: &str = "https://api.myanimelist.net/v2";

lazy_static! {
    static ref SUBSPLEASE_PATTERN: Regex = Regex::new(r"^\[(?:SubsPlease|Erai-raws|Judas|EMBER|PuyaSubs!|HorribleSubs)\] (.*?) - (\d{2,3})").unwrap();
    static ref BLURAY_PATTERN: Regex = Regex::new(r"^(.*?)(?:\.S(\d{1,2}))?\s*(?:E|Episode\s*)(\d{1,3})").unwrap();
    static ref NUMERIC_PATTERN: Regex = Regex::new(r"^(\d{2,3})(?:\v|\.|\s|$)").unwrap();
    static ref EPISODE_PATTERN: Regex = Regex::new(r"(?i)(?:E|Episode|第)\s*(\d{1,3})").unwrap();
    static ref IMAGE_CACHE: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

// HAMA API types
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeEpisode {
    pub number: i32,
    pub file_name: String,
    pub path: String,
    pub is_special: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HamaMetadata {
    pub title: String,
    pub season_count: i32,
    pub episode_count: i32,
    pub special_count: i32,
    pub year: Option<i32>,
    pub studio: Option<String>,
    pub genres: Vec<String>,
    pub summary: Option<String>,
    pub rating: Option<f32>,
    pub image_url: Option<String>,
    pub episodes: Vec<AnimeEpisode>,
    pub specials: Vec<AnimeEpisode>,
}

// HAMA API client
pub struct HamaClient {
    client: reqwest::Client,
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl HamaClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Scan folder and extract metadata
    pub async fn scan_folder(&self, folder_path: &str) -> Result<Vec<HamaMetadata>, String> {
        let mut results = Vec::new();
        let path = Path::new(folder_path);
        
        if !path.exists() {
            return Err("Folder does not exist".to_string());
        }

        tracing::info!("Scanning folder: {}", folder_path);

        // Map to store series and their episodes
        let mut series_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        // Scan all entries in the root directory
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            
            tracing::info!("Found entry: {}", path.display());
            
            if path.is_dir() {
                let folder_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Skip the Specials folder at root level
                if folder_name.to_lowercase() == "specials" {
                    continue;
                }

                // Extract series name from folder
                let series_name = self.extract_series_name(&path)?;
                tracing::info!("Adding folder for series: {}", series_name);
                series_map.entry(series_name)
                    .or_insert_with(Vec::new)
                    .push(path);

            } else if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Check if it's a video file
                if file_name.ends_with(".mkv") || file_name.ends_with(".mp4") {
                    // Extract series name from filename
                    if let Some(series_name) = self.extract_series_name_from_file(&file_name) {
                        tracing::info!("Found video file for series: {} - {}", series_name, file_name);
                        series_map.entry(series_name)
                            .or_insert_with(Vec::new)
                            .push(path);
                    }
                }
            }
        }

        tracing::info!("Found {} unique series", series_map.len());

        // Process each series
        for (series_name, paths) in series_map {
            let mut episodes = Vec::new();
            let mut specials = Vec::new();
            let mut episode_count = 0;
            let mut special_count = 0;

            // Process all paths for this series
            for path in &paths {
                if path.is_dir() {
                    // Scan directory for episodes
                    let main_episodes = self.scan_directory(path, false)?;
                    episodes.extend(main_episodes.0);
                    episode_count += main_episodes.1;

                    // Check for Specials subfolder
                    let specials_path = path.join("Specials");
                    if specials_path.exists() && specials_path.is_dir() {
                        let special_episodes = self.scan_directory(&specials_path, true)?;
                        specials.extend(special_episodes.0);
                        special_count += special_episodes.1;
                    }
                } else {
                    // Handle single file
                    let file_name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    let is_special = is_special_episode(&file_name);
                    let episode_number = extract_episode_number(&file_name);

                    let episode = AnimeEpisode {
                        number: episode_number,
                        file_name: file_name.clone(),
                        path: path.to_string_lossy().to_string(),
                        is_special,
                    };

                    if is_special {
                        specials.push(episode);
                        special_count += 1;
                    } else {
                        episodes.push(episode);
                        episode_count += 1;
                    }
                }
            }

            // Create metadata object
            let mut metadata = HamaMetadata {
                title: series_name.clone(),
                season_count: 1,
                episode_count,
                special_count,
                year: None,
                studio: None,
                genres: Vec::new(),
                summary: None,
                rating: None,
                image_url: None,
                episodes,
                specials,
            };

            // Try to fetch additional metadata and image
            if let Ok(Some(additional_data)) = self.fetch_metadata(&series_name).await {
                metadata.image_url = additional_data.image_url;
                metadata.summary = additional_data.summary;
                metadata.rating = additional_data.rating;
                metadata.genres = additional_data.genres;
                metadata.year = additional_data.year;
                metadata.studio = additional_data.studio;
            }

            tracing::info!("Found {} episodes and {} specials for {}", 
                metadata.episode_count, 
                metadata.special_count,
                metadata.title
            );

            results.push(metadata);
        }

        tracing::info!("Total anime series found: {}", results.len());
        Ok(results)
    }

    // Helper function to scan a directory for episodes
    fn scan_directory(&self, path: &Path, is_specials: bool) -> Result<(Vec<AnimeEpisode>, i32), String> {
        let mut episodes = Vec::new();
        let mut count = 0;

        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_name = entry.file_name().to_string_lossy().to_string();
            
            // Skip non-video files
            if !file_name.ends_with(".mkv") && !file_name.ends_with(".mp4") {
                continue;
            }

            // Determine if this is a special episode
            let is_special = is_specials || is_special_episode(&file_name);
            
            // Extract episode number
            let episode_number = extract_episode_number(&file_name);
            
            // Create episode object
            let episode = AnimeEpisode {
                number: episode_number,
                file_name: file_name.clone(),
                path: entry.path().to_string_lossy().to_string(),
                is_special,
            };

            episodes.push(episode);
            count += 1;
        }

        Ok((episodes, count))
    }

    // Extract series name from path or files
    fn extract_series_name(&self, path: &Path) -> Result<String, String> {
        let folder_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "Invalid folder name".to_string())?;

        // If folder name is numeric, try to extract from files
        if folder_name.chars().all(|c| c.is_numeric()) {
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                
                // Try SubsPlease pattern first
                if let Some(caps) = SUBSPLEASE_PATTERN.captures(&file_name) {
                    return Ok(clean_title_for_search(caps.get(1).unwrap().as_str()));
                }
                
                // Try Bluray pattern
                if let Some(caps) = BLURAY_PATTERN.captures(&file_name) {
                    return Ok(clean_title_for_search(caps.get(1).unwrap().as_str()));
                }
            }
            
            // If no pattern matches, use the first video file's name
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.ends_with(".mkv") || file_name.ends_with(".mp4") {
                    let clean_name = file_name
                        .split(" - ")
                        .next()
                        .unwrap_or(&file_name)
                        .to_string();
                    return Ok(clean_title_for_search(&clean_name));
                }
            }
        }

        Ok(clean_title_for_search(folder_name))
    }

    // Fetch metadata from various sources
    async fn fetch_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        // Try AniList first
        if let Ok(Some(metadata)) = self.fetch_anilist_metadata(title).await {
            return Ok(Some(metadata));
        }

        // Fallback to MyAnimeList
        if let Ok(Some(metadata)) = self.fetch_mal_metadata(title).await {
            return Ok(Some(metadata));
        }

        // Final fallback to AniDB
        self.fetch_anidb_metadata(title).await
    }

    // Fetch metadata from AniList
    async fn fetch_anilist_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        tracing::info!("Fetching metadata from AniList for: {}", title);
        let query = format!(
            r#"
            query ($search: String) {{
                Media (search: $search, type: ANIME) {{
                    title {{ romaji english native }}
                    description
                    coverImage {{ large }}
                    averageScore
                    genres
                    studios {{ nodes {{ name }} }}
                    startDate {{ year }}
                    episodes
                }}
            }}
            "#
        );

        let json = serde_json::json!({
            "query": query,
            "variables": {
                "search": title
            }
        });

        // Add delay to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let response = self.client.post(ANILIST_API_URL)
            .json(&json)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !response.status().is_success() {
            tracing::warn!("Failed to fetch from AniList: {}", response.status());
            return Ok(None);
        }

        let data: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        let media = data.get("data").and_then(|d| d.get("Media"));

        if let Some(media) = media {
            let image_url = media.get("coverImage")
                .and_then(|c| c.get("large"))
                .and_then(|u| u.as_str())
                .map(String::from)
                .unwrap_or_else(|| "https://placehold.co/225x319/gray/white/png?text=No+Image".to_string());

            // Cache the image URL
            let mut cache = self.cache.lock();
            cache.insert(title.to_string(), image_url.clone());
            save_image_cache(&cache);

            tracing::info!("Found metadata for '{}': {}", title, image_url);

            Ok(Some(HamaMetadata {
                title: title.to_string(),
                season_count: 1,
                episode_count: 0,
                special_count: 0,
                year: media.get("startDate").and_then(|d| d.get("year")).and_then(|y| y.as_i64()).map(|y| y as i32),
                studio: media.get("studios").and_then(|s| s.get("nodes")).and_then(|n| n.get(0)).and_then(|n| n.get("name")).and_then(|n| n.as_str()).map(String::from),
                genres: media.get("genres").and_then(|g| g.as_array()).map_or_else(Vec::new, |g| g.iter().filter_map(|v| v.as_str().map(String::from)).collect()),
                summary: media.get("description").and_then(|d| d.as_str()).map(String::from),
                rating: media.get("averageScore").and_then(|s| s.as_f64()).map(|s| (s / 10.0) as f32),
                image_url: Some(image_url),
                episodes: Vec::new(),
                specials: Vec::new(),
            }))
        } else {
            tracing::warn!("No metadata found for: {}", title);
            Ok(Some(HamaMetadata {
                title: title.to_string(),
                season_count: 1,
                episode_count: 0,
                special_count: 0,
                year: None,
                studio: None,
                genres: Vec::new(),
                summary: None,
                rating: None,
                image_url: Some("https://placehold.co/225x319/gray/white/png?text=No+Image".to_string()),
                episodes: Vec::new(),
                specials: Vec::new(),
            }))
        }
    }

    // Fetch metadata from MyAnimeList
    async fn fetch_mal_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        // Note: MAL requires authentication, so we'll skip implementation for now
        // In a real implementation, you'd need to handle OAuth2 authentication
        Ok(None)
    }

    // Fetch metadata from AniDB
    async fn fetch_anidb_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        // Note: AniDB has strict rate limiting, so we'll implement a basic version
        let clean_title = clean_title_for_search(title);
        let cache = self.cache.lock();
        
        if let Some(cached_url) = cache.get(&clean_title) {
            return Ok(Some(HamaMetadata {
                title: title.to_string(),
                season_count: 1,
                episode_count: 0,
                special_count: 0,
                year: None,
                studio: None,
                genres: Vec::new(),
                summary: None,
                rating: None,
                image_url: Some(cached_url.clone()),
                episodes: Vec::new(),
                specials: Vec::new(),
            }));
        }

        // For now, return None as AniDB requires registration and has strict rate limiting
        Ok(None)
    }

    // Helper function to extract series name from a file name
    fn extract_series_name_from_file(&self, file_name: &str) -> Option<String> {
        // Try SubsPlease pattern first
        if let Some(caps) = SUBSPLEASE_PATTERN.captures(file_name) {
            return Some(clean_title_for_search(caps.get(1)?.as_str()));
        }
        
        // Try Bluray pattern
        if let Some(caps) = BLURAY_PATTERN.captures(file_name) {
            return Some(clean_title_for_search(caps.get(1)?.as_str()));
        }
        
        // Try splitting by " - " and take the first part
        let parts: Vec<&str> = file_name.split(" - ").collect();
        if !parts.is_empty() {
            return Some(clean_title_for_search(parts[0]));
        }
        
        None
    }
}

// Helper function to clean titles for searching
fn clean_title_for_search(title: &str) -> String {
    let clean = title
        .replace("[SubsPlease]", "")
        .replace("[Erai-raws]", "")
        .replace("[Judas]", "")
        .replace("[EMBER]", "")
        .replace("[PuyaSubs!]", "")
        .replace("[HorribleSubs]", "")
        .replace("1080p", "")
        .replace("720p", "")
        .replace("480p", "")
        .replace("Blu-Ray", "")
        .replace("BluRay", "")
        .replace("10-Bit", "")
        .replace("Dual-Audio", "")
        .replace("TrueHD", "")
        .replace("x265", "")
        .replace("x264", "")
        .replace("HEVC", "")
        .replace("AAC", "")
        .replace("iAHD", "")
        .replace("Black.Clover", "Black Clover")
        .replace(".", " ")
        .replace("_", " ")
        .replace("[", "")
        .replace("]", "")
        .replace("(", "")
        .replace(")", "")
        .replace("  ", " ") // Remove double spaces
        .trim()
        .to_string();

    // Split by " - " and take the first part if it exists
    let parts: Vec<&str> = clean.split(" - ").collect();
    let final_title = if !parts.is_empty() {
        parts[0].trim()
    } else {
        &clean
    }.to_string();

    tracing::info!("Cleaned title '{}' to '{}'", title, final_title);
    final_title
}

// Helper function to extract episode number
fn extract_episode_number(filename: &str) -> i32 {
    // Try SubsPlease pattern first
    if let Some(caps) = SUBSPLEASE_PATTERN.captures(filename) {
        if let Some(ep_str) = caps.get(2) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using SubsPlease pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try Bluray pattern
    if let Some(caps) = BLURAY_PATTERN.captures(filename) {
        if let Some(ep_str) = caps.get(3) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using Bluray pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try numeric pattern (e.g., "01.mkv")
    if let Some(caps) = NUMERIC_PATTERN.captures(filename) {
        if let Some(ep_str) = caps.get(1) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using numeric pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try episode pattern
    if let Some(caps) = EPISODE_PATTERN.captures(filename) {
        if let Some(ep_str) = caps.get(1) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using episode pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try to find any number after " - " or "E" or "Episode"
    let parts: Vec<&str> = filename.split(&['-', 'E', 'e']).collect();
    for part in parts.iter().skip(1) {
        let clean_part = part.trim().split_whitespace().next().unwrap_or("");
        if let Ok(num) = clean_part.parse::<i32>() {
            tracing::debug!("Found episode number {} using fallback pattern in {}", num, filename);
            return num;
        }
    }

    tracing::debug!("No episode number found in {}, defaulting to 1", filename);
    1 // Default to episode 1 if no number found
}

// Helper function to determine if an episode is special
fn is_special_episode(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.contains("special") || 
    lower.contains("sp") ||
    lower.contains("ova") ||
    lower.contains("ncop") ||
    lower.contains("nced") ||
    lower.contains("opening") ||
    lower.contains("ending") ||
    lower.contains("preview") ||
    lower.contains("recap")
}

#[tauri::command]
pub async fn fetch_anime_metadata(folder_path: String) -> Result<Vec<HamaMetadata>, String> {
    tracing::info!("Scanning anime folder...");
    let hama = HamaClient::new();
    let metadata = hama.scan_folder(&folder_path).await?;
    tracing::info!("Finished scanning {} anime series", metadata.len());
    Ok(metadata)
}

fn get_cache_dir() -> PathBuf {
    let mut cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".cache"));
    cache_dir.push("anichain");
    fs::create_dir_all(&cache_dir).unwrap_or_default();
    cache_dir
}

fn load_image_cache() -> HashMap<String, String> {
    let cache_file = get_cache_dir().join("image_cache.json");
    if let Ok(contents) = fs::read_to_string(cache_file) {
        serde_json::from_str(&contents).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_image_cache(cache: &HashMap<String, String>) {
    let cache_file = get_cache_dir().join("image_cache.json");
    if let Ok(json) = serde_json::to_string(cache) {
        fs::write(cache_file, json).unwrap_or_default();
    }
}

// Helper function to calculate string similarity (Levenshtein distance based)
fn string_similarity(s1: &str, s2: &str) -> f32 {
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
    
    if len1 == 0 || len2 == 0 {
        return 0.0;
    }
    
    let distance = levenshtein_distance(s1, s2);
    let max_len = len1.max(len2) as f32;
    
    1.0 - (distance as f32 / max_len)
}

// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let len1 = s1_chars.len();
    let len2 = s2_chars.len();
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if s1_chars[i - 1] == s2_chars[j - 1] { 0 } else { 1 };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }
    
    matrix[len1][len2]
} 