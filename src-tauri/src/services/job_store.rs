use crate::domain::models::job::{EnqueueResult, JobCounts, JobPhase, JobStatus, SyncJob};
use rusqlite::{params, Connection};
use std::path::PathBuf;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct JobStore {
    conn: Arc<Mutex<Connection>>,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS jobs (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    torrent_id    TEXT    NOT NULL,
    name          TEXT    NOT NULL DEFAULT '',
    status        TEXT    NOT NULL DEFAULT 'queued',
    phase         TEXT    NOT NULL DEFAULT 'pending',
    targets       TEXT    NOT NULL DEFAULT '[]',
    done_targets  TEXT    NOT NULL DEFAULT '[]',
    progress      REAL    NOT NULL DEFAULT 0.0,
    bytes_total   INTEGER NOT NULL DEFAULT 0,
    bytes_done    INTEGER NOT NULL DEFAULT 0,
    speed_bps     REAL    NOT NULL DEFAULT 0.0,
    staging_path  TEXT    NOT NULL DEFAULT '',
    franchise     TEXT    NOT NULL DEFAULT '',
    source_uri    TEXT    NOT NULL DEFAULT '',
    gid           TEXT    NOT NULL DEFAULT '',
    message       TEXT    NOT NULL DEFAULT '',
    error         TEXT    NOT NULL DEFAULT '',
    attempts      INTEGER NOT NULL DEFAULT 0,
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    created_at    REAL    NOT NULL,
    updated_at    REAL    NOT NULL,
    finished_at   REAL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_jobs_one_active_per_torrent
    ON jobs(torrent_id) WHERE status IN ('queued', 'running');
CREATE INDEX IF NOT EXISTS idx_jobs_status  ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_updated ON jobs(updated_at DESC);

CREATE TABLE IF NOT EXISTS library_index (
    source      TEXT    NOT NULL,
    media_key   TEXT    NOT NULL,
    franchise   TEXT    NOT NULL DEFAULT '',
    title       TEXT    NOT NULL DEFAULT '',
    folder      TEXT    NOT NULL,
    media_type  TEXT    NOT NULL DEFAULT 'series',
    path        TEXT    NOT NULL DEFAULT '',
    -- Dinh danh title theo tung nguon. Mot title co the mang nhieu media_key
    -- (tmdb + tvdb + imdb); cac dong cung (source, item_uid) la CUNG mot phim,
    -- nho vay aggregator hop nhat duoc thay vi dem thanh nhieu muc.
    item_uid    TEXT    NOT NULL DEFAULT '',
    updated_at  REAL    NOT NULL,
    -- Khoa theo media_key chu KHONG theo folder: mot title co ca Tmdb lan Tvdb
    -- se sinh 2 dong khac media_key, va ta muon giu ca hai de tra bang ID nao
    -- cung trung. Khoa theo folder se de bep mat mot trong hai.
    PRIMARY KEY (source, media_key)
);
CREATE INDEX IF NOT EXISTS idx_lib_key ON library_index(media_key);

-- Ket qua da gom: moi dong la MOT title sau khi hop nhat cac ID trung nhau.
CREATE TABLE IF NOT EXISTS library_unified (
    root_key    TEXT    NOT NULL PRIMARY KEY,
    title       TEXT    NOT NULL DEFAULT '',
    franchise   TEXT    NOT NULL DEFAULT '',
    media_type  TEXT    NOT NULL DEFAULT 'series',
    in_draft    INTEGER NOT NULL DEFAULT 0,
    in_nas      INTEGER NOT NULL DEFAULT 0,
    in_drive    INTEGER NOT NULL DEFAULT 0,
    seen_by     TEXT    NOT NULL DEFAULT '[]',
    folders     TEXT    NOT NULL DEFAULT '{}',
    paths       TEXT    NOT NULL DEFAULT '{}',
    updated_at  REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_unified_franchise ON library_unified(franchise);

-- Tra bang BAT KY id nao (tmdb/tvdb/imdb) cung ra dung title, nho bang nay
-- anh xa moi khoa thanh phan ve khoa goc cua nhom sau union-find.
CREATE TABLE IF NOT EXISTS library_key_map (
    media_key   TEXT    NOT NULL PRIMARY KEY,
    root_key    TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_keymap_root ON library_key_map(root_key);

-- TMDb/TVDB khong co khai niem "collection" cho series (chi co cho phim),
-- nen nhieu series cung mot vu tru (vd Bay Vien Ngoc Rong / GT / Kai, hay
-- moi mua Super Sentai) khong co API nao de tra franchise. Bang nay luu ket
-- qua agy daemon suy luan tu ten phim. franchise='' nghia la da hoi va xac
-- nhan la phim/series doc lap, khong phai chua hoi -- tranh hoi lai moi lan.
CREATE TABLE IF NOT EXISTS franchise_ai_cache (
    root_key    TEXT    NOT NULL PRIMARY KEY,
    franchise   TEXT    NOT NULL DEFAULT '',
    checked_at  REAL    NOT NULL
);

CREATE TABLE IF NOT EXISTS collections_cache (
    id          INTEGER PRIMARY KEY CHECK (id = 1),
    payload     TEXT    NOT NULL,
    updated_at  REAL    NOT NULL
);
"#;

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

impl JobStore {
    pub fn new(db_path: Option<PathBuf>) -> Result<Self, String> {
        let path = db_path.unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".media-hub")
                .join("media_hub.db")
        });

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let conn = Connection::open(&path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| e.to_string())?;
        conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;

        // DB cu da co bang jobs roi thi CREATE TABLE IF NOT EXISTS khong them cot moi,
        // nen phai ALTER rieng. Cot da ton tai se bao loi -> bo qua co y.
        // Bang library_index ban dau dat PK (source, folder) lam mat dong khi
        // mot title co nhieu provider id. Bo bang cu di, indexer dung lai ngay.
        let needs_rebuild: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_index_list('library_index')
                  WHERE origin='pk'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|_| {
                conn.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('library_index') WHERE pk > 0 AND name='folder'",
                    [],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0)
                    > 0
            })
            .unwrap_or(false);
        if needs_rebuild {
            let _ = conn.execute_batch("DROP TABLE IF EXISTS library_index;");
            let _ = conn.execute_batch(SCHEMA);
        }

        for col in [
            "ALTER TABLE jobs ADD COLUMN franchise TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE jobs ADD COLUMN source_uri TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE jobs ADD COLUMN gid TEXT NOT NULL DEFAULT ''",
            "ALTER TABLE library_index ADD COLUMN item_uid TEXT NOT NULL DEFAULT ''",
        ] {
            let _ = conn.execute(col, []);
        }

        // Requeue stale jobs
        let _ = conn.execute(
            "UPDATE jobs SET status='queued', phase='pending', message='Khôi phục sau khi server khởi động lại', updated_at=? WHERE status='running'",
            params![now_secs()],
        );

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Them job tai moi, co gan franchise dich va nguon (magnet hoac https).
    pub fn enqueue_download(
        &self,
        name: &str,
        franchise: &str,
        source_uri: &str,
        staging_path: &str,
        targets: Vec<String>,
    ) -> i64 {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        let targets_json = serde_json::to_string(&targets).unwrap_or_else(|_| "[]".to_string());
        let torrent_id = format!("dl-{}", now as i64);

        let r = conn.execute(
            "INSERT INTO jobs (torrent_id, name, status, phase, targets, franchise, source_uri, staging_path, created_at, updated_at)
             VALUES (?, ?, 'queued', 'pending', ?, ?, ?, ?, ?, ?)",
            params![torrent_id, name, targets_json, franchise, source_uri, staging_path, now, now],
        );
        match r {
            Ok(_) => conn.last_insert_rowid(),
            Err(_) => -1,
        }
    }

    pub fn mark_running(&self, job_id: i64, gid: &str, message: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE jobs SET status='running', phase='download', gid=?, message=?, updated_at=? WHERE id=?",
            params![gid, message, now_secs(), job_id],
        );
    }

    pub fn update_progress(
        &self,
        job_id: i64,
        bytes_done: u64,
        bytes_total: u64,
        speed_bps: u64,
    ) {
        let progress = if bytes_total > 0 {
            (bytes_done as f64 / bytes_total as f64) * 100.0
        } else {
            0.0
        };
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "UPDATE jobs SET bytes_done=?, bytes_total=?, speed_bps=?, progress=?, updated_at=? WHERE id=?",
            params![
                bytes_done as i64,
                bytes_total as i64,
                speed_bps as f64,
                progress,
                now_secs(),
                job_id
            ],
        );
    }

    pub fn mark_done(&self, job_id: i64, message: &str) {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        let _ = conn.execute(
            "UPDATE jobs SET status='done', phase='done', progress=100.0, message=?, updated_at=?, finished_at=? WHERE id=?",
            params![message, now, now, job_id],
        );
    }

    pub fn mark_failed(&self, job_id: i64, error: &str) {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        let _ = conn.execute(
            "UPDATE jobs SET status='failed', message=?, error=?, updated_at=?, finished_at=? WHERE id=?",
            params![error, error, now, now, job_id],
        );
    }

    pub fn get_gid(&self, job_id: i64) -> Option<String> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT gid FROM jobs WHERE id=?", params![job_id], |r| {
            r.get::<_, String>(0)
        })
        .ok()
        .filter(|g| !g.is_empty())
    }

    pub fn is_cancel_requested(&self, job_id: i64) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT cancel_requested FROM jobs WHERE id=?",
            params![job_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|v| v == 1)
        .unwrap_or(false)
    }

    pub fn mark_cancelled(&self, job_id: i64) {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        let _ = conn.execute(
            "UPDATE jobs SET status='cancelled', message='Đã huỷ theo yêu cầu', updated_at=?, finished_at=? WHERE id=?",
            params![now, now, job_id],
        );
    }

    /// Thay toan bo index cua mot nguon trong 1 transaction.
    /// Xoa sach roi ghi lai de muc da bien mat ben nguon cung bien mat trong DB.
    pub fn replace_library_source(
        &self,
        source: &str,
        rows: &[(String, String, String, String, String, String, String)],
    ) -> Result<usize, String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM library_index WHERE source=?", params![source])
            .map_err(|e| e.to_string())?;

        let now = now_secs();
        let mut n = 0usize;
        for (media_key, franchise, title, folder, media_type, path, item_uid) in rows {
            let r = tx.execute(
                "INSERT OR REPLACE INTO library_index
                 (source, media_key, franchise, title, folder, media_type, path, item_uid, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![source, media_key, franchise, title, folder, media_type, path, item_uid, now],
            );
            if r.is_ok() {
                n += 1;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(n)
    }

    /// Doc toan bo index cua ca 3 nguon.
    /// Tra ve: (source, media_key, franchise, title, folder, media_type, path)
    pub fn load_library_index(
        &self,
    ) -> Vec<(String, String, String, String, String, String, String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT source, media_key, franchise, title, folder, media_type, path, item_uid
               FROM library_index",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
            ))
        });
        match rows {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn library_counts(&self) -> Vec<(String, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn
            .prepare("SELECT source, COUNT(*) FROM library_index GROUP BY source ORDER BY source")
        {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)));
        match rows {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Ghi de toan bo ket qua da gom trong 1 transaction.
    pub fn save_unified(
        &self,
        items: &[(String, String, String, String, bool, bool, bool, String, String, String)],
        key_map: &[(String, String)],
    ) -> Result<usize, String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM library_unified", []).map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM library_key_map", []).map_err(|e| e.to_string())?;

        let now = now_secs();
        let mut n = 0usize;
        for (root, title, franchise, mtype, draft, nas, drive, seen, folders, paths) in items {
            let r = tx.execute(
                "INSERT OR REPLACE INTO library_unified
                 (root_key, title, franchise, media_type, in_draft, in_nas, in_drive,
                  seen_by, folders, paths, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    root, title, franchise, mtype,
                    *draft as i64, *nas as i64, *drive as i64,
                    seen, folders, paths, now
                ],
            );
            if r.is_ok() {
                n += 1;
            }
        }
        for (key, root) in key_map {
            let _ = tx.execute(
                "INSERT OR REPLACE INTO library_key_map (media_key, root_key) VALUES (?, ?)",
                params![key, root],
            );
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(n)
    }

    /// Tra mot media id bat ky -> title da gom.
    /// Tra ve: (root_key, title, franchise, media_type, draft, nas, drive, seen_by, folders, paths)
    pub fn lookup_media(
        &self,
        media_key: &str,
    ) -> Option<(String, String, String, String, bool, bool, bool, String, String, String)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT u.root_key, u.title, u.franchise, u.media_type,
                    u.in_draft, u.in_nas, u.in_drive, u.seen_by, u.folders, u.paths
               FROM library_key_map m
               JOIN library_unified u ON u.root_key = m.root_key
              WHERE m.media_key = ?",
            params![media_key],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)? == 1,
                    r.get::<_, i64>(5)? == 1,
                    r.get::<_, i64>(6)? == 1,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                ))
            },
        )
        .ok()
    }

    pub fn unified_count(&self) -> i64 {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM library_unified", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Danh sach title dang la franchise don le (chua tung duoc AI hoi qua).
    /// franchise = title chinh la quy uoc "chua gom nhom" cua aggregator.
    pub fn list_uncached_for_ai(&self, limit: usize) -> Vec<(String, String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT u.root_key, u.title, u.media_type
               FROM library_unified u
              WHERE u.franchise = u.title
                AND NOT EXISTS (
                    SELECT 1 FROM franchise_ai_cache c WHERE c.root_key = u.root_key
                )
              ORDER BY u.root_key
              LIMIT ?",
        ) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let rows = stmt.query_map(params![limit as i64], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        });
        match rows {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Ghi ket qua AI cho mot loat title trong 1 transaction.
    /// franchise rong = da hoi, xac nhan la doc lap (van cache de khoi hoi lai).
    pub fn save_ai_franchise_batch(&self, results: &[(String, String)]) {
        let mut conn = self.conn.lock().unwrap();
        let now = now_secs();
        let tx = match conn.transaction() {
            Ok(t) => t,
            Err(_) => return,
        };
        for (root_key, franchise) in results {
            let _ = tx.execute(
                "INSERT OR REPLACE INTO franchise_ai_cache (root_key, franchise, checked_at)
                 VALUES (?, ?, ?)",
                params![root_key, franchise, now],
            );
        }
        let _ = tx.commit();
    }

    /// Ban do root_key -> ten franchise, chi lay nhung dong AI THAT SU tim
    /// ra franchise (bo qua dong franchise rong = da xac nhan doc lap).
    pub fn load_ai_franchise_map(&self) -> std::collections::HashMap<String, String> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn
            .prepare("SELECT root_key, franchise FROM franchise_ai_cache WHERE franchise != ''")
        {
            Ok(s) => s,
            Err(_) => return std::collections::HashMap::new(),
        };
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)));
        match rows {
            Ok(it) => it.filter_map(|r| r.ok()).collect(),
            Err(_) => std::collections::HashMap::new(),
        }
    }

    pub fn ai_cache_count(&self) -> (i64, i64) {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM franchise_ai_cache", [], |r| r.get(0))
            .unwrap_or(0);
        let found: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM franchise_ai_cache WHERE franchise != ''",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (total, found)
    }

    pub fn save_collections_snapshot(&self, payload: &str) {
        let conn = self.conn.lock().unwrap();
        let _ = conn.execute(
            "INSERT INTO collections_cache (id, payload, updated_at) VALUES (1, ?, ?)
             ON CONFLICT(id) DO UPDATE SET payload=excluded.payload, updated_at=excluded.updated_at",
            params![payload, now_secs()],
        );
    }

    pub fn load_collections_snapshot(&self) -> Option<(String, f64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row(
            "SELECT payload, updated_at FROM collections_cache WHERE id=1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
        )
        .ok()
    }

    pub fn enqueue(&self, torrent_id: &str, targets: Vec<String>, name: &str) -> EnqueueResult {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        let targets_json = serde_json::to_string(&targets).unwrap_or_else(|_| "[]".to_string());

        // Check if an active job already exists
        let mut stmt = conn
            .prepare("SELECT id, targets, status FROM jobs WHERE torrent_id=? AND status IN ('queued', 'running') LIMIT 1")
            .unwrap();
        let mut rows = stmt.query(params![torrent_id]).unwrap();

        if let Ok(Some(row)) = rows.next() {
            let id: i64 = row.get(0).unwrap_or(0);
            let existing_targets_raw: String = row.get(1).unwrap_or_else(|_| "[]".to_string());
            let mut existing_targets: Vec<String> =
                serde_json::from_str(&existing_targets_raw).unwrap_or_default();

            for t in targets {
                if !existing_targets.contains(&t) {
                    existing_targets.push(t);
                }
            }
            let merged_json = serde_json::to_string(&existing_targets).unwrap_or_default();
            let _ = conn.execute(
                "UPDATE jobs SET targets=?, updated_at=? WHERE id=?",
                params![merged_json, now, id],
            );

            return EnqueueResult {
                job_id: id,
                torrent_id: torrent_id.to_string(),
                is_new_download: false,
                message: format!("Đã gộp đích đến mới vào tác vụ đang chạy #{}", id),
            };
        }

        let _ = conn.execute(
            "INSERT INTO jobs (torrent_id, name, status, phase, targets, created_at, updated_at) VALUES (?, ?, 'queued', 'pending', ?, ?, ?)",
            params![torrent_id, name, targets_json, now, now],
        );
        let id = conn.last_insert_rowid();

        EnqueueResult {
            job_id: id,
            torrent_id: torrent_id.to_string(),
            is_new_download: true,
            message: format!("Đã xếp hàng tải Torrent #{}", torrent_id),
        }
    }

    pub fn request_cancel(&self, job_id: i64) -> bool {
        let conn = self.conn.lock().unwrap();
        let now = now_secs();
        if let Ok(status) = conn.query_row(
            "SELECT status FROM jobs WHERE id=?",
            params![job_id],
            |r| r.get::<_, String>(0),
        ) {
            if status == "queued" {
                let _ = conn.execute(
                    "UPDATE jobs SET status='cancelled', message='Đã hủy khi còn trong hàng đợi', updated_at=?, finished_at=? WHERE id=?",
                    params![now, now, job_id],
                );
                return true;
            } else if status == "running" {
                let _ = conn.execute(
                    "UPDATE jobs SET cancel_requested=1, updated_at=? WHERE id=?",
                    params![now, job_id],
                );
                return true;
            }
        }
        false
    }

    pub fn list_active(&self) -> Vec<SyncJob> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, torrent_id, name, status, phase, targets, done_targets, progress, speed_bps, bytes_done, bytes_total, message, staging_path, created_at, updated_at, finished_at, franchise, source_uri FROM jobs WHERE status IN ('queued', 'running') ORDER BY created_at ASC").unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok(SyncJob {
                    id: row.get(0)?,
                    torrent_id: row.get(1)?,
                    franchise: row.get::<_, String>(16).unwrap_or_default(),
                    source_uri: row.get::<_, String>(17).unwrap_or_default(),
                    name: row.get(2)?,
                    status: match row.get::<_, String>(3)?.as_str() {
                        "running" => JobStatus::Running,
                        "done" => JobStatus::Done,
                        "failed" => JobStatus::Failed,
                        "cancelled" => JobStatus::Cancelled,
                        _ => JobStatus::Pending,
                    },
                    phase: match row.get::<_, String>(4)?.as_str() {
                        "link" => JobPhase::Link,
                        "download" => JobPhase::Download,
                        "upload" => JobPhase::Upload,
                        "done" => JobPhase::Done,
                        _ => JobPhase::Pending,
                    },
                    targets: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    done_targets: serde_json::from_str(&row.get::<_, String>(6)?)
                        .unwrap_or_default(),
                    progress: row.get(7)?,
                    speed_bps: row.get::<_, f64>(8).unwrap_or(0.0) as u64,
                    bytes_done: row.get::<_, i64>(9).unwrap_or(0) as u64,
                    bytes_total: row.get::<_, i64>(10).unwrap_or(0) as u64,
                    message: row.get(11)?,
                    staging_path: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    finished_at: row.get(15)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn list_recent(&self, limit: usize) -> Vec<SyncJob> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, torrent_id, name, status, phase, targets, done_targets, progress, speed_bps, bytes_done, bytes_total, message, staging_path, created_at, updated_at, finished_at, franchise, source_uri FROM jobs ORDER BY COALESCE(finished_at, updated_at) DESC LIMIT ?").unwrap();
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SyncJob {
                    id: row.get(0)?,
                    torrent_id: row.get(1)?,
                    franchise: row.get::<_, String>(16).unwrap_or_default(),
                    source_uri: row.get::<_, String>(17).unwrap_or_default(),
                    name: row.get(2)?,
                    status: match row.get::<_, String>(3)?.as_str() {
                        "running" => JobStatus::Running,
                        "done" => JobStatus::Done,
                        "failed" => JobStatus::Failed,
                        "cancelled" => JobStatus::Cancelled,
                        _ => JobStatus::Pending,
                    },
                    phase: match row.get::<_, String>(4)?.as_str() {
                        "link" => JobPhase::Link,
                        "download" => JobPhase::Download,
                        "upload" => JobPhase::Upload,
                        "done" => JobPhase::Done,
                        _ => JobPhase::Pending,
                    },
                    targets: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
                    done_targets: serde_json::from_str(&row.get::<_, String>(6)?)
                        .unwrap_or_default(),
                    progress: row.get(7)?,
                    speed_bps: row.get::<_, f64>(8).unwrap_or(0.0) as u64,
                    bytes_done: row.get::<_, i64>(9).unwrap_or(0) as u64,
                    bytes_total: row.get::<_, i64>(10).unwrap_or(0) as u64,
                    message: row.get(11)?,
                    staging_path: row.get(12)?,
                    created_at: row.get(13)?,
                    updated_at: row.get(14)?,
                    finished_at: row.get(15)?,
                })
            })
            .unwrap();

        rows.filter_map(|r| r.ok()).collect()
    }

    pub fn counts(&self) -> JobCounts {
        let conn = self.conn.lock().unwrap();
        let mut counts = JobCounts {
            active: 0,
            pending: 0,
            done: 0,
            failed: 0,
            total: 0,
        };
        if let Ok(mut stmt) = conn.prepare("SELECT status, COUNT(*) FROM jobs GROUP BY status") {
            if let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, u32>(1)?))) {
                for item in rows.flatten() {
                    match item.0.as_str() {
                        "running" => counts.active += item.1,
                        "queued" => counts.pending += item.1,
                        "done" => counts.done += item.1,
                        "failed" | "cancelled" => counts.failed += item.1,
                        _ => {}
                    }
                    counts.total += item.1;
                }
            }
        }
        counts
    }
}
