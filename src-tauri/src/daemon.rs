/**
 * @file daemon.rs
 * @description Sidecar daemon manager in Rust.
 * Manages health-checking, process spawning, and graceful detachment for the Python backend during Phase 2.
 */

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

pub struct DaemonManager;

impl DaemonManager {
    /// Pings the server health endpoint.
    pub async fn check_health(url: &str, timeout_ms: u64) -> bool {
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let endpoint = format!("{}/api/settings", url.trim_end_matches('/'));
        match client.get(&endpoint).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Locates the optimal Python 3 executable on macOS / Linux.
    pub fn find_python_binary() -> String {
        let candidates = [
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
            "/usr/bin/python3",
            "/Library/Developer/CommandLineTools/usr/bin/python3",
            "python3",
        ];

        for cand in candidates {
            if cand.starts_with('/') && Path::new(cand).exists() {
                return cand.to_string();
            }
        }
        "python3".to_string()
    }

    /// Resolves root workspace and script path.
    pub fn resolve_paths() -> (PathBuf, PathBuf, PathBuf) {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let log_dir = PathBuf::from(&home).join(".media-hub").join(".logs");
        let _ = fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("server.log");

        // Locate apps/media-hub root relative to current executable or workspace
        let mut root_dir = PathBuf::from("/Volumes/512GB/AI Workspace/apps/media-hub");
        if !root_dir.exists() {
            if let Ok(cwd) = std::env::current_dir() {
                if cwd.join("scripts").join("server.py").exists() {
                    root_dir = cwd;
                } else if let Some(parent) = cwd.parent() {
                    if parent.join("scripts").join("server.py").exists() {
                        root_dir = parent.to_path_buf();
                    }
                }
            }
        }

        let script_path = root_dir.join("scripts").join("server.py");
        (root_dir, script_path, log_file)
    }

    /// Ensures the backend server is running, spawning it if needed.
    pub async fn ensure_server_running(url: &str, port: u16) -> bool {
        if Self::check_health(url, 300).await {
            println!("[Tauri Rust] ⚡ Backend server already running at {}", url);
            Self::attach_cli_service(url).await;
            return true;
        }

        let (root_dir, script_path, log_file) = Self::resolve_paths();
        let python_bin = Self::find_python_binary();

        println!("[Tauri Rust] 🚀 Spawning Python Backend Server:");
        println!("   - Python: {}", python_bin);
        println!("   - Script: {:?}", script_path);
        println!("   - Root:   {:?}", root_dir);
        println!("   - Log:    {:?}", log_file);

        let out_file = match OpenOptions::new().create(true).append(true).open(&log_file) {
            Ok(f) => Stdio::from(f),
            Err(_) => Stdio::null(),
        };

        let mut env_path = std::env::var("PATH").unwrap_or_default();
        let extra_paths = "/opt/homebrew/bin:/opt/homebrew/sbin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin";
        env_path = format!("{}:{}", extra_paths, env_path);

        match Command::new(&python_bin)
            .arg(&script_path)
            .current_dir(&root_dir)
            .env("PORT", port.to_string())
            .env("PYTHONUNBUFFERED", "1")
            .env("PATH", env_path)
            .stdout(out_file)
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => {
                println!("[Tauri Rust] ✅ Server spawned with PID: {}", child.id());
                // Poll until ready
                for _ in 0..30 {
                    tokio::time::sleep(Duration::from_millis(300)).await;
                    if Self::check_health(url, 300).await {
                        println!("[Tauri Rust] 🎉 Backend server is ONLINE!");
                        Self::attach_cli_service(url).await;
                        return true;
                    }
                }
                false
            }
            Err(e) => {
                eprintln!("[Tauri Rust] ❌ Failed to spawn server: {:?}", e);
                false
            }
        }
    }

    /// Background request to attach/ensure CLI agent service.
    async fn attach_cli_service(url: &str) {
        let endpoint = format!("{}/api/agent/service/ensure", url.trim_end_matches('/'));
        let client = reqwest::Client::new();
        let _ = client.post(&endpoint).send().await;
    }
}
