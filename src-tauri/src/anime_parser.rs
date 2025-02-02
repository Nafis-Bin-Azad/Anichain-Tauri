use anyhow::Result;
use futures::future::join_all;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use serde::Serialize;
use std::{
    collections::HashMap,
    path::Path,
};

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
        // Clean title pattern: Remove common video quality and release tags
        Regex::new(r"(?i)(\d{3,4}p|10.?bit|dual.?audio|bluray|webrip|x265|hevc|flac|aac|multi|remaster|\[.*?\]|\(.*?\)|\.|_)").unwrap(),
    ];

    static ref SPECIAL_PATTERNS: Vec<Regex> = vec![
        Regex::new(r"(?i)(Special|SP|OVA|ONA|Preview|Extra)").unwrap(),
    ];
}

/// Extracts a clean title, episode number, and (optional) season number from the filename.
fn extract_episode_info(filename: &str) -> Option<(String, i32, Option<i32>)> {
    // Try the first four patterns for basic title and episode extraction.
    for pattern in ANIME_PATTERNS.iter().take(4) {
        if let Some(caps) = pattern.captures(filename) {
            if let Some(title_match) = caps.name("title") {
                let raw_title = title_match.as_str().trim().to_string();
                // Clean the title by removing extraneous quality/release tags.
                let clean_title = ANIME_PATTERNS[6]
                    .replace_all(&raw_title, " ")
                    .trim()
                    .replace("  ", " ")
                    .to_string();

                // Extract season number using the season pattern.
                let season = if let Some(caps) = ANIME_PATTERNS[4].captures(filename) {
                    caps.name("season")
                        .or_else(|| caps.name("season2"))
                        .and_then(|s| s.as_str().parse().ok())
                } else {
                    None
                };

                // Extract episode number. Try the current pattern first,
                // then fall back to the separate episode pattern.
                let episode = if let Some(ep) = caps.name("episode") {
                    ep.as_str().parse().unwrap_or(1)
                } else if let Some(caps) = ANIME_PATTERNS[5].captures(filename) {
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

/// Asynchronously fetches additional anime information (such as title and image URL)
/// from the Jikan API using the provided title.
async fn fetch_anime_info(title: &str) -> Option<(String, String)> {
    tracing::info!("Fetching anime info for title: {}", title);
    let client = Client::new();
    let url = format!(
        "https://api.jikan.moe/v4/anime?q={}&limit=1",
        urlencoding::encode(title)
    );

    match client.get(&url).send().await {
        Ok(response) => {
            if let Ok(data) = response.json::<serde_json::Value>().await {
                if let Some(results) = data.get("data").and_then(|d| d.as_array()) {
                    if let Some(first) = results.first() {
                        let title = first
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or(title)
                            .to_string();
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

/// Parses an anime folder (recursively) to build metadata for each unique anime series.
pub async fn parse_anime_folder(path: &Path) -> Result<Vec<AnimeMetadata>> {
    tracing::info!("Parsing path: {:?}", path);
    let mut anime_map: HashMap<String, AnimeMetadata> = HashMap::new();

    // Recursively visit directories and process video files.
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

    // Process individual files.
    fn process_file(path: &Path, anime_map: &mut HashMap<String, AnimeMetadata>) -> Result<()> {
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        // Skip non-video files and hidden files.
        if ((!file_name.ends_with(".mkv") && !file_name.ends_with(".mp4"))
            || file_name.starts_with('.'))
        {
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

            // Create or get the correct season.
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

    // Start the recursive scan.
    visit_dirs(path, &mut anime_map)?;

    // Sort episodes and specials within each season.
    for metadata in anime_map.values_mut() {
        for season in &mut metadata.seasons {
            season.episodes.sort_by_key(|e| e.number);
            season.specials.sort_by_key(|e| e.number);
        }
    }

    // Collect futures to fetch additional metadata (e.g. proper title and image URL) for each anime.
    let fetch_futures = anime_map
        .values()
        .map(|metadata| {
            let clean_title = metadata.clean_title.clone();
            async move {
                if let Some((mal_title, image_url)) = fetch_anime_info(&clean_title).await {
                    (clean_title, mal_title, image_url)
                } else {
                    (clean_title, String::new(), String::new())
                }
            }
        })
        .collect::<Vec<_>>();

    let fetch_results = join_all(fetch_futures).await;
    // Update the metadata with fetched information.
    for (clean_title, mal_title, image_url) in fetch_results {
        if let Some(metadata) = anime_map.get_mut(&clean_title) {
            metadata.title = mal_title;
            metadata.image_url = Some(image_url);
        }
    }

    let mut anime_list: Vec<_> = anime_map.into_values().collect();
    anime_list.sort_by(|a, b| a.title.cmp(&b.title));

    tracing::info!("Found {} unique anime series", anime_list.len());
    Ok(anime_list)
}
