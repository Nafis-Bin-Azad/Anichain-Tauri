use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use std::path::Path;
use reqwest::Client;
use std::collections::HashMap;

#[derive(Debug, Serialize, Clone)]
pub struct AnimeEpisode {
    pub number: i32,
    pub file_name: String,
    pub path: String,
    pub is_special: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnimeSeason {
    pub number: i32,
    pub episodes: Vec<AnimeEpisode>,
    pub specials: Vec<AnimeEpisode>,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnimeMetadata {
    pub title: String,
    pub clean_title: String,
    pub seasons: Vec<AnimeSeason>,
    pub year: Option<i32>,
    pub studio: Option<String>,
    pub genres: Vec<String>,
    pub summary: Option<String>,
    pub rating: Option<f32>,
    pub total_episodes: i32,
    pub total_specials: i32,
    pub image_url: Option<String>,
    pub path: String,
}

lazy_static! {
    static ref ANIME_PATTERNS: Vec<Regex> = vec![
        // SubsPlease pattern with episode
        Regex::new(r"^\[SubsPlease\] (?P<title>.*?)(?:[ _]-[ _](?P<episode>\d{1,3}))").unwrap(),
        // Erai-raws pattern with episode
        Regex::new(r"^\[Erai-raws\] (?P<title>.*?)(?:[ _]-[ _](?P<episode>\d{1,3}))").unwrap(),
        // HorribleSubs pattern with episode
        Regex::new(r"^\[HorribleSubs\] (?P<title>.*?)(?:[ _]-[ _](?P<episode>\d{1,3}))").unwrap(),
        // General fansub pattern with episode
        Regex::new(r"^\[(?P<group>[^\]]+)\] (?P<title>[^-]+)(?:[ _]-[ _](?P<episode>\d{1,3}))").unwrap(),
        // Season pattern
        Regex::new(r"(?i)S(?P<season>\d{1,2})|Season[ _](?P<season2>\d{1,2})").unwrap(),
        // Episode pattern
        Regex::new(r"(?i)[ _-](?:E|EP|Episode)?(?P<episode>\d{1,3})(?:v\d)?[ _]?").unwrap(),
        // Clean title pattern
        Regex::new(r"(?i)(\d{3,4}p|10.?bit|dual.?audio|bluray|webrip|x265|hevc|flac|aac|multi|remaster|\[.*?\]|\(.*?\)|\.|_)").unwrap(),
    ];

    static ref SPECIAL_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(Special|SP|OVA|ONA|Preview|Extra)").unwrap(),
    ];
}

fn extract_episode_info(filename: &str) -> Option<(String, i32, Option<i32>)> {
    // Try all patterns to extract title and episode number
    for pattern in ANIME_PATTERNS.iter().take(4) {
        if let Some(caps) = pattern.captures(filename) {
            if let Some(title) = caps.name("title") {
                let raw_title = title.as_str().trim().to_string();
                // Clean the title
                let clean_title = ANIME_PATTERNS[6].replace_all(&raw_title, " ")
                    .trim()
                    .replace("  ", " ")
                    .to_string();
                
                // Extract season number
                let season = if let Some(caps) = ANIME_PATTERNS[4].captures(&filename) {
                    caps.name("season")
                        .or_else(|| caps.name("season2"))
                        .and_then(|s| s.as_str().parse().ok())
                } else {
                    None
                };

                // Extract episode number
                let episode = if let Some(ep) = caps.name("episode") {
                    ep.as_str().parse().unwrap_or(1)
                } else if let Some(caps) = ANIME_PATTERNS[5].captures(&filename) {
                    caps.name("episode")
                        .and_then(|e| e.as_str().parse().ok())
                        .unwrap_or(1)
                } else {
                    1
                };

                return Some((clean_title, episode, season));
            }
        }
    }
    None
}

async fn fetch_anime_info(title: &str) -> Option<(String, String)> {
    tracing::info!("Fetching anime info for title: {}", title);
    let client = Client::new();
    let url = format!("https://api.jikan.moe/v4/anime?q={}&limit=1", urlencoding::encode(title));
    
    match client.get(&url).send().await {
        Ok(response) => {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                if let Some(results) = data.get("data").and_then(|d| d.as_array()) {
                    if let Some(first) = results.first() {
                        let title = first.get("title").and_then(|t| t.as_str()).unwrap_or(title).to_string();
                        let image = first.get("images")
                            .and_then(|i| i.get("jpg"))
                            .and_then(|j| j.get("large_image_url"))
                            .and_then(|u| u.as_str())
                            .map(|s| s.to_string());
                        tracing::info!("Found anime: {} with image URL: {:?}", title, image);
                        return Some((title, image.unwrap_or_default()));
                    }
                }
            }
            tracing::warn!("No anime found for title: {}", title);
        }
        Err(e) => {
            tracing::error!("Error fetching anime info: {}", e);
        }
    }
    None
}

pub async fn parse_anime_folder(path: &Path) -> Result<Vec<AnimeMetadata>> {
    tracing::info!("Parsing path: {:?}", path);
    let mut anime_map: HashMap<String, AnimeMetadata> = HashMap::new();
    
    // Recursively walk through the directory
    fn visit_dirs(dir: &Path, anime_map: &mut HashMap<String, AnimeMetadata>) -> Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, anime_map)?;
                } else {
                    process_file(&path, anime_map)?;
                }
            }
        }
        Ok(())
    }

    fn process_file(path: &Path, anime_map: &mut HashMap<String, AnimeMetadata>) -> Result<()> {
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip non-video files and hidden files
        if (!file_name.ends_with(".mkv") && !file_name.ends_with(".mp4")) || file_name.starts_with('.') {
            return Ok(());
        }

        if let Some((title, episode_num, season_num)) = extract_episode_info(&file_name) {
            let season_num = season_num.unwrap_or(1);
            let is_special = SPECIAL_PATTERNS.iter().any(|p| p.is_match(&file_name));

            let episode = AnimeEpisode {
                number: episode_num,
                file_name: file_name.clone(),
                path: path.to_string_lossy().to_string(),
                is_special,
            };

            let entry = anime_map.entry(title.clone()).or_insert_with(|| AnimeMetadata {
                title: title.clone(),
                clean_title: title,
                seasons: vec![],
                year: None,
                studio: None,
                genres: vec![],
                summary: None,
                rating: None,
                total_episodes: 0,
                total_specials: 0,
                image_url: None,
                path: path.parent().unwrap_or(path).to_string_lossy().to_string(),
            });

            // Find or create season
            while entry.seasons.len() < season_num as usize {
                entry.seasons.push(AnimeSeason {
                    number: entry.seasons.len() as i32 + 1,
                    episodes: vec![],
                    specials: vec![],
                });
            }

            let season = &mut entry.seasons[season_num as usize - 1];
            if is_special {
                season.specials.push(episode);
                entry.total_specials += 1;
            } else {
                season.episodes.push(episode);
                entry.total_episodes += 1;
            }
        }

        Ok(())
    }

    // Start the recursive scan
    visit_dirs(path, &mut anime_map)?;

    // Sort episodes within each season
    for metadata in anime_map.values_mut() {
        for season in &mut metadata.seasons {
            season.episodes.sort_by_key(|e| e.number);
            season.specials.sort_by_key(|e| e.number);
        }

        // Fetch additional metadata
        if let Some((mal_title, image_url)) = fetch_anime_info(&metadata.clean_title).await {
            metadata.title = mal_title;
            metadata.image_url = Some(image_url);
        }
    }

    let mut anime_list: Vec<_> = anime_map.into_values().collect();
    anime_list.sort_by(|a, b| a.title.cmp(&b.title));

    tracing::info!("Found {} unique anime series", anime_list.len());
    Ok(anime_list)
} 