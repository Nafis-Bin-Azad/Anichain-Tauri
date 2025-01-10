pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
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
