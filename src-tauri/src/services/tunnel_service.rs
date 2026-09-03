use crate::domain::models::tunnel::TunnelStatus;
use crate::domain::traits::ITunnelService;
use async_trait::async_trait;
use regex::Regex;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct TunnelService {
    state_file: PathBuf,
}

impl TunnelService {
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let state_file = home.join(".media-hub").join("tunnel_state.json");
        Self { state_file }
    }

    fn find_cloudflared() -> Option<String> {
        let candidates = [
            "/opt/homebrew/bin/cloudflared",
            "/usr/local/bin/cloudflared",
            "/usr/bin/cloudflared",
        ];
        for cand in candidates {
            if std::path::Path::new(cand).exists() {
                return Some(cand.to_string());
            }
        }
        None
    }

    fn is_pid_alive(pid: u32) -> bool {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "command="])
            .output();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            return s.contains("cloudflared");
        }
        false
    }
}

impl Default for TunnelService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ITunnelService for TunnelService {
    fn get_status(&self) -> TunnelStatus {
        let bin = Self::find_cloudflared();
        let installed = bin.is_some();

        if self.state_file.exists() {
            if let Ok(content) = fs::read_to_string(&self.state_file) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    let pid = val["pid"].as_u64().map(|p| p as u32);
                    let url = val["url"].as_str().map(|s| s.to_string());
                    let started_at = val["started_at"].as_str().map(|s| s.to_string());

                    if let Some(p) = pid {
                        if Self::is_pid_alive(p) {
                            return TunnelStatus {
                                binary: bin.unwrap_or_default(),
                                installed,
                                running: url.is_some(),
                                url,
                                started_at,
                                pid: Some(p),
                                error: None,
                            };
                        }
                    }
                }
            }
        }

        TunnelStatus {
            binary: bin.unwrap_or_else(|| "Chưa cài đặt (brew install cloudflared)".to_string()),
            installed,
            running: false,
            url: None,
            started_at: None,
            pid: None,
            error: None,
        }
    }

    fn start(&self, port: u16, force_new: bool) -> Result<TunnelStatus, String> {
        let current = self.get_status();
        if current.running && current.url.is_some() && !force_new {
            return Ok(current);
        }

        let bin = Self::find_cloudflared().ok_or_else(|| {
            "cloudflared chưa được cài đặt (brew install cloudflared)".to_string()
        })?;

        let _ = self.stop();

        let child = Command::new(&bin)
            .args(["tunnel", "--url", &format!("http://127.0.0.1:{}", port)])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Không khởi chạy được cloudflared: {}", e))?;

        let pid = child.id();
        std::thread::sleep(std::time::Duration::from_millis(1500));

        let log_file = self.state_file.with_file_name("tunnel.log");
        let mut discovered_url = None;

        if log_file.exists() {
            if let Ok(content) = fs::read_to_string(&log_file) {
                let re = Regex::new(r"https://([a-zA-Z0-9-]+)\.trycloudflare\.com").unwrap();
                for cap in re.captures_iter(&content) {
                    if let Some(sub) = cap.get(1) {
                        if sub.as_str() != "api" {
                            discovered_url =
                                Some(format!("https://{}.trycloudflare.com", sub.as_str()));
                        }
                    }
                }
            }
        }

        let started_at = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let status = TunnelStatus {
            binary: bin,
            installed: true,
            running: true,
            url: discovered_url.clone(),
            started_at: Some(started_at.clone()),
            pid: Some(pid),
            error: None,
        };

        let state_val = serde_json::json!({
            "pid": pid,
            "url": discovered_url,
            "port": port,
            "started_at": started_at,
        });
        let _ = fs::write(&self.state_file, state_val.to_string());

        Ok(status)
    }

    fn stop(&self) -> Result<TunnelStatus, String> {
        let _ = Command::new("pkill")
            .args(["-f", "cloudflared tunnel"])
            .output();
        let _ = fs::remove_file(&self.state_file);
        Ok(self.get_status())
    }
}
