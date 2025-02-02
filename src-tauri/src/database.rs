// src/database.rs

use anyhow::Result;
use once_cell::sync::OnceCell;
use sqlx::sqlite::SqlitePool;
use crate::models::Anime;
use serde_json::to_string;

/// A global connection pool that is lazily initialized.
static POOL: OnceCell<SqlitePool> = OnceCell::new();

/// Initializes the SQLite database by creating a connection pool and ensuring the
/// required tables exist. The pool is stored globally for reuse across the application.
///
/// # Arguments
///
/// * `database_url` - A string slice representing the database URL (e.g., "sqlite://storage/database.sqlite").
///
/// # Returns
///
/// A [`SqlitePool`] wrapped in a `Result`.
pub async fn init(database_url: &str) -> Result<SqlitePool> {
    if let Some(pool) = POOL.get() {
        return Ok(pool.clone());
    }
    let pool = SqlitePool::connect(database_url).await?;
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS anime (
            id TEXT PRIMARY KEY,
            title TEXT,
            metadata TEXT
        )
        "#
    )
    .execute(&pool)
    .await?;
    // Store the pool globally so that subsequent calls reuse it.
    POOL.set(pool.clone()).unwrap();
    Ok(pool)
}

/// Retrieves an anime record from the database by its `id`.
///
/// # Arguments
///
/// * `id` - A string slice representing the anime's unique identifier.
///
/// # Returns
///
/// * `Ok(Some(anime))` if found,
/// * `Ok(None)` if not found,
/// * or an error wrapped in `Result` if the query fails.
pub async fn get_anime_by_id(id: &str) -> Result<Option<Anime>> {
    let pool = init("sqlite://storage/database.sqlite").await?;
    let rec = sqlx::query!("SELECT id, title, metadata FROM anime WHERE id = ?", id)
        .fetch_optional(&pool)
        .await?;
    if let Some(record) = rec {
        let anime: Anime = serde_json::from_str(&record.metadata)?;
        Ok(Some(anime))
    } else {
        Ok(None)
    }
}

/// Saves an anime record to the database. If a record with the same `id` already exists,
/// it will be replaced.
///
/// # Arguments
///
/// * `anime` - A reference to an [`Anime`] struct.
///
/// # Returns
///
/// An empty `Result` on success or an error wrapped in `Result` on failure.
pub async fn save_anime(anime: &Anime) -> Result<()> {
    let pool = init("sqlite://storage/database.sqlite").await?;
    let metadata_str = to_string(anime)?;
    sqlx::query!(
        "INSERT OR REPLACE INTO anime (id, title, metadata) VALUES (?, ?, ?)",
        anime.id,
        anime.title,
        metadata_str
    )
    .execute(&pool)
    .await?;
    Ok(())
}
