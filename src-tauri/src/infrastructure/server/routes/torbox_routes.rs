use crate::infrastructure::server::state::AppState;
use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct TorboxListQuery {
    pub bypass_cache: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TorboxAddPayload {
    pub magnet: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TorboxDeletePayload {
    pub torrent_id: Option<u64>,
}

pub async fn handle_list_torrents(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TorboxListQuery>,
) -> Json<Value> {
    let bypass = query
        .bypass_cache
        .as_deref()
        .map(|b| b == "true" || b == "1" || b == "2")
        .unwrap_or(true);

    match state.torbox.list_torrents(bypass).await {
        Ok(data) => Json(data),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

pub async fn handle_add_torrent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TorboxAddPayload>,
) -> Json<Value> {
    let magnet = match payload.magnet {
        Some(ref m) if !m.is_empty() => m,
        _ => return Json(json!({ "success": false, "error": "Thiếu magnet link" })),
    };

    match state.torbox.add_torrent(magnet).await {
        Ok(data) => Json(data),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}

pub async fn handle_delete_torrent(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TorboxDeletePayload>,
) -> Json<Value> {
    let tid = match payload.torrent_id {
        Some(id) => id,
        None => return Json(json!({ "success": false, "error": "Thiếu torrent_id" })),
    };

    match state.torbox.delete_torrent(tid).await {
        Ok(data) => Json(data),
        Err(e) => Json(json!({ "success": false, "error": e })),
    }
}
