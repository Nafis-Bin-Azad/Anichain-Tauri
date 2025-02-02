// src/models.rs

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Represents an anime series with metadata and seasons.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Anime {
    /// A unique identifier for the anime (typically taken from AniDB, TVDB, etc.).
    pub id: String,
    /// The primary title of the anime.
    pub title: String,
    /// An optional alternate or original title.
    pub original_title: Option<String>,
    /// The year of release (if available).
    pub year: Option<i32>,
    /// A short description or summary of the anime.
    pub summary: Option<String>,
    /// A list of seasons that the anime contains.
    pub seasons: Vec<Season>,
    /// A catch-all field to store unified metadata from various sources.
    pub meta: Option<Value>,
}

/// Represents a season within an anime series.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Season {
    /// The season number (for example, 1 for Season 1, 0 for Specials).
    pub season_number: i32,
    /// A list of episodes in this season.
    pub episodes: Vec<Episode>,
}

/// Represents a single episode within a season.
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Episode {
    /// The episode number within the season.
    pub episode_number: i32,
    /// The title of the episode.
    pub title: String,
    /// An optional file path to the local video file.
    pub file: Option<String>,
    /// An optional release date as a string (e.g., "2020-05-21").
    pub released_at: Option<String>,
}

impl Anime {
    /// Builds an `Anime` instance from a unified metadata JSON value.
    ///
    /// This function attempts to extract the anime ID from the "AniDB" source,
    /// the title and summary from the "TheTVDB" source, and the year from AniDB.
    ///
    /// # Arguments
    ///
    /// * `meta` - A reference to a `serde_json::Value` that contains unified metadata.
    ///
    /// # Returns
    ///
    /// An `Anime` instance built from the metadata.
    pub fn from_metadata(meta: &Value) -> Self {
        let id = meta
            .get("AniDB")
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = meta
            .get("TheTVDB")
            .and_then(|v| v.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let year = meta
            .get("AniDB")
            .and_then(|v| v.get("year"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32);

        let summary = meta
            .get("TheTVDB")
            .and_then(|v| v.get("summary"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Anime {
            id,
            title,
            original_title: None,
            year,
            summary,
            seasons: Vec::new(),
            meta: Some(meta.clone()),
        }
    }

    /// Creates an `Anime` instance from a search result JSON value.
    ///
    /// This function extracts the ID and name from the search result.
    ///
    /// # Arguments
    ///
    /// * `result` - A `serde_json::Value` representing a search result.
    ///
    /// # Returns
    ///
    /// An `Anime` instance populated with data from the search result.
    pub fn from_search_result(result: Value) -> Self {
        let id = result
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let title = result
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Anime {
            id,
            title,
            original_title: None,
            year: None,
            summary: None,
            seasons: Vec::new(),
            meta: Some(result),
        }
    }
}
