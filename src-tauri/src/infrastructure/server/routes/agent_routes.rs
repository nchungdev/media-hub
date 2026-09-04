use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_agent_queue(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.agent.list_commands())
}

pub async fn handle_token_usage(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({
        "success": true,
        "daily_used": 0,
        "weekly_used": 0,
        "models": {}
    }))
}

pub async fn handle_agent_live_logs(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.agent.get_live_logs())
}

pub async fn handle_agent_service_status(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.agent.ensure_service())
}

pub async fn handle_agent_sessions(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.agent.get_sessions())
}

pub async fn handle_agent_quota_status(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let q = state.quota.get_status();
    Json(json!(q))
}

#[derive(Deserialize)]
pub struct CommandRequest {
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub cmd: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub media_id: Option<String>,
}

pub async fn handle_agent_command(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommandRequest>,
) -> Json<Value> {
    let cmd = payload
        .command
        .or(payload.cmd)
        .or(payload.text)
        .or(payload.message)
        .unwrap_or_default();

    if cmd.trim().is_empty() {
        return Json(json!({ "success": false, "error": "Vui lòng nhập nội dung lệnh" }));
    }

    let item = state
        .agent
        .add_command(cmd.trim(), "MediaHub UI", payload.media_id.as_deref());
    Json(json!({ "success": true, "command": item }))
}

#[derive(Deserialize)]
pub struct SessionResetRequest {
    pub media_id: String,
}

pub async fn handle_agent_session_reset(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SessionResetRequest>,
) -> Json<Value> {
    let ok = state.agent.clear_media_session(&payload.media_id);
    Json(json!({
        "success": ok,
        "message": format!("Đã xử lý session cache cho {}", payload.media_id)
    }))
}

pub async fn handle_agent_logs_clear(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    state.agent.clear_live_logs();
    Json(json!({ "success": true, "message": "Đã xoá log console." }))
}

pub async fn handle_agent_stop(
    State(_state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({ "success": true, "message": "Đã dừng tiến trình CLI." }))
}

pub async fn handle_agent_resume(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    state.agent.trigger_worker();
    Json(json!({ "success": true, "message": "Đã kích hoạt lại hàng đợi CLI." }))
}

pub async fn handle_agent_service_ensure(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(state.agent.ensure_service())
}

#[derive(Deserialize)]
pub struct QuotaConfigRequest {
    pub daily_limit: Option<u32>,
    pub weekly_limit: Option<u32>,
}

pub async fn handle_agent_quota_config(
    State(state): State<Arc<AppState>>,
    Json(_payload): Json<QuotaConfigRequest>,
) -> Json<Value> {
    let q = state.quota.get_status();
    Json(json!({ "success": true, "quota": q }))
}

pub async fn handle_agent_quota_reset(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let q = state.quota.get_status();
    Json(json!({ "success": true, "message": "Đã reset bộ đếm Quota", "quota": q }))
}

/// Trang thai daemon agy (stream-json).
pub async fn handle_agy_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    Json(state.agy.status())
}

/// Su kien gan nhat agy ban ra (NDJSON da parse) -- nguon cho Live Console.
pub async fn handle_agy_events(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let events = state.agy.recent_events(200);
    Json(serde_json::json!({ "events": events, "count": events.len() }))
}

/// Gui mot luot vao daemon dang chay.
pub async fn handle_agy_send(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    let content = body
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if content.trim().is_empty() {
        return Json(serde_json::json!({ "success": false, "error": "thieu content" }));
    }
    match state.agy.send(&content) {
        Ok(_) => Json(serde_json::json!({ "success": true })),
        Err(e) => Json(serde_json::json!({ "success": false, "error": e })),
    }
}
