use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::Client;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeMetadata {
    pub title: String,
    pub image_url: String,
    pub synopsis: String,
    pub score: Option<f32>,
    pub episodes: Option<i32>,
    pub status: String,
    pub season: Option<String>,
    pub year: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Episode {
    pub title: String,
    pub number: i32,
    pub magnet_url: String,
    pub size: String,
    pub release_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeInfo {
    pub metadata: AnimeMetadata,
    pub latest_episode: Episode,
}

lazy_static! {
    static ref TITLE_REGEX: Regex = Regex::new(
        r"^\[SubsPlease\] (.+) - (\d+) \(1080p\)"
    ).unwrap();
}

// Add this constant for the default image
const DEFAULT_IMAGE_URL: &str = "https://placehold.co/225x319/png";

pub struct AnimeClient {
    client: Client,
    anime_list: Arc<Mutex<Vec<AnimeInfo>>>,
}

impl AnimeClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            anime_list: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn get_available_anime(&self) -> Result<Vec<AnimeInfo>> {
        let list = self.anime_list.lock().await;
        Ok(list.clone())
    }

    pub async fn refresh_anime_list(&self) -> Result<()> {
        // Fetch RSS feed
        let rss_url = "https://subsplease.org/rss/?r=1080";
        let rss_content = self.client.get(rss_url).send().await?.text().await?;
        
        // Parse RSS feed
        let channel = rss::Channel::read_from(rss_content.as_bytes())?;
        let mut anime_list = Vec::new();

        // Create basic entries for all anime first
        let items: Vec<_> = channel.items().into_iter().cloned().collect();
        for item in &items {
            let title = item.title().unwrap_or_default();
            if let Some(anime_info) = self.parse_title(title) {
                let metadata = AnimeMetadata {
                    title: anime_info.series_name.clone(),
                    image_url: String::from(DEFAULT_IMAGE_URL),
                    synopsis: String::new(),
                    score: None,
                    episodes: None,
                    status: String::from("Loading..."),
                    season: None,
                    year: None,
                };
                
                let episode = Episode {
                    title: title.to_string(),
                    number: anime_info.episode_number,
                    magnet_url: item.link().unwrap_or_default().to_string(),
                    size: item.description().unwrap_or_default().to_string(),
                    release_date: item.pub_date().unwrap_or_default().to_string(),
                };

                anime_list.push(AnimeInfo {
                    metadata,
                    latest_episode: episode,
                });
            }
        }

        // Update the shared list with basic entries
        {
            let mut list = self.anime_list.lock().await;
            *list = anime_list;
        }
        
        // Create a new client for the background task
        let background_client = Client::new();
        let shared_list = self.anime_list.clone();
        
        // Clone what we need for the background task
        let items = items.clone();
        
        // Spawn a background task to fetch metadata
        tokio::spawn(async move {
            let mut last_request = std::time::Instant::now();
            
            for (i, item) in items.iter().enumerate() {
                let title = item.title().unwrap_or_default();
                if let Some(captures) = TITLE_REGEX.captures(title) {
                    let series_name = captures.get(1).unwrap().as_str().to_string();
                    let anime_info = AnimeNameInfo {
                        series_name: series_name.clone(),
                        episode_number: captures.get(2).unwrap().as_str().parse().unwrap_or(0),
                    };
                    
                    // Add delay if needed to respect rate limit
                    let elapsed = last_request.elapsed();
                    if elapsed < Duration::from_millis(350) {
                        sleep(Duration::from_millis(350) - elapsed).await;
                    }
                    
                    // Try different search strategies
                    let search_attempts = vec![
                        anime_info.clean_title(),
                        anime_info.series_name.clone(),
                        anime_info.get_alternative_title(),
                    ];

                    let mut found_match = false;
                    for search_title in search_attempts {
                        if search_title.is_empty() { continue; }

                        let search_url = format!(
                            "https://api.jikan.moe/v4/anime?q={}&limit=1",
                            urlencoding::encode(&search_title)
                        );

                        println!("Attempting search with: {}", search_title);

                        if let Ok(response) = background_client.get(&search_url).send().await {
                            if let Ok(data) = response.json::<serde_json::Value>().await {
                                if let Some(results) = data.get("data") {
                                    if results.as_array().map_or(false, |arr| !arr.is_empty()) {
                                        if let Some(anime) = results.get(0) {
                                            let mut list = shared_list.lock().await;
                                            if i < list.len() {
                                                let image_url = anime.get("images")
                                                    .and_then(|i| i.get("jpg"))
                                                    .and_then(|j| j.get("large_image_url"))
                                                    .and_then(|u| u.as_str())
                                                    .map(|url| {
                                                        if url.is_empty() {
                                                            DEFAULT_IMAGE_URL.to_string()
                                                        } else {
                                                            url.to_string()
                                                        }
                                                    })
                                                    .unwrap_or_else(|| DEFAULT_IMAGE_URL.to_string());

                                                list[i].metadata = AnimeMetadata {
                                                    title: anime_info.series_name.clone(),
                                                    image_url,
                                                    synopsis: anime.get("synopsis").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                                                    score: anime.get("score").and_then(|s| s.as_f64()).map(|s| s as f32),
                                                    episodes: anime.get("episodes").and_then(|e| e.as_i64()).map(|e| e as i32),
                                                    status: anime.get("status").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                                                    season: anime.get("season").and_then(|s| s.as_str()).map(|s| s.to_string()),
                                                    year: anime.get("year").and_then(|y| y.as_i64()).map(|y| y as i32),
                                                };
                                            }
                                            found_match = true;
                                            println!("Found match using: {}", search_title);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                        
                        // Add delay between search attempts
                        sleep(Duration::from_millis(350)).await;
                    }

                    if !found_match {
                        let mut list = shared_list.lock().await;
                        if i < list.len() {
                            list[i].metadata.status = String::from("Unknown");
                            list[i].metadata.image_url = DEFAULT_IMAGE_URL.to_string();
                        }
                        println!("No matches found for: {}", anime_info.series_name);
                    }
                    
                    last_request = std::time::Instant::now();
                }
            }
        });

        Ok(())
    }

    fn parse_title(&self, title: &str) -> Option<AnimeNameInfo> {
        TITLE_REGEX.captures(title).map(|caps| {
            AnimeNameInfo {
                series_name: caps.get(1).unwrap().as_str().to_string(),
                episode_number: caps.get(2).unwrap().as_str().parse().unwrap_or(0),
            }
        })
    }
}

#[derive(Debug)]
struct AnimeNameInfo {
    series_name: String,
    episode_number: i32,
}

impl AnimeNameInfo {
    fn clean_title(&self) -> String {
        let search_title = self.series_name
            .replace("Season", "")
            .replace(|c: char| c.is_numeric() || c == 'S', "")
            .split('[').next().unwrap_or(&self.series_name)
            .split('(').next().unwrap_or(&self.series_name)
            .trim()
            .to_string();

        println!("Search title: '{}'", search_title);
        search_title
    }

    fn get_alternative_title(&self) -> String {
        // Try different title variations
        let title = self.series_name
            .split(" - ").next()           // Take first part before any dash
            .unwrap_or(&self.series_name)
            .split(':').next()             // Take first part before any colon
            .unwrap_or(&self.series_name)
            .trim()
            .to_string();

        if title != self.series_name {
            println!("Alternative title: '{}'", title);
        }
        title
    }
} 