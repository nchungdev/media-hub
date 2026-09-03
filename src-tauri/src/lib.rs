pub mod commands;
pub mod daemon;
pub mod server;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // 1. Start High-Performance Rust Axum Server on Port 8889
            tauri::async_runtime::spawn(async {
                server::start_rust_server(8889).await;
            });

            // 2. Ensure Python sidecar server is running on Port 8888
            tauri::async_runtime::spawn(async {
                let _ = daemon::DaemonManager::ensure_server_running(
                    "http://127.0.0.1:8888",
                    8888,
                )
                .await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_app_info,
            commands::open_external,
            commands::show_in_folder,
            commands::get_server_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
