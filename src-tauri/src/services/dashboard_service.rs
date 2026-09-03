use crate::domain::models::dashboard::{
    CloudStorageInfo, DashboardOverview, LocalDiskProbe, SystemHealth, TransferCard,
};
use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct DashboardService {
    settings: Arc<dyn ISettingsService>,
    job_store: Arc<JobStore>,
    cache: Mutex<Option<(Instant, DashboardOverview)>>,
}

fn fmt_bytes(n: u64) -> String {
    let mut num = n as f64;
    for unit in ["B", "KB", "MB", "GB", "TB", "PB"] {
        if num < 1024.0 {
            return format!("{:.1} {}", num, unit);
        }
        num /= 1024.0;
    }
    format!("{:.1} EB", num)
}

impl DashboardService {
    pub fn new(settings: Arc<dyn ISettingsService>, job_store: Arc<JobStore>) -> Self {
        Self {
            settings,
            job_store,
            cache: Mutex::new(None),
        }
    }

    pub fn probe_local_disk(&self, staging_dir: &str) -> LocalDiskProbe {
        let probe_path = if std::path::Path::new(staging_dir).exists() {
            staging_dir
        } else {
            "/"
        };

        if let Ok(output) = Command::new("df").arg("-Pk").arg(probe_path).output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 5 {
                        let total_kb: f64 = parts[1].parse().unwrap_or(0.0);
                        let used_kb: f64 = parts[2].parse().unwrap_or(0.0);
                        let free_kb: f64 = parts[3].parse().unwrap_or(0.0);
                        let total_gb = (total_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                        let used_gb = (used_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                        let free_gb = (free_kb / 1024.0 / 1024.0 * 10.0).round() / 10.0;
                        let pct = if total_kb > 0.0 {
                            ((used_kb / total_kb) * 100.0).round() as u32
                        } else {
                            0
                        };

                        return LocalDiskProbe {
                            name: format!("Ổ đệm ({})", probe_path),
                            path: staging_dir.to_string(),
                            total_gb: Some(total_gb),
                            used_gb: Some(used_gb),
                            free_gb: Some(free_gb),
                            percent: pct,
                            measured: true,
                            error: None,
                        };
                    }
                }
            }
        }

        LocalDiskProbe {
            name: "Ổ đệm".to_string(),
            path: staging_dir.to_string(),
            total_gb: None,
            used_gb: None,
            free_gb: None,
            percent: 0,
            measured: false,
            error: Some("Không đo được dung lượng đĩa".to_string()),
        }
    }

    pub fn probe_memory(&self) -> (String, String, u32) {
        let total_bytes = Command::new("sysctl")
            .arg("-n")
            .arg("hw.memsize")
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u64>().ok())
            .unwrap_or(0);

        let mut used_bytes = 0u64;
        if let Ok(vm) = Command::new("vm_stat").output() {
            let out = String::from_utf8_lossy(&vm.stdout);
            let mut active = 0u64;
            let mut wired = 0u64;
            let mut compressed = 0u64;

            for line in out.lines() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let k = parts[0].trim();
                    let v = parts[1].trim().trim_end_matches('.').parse::<u64>().unwrap_or(0);
                    if k.contains("Pages active") {
                        active = v;
                    } else if k.contains("Pages wired down") {
                        wired = v;
                    } else if k.contains("Pages occupied by compressor") {
                        compressed = v;
                    }
                }
            }
            used_bytes = (active + wired + compressed) * 4096;
        }

        if total_bytes > 0 {
            let total_gb = format!("{:.1} GB", total_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
            let used_gb = format!("{:.1} GB", used_bytes as f64 / 1024.0 / 1024.0 / 1024.0);
            let pct = ((used_bytes as f64 / total_bytes as f64) * 100.0).round() as u32;
            (total_gb, used_gb, pct)
        } else {
            ("—".to_string(), "—".to_string(), 0)
        }
    }

    pub fn get_overview(&self) -> DashboardOverview {
        let mut cache_guard = self.cache.lock().unwrap();
        if let Some((at, ref data)) = *cache_guard {
            if at.elapsed() < Duration::from_secs(20) {
                return data.clone();
            }
        }

        let cfg = self.settings.load();
        let disk = self.probe_local_disk(&cfg.staging_dir);
        let (ram_total, ram_used, ram_pct) = self.probe_memory();

        // CPU Load average
        let mut load_str = "—".to_string();
        let mut loadavg: [f64; 3] = [0.0, 0.0, 0.0];
        unsafe {
            if libc_getloadavg(&mut loadavg) > 0 {
                load_str = format!("{:.2}", loadavg[0]);
            }
        }

        let clouds = vec![
            CloudStorageInfo {
                id: "gdrive".to_string(),
                icon: "☁️".to_string(),
                name: format!("Google Drive ({}:)", cfg.gdrive_remote),
                path: format!("{}:{}", cfg.gdrive_remote, cfg.gdrive_root),
                connected: true,
                used_str: "Không giới hạn".to_string(),
                avail_str: "Không giới hạn".to_string(),
                total_str: "Unlimited".to_string(),
                percent: 0,
                badge: "Plex Main Cloud".to_string(),
            },
            CloudStorageInfo {
                id: "nas".to_string(),
                icon: "🖥️".to_string(),
                name: "NAS Storage".to_string(),
                path: cfg.nas_path.clone(),
                connected: !cfg.nas_host.is_empty(),
                used_str: "Sẵn sàng".to_string(),
                avail_str: "Mạng Nội Bộ".to_string(),
                total_str: "—".to_string(),
                percent: 0,
                badge: "Mạng Nội Bộ".to_string(),
            },
        ];

        let active_jobs = self.job_store.list_active();
        let mut downloads = Vec::new();
        let mut uploads = Vec::new();

        for j in active_jobs {
            let card = TransferCard {
                job_id: j.id,
                name: Some(j.name.clone()),
                title: Some(j.name.clone()),
                engine: Some("TorBox Cloud DDL".to_string()),
                dest: Some(j.targets.join(" + ")),
                dest_short: Some(j.targets.join(" + ")),
                dest_path: Some(j.staging_path),
                progress: j.progress,
                speed: Some(format!("{}/s", fmt_bytes(j.speed_bps))),
                eta: Some("—".to_string()),
                current_ep: Some(j.done_targets.len()),
                total_ep: Some(j.targets.len()),
                message: j.message,
            };
            if j.phase == crate::domain::models::job::JobPhase::Upload {
                uploads.push(card);
            } else {
                downloads.push(card);
            }
        }

        let counts = self.job_store.counts();
        let overview = DashboardOverview {
            success: true,
            measured_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64(),
            health: SystemHealth {
                cpu_load: load_str,
                ram_total_gb: ram_total,
                ram_used_gb: ram_used,
                ram_pct,
                local_disk: disk,
            },
            clouds,
            active_downloads: downloads,
            active_uploads: uploads,
            recent_media: Vec::new(),
            job_counts: serde_json::to_value(counts).unwrap_or_default(),
        };

        *cache_guard = Some((Instant::now(), overview.clone()));
        overview
    }
}

unsafe fn libc_getloadavg(loadavg: &mut [f64; 3]) -> i32 {
    extern "C" {
        fn getloadavg(loadavg: *mut f64, nelem: i32) -> i32;
    }
    getloadavg(loadavg.as_mut_ptr(), 3)
}
