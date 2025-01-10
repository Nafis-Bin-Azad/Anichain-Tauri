use crate::error::Error;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct QBittorrent {
    url: String,
    username: String,
    password: String,
    client: Client,
}

impl QBittorrent {
    pub fn new(url: String, username: String, password: String) -> Self {
        Self {
            url,
            username,
            password,
            client: Client::new(),
        }
    }

    pub async fn login(&self) -> Result<(), Error> {
        let form = [
            ("username", &self.username),
            ("password", &self.password),
        ];

        self.client
            .post(format!("{}/api/v2/auth/login", self.url))
            .form(&form)
            .send()
            .await?;

        Ok(())
    }
} 