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

        // Requeue stale jobs
        let _ = conn.execute(
            "UPDATE jobs SET status='queued', phase='pending', message='Khôi phục sau khi server khởi động lại', updated_at=? WHERE status='running'",
            params![now_secs()],
        );

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
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
        let mut stmt = conn.prepare("SELECT id, torrent_id, name, status, phase, targets, done_targets, progress, speed_bps, bytes_done, bytes_total, message, staging_path, created_at, updated_at, finished_at FROM jobs WHERE status IN ('queued', 'running') ORDER BY created_at ASC").unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok(SyncJob {
                    id: row.get(0)?,
                    torrent_id: row.get(1)?,
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
        let mut stmt = conn.prepare("SELECT id, torrent_id, name, status, phase, targets, done_targets, progress, speed_bps, bytes_done, bytes_total, message, staging_path, created_at, updated_at, finished_at FROM jobs ORDER BY COALESCE(finished_at, updated_at) DESC LIMIT ?").unwrap();
        let rows = stmt
            .query_map(params![limit as i64], |row| {
                Ok(SyncJob {
                    id: row.get(0)?,
                    torrent_id: row.get(1)?,
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
