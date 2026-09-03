use crate::domain::traits::ISettingsService;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct GDriveService {
    settings: Arc<dyn ISettingsService>,
    cache: Mutex<Option<(Instant, Vec<String>)>>,
}

impl GDriveService {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings,
            cache: Mutex::new(None),
        }
    }

    fn rclone_bin() -> String {
        if std::path::Path::new("/opt/homebrew/bin/rclone").exists() {
            "/opt/homebrew/bin/rclone".to_string()
        } else {
            "rclone".to_string()
        }
    }

    pub fn list_tv_shows(&self, force_refresh: bool) -> Vec<String> {
        let mut cache_guard = self.cache.lock().unwrap();
        if !force_refresh {
            if let Some((at, ref shows)) = *cache_guard {
                if at.elapsed() < Duration::from_secs(300) {
                    return shows.clone();
                }
            }
        }

        let cfg = self.settings.load();
        let remote = cfg.gdrive_remote;
        let root = cfg.gdrive_root;
        let path = format!("{}:{}/TV Shows", remote, root.trim_start_matches('/'));

        let mut shows = Vec::new();
        if let Ok(output) = Command::new(Self::rclone_bin())
            .arg("lsd")
            .arg(&path)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let name = parts[4..].join(" ");
                        shows.push(name);
                    }
                }
            }
        }

        *cache_guard = Some((Instant::now(), shows.clone()));
        shows
    }

    pub fn get_season_files(&self, show: &str, season: &str) -> Vec<String> {
        let cfg = self.settings.load();
        let remote = cfg.gdrive_remote;
        let root = cfg.gdrive_root;
        let path = format!(
            "{}:{}/TV Shows/{}/{}",
            remote,
            root.trim_start_matches('/'),
            show,
            season
        );

        let mut files = Vec::new();
        if let Ok(output) = Command::new(Self::rclone_bin())
            .arg("lsf")
            .arg(&path)
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    let f = line.trim();
                    if !f.is_empty() && !f.ends_with('/') {
                        files.push(f.to_string());
                    }
                }
            }
        }
        files
    }

    pub fn check_connection(&self, remote: &str, root: &str) -> Result<Vec<String>, String> {
        let path = format!("{}:{}", remote, root.trim_start_matches('/'));
        let output = Command::new(Self::rclone_bin())
            .arg("lsd")
            .arg(&path)
            .output()
            .map_err(|e| format!("Không thể thực thi rclone: {}", e))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let mut dirs = Vec::new();
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 5 {
                    dirs.push(parts[4..].join(" "));
                }
            }
            Ok(dirs)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            Err(if err.trim().is_empty() {
                "Lỗi kết nối rclone".to_string()
            } else {
                err.trim().to_string()
            })
        }
    }
}
