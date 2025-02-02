// src/api_fetcher.rs

use anyhow::Result;
use serde_json::{json, Value};
use log::info;
use crate::api_providers::{anilist, myanimelist, anidb, tvdb, themoviedb, omdb};

/// Fetch metadata from all online providers and merge into one JSON object.
pub async fn fetch_all_metadata(id: &str, movie: bool) -> Result<Value> {
    info!("Fetching metadata for id: {}", id);
    let mut unified = serde_json::Map::new();

    let anidb_meta = anidb::get_metadata(id, movie).await?;
    unified.insert("AniDB".to_string(), anidb_meta);

    let tvdb_meta = tvdb::get_metadata(id, movie).await?;
    unified.insert("TheTVDB".to_string(), tvdb_meta);

    let themoviedb_meta = themoviedb::get_metadata(id, movie).await?;
    unified.insert("TheMovieDb".to_string(), themoviedb_meta);

    let omdb_meta = omdb::get_metadata(movie, id).await?;
    unified.insert("OMDb".to_string(), omdb_meta);

    let anilist_meta = anilist::get_metadata(id, "").await?;
    unified.insert("AniList".to_string(), anilist_meta);

    let myanimelist_meta = myanimelist::get_metadata(id, movie).await?;
    unified.insert("MyAnimeList".to_string(), myanimelist_meta);

    info!("Fetched metadata from all providers.");
    Ok(Value::Object(unified))
}

/// Search for anime using all providers and merge results.
pub async fn search_all(query: &str, lang: &str, manual: bool, movie: bool) -> Result<Vec<crate::models::Anime>> {
    let mut results = Vec::new();
    // For simplicity, we call tvdb search.
    let tvdb_results = tvdb::search(query, lang, manual, movie).await?;
    results.extend(tvdb_results.into_iter().map(crate::models::Anime::from_search_result));
    Ok(results)
}

/// Fetch airing schedule from a provider.
pub async fn fetch_schedule() -> Result<Value> {
    let schedule = tvdb::fetch_schedule().await?;
    Ok(schedule)
}
