pub mod artwork;
pub mod collections;
pub mod quota_guard;
pub mod settings;
pub mod stream;
pub mod subtitles;
pub mod torbox;
pub mod tunnel;

use axum::{
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

/// Initializes and starts the embedded Axum Rust Web Server
pub async fn start_rust_server(port: u16) {
    let app = Router::new()
        .route("/api/rust_health", get(health_check))
        .route("/api/poster", get(artwork::handle_poster))
        .route(
            "/api/subtitles/download",
            get(subtitles::handle_sub_download),
        )
        .merge(collections::collections_routes())
        .merge(stream::stream_routes())
        .merge(settings::settings_routes())
        .merge(quota_guard::quota_routes())
        .merge(torbox::torbox_routes())
        .merge(tunnel::tunnel_routes())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[Rust Core Server] 🦀 Full-featured Axum server listening on http://{}", addr);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[Rust Core Server] ⚠️ Failed to bind port {}: {:?}", port, e);
            return;
        }
    };

    let _ = axum::serve(listener, app).await;
}

async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "engine": "Axum 0.7 + Tokio (Rust Pure Core)",
        "version": env!("CARGO_PKG_VERSION"),
        "modules": [
            "artwork_resolver",
            "subtitle_engine",
            "collections_scanner",
            "video_streamer",
            "settings_manager",
            "quota_guard",
            "torbox_client",
            "tunnel_manager"
        ]
    }))
}
