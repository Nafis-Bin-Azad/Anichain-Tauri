pub mod commands;
pub mod error;
pub mod qbittorrent;

pub use commands::*;
pub use error::*;
pub use qbittorrent::*;

use tauri::Builder;

pub fn run() {
    Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::fetch_rss_feed,
            commands::get_tracked_anime,
            commands::get_schedule,
            commands::track_anime,
            commands::untrack_anime,
            commands::initialize_qbittorrent,
            commands::get_qbittorrent_rules,
            commands::add_qbittorrent_rule
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
