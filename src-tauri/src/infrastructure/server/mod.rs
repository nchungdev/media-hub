pub mod routes;
pub mod state;

use axum::{
    routing::{get, post},
    Router,
};
use routes::{
    artwork_routes::handle_poster,
    collection_routes::handle_get_collections,
    quota_routes::{handle_get_quota, handle_increment_quota},
    settings_routes::{handle_get_settings, handle_save_settings},
    stream_routes::handle_stream,
    subtitle_routes::handle_subtitle,
    torbox_routes::{handle_add_torrent, handle_delete_torrent, handle_list_torrents},
    tunnel_routes::{handle_tunnel_start, handle_tunnel_status, handle_tunnel_stop},
};
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Artwork & Subtitles
        .route("/api/poster", get(handle_poster))
        .route("/api/subtitles/vtt", get(handle_subtitle))
        // Streaming
        .route("/api/stream", get(handle_stream))
        // Collections
        .route("/api/media/collections", get(handle_get_collections))
        // Settings
        .route(
            "/api/settings",
            get(handle_get_settings).post(handle_save_settings),
        )
        // Quota
        .route("/api/quota", get(handle_get_quota))
        .route("/api/quota/increment", post(handle_increment_quota))
        // Torbox
        .route(
            "/api/torbox/torrents",
            get(handle_list_torrents).post(handle_add_torrent),
        )
        .route("/api/torbox/delete", post(handle_delete_torrent))
        // Tunnel
        .route("/api/tunnel/status", get(handle_tunnel_status))
        .route("/api/tunnel/start", post(handle_tunnel_start))
        .route("/api/tunnel/stop", post(handle_tunnel_stop))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn start_server(
    port: u16,
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
