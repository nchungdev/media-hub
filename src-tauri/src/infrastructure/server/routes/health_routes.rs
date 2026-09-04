use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_services_status(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let res = state.health.check_all().await;
    Json(json!(res))
}

/// Bat / dung / chay ngay mot worker nen.
///
/// "Dung" khong lam thread thoat ma chi ha co `enabled` -- vong lap van song
/// nhung bo qua viec, nho vay bat lai duoc ma khong phai khoi dong lai app.
pub async fn handle_worker_control(
    axum::extract::Path((name, action)): axum::extract::Path<(String, String)>,
) -> Json<Value> {
    use crate::services::worker_status as ws;

    let (ok, message) = match action.as_str() {
        "stop" => (ws::set_enabled(&name, false), "đã dừng"),
        "start" => (ws::set_enabled(&name, true), "đã bật"),
        "restart" => {
            ws::set_enabled(&name, true);
            ws::request_run(&name);
            (true, "đã yêu cầu chạy lại ngay")
        }
        _ => (false, "hành động không hợp lệ (stop|start|restart)"),
    };

    Json(json!({
        "success": ok,
        "worker": name,
        "action": action,
        "message": message,
    }))
}

/// Trang thai cac worker nen -- phuc vu tab quan ly service.
pub async fn handle_worker_status() -> Json<serde_json::Value> {
    let workers = crate::services::worker_status::snapshot();
    let running = workers.iter().filter(|w| w.state == "running").count();
    let errors = workers.iter().filter(|w| w.state == "error").count();
    Json(serde_json::json!({
        "workers": workers,
        "total": workers.len(),
        "running": running,
        "errors": errors,
    }))
}
