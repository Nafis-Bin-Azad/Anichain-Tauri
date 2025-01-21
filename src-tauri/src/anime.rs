use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::Client;
use lazy_static::lazy_static;
use regex::Regex;
use tokio::time::{sleep, Duration};
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use tracing::{info, error};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleEntry {
    pub title: String,
    pub time: String,
    pub episode: i32,
    pub air_date: DateTime<Utc>,
    pub day: String,
}

lazy_static! {
    static ref TITLE_REGEX: Regex = Regex::new(
        r"^\[SubsPlease\] (.+) - (\d+) \(1080p\)"
    ).unwrap();
}

// Add this constant for the default image
const DEFAULT_IMAGE_URL: &str = "https://placehold.co/225x319/gray/white/png?text=IMAGE+NOT+AVAILABLE";

#[derive(Debug, Clone)]
pub struct AnimeClient {
    client: Arc<Client>,
    anime_list: Arc<Mutex<Vec<AnimeInfo>>>,
}

impl AnimeClient {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap()),
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
        let background_client = Arc::clone(&self.client);
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
                    }
                    
                    last_request = std::time::Instant::now();
                }
            }
        });

        Ok(())
    }

    pub async fn get_schedule(&self) -> Result<Vec<ScheduleEntry>> {
        let schedule_url = "https://subsplease.org/api/?f=schedule&tz=UTC";
        
        info!("Fetching schedule from SubsPlease API");
        let response = self.client.get(schedule_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .header("Accept", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            error!("Failed to fetch schedule: {}", response.status());
            return Err(anyhow::anyhow!("Failed to fetch schedule API"));
        }
        
        let text = response.text().await?;
        
        #[derive(Debug, Serialize, Deserialize)]
        struct ApiResponse {
            schedule: std::collections::HashMap<String, Vec<ApiShow>>,
        }

        #[derive(Debug, Serialize, Deserialize)]
        struct ApiShow {
            title: String,
            time: String,
            #[serde(default)]
            episode: Option<i32>,
        }

        let api_response: ApiResponse = serde_json::from_str(&text)?;
        let mut schedule = Vec::new();
        
        // Process each day's schedule
        for (day, shows) in api_response.schedule.iter() {
            for show in shows {
                if !show.title.is_empty() && !show.time.is_empty() {
                    if let Ok(air_date) = parse_schedule_time(&show.time) {
                        schedule.push(ScheduleEntry {
                            title: show.title.clone(),
                            time: show.time.clone(),
                            episode: show.episode.unwrap_or(0),
                            air_date,
                            day: day.clone(),
                        });
                    }
                }
            }
        }
        
        info!("Successfully fetched {} schedule entries", schedule.len());
        schedule.sort_by(|a, b| a.air_date.cmp(&b.air_date));
        Ok(schedule)
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
            // Alternative title: '{}', title
        }
        title
    }
}

// Helper function to parse schedule time
fn parse_schedule_time(time_str: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let today = now.date_naive();
    
    let time_parts: Vec<&str> = time_str.trim().split(':').collect();
    if time_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid time format: {}", time_str));
    }
    
    let hour: i32 = time_parts[0].trim().parse()?;
    let minute: i32 = time_parts[1].trim().parse()?;
    
    // Handle potential 24+ hour format
    let adjusted_hour = hour % 24;
    
    let schedule_time = today.and_hms_opt(adjusted_hour as u32, minute as u32, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time: {}:{}", adjusted_hour, minute))?;
    
    // Convert from JST to UTC
    let jst_offset = chrono::Duration::hours(9);
    let utc_time = DateTime::<Utc>::from_naive_utc_and_offset(schedule_time, Utc) - jst_offset;
    
    Ok(utc_time)
} 