pub mod commands;
pub mod domain;
pub mod infrastructure;
pub mod services;

use infrastructure::daemon::daemon_mgr::DaemonManager;
use infrastructure::server::{start_server, state::AppState};
use std::sync::Arc;

pub fn run() {
    let app_state = Arc::new(AppState::new());
    let server_state = app_state.clone();

    // Start Embedded High-Performance Rust Axum Server on Port 8889
    tokio::spawn(async move {
        if let Err(e) = start_server(8889, server_state).await {
            eprintln!("[Rust Server] Error running Axum server: {}", e);
        }
    });

    let daemon_mgr = DaemonManager::new();

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_log::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::get_server_status,
            commands::show_in_folder,
            commands::open_path,
            commands::get_quota_status,
        ])
        .setup(move |app| {
            // Optional: Start Python backend sidecar if not running
            let _ = daemon_mgr.start_sidecar(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Media Hub application");
}
