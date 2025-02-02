// main.rs

// Import the modules we defined in separate files.
mod common;
mod animelists;
mod tvdb4;
mod thetvdbv2;
mod anidb;
mod themoviedb;
mod fanarttv;
mod plex;
mod tvtunes;
mod omdb;
mod myanimelist;
mod anilist;
mod local;
mod anidb34;

use anyhow::Result;
use common::{validate_prefs, get_plex_libraries, log_info, write_logs, update_meta, close_logs};
use animelists::{get_metadata as anime_lists_get_metadata, ani_db_movie_sets};
use anidb::{get_metadata as anidb_get_metadata, get_anidb_titles_db, search as anidb_search};
use tvdb4::get_metadata as tvdb4_get_metadata;
use thetvdbv2::{get_metadata as thetvdb_get_metadata, search as thetvdb_search};
use themoviedb::{get_metadata as themoviedb_get_metadata, search as themoviedb_search};
use fanarttv::get_metadata as fanarttv_get_metadata;
use plex::get_metadata as plex_get_metadata;
use tvtunes::get_metadata as tvtunes_get_metadata;
use omdb::get_metadata as omdb_get_metadata;
use myanimelist::{get_metadata as myanimelist_get_metadata};
use anilist::{get_metadata as anilist_get_metadata};
use local::get_metadata as local_get_metadata;
use anidb34::adjust_mapping;
use serde_json::json;
use serde_json::Value;

/// Performs initial startup tasks: validate preferences, load Plex library info,
/// and load core files from various sources.
async fn start() -> Result<()> {
    common::log_info("HTTP Anidb Metadata Agent by ZeroQI (Forked from Atomicstrawberry's v0.4, AnimeLists XMLs by ScudLee)");
    // Set cache time for HTTP requests (for example, 30 minutes).
    common::set_http_cache_time(common::CACHE_1_MINUTE * 30);
    validate_prefs()?;
    get_plex_libraries()?;
    // Load core files.
    animelists::get_anidb_tvdb_map()?;
    animelists::get_anidb_movie_sets()?;
    anidb::get_anidb_titles_db()?;
    Ok(())
}

/// Simulate a search call that gathers results from multiple sources.
async fn search(media: &Value, lang: &str, manual: bool, movie: bool) -> Result<()> {
    common::log_info("=== Search() ===");
    let orig_title = if movie {
        media.get("title").and_then(|v| v.as_str()).unwrap_or("")
    } else {
        media.get("show").and_then(|v| v.as_str()).unwrap_or("")
    };
    common::log_info(&format!("title: '{}', manual: '{}', year: '{}'", orig_title, manual, media.get("year").unwrap_or(&json!(null))));
    common::log_info(&format!("start: {}", chrono::Utc::now().to_rfc3339()));
    // (If a special "clear-cache" title is detected, clear the HTTP cache.)
    if orig_title == "clear-cache" {
        common::clear_cache();
        // In production, you would add a search result indicating cache clearance.
        return Ok(());
    }
    // Forced ID: check if the title contains a forced ID pattern.
    if let Some(forced) = common::extract_forced_id(orig_title) {
        // Append a forced result.
        common::append_search_result(&json!({
            "id": forced,
            "name": format!("{} [{}]", common::extract_show_name(orig_title), forced),
            "year": media.get("year"),
            "lang": lang,
            "score": 100
        }));
        common::log_info(&format!("Forced ID found: {}", forced));
    } else {
        let mut max_score = 0;
        let mut n = 0;
        // For movies or single–season shows, search via AniDB.
        if movie || common::max_season(media)? <= 1 {
            let (score, count) = anidb_search(media, lang, manual, movie).await?;
            max_score = score;
            n = count;
        }
        // If the score is low and movie, try TheMovieDb.
        if max_score < 50 && movie {
            let score = themoviedb_search(media, lang, manual, movie).await?;
            if score > max_score { max_score = score; }
        }
        // For series, if score is low or if there are multiple season matches, try TheTVDBv2.
        if (max_score < 80 && !movie) || n > 1 {
            let score = thetvdb_search(media, lang, manual, movie).await?;
            if score > max_score { max_score = score; }
        }
        common::log_info(&format!("Search complete, max score: {}", max_score));
    }
    common::log_info(&format!("end: {}", chrono::Utc::now().to_rfc3339()));
    common::close_logs();
    Ok(())
}

/// Update metadata by gathering data from multiple modules and then updating the unified metadata.
/// This function calls GetMetadata from various modules, then writes logs and updates metadata fields.
async fn update(metadata: &mut Value, media: &Value, lang: &str, force: bool, movie: bool) -> Result<()> {
    common::log_info("=== Update() ===");
    common::log_info(&format!("id: {}, title: {}, lang: {}, force: {}, movie: {}", 
        metadata.get("id").unwrap_or(&json!("")), 
        metadata.get("title").unwrap_or(&json!("")), 
        lang, force, movie));
    common::log_info(&format!("start: {}", chrono::Utc::now().to_rfc3339()));
    
    // Create an error_log as a JSON object.
    let mut error_log = json!({
        "AniDB summaries missing": [],
        "AniDB posters missing": [],
        "anime-list AniDBid missing": [],
        "anime-list studio logos": [],
        "TVDB posters missing": [],
        "TVDB season posters missing": [],
        "anime-list TVDBid missing": [],
        "Plex themes missing": [],
        "Missing Episodes": [],
        "Missing Specials": [],
        "Missing Episode Summaries": [],
        "Missing Special Summaries": []
    });
    
    // Call each module's GetMetadata function:
    let (dict_anime_lists, AniDBid, TVDBid, TMDbid, IMDbid, mut mapping_list) =
        animelists::get_metadata(media, movie, &mut error_log, metadata.get("id").and_then(|v| v.as_str()).unwrap_or("")).await?;
    let dict_tvdb4 = tvdb4::get_metadata(media, movie, "tvdb4", TVDBid, &mut mapping_list)?;
    let (dict_thetvdb, IMDbid) =
        thetvdbv2::get_metadata(media, movie, &mut error_log, lang, "source", &AniDBid, TVDBid, IMDbid, &mut mapping_list).await?;
    let (dict_anidb, ANNid, MALids) =
        anidb::get_metadata(media, movie, &mut error_log, "source", &AniDBid, TVDBid, ani_db_movie_sets(), &mut mapping_list).await?;
    let (dict_themoviedb, TSDbid, TMDbid, IMDbid) =
        themoviedb::get_metadata(media, movie, TVDBid, TMDbid, IMDbid).await?;
    let dict_fanarttv = fanarttv::get_metadata(movie, TVDBid, TMDbid, IMDbid).await?;
    let dict_plex = plex::get_metadata(metadata, &mut error_log, TVDBid, dict_thetvdb.get("title").and_then(|v| v.as_str()).unwrap_or(""))?;
    let dict_tvtunes = tvtunes::get_metadata(metadata, dict_thetvdb.get("title").and_then(|v| v.as_str()).unwrap_or(""), dict(&mapping_list, &[&AniDBid, "name"]).as_str().unwrap_or(""))?;
    let dict_omdb = omdb::get_metadata(movie, IMDbid).await?;
    let (dict_myanimelist, MainMALid) =
        myanimelist::get_metadata(&MALids, if movie { "movie" } else { "tv" }, &dict_anidb).await?;
    let dict_anilist = anilist::get_metadata(&AniDBid, &MainMALid).await?;
    let dict_local = local::get_metadata(media, movie)?;
    
    // Optionally adjust mapping if needed.
    if anidb34::adjust_mapping("source", &mut mapping_list, &dict_anidb, &dict_thetvdb, &dict_fanarttv) {
        let (_new_dict_anidb, _ANNid, _MALids) =
            anidb::get_metadata(media, movie, &mut error_log, "source", &AniDBid, TVDBid, ani_db_movie_sets(), &mut mapping_list).await?;
    }
    
    common::log_info(&format!("=== Update() ==="));
    common::log_info(&format!("AniDBid: '{}', TVDBid: '{}', TMDbid: '{}', IMDbid: '{}', ANNid: '{}', MALid: '{}'", AniDBid, TVDBid, TMDbid, IMDbid, ANNid, MainMALid));
    write_logs(media, movie, &mut error_log, "source", &AniDBid, TVDBid);
    update_meta(metadata, media, movie, &json!({
        "AnimeLists": dict_anime_lists,
        "AniDB": dict_anidb,
        "TheTVDB": dict_thetvdb,
        "TheMovieDb": dict_themoviedb,
        "FanartTV": dict_fanarttv,
        "tvdb4": dict_tvdb4,
        "Plex": dict_plex,
        "TVTunes": dict_tvtunes,
        "OMDb": dict_omdb,
        "Local": dict_local,
        "AniList": dict_anilist,
        "MyAnimeList": dict_myanimelist
    }), &mut mapping_list);
    common::log_info(&format!("end: {}", chrono::Utc::now().to_rfc3339()));
    close_logs();
    Ok(())
}

/// Main entrypoint – the Rust “init” function that ties all modules together.
/// In production this might be integrated into your Tauri backend or Plex agent framework.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (e.g. with env_logger).
    env_logger::init();
    
    // Run startup: validate preferences, load libraries, and core files.
    start().await?;
    
    // Simulate a media object. In production, media is supplied by Plex.
    let media = json!({
        "dir": "/path/to/media/Some Show",
        "title": "Some Show",
        "show": "Some Show",
        "year": 2020,
        "seasons": {
            "1": {
                "episodes": {
                    "1": { "file": "/path/to/media/Some Show/Season 1/Episode 1.mkv" },
                    "2": { "file": "/path/to/media/Some Show/Season 1/Episode 2.mkv" }
                }
            }
        }
    });
    
    // Simulate metadata as a mutable JSON object (initially containing an id and title).
    let mut metadata = json!({
        "id": "anidb-12345",
        "title": "Some Show"
    });
    
    // Simulate a search call.
    search(&media, "en", false, false).await?;
    
    // Simulate an update call.
    update(&mut metadata, &media, "en", false, false).await?;
    
    // Log that the agents are ready (in production these would be exposed to Plex).
    common::log_info("Agents initialized: HamaTVAgent and HamaMovieAgent.");
    
    Ok(())
}
