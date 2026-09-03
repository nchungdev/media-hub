/**
 * @file commands.rs
 * @description Tauri command handlers exposed to frontend via window.__TAURI__.core.invoke()
 */

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub runtime: String,
    pub platform: String,
    pub arch: String,
}

#[derive(Serialize, Deserialize)]
pub struct ServerStatus {
    pub online: bool,
    pub port: u16,
    pub url: String,
}

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Media Hub".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime: "Tauri v2 + Rust".to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("magnet:") {
        open::that(&url).map_err(|e| e.to_string())
    } else {
        Err("Invalid protocol".to_string())
    }
}

#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_server_status() -> ServerStatus {
    let port = 8888;
    let url = format!("http://127.0.0.1:{}", port);
    let online = crate::daemon::DaemonManager::check_health(&url, 300).await;
    ServerStatus { online, port, url }
}
