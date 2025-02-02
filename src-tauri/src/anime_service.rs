// src/anime_service.rs

use anyhow::Result;
use serde_json::{json, Value};
use log::info;

use crate::api_fetcher;
use crate::file_scanner;
use crate::downloader;
use crate::database;
use crate::models::Anime;
use crate::config::Config;

/// Performs startup tasks such as validating preferences,
/// loading Plex library information, and initializing core files.
/// (Additional startup logic may be added as needed.)
pub async fn startup() -> Result<()> {
    info!("Running startup tasks...");
    // For example, you might load configuration, initialize the database,
    // load Plex libraries, or load core mapping files here.
    // In this stub, we simply log that startup is complete.
    Ok(())
}

/// Retrieves an anime record by its unique identifier.
/// If the anime is not found in the local database, it will be fetched
/// from online APIs and then saved into the database.
///
/// # Arguments
///
/// * `id` - A string slice that holds the anime ID.
///
/// # Returns
///
/// * `Ok(Anime)` on success.
/// * An error wrapped in `Result` if any step fails.
pub async fn get_anime_by_id(id: &str) -> Result<Anime> {
    // First, try to load the anime from the local database.
    if let Some(anime) = database::get_anime_by_id(id).await? {
        return Ok(anime);
    }
    // If not found locally, fetch unified metadata from online sources.
    // The second parameter here (false) indicates that we are not processing a movie.
    let unified_metadata = api_fetcher::fetch_all_metadata(id, false).await?;
    // Convert the unified metadata (a JSON Value) into our Anime model.
    let anime = Anime::from_metadata(&unified_metadata);
    // Save the newly fetched anime into the database for future use.
    database::save_anime(&anime).await?;
    Ok(anime)
}

/// Searches for anime based on a query string, language, manual flag,
/// and whether the query is for a movie.
///
/// # Arguments
///
/// * `query`  - The search string.
/// * `lang`   - Language code (e.g. "en").
/// * `manual` - Whether this is a manual search.
/// * `movie`  - Whether the search should be limited to movies.
///
/// # Returns
///
/// A vector of [`Anime`] objects representing the search results.
pub async fn search_anime(query: &str, lang: &str, manual: bool, movie: bool) -> Result<Vec<Anime>> {
    let results = api_fetcher::search_all(query, lang, manual, movie).await?;
    Ok(results)
}

/// Initiates the download of an anime via the downloader module.
/// For example, this could trigger a torrent download via qbittorrent.
///
/// # Arguments
///
/// * `anime_id` - A string slice representing the anime ID to download.
///
/// # Returns
///
/// An empty `Result` on success.
pub async fn download_anime(anime_id: &str) -> Result<()> {
    downloader::start_download(anime_id).await
}

/// Fetches the airing schedule for anime (e.g. from an API provider)
/// and returns it as a JSON value.
///
/// # Returns
///
/// A JSON value containing the schedule.
pub async fn fetch_schedule() -> Result<Value> {
    let schedule = api_fetcher::fetch_schedule().await?;
    Ok(schedule)
}
