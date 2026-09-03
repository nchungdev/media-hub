use crate::domain::models::settings::AppSettings;
use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_get_settings(State(state): State<Arc<AppState>>) -> Json<Value> {
    let s = state.settings.load();
    Json(serde_json::to_value(s).unwrap_or_default())
}

pub async fn handle_save_settings(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AppSettings>,
) -> Json<Value> {
    match state.settings.save(&payload) {
        Ok(()) => Json(json!({
            "success": true,
            "message": "Đã lưu cài đặt thành công!"
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}
