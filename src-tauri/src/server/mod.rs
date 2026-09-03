pub mod artwork;
pub mod subtitles;

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
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("[Rust Core Server] 🦀 High-performance Axum server listening on http://{}", addr);

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
        "engine": "Axum 0.7 + Tokio (Rust)",
        "version": env!("CARGO_PKG_VERSION")
    }))
}
