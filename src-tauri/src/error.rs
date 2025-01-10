#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to initialize qBittorrent client: {0}")]
    QBittorrentInitError(String),
    #[error("Failed to fetch RSS feed: {0}")]
    RssFeedError(String),
    #[error("Failed to fetch schedule: {0}")]
    ScheduleError(String),
    #[error("Failed to manage tracked anime: {0}")]
    TrackingError(String),
    #[error("Failed to manage qBittorrent rules: {0}")]
    RuleManagementError(String),
    #[error("qBittorrent client not initialized")]
    QBittorrentNotInitialized,
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::QBittorrentInitError(err.to_string())
    }
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