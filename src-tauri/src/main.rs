// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod qbittorrent;
mod anime;
mod db;

use qbittorrent::{QBittorrentClient, QBittorrentConfig};
use anime::{AnimeClient, AnimeInfo, ScheduleEntry};
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::{State, Manager, Emitter};
use tracing::Level;
use sqlx::SqlitePool;

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

#[tauri::command]
async fn connect_qbittorrent(
    state: State<'_, AppState>,
    url: String,
    username: String,
    password: String,
    window: tauri::Window,
) -> Result<(), String> {
    let config = QBittorrentConfig {
        url: url.clone(),
        username: username.clone(),
        password: password.clone(),
    };

    // Try to connect
    let result = state
        .qb_client
        .lock()
        .await
        .connect(config.clone())
        .await;

    match result {
        Ok(_) => {
            // Save credentials to database
            db::save_qbittorrent_config(&state.db_pool, &config)
                .await
                .map_err(|e| format!("Failed to save config: {}", e))?;

            // Emit connection status
            window.emit("qbittorrent-status", ConnectionStatus {
                is_connected: true,
                error_message: None,
            }).map_err(|e| e.to_string())?;

            Ok(())
        }
        Err(e) => {
            let error_msg = format!("Failed to connect: {}", e);
            tracing::error!("{}", error_msg);

            // Emit connection status with error
            window.emit("qbittorrent-status", ConnectionStatus {
                is_connected: false,
                error_message: Some(error_msg.clone()),
            }).map_err(|e| e.to_string())?;

            // Emit event to switch to settings tab
            window.emit("switch-to-settings", ()).map_err(|e| e.to_string())?;

            Err(error_msg)
        }
    }
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
        if let Err(e) = qb_client.add_torrent(&magnet_url, Some("Anime")).await {
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

    let app = tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            connect_qbittorrent,
            check_qbittorrent_connection,
            refresh_anime_list,
            get_available_anime,
            track_anime,
            get_schedule,
            get_settings,
            // ... other handlers ...
        ])
        .setup(|app| {
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
            
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(|_app_handle, event| match event {
        tauri::RunEvent::WindowEvent { 
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } => {
            api.prevent_close();
            tracing::info!("Close requested for window {}", label);
        }
        _ => {}
    });

    Ok(())
}
