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
    updated_at  REAL    NOT NULL,
    -- Khoa theo media_key chu KHONG theo folder: mot title co ca Tmdb lan Tvdb
    -- se sinh 2 dong khac media_key, va ta muon giu ca hai de tra bang ID nao
    -- cung trung. Khoa theo folder se de bep mat mot trong hai.
    PRIMARY KEY (source, media_key)
);
CREATE INDEX IF NOT EXISTS idx_lib_key ON library_index(media_key);

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
        rows: &[(String, String, String, String, String, String)],
    ) -> Result<usize, String> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute("DELETE FROM library_index WHERE source=?", params![source])
            .map_err(|e| e.to_string())?;

        let now = now_secs();
        let mut n = 0usize;
        for (media_key, franchise, title, folder, media_type, path) in rows {
            let r = tx.execute(
                "INSERT OR REPLACE INTO library_index
                 (source, media_key, franchise, title, folder, media_type, path, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                params![source, media_key, franchise, title, folder, media_type, path, now],
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
    pub fn load_library_index(&self) -> Vec<(String, String, String, String, String, String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match conn.prepare(
            "SELECT source, media_key, franchise, title, folder, media_type, path FROM library_index",
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
