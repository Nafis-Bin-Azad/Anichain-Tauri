use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use rss::Channel;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{error, info};

/// Default image URL used when no image is available.
const DEFAULT_IMAGE_URL: &str =
    "https://placehold.co/225x319/gray/white/png?text=IMAGE+NOT+AVAILABLE";

lazy_static! {
    static ref TITLE_REGEX: Regex =
        Regex::new(r"^\[SubsPlease\] (.+) - (\d+) \(1080p\)").unwrap();
}

/// Contains metadata information for an anime.
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

/// Represents an episode of an anime.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Episode {
    pub title: String,
    pub number: i32,
    pub magnet_url: String,
    pub size: String,
    pub release_date: String,
}

/// Combines anime metadata with its latest episode information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnimeInfo {
    pub metadata: AnimeMetadata,
    pub latest_episode: Episode,
}

/// Represents a schedule entry for an anime airing.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScheduleEntry {
    pub title: String,
    pub time: String,
    pub episode: i32,
    pub air_date: DateTime<Utc>,
    pub day: String,
}

/// Client for fetching anime information and schedules.
#[derive(Debug, Clone)]
pub struct AnimeClient {
    client: Arc<Client>,
    anime_list: Arc<Mutex<Vec<AnimeInfo>>>,
}

impl AnimeClient {
    /// Creates a new instance of `AnimeClient`.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client: Arc::new(client),
            anime_list: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns a clone of the current list of available anime.
    pub async fn get_available_anime(&self) -> Result<Vec<AnimeInfo>> {
        let list = self.anime_list.lock().await;
        Ok(list.clone())
    }

    /// Refreshes the anime list by fetching the RSS feed and updating detailed metadata in the background.
    pub async fn refresh_anime_list(&self) -> Result<()> {
        // Fetch RSS feed.
        let rss_url = "https://subsplease.org/rss/?r=1080";
        info!("Fetching RSS feed from {}", rss_url);
        let rss_content = self
            .client
            .get(rss_url)
            .send()
            .await?
            .text()
            .await?;

        // Parse the RSS feed.
        let channel = Channel::read_from(rss_content.as_bytes())?;
        let items: Vec<_> = channel.items().iter().cloned().collect();
        let mut basic_anime_list = Vec::new();

        // Create basic anime entries.
        for item in &items {
            let title = item.title().unwrap_or_default();
            if let Some(anime_info) = self.parse_title(title) {
                let metadata = AnimeMetadata {
                    title: anime_info.series_name.clone(),
                    image_url: DEFAULT_IMAGE_URL.to_string(),
                    synopsis: String::new(),
                    score: None,
                    episodes: None,
                    status: "Loading...".to_string(),
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

                basic_anime_list.push(AnimeInfo {
                    metadata,
                    latest_episode: episode,
                });
            }
        }

        // Update the shared list.
        {
            let mut list = self.anime_list.lock().await;
            *list = basic_anime_list;
        }

        // Spawn a background task to fetch detailed metadata.
        let background_client = Arc::clone(&self.client);
        let shared_list = self.anime_list.clone();
        let items_clone = items.clone();

        tokio::spawn(async move {
            let mut last_request = Instant::now();

            for (i, item) in items_clone.iter().enumerate() {
                let title = item.title().unwrap_or_default();
                if let Some(captures) = TITLE_REGEX.captures(title) {
                    let series_name = captures
                        .get(1)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default();
                    let episode_number = captures
                        .get(2)
                        .and_then(|m| m.as_str().parse().ok())
                        .unwrap_or(0);
                    let anime_info = AnimeNameInfo {
                        series_name: series_name.clone(),
                        episode_number,
                    };

                    // Respect rate limiting.
                    let elapsed = last_request.elapsed();
                    if elapsed < Duration::from_millis(350) {
                        sleep(Duration::from_millis(350) - elapsed).await;
                    }

                    // Try different search strategies.
                    let search_attempts = vec![
                        anime_info.clean_title(),
                        anime_info.series_name.clone(),
                        anime_info.get_alternative_title(),
                    ];

                    let mut found_match = false;
                    for search_title in search_attempts {
                        if search_title.is_empty() {
                            continue;
                        }

                        let search_url = format!(
                            "https://api.jikan.moe/v4/anime?q={}&limit=1",
                            urlencoding::encode(&search_title)
                        );

                        if let Ok(response) = background_client.get(&search_url).send().await {
                            if let Ok(data) = response.json::<serde_json::Value>().await {
                                if let Some(results) = data.get("data") {
                                    if results.as_array().map_or(false, |arr| !arr.is_empty()) {
                                        if let Some(anime) = results.get(0) {
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

                                            // Update the corresponding anime metadata.
                                            let mut list = shared_list.lock().await;
                                            if i < list.len() {
                                                list[i].metadata = AnimeMetadata {
                                                    title: anime_info.series_name.clone(),
                                                    image_url,
                                                    synopsis: anime
                                                        .get("synopsis")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or_default()
                                                        .to_string(),
                                                    score: anime
                                                        .get("score")
                                                        .and_then(|s| s.as_f64())
                                                        .map(|s| s as f32),
                                                    episodes: anime
                                                        .get("episodes")
                                                        .and_then(|e| e.as_i64())
                                                        .map(|e| e as i32),
                                                    status: anime
                                                        .get("status")
                                                        .and_then(|s| s.as_str())
                                                        .unwrap_or_default()
                                                        .to_string(),
                                                    season: anime
                                                        .get("season")
                                                        .and_then(|s| s.as_str())
                                                        .map(|s| s.to_string()),
                                                    year: anime
                                                        .get("year")
                                                        .and_then(|y| y.as_i64())
                                                        .map(|y| y as i32),
                                                };
                                            }
                                            found_match = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }

                        // Delay between search attempts.
                        sleep(Duration::from_millis(350)).await;
                    }

                    // Mark the anime as unknown if no match was found.
                    if !found_match {
                        let mut list = shared_list.lock().await;
                        if i < list.len() {
                            list[i].metadata.status = "Unknown".to_string();
                            list[i].metadata.image_url = DEFAULT_IMAGE_URL.to_string();
                        }
                    }

                    last_request = Instant::now();
                }
            }
        });

        Ok(())
    }

    /// Fetches the anime schedule from the SubsPlease API.
    pub async fn get_schedule(&self) -> Result<Vec<ScheduleEntry>> {
        let schedule_url = "https://subsplease.org/api/?f=schedule&tz=UTC";
        info!("Fetching schedule from SubsPlease API at {}", schedule_url);

        let response = self
            .client
            .get(schedule_url)
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) \
                 AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36",
            )
            .header("Accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            error!("Failed to fetch schedule: {}", response.status());
            return Err(anyhow::anyhow!("Failed to fetch schedule API"));
        }

        let text = response.text().await?;
        let api_response: ApiResponse = serde_json::from_str(&text)?;
        let mut schedule = Vec::new();

        // Process each day's schedule.
        for (day, shows) in api_response.schedule.into_iter() {
            for show in shows {
                if !show.title.is_empty() && !show.time.is_empty() {
                    if let Ok(air_date) = parse_schedule_time(&show.time) {
                        schedule.push(ScheduleEntry {
                            title: show.title,
                            time: show.time,
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

    /// Parses an anime title using a regular expression.
    fn parse_title(&self, title: &str) -> Option<AnimeNameInfo> {
        TITLE_REGEX.captures(title).map(|caps| AnimeNameInfo {
            series_name: caps
                .get(1)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
            episode_number: caps
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0),
        })
    }
}

/// Helper type for holding parsed anime name information.
#[derive(Debug)]
struct AnimeNameInfo {
    series_name: String,
    episode_number: i32,
}

impl AnimeNameInfo {
    /// Returns a cleaned title suitable for search queries.
    fn clean_title(&self) -> String {
        self.series_name
            .replace("Season", "")
            .replace(|c: char| c.is_numeric() || c == 'S', "")
            .split('[')
            .next()
            .unwrap_or(&self.series_name)
            .split('(')
            .next()
            .unwrap_or(&self.series_name)
            .trim()
            .to_string()
    }

    /// Returns an alternative title variation for search queries.
    fn get_alternative_title(&self) -> String {
        self.series_name
            .split(" - ")
            .next()
            .unwrap_or(&self.series_name)
            .split(':')
            .next()
            .unwrap_or(&self.series_name)
            .trim()
            .to_string()
    }
}

/// Private type representing the API response for schedule data.
#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    schedule: HashMap<String, Vec<ApiShow>>,
}

/// Private type representing an individual show in the schedule API.
#[derive(Debug, Serialize, Deserialize)]
struct ApiShow {
    title: String,
    time: String,
    #[serde(default)]
    episode: Option<i32>,
}

/// Parses a schedule time string (assumed to be in JST) in the format "HH:MM"
/// and returns the corresponding UTC time.
fn parse_schedule_time(time_str: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    let today = now.date_naive();

    let time_parts: Vec<&str> = time_str.trim().split(':').collect();
    if time_parts.len() != 2 {
        return Err(anyhow::anyhow!("Invalid time format: {}", time_str));
    }

    let hour: i32 = time_parts[0].trim().parse()?;
    let minute: i32 = time_parts[1].trim().parse()?;

    // Adjust the hour if it is 24+.
    let adjusted_hour = hour % 24;
    let schedule_time = today
        .and_hms_opt(adjusted_hour as u32, minute as u32, 0)
        .ok_or_else(|| anyhow::anyhow!("Invalid time: {}:{}", adjusted_hour, minute))?;

    // Convert from JST (UTC+9) to UTC.
    let jst_offset = ChronoDuration::hours(9);
    let utc_time = DateTime::<Utc>::from_naive_utc_and_offset(schedule_time, Utc) - jst_offset;

    Ok(utc_time)
}
