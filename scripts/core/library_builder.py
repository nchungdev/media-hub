#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Library metadata builder.

Artwork and metadata belong *next to the media*, in the Plex/Jellyfin layout that
media-collector --nfo already documents:

    <Show> (<Year>) {tvdb-NNNN}/
        tvshow.nfo
        poster.jpg
        fanart.jpg

media-collector is supposed to produce these during curation. This worker backfills
the shows that predate that, driven by the "Build Library" button rather than by page
loads: fetching a poster on every request would hit TMDb for artwork the library
should already own.

For each library folder it resolves TMDb metadata, writes poster/fanart/NFO into the
folder (rclone for Drive, ssh/scp for NAS), and caches the poster locally so the
dashboard can render without touching the network.
"""

import os
import re
import json
import time
import shlex
import shutil
import tempfile
import threading
import subprocess
import difflib
import urllib.parse
import urllib.request
from pathlib import Path

from core.artwork import cache_dir, _tmdb_get, _cache_key

TMDB_IMG = "https://image.tmdb.org/t/p"


def parse_folder(folder):
    """Pull the tvdb id, title and year out of a Plex-style folder name."""
    tvdb = None
    m = re.search(r"\{tvdb-(\d+)\}", folder) or re.search(r"\[tvdbid-(\d+)\]", folder)
    if m:
        tvdb = m.group(1)
    year = None
    ym = re.search(r"\((\d{4})\)", folder)
    if ym:
        year = ym.group(1)
    title = re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", folder).strip(" -_")
    return {"tvdb_id": tvdb, "title": title, "year": year}


def _xml_escape(s):
    return (str(s or "").replace("&", "&amp;").replace("<", "&lt;")
            .replace(">", "&gt;").replace('"', "&quot;"))


def build_nfo(info):
    lines = ['<?xml version="1.0" encoding="UTF-8" standalone="yes"?>', "<tvshow>",
             f"  <title>{_xml_escape(info.get('name') or info.get('title'))}</title>",
             f"  <originaltitle>{_xml_escape(info.get('original_name') or info.get('original_title'))}</originaltitle>",
             f"  <year>{_xml_escape((info.get('first_air_date') or info.get('release_date') or '')[:4])}</year>",
             f"  <plot>{_xml_escape(info.get('overview'))}</plot>"]
    for g in info.get("genres") or []:
        lines.append(f"  <genre>{_xml_escape(g.get('name'))}</genre>")
    for s in info.get("networks") or info.get("production_companies") or []:
        lines.append(f"  <studio>{_xml_escape(s.get('name'))}</studio>")
    if info.get("id"):
        lines.append(f'  <uniqueid type="tmdb" default="true">{info["id"]}</uniqueid>')
    ext = info.get("external_ids") or {}
    if ext.get("tvdb_id"):
        lines.append(f'  <uniqueid type="tvdb">{ext["tvdb_id"]}</uniqueid>')
    if ext.get("imdb_id"):
        lines.append(f'  <uniqueid type="imdb">{ext["imdb_id"]}</uniqueid>')
    if info.get("vote_average"):
        lines.append(f"  <rating>{info['vote_average']}</rating>")
    lines.append("</tvshow>")
    return "\n".join(lines) + "\n"


class LibraryBuilder:
    """One build at a time; progress is readable while it runs."""

    def __init__(self, gdrive_mgr, settings_loader, nas_lister):
        self.gdrive = gdrive_mgr
        self.load_settings = settings_loader
        self.list_nas = nas_lister
        self._lock = threading.Lock()
        self._state = {"running": False, "total": 0, "done": 0, "current": "",
                       "started_at": None, "finished_at": None, "results": [], "error": ""}
        self._cancel = threading.Event()

    # ---- state ----

    def status(self):
        with self._lock:
            return dict(self._state, results=list(self._state["results"])[-50:])

    def _set(self, **kw):
        with self._lock:
            self._state.update(kw)

    def cancel(self):
        self._cancel.set()
        return True

    def start(self, targets=("drive",), only_missing=True):
        with self._lock:
            if self._state["running"]:
                return {"success": False, "error": "Một tiến trình build đang chạy."}
            self._cancel.clear()
            self._state = {"running": True, "total": 0, "done": 0, "current": "Đang liệt kê thư viện...",
                           "started_at": time.time(), "finished_at": None, "results": [], "error": ""}
        threading.Thread(target=self._run, args=(list(targets), only_missing),
                         name="library-builder", daemon=True).start()
        return {"success": True, "message": "Đã bắt đầu dựng metadata thư viện."}

    # ---- the build ----

    def _run(self, targets, only_missing):
        try:
            cfg = self.load_settings()
            api_key = cfg.get("tmdb_api_key") or os.environ.get("TMDB_API_KEY")
            if not api_key:
                self._set(running=False, finished_at=time.time(),
                          error="Chưa cấu hình TMDb API Key trong tab Cài Đặt.")
                return

            folders = []
            if "drive" in targets:
                folders += [(s["name"], "drive") for s in (self.gdrive.list_tv_shows() or [])]
            if "nas" in targets:
                folders += [(n, "nas") for n in (self.list_nas() or [])]
            if not folders:
                self._set(running=False, finished_at=time.time(),
                          error="Không liệt kê được thư mục nào (kiểm tra rclone / SSH).")
                return

            self._set(total=len(folders))
            for folder, where in folders:
                if self._cancel.is_set():
                    self._append({"folder": folder, "status": "canceled"})
                    break
                self._set(current=f"[{where}] {folder}")
                try:
                    self._append(self._build_one(folder, where, cfg, api_key, only_missing))
                except Exception as e:
                    self._append({"folder": folder, "target": where, "status": "error", "detail": str(e)})
                with self._lock:
                    self._state["done"] += 1
            self._set(running=False, current="", finished_at=time.time())
        except Exception as e:
            self._set(running=False, finished_at=time.time(), error=str(e))

    def _append(self, row):
        with self._lock:
            self._state["results"].append(row)

    def _build_one(self, folder, where, cfg, api_key, only_missing):
        meta = parse_folder(folder)
        existing = self._existing_assets(folder, where, cfg)
        wanted = {"poster.jpg", "fanart.jpg", "tvshow.nfo"}
        missing = wanted - existing if only_missing else wanted
        if not missing:
            return {"folder": folder, "target": where, "status": "skipped",
                    "detail": "Đã đủ poster/fanart/nfo"}

        info = self._resolve(api_key, meta)
        if not info:
            return {"folder": folder, "target": where, "status": "needs_review",
                    "detail": f"Không tìm được kết quả TMDb đủ tin cậy cho \"{meta['title']}\" "
                              f"— bỏ qua để tránh ghi metadata sai."}

        match = info.get("_match") or {}
        if match.get("low_confidence"):
            return {"folder": folder, "target": where, "status": "needs_review",
                    "detail": f"Khớp yếu (score {match.get('score')}): \"{info.get('name') or info.get('title')}\" "
                              f"— chưa ghi, cần xác nhận thủ công.",
                    "tmdb_id": info.get("id"),
                    "matched_title": info.get("name") or info.get("title")}

        with tempfile.TemporaryDirectory() as tmp:
            tmpd = Path(tmp)
            staged = []
            if "poster.jpg" in missing and info.get("poster_path"):
                if self._download(f"{TMDB_IMG}/w500{info['poster_path']}", tmpd / "poster.jpg"):
                    staged.append("poster.jpg")
                    self._cache_poster(meta, tmpd / "poster.jpg")
            if "fanart.jpg" in missing and info.get("backdrop_path"):
                if self._download(f"{TMDB_IMG}/w1280{info['backdrop_path']}", tmpd / "fanart.jpg"):
                    staged.append("fanart.jpg")
            if "tvshow.nfo" in missing:
                (tmpd / "tvshow.nfo").write_text(build_nfo(info), encoding="utf-8")
                staged.append("tvshow.nfo")

            if not staged:
                return {"folder": folder, "target": where, "status": "not_found",
                        "detail": "TMDb không có artwork cho mục này"}
            self._upload(folder, where, cfg, tmpd, staged)
            if where == "drive":
                try:
                    self.gdrive.mark_assets(folder, staged)
                except Exception:
                    pass

        return {"folder": folder, "target": where, "status": "built",
                "detail": ", ".join(staged), "tmdb_id": info.get("id"),
                "matched_title": info.get("name") or info.get("title"),
                "match": info.get("_match")}

    @staticmethod
    def _titles_of(info):
        out = []
        for k in ("name", "title", "original_name", "original_title"):
            if info.get(k):
                out.append(info[k])
        for alt in (info.get("alternative_titles") or {}).get("results", []) or []:
            if alt.get("title"):
                out.append(alt["title"])
        return out

    @classmethod
    def _title_score(cls, wanted, info):
        """Best fuzzy match between the folder title and any title TMDb knows."""
        def norm(x):
            return re.sub(r"[^a-z0-9]+", " ", str(x).lower()).strip()
        w = norm(wanted)
        if not w:
            return 0.0
        return max((difflib.SequenceMatcher(None, w, norm(t)).ratio()
                    for t in cls._titles_of(info)), default=0.0)

    def _resolve(self, api_key, meta, min_score=0.62):
        """Resolve to a TMDb record, but only accept a confident match.

        A bare /find on a TVDB id is not enough: TVDB ids collide with unrelated shows
        in TMDb's index (454526 comes back as "DARK MOON: THE BLOOD ALTAR" for a folder
        named WUKONG), and the search fallback happily returns any first hit. Writing
        that into tvshow.nfo would corrupt the library, so require either the external
        id to round-trip or the title to actually look alike.
        """
        try:
            if meta["tvdb_id"]:
                found = _tmdb_get(f"/find/{meta['tvdb_id']}", {"external_source": "tvdb_id"}, api_key)
                for bucket, kind in (("tv_results", "tv"), ("movie_results", "movie")):
                    for item in found.get(bucket) or []:
                        info = _tmdb_get(f"/{kind}/{item['id']}",
                                         {"append_to_response": "external_ids,alternative_titles"}, api_key)
                        ext = str((info.get("external_ids") or {}).get("tvdb_id") or "")
                        score = self._title_score(meta["title"], info)
                        if ext == str(meta["tvdb_id"]) and score >= min_score:
                            info["_match"] = {"via": "tvdb_id", "score": round(score, 2)}
                            return info
                        # id matched but the name does not: almost certainly a collision.
                        info["_reject"] = {"via": "tvdb_id", "score": round(score, 2),
                                           "got": info.get("name") or info.get("title")}
                        rejected = info

            search = _tmdb_get("/search/multi", {"query": meta["title"]}, api_key)
            same_year, other = [], []
            for item in (search.get("results") or [])[:10]:
                if item.get("media_type") not in ("tv", "movie"):
                    continue
                score = self._title_score(meta["title"], item)
                d = (item.get("first_air_date") or item.get("release_date") or "")[:4]
                (same_year if (meta.get("year") and d == meta["year"]) else other).append((score, item))

            # The folder states a year, so a candidate from that year beats a closer
            # title from another year: "The File of Young Kindaichi (1997)" must not
            # resolve to the 2014 "Returns" sequel just because the names overlap.
            pick, via, floor = None, "title", min_score
            if same_year:
                pick = max(same_year, key=lambda x: x[0])
                via, floor = "title+year", 0.35
            if pick is None or pick[0] < floor:
                cand = max(other, key=lambda x: x[0]) if other else None
                if cand and (pick is None or cand[0] > pick[0]):
                    pick, via, floor = cand, "title", min_score

            if pick and pick[0] >= floor:
                best_score, best = pick
                info = _tmdb_get(f"/{best['media_type']}/{best['id']}",
                                 {"append_to_response": "external_ids,alternative_titles"}, api_key)
                info["_match"] = {"via": via, "score": round(best_score, 2),
                                  "low_confidence": best_score < min_score}
                return info
        except Exception:
            return None
        return None

    @staticmethod
    def _download(url, dest):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
            with urllib.request.urlopen(req, timeout=30) as r:
                data = r.read()
            if data:
                dest.write_bytes(data)
                return True
        except Exception:
            pass
        return False

    @staticmethod
    def _cache_poster(meta, src):
        """Keep a local copy so the dashboard renders without hitting Drive or TMDb."""
        try:
            root = cache_dir()
            key = _cache_key(meta.get("tvdb_id"), None, meta.get("title"))
            shutil.copyfile(src, root / f"{key}.jpg")
            if meta.get("tvdb_id") and meta.get("title"):
                # Also key it by title so a lookup without the id still hits the cache.
                shutil.copyfile(src, root / f"{_cache_key(None, None, meta['title'])}.jpg")
        except Exception:
            pass

    def _existing_assets(self, folder, where, cfg):
        try:
            if where == "drive":
                # The library index already records which assets each folder has, so a
                # build costs zero extra rclone calls for the ones already complete.
                for row in (self.gdrive.list_tv_shows() or []):
                    if row.get("name") == folder:
                        return {n for n, present in (("poster.jpg", row.get("has_poster")),
                                                     ("fanart.jpg", row.get("has_fanart")),
                                                     ("tvshow.nfo", row.get("has_nfo"))) if present}
            else:
                out = self._ssh(cfg, f"ls -1 {shlex.quote(cfg.get('nas_path','').rstrip('/') + '/' + folder)}")
                if out is not None:
                    return {l.strip() for l in out.splitlines() if l.strip()}
        except Exception:
            pass
        return set()

    def _ssh(self, cfg, remote_cmd, timeout=30):
        host, user = cfg.get("nas_host", ""), cfg.get("nas_user", "")
        if not host:
            return None
        key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
        cmd = ["ssh", "-p", str(int(cfg.get("nas_port", 22))), "-o", "BatchMode=yes",
               "-o", "ConnectTimeout=6", "-o", "StrictHostKeyChecking=no"]
        if key and os.path.exists(key):
            cmd += ["-i", key]
        cmd += [f"{user}@{host}", remote_cmd]
        res = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)
        return res.stdout if res.returncode == 0 else None

    def _upload(self, folder, where, cfg, tmpd, files):
        if where == "drive":
            remote = f"{cfg.get('gdrive_remote','gdrive')}:{cfg.get('gdrive_root','Phim')}/TV Shows/{folder}"
            cmd = [self.gdrive.rclone_bin, "--config", self.gdrive.rclone_config,
                   "copy", str(tmpd), remote, "--include", "{" + ",".join(files) + "}"]
            res = subprocess.run(cmd, capture_output=True, text=True, timeout=180)
            if res.returncode != 0:
                raise RuntimeError(res.stderr.strip()[:200] or "rclone copy thất bại")
        else:
            host, user = cfg.get("nas_host", ""), cfg.get("nas_user", "")
            if not host:
                raise RuntimeError("Chưa cấu hình NAS")
            dest = f"{cfg.get('nas_path','').rstrip('/')}/{folder}"
            self._ssh(cfg, f"mkdir -p {shlex.quote(dest)}")
            key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
            scp = ["scp", "-P", str(int(cfg.get("nas_port", 22))),
                   "-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=no"]
            if key and os.path.exists(key):
                scp += ["-i", key]
            scp += [str(tmpd / f) for f in files]
            scp += [f"{user}@{host}:{shlex.quote(dest)}/"]
            res = subprocess.run(scp, capture_output=True, text=True, timeout=180)
            if res.returncode != 0:
                raise RuntimeError(res.stderr.strip()[:200] or "scp thất bại")
