use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct TmdbSearchQuery {
    #[serde(default)]
    pub query: String,
}

pub async fn handle_tmdb_search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TmdbSearchQuery>,
) -> Json<Value> {
    let res = state.tmdb.search(&query.query).await;
    Json(res)
}
