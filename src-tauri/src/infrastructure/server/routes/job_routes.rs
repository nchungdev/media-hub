use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_list_jobs(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let active = state.job_store.list_active();
    let recent = state.job_store.list_recent(50);
    let counts = state.job_store.counts();

    Json(json!({
        "success": true,
        "active": active,
        "recent": recent,
        "counts": counts
    }))
}

pub async fn handle_pipelines(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let active = state.job_store.list_active();
    let recent = state.job_store.list_recent(20);
    let counts = state.job_store.counts();

    Json(json!({
        "monster": { "completed_eps": 0, "total_eps": 74 },
        "multi_show": { "current_show": "", "completed_eps": 0 },
        "active_syncs": active,
        "recent_syncs": recent,
        "job_counts": counts,
        "library_version": "2.0_rust"
    }))
}

#[derive(Deserialize)]
pub struct SyncRequest {
    #[serde(default)]
    pub ids: Vec<String>,
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub targets: Option<Vec<String>>,
    #[serde(default)]
    pub target: Option<String>,
}

pub async fn handle_sync_jobs(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SyncRequest>,
) -> Json<Value> {
    let mut targets = payload.targets.unwrap_or_default();
    if targets.is_empty() {
        if let Some(t) = payload.target {
            targets.push(t);
        } else {
            targets.push("drive".to_string());
        }
    }

    if payload.ids.is_empty() {
        return Json(json!({ "success": false, "error": "Chưa chọn mục để đồng bộ" }));
    }

    let mut results = Vec::new();
    for (idx, id) in payload.ids.iter().enumerate() {
        let name = payload.names.get(idx).cloned().unwrap_or_else(|| format!("Torrent #{}", id));
        let res = state.job_store.enqueue(id, targets.clone(), &name);
        results.push(res);
    }

    let queued = results.iter().filter(|r| r.is_new_download).count();
    let merged = results.len() - queued;
    let target_label = targets.join(" & ");

    Json(json!({
        "success": true,
        "message": format!(
            "🚀 Đã xếp {} tác vụ tải mới lên {}{}",
            queued,
            target_label,
            if merged > 0 { format!(", {} mục gộp vào tiến trình đang chạy", merged) } else { "".to_string() }
        ),
        "details": results
    }))
}

#[derive(Deserialize)]
pub struct CancelRequest {
    pub job_id: i64,
}

pub async fn handle_cancel_job(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CancelRequest>,
) -> Json<Value> {
    let ok = state.job_store.request_cancel(payload.job_id);
    Json(json!({
        "success": ok,
        "message": if ok { "Đã gửi yêu cầu hủy tác vụ." } else { "Tác vụ đã kết thúc, không thể hủy." }
    }))
}
