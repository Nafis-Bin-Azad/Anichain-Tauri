// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn greet(name: &str,email: &str) -> String {
  println!("Inside rust code");
  format!("Hello, {}! email: {}", name, email)
}

fn main() {
  app_lib::run();
}
