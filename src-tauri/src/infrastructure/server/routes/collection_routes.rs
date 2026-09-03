use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct RefreshQuery {
    pub refresh: Option<String>,
}

pub async fn handle_get_collections(
    State(state): State<Arc<AppState>>,
    Query(query): Query<RefreshQuery>,
) -> Json<Value> {
    let refresh = query
        .refresh
        .as_deref()
        .map(|r| r == "1" || r == "true")
        .unwrap_or(false);

    let resp = state.collections.get_collections(refresh);
    Json(serde_json::to_value(resp).unwrap_or_default())
}
