use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

/// Mot dong index truoc khi ghi xuong DB.
/// (media_key, franchise, title, folder, media_type, path)
type Row = (String, String, String, String, String, String);

/// Khoa doi chieu giua 3 nguon.
///
/// NAS va Drive luu phang theo chuan Plex nen ban than chung khong mang thong
/// tin franchise. Nhung ten thu muc o ca 3 noi deu da co san {tvdb-...} hoac
/// {tmdb-...}, nen ta doi chieu bang ID thay vi so khop ten -- ben vung truoc
/// khac biet ve cach viet ten, nam phat hanh hay khoang trang.
fn media_key(folder: &str) -> String {
    let tvdb = Regex::new(r"(?i)\{tvdb-(\d+)\}").unwrap();
    let tmdb = Regex::new(r"(?i)\{tmdb-(\d+)\}").unwrap();

    if let Some(c) = tvdb.captures(folder) {
        return format!("tvdb-{}", &c[1]);
    }
    if let Some(c) = tmdb.captures(folder) {
        return format!("tmdb-{}", &c[1]);
    }
    // Khong co ID -> dung ten da chuan hoa lam khoa du phong.
    let cleaned = Regex::new(r"[\{\[][^\}\]]*[\}\]]")
        .unwrap()
        .replace_all(folder, "");
    format!("name-{}", cleaned.trim().to_lowercase())
}

fn clean_title(folder: &str) -> String {
    Regex::new(r"[\{\[][^\}\]]*[\}\]]")
        .unwrap()
        .replace_all(folder, "")
        .trim()
        .to_string()
}

fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}

// ============================ NGUON 1: LOCAL ============================

/// Quet _franchise/<Ten>/{Movies,TV Shows}/<Tieu de>
/// Day la nguon DUY NHAT biet franchise -- hai nguon kia se muon lai qua media_key.
pub fn index_local(home: &Path) -> Vec<Row> {
    let root = home.join("_franchise");
    let mut rows = Vec::new();

    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(_) => return rows,
    };

    for fr in entries.flatten() {
        let fr_path = fr.path();
        if !fr_path.is_dir() {
            continue;
        }
        let fr_name = fr.file_name().to_string_lossy().to_string();
        if fr_name.starts_with('.') || fr_name.starts_with('_') {
            continue;
        }

        for (sub, mtype) in [("TV Shows", "series"), ("Movies", "movie")] {
            let dir = fr_path.join(sub);
            if let Ok(items) = std::fs::read_dir(&dir) {
                for it in items.flatten() {
                    if !it.path().is_dir() {
                        continue;
                    }
                    let folder = it.file_name().to_string_lossy().to_string();
                    if folder.starts_with('.') {
                        continue;
                    }
                    rows.push((
                        media_key(&folder),
                        fr_name.clone(),
                        clean_title(&folder),
                        folder,
                        mtype.to_string(),
                        it.path().to_string_lossy().to_string(),
                    ));
                }
            }
        }
    }
    rows
}

// ============================ NGUON 2: NAS ============================

pub fn index_nas(settings: &Arc<dyn ISettingsService>) -> Vec<Row> {
    let cfg = settings.load();
    if cfg.nas_host.is_empty() || cfg.nas_path.is_empty() {
        return Vec::new();
    }

    let mut rows = Vec::new();
    for (sub, mtype) in [("TV Shows", "series"), ("Movies", "movie")] {
        let remote_dir = format!("{}/{}", cfg.nas_path.trim_end_matches('/'), sub);

        let mut cmd = Command::new("ssh");
        cmd.arg("-p")
            .arg(cfg.nas_port.to_string())
            .arg("-o")
            .arg("BatchMode=yes")
            .arg("-o")
            .arg("ConnectTimeout=6")
            .arg("-o")
            .arg("StrictHostKeyChecking=no");

        if !cfg.nas_ssh_key.is_empty() {
            let key = expand_tilde(&cfg.nas_ssh_key);
            if key.exists() {
                cmd.arg("-i").arg(key);
            }
        }

        cmd.arg(format!("{}@{}", cfg.nas_user, cfg.nas_host))
            .arg(format!("ls -1 \"{}\"", remote_dir));

        if let Ok(out) = cmd.output() {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let folder = line.trim();
                    if folder.is_empty() || folder.starts_with('.') {
                        continue;
                    }
                    rows.push((
                        media_key(folder),
                        String::new(), // NAS phang -> chua biet franchise
                        clean_title(folder),
                        folder.to_string(),
                        mtype.to_string(),
                        format!("{}/{}", remote_dir, folder),
                    ));
                }
            }
        }
    }
    rows
}

// ============================ NGUON 3: GOOGLE DRIVE ============================

pub fn index_gdrive(settings: &Arc<dyn ISettingsService>) -> Vec<Row> {
    let cfg = settings.load();
    if cfg.gdrive_remote.is_empty() {
        return Vec::new();
    }

    let rclone = which_rclone();
    let mut rows = Vec::new();

    for (sub, mtype) in [("TV Shows", "series"), ("Movies", "movie")] {
        let remote = format!(
            "{}:{}/{}",
            cfg.gdrive_remote,
            cfg.gdrive_root.trim_matches('/'),
            sub
        );

        let out = Command::new(&rclone)
            .arg("lsf")
            .arg("--dirs-only")
            .arg("--max-depth")
            .arg("1")
            .arg(&remote)
            .output();

        if let Ok(out) = out {
            if out.status.success() {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    let folder = line.trim().trim_end_matches('/');
                    if folder.is_empty() || folder.starts_with('.') {
                        continue;
                    }
                    rows.push((
                        media_key(folder),
                        String::new(), // Drive phang -> chua biet franchise
                        clean_title(folder),
                        folder.to_string(),
                        mtype.to_string(),
                        format!("{}/{}", remote, folder),
                    ));
                }
            }
        }
    }
    rows
}

fn which_rclone() -> String {
    for c in [
        "/opt/homebrew/bin/rclone",
        "/usr/local/bin/rclone",
    ] {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p = home.join(".local/bin/rclone");
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    "rclone".to_string()
}

// ============================ WORKER NEN ============================

/// Ba worker chay doc lap, moi thang chi lo dung nguon cua minh va ghi
/// vao bang library_index. Nguon nao hong/cham thi chi nguon do thieu du
/// lieu, khong keo sap ca thu vien.
pub fn start(
    home: PathBuf,
    settings: Arc<dyn ISettingsService>,
    job_store: Arc<JobStore>,
) {
    // Local: quet nhanh, chay day hon.
    {
        let home = home.clone();
        let js = job_store.clone();
        std::thread::spawn(move || loop {
            let rows = index_local(&home);
            match js.replace_library_source("local", &rows) {
                Ok(n) => log::info!("[indexer/local] {} muc", n),
                Err(e) => log::error!("[indexer/local] loi: {}", e),
            }
            std::thread::sleep(Duration::from_secs(60));
        });
    }

    // NAS: qua SSH nen thua hon.
    {
        let st = settings.clone();
        let js = job_store.clone();
        std::thread::spawn(move || loop {
            let rows = index_nas(&st);
            match js.replace_library_source("nas", &rows) {
                Ok(n) => log::info!("[indexer/nas] {} muc", n),
                Err(e) => log::error!("[indexer/nas] loi: {}", e),
            }
            std::thread::sleep(Duration::from_secs(600));
        });
    }

    // Google Drive: goi rclone, ton quota nen thua nhat.
    {
        let st = settings.clone();
        let js = job_store.clone();
        std::thread::spawn(move || loop {
            let rows = index_gdrive(&st);
            match js.replace_library_source("gdrive", &rows) {
                Ok(n) => log::info!("[indexer/gdrive] {} muc", n),
                Err(e) => log::error!("[indexer/gdrive] loi: {}", e),
            }
            std::thread::sleep(Duration::from_secs(900));
        });
    }
}
