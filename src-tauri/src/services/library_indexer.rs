use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

/// Mot dong index truoc khi ghi xuong DB.
/// (media_key, franchise, title, folder, media_type, path)
type Row = (String, String, String, String, String, String, String);

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
                    let full = it.path().to_string_lossy().to_string();
                    rows.push((
                        media_key(&folder),
                        fr_name.clone(),
                        clean_title(&folder),
                        folder,
                        mtype.to_string(),
                        full.clone(),
                        full, // item_uid: duong dan la dinh danh duy nhat cua title
                    ));
                }
            }
        }
    }
    rows
}

// NGUON 2 (NAS) va 3 (Drive) da chuyen sang jellyfin_indexer / plex_indexer
// / gdrive_nfo_indexer -- chung lay ID that thay vi doan tu ten thu muc.

// ============================ WORKER NEN ============================

/// Ba worker chay doc lap, moi thang chi lo dung nguon cua minh va ghi
/// vao bang library_index. Nguon nao hong/cham thi chi nguon do thieu du
/// lieu, khong keo sap ca thu vien.
pub fn start(
    home: PathBuf,
    _settings: Arc<dyn ISettingsService>,
    job_store: Arc<JobStore>,
) {
    // Local chu yeu chay theo su kien tu watcher_service (notify). Vong lap nay
    // chi la luoi an toan phong khi notify bo sot su kien, nen de thua tay.
    {
        let home = home.clone();
        let js = job_store.clone();
        std::thread::spawn(move || loop {
            crate::services::worker_status::begin("indexer/local");
            let rows = index_local(&home);
            match js.replace_library_source("local", &rows) {
                Ok(n) => {
                    log::info!("[indexer/local] {} muc", n);
                    crate::services::worker_status::ok("indexer/local", n as i64, "quet _franchise/");
                    if let Err(e) = crate::services::library_aggregator::refresh_and_store(&js) {
                        log::warn!("[aggregator] khong ghi duoc bang da gom: {}", e);
                    }
                }
                Err(e) => {
                    log::error!("[indexer/local] loi: {}", e);
                    crate::services::worker_status::err("indexer/local", &e);
                }
            }
            std::thread::sleep(Duration::from_secs(900));
        });
    }

    // Nguon NAS doc ten thu muc da bo: no sinh khoa du phong "name-<ten>"
    // cho thu muc khong co tag {tvdb-}, ma khoa do khong bao gio khop voi
    // khoa ID that tu Jellyfin/Plex -> mot phim bi dem hai lan.
    // Gio NAS do jellyfin_indexer va plex_indexer dam nhiem (co ID that).

    // Google Drive do gdrive_nfo_indexer dam nhiem: no doc <uniqueid> trong
    // file .nfo that thay vi doan id tu ten thu muc, chinh xac hon.
}
