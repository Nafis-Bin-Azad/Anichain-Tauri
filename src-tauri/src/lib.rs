use tauri::Builder;

pub fn run() {
    Builder::default()
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn greet(name: &str, email: &str) -> String {
    println!("Inside rust code");
    format!("Hello, {}! email: {}", name, email)
}
