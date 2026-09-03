use crate::domain::traits::ISettingsService;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct NasService {
    settings: Arc<dyn ISettingsService>,
    cache: Mutex<Option<(Instant, Vec<String>)>>,
}

fn expand_tilde(p: &str) -> std::path::PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    std::path::PathBuf::from(p)
}

impl NasService {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings,
            cache: Mutex::new(None),
        }
    }

    pub fn list_nas_folders(&self) -> Vec<String> {
        let mut cache_guard = self.cache.lock().unwrap();
        if let Some((at, ref folders)) = *cache_guard {
            if at.elapsed() < Duration::from_secs(300) {
                return folders.clone();
            }
        }

        let cfg = self.settings.load();
        if cfg.nas_host.is_empty() || cfg.nas_path.is_empty() {
            return Vec::new();
        }

        let mut cmd = Command::new("ssh");
        cmd.arg("-p")
            .arg(cfg.nas_port.to_string())
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("ConnectTimeout=4")
            .arg("-o")
            .arg("StrictHostKeyChecking=no");

        if !cfg.nas_ssh_key.is_empty() {
            let expanded = expand_tilde(&cfg.nas_ssh_key);
            if expanded.exists() {
                cmd.arg("-i").arg(expanded);
            }
        }


        let user_host = format!("{}@{}", cfg.nas_user, cfg.nas_host);
        let ls_cmd = format!("ls -1 \"{}\"", cfg.nas_path);
        cmd.arg(user_host).arg(ls_cmd);

        let mut folders = Vec::new();
        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        folders.push(trimmed.to_string());
                    }
                }
            }
        }

        *cache_guard = Some((Instant::now(), folders.clone()));
        folders
    }

    pub fn scan_nas(
        &self,
        host: &str,
        user: &str,
        port: u16,
        key: &str,
        custom_path: &str,
    ) -> Result<Vec<String>, String> {
        if host.is_empty() {
            return Err("Thiếu địa chỉ IP NAS".to_string());
        }

        let mut cmd = Command::new("ssh");
        cmd.arg("-p")
            .arg(port.to_string())
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("ConnectTimeout=5")
            .arg("-o")
            .arg("StrictHostKeyChecking=no");

        if !key.is_empty() {
            let exp = expand_tilde(key);
            if exp.exists() {
                cmd.arg("-i").arg(exp);
            }
        } else {
            for k in ["id_ed25519", "id_rsa"] {
                if let Some(home) = dirs::home_dir() {
                    let p = home.join(".ssh").join(k);
                    if p.is_file() {
                        cmd.arg("-i").arg(p);
                        break;
                    }
                }
            }
        }

        let candidate_paths = [
            custom_path,
            "/srv/mergerfs/MainPool/Phim/TV Shows",
            "/srv/mergerfs/MainPool/Phim/Movies",
            "/srv/mergerfs/MainPool/Phim",
            "/volume1/video/TV Shows",
            "/volume1/video/Movies",
            "/volume1/Media",
            "/volume1/Plex",
        ];

        let mut script_parts = Vec::new();
        for p in candidate_paths {
            if !p.is_empty() {
                script_parts.push(format!("if [ -d \"{}\" ]; then printf \"FOUND:%s\\n\" \"{}\"; fi", p, p));
            }
        }
        let remote_script = script_parts.join("; ");

        cmd.arg(format!("{}@{}", user, host)).arg(remote_script);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let mut found = Vec::new();
                    for line in stdout.lines() {
                        if let Some(rest) = line.strip_prefix("FOUND:") {
                            found.push(rest.trim().to_string());
                        }
                    }
                    Ok(found)
                } else {
                    let err = String::from_utf8_lossy(&output.stderr);
                    Err(if err.trim().is_empty() {
                        "Lỗi kết nối SSH tới NAS".to_string()
                    } else {
                        err.trim().to_string()
                    })
                }
            }
            Err(e) => Err(format!("Lỗi chạy tiến trình SSH: {}", e)),
        }
    }
}
