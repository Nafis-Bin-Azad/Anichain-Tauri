use anyhow::{Result, anyhow};
use reqwest::Client as ReqwestClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QBittorrentConfig {
    pub url: String,
    pub username: String,
    pub password: String,
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

pub struct QBittorrentClient {
    client: ReqwestClient,
    base_url: Arc<Mutex<String>>,
    is_connected: Arc<Mutex<bool>>,
    username: Arc<Mutex<String>>,
    password: Arc<Mutex<String>>,
}

impl QBittorrentClient {
    pub fn new() -> Self {
        let client = ReqwestClient::builder()
            .cookie_store(true)
            .build()
            .unwrap();

        let instance = Self {
            client,
            base_url: Arc::new(Mutex::new("http://localhost:8080".to_string())),
            is_connected: Arc::new(Mutex::new(false)),
            username: Arc::new(Mutex::new("nafislord".to_string())),
            password: Arc::new(Mutex::new("Saphire 1".to_string())),
        };

        // Spawn a task to connect with hardcoded credentials
        tokio::spawn({
            let instance = instance.clone();
            async move {
                let config = QBittorrentConfig {
                    url: "http://localhost:8080".to_string(),
                    username: "nafislord".to_string(),
                    password: "Saphire 1".to_string(),
                };
                
                match instance.connect(config).await {
                    Ok(_) => {
                        tracing::info!("Successfully connected to qBittorrent");
                    }
                    Err(e) => {
                        if e.to_string().contains("banned") {
                            tracing::error!("IP has been banned. Please go to qBittorrent Web UI settings and remove the IP ban for localhost/127.0.0.1");
                        } else {
                            tracing::error!("Failed to connect to qBittorrent: {}", e);
                        }
                    }
                }
            }
        });

        instance
    }

    pub async fn connect(&self, config: QBittorrentConfig) -> Result<()> {
        // Store the credentials
        *self.base_url.lock().await = config.url.clone();
        *self.username.lock().await = config.username.clone();
        *self.password.lock().await = config.password.clone();

        // Try to authenticate
        let auth_url = format!("{}/api/v2/auth/login", config.url);
        let username = config.username.clone();
        let password = config.password.clone();
        
        // Build form data with proper field names
        let form = [
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];

        tracing::info!("Attempting to authenticate to qBittorrent at {}", auth_url);
        let auth_response = self.client.post(&auth_url)
            .form(&form)
            .send()
            .await?;

        let status = auth_response.status();
        if !status.is_success() {
            *self.is_connected.lock().await = false;
            let error = auth_response.text().await?;
            return Err(anyhow!("Authentication failed: {} - {}", status, error));
        }

        // Get the SID cookie
        let mut cookies = auth_response.cookies();
        if !cookies.any(|c| c.name() == "SID") {
            *self.is_connected.lock().await = false;
            return Err(anyhow!("No session cookie received from qBittorrent"));
        }

        // Test connection by getting app version
        let version_url = format!("{}/api/v2/app/version", config.url);
        tracing::info!("Testing connection by getting app version");
        let version_response = self.client.get(&version_url)
            .send()
            .await?;

        let status = version_response.status();
        if !status.is_success() {
            *self.is_connected.lock().await = false;
            let error = version_response.text().await?;
            return Err(anyhow!("Connection test failed: {} - {}", status, error));
        }

        // Set connected status
        *self.is_connected.lock().await = true;
        tracing::info!("Successfully connected to qBittorrent");
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

    pub async fn add_torrent(&self, magnet_url: &str, category: Option<&str>) -> Result<()> {
        // First check if we need to re-authenticate
        let base_url = self.base_url.lock().await.clone();
        let username = self.username.lock().await.clone();
        let password = self.password.lock().await.clone();
        
        // Try to authenticate first
        let auth_url = format!("{}/api/v2/auth/login", base_url);
        let form = [
            ("username", username.as_str()),
            ("password", password.as_str()),
        ];
        
        let auth_response = self.client.post(&auth_url)
            .form(&form)
            .send()
            .await?;

        if !auth_response.status().is_success() {
            return Err(anyhow!("Authentication failed: {}", auth_response.status()));
        }

        // Now add the torrent
        let url = format!("{}/api/v2/torrents/add", base_url);
        let mut form = vec![("urls", magnet_url)];
        
        if let Some(cat) = category {
            form.push(("category", cat));
        }

        let response = self.client.post(&url)
            .form(&form)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to add torrent: {} (Status: {})", error, status));
        }

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
}

impl Clone for QBittorrentClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            is_connected: self.is_connected.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
} 