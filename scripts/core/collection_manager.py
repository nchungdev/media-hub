#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Media Collection Manager — Core Engine for Unified Media Collections.

Aggregates Movies and TV Series across:
1. Local Workspace & Staging (.media-hub, TV Shows, Movies)
2. Remote NAS Storage (SSH query / MergerFS Pool)
3. Google Drive / Plex Storage (via rclone & SQLite LibraryStore)
4. Subtitle Studio (Vietnamese & English subtitle files)
5. TorBox Cloud / Local Active Downloads

Provides the 3 core status pillars for every media item:
- Download Status (Local Video File)
- Sync Status (NAS Storage & Google Drive)
- Subtitle Status (Vietsub .ass, .srt, .vtt)
"""

import os
import re
import json
import time
import shlex
import urllib.parse
import subprocess
import threading
from pathlib import Path

from core.settings import load_unified_settings
from core.library_store import LibraryStore

KNOWN_COLLECTIONS_META = {
    "72281": {"title": "Black Jack (1993)", "vn": "Bác Sĩ Quái Dị Black Jack (OVA)", "type": "series", "year": "1993", "qual": "1080p BDRip", "episodes": 12, "vietsub": True},
    "81092": {"title": "Black Jack (2004)", "vn": "Bác Sĩ Quái Dị Black Jack (TV Series)", "type": "series", "year": "2004", "qual": "1080p BDRip", "episodes": 89, "vietsub": True},
    "79354": {"title": "The File of Young Kindaichi (1997)", "vn": "Thám Tử Kindaichi (Anime 1997)", "type": "series", "year": "1997", "qual": "480p DVD", "episodes": 148, "vietsub": True},
    "279782": {"title": "The File of Young Kindaichi Returns (2014)", "vn": "Thám Tử Kindaichi Returns", "type": "series", "year": "2014", "qual": "1080p BDRip", "episodes": 47, "vietsub": True},
    "79460": {"title": "The Files of the Young Kindaichi (1995)", "vn": "Thám Tử Kindaichi (Live Action)", "type": "series", "year": "1995", "qual": "1080p BDRip", "episodes": 13, "vietsub": True},
    "227501": {"title": "Mashin Hero Wataru (1988)", "vn": "Thần Long Đấu Sĩ Wataru", "type": "series", "year": "1988", "qual": "1080p BDRip", "episodes": 150, "vietsub": True},
    "74599": {"title": "Monster (2004)", "vn": "Quái Vật Monster", "type": "series", "year": "2004", "qual": "1080p BluRay", "episodes": 74, "vietsub": True},
    "75939": {"title": "Battle B-Daman (2004)", "vn": "Chiến Binh B-Daman", "type": "series", "year": "2004", "qual": "1080p / 480p", "episodes": 103, "vietsub": True},
    "79178": {"title": "Transformers - Car Robots (2000)", "vn": "Transformers: Car Robots", "type": "series", "year": "2000", "qual": "480p DVD", "episodes": 39, "vietsub": False},
    "454526": {"title": "WUKONG: Đại Viên Hồn (2025)", "vn": "Tây Hành Kỷ: Đại Viên Hồn", "type": "series", "year": "2025", "qual": "1080p WEB-DL", "episodes": 12, "vietsub": True},
    "350711": {"title": "The Westward (2018)", "vn": "Tây Hành Kỷ", "type": "series", "year": "2018", "qual": "1080p WEB-DL", "episodes": 21, "vietsub": True},
    "259259": {"title": "Kingdom (2012)", "vn": "Vương Giả Thiên Hạ", "type": "series", "year": "2012", "qual": "1080p BDRip", "episodes": 150, "vietsub": True},
    "80674": {"title": "Furuhata Ninzaburo (1994)", "vn": "Thám Tử Cổ Điển Furuhata", "type": "series", "year": "1994", "qual": "480p DVD", "episodes": 44, "vietsub": True},
    "320122": {"title": "The Three-Eyed One (1990)", "vn": "Cậu Bé 3 Mắt (Mitsume ga Tooru)", "type": "series", "year": "1990", "qual": "480p DVD", "episodes": 48, "vietsub": True},
    "230211": {"title": "Tantei Gakuen Q (2003)", "vn": "Học Viện Thám Tử Q", "type": "series", "year": "2003", "qual": "480p DVD", "episodes": 45, "vietsub": True},
    "335191": {"title": "Hakyuu Houshin Engi (2018)", "vn": "Bá Khí Phong Thần Diễn Nghĩa", "type": "series", "year": "2018", "qual": "1080p BDRip", "episodes": 24, "vietsub": True},
    "79284": {"title": "Houshin Engi (1999)", "vn": "Phong Thần Bảng (1999)", "type": "series", "year": "1999", "qual": "480p DVD", "episodes": 26, "vietsub": True},
    "299770": {"title": "Young Black Jack (2015)", "vn": "Bác Sĩ Black Jack Thời Trẻ", "type": "series", "year": "2015", "qual": "1080p BDRip", "episodes": 12, "vietsub": True},
    "252384": {"title": "Young Black Jack (2015)", "vn": "Bác Sĩ Black Jack Thời Trẻ", "type": "series", "year": "2015", "qual": "1080p BDRip", "episodes": 12, "vietsub": True}
}


class MediaCollectionManager:
    _instance = None
    _lock = threading.RLock()

    def __new__(cls, *args, **kwargs):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(MediaCollectionManager, cls).__new__(cls)
                cls._instance._init_mgr()
            return cls._instance

    def _init_mgr(self):
        self._cache = None
        self._last_fetch = 0
        self._ttl = 20  # 20 seconds cache window for snappy UI

    def get_collections(self, force_refresh=False):
        now = time.time()
        if not force_refresh and self._cache is not None and (now - self._last_fetch) < self._ttl:
            return self._cache

        with self._lock:
            if not force_refresh and self._cache is not None and (now - self._last_fetch) < self._ttl:
                return self._cache
            self._cache = self._scan_all()
            self._last_fetch = time.time()
            return self._cache

    def _scan_all(self):
        cfg = load_unified_settings()
        hub_home = cfg.get("media_hub_home") or os.path.join(os.getcwd(), ".media-hub")
        workspace_dir = cfg.get("workspace_dir") or os.getcwd()
        staging_dir = cfg.get("staging_dir") or os.path.join(hub_home, ".staging")
        
        # 1. Google Drive Catalog
        gdrive_shows = {}
        try:
            store = LibraryStore()
            shows = store.list_shows("drive")
            for s in shows:
                gdrive_shows[s["name"]] = s
        except Exception:
            pass

        # 2. NAS Storage Directory Listing via SSH
        nas_folders = set()
        nas_base = cfg.get("nas_path", "/srv/mergerfs/MainPool/Phim/TV Shows").rstrip("/")
        if "/volume1/" in nas_base:
            nas_base = "/srv/mergerfs/MainPool/Phim/TV Shows"
        key = os.path.expanduser(cfg.get("nas_ssh_key", "~/.ssh/id_ed25519"))
        user = cfg.get("nas_user", "chungnh")
        host = cfg.get("nas_host", "192.168.1.37")
        try:
            ssh_cmd = ["ssh", "-p", "22", "-o", "BatchMode=yes", "-o", "ConnectTimeout=3", "-o", "StrictHostKeyChecking=no"]
            if os.path.exists(key):
                ssh_cmd += ["-i", key]
            q_base = shlex.quote(nas_base)
            ssh_cmd += [f"{user}@{host}", f'if [ -d {q_base} ]; then ls -1 {q_base}; fi']
            res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=5)
            if res.returncode == 0:
                for line in res.stdout.splitlines():
                    l = line.strip()
                    if l:
                        nas_folders.add(l)
        except Exception:
            pass

        # 3. Local Workspace Scan (.media-hub, TV Shows, Movies)
        local_projects = {}
        candidate_roots = [hub_home, workspace_dir]
        for c_root in candidate_roots:
            if not os.path.exists(c_root):
                continue
            # Look for TV Shows and Movies subfolders
            for sub in ["TV Shows", "Movies"]:
                dir_path = os.path.join(c_root, sub)
                if os.path.exists(dir_path):
                    for item in os.listdir(dir_path):
                        if item.startswith("."):
                            continue
                        sp = os.path.join(dir_path, item)
                        if os.path.isdir(sp):
                            local_projects[item] = sp
            # Also check .media-hub subprojects (e.g. .media-hub/Monster/TV Shows/...)
            for item in os.listdir(c_root):
                if item.startswith("."):
                    continue
                sp = os.path.join(c_root, item)
                if os.path.isdir(sp):
                    for sub in ["TV Shows", "Movies"]:
                        s_sub = os.path.join(sp, sub)
                        if os.path.exists(s_sub):
                            for sub_item in os.listdir(s_sub):
                                sub_p = os.path.join(s_sub, sub_item)
                                if os.path.isdir(sub_p):
                                    local_projects[sub_item] = sub_p

        # 4. Synthesize all unique collection keys grouped by canonical identity
        def get_canonical_key(f_name: str) -> str:
            m_t = re.search(r"\{tvdb-(\d+)\}", f_name) or re.search(r"\[tvdbid-(\d+)\]", f_name)
            if m_t:
                return f"tvdb-{m_t.group(1)}"
            m_tm = re.search(r"\{tmdb-(\d+)\}", f_name) or re.search(r"\[tmdbid-(\d+)\]", f_name)
            if m_tm:
                return f"tmdb-{m_tm.group(1)}"
            cl = re.sub(r"\{.*?\}|\[.*?\]", "", f_name).strip().lower()
            return re.sub(r"\s+", " ", cl)

        all_keys = set(gdrive_shows.keys()) | nas_folders | set(local_projects.keys())
        grouped_folders = {}
        for f in all_keys:
            ck = get_canonical_key(f)
            if ck not in grouped_folders:
                grouped_folders[ck] = []
            grouped_folders[ck].append(f)

        collections = []
        for ck, folder_list in sorted(grouped_folders.items()):
            # Select primary folder for metadata display (prefer ones with TVDB tag)
            folder_list.sort(key=lambda x: (0 if "tvdb" in x.lower() else 1, -len(x)))
            folder = folder_list[0]

            m_tvdb = re.search(r"\{tvdb-(\d+)\}", folder) or re.search(r"\[tvdbid-(\d+)\]", folder)
            tvdb_id = m_tvdb.group(1) if m_tvdb else ""
            if not tvdb_id and ck.startswith("tvdb-"):
                tvdb_id = ck.replace("tvdb-", "")
            
            m_tmdb = re.search(r"\{tmdb-(\d+)\}", folder) or re.search(r"\[tmdbid-(\d+)\]", folder)
            tmdb_id = m_tmdb.group(1) if m_tmdb else ""
            if not tmdb_id and ck.startswith("tmdb-"):
                tmdb_id = ck.replace("tmdb-", "")
            
            m_year = re.search(r"\((\d{4})\)", folder)
            year = m_year.group(1) if m_year else ""

            meta = KNOWN_COLLECTIONS_META.get(tvdb_id, {})
            clean_title = meta.get("title") or re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", folder).strip(" -_")
            vn_title = meta.get("vn", clean_title)
            media_type = meta.get("type", "series" if ("TV Shows" in folder or "S01" in folder or "Season" in folder or meta.get("episodes", 0) > 1) else "series")
            if "movie" in folder.lower() or "ova" in folder.lower():
                media_type = "movie"

            in_gdrive = any(f in gdrive_shows for f in folder_list)
            in_nas = any(f in nas_folders for f in folder_list)
            in_local = any(f in local_projects for f in folder_list)

            local_paths = [local_projects[f] for f in folder_list if f in local_projects and os.path.exists(local_projects[f])]
            primary_local_path = local_paths[0] if local_paths else ""

            # Scan episodes / files for detailed tri-status across all local folders
            episodes_dict = {}
            for lp in local_paths:
                for root, dirs, files in os.walk(lp):
                    for f in files:
                        if f.startswith("."):
                            continue
                        m_ep = re.search(r"S(\d+)E(\d+)", f, re.IGNORECASE)
                        if m_ep:
                            s_num = int(m_ep.group(1))
                            e_num = int(m_ep.group(2))
                            ep_key = f"S{s_num:02d}E{e_num:02d}"
                            if ep_key not in episodes_dict:
                                episodes_dict[ep_key] = {
                                    "key": ep_key,
                                    "season_num": s_num,
                                    "ep_num": e_num,
                                    "name": f,
                                    "video": False,
                                    "in_nas": in_nas,
                                    "in_gdrive": in_gdrive,
                                    "vi_ass": False,
                                    "vi_srt": False,
                                    "vi_vtt": False,
                                    "eng_sub": False
                                }
                            if f.endswith((".mkv", ".mp4", ".m4v", ".avi")):
                                episodes_dict[ep_key]["video"] = True
                                episodes_dict[ep_key]["name"] = f
                            elif f.endswith(".vi.ass"):
                                episodes_dict[ep_key]["vi_ass"] = True
                            elif f.endswith(".vi.srt"):
                                episodes_dict[ep_key]["vi_srt"] = True
                            elif f.endswith(".vi.vtt"):
                                episodes_dict[ep_key]["vi_vtt"] = True
                            elif f.endswith((".eng.ass", ".eng.srt", ".ass", ".srt")):
                                episodes_dict[ep_key]["eng_sub"] = True

            # If episodes not found locally, generate from known metadata or GDrive count
            total_eps = meta.get("episodes", len(episodes_dict))
            if total_eps == 0:
                for f in folder_list:
                    if f in gdrive_shows:
                        total_eps = max(total_eps, gdrive_shows[f].get("file_count", 0))

            # Calculate counts
            downloaded_video_count = sum(1 for ep in episodes_dict.values() if ep["video"])
            vietsub_count = sum(1 for ep in episodes_dict.values() if (ep["vi_ass"] or ep["vi_srt"] or ep["vi_vtt"]))
            if vietsub_count == 0 and meta.get("vietsub", False):
                # If marked as having full vietsub on remote
                vietsub_count = total_eps

            # Pillar 1: Download Status
            if downloaded_video_count >= total_eps and total_eps > 0:
                dl_state = "complete"
                dl_label = f"✓ {downloaded_video_count}/{total_eps} tập (Đủ Video)"
                dl_color = "emerald"
            elif downloaded_video_count > 0:
                dl_state = "partial"
                dl_label = f"⏳ {downloaded_video_count}/{total_eps} tập (Local Buffer)"
                dl_color = "amber"
            else:
                dl_state = "cloud"
                dl_label = "☁️ Đám Mây"
                dl_color = "blue"

            # Pillar 2: Sync Status
            if in_nas and in_gdrive:
                sync_state = "synced_both"
                sync_label = "🟢 Cả NAS & Drive"
                sync_color = "emerald"
            elif in_nas:
                sync_state = "only_nas"
                sync_label = "🟡 Chỉ NAS"
                sync_color = "amber"
            elif in_gdrive:
                sync_state = "only_gdrive"
                sync_label = "🔵 Chỉ Drive"
                sync_color = "blue"
            else:
                sync_state = "unsynced"
                sync_label = "⚪ Chưa có trên NAS"
                sync_color = "zinc"

            # Pillar 3: Subtitle Status
            sub_percent = round((vietsub_count / total_eps * 100) if total_eps > 0 else 0, 1)
            if sub_percent >= 100:
                sub_state = "complete"
                sub_label = "🎉 100% Vietsub"
                sub_color = "emerald"
            elif vietsub_count > 0:
                sub_state = "translating"
                sub_label = f"⏳ {sub_percent}% Vietsub"
                sub_color = "amber"
            else:
                sub_state = "missing"
                sub_label = "⚪ Chưa Dịch"
                sub_color = "zinc"

            # Group episodes by Season
            seasons_map = {}
            for ep_key, ep in sorted(episodes_dict.items()):
                s_num = ep["season_num"]
                if s_num not in seasons_map:
                    s_name = "Specials (Season 00)" if s_num == 0 else f"Season {s_num:02d}"
                    seasons_map[s_num] = {
                        "season_num": s_num,
                        "name": s_name,
                        "episodes": []
                    }
                has_vi = ep["vi_ass"] or ep["vi_srt"] or ep["vi_vtt"]
                sub_tags = []
                if ep["vi_ass"]: sub_tags.append(".vi.ass")
                if ep["vi_srt"]: sub_tags.append(".vi.srt")
                if ep["vi_vtt"]: sub_tags.append(".vi.vtt")

                seasons_map[s_num]["episodes"].append({
                    "key": ep["key"],
                    "num": ep["ep_num"],
                    "name": ep["name"],
                    "video": ep["video"],
                    "in_nas": in_nas,
                    "in_gdrive": in_gdrive,
                    "has_vi_sub": has_vi,
                    "sub_types": sub_tags,
                    "has_eng_sub": ep["eng_sub"]
                })

            seasons_list = [seasons_map[k] for k in sorted(seasons_map.keys())]

            if tvdb_id:
                poster_url = f"/api/poster?tvdb={tvdb_id}"
            elif tmdb_id:
                poster_url = f"/api/poster?tmdb={tmdb_id}"
            else:
                poster_url = f"/api/poster?title={urllib.parse.quote(clean_title)}"

            collections.append({
                "id": f"col-{tvdb_id or tmdb_id or folder}",
                "folder": folder,
                "tvdb_id": tvdb_id,
                "tmdb_id": tmdb_id,
                "title": clean_title,
                "vn_title": vn_title,
                "year": year,
                "type": media_type,
                "poster": poster_url,
                "total_episodes": max(total_eps, len(episodes_dict)),
                "download": {
                    "state": dl_state,
                    "label": dl_label,
                    "color": dl_color,
                    "downloaded": downloaded_video_count,
                    "total": total_eps
                },
                "sync": {
                    "state": sync_state,
                    "label": sync_label,
                    "color": sync_color,
                    "in_nas": in_nas,
                    "in_gdrive": in_gdrive,
                    "in_local": in_local
                },
                "subtitle": {
                    "state": sub_state,
                    "label": sub_label,
                    "color": sub_color,
                    "completed": vietsub_count,
                    "total": total_eps,
                    "percent": sub_percent
                },
                "has_glossary": os.path.exists(os.path.join(primary_local_path, "glossary.json")) if primary_local_path else False,
                "has_progress": os.path.exists(os.path.join(primary_local_path, "PROGRESS.md")) if primary_local_path else False,
                "local_path": primary_local_path,
                "seasons": seasons_list
            })

        # Summary KPIs
        total_items = len(collections)
        total_series = sum(1 for c in collections if c["type"] == "series")
        total_movies = sum(1 for c in collections if c["type"] == "movie")
        synced_both = sum(1 for c in collections if c["sync"]["state"] == "synced_both")
        sub_complete = sum(1 for c in collections if c["subtitle"]["state"] == "complete")

        return {
            "summary": {
                "total_items": total_items,
                "total_series": total_series,
                "total_movies": total_movies,
                "synced_both": synced_both,
                "sub_complete": sub_complete
            },
            "collections": collections
        }


# Singleton export
collection_mgr = MediaCollectionManager()
