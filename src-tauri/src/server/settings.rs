/**
 * @file settings.rs
 * @description Settings configuration management in Rust.
 */

use axum::{
    response::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_provider")]
    pub default_provider: String,
    #[serde(default = "default_concurrent")]
    pub max_concurrent_downloads: u32,
    #[serde(default = "default_workspace")]
    pub media_hub_home: String,
    #[serde(default = "default_workspace")]
    pub workspace_dir: String,
    #[serde(default = "default_movies")]
    pub movies_dirname: String,
    #[serde(default = "default_tv")]
    pub tv_dirname: String,
    #[serde(default)]
    pub staging_dir: String,
    #[serde(default)]
    pub logs_dir: String,
    #[serde(default)]
    pub torbox_token: String,
    #[serde(default)]
    pub tmdb_api_key: String,
    #[serde(default = "default_lang")]
    pub tmdb_lang: String,
    #[serde(default = "default_aria2_host")]
    pub aria2_rpc_host: String,
    #[serde(default = "default_aria2_port")]
    pub aria2_rpc_port: u16,
    #[serde(default)]
    pub aria2_rpc_secret: String,
    #[serde(default)]
    pub nas_host: String,
    #[serde(default = "default_nas_user")]
    pub nas_user: String,
    #[serde(default = "default_nas_port")]
    pub nas_port: u16,
    #[serde(default)]
    pub nas_ssh_key: String,
    #[serde(default = "default_nas_path")]
    pub nas_path: String,
    #[serde(default = "default_gdrive_remote")]
    pub gdrive_remote: String,
    #[serde(default = "default_gdrive_root")]
    pub gdrive_root: String,
    #[serde(default = "default_sync_targets")]
    pub sync_targets: Vec<String>,
    #[serde(default = "default_sync_transfers")]
    pub sync_transfers: u32,
    #[serde(default = "default_auto_purge")]
    pub auto_purge: bool,
}

fn default_provider() -> String { "torbox".to_string() }
fn default_concurrent() -> u32 { 2 }
fn default_workspace() -> String { "/Volumes/512GB/AI Workspace".to_string() }
fn default_movies() -> String { "Movies".to_string() }
fn default_tv() -> String { "TV Shows".to_string() }
fn default_lang() -> String { "vi-VN".to_string() }
fn default_aria2_host() -> String { "127.0.0.1".to_string() }
fn default_aria2_port() -> u16 { 6800 }
fn default_nas_user() -> String { "admin".to_string() }
fn default_nas_port() -> u16 { 22 }
fn default_nas_path() -> String { "/volume1/video/TV Shows".to_string() }
fn default_gdrive_remote() -> String { "gdrive".to_string() }
fn default_gdrive_root() -> String { "Phim".to_string() }
fn default_sync_targets() -> Vec<String> { vec!["drive".to_string()] }
fn default_sync_transfers() -> u32 { 4 }
fn default_auto_purge() -> bool { true }

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            max_concurrent_downloads: default_concurrent(),
            media_hub_home: default_workspace(),
            workspace_dir: default_workspace(),
            movies_dirname: default_movies(),
            tv_dirname: default_tv(),
            staging_dir: String::new(),
            logs_dir: String::new(),
            torbox_token: String::new(),
            tmdb_api_key: String::new(),
            tmdb_lang: default_lang(),
            aria2_rpc_host: default_aria2_host(),
            aria2_rpc_port: default_aria2_port(),
            aria2_rpc_secret: String::new(),
            nas_host: String::new(),
            nas_user: default_nas_user(),
            nas_port: default_nas_port(),
            nas_ssh_key: String::new(),
            nas_path: default_nas_path(),
            gdrive_remote: default_gdrive_remote(),
            gdrive_root: default_gdrive_root(),
            sync_targets: default_sync_targets(),
            sync_transfers: default_sync_transfers(),
            auto_purge: default_auto_purge(),
        }
    }
}

pub struct SettingsManager;

impl SettingsManager {
    pub fn get_settings_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = PathBuf::from(home).join(".media-hub");
        let _ = fs::create_dir_all(&dir);
        dir.join("app_settings.json")
    }

    pub fn load() -> AppSettings {
        let path = Self::get_settings_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(settings) = serde_json::from_str(&content) {
                    return settings;
                }
            }
        }
        AppSettings::default()
    }

    pub fn save(settings: &AppSettings) -> Result<(), String> {
        let path = Self::get_settings_path();
        let content = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
        fs::write(path, content).map_err(|e| e.to_string())
    }
}

static CACHED_SETTINGS: RwLock<Option<AppSettings>> = RwLock::new(None);

pub async fn handle_get_settings() -> Json<AppSettings> {
    {
        let read = CACHED_SETTINGS.read().unwrap();
        if let Some(ref s) = *read {
            return Json(s.clone());
        }
    }

    let s = SettingsManager::load();
    let mut write = CACHED_SETTINGS.write().unwrap();
    *write = Some(s.clone());
    Json(s)
}

pub async fn handle_save_settings(Json(payload): Json<AppSettings>) -> Json<serde_json::Value> {
    if let Err(e) = SettingsManager::save(&payload) {
        return Json(serde_json::json!({
            "success": false,
            "error": e
        }));
    }

    let mut write = CACHED_SETTINGS.write().unwrap();
    *write = Some(payload);

    Json(serde_json::json!({
        "success": true,
        "message": "Cấu hình đã được lưu thành công trong Rust Core"
    }))
}

pub fn settings_routes() -> Router {
    Router::new()
        .route("/api/settings", get(handle_get_settings).post(handle_save_settings))
}
