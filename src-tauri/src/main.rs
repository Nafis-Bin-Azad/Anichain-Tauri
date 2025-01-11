// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod qbittorrent;
mod anime;

use qbittorrent::{QBittorrentClient, QBittorrentConfig, TorrentInfo, RssRule, RssRuleInfo, RssArticle};
use anime::{AnimeClient, AnimeInfo};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, fs, path::PathBuf};
use tokio::sync::Mutex;
use tauri::State;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub qbittorrent: Option<QBittorrentConfig>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            qbittorrent: None,
        }
    }
}

struct AppState {
    qb_client: Arc<QBittorrentClient>,
    settings: Arc<Mutex<Settings>>,
    anime_client: Arc<AnimeClient>,
}

fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "anichain", "anichain") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

fn load_settings() -> Settings {
    let config_dir = get_config_dir();
    let settings_path = config_dir.join("settings.json");
    if let Ok(contents) = fs::read_to_string(&settings_path) {
        if let Ok(settings) = serde_json::from_str(&contents) {
            return settings;
        }
    }
    Settings::default()
}

fn save_settings(settings: &Settings) -> Result<(), String> {
    let config_dir = get_config_dir();
    fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let settings_path = config_dir.join("settings.json");
    let contents = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(settings_path, contents).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().await;
    Ok((*settings).clone())
}

#[tauri::command]
async fn save_qbittorrent_settings(
    state: State<'_, AppState>,
    config: QBittorrentConfig,
) -> Result<(), String> {
    // Try to connect first
    state.qb_client.connect(config.clone()).await.map_err(|e| e.to_string())?;
    
    // If connection successful, save settings
    let mut settings = state.settings.lock().await;
    settings.qbittorrent = Some(config);
    save_settings(&*settings)?;
    
    Ok(())
}

#[tauri::command]
async fn connect_qbittorrent(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> Result<(), String> {
    let config = QBittorrentConfig {
        url,
        username,
        password,
    };

    state
        .qb_client
        .connect(config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_qbittorrent_connection(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.qb_client.is_connected().await)
}

#[tauri::command]
async fn get_torrents(state: State<'_, AppState>) -> Result<Vec<TorrentInfo>, String> {
    state
        .qb_client
        .get_torrents()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_torrent(state: State<'_, AppState>, magnet_url: String) -> Result<(), String> {
    state
        .qb_client
        .add_torrent(&magnet_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_torrent(
    state: State<'_, AppState>,
    hash: String,
    delete_files: bool,
) -> Result<(), String> {
    state
        .qb_client
        .remove_torrent(&hash, delete_files)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pause_torrent(state: State<'_, AppState>, hash: String) -> Result<(), String> {
    state
        .qb_client
        .pause_torrent(&hash)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn resume_torrent(state: State<'_, AppState>, hash: String) -> Result<(), String> {
    state
        .qb_client
        .resume_torrent(&hash)
        .await
        .map_err(|e| e.to_string())
}

// RSS Commands
#[tauri::command]
async fn add_rss_feed(state: State<'_, AppState>, url: String) -> Result<(), String> {
    state
        .qb_client
        .add_rss_feed(&url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_rss_rules(state: State<'_, AppState>) -> Result<Vec<RssRule>, String> {
    state
        .qb_client
        .get_rss_rules()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_rss_rule(
    state: State<'_, AppState>,
    rule_name: String,
    rule_def: RssRuleInfo,
) -> Result<(), String> {
    state
        .qb_client
        .add_rss_rule(&rule_name, rule_def)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_rss_rule(state: State<'_, AppState>, rule_name: String) -> Result<(), String> {
    state
        .qb_client
        .remove_rss_rule(&rule_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_rss_items(state: State<'_, AppState>, feed_url: String) -> Result<Vec<RssArticle>, String> {
    state
        .qb_client
        .get_rss_items(&feed_url)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_available_anime(state: State<'_, AppState>) -> Result<Vec<AnimeInfo>, String> {
    state
        .anime_client
        .get_available_anime()
        .await
        .map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() {
    let settings = load_settings();
    let qb_client = Arc::new(QBittorrentClient::new());
    let anime_client = Arc::new(AnimeClient::new());
    
    // Try to connect if we have saved settings
    if let Some(config) = &settings.qbittorrent {
        let qb_client = qb_client.clone();
        let config = config.clone();
        let _ = qb_client.connect(config).await;
    }
    
    let app_state = AppState {
        qb_client: qb_client.clone(),
        settings: Arc::new(Mutex::new(settings)),
        anime_client: anime_client.clone(),
    };

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            connect_qbittorrent,
            check_qbittorrent_connection,
            get_torrents,
            add_torrent,
            remove_torrent,
            pause_torrent,
            resume_torrent,
            add_rss_feed,
            get_rss_rules,
            add_rss_rule,
            remove_rss_rule,
            get_rss_items,
            get_settings,
            save_qbittorrent_settings,
            get_available_anime,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
