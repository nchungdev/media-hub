#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Google Drive & Plex Library Core Module

Reads come from the SQLite library index (core/library_store.py). The previous JSON
cache stored one entry per rclone invocation, so opening a show meant an `rclone lsf`
per season, each rewriting the whole cache file. A refresh is now a single recursive
`rclone lsjson -R` that fills shows, seasons and files in one transaction.
"""

import os
import json
import time
import shutil
import threading
import subprocess

from core.library_store import LibraryStore, DEFAULT_TTL

RCLONE_BIN = shutil.which("rclone") or "/opt/homebrew/bin/rclone"


def find_rclone_config():
    candidates = [
        os.path.expanduser("~/.config/rclone/rclone.conf"),
        os.path.expanduser("~/.agy-account2/.config/rclone/rclone.conf"),
        os.path.expanduser("~/.rclone.conf"),
    ]
    for c in candidates:
        if os.path.exists(c):
            return c
    return candidates[0]


RCLONE_CONFIG = find_rclone_config()
BASE_GDRIVE = "gdrive:Phim/TV Shows"

ASSET_NAMES = {"poster.jpg", "fanart.jpg", "tvshow.nfo"}


class GDriveManager:
    def __init__(self, rclone_bin=RCLONE_BIN, rclone_config=None, base_remote=BASE_GDRIVE, store=None):
        self.rclone_bin = rclone_bin if (isinstance(rclone_bin, str) and os.path.exists(rclone_bin)) else "rclone"
        self.rclone_config = rclone_config or find_rclone_config()
        self.base_remote = base_remote
        self.store = store or LibraryStore()
        self._refresh_lock = threading.Lock()

    def _run(self, args):
        return subprocess.run([self.rclone_bin, "--config", self.rclone_config] + args,
                              capture_output=True, text=True)

    # ---------------- refresh ----------------

    def refresh(self, force=False, ttl=DEFAULT_TTL):
        """Rescan the whole Drive library into the index. Returns True when it ran."""
        if not force and not self.store.is_stale("drive", ttl):
            return False
        if not self._refresh_lock.acquire(blocking=False):
            return False  # another refresh is already in flight
        try:
            res = self._run(["lsjson", "-R", "--files-only", "--no-modtime",
                             self.base_remote, "--timeout=60s"])
            if res.returncode != 0:
                print(f"[GDrive] lsjson thất bại: {res.stderr.strip()[:160]}")
                return False
            try:
                items = json.loads(res.stdout or "[]")
            except Exception as e:
                print(f"[GDrive] Không phân tích được lsjson: {e}")
                return False

            shows = {}
            for it in items:
                rel = (it.get("Path") or "").strip()
                if not rel:
                    continue
                parts = rel.split("/")
                show = parts[0]
                entry = shows.setdefault(show, {"path": f"{self.base_remote}/{show}",
                                                "files": [], "assets": set()})
                if len(parts) == 2:
                    season, filename = "", parts[1]
                    if filename in ASSET_NAMES:
                        entry["assets"].add(filename)
                        continue
                elif len(parts) >= 3:
                    season, filename = parts[1], parts[-1]
                else:
                    continue
                entry["files"].append((season, filename, int(it.get("Size") or 0)))

            self.store.replace_source("drive", shows)
            print(f"[GDrive] Đã lập chỉ mục {len(shows)} shows, {len(items)} files.")
            return True
        finally:
            self._refresh_lock.release()

    def _ensure_fresh(self, force_refresh=False):
        if force_refresh or self.store.is_stale("drive"):
            self.refresh(force=force_refresh)

    # ---------------- reads (indexed, no network) ----------------

    def list_tv_shows(self, force_refresh=False):
        self._ensure_fresh(force_refresh)
        return self.store.list_shows("drive")

    def get_show_seasons(self, show_name, force_refresh=False):
        self._ensure_fresh(force_refresh)
        return self.store.list_seasons(show_name, "drive")

    def get_season_files(self, show_name, season_name, force_refresh=False):
        self._ensure_fresh(force_refresh)
        return self.store.list_files(show_name, season_name, "drive")

    def stats(self):
        return self.store.stats("drive")

    def missing_assets(self):
        return self.store.missing_assets("drive")

    def mark_assets(self, show_name, filenames, source="drive"):
        """Record newly written poster/fanart/nfo without re-listing the whole remote."""
        return self.store.mark_assets(show_name, filenames, source)

    def get_cache_version(self):
        return self.store.version()

    def bump_version(self):
        self.store.set_meta("version", str(int(self.store.version() or 0) + 1))
        return self.store.version()
