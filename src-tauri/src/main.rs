// src/main.rs

mod api_fetcher;
mod anime_service;
mod config;
mod database;
mod downloader;
mod file_scanner;
mod models;
mod server;
mod utils;

mod api_providers {
    pub mod anilist;
    pub mod myanimelist;
    pub mod anidb;
    pub mod tvdb;
    pub mod themoviedb;
    pub mod omdb;
}

use anyhow::Result;
use config::Config;
use server::run_server;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging (env_logger uses RUST_LOG env variable)
    env_logger::init();

    // Load configuration.
    let config = Config::load("config.toml")?;

    // Initialize database.
    database::init(&config.database_url).await?;

    // Run startup tasks.
    anime_service::startup().await?;

    // Start the HTTP server.
    run_server(config.server_port).await?;

    Ok(())
}
