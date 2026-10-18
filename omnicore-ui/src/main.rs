// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn init_engine() -> String {
    // Basic interaction with the engine
    let _bridge = omnicore_engine::bridges::matrix::MatrixBridge::new();
    "Engine and bridges initialized successfully".to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, init_engine])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
