use serde::{Deserialize, Serialize};
use anyhow::Result;
use reqwest::Client;
use lazy_static::lazy_static;
use regex::Regex;

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
}

impl AnimeClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn get_available_anime(&self) -> Result<Vec<AnimeInfo>> {
        // Fetch RSS feed
        let rss_url = "https://subsplease.org/rss/?r=1080";
        let rss_content = self.client.get(rss_url).send().await?.text().await?;
        
        // Parse RSS feed
        let channel = rss::Channel::read_from(rss_content.as_bytes())?;
        let mut anime_list = Vec::new();

        for item in channel.items() {
            let title = item.title().unwrap_or_default();
            // Extract anime name and episode number using regex
            if let Some(anime_info) = self.parse_title(title) {
                if let Some(metadata) = self.fetch_anime_metadata(&anime_info.series_name).await? {
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
        }

        Ok(anime_list)
    }

    async fn fetch_anime_metadata(&self, title: &str) -> Result<Option<AnimeMetadata>> {
        // Use Jikan API to search for anime
        let search_url = format!(
            "https://api.jikan.moe/v4/anime?q={}&limit=1",
            urlencoding::encode(title)
        );
        
        let response = self.client.get(&search_url).send().await?;
        let data: serde_json::Value = response.json().await?;
        
        if let Some(anime) = data.get("data").and_then(|d| d.get(0)) {
            Ok(Some(AnimeMetadata {
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
            }))
        } else {
            Ok(None)
        }
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