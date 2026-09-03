pub struct DaemonManager;

impl DaemonManager {
    pub fn new() -> Self {
        Self
    }

    pub fn start_sidecar(&self, _app_handle: &tauri::AppHandle) -> Result<(), String> {
        // Pure Rust Axum server is active natively; no legacy Python server.py needed.
        log::info!("🦀 Pure Rust Core Server is active natively on port 8888.");
        Ok(())
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}

