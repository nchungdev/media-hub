use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn handle_dashboard_overview(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let overview = state.dashboard.get_overview();
    Json(json!(overview))
}
