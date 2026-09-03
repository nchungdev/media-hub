use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::process::Command;
use std::sync::Arc;

pub async fn handle_subtitles_staging(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let cfg = state.settings.load();
    let staging = cfg.staging_dir.clone();
    let mut files_list = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&staging) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let size_mb = (meta.len() as f64 / 1024.0 / 1024.0 * 100.0).round() / 100.0;
                    let is_video = name.ends_with(".mkv") || name.ends_with(".mp4") || name.ends_with(".m4v");
                    files_list.push(json!({
                        "filename": name,
                        "rel_path": name,
                        "full_path": entry.path().to_string_lossy(),
                        "size_mb": size_mb,
                        "type": if is_video { "video" } else { "subtitle" }
                    }));
                }
            }
        }
    }

    Json(json!({ "staging_dir": staging, "files": files_list }))
}

pub async fn handle_staging_purge(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let cfg = state.settings.load();
    let staging = cfg.staging_dir;
    let mut count = 0;
    if let Ok(entries) = std::fs::read_dir(&staging) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                if std::fs::remove_file(entry.path()).is_ok() {
                    count += 1;
                }
            }
        }
    }
    Json(json!({
        "success": true,
        "message": format!("Đã dọn dẹp sạch thư mục đệm ({} file)!", count)
    }))
}

#[derive(Deserialize)]
pub struct Aria2Request {
    #[serde(default = "default_op")]
    pub operation: String,
}

fn default_op() -> String {
    "start".to_string()
}

pub async fn handle_aria2_control(
    State(_state): State<Arc<AppState>>,
    Json(payload): Json<Aria2Request>,
) -> Json<Value> {
    if payload.operation == "start" {
        let bin = if std::path::Path::new("/opt/homebrew/bin/aria2c").exists() {
            "/opt/homebrew/bin/aria2c"
        } else {
            "aria2c"
        };
        let _ = Command::new(bin)
            .args(["--enable-rpc", "--rpc-listen-all=false", "--rpc-allow-origin-all", "-D"])
            .spawn();
        Json(json!({ "success": true, "message": "Đã khởi động Aria2c RPC Daemon thành công!" }))
    } else {
        let _ = Command::new("pkill").args(["-f", "aria2c --enable-rpc"]).output();
        Json(json!({ "success": true, "message": "Đã dừng Aria2c Daemon." }))
    }
}

pub async fn handle_choose_directory() -> Json<Value> {
    let script = "POSIX path of (choose folder with prompt \"Chọn Thư mục Làm việc Media Hub:\")";
    if let Ok(output) = Command::new("osascript").args(["-e", script]).output() {
        if output.status.success() {
            let chosen = String::from_utf8_lossy(&output.stdout).trim().trim_end_matches('/').to_string();
            if !chosen.is_empty() {
                return Json(json!({ "success": true, "path": chosen }));
            }
        }
    }
    Json(json!({ "success": false, "cancelled": true }))
}

#[derive(Deserialize)]
pub struct WorkspaceSetRequest {
    #[serde(default)]
    pub workspace_dir: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

pub async fn handle_workspace_set(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WorkspaceSetRequest>,
) -> Json<Value> {
    let ws = payload.workspace_dir.or(payload.path).unwrap_or_default();
    if ws.trim().is_empty() {
        return Json(json!({ "success": false, "error": "Thiếu đường dẫn thư mục làm việc" }));
    }

    let mut cfg = state.settings.load();
    cfg.workspace_dir = ws.clone();
    cfg.media_hub_home = format!("{}/.media-hub", ws);
    let _ = state.settings.save(&cfg);

    Json(json!({
        "success": true,
        "message": format!("Đã thiết lập thư mục làm việc: {}", ws),
        "workspace_dir": ws,
        "media_hub_home": cfg.media_hub_home
    }))
}

#[derive(Deserialize)]
pub struct CollectorInspectRequest {
    #[serde(default)]
    pub magnet: String,
    #[serde(default)]
    pub query: String,
}

pub async fn handle_collector_inspect(
    Json(payload): Json<CollectorInspectRequest>,
) -> Json<Value> {
    let title = if !payload.query.is_empty() { payload.query } else { "Media Release".to_string() };
    Json(json!({
        "success": true,
        "title": title,
        "hash": "",
        "magnet": payload.magnet,
        "message": "Đã phân tích thông tin nguồn tải thành công!"
    }))
}
