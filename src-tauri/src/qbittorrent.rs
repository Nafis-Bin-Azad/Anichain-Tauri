use crate::error::Error;
use reqwest::{Client, ClientBuilder};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct QBittorrent {
    client: Client,
    base_url: String,
}

impl QBittorrent {
    pub fn new(url: String, _username: String, _password: String) -> Self {
        Self {
            client: ClientBuilder::new()
                .cookie_store(true)
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: url.trim_end_matches('/').to_string(),
        }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<(), Error> {
        let form = HashMap::from([
            ("username", username),
            ("password", password),
        ]);

        self.client
            .post(format!("{}/api/v2/auth/login", self.base_url))
            .form(&form)
            .send()
            .await?;

        Ok(())
    }

    pub async fn add_rss_rule(&self, name: &str, pattern: &str, save_path: &str) -> Result<(), Error> {
        let mut form = HashMap::new();
        let rule_def = format!(
            r#"{{"enabled":true,"mustContain":"{}","savePath":"{}"}}"#,
            pattern, save_path
        );
        
        form.insert("ruleDef", rule_def.to_string());
        form.insert("ruleName", name.to_string());
        
        self.client
            .post(&format!("{}/api/v2/rss/setRule", self.base_url))
            .form(&form)
            .send()
            .await?;
        
        Ok(())
    }

    pub async fn remove_rss_rule(&self, name: &str) -> Result<(), Error> {
        let mut form = HashMap::new();
        form.insert("ruleName", name);

        self.client
            .post(format!("{}/api/v2/rss/removeRule", self.base_url))
            .form(&form)
            .send()
            .await?;

        Ok(())
    }
} 