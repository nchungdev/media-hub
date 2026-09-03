#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Sync Job Store — SQLite-backed persistent state for the download/sync pipeline.

Replaces the previous whole-file JSON rewrite (media_sync_state.json), which was
non-atomic and could not be written safely from several worker threads at once.

Schema notes:
  * A partial UNIQUE index enforces "one active job per torrent" at the DB level,
    so de-duplication cannot be lost to a race between two HTTP handler threads.
  * `targets` / `done_targets` are JSON arrays of destination ids ("drive", "nas").
"""

import os
import json
import time
import sqlite3
import threading
from pathlib import Path

# Path comes from the project root (.mediahub), shared with LibraryStore.
LEGACY_JSON_PATH = Path.home() / ".gemini" / "config" / "media_sync_state.json"


def default_db_path():
    from core.settings import resolve_dirs, load_unified_settings
    return Path(resolve_dirs(load_unified_settings(), create=True)["db_path"])

ACTIVE_STATUSES = ("queued", "running")

SCHEMA = """
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
"""


def _loads(raw, fallback):
    try:
        val = json.loads(raw)
        return val if isinstance(val, list) else fallback
    except Exception:
        return fallback


class JobStore:
    def __init__(self, db_path=None):
        self.db_path = Path(db_path) if db_path else default_db_path()
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.RLock()
        self._conn = sqlite3.connect(str(self.db_path), check_same_thread=False, isolation_level=None)
        self._conn.row_factory = sqlite3.Row
        with self._lock:
            self._conn.execute("PRAGMA journal_mode=WAL")
            self._conn.execute("PRAGMA synchronous=NORMAL")
            self._conn.execute("PRAGMA busy_timeout=5000")
            self._conn.executescript(SCHEMA)
        self._migrate_legacy_json()
        self.requeue_stale_running()

    # ---------- serialisation helpers ----------

    @staticmethod
    def _row_to_dict(row):
        if row is None:
            return None
        d = dict(row)
        d["targets"] = _loads(d.get("targets") or "[]", [])
        d["done_targets"] = _loads(d.get("done_targets") or "[]", [])
        d["cancel_requested"] = bool(d.get("cancel_requested"))
        # Field kept for backwards compatibility with the old UI contract.
        d["torrent_id"] = str(d.get("torrent_id"))
        return d

    # ---------- lifecycle ----------

    def _migrate_legacy_json(self):
        """One-shot import of the old media_sync_state.json, then retire the file."""
        if not LEGACY_JSON_PATH.is_file():
            return
        try:
            with open(LEGACY_JSON_PATH, "r", encoding="utf-8") as f:
                legacy = json.load(f)
        except Exception:
            return
        now = time.time()
        for tid, job in (legacy or {}).items():
            targets = job.get("targets") or []
            if isinstance(targets, str):
                targets = [targets]
            try:
                self.enqueue(tid, list(targets), job.get("name", ""))
            except Exception:
                pass
        try:
            LEGACY_JSON_PATH.rename(LEGACY_JSON_PATH.with_suffix(".json.migrated"))
        except Exception:
            pass
        print(f"[JobStore] Đã nhập {len(legacy or {})} job từ media_sync_state.json vào SQLite.")

    def requeue_stale_running(self):
        """After a crash/restart, jobs stuck in 'running' have no worker. Requeue them."""
        with self._lock:
            cur = self._conn.execute(
                "UPDATE jobs SET status='queued', phase='pending', message='Khôi phục sau khi server khởi động lại',"
                " updated_at=? WHERE status='running'",
                (time.time(),),
            )
            if cur.rowcount:
                print(f"[JobStore] Đưa {cur.rowcount} job dở dang về hàng đợi sau khi khởi động lại.")
            return cur.rowcount

    # ---------- writes ----------

    def enqueue(self, torrent_id, targets, name=""):
        """Register a sync request. If the torrent already has an active job, merge
        the new destination into it instead of downloading a second time."""
        tid = str(torrent_id)
        targets = [t for t in (targets or []) if t]
        now = time.time()
        with self._lock:
            row = self._conn.execute(
                f"SELECT * FROM jobs WHERE torrent_id=? AND status IN {ACTIVE_STATUSES} LIMIT 1", (tid,)
            ).fetchone()

            if row is not None:
                existing = self._row_to_dict(row)
                merged = list(dict.fromkeys(existing["targets"] + targets))
                added = [t for t in targets if t not in existing["targets"]]
                self._conn.execute(
                    "UPDATE jobs SET targets=?, updated_at=? WHERE id=?",
                    (json.dumps(merged), now, existing["id"]),
                )
                existing["targets"] = merged
                return {
                    "is_new_download": False,
                    "job_id": existing["id"],
                    "status": existing["status"],
                    "targets": merged,
                    "message": (
                        f"Torrent #{tid} đang trong tiến trình xử lý. Đã bổ sung đích đến: "
                        f"{', '.join(added)} (chỉ tải 1 lần từ TorBox)"
                        if added
                        else f"Torrent #{tid} đã nằm trong hàng đợi cho đích {', '.join(merged)}."
                    ),
                }

            cur = self._conn.execute(
                "INSERT INTO jobs (torrent_id, name, status, phase, targets, created_at, updated_at)"
                " VALUES (?,?,'queued','pending',?,?,?)",
                (tid, name or "", json.dumps(targets), now, now),
            )
            return {
                "is_new_download": True,
                "job_id": cur.lastrowid,
                "status": "queued",
                "targets": targets,
                "message": f"Đã xếp hàng tải 1 lần từ TorBox cho #{tid} và đẩy lên {', '.join(targets)}",
            }

    def claim_next(self):
        """Atomically take the oldest queued job and mark it running."""
        now = time.time()
        with self._lock:
            row = self._conn.execute(
                "SELECT * FROM jobs WHERE status='queued' ORDER BY created_at ASC LIMIT 1"
            ).fetchone()
            if row is None:
                return None
            self._conn.execute(
                "UPDATE jobs SET status='running', phase='link', attempts=attempts+1, updated_at=? WHERE id=?",
                (now, row["id"]),
            )
            return self._row_to_dict(
                self._conn.execute("SELECT * FROM jobs WHERE id=?", (row["id"],)).fetchone()
            )

    def update(self, job_id, **fields):
        if not fields:
            return
        for list_field in ("targets", "done_targets"):
            if list_field in fields and not isinstance(fields[list_field], str):
                fields[list_field] = json.dumps(list(fields[list_field]))
        fields["updated_at"] = time.time()
        cols = ", ".join(f"{k}=?" for k in fields)
        with self._lock:
            self._conn.execute(f"UPDATE jobs SET {cols} WHERE id=?", (*fields.values(), job_id))

    def finish(self, job_id, status="done", error="", message=""):
        now = time.time()
        with self._lock:
            self._conn.execute(
                "UPDATE jobs SET status=?, phase=?, error=?, message=?, progress=?, updated_at=?, finished_at=?"
                " WHERE id=?",
                (
                    status,
                    "done" if status == "done" else status,
                    error or "",
                    message or "",
                    100.0 if status == "done" else 0.0,
                    now,
                    now,
                    job_id,
                ),
            )

    def request_cancel(self, job_id):
        with self._lock:
            row = self._conn.execute("SELECT status FROM jobs WHERE id=?", (job_id,)).fetchone()
            if row is None:
                return False
            if row["status"] == "queued":
                self._conn.execute(
                    "UPDATE jobs SET status='canceled', message='Đã hủy khi còn trong hàng đợi',"
                    " updated_at=?, finished_at=? WHERE id=?",
                    (time.time(), time.time(), job_id),
                )
            elif row["status"] == "running":
                self._conn.execute(
                    "UPDATE jobs SET cancel_requested=1, updated_at=? WHERE id=?", (time.time(), job_id)
                )
            else:
                return False
            return True

    def is_cancel_requested(self, job_id):
        with self._lock:
            row = self._conn.execute("SELECT cancel_requested FROM jobs WHERE id=?", (job_id,)).fetchone()
            return bool(row and row["cancel_requested"])

    def purge_finished(self, older_than_days=30):
        cutoff = time.time() - older_than_days * 86400
        with self._lock:
            cur = self._conn.execute(
                "DELETE FROM jobs WHERE status IN ('done','failed','canceled') AND COALESCE(finished_at,0) < ?",
                (cutoff,),
            )
            return cur.rowcount

    # ---------- reads ----------

    def get(self, job_id):
        with self._lock:
            return self._row_to_dict(
                self._conn.execute("SELECT * FROM jobs WHERE id=?", (job_id,)).fetchone()
            )

    def get_active_by_torrent(self, torrent_id):
        with self._lock:
            return self._row_to_dict(
                self._conn.execute(
                    f"SELECT * FROM jobs WHERE torrent_id=? AND status IN {ACTIVE_STATUSES}"
                    " ORDER BY created_at DESC LIMIT 1",
                    (str(torrent_id),),
                ).fetchone()
            )

    def active_by_torrent_map(self):
        """One query for the whole torrent list, instead of N queries in a loop."""
        with self._lock:
            rows = self._conn.execute(
                f"SELECT * FROM jobs WHERE status IN {ACTIVE_STATUSES}"
            ).fetchall()
        return {str(r["torrent_id"]): self._row_to_dict(r) for r in rows}

    def list_active(self):
        with self._lock:
            rows = self._conn.execute(
                f"SELECT * FROM jobs WHERE status IN {ACTIVE_STATUSES} ORDER BY created_at ASC"
            ).fetchall()
        return [self._row_to_dict(r) for r in rows]

    def list_recent(self, limit=50):
        with self._lock:
            rows = self._conn.execute(
                "SELECT * FROM jobs ORDER BY COALESCE(finished_at, updated_at) DESC LIMIT ?", (limit,)
            ).fetchall()
        return [self._row_to_dict(r) for r in rows]

    def counts(self):
        with self._lock:
            rows = self._conn.execute("SELECT status, COUNT(*) c FROM jobs GROUP BY status").fetchall()
        return {r["status"]: r["c"] for r in rows}
