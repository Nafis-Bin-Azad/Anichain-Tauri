use std::{
    collections::HashSet,
    fs,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Result};
use directories::ProjectDirs;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool},
    Row,
};
use tracing::{error, info, warn};

use crate::{
    hama::HamaMetadata,
    qbittorrent::QBittorrentConfig,
};

/// Initializes the database by ensuring the data directory exists,
/// creating a SQLite connection pool, and setting up necessary tables.
/// It also inserts default qBittorrent settings if none exist.
pub async fn init_db() -> Result<SqlitePool> {
    // Get the application’s project directories.
    let project_dirs = ProjectDirs::from("com", "nafislord", "anichain")
        .ok_or_else(|| anyhow!("Failed to get project directories"))?;
    let data_dir = project_dirs.data_dir();

    // Create the data directory if it does not exist.
    fs::create_dir_all(data_dir)?;

    // Build the database file path.
    let db_path = data_dir.join("anichain.db");
    info!("Using database at: {}", db_path.display());

    // Connect to the database with the `create_if_missing` option enabled.
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal),
    )
    .await?;

    // Create the qbittorrent settings table.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS qbittorrent_settings (
            id INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            download_folder TEXT NOT NULL DEFAULT 'downloads'
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Create the anime metadata table.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS anime_metadata (
            title TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            metadata TEXT NOT NULL,
            last_modified INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE(title)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Check if qBittorrent settings already exist.
    let count: (i32,) = sqlx::query_as("SELECT COUNT(*) FROM qbittorrent_settings")
        .fetch_one(&pool)
        .await?;

    if count.0 == 0 {
        info!("No existing qBittorrent settings found, inserting defaults...");
        let default_config = QBittorrentConfig {
            url: "http://127.0.0.1:8080".to_string(),
            username: "nafislord".to_string(),
            password: "Saphire 1".to_string(),
            download_folder: "downloads".to_string(),
        };
        save_qbittorrent_config(&pool, &default_config).await?;
        info!("Default qBittorrent settings saved successfully");
    }

    Ok(pool)
}

/// Saves the given qBittorrent configuration to the database.
pub async fn save_qbittorrent_config(pool: &SqlitePool, config: &QBittorrentConfig) -> Result<()> {
    info!("Saving qBittorrent config to database...");
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO qbittorrent_settings (id, url, username, password, download_folder)
        VALUES (1, ?, ?, ?, ?)
        "#,
    )
    .bind(&config.url)
    .bind(&config.username)
    .bind(&config.password)
    .bind(&config.download_folder)
    .execute(pool)
    .await?;
    info!("qBittorrent config saved successfully");
    Ok(())
}

/// Retrieves the qBittorrent configuration from the database, if it exists.
pub async fn get_qbittorrent_config(pool: &SqlitePool) -> Result<Option<QBittorrentConfig>> {
    info!("Retrieving qBittorrent config from database...");
    let row = sqlx::query(
        r#"
        SELECT url, username, password, download_folder
        FROM qbittorrent_settings
        WHERE id = 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    let config = row.map(|row| QBittorrentConfig {
        url: row.get("url"),
        username: row.get("username"),
        password: row.get("password"),
        download_folder: row.get("download_folder"),
    });

    if let Some(ref cfg) = config {
        info!("Found qBittorrent config with URL: {}", cfg.url);
    } else {
        info!("No qBittorrent config found in database");
    }

    Ok(config)
}

/// Stores a list of anime metadata entries in the database.
/// For each anime, it determines an appropriate file path, computes
/// the last modified time, serializes the metadata to JSON, and inserts or replaces
/// the entry in the database.
pub async fn store_anime_metadata(pool: &SqlitePool, metadata: &[HamaMetadata]) -> Result<()> {
    info!("Storing {} anime series in database", metadata.len());

    for anime in metadata {
        // Determine the file path for the anime.
        let path = get_anime_path(anime);

        info!(
            "Processing anime: {} with path: {}, episodes: {}, specials: {}",
            anime.title, path, anime.episode_count, anime.special_count
        );

        // Get the last modified timestamp (in seconds since UNIX_EPOCH).
        let last_modified = get_file_modified_time(&path);

        // Serialize the metadata to JSON.
        let json = serde_json::to_string(anime)
            .map_err(|e| anyhow!("Failed to serialize metadata for {}: {}", anime.title, e))?;

        // Insert or replace the metadata in the database.
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO anime_metadata (title, path, metadata, last_modified)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&anime.title)
        .bind(&path)
        .bind(&json)
        .bind(last_modified)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to store metadata for {}: {}", anime.title, e))?;

        info!(
            "Stored/updated metadata for: {} with {} episodes and {} specials",
            anime.title, anime.episode_count, anime.special_count
        );
    }

    Ok(())
}

/// Retrieves all unique anime metadata entries from the database.
pub async fn get_anime_metadata(pool: &SqlitePool) -> Result<Vec<HamaMetadata>> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT title, metadata
        FROM anime_metadata
        ORDER BY title
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch anime metadata: {}", e);
        anyhow!("Failed to fetch anime metadata: {}", e)
    })?;

    let mut result = Vec::new();
    let mut seen_titles = HashSet::new();

    for row in rows {
        let json: String = row.get("metadata");
        match serde_json::from_str::<HamaMetadata>(&json) {
            Ok(metadata) => {
                if seen_titles.insert(metadata.title.clone()) {
                    info!(
                        "Retrieved metadata for: {} with {} episodes and {} specials",
                        metadata.title, metadata.episode_count, metadata.special_count
                    );
                    result.push(metadata);
                } else {
                    warn!("Skipping duplicate entry for: {}", metadata.title);
                }
            }
            Err(e) => {
                error!("Failed to parse metadata JSON: {}", e);
            }
        }
    }

    info!("Retrieved {} unique anime series from database", result.len());
    Ok(result)
}

/// Helper function to get the last modified time of the file at the given path.
/// Returns the timestamp as seconds since UNIX_EPOCH.
fn get_file_modified_time(path: &str) -> i64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or_else(|_| SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::default())
        .as_secs() as i64
}

/// Helper function to determine the file path for an anime.
/// It uses the first valid path from either the episodes or specials lists.
/// If none is found, it logs a warning and falls back to using the anime title.
fn get_anime_path(anime: &HamaMetadata) -> String {
    anime
        .episodes
        .first()
        .or_else(|| anime.specials.first())
        .map(|entry| entry.path.clone())
        .unwrap_or_else(|| {
            warn!("No valid path found for '{}', using title as path", anime.title);
            anime.title.clone()
        })
}
