use anyhow::Result;
use reqwest::{Client, ClientBuilder};
use rss::Channel;
use scraper::{Html, Selector};
use serde::{Serialize, Deserialize};
use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;

static TRACKED_ANIME: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));
static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| Client::new());
static QB_CLIENT: Lazy<Mutex<Option<Arc<QBittorrent>>>> = Lazy::new(|| Mutex::new(None));

const SUBSPLEASE_RSS_URL: &str = "https://subsplease.org/rss/?r=1080";
const SUBSPLEASE_SCHEDULE_URL: &str = "https://subsplease.org/schedule/";

struct QBittorrent {
    client: Client,
    base_url: String,
}

impl QBittorrent {
    async fn new(url: &str, username: &str, password: &str) -> Result<Self, String> {
        let client = ClientBuilder::new()
            .cookie_store(true)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let qb = QBittorrent {
            client,
            base_url: url.trim_end_matches('/').to_string(),
        };

        // Login
        let form = HashMap::from([
            ("username", username),
            ("password", password),
        ]);

        let response = qb.client
            .post(format!("{}/api/v2/auth/login", qb.base_url))
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("Failed to send login request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Login failed: {}", response.status()));
        }

        Ok(qb)
    }

    async fn add_rss_rule(&self, name: &str, pattern: &str, save_path: &str) -> Result<(), String> {
        let rule_def = format!(
            r#"{{"enabled":true,"mustContain":"{}","mustNotContain":"","useRegex":false,"episodeFilter":"","smartFilter":false,"previouslyMatchedEpisodes":[],"affectedFeeds":["{}"],"ignoreDays":0,"lastMatch":"","addPaused":false,"assignedCategory":"","savePath":"{}"}}"#,
            pattern, SUBSPLEASE_RSS_URL, save_path
        );

        let mut form = HashMap::new();
        form.insert("ruleName", name);
        form.insert("ruleDef", &rule_def);

        let response = self.client
            .post(format!("{}/api/v2/rss/setRule", self.base_url))
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("Failed to add RSS rule: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to add RSS rule: {}", response.status()));
        }

        Ok(())
    }

    async fn remove_rss_rule(&self, name: &str) -> Result<(), String> {
        let mut form = HashMap::new();
        form.insert("ruleName", name);

        let response = self.client
            .post(format!("{}/api/v2/rss/removeRule", self.base_url))
            .form(&form)
            .send()
            .await
            .map_err(|e| format!("Failed to remove RSS rule: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to remove RSS rule: {}", response.status()));
        }

        Ok(())
    }

    async fn get_rss_rules(&self) -> Result<Vec<QBitTorrentRule>, String> {
        let response = self.client
            .get(format!("{}/api/v2/rss/rules", self.base_url))
            .send()
            .await
            .map_err(|e| format!("Failed to get RSS rules: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Failed to get RSS rules: {}", response.status()));
        }

        let rules: HashMap<String, serde_json::Value> = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse RSS rules: {}", e))?;

        Ok(rules
            .into_iter()
            .map(|(name, rule)| {
                let rule_obj = rule.as_object().unwrap();
                QBitTorrentRule {
                    name,
                    pattern: rule_obj.get("mustContain").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    save_path: rule_obj.get("savePath").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    enabled: rule_obj.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false),
                }
            })
            .collect())
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct AnimeEntry {
    pub title: String,
    pub link: String,
    pub date: String,
    pub image_url: Option<String>,
    pub summary: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct ScheduleEntry {
    pub title: String,
    pub episode: String,
    pub air_date: String,
    pub time: String,
    pub eta: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QBitTorrentRule {
    pub name: String,
    pub pattern: String,
    pub save_path: String,
    pub enabled: bool,
}

#[tauri::command]
pub async fn initialize_qbittorrent(url: String, username: String, password: String) -> Result<(), String> {
    let client = QBittorrent::new(&url, &username, &password).await?;
    let mut qb = QB_CLIENT.lock().unwrap();
    *qb = Some(Arc::new(client));
    Ok(())
}

#[tauri::command]
pub async fn fetch_rss_feed() -> Result<Vec<AnimeEntry>, String> {
    let response = HTTP_CLIENT
        .get(SUBSPLEASE_RSS_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let content = response.bytes().await.map_err(|e| e.to_string())?;
    let channel = Channel::read_from(&content[..]).map_err(|e| e.to_string())?;
    
    let entries: Vec<AnimeEntry> = channel
        .items()
        .iter()
        .map(|item| AnimeEntry {
            title: item.title().unwrap_or("Unknown").to_string(),
            link: item.link().unwrap_or("").to_string(),
            date: item.pub_date().unwrap_or("Unknown").to_string(),
            image_url: None,
            summary: item.description().map(String::from),
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn get_schedule() -> Result<Vec<ScheduleEntry>, String> {
    let response = HTTP_CLIENT
        .get(SUBSPLEASE_SCHEDULE_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    let html = response.text().await.map_err(|e| e.to_string())?;
    let document = Html::parse_document(&html);
    
    let schedule_selector = Selector::parse(".schedule-page .all-schedule .schedule-table tr").unwrap();
    let title_selector = Selector::parse("td:nth-child(1)").unwrap();
    let time_selector = Selector::parse("td:nth-child(2)").unwrap();
    
    let mut schedule: Vec<ScheduleEntry> = Vec::new();
    
    for element in document.select(&schedule_selector) {
        let title = element
            .select(&title_selector)
            .next()
            .and_then(|e| Some(e.text().collect::<String>().trim().to_string()))
            .unwrap_or_default();
            
        let time = element
            .select(&time_selector)
            .next()
            .and_then(|e| Some(e.text().collect::<String>().trim().to_string()))
            .unwrap_or_default();
            
        if !title.is_empty() && !time.is_empty() {
            schedule.push(ScheduleEntry {
                title,
                episode: "TBA".to_string(),
                air_date: chrono::Utc::now().date_naive().to_string(),
                time,
                eta: None,
            });
        }
    }
    
    Ok(schedule)
}

#[tauri::command]
pub async fn get_tracked_anime() -> Result<Vec<String>, String> {
    let tracked = TRACKED_ANIME.lock().unwrap().clone();
    Ok(tracked)
}

#[tauri::command]
pub async fn track_anime(title: String) -> Result<(), String> {
    println!("🎯 Tracking anime: {}", title);
    
    // First check if already tracked
    {
        let mut tracked = TRACKED_ANIME.lock().unwrap();
        if tracked.contains(&title) {
            println!("ℹ️ Anime already tracked");
            return Ok(());
        }
        tracked.push(title.clone());
        println!("✅ Anime tracked successfully. Current list: {:?}", *tracked);
    }
    
    // Then add RSS rule if qBittorrent is available
    let qb_client = {
        let guard = QB_CLIENT.lock().unwrap();
        guard.as_ref().map(Arc::clone)
    };

    if let Some(client) = qb_client {
        let rule_name = format!("SubsPlease - {}", title);
        let pattern = format!("SubsPlease.*{}.*1080p", title);
        
        if let Err(e) = client.add_rss_rule(&rule_name, &pattern, "/downloads/anime").await {
            println!("❌ Failed to add qBittorrent RSS rule: {}", e);
        } else {
            println!("✅ Added qBittorrent RSS rule for: {}", title);
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn untrack_anime(title: String) -> Result<(), String> {
    println!("🎯 Untracking anime: {}", title);
    
    // First remove from tracked list
    {
        let mut tracked = TRACKED_ANIME.lock().unwrap();
        tracked.retain(|t| t != &title);
        println!("✅ Anime untracked successfully. Current list: {:?}", *tracked);
    }
    
    // Then remove RSS rule if qBittorrent is available
    let qb_client = {
        let guard = QB_CLIENT.lock().unwrap();
        guard.as_ref().map(Arc::clone)
    };

    if let Some(client) = qb_client {
        let rule_name = format!("SubsPlease - {}", title);
        if let Err(e) = client.remove_rss_rule(&rule_name).await {
            println!("❌ Failed to remove qBittorrent RSS rule: {}", e);
        } else {
            println!("✅ Removed qBittorrent RSS rule for: {}", title);
        }
    }
    
    Ok(())
}

#[tauri::command]
pub async fn get_qbittorrent_rules() -> Result<Vec<QBitTorrentRule>, String> {
    let qb_client = {
        let guard = QB_CLIENT.lock().unwrap();
        guard.as_ref().map(Arc::clone)
    };
    
    let client = qb_client.ok_or("qBittorrent not initialized")?;
    client.get_rss_rules().await
}

#[tauri::command]
pub async fn add_qbittorrent_rule(rule: QBitTorrentRule) -> Result<(), String> {
    let qb_client = {
        let guard = QB_CLIENT.lock().unwrap();
        guard.as_ref().map(Arc::clone)
    };
    
    let client = qb_client.ok_or("qBittorrent not initialized")?;
    client.add_rss_rule(&rule.name, &rule.pattern, &rule.save_path).await
} 