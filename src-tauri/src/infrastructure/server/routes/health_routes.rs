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
