use sqlx::{sqlite::SqlitePool, Row};
use anyhow::{Result, anyhow};
use crate::qbittorrent::QBittorrentConfig;
use tracing;
use std::fs;
use directories::ProjectDirs;

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
    
    // Create SQLite connection URL with proper format for file-based database
    let database_url = format!("sqlite:{}?mode=rwc", db_path.display());
    
    // Connect to database
    let pool = SqlitePool::connect(&database_url).await?;

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

    // Check if settings exist
    let count: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM qbittorrent_settings")
        .fetch_one(&pool)
        .await?;

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
    let row = sqlx::query("SELECT url, username, password, download_folder FROM qbittorrent_settings WHERE id = 1")
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