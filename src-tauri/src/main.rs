// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod qbittorrent;
mod anime;
mod db;
mod anime_folder;
mod hama;

use qbittorrent::{QBittorrentClient, QBittorrentConfig};
use anime::{AnimeClient, AnimeInfo, ScheduleEntry};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{State, Manager, Emitter};
use tracing::Level;
use sqlx::SqlitePool;
use std::path::Path;
use notify::{Watcher, RecursiveMode, Event};
use std::sync::mpsc::channel;
use std::time::Duration;
use db::{get_qbittorrent_config, store_anime_metadata};
use hama::fetch_anime_metadata;
use std::sync::OnceLock;

#[derive(Clone, Debug)]
pub struct AppState {
    pub qb_client: Arc<Mutex<QBittorrentClient>>,
    pub anime_client: Arc<AnimeClient>,
    pub tracked_anime: Arc<Mutex<std::collections::HashSet<String>>>,
    pub db_pool: Arc<SqlitePool>,
}

#[derive(Debug, Serialize, Clone)]
struct ConnectionStatus {
    is_connected: bool,
    error_message: Option<String>,
}

#[derive(Debug, Serialize)]
struct Settings {
    qbittorrent: Option<QBittorrentConfig>,
}

static APP_STATE: OnceLock<AppState> = OnceLock::new();

#[tauri::command]
async fn connect_qbittorrent(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
) -> Result<(), String> {
    let config = QBittorrentConfig {
        url,
        username,
        password,
        download_folder: state.qb_client.lock().await.get_download_folder().await
            .map_err(|e| format!("Failed to get download folder: {}", e))?,
    };

    // Save to database first
    db::save_qbittorrent_config(&state.db_pool, &config)
        .await
        .map_err(|e| format!("Failed to save config: {}", e))?;

    // Then try to connect
    let client = state.qb_client.lock().await;
    client
        .connect(config)
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn check_qbittorrent_connection(
    state: State<'_, AppState>,
    window: tauri::Window,
) -> Result<bool, String> {
    let is_connected = state.qb_client.lock().await.is_connected().await;
    
    window.emit("qbittorrent-status", ConnectionStatus {
        is_connected,
        error_message: if !is_connected {
            Some("Not connected to qBittorrent".to_string())
        } else {
            None
        },
    }).map_err(|e| e.to_string())?;

    Ok(is_connected)
}

#[tauri::command]
async fn refresh_anime_list(state: State<'_, AppState>) -> Result<(), String> {
    state.anime_client.refresh_anime_list().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_available_anime(state: State<'_, AppState>) -> Result<Vec<AnimeInfo>, String> {
    state.anime_client.get_available_anime().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn track_anime(
    state: State<'_, AppState>,
    title: String,
    magnet_url: String,
    window: tauri::Window,
) -> Result<(), String> {
    // Add to tracked anime set
    {
        let mut tracked = state.tracked_anime.lock().await;
        tracked.insert(title.clone());
    }

    // Add to qBittorrent if connected
    let qb_client = state.qb_client.lock().await;
    if qb_client.is_connected().await {
        if let Err(e) = qb_client.add_torrent(&magnet_url).await {
            tracing::error!("Failed to add torrent to qBittorrent: {}", e);
            return Err(format!("Failed to add torrent: {}", e));
        }
        tracing::info!("Successfully added torrent for: {}", title);
    } else {
        // If not connected, switch to settings tab
        window.emit("switch-to-settings", ()).map_err(|e| e.to_string())?;
        return Err("Not connected to qBittorrent".to_string());
    }

    Ok(())
}

#[tauri::command]
async fn get_schedule(state: State<'_, AppState>) -> Result<Vec<ScheduleEntry>, String> {
    state.anime_client.get_schedule().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    let config = db::get_qbittorrent_config(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to get settings: {}", e))?;
    
    Ok(Settings {
        qbittorrent: config,
    })
}

#[tauri::command]
async fn get_download_folder(state: State<'_, AppState>) -> Result<String, String> {
    tracing::info!("=== Retrieving Download Folder ===");
    let client = state.qb_client.lock().await;
    match client.get_download_folder().await {
        Ok(folder) => {
            tracing::info!("Current download folder path: {}", folder);
            Ok(folder)
        }
        Err(e) => {
            tracing::error!("Failed to get download folder: {}", e);
            Err(format!("Failed to get download folder: {}", e))
        }
    }
}

#[tauri::command]
async fn set_download_folder(
    state: State<'_, AppState>,
    folder: String,
    window: tauri::Window,
) -> Result<(), String> {
    tracing::info!("=== Setting Download Folder ===");
    tracing::info!("Requested new download folder: {}", folder);
    
    let client = state.qb_client.lock().await;
    
    // Get current folder for logging
    if let Ok(current_folder) = client.get_download_folder().await {
        tracing::info!("Current download folder: {}", current_folder);
    }
    
    match client.set_download_folder(folder.clone()).await {
        Ok(_) => {
            tracing::info!("Successfully updated download folder to: {}", folder);
            
            // Get the current config to update
            match db::get_qbittorrent_config(&state.db_pool).await {
                Ok(Some(mut config)) => {
                    tracing::info!("Updating database with new download folder");
                    config.download_folder = folder.clone();
                    
                    // Save updated config to database
                    if let Err(e) = db::save_qbittorrent_config(&state.db_pool, &config).await {
                        tracing::error!("Failed to save config to database: {}", e);
                        return Err(format!("Failed to save config: {}", e));
                    }
                    tracing::info!("Successfully saved new download folder to database");
                }
                Ok(None) => {
                    tracing::error!("No qBittorrent config found in database");
                    return Err("No qBittorrent config found".to_string());
                }
                Err(e) => {
                    tracing::error!("Failed to get config from database: {}", e);
                    return Err(format!("Failed to get config: {}", e));
                }
            }

            // Notify frontend
            if let Err(e) = window.emit("download-folder-changed", ()) {
                tracing::error!("Failed to emit download-folder-changed event: {}", e);
                return Err(format!("Failed to emit event: {}", e));
            }
            tracing::info!("Notified frontend of download folder change");

            Ok(())
        }
        Err(e) => {
            tracing::error!("Failed to set download folder: {}", e);
            Err(format!("Failed to set download folder: {}", e))
        }
    }
}

#[tauri::command]
async fn delete_downloaded_file(
    _state: State<'_, AppState>,
    filename: String,
) -> Result<(), String> {
    // Use the full path provided by the frontend
    std::fs::remove_file(filename)
        .map_err(|e| format!("Failed to delete file: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn get_downloaded_files(state: State<'_, AppState>) -> Result<Vec<DownloadedFile>, String> {
    let client = state.qb_client.lock().await;
    let download_folder = client
        .get_download_folder()
        .await
        .map_err(|e| format!("Failed to get download folder: {}", e))?;

    let mut files = Vec::new();
    let path = std::path::Path::new(&download_folder);
    
    if !path.exists() {
        return Ok(files);
    }

    for entry in std::fs::read_dir(path).map_err(|e| format!("Failed to read directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("Failed to read metadata: {}", e))?;
        
        if metadata.is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();
            let size = metadata.len();
            let size_str = if size < 1024 {
                format!("{}B", size)
            } else if size < 1024 * 1024 {
                format!("{:.1}KB", size as f64 / 1024.0)
            } else if size < 1024 * 1024 * 1024 {
                format!("{:.1}MB", size as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.1}GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
            };

            files.push(DownloadedFile {
                filename,
                size: size_str,
            });
        }
    }

    // Sort by filename
    files.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(files)
}

#[derive(Debug, Serialize)]
struct DownloadedFile {
    filename: String,
    size: String,
}

#[tauri::command]
async fn scan_downloaded_anime(state: State<'_, AppState>) -> Result<Vec<hama::HamaMetadata>, String> {
    let config = db::get_qbittorrent_config(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to get config: {}", e))?
        .ok_or_else(|| "No qBittorrent config found".to_string())?;
        
    let download_folder = config.download_folder;
    
    if !Path::new(&download_folder).exists() {
        return Err(format!("Download folder does not exist: {}", download_folder));
    }
    
    tracing::info!("Scanning download folder: {}", download_folder);
    
    let metadata = hama::fetch_anime_metadata(download_folder).await?;
    
    tracing::info!(
        "Found {} anime series in downloads folder",
        metadata.len()
    );
    
    for anime in &metadata {
        tracing::info!(
            "Anime: {} - {} episodes, {} specials",
            anime.title,
            anime.episode_count,
            anime.special_count
        );
    }
    
    Ok(metadata)
}

#[tauri::command]
async fn get_cached_anime_metadata(state: State<'_, AppState>) -> Result<Vec<hama::HamaMetadata>, String> {
    db::get_anime_metadata(&state.db_pool)
        .await
        .map_err(|e| format!("Failed to get cached anime metadata: {}", e))
}

#[tauri::command]
async fn start_folder_watch(app_handle: tauri::AppHandle, folder_path: String) -> Result<(), String> {
    let (tx, rx) = channel();
    
    let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
        match res {
            Ok(event) => {
                match event.kind {
                    notify::EventKind::Remove(_) | 
                    notify::EventKind::Create(_) |
                    notify::EventKind::Modify(_) => {
                        if let Ok(_) = tx.send(()) {
                            tracing::debug!("Detected file system change: {:?}", event.kind);
                        }
                    },
                    _ => {}
                }
            }
            Err(e) => tracing::error!("Watch error: {:?}", e),
        }
    }).map_err(|e| e.to_string())?;

    watcher.watch(folder_path.as_ref(), RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    // Keep watcher alive by storing it
    let watcher = std::sync::Arc::new(std::sync::Mutex::new(watcher));
    let watcher_clone = watcher.clone();
    
    // Spawn a task to handle debounced updates
    tauri::async_runtime::spawn(async move {
        let mut last_update = std::time::Instant::now();
        while rx.recv().is_ok() {
            if last_update.elapsed() > Duration::from_secs(2) {  // Reduced debounce time to 2 seconds
                tracing::info!("Triggering rescan due to file system changes");
                if let Err(e) = scan_anime_folder(&app_handle).await {
                    tracing::error!("Failed to rescan folder: {}", e);
                }
                last_update = std::time::Instant::now();
            }
        }
        // Keep watcher alive until the end of the task
        drop(watcher_clone);
    });

    Ok(())
}

async fn scan_anime_folder(app_handle: &tauri::AppHandle) -> Result<(), String> {
    let state = app_handle.state::<AppState>();
    let config = get_qbittorrent_config(&state.db_pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No qBittorrent config found".to_string())?;

    let folder_path = config.download_folder;
    tracing::info!("Download folder from config: {}", folder_path);

    let metadata = fetch_anime_metadata(folder_path)
        .await
        .map_err(|e| e.to_string())?;

    // Clear existing metadata before storing new data
    sqlx::query("DELETE FROM anime_metadata")
        .execute(&*state.db_pool)
        .await
        .map_err(|e| e.to_string())?;

    store_anime_metadata(&state.db_pool, &metadata)
        .await
        .map_err(|e| e.to_string())?;

    if let Err(e) = app_handle.emit("anime_data_ready", ()) {
        tracing::error!("Failed to emit anime_data_ready event: {}", e);
    }

    Ok(())
}

#[tauri::command]
async fn fetch_anime_metadata_command(folder_path: String) -> Result<Vec<hama::HamaMetadata>, String> {
    // If the folder path ends with a specific anime title, get it from cache
    if let Some(title) = folder_path.split('/').last() {
        let state = APP_STATE.get().ok_or("App state not initialized")?;
        let cached_metadata = db::get_anime_metadata(&state.db_pool)
            .await
            .map_err(|e| format!("Failed to get cached metadata: {}", e))?;
        
        // Find the specific anime in the cached metadata
        let filtered_metadata: Vec<_> = cached_metadata
            .into_iter()
            .filter(|m| m.title == title)
            .collect();
            
        if !filtered_metadata.is_empty() {
            return Ok(filtered_metadata);
        }
    }
    
    // If not found in cache or scanning entire folder, use HamaClient
    let client = hama::HamaClient::new();
    client.scan_folder(&folder_path).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .with_env_filter("info,hyper=warn,sqlx=warn")
        .init();

    // Initialize database
    let db_pool = db::init_db().await?;
    let db_pool = Arc::new(db_pool);

    let qb_client = Arc::new(Mutex::new(QBittorrentClient::new()));
    let anime_client = Arc::new(AnimeClient::new());
    
    // Try to connect if we have saved credentials
    if let Ok(Some(config)) = db::get_qbittorrent_config(&db_pool).await {
        tracing::info!("Found saved qBittorrent credentials, attempting to connect...");
        let client = qb_client.lock().await;
        match client.connect(config.clone()).await {
            Ok(_) => {
                tracing::info!("Successfully connected to qBittorrent using saved credentials");
            }
            Err(e) => {
                tracing::error!("Failed to connect with saved credentials: {}", e);
            }
        }
    } else {
        tracing::info!("No saved qBittorrent credentials found");
        // Create default config
        let default_config = QBittorrentConfig {
            url: "http://127.0.0.1:8080".to_string(),
            username: "nafislord".to_string(),
            password: "Saphire 1".to_string(),
            download_folder: "downloads".to_string(),
        };
        
        // Try to connect with default config
        tracing::info!("Attempting to connect with default credentials...");
        let client = qb_client.lock().await;
        match client.connect(default_config.clone()).await {
            Ok(_) => {
                tracing::info!("Successfully connected with default credentials");
                // Save the successful config
                if let Err(e) = db::save_qbittorrent_config(&db_pool, &default_config).await {
                    tracing::error!("Failed to save default config: {}", e);
                }
            }
            Err(e) => {
                tracing::error!("Failed to connect with default credentials: {}", e);
            }
        }
    }
    
    let app_state = AppState {
        qb_client,
        anime_client: anime_client.clone(),
        tracked_anime: Arc::new(Mutex::new(std::collections::HashSet::new())),
        db_pool,
    };
    
    // Set the global app state
    APP_STATE.set(app_state.clone()).unwrap();

    let app = tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            connect_qbittorrent,
            check_qbittorrent_connection,
            refresh_anime_list,
            get_available_anime,
            track_anime,
            get_schedule,
            get_settings,
            get_download_folder,
            set_download_folder,
            delete_downloaded_file,
            get_downloaded_files,
            scan_downloaded_anime,
            get_cached_anime_metadata,
            start_folder_watch,
            fetch_anime_metadata_command,
        ])
        .setup(|app| {
            let handle = app.handle();
            let window = app.get_webview_window("main").expect("main window not found");
            let state = app.state::<AppState>();
            
            // Clone what we need for the async task
            let window = window.clone();
            let qb_client = state.qb_client.clone();
            
            // Spawn a task to check connection after window is ready
            tauri::async_runtime::spawn(async move {
                let is_connected = qb_client.lock().await.is_connected().await;
                
                if let Err(e) = window.emit("qbittorrent-status", ConnectionStatus {
                    is_connected,
                    error_message: if !is_connected {
                        Some("Not connected to qBittorrent".to_string())
                    } else {
                        None
                    },
                }) {
                    tracing::error!("Failed to emit initial connection status: {}", e);
                }
            });
            
            // Clone what we need for background anime data loading
            let app_handle = handle.clone();
            let db_pool = state.db_pool.clone();
            let qb_client = state.qb_client.clone();
            
            // Spawn background task to load anime data
            tokio::spawn(async move {
                tracing::info!("Starting background anime data load");
                
                // Get download folder from qBittorrent config
                let download_folder = match qb_client.lock().await.get_download_folder().await {
                    Ok(folder) => folder,
                    Err(e) => {
                        tracing::error!("Failed to get download folder: {}", e);
                        return;
                    }
                };

                let path = std::path::Path::new(&download_folder);
                if path.exists() {
                    match hama::fetch_anime_metadata(download_folder).await {
                        Ok(metadata) => {
                            // Store metadata in database
                            if let Err(e) = db::store_anime_metadata(&db_pool, &metadata).await {
                                tracing::error!("Failed to store anime metadata: {}", e);
                            }
                            // Emit event to frontend that data is ready
                            if let Err(e) = app_handle.emit("anime_data_ready", ()) {
                                tracing::error!("Failed to emit anime_data_ready event: {}", e);
                            }
                        }
                        Err(e) => tracing::error!("Failed to fetch anime metadata: {}", e),
                    }
                } else {
                    tracing::warn!("Download folder does not exist: {}", download_folder);
                }
            });

            // Start the folder watcher
            let app_handle = handle.clone();
            let db_pool = state.db_pool.clone();
            
            // Get the config and start the watcher in a blocking context
            tokio::spawn(async move {
                if let Ok(Some(config)) = get_qbittorrent_config(&db_pool).await {
                    if let Err(e) = start_folder_watch(app_handle.clone(), config.download_folder).await {
                        tracing::error!("Failed to start folder watcher: {}", e);
                    }
                }
            });
            
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::WindowEvent { 
            label,
            event: tauri::WindowEvent::CloseRequested { api: _, .. },
            ..
        } => {
            tracing::info!("Close requested for window {}", label);
            // Don't prevent closing, just exit the app
            std::process::exit(0);
        }
        _ => {}
    });

    Ok(())
}
