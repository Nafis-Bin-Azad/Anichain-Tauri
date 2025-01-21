use sqlx::{sqlite::SqlitePool, Row};
use anyhow::{Result, anyhow};
use crate::qbittorrent::QBittorrentConfig;
use crate::hama::HamaMetadata;
use tracing;
use std::fs;
use directories::ProjectDirs;
use chrono;

pub async fn init_db() -> Result<SqlitePool> {
    // Get the app's data directory
    let project_dirs = ProjectDirs::from("com", "nafislord", "anichain")
        .ok_or_else(|| anyhow!("Failed to get project directories"))?;
    
    // Create data directory if it doesn't exist
    let data_dir = project_dirs.data_dir();
    fs::create_dir_all(data_dir)?;
    
    // Create database path
    let db_path = data_dir.join("anichain.db");
    tracing::info!("Using database at: {}", db_path.display());
    
    // Connect to database with create_if_missing option
    let pool = SqlitePool::connect_with(
        sqlx::sqlite::SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
    ).await?;

    // Create tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS qbittorrent_settings (
            id INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL,
            download_folder TEXT NOT NULL DEFAULT 'downloads'
        )"
    )
    .execute(&pool)
    .await?;

    // Create anime metadata table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS anime_metadata (
            title TEXT PRIMARY KEY,
            path TEXT NOT NULL,
            metadata TEXT NOT NULL,
            last_modified INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
            UNIQUE(title)
        )"
    )
    .execute(&pool)
    .await?;

    // Check if settings exist
    let count = sqlx::query_as::<_, (i32,)>("SELECT COUNT(*) FROM qbittorrent_settings")
        .fetch_one(&pool)
        .await?
        .0;

    // Insert default settings if none exist
    if count == 0 {
        tracing::info!("No existing qBittorrent settings found, inserting defaults...");
        let default_config = QBittorrentConfig {
            url: "http://127.0.0.1:8080".to_string(),
            username: "nafislord".to_string(),
            password: "Saphire 1".to_string(),
            download_folder: "downloads".to_string(),
        };
        save_qbittorrent_config(&pool, &default_config).await?;
        tracing::info!("Default qBittorrent settings saved successfully");
    }

    Ok(pool)
}

pub async fn save_qbittorrent_config(pool: &SqlitePool, config: &QBittorrentConfig) -> Result<()> {
    tracing::info!("Saving qBittorrent config to database...");
    sqlx::query(
        "INSERT OR REPLACE INTO qbittorrent_settings (id, url, username, password, download_folder) 
         VALUES (1, ?, ?, ?, ?)"
    )
    .bind(&config.url)
    .bind(&config.username)
    .bind(&config.password)
    .bind(&config.download_folder)
    .execute(pool)
    .await?;
    tracing::info!("qBittorrent config saved successfully");
    Ok(())
}

pub async fn get_qbittorrent_config(pool: &SqlitePool) -> Result<Option<QBittorrentConfig>> {
    tracing::info!("Retrieving qBittorrent config from database...");
    let row = sqlx::query(
        "SELECT url, username, password, download_folder 
         FROM qbittorrent_settings 
         WHERE id = 1"
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
        tracing::info!("Found qBittorrent config with URL: {}", cfg.url);
    } else {
        tracing::info!("No qBittorrent config found in database");
    }

    Ok(config)
}

pub async fn store_anime_metadata(pool: &SqlitePool, metadata: &[HamaMetadata]) -> Result<()> {
    tracing::info!("Storing {} anime series in database", metadata.len());
    for anime in metadata {
        // Get the first valid path from either episodes or specials
        let path = anime.episodes.first()
            .or_else(|| anime.specials.first())
            .map(|e| e.path.clone())
            .unwrap_or_else(|| {
                tracing::warn!("No valid path found for {}, using title as path", anime.title);
                anime.title.clone()
            });

        tracing::info!("Processing anime: {} with path: {}, episodes: {}, specials: {}", 
            anime.title, path, anime.episode_count, anime.special_count);

        // Get file modification time
        let last_modified = fs::metadata(&path)
            .map(|m| m.modified().unwrap_or_else(|_| std::time::SystemTime::now()))
            .unwrap_or_else(|_| std::time::SystemTime::now())
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Serialize metadata to JSON
        let json = serde_json::to_string(&anime)
            .map_err(|e| anyhow!("Failed to serialize metadata: {}", e))?;

        // Insert or replace metadata in database
        sqlx::query(
            "INSERT OR REPLACE INTO anime_metadata (title, path, metadata, last_modified) VALUES (?, ?, ?, ?)"
        )
        .bind(&anime.title)
        .bind(&path)
        .bind(&json)
        .bind(last_modified)
        .execute(pool)
        .await
        .map_err(|e| anyhow!("Failed to store metadata: {}", e))?;

        tracing::info!("Stored/updated metadata for: {} with {} episodes and {} specials", 
            anime.title, anime.episode_count, anime.special_count);
    }

    Ok(())
}

pub async fn get_anime_metadata(pool: &SqlitePool) -> Result<Vec<HamaMetadata>> {
    // Use DISTINCT to ensure we only get unique entries
    let rows = match sqlx::query(
        "SELECT DISTINCT title, metadata FROM anime_metadata ORDER BY title"
    )
    .fetch_all(pool)
    .await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!("Failed to fetch anime metadata: {}", e);
            return Err(anyhow!("Failed to fetch anime metadata: {}", e));
        }
    };

    let mut result = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();

    for row in rows {
        let json: String = row.get("metadata");
        match serde_json::from_str(&json) {
            Ok(metadata) => {
                let metadata: HamaMetadata = metadata;
                // Only add if we haven't seen this title before
                if seen_titles.insert(metadata.title.clone()) {
                    tracing::info!("Retrieved metadata for: {} with {} episodes and {} specials", 
                        metadata.title, metadata.episode_count, metadata.special_count);
                    result.push(metadata);
                } else {
                    tracing::warn!("Skipping duplicate entry for: {}", metadata.title);
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse metadata JSON: {}", e);
            }
        }
    }
    
    tracing::info!("Retrieved {} unique anime series from database", result.len());
    Ok(result)
} 