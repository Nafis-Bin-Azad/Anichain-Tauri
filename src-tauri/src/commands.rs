use crate::error::Error;
use crate::qbittorrent::QBittorrent;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::sync::Arc;

static TRACKED_ANIME: Mutex<Vec<String>> = Mutex::new(Vec::new());
static QB_CLIENT: Mutex<Option<Arc<QBittorrent>>> = Mutex::new(None);

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

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub pattern: String,
    pub save_path: String,
}

#[tauri::command(rename_all = "snake_case")]
pub async fn initialize_qbittorrent(url: String, username: String, password: String) -> Result<(), Error> {
    let client = QBittorrent::new(url, username.clone(), password.clone());
    client.login(&username, &password).await?;
    let mut qb = QB_CLIENT.lock().unwrap();
    *qb = Some(Arc::new(client));
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn fetch_rss_feed() -> Result<Vec<AnimeEntry>, Error> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://subsplease.org/rss/?r=1080")
        .send()
        .await?
        .text()
        .await?;

    let feed = feed_rs::parser::parse(response.as_bytes())?;
    let entries: Vec<AnimeEntry> = feed.entries.into_iter().map(|entry| {
        AnimeEntry {
            title: entry.title.map(|t| t.content).unwrap_or_default(),
            link: entry.links.first().map(|l| l.href.clone()).unwrap_or_default(),
            date: entry.published.map(|d| d.to_string()).unwrap_or_default(),
            image_url: None,
            summary: entry.summary.map(|s| s.content),
        }
    }).collect();

    Ok(entries)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_schedule() -> Result<Vec<ScheduleEntry>, Error> {
    Ok(vec![ScheduleEntry {
        title: "Test Schedule".to_string(),
        episode: "1".to_string(),
        air_date: "2024-01-10".to_string(),
        time: "12:00".to_string(),
        eta: None,
    }])
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_tracked_anime() -> Result<Vec<String>, Error> {
    let tracked = TRACKED_ANIME.lock().unwrap().clone();
    Ok(tracked)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn track_anime(title: String) -> Result<(), Error> {
    let mut tracked = TRACKED_ANIME.lock().unwrap();
    if !tracked.contains(&title) {
        tracked.push(title);
    }
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn untrack_anime(title: String) -> Result<(), Error> {
    let mut tracked = TRACKED_ANIME.lock().unwrap();
    tracked.retain(|t| t != &title);
    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn get_qbittorrent_rules() -> Result<Vec<QBitTorrentRule>, Error> {
    Ok(vec![QBitTorrentRule {
        name: "Test Rule".to_string(),
        pattern: "Test Pattern".to_string(),
        save_path: "/downloads".to_string(),
        enabled: true,
    }])
}

#[tauri::command(rename_all = "snake_case")]
pub async fn add_qbittorrent_rule(rule: Rule) -> Result<(), Error> {
    let client = {
        let qb_client = QB_CLIENT.lock().unwrap();
        qb_client.as_ref().cloned().ok_or(Error::QBittorrentNotInitialized)?
    }; 
    
    client.add_rss_rule(&rule.name, &rule.pattern, &rule.save_path).await?;
    Ok(())
}