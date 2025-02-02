// src/server.rs

use axum::{
    extract::{Path, Query},
    routing::{get, post},
    Router, Json,
};
use anyhow::Result;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

// Import our anime service and models (ensure these modules are implemented in your project)
use crate::anime_service;
use crate::models::Anime;

/// Query parameters for the /search endpoint.
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    lang: Option<String>,
    manual: Option<bool>,
}

/// Request payload for the /download endpoint.
#[derive(Deserialize)]
struct DownloadRequest {
    anime_id: String,
}

/// Starts the Axum HTTP server on the given port.
pub async fn run_server(port: u16) -> Result<()> {
    // Build the router with the desired routes.
    let app = Router::new()
        .route("/anime/:id", get(get_anime))
        .route("/search", get(search_anime))
        .route("/download", post(download_anime))
        .route("/schedule", get(get_schedule));

    // Bind the server to localhost on the specified port.
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Server running at http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

/// Handler for GET /anime/:id.
/// Returns the anime metadata for the given id.
/// On error, it logs the error and returns a default Anime.
async fn get_anime(Path(id): Path<String>) -> Json<Anime> {
    match anime_service::get_anime_by_id(&id).await {
        Ok(anime) => Json(anime),
        Err(err) => {
            eprintln!("Error fetching anime by id {}: {:?}", id, err);
            Json(Anime::default())
        }
    }
}

/// Handler for GET /search.
/// Expects query parameters: q (query), lang (optional), and manual (optional).
/// Returns a list of matching Anime entries.
async fn search_anime(Query(params): Query<SearchQuery>) -> Json<Vec<Anime>> {
    let lang = params.lang.unwrap_or_else(|| "en".to_string());
    match anime_service::search_anime(&params.q, &lang, params.manual.unwrap_or(false), false).await {
        Ok(results) => Json(results),
        Err(err) => {
            eprintln!("Error searching anime: {:?}", err);
            Json(Vec::new())
        }
    }
}

/// Handler for POST /download.
/// Expects a JSON body containing an `anime_id`.
/// Returns a confirmation message or an error message.
async fn download_anime(Json(req): Json<DownloadRequest>) -> Json<String> {
    match anime_service::download_anime(&req.anime_id).await {
        Ok(_) => Json("Download started".to_string()),
        Err(err) => {
            eprintln!("Error starting download for {}: {:?}", req.anime_id, err);
            Json(format!("Error: {}", err))
        }
    }
}

/// Handler for GET /schedule.
/// Returns the current anime airing schedule as JSON.
async fn get_schedule() -> Json<serde_json::Value> {
    match anime_service::fetch_schedule().await {
        Ok(schedule) => Json(schedule),
        Err(err) => {
            eprintln!("Error fetching schedule: {:?}", err);
            Json(json!({}))
        }
    }
}
