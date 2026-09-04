use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

/// Google Drive khong co Plex/Jellyfin quet san nhu NAS, nen phai tu dung
/// bang tra cuu: doc file .nfo trong tung thu muc de lay <uniqueid> that.
///
/// Doc NFO dang tin hon doc ten thu muc: da gap truong hop ten thu muc ghi
/// {tvdb-72281} nhung NFO ghi tvdb 78864 -- tag trong ten co the cu hoac sai,
/// con NFO la thu Jellyfin/Plex thuc su dung de nhan dien.
pub fn start(home: PathBuf, settings: Arc<dyn ISettingsService>, job_store: Arc<JobStore>) {
    let cache = home.join("_app").join("cache").join("gdrive-nfo");
    std::thread::spawn(move || loop {
        crate::services::worker_status::begin("indexer/gdrive-nfo");
        match build_index(&settings, &cache) {
            Ok(rows) => match job_store.replace_library_source("gdrive", &rows) {
                Ok(n) => {
                    log::info!("[indexer/gdrive-nfo] {} muc", n);
                    // Gom lai ngay sau khi nguon nay doi, thay vi hen gio.
                    if let Err(e) = crate::services::library_aggregator::refresh_and_store(&job_store) {
                        log::warn!("[aggregator] khong ghi duoc bang da gom: {}", e);
                    }

                    crate::services::worker_status::ok(
                        "indexer/gdrive-nfo",
                        n as i64,
                        "doc .nfo tren Google Drive",
                    );
                }
                Err(e) => {
                    log::error!("[indexer/gdrive-nfo] loi ghi DB: {}", e);
                    crate::services::worker_status::err("indexer/gdrive-nfo", &e);
                }
            },
            Err(e) => {
                log::warn!("[indexer/gdrive-nfo] {}", e);
                crate::services::worker_status::err("indexer/gdrive-nfo", &e);
            }
        }
        std::thread::sleep(Duration::from_secs(900));
    });
}

type Row = (String, String, String, String, String, String, String);

fn build_index(settings: &Arc<dyn ISettingsService>, cache: &Path) -> Result<Vec<Row>, String> {
    let cfg = settings.load();
    if cfg.gdrive_remote.is_empty() {
        return Err("chua cau hinh gdrive_remote".into());
    }
    let rclone = which_rclone();
    let root = format!("{}:{}", cfg.gdrive_remote, cfg.gdrive_root.trim_matches('/'));

    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;

    // Do dau van tay truoc (ten + thoi diem sua cua cac .nfo) roi moi quyet dinh
    // co keo ve khong. `lsf` nhe hon nhieu so voi `copy`, nen 15 phut mot lan
    // gan nhu khong ton gi khi thu muc khong doi.
    let stamp_file = cache.join(".nfo-stamp");
    let listing = Command::new(&rclone)
        .arg("lsf")
        .arg("-R")
        .arg("--include")
        .arg("*.nfo")
        .arg("--format")
        .arg("pt")
        .arg(&root)
        .output()
        .map_err(|e| e.to_string())?;
    let stamp = if listing.status.success() {
        String::from_utf8_lossy(&listing.stdout).to_string()
    } else {
        String::new()
    };
    let unchanged = !stamp.is_empty()
        && std::fs::read_to_string(&stamp_file).unwrap_or_default() == stamp;

    // Goi rclone MOT lan de keo het .nfo ve. Truoc day moi thu muc mot lenh
    // `rclone cat`, do mat 38 giay/lan (nap config + xac thuc OAuth + di bo
    // tung cap) nen 28 thu muc thanh ~18 phut, lau hon ca chu ky lap.
    let out = if unchanged {
        // Khong doi -> dung ban cache san co, khong goi copy.
        Command::new("true").output().map_err(|e| e.to_string())?
    } else {
        Command::new(&rclone)
        .arg("copy")
        .arg(&root)
        .arg(cache)
        .arg("--include")
        .arg("*.nfo")
        .arg("--max-depth")
        .arg("3")
        .arg("--transfers")
        .arg("8")
        .arg("--checkers")
        .arg("8")
        .output()
        .map_err(|e| e.to_string())?
    };
    if !out.status.success() {
        return Err(format!(
            "rclone copy that bai: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    if !unchanged && !stamp.is_empty() {
        let _ = std::fs::write(&stamp_file, &stamp);
    }

    // Doc ban cache local, khong cham toi Drive nua.
    let mut rows = Vec::new();
    for (sub, mtype) in [("TV Shows", "series"), ("Movies", "movie")] {
        let dir = cache.join(sub);
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let folder_path = entry.path();
            if !folder_path.is_dir() {
                continue;
            }
            let folder = entry.file_name().to_string_lossy().to_string();
            if folder.starts_with('.') {
                continue;
            }

            let content = ["tvshow.nfo", "movie.nfo"]
                .iter()
                .filter_map(|n| std::fs::read_to_string(folder_path.join(n)).ok())
                .next();

            let remote_path = format!("{}/{}/{}", root, sub, folder);
            match content {
                Some(xml) => {
                    let title =
                        extract_tag(&xml, "title").unwrap_or_else(|| clean_title(&folder));
                    let mut found = false;
                    for (kind, prefix) in [("tvdb", "tvdb-"), ("tmdb", "tmdb-")] {
                        if let Some(id) = extract_uniqueid(&xml, kind) {
                            rows.push((
                                format!("{}{}", prefix, id),
                                String::new(),
                                title.clone(),
                                folder.clone(),
                                mtype.to_string(),
                                remote_path.clone(),
                                remote_path.clone(),
                            ));
                            found = true;
                        }
                    }
                    if !found {
                        if let Some(k) = key_from_folder_name(&folder) {
                            rows.push((
                                k,
                                String::new(),
                                title,
                                folder.clone(),
                                mtype.to_string(),
                                remote_path.clone(),
                                remote_path.clone(),
                            ));
                        }
                    }
                }
                None => {
                    // Khong co NFO -> quay ve doc id tu ten thu muc.
                    if let Some(k) = key_from_folder_name(&folder) {
                        rows.push((
                            k,
                            String::new(),
                            clean_title(&folder),
                            folder.clone(),
                            mtype.to_string(),
                            remote_path.clone(),
                            remote_path.clone(),
                        ));
                    }
                }
            }
        }
    }

    Ok(rows)
}



fn extract_uniqueid(xml: &str, kind: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r#"(?is)<uniqueid[^>]*type\s*=\s*["']{}["'][^>]*>\s*([0-9]+)\s*</uniqueid>"#,
        regex::escape(kind)
    ))
    .ok()?;
    re.captures(xml).map(|c| c[1].to_string())
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let re = Regex::new(&format!(r"(?is)<{0}>\s*(.*?)\s*</{0}>", regex::escape(tag))).ok()?;
    re.captures(xml)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty())
}

fn key_from_folder_name(folder: &str) -> Option<String> {
    let tvdb = Regex::new(r"(?i)\{tvdb-(\d+)\}").ok()?;
    let tmdb = Regex::new(r"(?i)\{tmdb-(\d+)\}").ok()?;
    if let Some(c) = tvdb.captures(folder) {
        return Some(format!("tvdb-{}", &c[1]));
    }
    if let Some(c) = tmdb.captures(folder) {
        return Some(format!("tmdb-{}", &c[1]));
    }
    None
}

fn clean_title(folder: &str) -> String {
    Regex::new(r"[\{\[][^\}\]]*[\}\]]")
        .map(|re| re.replace_all(folder, "").trim().to_string())
        .unwrap_or_else(|_| folder.to_string())
}

fn which_rclone() -> String {
    for c in ["/opt/homebrew/bin/rclone", "/usr/local/bin/rclone"] {
        if Path::new(c).exists() {
            return c.to_string();
        }
    }
    if let Some(home) = dirs::home_dir() {
        let p: PathBuf = home.join(".local/bin/rclone");
        if p.exists() {
            return p.to_string_lossy().to_string();
        }
    }
    "rclone".to_string()
}
