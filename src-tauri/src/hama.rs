use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use tracing;
use reqwest;
use std::sync::Arc;
use parking_lot::Mutex;
use std::collections::HashMap;
use urlencoding;
use flate2;
use roxmltree;

// Module organization
mod constants {
    pub const ANIDB_API_DOMAIN: &str = "http://api.anidb.net:9001";
    pub const ANIDB_HTTP_API_URL: &str = "http://api.anidb.net:9001/httpapi?request=anime&client=hama&clientver=1&protover=1&aid=";
    pub const ANIDB_IMAGE_DOMAIN: &str = "https://cdn.anidb.net";
    pub const ANIDB_PIC_BASE_URL: &str = "https://cdn.anidb.net/images/main/";
    pub const ANILIST_API_URL: &str = "https://graphql.anilist.co";
    pub const MAL_API_URL: &str = "https://api.myanimelist.net/v2";
}

mod patterns {
    use lazy_static::lazy_static;
    use regex::Regex;

    lazy_static! {
        pub static ref SUBSPLEASE: Regex = Regex::new(r"^\[(?:SubsPlease|Erai-raws|Judas|EMBER|PuyaSubs!|HorribleSubs)\] (.*?) - (\d{2,3})").unwrap();
        pub static ref BLURAY: Regex = Regex::new(r"^(.*?)(?:\.S(\d{1,2}))?\s*(?:E|Episode\s*)(\d{1,3})").unwrap();
        pub static ref NUMERIC: Regex = Regex::new(r"^(\d{2,3})(?:\v|\.|\s|$)").unwrap();
        pub static ref EPISODE: Regex = Regex::new(r"(?i)(?:E|Episode|第)\s*(\d{1,3})").unwrap();
    }
}

// Types
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
    pub original_title: Option<String>,
    pub season_count: i32,
    pub episode_count: i32,
    pub special_count: i32,
    pub year: Option<i32>,
    pub studio: Option<String>,
    pub genres: Vec<String>,
    pub summary: Option<String>,
    pub rating: Option<f32>,
    pub content_rating: Option<String>,
    pub image_url: Option<String>,
    pub banner_url: Option<String>,
    pub theme_url: Option<String>,
    pub originally_available_at: Option<String>,
    pub directors: Vec<String>,
    pub writers: Vec<String>,
    pub collections: Vec<String>,
    pub episodes: Vec<AnimeEpisode>,
    pub specials: Vec<AnimeEpisode>,
}

// Cache management
struct ImageCache {
    cache: Arc<Mutex<HashMap<String, String>>>,
}

impl ImageCache {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(load_image_cache())),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        self.cache.lock().get(key).cloned()
    }

    fn set(&self, key: String, value: String) {
        let mut cache = self.cache.lock();
        cache.insert(key, value);
        save_image_cache(&cache);
    }
}

// Main client implementation
pub struct HamaClient {
    http_client: reqwest::Client,
    image_cache: ImageCache,
}

impl HamaClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            image_cache: ImageCache::new(),
        }
    }

    pub async fn scan_folder(&self, folder_path: &str) -> Result<Vec<HamaMetadata>, String> {
        let path = Path::new(folder_path);
        if !path.exists() {
            return Err("Folder does not exist".to_string());
        }

        tracing::info!("Scanning folder: {}", folder_path);
        
        // First, find all video files recursively
        let mut video_files = Vec::new();
        self.collect_video_files(path, &mut video_files)?;
        tracing::info!("Found {} video files", video_files.len());

        // Group files by series based on filename patterns
        let mut series_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for file_path in video_files {
            if let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) {
                // Try to extract series name from filename
                if let Some(series_name) = self.extract_series_name_from_file(file_name) {
                    tracing::info!("Found video file for series '{}': {}", series_name, file_name);
                    series_map.entry(series_name)
                        .or_insert_with(Vec::new)
                        .push(file_path);
                } else {
                    tracing::warn!("Could not extract series name from: {}", file_name);
                }
            }
        }

        tracing::info!("Found {} potential series", series_map.len());
        for (series_name, paths) in &series_map {
            tracing::info!("Series '{}' has {} files:", series_name, paths.len());
            for path in paths {
                tracing::debug!("  - {:?}", path);
            }
        }

        let results = self.process_series_map(series_map).await?;
        tracing::info!("Successfully processed {} series", results.len());
        
        for result in &results {
            tracing::info!(
                "Processed series: {} - {} episodes, {} specials",
                result.title,
                result.episode_count,
                result.special_count
            );
        }

        Ok(results)
    }

    fn collect_video_files(&self, path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        if path.is_dir() {
            tracing::debug!("Scanning directory: {:?}", path);
            for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_dir() {
                    self.collect_video_files(&path, files)?;
                } else if path.is_file() {
                    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                        if is_video_file(file_name) {
                            tracing::info!("Found video file: {}", file_name);
                            files.push(path);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn process_series_map(&self, series_map: HashMap<String, Vec<PathBuf>>) -> Result<Vec<HamaMetadata>, String> {
        let mut results = Vec::new();

        for (series_name, paths) in series_map {
            tracing::info!("Processing series: {}", series_name);
            let (episodes, specials, episode_count, special_count) = self.collect_episodes(&paths)?;
            
            tracing::info!(
                "Collected episodes for {}: {} regular, {} special",
                series_name,
                episode_count,
                special_count
            );
            
            let mut metadata = self.create_base_metadata(
                &series_name,
                episodes,
                specials,
                episode_count,
                special_count
            );

            if let Ok(Some(additional_data)) = self.fetch_metadata(&series_name).await {
                tracing::info!("Found additional metadata for: {}", series_name);
                self.update_metadata(&mut metadata, additional_data);
            } else {
                tracing::warn!("No additional metadata found for: {}", series_name);
            }

            results.push(metadata);
        }

        Ok(results)
    }

    fn create_base_metadata(
        &self,
        title: &str,
        episodes: Vec<AnimeEpisode>,
        specials: Vec<AnimeEpisode>,
        episode_count: i32,
        special_count: i32
    ) -> HamaMetadata {
        HamaMetadata {
            title: title.to_string(),
            original_title: None,
            season_count: 1,
            episode_count,
            special_count,
            year: None,
            studio: None,
            genres: Vec::new(),
            summary: None,
            rating: None,
            content_rating: None,
            image_url: Some("https://placehold.co/225x319/gray/white/png?text=No+Image".to_string()),
            banner_url: None,
            theme_url: None,
            originally_available_at: None,
            directors: Vec::new(),
            writers: Vec::new(),
            collections: Vec::new(),
            episodes,
            specials,
        }
    }

    fn update_metadata(&self, base: &mut HamaMetadata, additional: HamaMetadata) {
        base.image_url = additional.image_url;
        base.summary = additional.summary;
        base.rating = additional.rating;
        base.genres = additional.genres;
        base.year = additional.year;
        base.studio = additional.studio;
    }

    fn process_directory(&self, path: &Path, series_map: &mut HashMap<String, Vec<PathBuf>>) -> Result<(), String> {
        let folder_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip the Specials folder at root level
        if folder_name.to_lowercase() == "specials" {
            return Ok(());
        }

        let series_name = self.extract_series_name(path)?;
        tracing::info!("Processing directory for series: {}", series_name);
        
        // Recursively scan this directory for video files
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                if let Some(file_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                    if is_video_file(file_name) {
                        tracing::info!("Found video file: {} in directory: {}", file_name, series_name);
                        series_map.entry(series_name.clone())
                            .or_insert_with(Vec::new)
                            .push(entry_path);
                    }
                }
            } else if entry_path.is_dir() {
                // Recursively process subdirectories
                tracing::info!("Found subdirectory: {:?} in series: {}", entry_path, series_name);
                for sub_entry in fs::read_dir(&entry_path).map_err(|e| e.to_string())? {
                    let sub_entry = sub_entry.map_err(|e| e.to_string())?;
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        if let Some(file_name) = sub_path.file_name().and_then(|n| n.to_str()) {
                            if is_video_file(file_name) {
                                tracing::info!("Found video file: {} in subdirectory of {}", file_name, series_name);
                                series_map.entry(series_name.clone())
                                    .or_insert_with(Vec::new)
                                    .push(sub_path);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    fn process_file(&self, path: &Path, series_map: &mut HashMap<String, Vec<PathBuf>>) -> Result<(), String> {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        if !is_video_file(&file_name) {
            return Ok(());
        }

        if let Some(series_name) = self.extract_series_name_from_file(&file_name) {
            tracing::info!("Found video file for series: {} - {}", series_name, file_name);
            series_map.entry(series_name)
                .or_insert_with(Vec::new)
                .push(path.to_path_buf());
        }

        Ok(())
    }

    fn collect_episodes(&self, paths: &[PathBuf]) -> Result<(Vec<AnimeEpisode>, Vec<AnimeEpisode>, i32, i32), String> {
        let mut episodes = Vec::new();
        let mut specials = Vec::new();
        let mut episode_count = 0;
        let mut special_count = 0;

        tracing::info!("Starting episode collection for {} paths", paths.len());

        for path in paths {
            if path.is_dir() {
                tracing::info!("Processing directory: {:?}", path);
                let entries = fs::read_dir(path).map_err(|e| e.to_string())?;
                for entry in entries {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let path = entry.path();
                    
                    if path.is_file() {
                        let file_name = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();

                        if !is_video_file(&file_name) {
                            tracing::debug!("Skipping non-video file: {}", file_name);
                            continue;
                        }

                        let (episode, is_special) = self.process_single_file(&path)?;
                        if is_special {
                            tracing::info!("Found special episode: {} (number: {})", file_name, episode.number);
                            specials.push(episode);
                            special_count += 1;
                        } else {
                            tracing::info!("Found regular episode: {} (number: {})", file_name, episode.number);
                            episodes.push(episode);
                            episode_count += 1;
                        }
                    }
                }
            } else if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if !is_video_file(&file_name) {
                    tracing::debug!("Skipping non-video file: {}", file_name);
                    continue;
                }

                let (episode, is_special) = self.process_single_file(path)?;
                if is_special {
                    tracing::info!("Found special episode: {} (number: {})", file_name, episode.number);
                    specials.push(episode);
                    special_count += 1;
                } else {
                    tracing::info!("Found regular episode: {} (number: {})", file_name, episode.number);
                    episodes.push(episode);
                    episode_count += 1;
                }
            }
        }

        // Sort episodes by number
        episodes.sort_by_key(|e| e.number);
        specials.sort_by_key(|e| e.number);

        tracing::info!("Episode collection complete:");
        tracing::info!("Regular episodes: {} (count: {})", episodes.len(), episode_count);
        tracing::info!("Special episodes: {} (count: {})", specials.len(), special_count);
        
        for episode in &episodes {
            tracing::debug!("Regular episode: {} (number: {})", episode.file_name, episode.number);
        }
        for special in &specials {
            tracing::debug!("Special episode: {} (number: {})", special.file_name, special.number);
        }

        Ok((episodes, specials, episode_count, special_count))
    }

    fn process_single_file(&self, path: &Path) -> Result<(AnimeEpisode, bool), String> {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let is_special = file_name.to_lowercase().contains("special") || 
                        file_name.to_lowercase().contains("ova") ||
                        path.parent().map(|p| p.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_lowercase() == "specials")
                        .unwrap_or(false);

        let number = if is_special {
            // Try to extract number from special episode
            if let Some(cap) = patterns::EPISODE.captures(&file_name) {
                cap[1].parse().unwrap_or(0)
            } else {
                0
            }
        } else {
            // Try different patterns for regular episodes
            if let Some(cap) = patterns::SUBSPLEASE.captures(&file_name) {
                cap[2].parse().unwrap_or(0)
            } else if let Some(cap) = patterns::BLURAY.captures(&file_name) {
                cap[3].parse().unwrap_or(0)
            } else if let Some(cap) = patterns::NUMERIC.captures(&file_name) {
                cap[1].parse().unwrap_or(0)
            } else if let Some(cap) = patterns::EPISODE.captures(&file_name) {
                cap[1].parse().unwrap_or(0)
            } else {
                0
            }
        };

        Ok((AnimeEpisode {
            number,
            file_name,
            path: path.to_string_lossy().to_string(),
            is_special,
        }, is_special))
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
                if let Some(caps) = patterns::SUBSPLEASE.captures(&file_name) {
                    return Ok(clean_title_for_search(caps.get(1).unwrap().as_str()));
                }
                
                // Try Bluray pattern
                if let Some(caps) = patterns::BLURAY.captures(&file_name) {
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

        let response = self.http_client.post(constants::ANILIST_API_URL)
            .json(&json)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
        
        if let Some(media) = json.get("data").and_then(|d| d.get("Media")) {
            let image_url = media.get("coverImage")
                .and_then(|c| c.get("large"))
                .and_then(|u| u.as_str())
                .unwrap_or("https://placehold.co/225x319/gray/white/png?text=No+Image")
                .to_string();

            Ok(Some(HamaMetadata {
                title: title.to_string(),
                original_title: None,
                season_count: 1,
                episode_count: media.get("episodes").and_then(|e| e.as_i64()).map(|e| e as i32).unwrap_or(0),
                special_count: 0,
                year: media.get("startDate").and_then(|d| d.get("year")).and_then(|y| y.as_i64()).map(|y| y as i32),
                studio: media.get("studios").and_then(|s| s.get("nodes")).and_then(|n| n.get(0)).and_then(|n| n.get("name")).and_then(|n| n.as_str()).map(String::from),
                genres: media.get("genres").and_then(|g| g.as_array()).map_or_else(Vec::new, |g| g.iter().filter_map(|v| v.as_str().map(String::from)).collect()),
                summary: media.get("description").and_then(|d| d.as_str()).map(String::from),
                rating: media.get("averageScore").and_then(|s| s.as_f64()).map(|s| (s / 10.0) as f32),
                content_rating: None,
                image_url: Some(image_url),
                banner_url: None,
                theme_url: None,
                originally_available_at: None,
                directors: Vec::new(),
                writers: Vec::new(),
                collections: Vec::new(),
                episodes: Vec::new(),
                specials: Vec::new(),
            }))
        } else {
            tracing::warn!("No metadata found for: {}", title);
            Ok(None)
        }
    }

    // Fetch metadata from MyAnimeList
    async fn fetch_mal_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        tracing::info!("Fetching metadata from MyAnimeList for: {}", title);
        
        // First search for the anime
        let search_url = format!("{}/anime?q={}&limit=1", constants::MAL_API_URL, urlencoding::encode(title));
        
        // Add delay to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let response = match self.http_client
            .get(&search_url)
            .header("X-MAL-CLIENT-ID", std::env::var("MAL_CLIENT_ID").unwrap_or_default())
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Failed to fetch from MAL: {}", e);
                    return Ok(None);
                }
            };

        let json: serde_json::Value = match response.json().await {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to parse MAL response: {}", e);
                return Ok(None);
            }
        };

        if let Some(data) = json.get("data").and_then(|d| d.as_array()).and_then(|a| a.first()) {
            let node = &data["node"];
            
            let id = node["id"].as_i64().unwrap_or_default();
            
            // Get detailed info
            let details_url = format!("{}/anime/{}?fields=id,title,start_date,synopsis,mean,genres,rating,studios,media_type,num_episodes,pictures", 
                constants::MAL_API_URL, 
                id
            );

            // Add delay
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

            let details = match self.http_client
                .get(&details_url)
                .header("X-MAL-CLIENT-ID", std::env::var("MAL_CLIENT_ID").unwrap_or_default())
                .send()
                .await {
                    Ok(resp) => resp.json::<serde_json::Value>().await.ok(),
                    Err(_) => None,
                };

            if let Some(details) = details {
                let image_url = details["main_picture"]["large"].as_str()
                    .unwrap_or("https://placehold.co/225x319/gray/white/png?text=No+Image")
                    .to_string();

                let genres = details["genres"].as_array()
                    .map_or_else(Vec::new, |g| {
                        g.iter()
                            .filter_map(|genre| genre["name"].as_str().map(String::from))
                            .collect()
                    });

                let studio = details["studios"].as_array()
                    .and_then(|s| s.first())
                    .and_then(|s| s["name"].as_str())
                    .map(String::from);

                return Ok(Some(HamaMetadata {
                    title: details["title"].as_str().unwrap_or(title).to_string(),
                    original_title: None,
                    season_count: 1,
                    episode_count: details["num_episodes"].as_i64().map(|e| e as i32).unwrap_or(0),
                    special_count: 0,
                    year: details["start_date"]
                        .as_str()
                        .and_then(|d| d.split('-').next())
                        .and_then(|y| y.parse().ok()),
                    studio,
                    genres,
                    summary: details["synopsis"].as_str().map(String::from),
                    rating: details["mean"].as_f64().map(|r| r as f32),
                    content_rating: details["rating"]
                        .as_str()
                        .map(|r| match r {
                            "g" => "G - All Ages",
                            "pg" => "PG - Children",
                            "pg_13" => "PG-13 - Teens 13 and Older",
                            "r" => "R - 17+ (violence & profanity)",
                            "r+" => "R+ - Profanity & Mild Nudity",
                            "rx" => "Rx - Hentai",
                            _ => r,
                        }.to_string()),
                    image_url: Some(image_url),
                    banner_url: None,
                    theme_url: None,
                    originally_available_at: details["start_date"].as_str().map(String::from),
                    directors: Vec::new(),
                    writers: Vec::new(),
                    collections: Vec::new(),
                    episodes: Vec::new(),
                    specials: Vec::new(),
                }));
            }
        }

        Ok(None)
    }

    // Fetch metadata from AniDB
    async fn fetch_anidb_metadata(&self, title: &str) -> Result<Option<HamaMetadata>, String> {
        tracing::info!("Fetching metadata from AniDB for: {}", title);
        let clean_title = clean_title_for_search(title);
        
        // Check cache first
        let cached_url = self.image_cache.get(&clean_title);
        if let Some(cached_url) = cached_url {
            tracing::info!("Found cached image for: {}", clean_title);
            return Ok(Some(HamaMetadata {
                title: title.to_string(),
                original_title: None,
                season_count: 1,
                episode_count: 0,
                special_count: 0,
                year: None,
                studio: None,
                genres: Vec::new(),
                summary: None,
                rating: None,
                content_rating: None,
                image_url: Some(cached_url),
                banner_url: None,
                theme_url: None,
                originally_available_at: None,
                directors: Vec::new(),
                writers: Vec::new(),
                collections: Vec::new(),
                episodes: Vec::new(),
                specials: Vec::new(),
            }));
        }

        // Add delay to respect rate limits
        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        // Search AniDB using their HTTP API
        let search_url = format!("{}/anime-titles.xml.gz", constants::ANIDB_API_DOMAIN);
        
        let response = match self.http_client
            .get(&search_url)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Failed to fetch from AniDB: {}", e);
                    return Ok(None);
                }
            };

        let bytes = match response.bytes().await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to get response bytes: {}", e);
                return Ok(None);
            }
        };

        // Create GzDecoder with mut
        let mut decoder = flate2::read::GzDecoder::new(&bytes[..]);
        
        // Read and decompress the gzipped data
        let mut decompressed = Vec::new();
        if let Err(e) = decoder.read_to_end(&mut decompressed) {  // Simplified read_to_end call
            tracing::error!("Failed to decompress data: {}", e);
            return Ok(None);
        }

        // Convert bytes to string
        let xml_str = match std::str::from_utf8(&decompressed) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to convert bytes to string: {}", e);
                return Ok(None);
            }
        };

        // Parse XML
        let doc = match roxmltree::Document::parse(xml_str) {
            Ok(doc) => doc,
            Err(e) => {
                tracing::error!("Failed to parse XML: {}", e);
                return Ok(None);
            }
        };

        // Find best matching anime
        let mut best_match = None;
        let mut best_score = 0.0;

        for anime in doc.descendants().filter(|n| n.has_tag_name("anime")) {
            for title_node in anime.children().filter(|n| n.has_tag_name("title")) {
                let anime_title = title_node.text().unwrap_or_default();
                let score = string_similarity(&clean_title.to_lowercase(), &anime_title.to_lowercase());
                if score > best_score {
                    best_score = score;
                    best_match = Some(anime);
                }
            }
        }

        if let Some(anime) = best_match {
            if best_score > 0.8 {  // Only use if confidence is high enough
                let aid = anime.attribute("aid").unwrap_or_default();
                
                // Get detailed info
                let details_url = format!("{}{}", constants::ANIDB_HTTP_API_URL, aid);
                
                // Add delay
                tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

                let response = match self.http_client
                    .get(&details_url)
                    .send()
                    .await {
                        Ok(resp) => resp,
                        Err(e) => {
                            tracing::error!("Failed to fetch anime details: {}", e);
                            return Ok(None);
                        }
                    };

                let text = match response.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::error!("Failed to get response text: {}", e);
                        return Ok(None);
                    }
                };

                let doc = match roxmltree::Document::parse(&text) {
                    Ok(doc) => doc,
                    Err(e) => {
                        tracing::error!("Failed to parse XML: {}", e);
                        return Ok(None);
                    }
                };

                if let Some(anime) = doc.descendants().find(|n| n.has_tag_name("anime")) {
                    let image_url = format!("{}{}", 
                        constants::ANIDB_PIC_BASE_URL,
                        anime.descendants()
                            .find(|n| n.has_tag_name("picture"))
                            .and_then(|n| n.text())
                            .unwrap_or_default()
                    );

                    // Cache the image URL
                    self.image_cache.set(clean_title, image_url.clone());

                    let genres = anime.descendants()
                        .filter(|n| n.has_tag_name("tag"))
                        .filter_map(|n| n.text())
                        .map(String::from)
                        .collect();

                    return Ok(Some(HamaMetadata {
                        title: title.to_string(),
                        original_title: anime.descendants()
                            .find(|n| n.has_tag_name("title") && n.attribute("xml:lang") == Some("ja"))
                            .and_then(|n| n.text())
                            .map(String::from),
                        season_count: 1,
                        episode_count: anime.descendants()
                            .find(|n| n.has_tag_name("episodecount"))
                            .and_then(|n| n.text())
                            .and_then(|t| t.parse().ok())
                            .unwrap_or(0),
                        special_count: 0,
                        year: anime.descendants()
                            .find(|n| n.has_tag_name("startdate"))
                            .and_then(|n| n.text())
                            .and_then(|d| d.split('-').next())
                            .and_then(|y| y.parse().ok()),
                        studio: anime.descendants()
                            .find(|n| n.has_tag_name("creators"))
                            .and_then(|n| n.text())
                            .map(String::from),
                        genres,
                        summary: anime.descendants()
                            .find(|n| n.has_tag_name("description"))
                            .and_then(|n| n.text())
                            .map(String::from),
                        rating: anime.descendants()
                            .find(|n| n.has_tag_name("ratings"))
                            .and_then(|n| n.text())
                            .and_then(|t| t.parse().ok())
                            .map(|r: f32| r / 10.0),
                        content_rating: None,
                        image_url: Some(image_url),
                        banner_url: None,
                        theme_url: None,
                        originally_available_at: anime.descendants()
                            .find(|n| n.has_tag_name("startdate"))
                            .and_then(|n| n.text())
                            .map(String::from),
                        directors: Vec::new(),
                        writers: Vec::new(),
                        collections: Vec::new(),
                        episodes: Vec::new(),
                        specials: Vec::new(),
                    }));
                }
            }
        }

        Ok(None)
    }

    // Helper function to extract series name from a file name
    fn extract_series_name_from_file(&self, file_name: &str) -> Option<String> {
        // Try SubsPlease pattern first
        if let Some(caps) = patterns::SUBSPLEASE.captures(file_name) {
            let title = clean_title_for_search(caps.get(1)?.as_str());
            tracing::info!("Extracted title '{}' using SubsPlease pattern from '{}'", title, file_name);
            return Some(title);
        }
        
        // Try Bluray pattern
        if let Some(caps) = patterns::BLURAY.captures(file_name) {
            let title = clean_title_for_search(caps.get(1)?.as_str());
            tracing::info!("Extracted title '{}' using Bluray pattern from '{}'", title, file_name);
            return Some(title);
        }
        
        // Try splitting by " - " and take the first part
        let parts: Vec<&str> = file_name.split(" - ").collect();
        if !parts.is_empty() {
            let title = clean_title_for_search(parts[0]);
            tracing::info!("Extracted title '{}' using split pattern from '{}'", title, file_name);
            return Some(title);
        }

        // Try extracting from the file name itself
        let name_without_ext = file_name.rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(file_name);
        
        // Remove episode numbers and common patterns
        let clean_name = name_without_ext
            .replace(|c: char| c.is_numeric(), "")
            .replace("Episode", "")
            .replace("Ep", "")
            .replace("E", "");
            
        let title = clean_title_for_search(&clean_name);
        if !title.is_empty() {
            tracing::info!("Extracted title '{}' using fallback pattern from '{}'", title, file_name);
            return Some(title);
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
        .replace("  ", " ")
        .trim()
        .to_string();

    // Split by " - " and take the first part if it exists, but only if there's content after the dash
    let parts: Vec<&str> = clean.split(" - ").collect();
    let final_title = if parts.len() > 1 && !parts[1].trim().is_empty() {
        parts[0].trim()
    } else {
        clean.trim()
    }.to_string();

    tracing::info!("Cleaned title '{}' to '{}'", title, final_title);
    final_title
}

// Helper function to extract episode number
fn extract_episode_number(filename: &str) -> i32 {
    // Try SubsPlease pattern first
    if let Some(caps) = patterns::SUBSPLEASE.captures(filename) {
        if let Some(ep_str) = caps.get(2) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using SubsPlease pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try Bluray pattern
    if let Some(caps) = patterns::BLURAY.captures(filename) {
        if let Some(ep_str) = caps.get(3) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using Bluray pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try numeric pattern (e.g., "01.mkv")
    if let Some(caps) = patterns::NUMERIC.captures(filename) {
        if let Some(ep_str) = caps.get(1) {
            if let Ok(num) = ep_str.as_str().parse::<i32>() {
                tracing::debug!("Found episode number {} using numeric pattern in {}", num, filename);
                return num;
            }
        }
    }

    // Try episode pattern
    if let Some(caps) = patterns::EPISODE.captures(filename) {
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

// Helper functions
fn is_video_file(file_name: &str) -> bool {
    file_name.ends_with(".mkv") || file_name.ends_with(".mp4")
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