pub mod auth;
pub mod routes;
pub mod spa;
pub mod state;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use routes::{
    agent_routes::*, artwork_routes::handle_poster, collection_routes::handle_get_collections,
    dashboard_routes::handle_dashboard_overview, gdrive_routes::*, health_routes::{handle_services_status, handle_worker_status},
    job_routes::*, library_routes::*, nas_routes::handle_nas_scan,
    quota_routes::{handle_get_quota, handle_increment_quota},
    settings_routes::{handle_get_settings, handle_save_settings},
    staging_routes::*, stream_routes::handle_stream, subtitle_routes::{handle_subtitle, handle_subtitle_projects},
    tmdb_routes::handle_tmdb_search,
    torbox_routes::{handle_add_torrent, handle_delete_torrent, handle_list_torrents},
    tunnel_routes::{handle_tunnel_start, handle_tunnel_status, handle_tunnel_stop},
};
use spa::{handle_spa_index, handle_static};
use state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn build_router(state: Arc<AppState>) -> Router {
    // 1. API Routes (with AppState DI)
    let api_router = Router::new()
        // Dashboard & Health
        .route("/api/dashboard/overview", get(handle_dashboard_overview))
        .route("/api/services/status", get(handle_services_status))
        .route("/api/services/workers", get(handle_worker_status))
        // Jobs & Pipeline
        .route("/api/download/jobs", get(handle_list_jobs))
        .route("/api/pipelines", get(handle_pipelines))
        .route("/api/download/sync", post(handle_sync_jobs))
        .route("/api/download/cancel", post(handle_cancel_job))
        // GDrive & NAS
        .route("/api/gdrive/shows", get(handle_gdrive_shows))
        .route("/api/gdrive/season_files", get(handle_gdrive_season_files))
        .route("/api/gdrive/check", post(handle_gdrive_check))
        .route("/api/nas/scan", post(handle_nas_scan))
        // Library
        .route("/api/library/cross_check", get(handle_cross_check).post(handle_cross_check))
        .route("/api/library/stats", get(handle_library_stats))
        .route("/api/library/unified", get(handle_unified_library))
        .route("/api/library/build/status", get(handle_library_build_status))
        .route("/api/library/build", post(handle_library_build))
        .route("/api/library/refresh", post(handle_library_refresh))
        .route("/api/library/build/cancel", post(handle_library_build_cancel))
        // Media & Streaming
        .route("/api/poster", get(handle_poster))
        .route("/api/stream", get(handle_stream))
        .route("/api/media/collections", get(handle_get_collections))
        .route("/api/subtitles/vtt", get(handle_subtitle))
        .route("/api/subtitles/projects", get(handle_subtitle_projects))
        .route("/api/tmdb/search", get(handle_tmdb_search))
        // TorBox
        .route("/api/torbox", get(handle_list_torrents))
        .route("/api/torbox/torrents", get(handle_list_torrents).post(handle_add_torrent))
        .route("/api/torbox/add", post(handle_add_torrent))
        .route("/api/torbox/delete", post(handle_delete_torrent))
        // Settings & Workspace
        .route("/api/settings", get(handle_get_settings).post(handle_save_settings))
        .route("/api/fs/choose_directory", post(handle_choose_directory))
        .route("/api/workspace/set", post(handle_workspace_set))
        // Staging & Aria2
        .route("/api/subtitles/staging", get(handle_subtitles_staging))
        .route("/api/staging/purge", post(handle_staging_purge))
        .route("/api/aria2/control", post(handle_aria2_control))
        .route("/api/collector/inspect", post(handle_collector_inspect))
        // Quota & Tunnel
        .route("/api/quota", get(handle_get_quota))
        .route("/api/quota/increment", post(handle_increment_quota))
        .route("/api/tunnel/status", get(handle_tunnel_status))
        .route("/api/tunnel/start", post(handle_tunnel_start))
        .route("/api/tunnel/stop", post(handle_tunnel_stop))
        // AI Agent Bridge
        .route("/api/agent/queue", get(handle_agent_queue))
        .route("/api/agent/token_usage", get(handle_token_usage))
        .route("/api/agent/live_logs", get(handle_agent_live_logs))
        .route("/api/agent/live_logs/clear", post(handle_agent_logs_clear))
        .route("/api/agent/service/status", get(handle_agent_service_status))
        .route("/api/agent/service/ensure", get(handle_agent_service_ensure).post(handle_agent_service_ensure))
        .route("/api/agent/sessions", get(handle_agent_sessions))
        .route("/api/agent/session/reset", post(handle_agent_session_reset))
        .route("/api/agent/quota_status", get(handle_agent_quota_status))
        .route("/api/agent/quota_config", post(handle_agent_quota_config))
        .route("/api/agent/quota_reset", post(handle_agent_quota_reset))
        .route("/api/agent/command", post(handle_agent_command))
        .route("/api/agent/stop", post(handle_agent_stop))
        .route("/api/agent/resume", post(handle_agent_resume))
        // Middleware layers
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(auth::auth_middleware))
        .with_state(state);

    // 2. SPA Navigation Routes & Static Assets
    let spa_routes = [
        "/", "/index.html", "/home", "/overview", "/torbox", "/torrents", "/downloader",
        "/gdrive", "/library", "/plex", "/pipelines", "/sync", "/subtitles", "/subtitle-studio",
        "/tokens", "/token-usage", "/analytics", "/console", "/logs", "/terminal",
        "/settings", "/config", "/agent", "/chat",
    ];
    let mut spa_router = Router::new();
    for route in spa_routes {
        spa_router = spa_router.route(route, get(handle_spa_index));
    }
    spa_router = spa_router.route("/static/*file_path", get(handle_static));

    spa_router.merge(api_router)
}


pub async fn start_server(
    port: u16,
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = build_router(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    log::info!("🚀 Media Hub Pure Rust Axum Server listening on http://0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
