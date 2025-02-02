// main.rs

use anyhow::Result;
use chrono::Utc;
use env_logger;
use log::{info, error};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

// Assume that these modules have been implemented in separate files:
// - common: shared helper functions and globals
// - animelists, tvdb4, thetvdbv2, anidb, themoviedb, fanarttv, plex, tvtunes, omdb, myanimelist, anilist, local, anidb34
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

use common::{
    close_logs, get_plex_libraries, log_info, set_http_cache_time, validate_prefs, CACHE_1_MINUTE,
};
use animelists::{get_metadata as anime_lists_get_metadata, get_anidb_tvdb_map, get_anidb_movie_sets};
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

/// Startup tasks: validate preferences, load Plex library info, load core files.
async fn start() -> Result<()> {
    log_info("HTTP Anidb Metadata Agent by ZeroQI (Forked from Atomicstrawberry's v0.4, AnimeLists XMLs by ScudLee)");
    // Set HTTP cache time (here 30 minutes).
    set_http_cache_time(CACHE_1_MINUTE * 30);
    validate_prefs()?;
    get_plex_libraries()?;
    // Load core mapping and title files.
    anime_lists_get_metadata().await?; // Loads AniDB-TVDB mapping
    get_anidb_movie_sets()?;
    anidb_get_metadata::get_anidb_titles_db().await?;
    Ok(())
}

/// Simulate a search call that calls the search functions from various modules.
async fn search(media: &Value, lang: &str, manual: bool, movie: bool) -> Result<()> {
    log_info("=== Search() ===");
    // For movies, use media.title; for series, use media.show.
    let orig_title = if movie {
        media.get("title").and_then(|v| v.as_str()).unwrap_or("")
    } else {
        media.get("show").and_then(|v| v.as_str()).unwrap_or("")
    };
    log_info(&format!("Search title: '{}', manual: '{}', year: '{}'", orig_title, manual, media.get("year").unwrap_or(&json!(null))));
    log_info(&format!("start: {}", Utc::now().to_rfc3339()));
    
    // Check for a forced ID pattern (if present, extract and append a result).
    if let Some(forced) = common::extract_forced_id(orig_title) {
        common::append_search_result(&json!({
            "id": forced,
            "name": format!("{} [{}]", common::extract_show_name(orig_title), forced),
            "year": media.get("year"),
            "lang": lang,
            "score": 100
        }));
        log_info(&format!("Forced ID found: {}", forced));
    } else {
        let mut max_score = 0;
        let mut n = 0;
        // If movie or single–season series, use AniDB search.
        if movie || common::max_season(media)? <= 1 {
            let (score, count) = anidb_search(media, lang, manual, movie).await?;
            max_score = score;
            n = count;
        }
        // If score is low and movie, try TheMovieDb.
        if max_score < 50 && movie {
            let score = themoviedb_search(media, lang, manual, movie).await?;
            if score > max_score { max_score = score; }
        }
        // For series, if score is low or there are multiple season matches, try TheTVDBv2.
        if (!movie && max_score < 80) || (n > 1) {
            let score = thetvdb_search(media, lang, manual, movie).await?;
            if score > max_score { max_score = score; }
        }
        log_info(&format!("Search complete, max score: {}", max_score));
    }
    log_info(&format!("end: {}", Utc::now().to_rfc3339()));
    common::close_logs();
    Ok(())
}

/// Update metadata by gathering data from all modules and then updating the unified metadata.
async fn update(metadata: &mut Value, media: &Value, lang: &str, force: bool, movie: bool) -> Result<()> {
    common::log_info("=== Update() ===");
    common::log_info(&format!(
        "id: {}, title: {}, lang: {}, force: {}, movie: {}",
        metadata.get("id").unwrap_or(&json!("")),
        metadata.get("title").unwrap_or(&json!("")),
        lang, force, movie
    ));
    common::log_info(&format!("start: {}", Utc::now().to_rfc3339()));
    
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
    
    // Call each module's GetMetadata function.
    let (dict_anime_lists, AniDBid, TVDBid, TMDbid, IMDbid, mut mapping_list) =
        animelists::get_metadata(media, movie, &mut error_log, metadata.get("id").and_then(|v| v.as_str()).unwrap_or("")).await?;
    let dict_tvdb4 = tvdb4_get_metadata(media, movie, "tvdb4", TVDBid, &mut mapping_list)?;
    let (dict_thetvdb, IMDbid) =
        thetvdb_get_metadata(media, movie, &mut error_log, lang, "source", &AniDBid, TVDBid, IMDbid, &mut mapping_list).await?;
    let (dict_anidb, ANNid, MALids) =
        anidb::get_metadata(media, movie, &mut error_log, "source", &AniDBid, TVDBid, animelists::ani_db_movie_sets(), &mut mapping_list).await?;
    let (dict_themoviedb, TSDbid, TMDbid, IMDbid) =
        themoviedb_get_metadata(media, movie, TVDBid, TMDbid, IMDbid).await?;
    let dict_fanarttv = fanarttv_get_metadata(movie, TVDBid, TMDbid, IMDbid).await?;
    let dict_plex = plex_get_metadata(metadata, &mut error_log, TVDBid, dict_thetvdb.get("title").and_then(|v| v.as_str()).unwrap_or(""))?;
    let dict_tvtunes = tvtunes_get_metadata(metadata, dict_thetvdb.get("title").and_then(|v| v.as_str()).unwrap_or(""), dict(&mapping_list, &[&AniDBid, "name"]).as_str().unwrap_or(""))?;
    let dict_omdb = omdb_get_metadata(movie, IMDbid).await?;
    let (dict_myanimelist, MainMALid) =
        myanimelist_get_metadata(&MALids, if movie { "movie" } else { "tv" }, &dict_anidb).await?;
    let dict_anilist = anilist_get_metadata(&AniDBid, &MainMALid).await?;
    let dict_local = local_get_metadata(media, movie)?;
    
    // Optionally adjust mapping if required.
    if anidb34::adjust_mapping("source", &mut mapping_list, &dict_anidb, &dict_thetvdb, &dict_fanarttv) {
        let (_new_dict_anidb, _ANNid, _MALids) =
            anidb::get_metadata(media, movie, &mut error_log, "source", &AniDBid, TVDBid, animelists::ani_db_movie_sets(), &mut mapping_list).await?;
    }
    
    common::log_info("=== Update() ===");
    common::log_info(&format!(
        "AniDBid: '{}', TVDBid: '{}', TMDbid: '{}', IMDbid: '{}', ANNid: '{}', MALid: '{}'",
        AniDBid, TVDBid, TMDbid, IMDbid, ANNid, MainMALid
    ));
    common::write_logs(media, movie, &mut error_log, "source", &AniDBid, TVDBid);
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
    common::log_info(&format!("end: {}", Utc::now().to_rfc3339()));
    common::close_logs();
    Ok(())
}

/// A simplified scanner function that scans a given directory (similar to AbsoluteSeriesScanner.py)
fn scan_directory(path: &str) -> Result<Vec<String>> {
    use std::fs;
    use std::path::Path;
    use common::{cleanse_title, natural_sort_key};
    let mut media_files = Vec::new();
    let p = Path::new(path);
    if p.is_dir() {
        for entry in fs::read_dir(p)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                // Optionally, recursively scan subdirectories.
                let sub_files = scan_directory(entry_path.to_str().unwrap())?;
                media_files.extend(sub_files);
            } else {
                // Filter files by extension.
                if let Some(ext) = entry_path.extension().and_then(|s| s.to_str()) {
                    if common::VIDEO_EXTS().contains(&ext.to_lowercase().as_str()) {
                        media_files.push(entry_path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    media_files.sort_by(|a, b| {
        let key_a = natural_sort_key(a);
        let key_b = natural_sort_key(b);
        key_a.cmp(&key_b)
    });
    Ok(media_files)
}

/// Main entrypoint: ties together initialization, scanning, search, and update.
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging.
    env_logger::init();
    
    // Run startup tasks.
    start().await?;
    
    // For demonstration, if a command-line argument is provided, treat it as the path to scan.
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <path_to_scan>", args[0]);
        return Ok(());
    }
    let scan_path = &args[1];
    common::log_info(&format!("Scanning directory: {}", scan_path));
    let files = scan_directory(scan_path)?;
    common::log_info(&format!("Files detected: {:?}", files));
    
    // Create a dummy media object.
    let media = json!({
        "dir": scan_path,
        "title": "Some Show",
        "show": "Some Show",
        "year": 2020,
        "seasons": { "1": { "episodes": { "1": { "file": files.get(0).unwrap_or(&"".to_string()) } } } }
    });
    
    // Simulate a search call.
    search(&media, "en", false, false).await?;
    
    // Simulate an update call (the metadata object is updated in place).
    let mut metadata = json!({
        "id": "anidb-12345",
        "title": "Some Show"
    });
    update(&mut metadata, &media, "en", false, false).await?;
    
    // For demonstration, print the list of scanned files.
    println!("Scanned files:");
    for file in files {
        println!("{}", file);
    }
    
    Ok(())
}
