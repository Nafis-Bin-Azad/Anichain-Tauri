use anyhow::{Result, anyhow};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QBittorrentConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub download_folder: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TorrentInfo {
    pub name: String,
    pub size: u64,
    pub progress: f64,
    pub download_speed: u64,
    pub state: String,
    pub hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RssRuleInfo {
    pub name: String,
    pub enabled: bool,
    pub must_contain: String,
    pub must_not_contain: String,
    pub feed_url: String,
    pub save_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RssRule {
    pub enabled: bool,
    pub must_contain: String,
    pub must_not_contain: String,
    pub save_path: String,
    pub feed_urls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RssArticle {
    pub title: String,
    pub description: String,
    pub link: String,
    pub date: String,
    pub id: String,
}

#[derive(Debug, Clone)]
pub struct QBittorrentClient {
    client: Arc<ReqwestClient>,
    base_url: Arc<Mutex<String>>,
    is_connected: Arc<Mutex<bool>>,
    username: Arc<Mutex<String>>,
    password: Arc<Mutex<String>>,
    config: Arc<Mutex<QBittorrentConfig>>,
}

impl QBittorrentClient {
    pub fn new() -> Self {
        Self {
            client: Arc::new(ReqwestClient::builder()
                .cookie_store(true)
                .build()
                .unwrap()),
            base_url: Arc::new(Mutex::new(String::new())),
            is_connected: Arc::new(Mutex::new(false)),
            username: Arc::new(Mutex::new(String::new())),
            password: Arc::new(Mutex::new(String::new())),
            config: Arc::new(Mutex::new(QBittorrentConfig {
                url: String::new(),
                username: String::new(),
                password: String::new(),
                download_folder: String::new(),
            })),
        }
    }

    pub async fn connect(&self, config: QBittorrentConfig) -> Result<()> {
        let mut base_url = self.base_url.lock().await;
        *base_url = config.url.clone();

        let mut username = self.username.lock().await;
        *username = config.username.clone();

        let mut password = self.password.lock().await;
        *password = config.password.clone();

        let mut stored_config = self.config.lock().await;
        *stored_config = config.clone();

        // Clone the values before using them in the form
        let username_str = username.clone();
        let password_str = password.clone();
        let base_url_str = base_url.clone();

        // Try to login
        let form = [("username", username_str.as_str()), ("password", password_str.as_str())];
        let response = self.client
            .post(format!("{}/api/v2/auth/login", base_url_str))
            .form(&form)
            .send()
            .await?;

        if !response.status().is_success() {
            let mut is_connected = self.is_connected.lock().await;
            *is_connected = false;
            return Err(anyhow!("Failed to login: {}", response.status()));
        }

        let mut is_connected = self.is_connected.lock().await;
        *is_connected = true;

        Ok(())
    }

    pub async fn is_connected(&self) -> bool {
        *self.is_connected.lock().await
    }

    pub async fn get_torrents(&self) -> Result<Vec<TorrentInfo>> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/torrents/info", base_url);
        let response = self.client.get(&url).send().await?;
        let torrents = response.json().await?;
        Ok(torrents)
    }

    pub async fn add_torrent(&self, magnet_url: &str) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/torrents/add", base_url);
        
        let config = self.config.lock().await;
        let params = [
            ("urls", magnet_url),
            ("savepath", &config.download_folder),
        ];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn remove_torrent(&self, hash: &str, delete_files: bool) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/torrents/delete", base_url);
        let params = [
            ("hashes", hash.to_string()),
            ("deleteFiles", delete_files.to_string()),
        ];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn pause_torrent(&self, hash: &str) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/torrents/pause", base_url);
        let params = [("hashes", hash)];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn resume_torrent(&self, hash: &str) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/torrents/resume", base_url);
        let params = [("hashes", hash)];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    // RSS Functions
    pub async fn add_rss_feed(&self, url: &str) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let api_url = format!("{}/api/v2/rss/addFeed", base_url);
        let params = [("url", url)];

        self.client.post(&api_url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_rss_rules(&self) -> Result<Vec<RssRule>> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/rss/rules", base_url);
        let response = self.client.get(&url).send().await?;
        let rules = response.json().await?;
        Ok(rules)
    }

    pub async fn add_rss_rule(&self, rule_name: &str, rule_def: RssRuleInfo) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/rss/setRule", base_url);
        let params = [
            ("ruleName", rule_name.to_string()),
            ("enabled", rule_def.enabled.to_string()),
            ("mustContain", rule_def.must_contain),
            ("mustNotContain", rule_def.must_not_contain),
            ("feedURL", rule_def.feed_url),
            ("savePath", rule_def.save_path),
        ];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn remove_rss_rule(&self, rule_name: &str) -> Result<()> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/rss/removeRule", base_url);
        let params = [("ruleName", rule_name)];

        self.client.post(&url)
            .form(&params)
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_rss_items(&self, feed_url: &str) -> Result<Vec<RssArticle>> {
        let base_url = self.base_url.lock().await;
        let url = format!("{}/api/v2/rss/items", base_url);
        let params = [("withData", "true"), ("feedID", feed_url)];

        let response = self.client.get(&url)
            .query(&params)
            .send()
            .await?;
        let items = response.json().await?;
        Ok(items)
    }

    pub async fn get_download_folder(&self) -> Result<String, String> {
        tracing::info!("Retrieving download folder from qBittorrent configuration");
        let config = self.config.lock().await;
        tracing::info!("Download folder from config: {}", config.download_folder);
        Ok(config.download_folder.clone())
    }

    pub async fn set_download_folder(&self, folder: String) -> Result<(), String> {
        tracing::info!("Attempting to set qBittorrent download folder to: {}", folder);
        let mut config = self.config.lock().await;
        
        // Update the download folder in qBittorrent
        let preferences = json!({
            "save_path": &folder
        });
        
        let url = format!("{}/api/v2/app/setPreferences", config.url);
        tracing::info!("Sending request to update qBittorrent preferences at: {}", url);
        
        match self.client.post(&url)
            .form(&[("json", preferences.to_string())])
            .send()
            .await {
                Ok(_) => {
                    tracing::info!("Successfully updated qBittorrent preferences");
                    tracing::info!("Updating local config with new download folder: {}", folder);
                    config.download_folder = folder;
                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to update qBittorrent preferences: {}", e);
                    Err(format!("Failed to set download folder: {}", e))
                }
            }
    }
} 