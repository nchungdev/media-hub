use crate::domain::traits::ISettingsService;
use crate::services::job_store::JobStore;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

/// Duong dan DB cua Jellyfin tren NAS.
const REMOTE_DB: &str = "/appdata/jellyfin/data/jellyfin.db";

/// Worker theo doi DB cua Jellyfin tren NAS.
///
/// Jellyfin da quet san toan bo thu vien NAS va luu ca Tmdb/Tvdb id cho tung
/// muc, nen dung no lam bang tra cuu "NAS da co phim nay chua" chinh xac hon
/// nhieu so voi doc ten thu muc -- ke ca khi ten thu muc khong kem {tvdb-...}.
///
/// DB nang ~110MB nen chi keo ve khi mtime/size doi, dung nhu yeu cau:
/// cache xuong local, co thay doi moi cap nhat lai.
pub fn start(home: PathBuf, settings: Arc<dyn ISettingsService>, job_store: Arc<JobStore>) {
    std::thread::spawn(move || {
        let cache_dir = home.join("_app").join("cache");
        let _ = std::fs::create_dir_all(&cache_dir);
        let local_db = cache_dir.join("jellyfin.db");
        let stamp_file = cache_dir.join("jellyfin.db.stamp");

        loop {
            match sync_once(&settings, &local_db, &stamp_file, &job_store) {
                Ok(Some(n)) => log::info!("[indexer/jellyfin] cap nhat: {} muc", n),
                Ok(None) => log::debug!("[indexer/jellyfin] DB khong doi, bo qua"),
                Err(e) => log::warn!("[indexer/jellyfin] {}", e),
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

    // 1. Hoi dau van tay cua DB tren NAS (mtime + size) truoc khi keo 110MB ve.
    let stat = ssh_capture(
        settings,
        &format!("stat -c '%Y %s' {} 2>/dev/null", REMOTE_DB),
    )?;
    let stamp = stat.trim().to_string();
    if stamp.is_empty() {
        return Err("khong doc duoc jellyfin.db tren NAS".into());
    }

    let old = std::fs::read_to_string(stamp_file).unwrap_or_default();
    if old.trim() == stamp && local_db.exists() {
        return Ok(None); // chua doi -> khoi keo ve
    }

    // 2. DB dang duoc Jellyfin mo, copy sang /tmp truoc roi moi keo ve
    //    de tranh doc trung luc dang ghi.
    ssh_capture(
        settings,
        &format!("sudo -n cp {} /tmp/jf-index.db 2>/dev/null; echo ok", REMOTE_DB),
    )?;

    let key = expand_tilde(&cfg.nas_ssh_key);
    let mut scp = Command::new("scp");
    scp.arg("-P").arg(cfg.nas_port.to_string())
        .arg("-o").arg("BatchMode=yes")
        .arg("-o").arg("StrictHostKeyChecking=no");
    if key.exists() {
        scp.arg("-i").arg(&key);
    }
    scp.arg(format!("{}@{}:/tmp/jf-index.db", cfg.nas_user, cfg.nas_host))
        .arg(local_db);

    let out = scp.output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "scp that bai: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    // 3. Doc ban cache local, khong dung toi DB that tren NAS nua.
    let rows = parse_db(local_db)?;
    let n = job_store
        .replace_library_source("jellyfin", &rows)
        .map_err(|e| e.to_string())?;

    let _ = std::fs::write(stamp_file, &stamp);
    Ok(Some(n))
}

/// Rut tung phim/series kem Tmdb/Tvdb id, quy ve dung dinh dang media_key
/// ma cac indexer khac dang dung ("tvdb-123" / "tmdb-456").
fn parse_db(db: &Path) -> Result<Vec<(String, String, String, String, String, String)>, String> {
    let uri = format!("file:{}?mode=ro&immutable=1", db.to_string_lossy());
    let conn = Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT b.Name, b.Path, b.Type, p.ProviderId, p.ProviderValue
               FROM BaseItems b
               JOIN BaseItemProviders p ON p.ItemId = b.Id
              WHERE b.Type IN (
                    'MediaBrowser.Controller.Entities.Movies.Movie',
                    'MediaBrowser.Controller.Entities.TV.Series')
                AND p.ProviderId IN ('Tmdb','Tvdb')",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    let it = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, Option<String>>(0)?.unwrap_or_default(),
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in it.flatten() {
        let (name, path, itype, provider, value) = row;
        if value.trim().is_empty() {
            continue;
        }
        let media_key = match provider.as_str() {
            "Tvdb" => format!("tvdb-{}", value.trim()),
            "Tmdb" => format!("tmdb-{}", value.trim()),
            _ => continue,
        };
        let media_type = if itype.ends_with("Movie") {
            "movie"
        } else {
            "series"
        };
        let folder = Path::new(&path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| name.clone());

        rows.push((
            media_key,
            String::new(), // Jellyfin phang -> franchise lay tu local qua media_key
            name,
            folder,
            media_type.to_string(),
            path,
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
