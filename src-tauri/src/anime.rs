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
                    image_url: String::new(),
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
        
        // Spawn a background task to fetch metadata
        tokio::spawn(async move {
            let mut last_request = std::time::Instant::now();
            
            for (i, item) in items.iter().enumerate() {
                let mut list = shared_list.lock().await;
                if i >= list.len() {
                    continue;
                }
                
                let title = item.title().unwrap_or_default();
                if let Some(captures) = TITLE_REGEX.captures(title) {
                    let series_name = captures.get(1).unwrap().as_str().to_string();
                    
                    // Add delay if needed to respect rate limit (3 requests per second)
                    let elapsed = last_request.elapsed();
                    if elapsed < Duration::from_millis(350) {
                        drop(list); // Release the lock during sleep
                        sleep(Duration::from_millis(350) - elapsed).await;
                        list = shared_list.lock().await;
                    }
                    
                    // Use the background client to fetch metadata
                    let search_url = format!(
                        "https://api.jikan.moe/v4/anime?q={}&limit=1",
                        urlencoding::encode(&series_name)
                    );
                    
                    // Release the lock while making the HTTP request
                    drop(list);
                    
                    match background_client.get(&search_url).send().await {
                        Ok(response) => {
                            if let Ok(data) = response.json::<serde_json::Value>().await {
                                let mut list = shared_list.lock().await;
                                if let Some(anime) = data.get("data").and_then(|d| d.get(0)) {
                                    let metadata = AnimeMetadata {
                                        title: anime.get("title").and_then(|t| t.as_str()).unwrap_or_default().to_string(),
                                        image_url: anime.get("images")
                                            .and_then(|i| i.get("jpg"))
                                            .and_then(|j| j.get("large_image_url"))
                                            .and_then(|u| u.as_str())
                                            .unwrap_or_default()
                                            .to_string(),
                                        synopsis: anime.get("synopsis").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                                        score: anime.get("score").and_then(|s| s.as_f64()).map(|s| s as f32),
                                        episodes: anime.get("episodes").and_then(|e| e.as_i64()).map(|e| e as i32),
                                        status: anime.get("status").and_then(|s| s.as_str()).unwrap_or_default().to_string(),
                                        season: anime.get("season").and_then(|s| s.as_str()).map(|s| s.to_string()),
                                        year: anime.get("year").and_then(|y| y.as_i64()).map(|y| y as i32),
                                    };
                                    if i < list.len() {
                                        list[i].metadata = metadata;
                                    }
                                } else if i < list.len() {
                                    list[i].metadata.status = String::from("Unknown");
                                }
                            }
                        }
                        Err(e) => {
                            let mut list = shared_list.lock().await;
                            println!("Error fetching metadata for {}: {}", series_name, e);
                            if i < list.len() {
                                list[i].metadata.status = String::from("Error loading metadata");
                            }
                        }
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