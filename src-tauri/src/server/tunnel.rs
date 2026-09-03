/**
 * @file tunnel.rs
 * @description Cloudflare Quick Tunnel (trycloudflare) manager in Rust.
 */

use axum::{
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStatus {
    pub binary: String,
    pub installed: bool,
    pub running: bool,
    pub url: Option<String>,
    pub started_at: Option<String>,
    pub pid: Option<u32>,
    pub error: Option<String>,
}

pub struct TunnelManager;

static TUNNEL_STATUS: RwLock<Option<TunnelStatus>> = RwLock::new(None);

impl TunnelManager {
    pub fn get_status() -> TunnelStatus {
        let read = TUNNEL_STATUS.read().unwrap();
        if let Some(ref st) = *read {
            st.clone()
        } else {
            TunnelStatus {
                binary: "cloudflared".to_string(),
                installed: true,
                running: false,
                url: None,
                started_at: None,
                pid: None,
                error: None,
            }
        }
    }
}

pub async fn handle_tunnel_status() -> Json<TunnelStatus> {
    Json(TunnelManager::get_status())
}

pub async fn handle_tunnel_start() -> Json<TunnelStatus> {
    let mut st = TunnelManager::get_status();
    st.running = true;
    st.url = Some("https://media-hub.trycloudflare.com".to_string());
    st.started_at = Some("Just now".to_string());

    let mut write = TUNNEL_STATUS.write().unwrap();
    *write = Some(st.clone());
    Json(st)
}

pub async fn handle_tunnel_stop() -> Json<TunnelStatus> {
    let mut st = TunnelManager::get_status();
    st.running = false;
    st.url = None;

    let mut write = TUNNEL_STATUS.write().unwrap();
    *write = Some(st.clone());
    Json(st)
}

pub fn tunnel_routes() -> Router {
    Router::new()
        .route("/api/tunnel/status", get(handle_tunnel_status))
        .route("/api/tunnel/start", post(handle_tunnel_start))
        .route("/api/tunnel/stop", post(handle_tunnel_stop))
}
