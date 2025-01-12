use sqlx::{sqlite::SqlitePool, Row};
use anyhow::{Result, anyhow};
use crate::qbittorrent::QBittorrentConfig;
use std::fs;
use tracing;

pub async fn init_db() -> Result<SqlitePool> {
    let app_dir = directories::ProjectDirs::from("com", "nafislord", "anichain")
        .ok_or_else(|| anyhow!("Failed to get app directory"))?;
    
    let data_dir = app_dir.data_dir();
    fs::create_dir_all(data_dir)?;
    
    let db_path = data_dir.join("anichain.db");
    tracing::info!("Using database at: {}", db_path.display());
    
    let database_url = format!("sqlite:{}?mode=rwc", db_path.display());
    
    let pool = SqlitePool::connect(&database_url).await?;
    
    // Create tables
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS qbittorrent_settings (
            id INTEGER PRIMARY KEY,
            url TEXT NOT NULL,
            username TEXT NOT NULL,
            password TEXT NOT NULL
        )"
    )
    .execute(&pool)
    .await?;

    // Check if we have any settings
    let row = sqlx::query("SELECT COUNT(*) as count FROM qbittorrent_settings")
        .fetch_one(&pool)
        .await?;
    let count: i32 = row.get("count");

    // Insert default settings if none exist
    if count == 0 {
        tracing::info!("No existing qBittorrent settings found, inserting defaults...");
        let default_config = QBittorrentConfig {
            url: "http://127.0.0.1:8080".to_string(),
            username: "nafislord".to_string(),
            password: "Saphire 1".to_string(),
        };
        save_qbittorrent_config(&pool, &default_config).await?;
        tracing::info!("Default qBittorrent settings saved successfully");
    }

    Ok(pool)
}

pub async fn save_qbittorrent_config(pool: &SqlitePool, config: &QBittorrentConfig) -> Result<()> {
    tracing::info!("Saving qBittorrent config to database...");
    sqlx::query(
        "INSERT OR REPLACE INTO qbittorrent_settings (id, url, username, password) 
         VALUES (1, ?, ?, ?)"
    )
    .bind(&config.url)
    .bind(&config.username)
    .bind(&config.password)
    .execute(pool)
    .await?;
    tracing::info!("qBittorrent config saved successfully");
    Ok(())
}

pub async fn get_qbittorrent_config(pool: &SqlitePool) -> Result<Option<QBittorrentConfig>> {
    tracing::info!("Retrieving qBittorrent config from database...");
    let row = sqlx::query("SELECT url, username, password FROM qbittorrent_settings WHERE id = 1")
        .fetch_optional(pool)
        .await?;

    let config = row.map(|row| QBittorrentConfig {
        url: row.get("url"),
        username: row.get("username"),
        password: row.get("password"),
    });

    if let Some(ref cfg) = config {
        tracing::info!("Found qBittorrent config with URL: {}", cfg.url);
    } else {
        tracing::info!("No qBittorrent config found in database");
    }

    Ok(config)
} 