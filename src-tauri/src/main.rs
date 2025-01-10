// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app_lib::commands::*;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_http::init())
        .invoke_handler(tauri::generate_handler![
            initialize_qbittorrent,
            fetch_rss_feed,
            get_schedule,
            get_tracked_anime,
            track_anime,
            untrack_anime,
            get_qbittorrent_rules,
            add_qbittorrent_rule
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
