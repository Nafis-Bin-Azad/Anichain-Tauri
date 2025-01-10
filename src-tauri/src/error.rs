#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("Failed to parse RSS feed: {0}")]
    RssFeedError(#[from] feed_rs::parser::ParseFeedError),
    #[error("Failed to fetch schedule: {0}")]
    ScheduleError(String),
    #[error("Failed to manage tracked anime: {0}")]
    TrackingError(String),
    #[error("Failed to manage qBittorrent rules: {0}")]
    RuleManagementError(String),
    #[error("qBittorrent client not initialized")]
    QBittorrentNotInitialized,
}

// Implement Serialize manually to ensure proper error serialization
impl serde::Serialize for Error {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Error", 2)?;
        state.serialize_field("error", &self.to_string())?;
        state.end()
    }
} 