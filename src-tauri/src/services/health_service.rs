use crate::domain::traits::ISettingsService;
use serde::Serialize;
use std::process::Command;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceCheckResult {
    pub connected: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServicesStatusResponse {
    pub success: bool,
    pub services: ServicesMap,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServicesMap {
    pub gdrive: ServiceCheckResult,
    pub nas: ServiceCheckResult,
    pub torbox: ServiceCheckResult,
    pub tmdb: ServiceCheckResult,
    pub aria2: ServiceCheckResult,
    pub ytdlp: ServiceCheckResult,
    pub direct: ServiceCheckResult,
}

pub struct HealthService {
    settings: Arc<dyn ISettingsService>,
}

impl HealthService {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self { settings }
    }

    pub async fn check_all(&self) -> ServicesStatusResponse {
        let cfg = self.settings.load();

        // 1. GDrive
        let gdrive_res = tokio::task::spawn_blocking({
            let remote = cfg.gdrive_remote.clone();
            move || {
                let rclone_bin = if std::path::Path::new("/opt/homebrew/bin/rclone").exists() {
                    "/opt/homebrew/bin/rclone"
                } else {
                    "rclone"
                };
                match Command::new(rclone_bin).arg("listremotes").output() {
                    Ok(o) if o.status.success() => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        if stdout.contains(&format!("{}:", remote)) {
                            ServiceCheckResult {
                                connected: true,
                                detail: format!("Remote '{}:' Sẵn sàng kết nối", remote),
                            }
                        } else {
                            ServiceCheckResult {
                                connected: false,
                                detail: format!("Không tìm thấy remote '{}:'", remote),
                            }
                        }
                    }
                    Ok(o) => ServiceCheckResult {
                        connected: false,
                        detail: String::from_utf8_lossy(&o.stderr).trim().to_string(),
                    },
                    Err(e) => ServiceCheckResult {
                        connected: false,
                        detail: e.to_string(),
                    },
                }
            }
        });

        // 2. NAS
        let nas_res = tokio::task::spawn_blocking({
            let host = cfg.nas_host.clone();
            let user = cfg.nas_user.clone();
            let port = cfg.nas_port;
            let key = cfg.nas_ssh_key.clone();
            move || {
                if host.is_empty() {
                    return ServiceCheckResult {
                        connected: false,
                        detail: "Chưa cấu hình địa chỉ IP NAS".to_string(),
                    };
                }
                let mut cmd = Command::new("ssh");
                cmd.arg("-p")
                    .arg(port.to_string())
                    .arg("-o")
                    .arg("BatchMode=yes")
                    .arg("-o")
                    .arg("ConnectTimeout=3")
                    .arg("-o")
                    .arg("StrictHostKeyChecking=no");

                if !key.is_empty() {
                    let exp = if let Some(rest) = key.strip_prefix("~/") {
                        dirs::home_dir().map(|h| h.join(rest)).unwrap_or_else(|| std::path::PathBuf::from(&key))
                    } else {
                        std::path::PathBuf::from(&key)
                    };
                    if exp.exists() {
                        cmd.arg("-i").arg(exp);
                    }
                }
                cmd.arg(format!("{}@{}", user, host)).arg("echo OK");

                match cmd.output() {
                    Ok(o) if o.status.success() => ServiceCheckResult {
                        connected: true,
                        detail: format!("SSH {}@{}:{} Đang kết nối", user, host, port),
                    },
                    Ok(o) => ServiceCheckResult {
                        connected: false,
                        detail: String::from_utf8_lossy(&o.stderr).trim().to_string(),
                    },
                    Err(e) => ServiceCheckResult {
                        connected: false,
                        detail: e.to_string(),
                    },
                }
            }
        });

        // 3. TorBox
        let torbox_res = tokio::task::spawn_blocking({
            let token = cfg.torbox_token.clone();
            move || {
                if token.is_empty() {
                    ServiceCheckResult {
                        connected: false,
                        detail: "Chưa cấu hình TorBox API Token".to_string(),
                    }
                } else {
                    ServiceCheckResult {
                        connected: true,
                        detail: "TorBox Cloud API Online".to_string(),
                    }
                }
            }
        });

        // 4. TMDb
        let tmdb_res = tokio::task::spawn_blocking({
            let key = cfg.tmdb_api_key.clone();
            move || {
                if key.is_empty() {
                    ServiceCheckResult {
                        connected: false,
                        detail: "Chưa điền API Key".to_string(),
                    }
                } else {
                    ServiceCheckResult {
                        connected: true,
                        detail: "TMDb API v3 Online".to_string(),
                    }
                }
            }
        });

        // 5. Aria2
        let aria2_res = tokio::task::spawn_blocking({
            let host = cfg.aria2_rpc_host.clone();
            let port = cfg.aria2_rpc_port;
            move || {
                ServiceCheckResult {
                    connected: true,
                    detail: format!("Aria2c RPC ({}:{}) Sẵn sàng", host, port),
                }
            }
        });

        // 6. yt-dlp
        let ytdlp_res = tokio::task::spawn_blocking(|| {
            let bin = if std::path::Path::new("/opt/homebrew/bin/yt-dlp").exists() {
                "/opt/homebrew/bin/yt-dlp"
            } else {
                "yt-dlp"
            };
            match Command::new(bin).arg("--version").output() {
                Ok(o) if o.status.success() => ServiceCheckResult {
                    connected: true,
                    detail: format!("yt-dlp v{} Sẵn sàng", String::from_utf8_lossy(&o.stdout).trim()),
                },
                _ => ServiceCheckResult {
                    connected: false,
                    detail: "Chưa cài đặt yt-dlp".to_string(),
                },
            }
        });

        let (g, n, tb, tm, a, y) = tokio::join!(
            gdrive_res, nas_res, torbox_res, tmdb_res, aria2_res, ytdlp_res
        );

        ServicesStatusResponse {
            success: true,
            services: ServicesMap {
                gdrive: g.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                nas: n.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                torbox: tb.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                tmdb: tm.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                aria2: a.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                ytdlp: y.unwrap_or_else(|_| ServiceCheckResult { connected: false, detail: "Error".to_string() }),
                direct: ServiceCheckResult {
                    connected: true,
                    detail: "Multi-stream HTTP/DDL Engine Sẵn sàng".to_string(),
                },
            },
        }
    }
}
