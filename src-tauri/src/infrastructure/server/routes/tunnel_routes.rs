use crate::infrastructure::server::state::AppState;
use axum::{extract::State, response::Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct TunnelStartPayload {
    pub port: Option<u16>,
    pub force: Option<bool>,
}

pub async fn handle_tunnel_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    let s = state.tunnel.get_status();
    Json(serde_json::to_value(s).unwrap_or_default())
}

pub async fn handle_tunnel_start(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TunnelStartPayload>,
) -> Json<Value> {
    let port = payload.port.unwrap_or(8888);
    let force = payload.force.unwrap_or(false);

    match state.tunnel.start(port, force) {
        Ok(s) => Json(json!({
            "success": true,
            "status": s,
            "url": s.url
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}

pub async fn handle_tunnel_stop(State(state): State<Arc<AppState>>) -> Json<Value> {
    match state.tunnel.stop() {
        Ok(s) => Json(json!({
            "success": true,
            "message": "Đã tắt Cloudflare Tunnel thành công.",
            "status": s
        })),
        Err(e) => Json(json!({
            "success": false,
            "error": e
        })),
    }
}
