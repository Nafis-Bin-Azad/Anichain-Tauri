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

lazy_static! {
    static ref SUBSPLEASE_PATTERN: Regex = Regex::new(r"^\[SubsPlease\] (.*?) - (\d{2,3})").unwrap();
    static ref BLURAY_PATTERN: Regex = Regex::new(r"^(.*?)\.S(\d{2})E(\d{2})").unwrap();
    static ref NUMERIC_PATTERN: Regex = Regex::new(r"^(\d{2,3})\.mkv$").unwrap();
    static ref IMAGE_CACHE: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
}

const ANIDB_IMAGE_DOMAIN: &str = "https://cdn.anidb.net";
const ANIDB_PIC_BASE_URL: &str = "https://cdn.anidb.net/images/main/";

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

/// Core HAMA functionality interface
pub struct HamaInterface {}

impl HamaInterface {
    pub fn new() -> Self {
        Self {}
    }

    /// Scan a folder for anime episodes
    pub fn scan_folder(&self, folder_path: &str) -> Result<Vec<HamaMetadata>, String> {
        let mut metadata = Vec::new();
        let path = std::path::Path::new(folder_path);
        
        if !path.exists() {
            return Err(format!("Folder does not exist: {}", folder_path));
        }
        
        // Group files by series
        let mut series_map: std::collections::HashMap<String, Vec<(String, bool)>> = std::collections::HashMap::new();
        
        // Function to scan directory and its subdirectories
        fn scan_dir(dir: &Path, series_map: &mut HashMap<String, Vec<(String, bool)>>) -> Result<(), String> {
            for entry in std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))? {
                let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip if this is a "Specials" folder - we'll handle its contents under the parent anime
                    if path.file_name()
                        .and_then(|n| n.to_str())
                        .map_or(false, |name| name.to_lowercase() == "specials") {
                        continue;
                    }
                    // Recursively scan subdirectories
                    scan_dir(&path, series_map)?;
                    continue;
                }
                
                // Skip non-video files
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if !["mkv", "mp4", "avi"].contains(&ext.as_str()) {
                        continue;
                    }
                } else {
                    continue;
                }
                
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                // Try to extract series name from filename or parent folder
                let (series_name, is_special) = if let Some(caps) = SUBSPLEASE_PATTERN.captures(&file_name) {
                    let series_name = caps.get(1)
                        .map(|m| m.as_str().trim().to_string())
                        .unwrap_or_else(|| file_name.clone());
                    
                    let is_special = file_name.to_lowercase().contains("special") || 
                                   file_name.to_lowercase().contains("sp") || 
                                   file_name.to_lowercase().contains("ova");
                    
                    (series_name, is_special)
                } else if let Some(parent) = path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str()) {
                    // For files in subdirectories, use parent folder name
                    let is_special = parent.to_lowercase() == "specials" ||
                                   file_name.to_lowercase().contains("special") ||
                                   file_name.to_lowercase().contains("sp") ||
                                   file_name.to_lowercase().contains("ova");
                    
                    // Go up one more level if parent is numeric or "Specials"
                    let series_name = if parent.chars().all(|c| c.is_ascii_digit()) || parent.to_lowercase() == "specials" {
                        path.parent()
                            .and_then(|p| p.parent())
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| parent.to_string())
                    } else {
                        parent.to_string()
                    };
                    
                    (series_name, is_special)
                } else {
                    continue;
                };
                
                // Clean up series name
                let clean_series_name = series_name
                    .replace(".1080p.Blu-Ray.10-Bit.Dual-Audio.TrueHD.x265-iAHD", "")
                    .replace("Black.Clover", "Black Clover");
                
                series_map.entry(clean_series_name)
                    .or_default()
                    .push((path.to_string_lossy().to_string(), is_special));
            }
            Ok(())
        }
        
        // Scan the root directory and all subdirectories
        scan_dir(path, &mut series_map)?;
        
        // Convert grouped files into metadata
        for (series_name, files) in series_map {
            let mut episodes = Vec::new();
            let mut specials = Vec::new();
            
            for (file_path, is_special) in files {
                let path = std::path::Path::new(&file_path);
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                
                let episode = AnimeEpisode {
                    number: extract_episode_number(&file_name),
                    file_name,
                    path: file_path,
                    is_special,
                };
                
                if is_special {
                    specials.push(episode);
                } else {
                    episodes.push(episode);
                }
            }
            
            // Sort episodes by number
            episodes.sort_by_key(|e| e.number);
            specials.sort_by_key(|e| e.number);
            
            let anime_metadata = HamaMetadata {
                title: series_name,
                season_count: 1,
                episode_count: episodes.len() as i32,
                special_count: specials.len() as i32,
                year: None,
                studio: None,
                genres: Vec::new(),
                summary: None,
                rating: None,
                image_url: None,
                episodes,
                specials,
            };
            
            metadata.push(anime_metadata);
        }
        
        tracing::info!("Found {} anime series", metadata.len());
        for anime in &metadata {
            tracing::info!(
                "Found anime: {} with {} episodes and {} specials",
                anime.title,
                anime.episode_count,
                anime.special_count
            );
        }
        
        Ok(metadata)
    }
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

#[tauri::command]
pub async fn fetch_anime_metadata(folder_path: String) -> Result<Vec<HamaMetadata>, String> {
    tracing::info!("Scanning anime folder...");
    
    let hama = HamaInterface::new();
    let mut metadata = hama.scan_folder(&folder_path)?;
    
    // Create a reqwest client for fetching images
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    // Load image cache at the start
    {
        let mut image_cache = IMAGE_CACHE.lock();
        if image_cache.is_empty() {
            *image_cache = load_image_cache();
        }
    }
    
    // Fetch additional metadata for each anime
    for anime in &mut metadata {
        // Check cache first
        {
            let image_cache = IMAGE_CACHE.lock();
            if let Some(cached_url) = image_cache.get(&anime.title) {
                anime.image_url = Some(cached_url.clone());
                continue;
            }
        }
        
        // Clean up the title for better search results
        let clean_title = anime.title
            .replace("[SubsPlease]", "")
            .replace("Black.Clover", "Black Clover")
            .split(" - ")
            .next()
            .unwrap_or(&anime.title)
            .trim()
            .to_string();
            
        // Add delay to respect rate limits
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            
        // Try Jikan API first
        let query_url = format!(
            "https://api.jikan.moe/v4/anime?q={}&limit=5",
            urlencoding::encode(&clean_title)
        );
        
        tracing::info!("Fetching image for: {}", clean_title);
        
        match client.get(&query_url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<serde_json::Value>().await {
                        Ok(data) => {
                            if let Some(results) = data.get("data").and_then(|d| d.as_array()) {
                                for result in results.iter() {
                                    let result_title = result.get("title").and_then(|t| t.as_str())
                                        .or_else(|| result.get("title_english").and_then(|t| t.as_str()))
                                        .unwrap_or("");
                                        
                                    // Check if this is a good match
                                    if result_title.to_lowercase().contains(&clean_title.to_lowercase()) 
                                        || clean_title.to_lowercase().contains(&result_title.to_lowercase()) {
                                        // Extract image URL
                                        if let Some(image_url) = result
                                            .get("images")
                                            .and_then(|i| i.get("jpg"))
                                            .and_then(|j| j.get("large_image_url"))
                                            .and_then(|u| u.as_str())
                                        {
                                            // Verify image URL is accessible
                                            match client.head(image_url).send().await {
                                                Ok(img_response) if img_response.status().is_success() => {
                                                    tracing::info!("Found image for '{}': {}", clean_title, image_url);
                                                    anime.image_url = Some(image_url.to_string());
                                                    
                                                    // Cache the image URL
                                                    {
                                                        let mut image_cache = IMAGE_CACHE.lock();
                                                        image_cache.insert(anime.title.clone(), image_url.to_string());
                                                        save_image_cache(&image_cache);
                                                    }
                                                    
                                                    // Extract other metadata
                                                    if let Some(synopsis) = result.get("synopsis").and_then(|s| s.as_str()) {
                                                        anime.summary = Some(synopsis.to_string());
                                                    }
                                                    if let Some(score) = result.get("score").and_then(|s| s.as_f64()) {
                                                        anime.rating = Some(score as f32);
                                                    }
                                                    if let Some(genres) = result.get("genres").and_then(|g| g.as_array()) {
                                                        anime.genres = genres
                                                            .iter()
                                                            .filter_map(|g| g.get("name").and_then(|n| n.as_str()))
                                                            .map(|s| s.to_string())
                                                            .collect();
                                                    }
                                                    break;
                                                }
                                                _ => continue
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => tracing::error!("Failed to parse Jikan API response for '{}': {}", clean_title, e)
                    }
                }
            }
            Err(e) => tracing::error!("Failed to fetch from Jikan API for '{}': {}", clean_title, e)
        }
        
        // If no image was found, set a default image
        if anime.image_url.is_none() {
            let default_url = "https://placehold.co/225x319/gray/white/png?text=No+Image".to_string();
            anime.image_url = Some(default_url.clone());
            {
                let mut image_cache = IMAGE_CACHE.lock();
                image_cache.insert(anime.title.clone(), default_url);
                save_image_cache(&image_cache);
            }
        }
    }
    
    tracing::info!("Finished scanning {} anime series", metadata.len());
    Ok(metadata)
}

fn extract_episode_number(filename: &str) -> i32 {
    // Try SubsPlease pattern first
    if let Some(caps) = SUBSPLEASE_PATTERN.captures(filename) {
        if let Some(ep_str) = caps.get(2) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                return num;
            }
        }
    }

    // Try to find episode number in the filename
    if let Some(ep_str) = filename.split(" - ").nth(1) {
        // Extract the number
        if let Some(num_str) = ep_str.split_whitespace().next() {
            if let Ok(num) = num_str.parse::<i32>() {
                return num;
            }
        }
    }

    1 // Default episode number
} 