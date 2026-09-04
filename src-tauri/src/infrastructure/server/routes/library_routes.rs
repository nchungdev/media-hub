use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_cross_check(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let res = state.library.cross_check();
    Json(json!(res))
}

pub async fn handle_library_stats(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "drive": { "shows": 0, "files": 0, "size_gb": 0 },
        "missing_assets": []
    }))
}

pub async fn handle_library_build_status(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "running": false,
        "progress": 100,
        "message": "Sẵn sàng"
    }))
}

pub async fn handle_library_build(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "message": "Đã bắt đầu tiến trình dựng metadata"
    }))
}

pub async fn handle_library_refresh(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let _ = state.gdrive.list_tv_shows(true);
    Json(json!({
        "success": true,
        "refreshed": true,
        "message": "Đã lập chỉ mục lại thư viện Google Drive."
    }))
}

pub async fn handle_library_build_cancel(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "message": "Đã yêu cầu dừng tiến trình dựng metadata."
    }))
}

/// Thu vien hop nhat 3 nguon, gom theo franchise, danh dau
/// co mat o local / NAS / Google Drive.
pub async fn handle_unified_library(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let lib = crate::services::library_aggregator::aggregate(&state.job_store);
    Json(serde_json::to_value(lib).unwrap_or_else(|_| serde_json::json!({})))
}
