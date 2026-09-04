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

        crate::services::worker_status::register("indexer/jellyfin");
        loop {
            if !crate::services::worker_status::is_enabled("indexer/jellyfin") {
                crate::services::worker_status::sleep_interruptible(
                    "indexer/jellyfin",
                    Duration::from_secs(5),
                );
                continue;
            }
            crate::services::worker_status::begin("indexer/jellyfin");
            match sync_once(&settings, &local_db, &stamp_file, &job_store) {
                Ok(Some(n)) => {
                    log::info!("[indexer/jellyfin] cap nhat: {} muc", n);
                    // Gom lai ngay sau khi nguon nay doi, thay vi hen gio.
                    if let Err(e) = crate::services::library_aggregator::refresh_and_store(&job_store) {
                        log::warn!("[aggregator] khong ghi duoc bang da gom: {}", e);
                    }

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
            crate::services::worker_status::sleep_interruptible(
                "indexer/jellyfin",
                Duration::from_secs(300),
            );
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
    let coll_cache = local_db
        .parent()
        .unwrap_or(Path::new("."))
        .join("tmdb_collections.json");
    let rows = parse_db(local_db, &cfg.tmdb_api_key, &coll_cache)?;
    // Dem TITLE rieng biet chu khong dem dong: mot phim mang nhieu id
    // (tmdb + tvdb + imdb) se sinh nhieu dong, bao so dong la gap doi su that.
    let n_titles = rows
        .iter()
        .map(|r| r.6.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();
    job_store
        .replace_library_source("jellyfin", &rows)
        .map_err(|e| e.to_string())?;

    let _ = std::fs::write(stamp_file, &stamp);
    Ok(Some(n_titles))
}

/// Rut tung phim/series kem Tmdb/Tvdb id, quy ve dung dinh dang media_key
/// ma cac indexer khac dang dung ("tvdb-123" / "tmdb-456").
fn parse_db(
    db: &Path,
    tmdb_key: &str,
    coll_cache: &Path,
) -> Result<Vec<(String, String, String, String, String, String, String)>, String> {
    let uri = format!("file:{}?mode=ro&immutable=1", db.to_string_lossy());
    let conn = Connection::open_with_flags(
        &uri,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .map_err(|e| e.to_string())?;

    // Truoc het dung ban do media_key -> ten BoxSet.
    // Jellyfin luu thanh vien BoxSet trong cot Data (JSON, khoa LinkedChildren),
    // khong quan he hoa, nen phai doc JSON roi noi lai qua ItemId.
    let mut franchise_map = boxset_franchises(&conn);
    let n_boxset = franchise_map.len();

    // Nguon thu hai: TMDb Collection. Jellyfin luu san collection id cho tung
    // muc, chi thieu ten -- lay ten qua TMDb API roi cache lai. BoxSet duoc
    // uu tien vi do chinh nguoi dung sap, TMDb chi lap cho phan con trong.
    for (k, v) in tmdb_collection_franchises(&conn, tmdb_key, coll_cache) {
        franchise_map.entry(k).or_insert(v);
    }

    // Lop thu ba: hoi thang TMDb tung phim con trong. Jellyfin bo sot khong it
    // (vd Ant-Man khong duoc gan TmdbCollection du TMDb co collection "Nguoi Kien").
    let movie_cache = coll_cache
        .parent()
        .unwrap_or(Path::new("."))
        .join("tmdb_movie_collections.json");
    for (k, v) in tmdb_movie_collections(&conn, tmdb_key, &movie_cache, &franchise_map) {
        franchise_map.entry(k).or_insert(v);
    }
    log::info!(
        "[indexer/jellyfin] franchise: {} tu BoxSet, {} tong (them TMDb Collection)",
        n_boxset,
        franchise_map.len()
    );

    let mut stmt = conn
        .prepare(
            "SELECT b.Name, b.Path, b.Type, p.ProviderId, p.ProviderValue, b.Id
               FROM BaseItems b
               JOIN BaseItemProviders p ON p.ItemId = b.Id
              WHERE b.Type IN (
                    'MediaBrowser.Controller.Entities.Movies.Movie',
                    'MediaBrowser.Controller.Entities.TV.Series')
                AND p.ProviderId IN ('Tmdb','Tvdb','Imdb')",
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
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    for row in it.flatten() {
        let (name, path, itype, provider, value, item_id) = row;
        if value.trim().is_empty() {
            continue;
        }
        let media_key = match provider.as_str() {
            "Tvdb" => format!("tvdb-{}", value.trim()),
            "Tmdb" => format!("tmdb-{}", value.trim()),
            "Imdb" => format!("imdb-{}", value.trim()),
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
            item_id,
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

/// Ban do media_key -> ten TMDb Collection.
/// Jellyfin luu collection id trong BaseItemProviders nhung khong luu ten,
/// nen phai hoi TMDb mot lan roi cache xuong dia -- 71 collection thi chi
/// ton 71 loi goi cho lan dau, cac lan sau doc cache.
fn tmdb_collection_franchises(
    conn: &Connection,
    api_key: &str,
    cache_path: &Path,
) -> std::collections::HashMap<String, String> {
    use std::collections::{HashMap, HashSet};
    let mut out: HashMap<String, String> = HashMap::new();
    if api_key.trim().is_empty() {
        return out;
    }

    // 1. ItemId -> collection id, va ItemId -> media_key
    let mut coll_of: HashMap<String, String> = HashMap::new();
    let mut keys_of: HashMap<String, Vec<String>> = HashMap::new();

    if let Ok(mut stmt) = conn.prepare(
        "SELECT ItemId, ProviderId, ProviderValue FROM BaseItemProviders
          WHERE ProviderId IN ('TmdbCollection','Tmdb','Tvdb')",
    ) {
        if let Ok(it) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        }) {
            for (item_id, provider, value) in it.flatten() {
                let v = value.trim().to_string();
                if v.is_empty() {
                    continue;
                }
                match provider.as_str() {
                    "TmdbCollection" => {
                        coll_of.insert(item_id, v);
                    }
                    "Tvdb" => keys_of.entry(item_id).or_default().push(format!("tvdb-{}", v)),
                    "Tmdb" => keys_of.entry(item_id).or_default().push(format!("tmdb-{}", v)),
                    _ => {}
                }
            }
        }
    }
    if coll_of.is_empty() {
        return out;
    }

    // 2. Ten collection: doc cache truoc, chi hoi TMDb phan con thieu
    let mut names: HashMap<String, String> = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let wanted: HashSet<String> = coll_of.values().cloned().collect();
    let missing: Vec<String> = wanted
        .iter()
        .filter(|id| !names.contains_key(*id))
        .cloned()
        .collect();

    if !missing.is_empty() {
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            let client = reqwest::Client::new();
            rt.block_on(async {
                for id in &missing {
                    let url = format!(
                        "https://api.themoviedb.org/3/collection/{}?api_key={}&language=vi-VN",
                        id, api_key
                    );
                    match client.get(&url).send().await {
                        Ok(resp) => {
                            if let Ok(v) = resp.json::<serde_json::Value>().await {
                                if let Some(n) = v.get("name").and_then(|n| n.as_str()) {
                                    if !n.trim().is_empty() {
                                        names.insert(id.clone(), n.trim().to_string());
                                    }
                                }
                            }
                        }
                        Err(e) => log::warn!("[indexer/jellyfin] TMDb collection {}: {}", id, e),
                    }
                }
            });
            if let Ok(txt) = serde_json::to_string_pretty(&names) {
                let _ = std::fs::write(cache_path, txt);
            }
        }
    }

    // 3. Rap lai: media_key -> ten collection
    for (item_id, coll_id) in &coll_of {
        if let Some(name) = names.get(coll_id) {
            if let Some(keys) = keys_of.get(item_id) {
                for k in keys {
                    out.insert(k.clone(), name.clone());
                }
            }
        }
    }
    out
}

/// Hoi TMDb `belongs_to_collection` cho tung phim chua co franchise.
/// Cache ca ket qua rong (phim khong thuoc collection nao) de khoi hoi lai.
fn tmdb_movie_collections(
    conn: &Connection,
    api_key: &str,
    cache_path: &Path,
    already: &std::collections::HashMap<String, String>,
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut out: HashMap<String, String> = HashMap::new();
    if api_key.trim().is_empty() {
        return out;
    }

    // Chi lam voi PHIM: TMDb khong co khai niem collection cho TV series.
    let mut movie_ids: Vec<String> = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT DISTINCT p.ProviderValue
           FROM BaseItems b JOIN BaseItemProviders p ON p.ItemId = b.Id
          WHERE b.Type = 'MediaBrowser.Controller.Entities.Movies.Movie'
            AND p.ProviderId = 'Tmdb'",
    ) {
        if let Ok(it) = stmt.query_map([], |r| r.get::<_, String>(0)) {
            for v in it.flatten() {
                let v = v.trim().to_string();
                if !v.is_empty() && !already.contains_key(&format!("tmdb-{}", v)) {
                    movie_ids.push(v);
                }
            }
        }
    }
    if movie_ids.is_empty() {
        return out;
    }

    // Cache: id -> ten collection ("" nghia la da hoi va phim khong co collection)
    let mut cache: HashMap<String, String> = std::fs::read_to_string(cache_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();

    let missing: Vec<String> = movie_ids
        .iter()
        .filter(|id| !cache.contains_key(*id))
        .cloned()
        .collect();

    if !missing.is_empty() {
        log::info!("[indexer/jellyfin] hoi TMDb collection cho {} phim", missing.len());
        if let Ok(rt) = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            let client = reqwest::Client::new();
            rt.block_on(async {
                for id in &missing {
                    let url = format!(
                        "https://api.themoviedb.org/3/movie/{}?api_key={}&language=vi-VN",
                        id, api_key
                    );
                    match client.get(&url).send().await {
                        Ok(resp) => {
                            if let Ok(v) = resp.json::<serde_json::Value>().await {
                                let name = v
                                    .get("belongs_to_collection")
                                    .and_then(|c| c.get("name"))
                                    .and_then(|n| n.as_str())
                                    .unwrap_or("")
                                    .trim()
                                    .to_string();
                                cache.insert(id.clone(), name);
                            }
                        }
                        Err(_) => {}
                    }
                }
            });
            if let Ok(txt) = serde_json::to_string_pretty(&cache) {
                let _ = std::fs::write(cache_path, txt);
            }
        }
    }

    for (id, name) in &cache {
        if !name.is_empty() {
            out.insert(format!("tmdb-{}", id), name.clone());
        }
    }
    out
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
