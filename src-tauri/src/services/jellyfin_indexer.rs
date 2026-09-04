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
            crate::services::worker_status::begin("indexer/jellyfin");
            match sync_once(&settings, &local_db, &stamp_file, &job_store) {
                Ok(Some(n)) => {
                    log::info!("[indexer/jellyfin] cap nhat: {} muc", n);
                    crate::services::worker_status::ok(
                        "indexer/jellyfin",
                        n as i64,
                        "doc jellyfin.db tu NAS",
                    );
                }
                Ok(None) => {
                    crate::services::worker_status::ok(
                        "indexer/jellyfin",
                        -1,
                        "DB khong doi, bo qua",
                    );
                }
                Err(e) => {
                    log::warn!("[indexer/jellyfin] {}", e);
                    crate::services::worker_status::err("indexer/jellyfin", &e);
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

    // Truoc het dung ban do media_key -> ten BoxSet.
    // Jellyfin luu thanh vien BoxSet trong cot Data (JSON, khoa LinkedChildren),
    // khong quan he hoa, nen phai doc JSON roi noi lai qua ItemId.
    let franchise_map = boxset_franchises(&conn);
    log::info!(
        "[indexer/jellyfin] {} muc co franchise tu BoxSet",
        franchise_map.len()
    );

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

        let franchise = franchise_map.get(&media_key).cloned().unwrap_or_default();
        rows.push((
            media_key,
            franchise,
            name,
            folder,
            media_type.to_string(),
            path,
        ));
    }

    Ok(rows)
}

/// Jellyfin luu Id o hai dang khac nhau: BaseItems.Id la GUID co gach va chu
/// hoa, con LinkedChildren trong JSON lai la hex thuong khong gach. Phai quy ve
/// mot dang thi moi noi duoc hai ben.
fn norm_guid(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Doc cac BoxSet (collection) cua Jellyfin va tra ve ban do media_key -> ten BoxSet.
/// Day la nguon franchise cho nhung title chi co tren NAS, khong co ban local
/// de muon franchise qua media_key.
fn boxset_franchises(conn: &Connection) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut map: HashMap<String, String> = HashMap::new();

    // 1. ItemId -> media_key (chi lay muc co Tmdb/Tvdb)
    let mut key_of: HashMap<String, String> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT ItemId, ProviderId, ProviderValue FROM BaseItemProviders
          WHERE ProviderId IN ('Tmdb','Tvdb')",
    ) {
        if let Ok(it) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        }) {
            for (item_id, provider, value) in it.flatten() {
                let v = value.trim();
                if v.is_empty() {
                    continue;
                }
                let k = match provider.as_str() {
                    "Tvdb" => format!("tvdb-{}", v),
                    "Tmdb" => format!("tmdb-{}", v),
                    _ => continue,
                };
                key_of.insert(format!("{}|{}", norm_guid(&item_id), provider), k);
            }
        }
    }

    // 2. Doc tung BoxSet, boc LinkedChildren trong JSON
    let mut stmt = match conn.prepare(
        "SELECT Name, Data FROM BaseItems
          WHERE Type = 'MediaBrowser.Controller.Entities.Movies.BoxSet'
            AND Data IS NOT NULL",
    ) {
        Ok(s) => s,
        Err(_) => return map,
    };

    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?.unwrap_or_default(),
            r.get::<_, Option<String>>(1)?.unwrap_or_default(),
        ))
    });

    if let Ok(it) = rows {
        for (name, data) in it.flatten() {
            if name.is_empty() || data.is_empty() {
                continue;
            }
            let parsed: serde_json::Value = match serde_json::from_str(&data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let children = match parsed.get("LinkedChildren").and_then(|c| c.as_array()) {
                Some(c) => c,
                None => continue,
            };
            for child in children {
                let item_id = match child.get("ItemId").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => continue,
                };
                for provider in ["Tvdb", "Tmdb"] {
                    if let Some(k) = key_of.get(&format!("{}|{}", norm_guid(item_id), provider)) {
                        map.insert(k.clone(), name.clone());
                    }
                }
            }
        }
    }

    map
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
