use anyhow::{anyhow, Context, Result};
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;

// API endpoint constants
const API_BASE_PATH: &str = "/api/v2";
const AUTH_LOGIN: &str = "/auth/login";
const TORRENTS_INFO: &str = "/torrents/info";
const TORRENTS_ADD: &str = "/torrents/add";
const TORRENTS_DELETE: &str = "/torrents/delete";
const TORRENTS_PAUSE: &str = "/torrents/pause";
const TORRENTS_RESUME: &str = "/torrents/resume";
const RSS_ADD_FEED: &str = "/rss/addFeed";
const RSS_RULES: &str = "/rss/rules";
const RSS_SET_RULE: &str = "/rss/setRule";
const RSS_REMOVE_RULE: &str = "/rss/removeRule";
const RSS_ITEMS: &str = "/rss/items";
const APP_PREFERENCES: &str = "/app/setPreferences";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QBittorrentConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub download_directory: String,
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
pub struct RssRuleDefinition {
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
struct ConnectionState {
    base_url: String,
    username: String,
    password: String,
    is_authenticated: bool,
    config: QBittorrentConfig,
}

/// A client to interact with the qBittorrent API.
#[derive(Debug, Clone)]
pub struct QBittorrentClient {
    http_client: Arc<HttpClient>,
    state: Arc<Mutex<ConnectionState>>,
}

impl QBittorrentClient {
    /// Creates a new client instance with the given configuration.
    pub fn new(config: QBittorrentConfig) -> Result<Self> {
        let http_client = HttpClient::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            http_client: Arc::new(http_client),
            state: Arc::new(Mutex::new(ConnectionState {
                base_url: config.base_url.clone(),
                username: config.username.clone(),
                password: config.password.clone(),
                is_authenticated: false,
                config,
            })),
        })
    }

    async fn build_api_url(&self, endpoint: &str) -> Result<String> {
        let state = self.state.lock().await;
        Ok(format!("{}{}{}", state.base_url, API_BASE_PATH, endpoint))
    }

    async fn ensure_authenticated(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        if !state.is_authenticated {
            self.authenticate(&state.username, &state.password).await?;
            state.is_authenticated = true;
        }
        Ok(())
    }

    async fn authenticate(&self, username: &str, password: &str) -> Result<()> {
        let login_url = self.build_api_url(AUTH_LOGIN).await?;
        let response = self
            .http_client
            .post(&login_url)
            .form(&[("username", username), ("password", password)])
            .send()
            .await
            .context("Authentication request failed")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Authentication failed with status: {}",
                response.status()
            ));
        }

        Ok(())
    }

    // === Torrent Management Methods ===

    /// Retrieves the list of torrents.
    pub async fn get_torrents(&self) -> Result<Vec<TorrentInfo>> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(TORRENTS_INFO).await?;
        let torrents = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch torrents")?
            .json::<Vec<TorrentInfo>>()
            .await
            .context("Failed to parse torrents response")?;
        Ok(torrents)
    }

    /// Adds a torrent using the provided magnet URI.
    pub async fn add_torrent(&self, magnet_uri: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(TORRENTS_ADD).await?;
        let state = self.state.lock().await;
        self.http_client
            .post(&url)
            .form(&[
                ("urls", magnet_uri),
                ("savepath", &state.config.download_directory),
            ])
            .send()
            .await
            .context("Failed to add torrent")?;
        Ok(())
    }

    /// Removes a torrent by hash. Set `delete_files` to `true` to remove associated files.
    pub async fn remove_torrent(&self, hash: &str, delete_files: bool) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(TORRENTS_DELETE).await?;
        let delete_files_str = if delete_files { "true" } else { "false" };
        self.http_client
            .post(&url)
            .form(&[("hashes", hash), ("deleteFiles", delete_files_str)])
            .send()
            .await
            .context("Failed to remove torrent")?;
        Ok(())
    }

    /// Pauses the torrent with the specified hash.
    pub async fn pause_torrent(&self, hash: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(TORRENTS_PAUSE).await?;
        self.http_client
            .post(&url)
            .form(&[("hashes", hash)])
            .send()
            .await
            .context("Failed to pause torrent")?;
        Ok(())
    }

    /// Resumes the torrent with the specified hash.
    pub async fn resume_torrent(&self, hash: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(TORRENTS_RESUME).await?;
        self.http_client
            .post(&url)
            .form(&[("hashes", hash)])
            .send()
            .await
            .context("Failed to resume torrent")?;
        Ok(())
    }

    // === RSS Management Methods ===

    /// Adds a new RSS feed.
    pub async fn add_rss_feed(&self, feed_url: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(RSS_ADD_FEED).await?;
        self.http_client
            .post(&url)
            .form(&[("url", feed_url)])
            .send()
            .await
            .context("Failed to add RSS feed")?;
        Ok(())
    }

    /// Retrieves the list of RSS rules.
    pub async fn get_rss_rules(&self) -> Result<Vec<RssRule>> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(RSS_RULES).await?;
        let rules = self
            .http_client
            .get(&url)
            .send()
            .await
            .context("Failed to fetch RSS rules")?
            .json::<Vec<RssRule>>()
            .await
            .context("Failed to parse RSS rules response")?;
        Ok(rules)
    }

    /// Adds a new RSS rule.
    pub async fn add_rss_rule(&self, rule: RssRuleDefinition) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(RSS_SET_RULE).await?;
        let params = [
            ("ruleName", rule.name.as_str()),
            ("enabled", if rule.enabled { "true" } else { "false" }),
            ("mustContain", rule.must_contain.as_str()),
            ("mustNotContain", rule.must_not_contain.as_str()),
            ("feedURL", rule.feed_url.as_str()),
            ("savePath", rule.save_path.as_str()),
        ];
        self.http_client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to add RSS rule")?;
        Ok(())
    }

    /// Removes an RSS rule by name.
    pub async fn remove_rss_rule(&self, rule_name: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(RSS_REMOVE_RULE).await?;
        self.http_client
            .post(&url)
            .form(&[("ruleName", rule_name)])
            .send()
            .await
            .context("Failed to remove RSS rule")?;
        Ok(())
    }

    /// Retrieves RSS items for the given feed URL.
    pub async fn get_rss_items(&self, feed_url: &str) -> Result<Vec<RssArticle>> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(RSS_ITEMS).await?;
        let items = self
            .http_client
            .get(&url)
            .query(&[("withData", "true"), ("feedID", feed_url)])
            .send()
            .await
            .context("Failed to fetch RSS items")?
            .json::<Vec<RssArticle>>()
            .await
            .context("Failed to parse RSS items response")?;
        Ok(items)
    }

    // === Configuration Methods ===

    /// Returns the current download directory.
    pub async fn get_download_directory(&self) -> Result<String> {
        let state = self.state.lock().await;
        Ok(state.config.download_directory.clone())
    }

    /// Updates the download directory.
    pub async fn set_download_directory(&self, new_path: &str) -> Result<()> {
        self.ensure_authenticated().await?;
        let url = self.build_api_url(APP_PREFERENCES).await?;
        // Prepare the JSON payload as a string.
        let json_payload = json!({ "save_path": new_path }).to_string();
        self.http_client
            .post(&url)
            .form(&[("json", &json_payload)])
            .send()
            .await
            .context("Failed to update download directory")?;
        
        let mut state = self.state.lock().await;
        state.config.download_directory = new_path.to_string();
        Ok(())
    }
}
