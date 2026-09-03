#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Library index — the Google Drive / NAS catalogue, cached in SQLite.

Replaces gdrive_cache.json, which stored one entry per rclone call: listing a show's
episodes meant a separate `rclone lsf` round trip per season, each rewriting the whole
JSON file. Browsing a library of 30 shows cost dozens of network calls.

The refresh now does a single recursive `rclone lsjson -R` and stores shows, seasons
and files as rows, so every later read is a local indexed query.
"""

import os
import re
import json
import time
import sqlite3
import threading
import subprocess
from pathlib import Path

DEFAULT_TTL = 900  # 15 minutes, matching the old cache window

SCHEMA = """
CREATE TABLE IF NOT EXISTS library_shows (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    source      TEXT NOT NULL,
    name        TEXT NOT NULL,
    path        TEXT NOT NULL DEFAULT '',
    tvdb_id     TEXT NOT NULL DEFAULT '',
    title       TEXT NOT NULL DEFAULT '',
    year        TEXT NOT NULL DEFAULT '',
    has_poster  INTEGER NOT NULL DEFAULT 0,
    has_fanart  INTEGER NOT NULL DEFAULT 0,
    has_nfo     INTEGER NOT NULL DEFAULT 0,
    file_count  INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    updated_at  REAL NOT NULL,
    UNIQUE(source, name)
);

CREATE TABLE IF NOT EXISTS library_files (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    show_id   INTEGER NOT NULL REFERENCES library_shows(id) ON DELETE CASCADE,
    season    TEXT NOT NULL DEFAULT '',
    filename  TEXT NOT NULL,
    size      INTEGER NOT NULL DEFAULT 0,
    UNIQUE(show_id, season, filename)
);

CREATE TABLE IF NOT EXISTS library_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_files_show   ON library_files(show_id, season);
CREATE INDEX IF NOT EXISTS idx_shows_source ON library_shows(source);
"""

_ASSETS = {"poster.jpg": "has_poster", "fanart.jpg": "has_fanart", "tvshow.nfo": "has_nfo"}


def parse_show_name(name):
    m = re.search(r"\{tvdb-(\d+)\}", name) or re.search(r"\[tvdbid-(\d+)\]", name)
    ym = re.search(r"\((\d{4})\)", name)
    return {
        "tvdb_id": m.group(1) if m else "",
        "year": ym.group(1) if ym else "",
        "title": re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", name).strip(" -_"),
    }


class LibraryStore:
    def __init__(self, db_path=None):
        from core.job_store import default_db_path
        self.db_path = Path(db_path) if db_path else default_db_path()
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        self._lock = threading.RLock()
        self._conn = sqlite3.connect(str(self.db_path), check_same_thread=False, isolation_level=None)
        self._conn.row_factory = sqlite3.Row
        with self._lock:
            self._conn.execute("PRAGMA journal_mode=WAL")
            self._conn.execute("PRAGMA synchronous=NORMAL")
            self._conn.execute("PRAGMA busy_timeout=5000")
            self._conn.execute("PRAGMA foreign_keys=ON")
            self._conn.executescript(SCHEMA)

    # ---- meta ----

    def get_meta(self, key, default=None):
        with self._lock:
            row = self._conn.execute("SELECT value FROM library_meta WHERE key=?", (key,)).fetchone()
        return row["value"] if row else default

    def set_meta(self, key, value):
        with self._lock:
            self._conn.execute(
                "INSERT INTO library_meta(key,value) VALUES(?,?) "
                "ON CONFLICT(key) DO UPDATE SET value=excluded.value", (key, str(value)))

    def last_refresh(self, source):
        try:
            return float(self.get_meta(f"last_refresh:{source}", 0) or 0)
        except ValueError:
            return 0.0

    def is_stale(self, source, ttl=DEFAULT_TTL):
        return (time.time() - self.last_refresh(source)) > ttl

    def version(self):
        return self.get_meta("version", "0")

    # ---- writes ----

    def replace_source(self, source, shows):
        """Swap in a freshly scanned catalogue for one source, in a single transaction.

        `shows` maps folder name -> {"path", "files": [(season, filename, size)], "assets": {names}}
        """
        now = time.time()
        with self._lock:
            self._conn.execute("BEGIN IMMEDIATE")
            try:
                self._conn.execute("DELETE FROM library_shows WHERE source=?", (source,))
                for name, data in shows.items():
                    meta = parse_show_name(name)
                    assets = data.get("assets") or set()
                    files = data.get("files") or []
                    media = [f for f in files if os.path.splitext(f[1])[1].lower()
                             in (".mkv", ".mp4", ".avi", ".m4v", ".ts")]
                    cur = self._conn.execute(
                        "INSERT INTO library_shows"
                        "(source,name,path,tvdb_id,title,year,has_poster,has_fanart,has_nfo,"
                        " file_count,total_bytes,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?,?)",
                        (source, name, data.get("path", ""), meta["tvdb_id"], meta["title"], meta["year"],
                         int("poster.jpg" in assets), int("fanart.jpg" in assets), int("tvshow.nfo" in assets),
                         len(media), sum(f[2] for f in files), now))
                    show_id = cur.lastrowid
                    if files:
                        self._conn.executemany(
                            "INSERT OR IGNORE INTO library_files(show_id,season,filename,size) VALUES(?,?,?,?)",
                            [(show_id, s, fn, sz) for s, fn, sz in files])
                self._conn.execute("COMMIT")
            except Exception:
                self._conn.execute("ROLLBACK")
                raise
        self.set_meta(f"last_refresh:{source}", now)
        self.set_meta("version", str(int(self.version() or 0) + 1))

    def mark_assets(self, show_name, filenames, source="drive"):
        cols = [_ASSETS[f] for f in filenames if f in _ASSETS]
        if not cols:
            return False
        sets = ", ".join(f"{c}=1" for c in cols)
        with self._lock:
            cur = self._conn.execute(
                f"UPDATE library_shows SET {sets}, updated_at=? WHERE source=? AND name=?",
                (time.time(), source, show_name))
        return cur.rowcount > 0

    # ---- reads ----

    def list_shows(self, source="drive"):
        with self._lock:
            rows = self._conn.execute(
                "SELECT * FROM library_shows WHERE source=? ORDER BY name COLLATE NOCASE", (source,)
            ).fetchall()
        return [{"name": r["name"], "path": r["path"], "tvdb_id": r["tvdb_id"],
                 "title": r["title"], "year": r["year"], "file_count": r["file_count"],
                 "total_bytes": r["total_bytes"],
                 "has_poster": bool(r["has_poster"]), "has_fanart": bool(r["has_fanart"]),
                 "has_nfo": bool(r["has_nfo"])} for r in rows]

    def list_seasons(self, show_name, source="drive"):
        with self._lock:
            rows = self._conn.execute(
                "SELECT f.season, COUNT(*) c FROM library_files f "
                "JOIN library_shows s ON s.id=f.show_id "
                "WHERE s.source=? AND s.name=? AND f.season<>'' "
                "GROUP BY f.season ORDER BY f.season COLLATE NOCASE", (source, show_name)).fetchall()
        return [r["season"] for r in rows]

    def list_files(self, show_name, season, source="drive"):
        with self._lock:
            rows = self._conn.execute(
                "SELECT f.filename FROM library_files f JOIN library_shows s ON s.id=f.show_id "
                "WHERE s.source=? AND s.name=? AND f.season=? ORDER BY f.filename COLLATE NOCASE",
                (source, show_name, season)).fetchall()
        return [r["filename"] for r in rows]

    def missing_assets(self, source="drive"):
        with self._lock:
            rows = self._conn.execute(
                "SELECT name, has_poster, has_fanart, has_nfo FROM library_shows "
                "WHERE source=? AND (has_poster=0 OR has_fanart=0 OR has_nfo=0) "
                "ORDER BY name COLLATE NOCASE", (source,)).fetchall()
        return [{"name": r["name"],
                 "missing": [k for k, col in (("poster.jpg", "has_poster"),
                                              ("fanart.jpg", "has_fanart"),
                                              ("tvshow.nfo", "has_nfo")) if not r[col]]}
                for r in rows]

    def stats(self, source="drive"):
        with self._lock:
            row = self._conn.execute(
                "SELECT COUNT(*) shows, COALESCE(SUM(file_count),0) files, "
                "COALESCE(SUM(total_bytes),0) bytes, "
                "SUM(has_poster) posters, SUM(has_nfo) nfos "
                "FROM library_shows WHERE source=?", (source,)).fetchone()
        return {"shows": row["shows"], "files": row["files"], "bytes": row["bytes"],
                "with_poster": row["posters"] or 0, "with_nfo": row["nfos"] or 0,
                "last_refresh": self.last_refresh(source)}
