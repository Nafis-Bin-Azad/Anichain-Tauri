// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn get_anime_description(title: &str) -> String {
    match title {
        "Jujutsu Kaisen Season 2" => "The hidden world of Jujutsu Sorcery continues as Yuji Itadori and his friends face new challenges and powerful curses.".to_string(),
        "Solo Leveling" => "In a world where hunters must battle deadly monsters to protect humanity, Sung Jinwoo is known as the weakest of all hunters.".to_string(),
        "Demon Slayer" => "Tanjiro's journey continues as he faces powerful demons while trying to turn his sister back into a human.".to_string(),
        _ => "Description not available.".to_string(),
    }
}

#[tauri::command]
fn get_settings() -> serde_json::Value {
    serde_json::json!({
        "downloadFolder": "~/Downloads/Anime",
        "rssUrl": "https://example.com/anime.rss",
        "qbHost": "http://localhost:8080",
        "qbUsername": "admin",
        "qbPassword": "adminpass"
    })
}

#[tauri::command]
fn save_settings(settings: serde_json::Value) -> Result<(), String> {
    println!("Saving settings: {:?}", settings);
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_anime_description,
            get_settings,
            save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
