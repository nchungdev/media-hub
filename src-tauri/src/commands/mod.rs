use crate::domain::models::app_info::{AppInfo, ServerStatus};
use crate::infrastructure::server::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub fn get_app_info() -> AppInfo {
    AppInfo {
        name: "Antigravity Media Hub".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        runtime: "Tauri 2.0 (Rust Native)".to_string(),
        platform: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    }
}

#[tauri::command]
pub fn get_server_status() -> ServerStatus {
    ServerStatus {
        online: true,
        port: 8888,
        url: "http://127.0.0.1:8888".to_string(),
    }
}

#[tauri::command]
pub fn show_in_folder(path: String) -> Result<(), String> {
    let p = std::path::PathBuf::from(path);
    if p.exists() {
        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("open").arg("-R").arg(&p).spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = std::process::Command::new("explorer")
                .arg(format!("/select,{}", p.display()))
                .spawn();
        }
        #[cfg(target_os = "linux")]
        {
            if let Some(parent) = p.parent() {
                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
            }
        }
        Ok(())
    } else {
        Err("Path does not exist".to_string())
    }
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_quota_status(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let q = state.quota.get_status();
    serde_json::to_value(q).unwrap_or_default()
}
