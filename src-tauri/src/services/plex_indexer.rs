use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

const REMOTE_DB: &str =
    "/appdata/plex/database/Library/Application Support/Plex Media Server/Plug-in Support/Databases/com.plexapp.plugins.library.db";

/// Worker theo doi DB cua Plex tren NAS.
///
/// Plex bo sung cho Jellyfin: hai ben quet cung thu vien NAS nhung nhan dien
/// khac nhau, nen title ben nay chua nhan ra thi ben kia co the da co.
///
/// Plex khong dung de lay franchise: kiem tra tags_collection cho ket qua 0
/// muc -- thu vien nay khong dat collection nao trong Plex.
pub fn start(home: PathBuf, settings: Arc<dyn ISettingsService>, job_store: Arc<JobStore>) {
    std::thread::spawn(move || {
        let cache_dir = home.join("_app").join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let local_db = cache_dir.join("plex.db");
        let stamp_file = cache_dir.join("plex.db.stamp");

        loop {
            crate::services::worker_status::begin("indexer/plex");
            match sync_once(&settings, &local_db, &stamp_file, &job_store) {
                Ok(Some(n)) => {
                    log::info!("[indexer/plex] cap nhat: {} muc", n);
                    // Gom lai ngay sau khi nguon nay doi, thay vi hen gio.
                    if let Err(e) = crate::services::library_aggregator::refresh_and_store(&job_store) {
                        log::warn!("[aggregator] khong ghi duoc bang da gom: {}", e);
                    }

                    crate::services::worker_status::ok(
                        "indexer/plex",
                        n as i64,
                        "doc com.plexapp.plugins.library.db",
                    );
                }
                Ok(None) => {
                    crate::services::worker_status::ok("indexer/plex", -1, "DB khong doi, bo qua");
                }
                Err(e) => {
                    log::warn!("[indexer/plex] {}", e);
                    crate::services::worker_status::err("indexer/plex", &e);
                }
            }
            std::thread::sleep(Duration::from_secs(300));
        }
    });
}

fn sync_once(
    settings: &Arc<dyn ISettingsService>,
    local_db: &Path,
    stamp_file: &Path,
    job_store: &Arc<JobStore>,
) -> Result<Option<usize>, String> {
    let cfg = settings.load();
    if cfg.nas_host.is_empty() {
        return Err("chua cau hinh nas_host".into());
    }

    let stat = ssh_capture(
        settings,
        &format!("sudo -n stat -c '%Y %s' '{}' 2>/dev/null", REMOTE_DB),
    )?;
    let stamp = stat.trim().to_string();
    if stamp.is_empty() {
        return Err("khong doc duoc plex db tren NAS".into());
    }

    let old = std::fs::read_to_string(stamp_file).unwrap_or_default();
    if old.trim() == stamp && local_db.exists() {
        return Ok(None);
    }

    ssh_capture(
        settings,
        &format!(
            "sudo -n cp '{}' /tmp/plex-index.db 2>/dev/null; sudo -n chmod a+r /tmp/plex-index.db; echo ok",
            REMOTE_DB
        ),
    )?;

    let key = expand_tilde(&cfg.nas_ssh_key);
    let mut scp = Command::new("scp");
    scp.arg("-P").arg(cfg.nas_port.to_string())
        .arg("-o").arg("BatchMode=yes")
        .arg("-o").arg("StrictHostKeyChecking=no");
    if key.exists() {
        scp.arg("-i").arg(&key);
    }
    scp.arg(format!("{}@{}:/tmp/plex-index.db", cfg.nas_user, cfg.nas_host))
        .arg(local_db);

    let out = scp.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "scp that bai: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let rows = parse_db(local_db)?;
    // Dem TITLE rieng biet chu khong dem dong: Plex cap ca tmdb + tvdb + imdb
    // cho mot phim nen so dong gap ba so phim that.
    let n_titles = rows
        .iter()
        .map(|r| r.6.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    job_store
        .replace_library_source("plex", &rows)
        .map_err(|e| e.to_string())?;
    let _ = std::fs::write(stamp_file, &stamp);
    Ok(Some(n_titles))
}

fn parse_db(
    db: &Path,
) -> Result<Vec<(String, String, String, String, String, String, String)>, String> {
    let uri = format!("file:{}?mode=ro&immutable=1", db.to_string_lossy());
    let conn = Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| e.to_string())?;

    // Plex luu id ngoai trong bang tags (tag_type 314) duoi dang "tmdb://12345",
    // noi voi metadata_items qua bang taggings.
    let mut stmt = conn
        .prepare(
            "SELECT mi.title, mi.metadata_type, t.tag, mi.id
               FROM metadata_items mi
               JOIN taggings tg ON tg.metadata_item_id = mi.id
               JOIN tags t ON t.id = tg.tag_id
              WHERE mi.metadata_type IN (1, 2) AND t.tag_type = 314",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    let it = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get::<_, i64>(1)?,
                r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                r.get::<_, i64>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for (title, mtype_num, tag, item_id) in it.flatten() {
        let (scheme, value) = match tag.split_once("://") {
            Some((s, v)) => (s, v.trim()),
            None => continue,
        };
        let media_key = match scheme {
            "tmdb" => format!("tmdb-{}", value),
            "tvdb" => format!("tvdb-{}", value),
            "imdb" => format!("imdb-{}", value),
            _ => continue,
        };
        if value.is_empty() {
            continue;
        }
        let media_type = if mtype_num == 1 { "movie" } else { "series" };
        rows.push((
            media_key,
            String::new(), // Plex khong co collection nao -> khong cho franchise
            title.clone(),
            title,
            media_type.to_string(),
            String::new(),
            item_id.to_string(),
        ));
    }

    Ok(rows)
}

fn ssh_capture(settings: &Arc<dyn ISettingsService>, remote_cmd: &str) -> Result<String, String> {
    let cfg = settings.load();
    let key = expand_tilde(&cfg.nas_ssh_key);

    let mut cmd = Command::new("ssh");
    cmd.arg("-p").arg(cfg.nas_port.to_string())
        .arg("-o").arg("BatchMode=yes")
        .arg("-o").arg("ConnectTimeout=6")
        .arg("-o").arg("StrictHostKeyChecking=no");
    if key.exists() {
        cmd.arg("-i").arg(&key);
    }
    cmd.arg(format!("{}@{}", cfg.nas_user, cfg.nas_host))
        .arg(remote_cmd);

    let out = cmd.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "ssh loi: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn expand_tilde(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(p)
}
