use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::Value;
use std::sync::Arc;

pub async fn handle_get_quota(State(state): State<Arc<AppState>>) -> Json<Value> {
    let q = state.quota.get_status();
    Json(serde_json::to_value(q).unwrap_or_default())
}

pub async fn handle_increment_quota(State(state): State<Arc<AppState>>) -> Json<Value> {
    let q = state.quota.increment();
    Json(serde_json::to_value(q).unwrap_or_default())
}
