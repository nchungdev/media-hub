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
