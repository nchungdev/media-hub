#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity Media Hub & Command Center HTTP Server
Serves Web UI on port 8888 and provides REST APIs for live monitoring and command dispatch.
"""

import os
import re
import sys
import glob
import math
import hmac
import secrets
import threading
import json
import time
import shutil
import urllib.parse
import urllib.request
import shlex
import subprocess
from pathlib import Path
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler

# Import Core Modules
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, BASE_DIR)

from core.torbox_manager import TorBoxManager
from core.monitor import PipelineMonitor
from core.gdrive_manager import GDriveManager
from core.agent_bridge import AgentBridge
from core.settings import load_unified_settings
from core.artwork import get_poster_bytes
from core.library_builder import LibraryBuilder
from core.job_store import JobStore
from core.sync_worker import SyncWorker
from core.tunnel import tunnel_mgr

# Initialize Core Managers
torbox_mgr = TorBoxManager()
pipeline_mon = PipelineMonitor()
gdrive_mgr = GDriveManager()
agent_bridge = AgentBridge()

# ---- module-level caches ----
_torbox_api_cache = None
_torbox_api_cache_time = 0
_last_overview_cache = {}
_last_overview_time = 0

# Asset roots. Requests under /static/ are resolved against these and are rejected if
# the resolved path escapes the root, so a "../" in the URL cannot read arbitrary files.
APP_DIR = os.path.dirname(BASE_DIR)
SKILL_DIR = APP_DIR
STATIC_ROOTS = [
    os.path.join(APP_DIR, "static"),
    os.path.join(BASE_DIR, "static"),
]
TEMPLATE_ROOTS = [
    os.path.join(APP_DIR, "templates"),
    os.path.join(BASE_DIR, "templates"),
]

def sibling_skill_script(plugin_name, script_name):
    """Locate a script in a sibling skill. Searches workspace .agents/skills,
    global plugins, and MEDIA_HUB_SKILLS_PATH."""
    rel = os.path.join("scripts", script_name)
    patterns = []

    for extra in os.environ.get("MEDIA_HUB_SKILLS_PATH", "").split(os.pathsep):
        extra = extra.strip()
        if extra:
            patterns.append(os.path.join(extra, plugin_name, rel))
            patterns.append(os.path.join(extra, plugin_name, "skills", plugin_name, rel))

    from core.settings import load_unified_settings
    try:
        cfg = load_unified_settings()
        hub_home = cfg.get("media_hub_home") or os.getcwd()
        ws_dir = str(Path(hub_home).parent) if os.path.basename(hub_home) == ".media-hub" else str(Path(hub_home))
        patterns.append(os.path.join(ws_dir, ".agents", "skills", plugin_name, rel))
        patterns.append(os.path.join(ws_dir, "skills", plugin_name, rel))
    except Exception:
        pass

    # realpath too
    roots = []
    for skill_dir in (SKILL_DIR, os.path.realpath(SKILL_DIR)):
        if skill_dir not in roots:
            roots.append(skill_dir)

    for skill_dir in roots:
        skills_root = os.path.dirname(skill_dir)          # .../skills
        plugin_root = os.path.dirname(skills_root)        # .../<plugin> or .../<version>
        patterns.append(os.path.join(skills_root, plugin_name, rel))
        patterns.append(os.path.join(os.path.dirname(plugin_root), plugin_name, "skills", plugin_name, rel))
        patterns.append(os.path.join(
            os.path.dirname(os.path.dirname(plugin_root)), plugin_name, "*", "skills", plugin_name, rel))

    for pattern in patterns:
        for candidate in sorted(glob.glob(pattern), reverse=True):
            if os.path.isfile(candidate):
                return candidate
    return None


# Set by launcher.py when a public tunnel is opened; empty means local/LAN mode.
AUTH_TOKEN = os.environ.get("MEDIA_HUB_TOKEN", "").strip()

CONFIG_FILE = os.path.join(BASE_DIR, "config.json")
config = {"port": 8888, "host": "0.0.0.0"}
if os.path.exists(CONFIG_FILE):
    try:
        with open(CONFIG_FILE, "r", encoding="utf-8") as f:
            config.update(json.load(f))
    except Exception:
        pass

# launcher.py passes --port through the environment; config.json is the fallback.
PORT = int(os.environ.get("MEDIA_HUB_PORT") or config.get("port", 8888))
HOST = str(config.get("host", "0.0.0.0"))

# ==================== LIBRARY PRESENCE (measured, not assumed) ====================
_nas_folder_cache = {"data": [], "at": 0.0}
_NAS_FOLDER_TTL = 300


def _clean_titles(names):
    """Strip year/tvdb/bracket noise so folder names can be matched against release names."""
    out = []
    for raw in names or []:
        c = re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", str(raw)).strip().lower()
        if c:
            out.append(c)
    return out


def _title_matches(release_name, titles):
    for t in titles:
        if len(t) >= 4 and (t in release_name
                            or any(w in release_name for w in t.split() if len(w) > 4)):
            return True
    return False


def list_nas_folders():
    """Directory names under the configured NAS library path, cached for 5 minutes."""
    now = time.time()
    if _nas_folder_cache["data"] and now - _nas_folder_cache["at"] < _NAS_FOLDER_TTL:
        return _nas_folder_cache["data"]

    cfg = load_unified_settings()
    host, user, nas_path = cfg.get("nas_host", ""), cfg.get("nas_user", ""), cfg.get("nas_path", "")
    if not host or not nas_path:
        return _nas_folder_cache["data"]

    key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
    ssh_cmd = ["ssh", "-p", str(int(cfg.get("nas_port", 22))), "-o", "BatchMode=yes",
               "-o", "ConnectTimeout=4", "-o", "StrictHostKeyChecking=no"]
    if key and os.path.exists(key):
        ssh_cmd += ["-i", key]
    ssh_cmd += [f"{user}@{host}", f"ls -1 {shlex.quote(nas_path)}"]
    try:
        res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=8)
        if res.returncode != 0:
            return _nas_folder_cache["data"]  # keep the last good listing
        folders = [l.strip() for l in res.stdout.splitlines() if l.strip()]
        _nas_folder_cache.update({"data": folders, "at": now})
        return folders
    except Exception:
        return _nas_folder_cache["data"]


def list_local_staging():
    cfg = load_unified_settings()
    staging = cfg.get("staging_dir", "")
    try:
        return [d for d in os.listdir(staging) if os.path.isdir(os.path.join(staging, d))]
    except Exception:
        return []


# ==================== LIVE SYSTEM OVERVIEW ====================
# Everything here is measured. When a probe fails the field reports "Không đo được"
# rather than a plausible-looking constant, so the dashboard never shows numbers that
# were never true.

OVERVIEW_TTL = 20  # seconds; the NAS/Drive probes are network calls


def _fmt_bytes(n):
    if n is None:
        return "Không rõ"
    n = float(n)
    for unit in ("B", "KB", "MB", "GB", "TB", "PB"):
        if abs(n) < 1024.0:
            return f"{n:.1f} {unit}"
        n /= 1024.0
    return f"{n:.1f} EB"


def _probe_local_disk(staging_dir):
    """Usage of the volume the staging buffer actually lives on."""
    probe = staging_dir if os.path.exists(staging_dir) else os.path.dirname(staging_dir) or "/"
    if not os.path.exists(probe):
        probe = "/"
    try:
        vfs = os.statvfs(probe)
        total = vfs.f_blocks * vfs.f_frsize
        avail = vfs.f_bavail * vfs.f_frsize
        used = total - (vfs.f_bfree * vfs.f_frsize)
        return {
            "name": f"Ổ đệm ({probe})",
            "path": staging_dir,
            "total_gb": round(total / 1024**3, 1),
            "used_gb": round(used / 1024**3, 1),
            "free_gb": round(avail / 1024**3, 1),
            # used/(used+avail), the same convention df(1) prints, so the dashboard and
            # the shell agree. used/total counts reserved blocks and reads low.
            "percent": math.ceil(used / (used + avail) * 100) if (used + avail) else 0,
            "measured": True,
        }
    except Exception as e:
        return {"name": "Ổ đệm", "path": staging_dir, "total_gb": None, "used_gb": None,
                "free_gb": None, "percent": 0, "measured": False, "error": str(e)}


def _probe_memory():
    """Real memory pressure on macOS via vm_stat; falls back to marking itself unmeasured."""
    total = None
    try:
        res = subprocess.run(["sysctl", "-n", "hw.memsize"], capture_output=True, text=True, timeout=2)
        total = int(res.stdout.strip())
    except Exception:
        pass

    used = None
    try:
        vm = subprocess.run(["vm_stat"], capture_output=True, text=True, timeout=2)
        if vm.returncode == 0:
            page_size = 4096
            m = re.search(r"page size of (\d+) bytes", vm.stdout)
            if m:
                page_size = int(m.group(1))
            pages = {}
            for line in vm.stdout.splitlines():
                pm = re.match(r'"?([A-Za-z ][^:"]*)"?:\s+(\d+)', line.strip())
                if pm:
                    pages[pm.group(1).strip().lower()] = int(pm.group(2))
            # "Used" the way Activity Monitor reports it: resident, non-reclaimable pages.
            used_pages = (
                pages.get("pages active", 0)
                + pages.get("pages wired down", 0)
                + pages.get("pages occupied by compressor", 0)
            )
            if used_pages:
                used = used_pages * page_size
    except Exception:
        pass

    if total and used:
        return {"ram_total_gb": round(total / 1024**3, 1),
                "ram_used_gb": round(used / 1024**3, 1),
                "ram_pct": int(used / total * 100),
                "measured": True}
    return {"ram_total_gb": round(total / 1024**3, 1) if total else None,
            "ram_used_gb": None, "ram_pct": 0, "measured": False}


def _probe_nas(cfg):
    """df on the NAS over SSH. Returns None when the host is unreachable."""
    host, user = cfg.get("nas_host", ""), cfg.get("nas_user", "")
    nas_path = cfg.get("nas_path", "")
    if not host or not nas_path:
        return None
    key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
    ssh_cmd = ["ssh", "-p", str(int(cfg.get("nas_port", 22))), "-o", "BatchMode=yes",
               "-o", "ConnectTimeout=4", "-o", "StrictHostKeyChecking=no"]
    if key and os.path.exists(key):
        ssh_cmd += ["-i", key]
    ssh_cmd += [f"{user}@{host}", f"df -Pk {shlex.quote(nas_path)}"]
    try:
        res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=8)
        if res.returncode != 0:
            return None
        rows = [l.split() for l in res.stdout.strip().splitlines()[1:] if l.split()]
        if not rows:
            return None
        parts = rows[-1]
        total, used, avail = int(parts[1]) * 1024, int(parts[2]) * 1024, int(parts[3]) * 1024
        return {"total": total, "used": used, "avail": avail,
                "percent": math.ceil(used / (used + avail) * 100) if (used + avail) else 0}
    except Exception:
        return None


def _probe_gdrive(cfg):
    """`rclone about` for the configured remote. None when rclone/the remote is unavailable."""
    remote = cfg.get("gdrive_remote", "gdrive")
    try:
        res = subprocess.run(
            [gdrive_mgr.rclone_bin, "--config", gdrive_mgr.rclone_config,
             "about", f"{remote}:", "--json"],
            capture_output=True, text=True, timeout=10,
        )
        if res.returncode != 0:
            return None
        data = json.loads(res.stdout)
        total, used = data.get("total"), data.get("used")
        return {"total": total, "used": used, "free": data.get("free"),
                "percent": math.ceil(used / total * 100) if (total and used) else 0}
    except Exception:
        return None


def _jobs_to_transfers():
    """Turn live worker jobs into the download/upload cards the dashboard renders."""
    downloads, uploads = [], []
    for j in job_store.list_active():
        pct = round(j.get("progress") or 0, 1)
        speed = j.get("speed_bps") or 0
        done, total = j.get("bytes_done") or 0, j.get("bytes_total") or 0
        eta = "—"
        if speed > 0 and total > done:
            secs = int((total - done) / speed)
            eta = f"{secs // 60}m{secs % 60:02d}s" if secs >= 60 else f"{secs}s"

        if j["phase"] in ("pending", "link", "download"):
            downloads.append({
                "job_id": j["id"],
                "name": j["name"] or f"Torrent #{j['torrent_id']}",
                "engine": "TorBox Cloud DDL",
                "dest_path": j["staging_path"] or "",
                "progress": pct,
                "speed": f"{_fmt_bytes(speed)}/s" if speed else "—",
                "eta": eta,
                "message": j["message"],
            })
        else:
            targets = j.get("targets") or []
            label = " + ".join("Google Drive" if t == "drive" else "NAS Storage" for t in targets)
            uploads.append({
                "job_id": j["id"],
                "title": j["name"] or f"Torrent #{j['torrent_id']}",
                "dest": label or "Google Drive",
                "dest_short": " + ".join("☁️ gdrive" if t == "drive" else "🖥️ NAS" for t in targets),
                "progress": pct,
                "current_ep": len(j.get("done_targets") or []),
                "total_ep": len(targets) or 1,
                "message": j["message"],
            })
    return downloads, uploads


def _recent_from_jobs(limit=8):
    """Recently finished transfers, from the job history rather than a fixed list."""
    out = []
    for j in job_store.list_recent(limit=40):
        if j["status"] != "done":
            continue
        finished = j.get("finished_at") or j.get("updated_at") or 0
        age = time.time() - finished
        when = ("Vừa xong" if age < 300 else
                f"{int(age // 60)} phút trước" if age < 3600 else
                f"{int(age // 3600)} giờ trước" if age < 86400 else
                f"{int(age // 86400)} ngày trước")
        dests = j.get("done_targets") or j.get("targets") or []
        out.append({
            "id": j["torrent_id"],
            "title": j["name"] or f"Torrent #{j['torrent_id']}",
            "vn": j["name"] or "",
            "year": "",
            "qual": "",
            "episodes": _fmt_bytes(j.get("bytes_total") or 0),
            "sub": "",
            "dest": " & ".join("Google Drive" if d == "drive" else "NAS Storage" for d in dests),
            "time": when,
        })
        if len(out) >= limit:
            break
    return out


def get_cached_overview_data():
    global _last_overview_cache, _last_overview_time
    now = time.time()
    if _last_overview_cache and (now - _last_overview_time < OVERVIEW_TTL):
        return _last_overview_cache

    cfg = load_unified_settings()
    staging_dir = cfg.get("staging_dir", "")

    try:
        load1 = round(os.getloadavg()[0], 2)
    except Exception:
        load1 = None

    # Network probes run in parallel so the endpoint stays responsive.
    import concurrent.futures
    with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex:
        f_nas = ex.submit(_probe_nas, cfg)
        f_gd = ex.submit(_probe_gdrive, cfg)
        nas, gd = f_nas.result(), f_gd.result()

    mem = _probe_memory()
    downloads, uploads = _jobs_to_transfers()

    clouds = [{
        "id": "gdrive",
        "icon": "☁️",
        "name": f"Google Drive ({cfg.get('gdrive_remote', 'gdrive')}:)",
        "path": f"{cfg.get('gdrive_remote', 'gdrive')}:{cfg.get('gdrive_root', 'Phim')}",
        "connected": gd is not None,
        "used_str": _fmt_bytes(gd["used"]) if gd else "Không đo được",
        "avail_str": (_fmt_bytes(gd["free"]) if gd and gd.get("free") else "Không giới hạn") if gd else "Không kết nối",
        "total_str": (_fmt_bytes(gd["total"]) if gd and gd.get("total") else "Unlimited") if gd else "—",
        "percent": gd["percent"] if gd else 0,
        "badge": "Plex Main Cloud",
    }, {
        "id": "nas",
        "icon": "🖥️",
        "name": "NAS Storage",
        "path": cfg.get("nas_path", ""),
        "connected": nas is not None,
        "used_str": _fmt_bytes(nas["used"]) if nas else "Không đo được",
        "avail_str": f"{_fmt_bytes(nas['avail'])} trống" if nas else "Không kết nối",
        "total_str": _fmt_bytes(nas["total"]) if nas else "—",
        "percent": nas["percent"] if nas else 0,
        "badge": "Mạng Nội Bộ",
    }]

    result = {
        "success": True,
        "measured_at": now,
        "health": {
            "cpu_load": load1 if load1 is not None else "—",
            "ram_total_gb": mem["ram_total_gb"] if mem["ram_total_gb"] else "—",
            "ram_used_gb": mem["ram_used_gb"] if mem["measured"] else "—",
            "ram_pct": mem["ram_pct"],
            "local_disk": _probe_local_disk(staging_dir),
        },
        "clouds": clouds,
        "active_downloads": downloads,
        "active_uploads": uploads,
        "recent_media": _recent_from_jobs(),
        "job_counts": job_store.counts(),
    }
    _last_overview_cache = result
    _last_overview_time = now
    return result



# ==================== JOB STORE & SYNC WORKER ====================
# The dashboard used to only record sync *intent* in a JSON file that nothing read.
# Jobs now live in SQLite and are executed for real by SyncWorker.
job_store = JobStore()
library_builder = LibraryBuilder(gdrive_mgr, load_unified_settings, list_nas_folders)

sync_worker = SyncWorker(
    job_store,
    torbox_mgr,
    gdrive_mgr,
    load_unified_settings,
    concurrency=int(load_unified_settings().get("max_concurrent_downloads", 2)),
)


class MediaHubHandler(BaseHTTPRequestHandler):
    # ---- access control ----
    # Empty token (the default for localhost/LAN use) leaves the dashboard wide open as
    # before. The launcher sets MEDIA_HUB_TOKEN whenever it opens a public tunnel, which
    # turns this on so the whole API is not exposed to the internet unauthenticated.
    def _authorized(self):
        if not AUTH_TOKEN:
            return True
        supplied = self.headers.get("X-Media-Hub-Token", "")
        if not supplied:
            cookies = self.headers.get("Cookie", "")
            for part in cookies.split(";"):
                name, _, value = part.strip().partition("=")
                if name == "mh_token":
                    supplied = value
                    break
        if not supplied:
            supplied = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query).get("k", [""])[0]
        return hmac.compare_digest(supplied, AUTH_TOKEN)

    def _reject_unauthorized(self):
        body = (
            b"<h1>401 - Media Hub</h1>"
            b"<p>Dashboard dang chay o che do cong khai. Hay mo link kem token: "
            b"<code>?k=&lt;token&gt;</code> (token duoc in ra khi khoi chay).</p>"
        )
        self.send_response(401)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _maybe_set_token_cookie(self):
        """When the token arrives as ?k=..., persist it so later requests carry it."""
        if not AUTH_TOKEN:
            return
        q = urllib.parse.parse_qs(urllib.parse.urlparse(self.path).query).get("k", [""])[0]
        if q and hmac.compare_digest(q, AUTH_TOKEN):
            self.send_header("Set-Cookie", f"mh_token={AUTH_TOKEN}; Path=/; HttpOnly; SameSite=Lax")

    def _send_json(self, data, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()
        self.wfile.write(json.dumps(data, ensure_ascii=False).encode("utf-8"))

    def _send_html(self, html_content, status=200):
        self.send_response(status)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self._maybe_set_token_cookie()
        self.end_headers()
        self.wfile.write(html_content.encode("utf-8"))

    def do_HEAD(self):
        self.do_GET()

    def do_OPTIONS(self):
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        self.send_header("Access-Control-Allow-Headers", "Content-Type")
        self.end_headers()

    def do_GET(self):
        if not self._authorized():
            return self._reject_unauthorized()
        parsed_url = urllib.parse.urlparse(self.path)
        path = parsed_url.path

        # 1. Web UI Routes (SPA Routing for URL paths)
        UI_ROUTES = {
            "/", "/index.html", "/home", "/overview",
            "/torbox", "/torrents", "/downloader",
            "/gdrive", "/library", "/plex",
            "/pipelines", "/sync",
            "/subtitles", "/subtitle-studio",
            "/tokens", "/token-usage", "/analytics",
            "/console", "/logs", "/terminal",
            "/settings", "/config",
            "/agent", "/chat"
        }
        if path in UI_ROUTES:
            for tp in (os.path.join(root, "index.html") for root in TEMPLATE_ROOTS):
                if os.path.exists(tp):
                    with open(tp, "r", encoding="utf-8") as f:
                        return self._send_html(f.read())
            return self._send_html("<h1>Antigravity Media Hub</h1><p>Template missing.</p>", status=404)

        # 1.1 Static Assets Routing (/static/...)
        elif path.startswith("/static/"):
            # BaseHTTPRequestHandler does NOT normalise the request path, so "/static/../.."
            # used to escape the asset directory and serve any file on disk (~/.env,
            # ~/.ssh/id_ed25519). Resolve the path and require it to stay inside a root.
            file_rel = urllib.parse.unquote(path[len("/static/"):])
            file_path = None
            for root in STATIC_ROOTS:
                try:
                    candidate = (Path(root) / file_rel).resolve()
                    if candidate.is_file() and candidate.is_relative_to(Path(root).resolve()):
                        file_path = str(candidate)
                        break
                except (OSError, ValueError):
                    continue

            if file_path:
                content_type = "image/jpeg"
                if file_path.endswith(".png"): content_type = "image/png"
                elif file_path.endswith(".jpg") or file_path.endswith(".jpeg"): content_type = "image/jpeg"
                elif file_path.endswith(".svg"): content_type = "image/svg+xml"
                elif file_path.endswith(".css"): content_type = "text/css"
                elif file_path.endswith(".js"): content_type = "application/javascript"
                with open(file_path, "rb") as f:
                    data = f.read()
                self.send_response(200)
                self.send_header("Content-Type", content_type)
                self.send_header("Content-Length", str(len(data)))
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Cache-Control", "public, max-age=86400")
                self.end_headers()
                self.wfile.write(data)
                return
            return self.send_error(404, "Static file not found")

        # 1.2 Poster artwork: fetched from TMDb on demand and cached outside the repo,
        #     replacing the 34 image files that used to be committed into the skill.
        elif path == "/api/poster":
            qp = urllib.parse.parse_qs(parsed_url.query)
            data, ctype = get_poster_bytes(
                tvdb_id=qp.get("tvdb", [""])[0].strip() or None,
                tmdb_id=qp.get("tmdb", [""])[0].strip() or None,
                title=qp.get("title", [""])[0].strip() or None,
            )
            self.send_response(200)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Access-Control-Allow-Origin", "*")
            # Placeholders must not stick around once a key is configured.
            self.send_header("Cache-Control",
                             "public, max-age=604800" if ctype == "image/jpeg" else "public, max-age=300")
            self.end_headers()
            self.wfile.write(data)
            return

                                        # 2. REST API: TorBox List with 3-Way Storage Tracking (Local, GDrive, NAS)
        elif path in ["/api/torbox", "/api/torbox/clear_cache"]:
            global _torbox_api_cache, _torbox_api_cache_time
            now = time.time()
            bypass_cache = (path == "/api/torbox/clear_cache") or ("refresh=true" in self.path) or ("bypass_cache=true" in self.path) or ("clear_cache=true" in self.path)
            if not bypass_cache and _torbox_api_cache and (now - _torbox_api_cache_time < 3):
                return self._send_json(_torbox_api_cache)

            # Re-read config if key missing
            if not torbox_mgr.api_key:
                cfg = load_unified_settings()
                tok = cfg.get("torbox_token") or cfg.get("api_key")
                if tok:
                    torbox_mgr.api_key = str(tok).strip()

            res = torbox_mgr.list_torrents()
            if res.get("success") and "data" in res and isinstance(res["data"], list):
                # One query for every active job, instead of one per torrent in the loop.
                active_jobs = job_store.active_by_torrent_map()
                try:
                    gdrive_raw = [s.get("name") or s.get("folder") or "" for s in gdrive_mgr.list_tv_shows()]
                except Exception:
                    gdrive_raw = []
                clean_gdrive_titles = _clean_titles(gdrive_raw)
                # Previously a hard-coded list of seven show names decided the "on NAS"
                # badge. Ask the NAS what it actually holds (cached, with the local
                # staging buffer listed too so "Đã Về Máy" can ever be true).
                clean_nas_titles = _clean_titles(list_nas_folders())
                clean_local_titles = _clean_titles(list_local_staging())

                for t in res["data"]:
                    name = t.get("name", "").lower()
                    locations = []
                    for bucket, titles in (("gdrive", clean_gdrive_titles),
                                           ("nas", clean_nas_titles),
                                           ("local", clean_local_titles)):
                        if _title_matches(name, titles):
                            locations.append(bucket)

                    t["locations"] = locations
                    t["synced_destinations"] = [loc for loc in locations if loc in ["gdrive", "nas"]]
                    t["is_completed_and_synced"] = len(t["synced_destinations"]) > 0
                    t["is_on_local"] = "local" in locations
                    t["is_on_gdrive"] = "gdrive" in locations
                    t["is_on_nas"] = "nas" in locations
                    
                    # Attach the live job for this torrent, from the single map fetched above.
                    sync_job = active_jobs.get(str(t.get("id")))
                    if sync_job:
                        t["sync_in_progress"] = {
                            "job_id": sync_job["id"],
                            "status": "syncing" if sync_job["status"] == "running" else sync_job["status"],
                            "phase": sync_job["phase"],
                            "progress": sync_job["progress"],
                            "message": sync_job["message"],
                            "targets": sync_job["targets"],
                            "done_targets": sync_job["done_targets"],
                            "target": sync_job["targets"][0] if sync_job["targets"] else "drive",
                        }
                    else:
                        t["sync_in_progress"] = None

            # Only cache successful responses, so a transient TorBox error is not
            # served back to the UI for the next 3 seconds.
            if res.get("success"):
                _torbox_api_cache = res
                _torbox_api_cache_time = now
            return self._send_json(res)

        # 2.1 REST API: TorBox Download Link
        elif path == "/api/torbox/download_link":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            t_id = query_params.get("id", [""])[0]
            if not t_id:
                return self._send_json({"success": False, "error": "Missing id parameter"})
            res = torbox_mgr.request_download_link(t_id)
            return self._send_json(res)

                # 3. REST API: Live Pipelines Status with Dynamic Active Sync Tasks
        elif path == "/api/pipelines":
            monster = pipeline_mon.get_monster_status()
            multi = pipeline_mon.get_multi_show_status()
            active_sync_jobs = job_store.list_active()
            lib_ver = f"M:{monster.get('completed_eps',0)}_MS:{multi.get('current_show','')}:{multi.get('completed_eps',0)}_{gdrive_mgr.get_cache_version()}"
            return self._send_json({
                "monster": monster,
                "multi_show": multi,
                "active_syncs": active_sync_jobs,
                "recent_syncs": job_store.list_recent(limit=20),
                "job_counts": job_store.counts(),
                "library_version": lib_ver
            })

        # 3.04 REST API: Library index stats (shows/files/assets, from SQLite)
        elif path == "/api/library/stats":
            return self._send_json({
                "success": True,
                "drive": gdrive_mgr.stats(),
                "missing_assets": gdrive_mgr.missing_assets(),
            })

        # 3.05 REST API: Library metadata build progress
        elif path == "/api/library/build/status":
            return self._send_json(library_builder.status())

        # 3.1 REST API: Download / Sync Job Queue
        elif path == "/api/download/jobs":
            return self._send_json({
                "success": True,
                "active": job_store.list_active(),
                "recent": job_store.list_recent(limit=50),
                "counts": job_store.counts(),
            })

        # 4. REST API: GDrive Shows List
        elif path == "/api/gdrive/shows":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            refresh = query_params.get("refresh", ["0"])[0].lower() in ["1", "true"]
            shows = gdrive_mgr.list_tv_shows(force_refresh=refresh)
            return self._send_json({"shows": shows})

        # 4.1 REST API: GDrive Season Files
        elif path == "/api/gdrive/season_files":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            show = query_params.get("show", [""])[0]
            season = query_params.get("season", [""])[0]
            refresh = query_params.get("refresh", ["0"])[0].lower() in ["1", "true"]
            if not show or not season:
                return self._send_json({"files": []})
            files = gdrive_mgr.get_season_files(show, season, force_refresh=refresh)
            return self._send_json({"files": files})

        # 4.15 REST API: Generate M3U Playlist file for VLC / IINA
        elif path == "/api/playlist.m3u":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            show = query_params.get("show", [""])[0]
            season = query_params.get("season", [""])[0]
            filename = query_params.get("file", [""])[0]
            host = self.headers.get("Host", f"{HOST}:{PORT}")
            proto = "https" if ("trycloudflare" in host or self.headers.get("X-Forwarded-Proto") == "https") else "http"
            
            stream_url = f"{proto}://{host}/api/stream?show={urllib.parse.quote(show)}&season={urllib.parse.quote(season)}&file={urllib.parse.quote(filename)}"
            m3u_content = f"#EXTM3U\n#EXTINF:-1,{filename}\n{stream_url}\n"
            
            self.send_response(200)
            self.send_header("Content-Type", "audio/x-mpegurl; charset=utf-8")
            self.send_header("Content-Disposition", f'attachment; filename="{filename}.m3u"')
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(m3u_content.encode("utf-8"))
            return

        # 4.2 Streaming & Direct Download (/api/stream, /api/download)
        elif path == "/api/stream" or path == "/api/download":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            show = query_params.get("show", [""])[0]
            season = query_params.get("season", [""])[0]
            filename = query_params.get("file", [""])[0]

            if not show or not season or not filename:
                return self.send_error(400, "Missing show, season, or file parameter")

            gdrive_path = f"gdrive:Phim/TV Shows/{show}/{season}/{filename}"
            is_download = (path == "/api/download")
            
            content_type = "video/mp4"
            if filename.endswith(".mkv"): content_type = "video/x-matroska"
            elif filename.endswith(".avi"): content_type = "video/x-msvideo"
            elif filename.endswith(".ass") or filename.endswith(".srt"): content_type = "text/plain; charset=utf-8"

            buffer_mb = 32
            try: buffer_mb = max(4, min(512, int(query_params.get("buffer_mb", ["32"])[0])))
            except Exception: pass

            chunk_size = 256 * 1024
            try:
                chunk_kb = int(query_params.get("chunk_kb", ["256"])[0])
                chunk_size = max(64, min(4096, chunk_kb)) * 1024
            except Exception: pass

            # If web streaming an MKV file, remux/transcode to fragmented MP4 for 100% HTML5 browser compatibility
            if not is_download and (filename.endswith(".mkv") or filename.endswith(".avi")):
                self.send_response(200)
                self.send_header("Content-Type", "video/mp4")
                self.send_header("Accept-Ranges", "bytes")
                self.send_header("Access-Control-Allow-Origin", "*")
                self.send_header("Content-Disposition", f'inline; filename="{filename}.mp4"')
                self.end_headers()

                rclone_proc = subprocess.Popen([
                    gdrive_mgr.rclone_bin,
                    "--config", gdrive_mgr.rclone_config,
                    f"--buffer-size={buffer_mb}M",
                    "cat", gdrive_path
                ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

                ffmpeg_bin = "/opt/homebrew/bin/ffmpeg" if os.path.exists("/opt/homebrew/bin/ffmpeg") else "ffmpeg"
                ffmpeg_cmd = [
                    ffmpeg_bin,
                    "-loglevel", "error",
                    "-i", "pipe:0",
                    "-c:v", "libx264",
                    "-preset", "ultrafast",
                    "-tune", "zerolatency",
                    "-pix_fmt", "yuv420p",
                    "-crf", "22",
                    "-c:a", "aac",
                    "-b:a", "192k",
                    "-movflags", "frag_keyframe+empty_moov+default_base_moof",
                    "-f", "mp4",
                    "pipe:1"
                ]

                ff_proc = subprocess.Popen(ffmpeg_cmd, stdin=rclone_proc.stdout, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
                rclone_proc.stdout.close()
                
                try:
                    while True:
                        chunk = ff_proc.stdout.read(chunk_size)
                        if not chunk:
                            break
                        self.wfile.write(chunk)
                except Exception:
                    pass
                finally:
                    ff_proc.terminate()
                    rclone_proc.terminate()
                    try: ff_proc.wait(timeout=1)
                    except Exception: ff_proc.kill()
                    try: rclone_proc.wait(timeout=1)
                    except Exception: rclone_proc.kill()
                return

            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Accept-Ranges", "bytes")
            self.send_header("Access-Control-Allow-Origin", "*")
            if is_download:
                self.send_header("Content-Disposition", f'attachment; filename="{filename}"')
            else:
                self.send_header("Content-Disposition", f'inline; filename="{filename}"')
            self.end_headers()

            rclone_cmd = [
                gdrive_mgr.rclone_bin,
                "--config", gdrive_mgr.rclone_config,
                f"--buffer-size={buffer_mb}M",
                "cat", gdrive_path
            ]
            
            proc = subprocess.Popen(rclone_cmd, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
            try:
                while True:
                    chunk = proc.stdout.read(chunk_size)
                    if not chunk:
                        break
                    self.wfile.write(chunk)
            except Exception:
                pass
            finally:
                proc.terminate()
                try: proc.wait(timeout=1)
                except Exception: proc.kill()
            return

        # 4.3 REST API: Subtitles List (/api/subtitles)
        elif path == "/api/subtitles":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            show = query_params.get("show", [""])[0]
            season = query_params.get("season", [""])[0]
            filename = query_params.get("file", [""])[0]

            if not show or not season or not filename:
                return self._send_json({"subtitles": []})

            all_files = gdrive_mgr.get_season_files(show, season)
            base_name = os.path.splitext(filename)[0]
            
            subtitles = []
            
            # 1. Look for External / Standalone Subtitles (.ass, .srt, .vtt)
            for f in all_files:
                if (f.endswith(".ass") or f.endswith(".srt") or f.endswith(".vtt")) and (f.startswith(base_name) or base_name.startswith(os.path.splitext(f)[0])):
                    lang = "Phụ đề"
                    f_lower = f.lower()
                    if ".vi." in f_lower or "vietsub" in f_lower or ".vie." in f_lower or ".vn." in f_lower:
                        lang = "🇻🇳 Tiếng Việt"
                    elif ".en." in f_lower or ".eng." in f_lower:
                        lang = "🇬🇧 Tiếng Anh (English)"
                    elif ".ja." in f_lower or ".jpn." in f_lower or ".jap." in f_lower:
                        lang = "🇯🇵 Tiếng Nhật (Japanese)"
                    elif ".zh." in f_lower or ".chi." in f_lower:
                        lang = "🇨🇳 Tiếng Trung"
                    else:
                        lang = f"📄 {os.path.splitext(f)[1].upper().replace('.', '')}"

                    subtitles.append({
                        "type": "external",
                        "label": f"{lang} (File rời)",
                        "file": f,
                        "url": f"/api/subtitle/vtt?show={urllib.parse.quote(show)}&season={urllib.parse.quote(season)}&sub={urllib.parse.quote(f)}"
                    })

            # 2. Add Muxed Subtitle option for MKV files
            if filename.endswith(".mkv"):
                subtitles.append({
                    "type": "muxed",
                    "label": "📦 Phụ đề tích hợp sẵn (Muxed Track 1)",
                    "track_index": 0,
                    "url": f"/api/subtitle/vtt?show={urllib.parse.quote(show)}&season={urllib.parse.quote(season)}&file={urllib.parse.quote(filename)}&muxed=1&track=0"
                })

            return self._send_json({"subtitles": subtitles})

        # 4.4 REST API: Serve Subtitle as WebVTT (/api/subtitle/vtt)
        elif path == "/api/subtitle/vtt":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            show = query_params.get("show", [""])[0]
            season = query_params.get("season", [""])[0]
            sub_file = query_params.get("sub", [""])[0]
            is_muxed = query_params.get("muxed", ["0"])[0] == "1"
            video_file = query_params.get("file", [""])[0]
            track_id = query_params.get("track", ["0"])[0]

            self.send_response(200)
            self.send_header("Content-Type", "text/vtt; charset=utf-8")
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()

            ffmpeg_bin = "/opt/homebrew/bin/ffmpeg" if os.path.exists("/opt/homebrew/bin/ffmpeg") else "ffmpeg"

            if is_muxed and video_file:
                # Extract muxed subtitle track from MKV
                gdrive_path = f"gdrive:Phim/TV Shows/{show}/{season}/{video_file}"
                rclone_proc = subprocess.Popen([
                    gdrive_mgr.rclone_bin, "--config", gdrive_mgr.rclone_config, "cat", gdrive_path
                ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

                ff_proc = subprocess.Popen([
                    ffmpeg_bin, "-loglevel", "error", "-i", "pipe:0", "-map", f"0:s:{track_id}", "-f", "webvtt", "pipe:1"
                ], stdin=rclone_proc.stdout, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
                rclone_proc.stdout.close()
                
                try:
                    out, _ = ff_proc.communicate(timeout=10)
                    self.wfile.write(out if out else b"WEBVTT\n\n")
                except Exception:
                    self.wfile.write(b"WEBVTT\n\n")
                finally:
                    ff_proc.kill()
                    rclone_proc.kill()
                return
            elif sub_file:
                # Convert external ASS/SRT file to WebVTT
                gdrive_path = f"gdrive:Phim/TV Shows/{show}/{season}/{sub_file}"
                rclone_proc = subprocess.Popen([
                    gdrive_mgr.rclone_bin, "--config", gdrive_mgr.rclone_config, "cat", gdrive_path
                ], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

                ff_proc = subprocess.Popen([
                    ffmpeg_bin, "-loglevel", "error", "-i", "pipe:0", "-f", "webvtt", "pipe:1"
                ], stdin=rclone_proc.stdout, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
                rclone_proc.stdout.close()
                
                try:
                    out, _ = ff_proc.communicate(timeout=10)
                    self.wfile.write(out if out else b"WEBVTT\n\n")
                except Exception:
                    self.wfile.write(b"WEBVTT\n\n")
                finally:
                    ff_proc.kill()
                    rclone_proc.kill()
                return
            else:
                self.wfile.write(b"WEBVTT\n\n")
                return

        # 4.5 REST API: Download Subtitle File (/api/subtitles/download)
        elif path == "/api/subtitles/download":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            file_path = query_params.get("path", [""])[0]
            if not file_path or not os.path.exists(file_path):
                return self._send_json({"error": "File not found"}, status=404)
            
            filename = os.path.basename(file_path)
            content_type = "text/plain; charset=utf-8"
            if file_path.endswith(".ass"):
                content_type = "text/x-ssa; charset=utf-8"
            elif file_path.endswith(".srt"):
                content_type = "application/x-subrip; charset=utf-8"
            elif file_path.endswith(".vtt"):
                content_type = "text/vtt; charset=utf-8"
            
            with open(file_path, "rb") as f:
                data = f.read()
            
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Disposition", f'attachment; filename="{urllib.parse.quote(filename)}"')
            self.send_header("Content-Length", str(len(data)))
            self.send_header("Access-Control-Allow-Origin", "*")
            self.end_headers()
            self.wfile.write(data)
            return

        # 5. REST API: Agent Command Queue
        elif path == "/api/agent/queue":
            queue = agent_bridge.list_commands()
            return self._send_json(queue)

        # 5.1 REST API: Token Usage & Analytics Report (/api/agent/token_usage)
        elif path == "/api/agent/token_usage":
            report = agent_bridge.get_token_usage_report()
            return self._send_json(report)

        # 5.2 REST API: Live CLI Console Logs (/api/agent/live_logs)
        elif path == "/api/agent/live_logs":
            logs_data = agent_bridge.get_live_logs()
            return self._send_json(logs_data)

        # 10. REST API: Live Dashboard Overview & Machine Health (/api/dashboard/overview)
        elif path == "/api/dashboard/overview":
            data = get_cached_overview_data()
            return self._send_json(data)
        # 6. REST API: Media Hub Settings (/api/settings)
        elif path == "/api/settings":
            cfg = load_unified_settings()
            return self._send_json(cfg)
        # 11. REST API: Cloudflare Quick Tunnel Status (/api/tunnel/status)
        elif path == "/api/tunnel/status":
            return self._send_json(tunnel_mgr.get_status())

        # 9. REST API: Cross-Storage Scan & Compare (GDrive vs NAS vs Local)
        elif path == "/api/library/cross_check":
            cfg = load_unified_settings()
            key = os.path.expanduser(cfg.get("nas_ssh_key", "~/.ssh/id_ed25519"))
            user = cfg.get("nas_user", "chungnh")
            host = cfg.get("nas_host", "192.168.1.37")
            nas_base = cfg.get("nas_path", "/srv/mergerfs/MainPool/Phim/TV Shows").rstrip("/")
            if "/volume1/" in nas_base:
                nas_base = "/srv/mergerfs/MainPool/Phim/TV Shows"
            staging_dir = cfg.get("staging_dir") or os.path.join(cfg.get("media_hub_home", os.getcwd()), ".staging")
            
            # 1. Get GDrive Shows
            gdrive_shows_list = gdrive_mgr.list_tv_shows()
            gdrive_shows = {item["name"]: True for item in gdrive_shows_list if isinstance(item, dict) and "name" in item}
            
            # 2. Get NAS Directory listings via SSH
            nas_folders = {}
            try:
                ssh_cmd = ["ssh", "-p", "22", "-o", "BatchMode=yes", "-o", "ConnectTimeout=3", "-o", "StrictHostKeyChecking=no"]
                if os.path.exists(key):
                    ssh_cmd += ["-i", key]
                # nas_base comes from settings, which any client can POST to /api/settings.
                # shlex.quote stops a quote in the path from closing the string and
                # appending arbitrary commands to the remote shell.
                q_base = shlex.quote(nas_base)
                ssh_cmd += [f"{user}@{host}", f'if [ -d {q_base} ]; then ls -1 {q_base}; fi']
                res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=5)
                if res.returncode == 0:
                    for line in res.stdout.splitlines():
                        line = line.strip()
                        if line:
                            nas_folders[line] = True
            except Exception as e:
                print("NAS scan error:", e)

            # 3. Check Local Staging Directory
            local_folders = {}
            if os.path.exists(staging_dir):
                try:
                    for item in os.listdir(staging_dir):
                        if os.path.isdir(os.path.join(staging_dir, item)):
                            local_folders[item] = True
                except Exception:
                    pass

            # 4. Synthesize Comparisons & Smart Proposals
            comparisons = []
            
            # Map of known show definitions for rich metadata
            known_meta = {
                "72281": {"title": "Black Jack (1993)", "vn": "Bác Sĩ Quái Dị Black Jack (OVA)", "qual": "1080p BDRip", "episodes": 12, "vietsub": True},
                "81092": {"title": "Black Jack (2004)", "vn": "Bác Sĩ Quái Dị Black Jack (TV Series)", "qual": "1080p BDRip", "episodes": 89, "vietsub": True},
                "79354": {"title": "The File of Young Kindaichi (1997)", "vn": "Thám Tử Kindaichi (Anime 1997)", "qual": "480p DVD", "episodes": 148, "vietsub": True},
                "279782": {"title": "The File of Young Kindaichi Returns (2014)", "vn": "Thám Tử Kindaichi Returns", "qual": "1080p BDRip", "episodes": 47, "vietsub": True},
                "79460": {"title": "The Files of the Young Kindaichi (1995)", "vn": "Thám Tử Kindaichi (Live Action)", "qual": "1080p BDRip", "episodes": 13, "vietsub": True},
                "227501": {"title": "Mashin Hero Wataru (1988)", "vn": "Thần Long Đấu Sĩ Wataru", "qual": "1080p BDRip", "episodes": 150, "vietsub": True},
                "74599": {"title": "Monster (2004)", "vn": "Quái Vật Monster", "qual": "1080p BluRay", "episodes": 74, "vietsub": True},
                "75939": {"title": "Battle B-Daman (2004)", "vn": "Chiến Binh B-Daman", "qual": "1080p / 480p", "episodes": 103, "vietsub": True},
                "79178": {"title": "Transformers - Car Robots (2000)", "vn": "Transformers: Car Robots", "qual": "480p DVD", "episodes": 39, "vietsub": False},
                "454526": {"title": "WUKONG: Đại Viên Hồn (2025)", "vn": "Tây Hành Kỷ: Đại Viên Hồn", "qual": "1080p WEB-DL", "episodes": 12, "vietsub": True},
                "350711": {"title": "The Westward (2018)", "vn": "Tây Hành Kỷ", "qual": "1080p WEB-DL", "episodes": 21, "vietsub": True},
                "259259": {"title": "Kingdom (2012)", "vn": "Vương Giả Thiên Hạ", "qual": "1080p BDRip", "episodes": 150, "vietsub": True},
                "80674": {"title": "Furuhata Ninzaburo (1994)", "vn": "Thám Tử Cổ Điển Furuhata", "qual": "480p DVD", "episodes": 44, "vietsub": True},
                "320122": {"title": "The Three-Eyed One (1990)", "vn": "Cậu Bé 3 Mắt (Mitsume ga Tooru)", "qual": "480p DVD", "episodes": 48, "vietsub": True},
                "230211": {"title": "Tantei Gakuen Q (2003)", "vn": "Học Viện Thám Tử Q", "qual": "480p DVD", "episodes": 45, "vietsub": True},
                "335191": {"title": "Hakyuu Houshin Engi (2018)", "vn": "Bá Khí Phong Thần Diễn Nghĩa", "qual": "1080p BDRip", "episodes": 24, "vietsub": True},
                "79284": {"title": "Houshin Engi (1999)", "vn": "Phong Thần Bảng (1999)", "qual": "480p DVD", "episodes": 26, "vietsub": True},
                "299770": {"title": "Young Black Jack (2015)", "vn": "Bác Sĩ Black Jack Thời Trẻ", "qual": "1080p BDRip", "episodes": 12, "vietsub": True},
                "252384": {"title": "Young Black Jack (2015)", "vn": "Bác Sĩ Black Jack Thời Trẻ", "qual": "1080p BDRip", "episodes": 12, "vietsub": True}
            }

            all_folder_keys = set(gdrive_shows.keys()) | set(nas_folders.keys())
            
            for folder in sorted(all_folder_keys):
                in_gdrive = folder in gdrive_shows
                in_nas = folder in nas_folders
                in_local = folder in local_folders
                
                # Extract ID or title
                m = re.search(r"\{tvdb-(\d+)\}", folder)
                tvdb_id = m.group(1) if m else ""
                meta = known_meta.get(tvdb_id, {
                    "title": folder.split("{")[0].strip(),
                    "vn": folder.split("{")[0].strip(),
                    "qual": "1080p / 480p",
                    "episodes": 0,
                    "vietsub": True
                })

                # Determine Smart Proposals
                proposals = []
                if in_gdrive and not in_nas:
                    proposals.append({
                        "action": "sync_to_nas",
                        "label": "☁️ ➔ 🖥️ Đồng bộ sang NAS",
                        "desc": "Phim đã có trên Google Drive, đẩy sang NAS Storage qua SSH rclone",
                        "color": "amber"
                    })
                elif in_nas and not in_gdrive:
                    proposals.append({
                        "action": "sync_to_drive",
                        "label": "🖥️ ➔ ☁️ Sao lưu lên Drive",
                        "desc": "Phim đã có trên NAS, sao lưu lên Google Drive Plex",
                        "color": "emerald"
                    })
                
                if not meta.get("vietsub", True):
                    proposals.append({
                        "action": "translate_vietsub",
                        "label": "🇻🇳 Dịch Phụ Đề Vietsub",
                        "desc": "Chưa có phụ đề tiếng Việt chuẩn, kích hoạt AI dịch tự động",
                        "color": "purple"
                    })

                if in_gdrive and in_nas and meta.get("vietsub", True):
                    proposals.append({
                        "action": "perfect",
                        "label": "✓ Đã Đồng Bộ Hoàn Hảo",
                        "desc": "Đã có đầy đủ trên Google Drive & NAS kèm phụ đề Vietsub",
                        "color": "blue"
                    })

                comparisons.append({
                    "folder": folder,
                    "tvdb_id": tvdb_id,
                    "title": meta.get("title"),
                    "vn": meta.get("vn"),
                    "qual": meta.get("qual"),
                    "poster": f"/api/poster?tvdb={tvdb_id}" if tvdb_id else f"/api/poster?title={urllib.parse.quote(folder)}",
                    "in_gdrive": in_gdrive,
                    "in_nas": in_nas,
                    "in_local": in_local,
                    "proposals": proposals
                })

            summary = {
                "total_shows": len(comparisons),
                "synced_both": sum(1 for c in comparisons if c["in_gdrive"] and c["in_nas"]),
                "only_gdrive": sum(1 for c in comparisons if c["in_gdrive"] and not c["in_nas"]),
                "only_nas": sum(1 for c in comparisons if not c["in_gdrive"] and c["in_nas"]),
                "need_sub": sum(1 for c in comparisons if any(p["action"] == "translate_vietsub" for p in c["proposals"]))
            }

            return self._send_json({"success": True, "summary": summary, "shows": comparisons})


        # 8. REST API: Service Connection Health Checks (/api/services/status)
        elif path == "/api/services/status":
            import concurrent.futures
            cfg = load_unified_settings()
            
            def check_gdrive():
                remote = cfg.get("gdrive_remote", "gdrive").strip()
                try:
                    res = subprocess.run([
                        gdrive_mgr.rclone_bin, "--config", gdrive_mgr.rclone_config, "listremotes"
                    ], capture_output=True, text=True, timeout=3)
                    if res.returncode == 0 and f"{remote}:" in res.stdout:
                        return {"connected": True, "detail": f"Remote '{remote}:' Sẵn sàng kết nối"}
                    return {"connected": False, "detail": f"Không tìm thấy remote '{remote}:'"}
                except Exception as e:
                    return {"connected": False, "detail": str(e)}

            def check_nas():
                host = cfg.get("nas_host", "192.168.1.37")
                user = cfg.get("nas_user", "chungnh")
                port = int(cfg.get("nas_port", 22))
                key = os.path.expanduser(cfg.get("nas_ssh_key", "~/.ssh/id_ed25519"))
                try:
                    ssh_cmd = ["ssh", "-p", str(port), "-o", "BatchMode=yes", "-o", "ConnectTimeout=3", "-o", "StrictHostKeyChecking=no"]
                    if os.path.exists(key):
                        ssh_cmd += ["-i", key]
                    ssh_cmd += [f"{user}@{host}", "echo OK"]
                    res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=4)
                    return {"connected": res.returncode == 0, "detail": f"SSH {user}@{host}:{port} Đang kết nối" if res.returncode == 0 else (res.stderr.strip() or "SSH Timeout")}
                except Exception as e:
                    return {"connected": False, "detail": str(e)}

            def check_torbox():
                try:
                    res = torbox_mgr.list_torrents()
                    return {"connected": res.get("success", False), "detail": "TorBox Cloud API Online" if res.get("success") else (res.get("error") or "Không thể xác thực Token")}
                except Exception as e:
                    return {"connected": False, "detail": str(e)}

            def check_tmdb():
                tmdb_key = cfg.get("tmdb_api_key")
                if not tmdb_key:
                    return {"connected": False, "detail": "Chưa điền API Key"}
                try:
                    req = urllib.request.Request(f"https://api.themoviedb.org/3/configuration?api_key={tmdb_key}")
                    with urllib.request.urlopen(req, timeout=3) as resp:
                        return {"connected": resp.status == 200, "detail": "TMDb API v3 Online"}
                except Exception as e:
                    return {"connected": False, "detail": str(e)}

            def check_aria2():
                aria_host = cfg.get("aria2_rpc_host", "127.0.0.1")
                aria_port = int(cfg.get("aria2_rpc_port", 6800))
                secret = cfg.get("aria2_rpc_secret", "").strip()
                try:
                    params = [f"token:{secret}"] if secret else []
                    payload = json.dumps({
                        "jsonrpc": "2.0",
                        "id": "health",
                        "method": "aria2.getVersion",
                        "params": params
                    }).encode("utf-8")
                    req = urllib.request.Request(
                        f"http://{aria_host}:{aria_port}/jsonrpc",
                        data=payload,
                        headers={"Content-Type": "application/json"}
                    )
                    with urllib.request.urlopen(req, timeout=2) as resp:
                        data = json.loads(resp.read().decode("utf-8"))
                        ver = data.get("result", {}).get("version", "")
                        return {"connected": True, "detail": f"Aria2c RPC v{ver} Sẵn sàng"}
                except Exception:
                    return {"connected": False, "detail": f"Aria2 RPC ({aria_host}:{aria_port}) Offline"}

            def check_ytdlp():
                ytdlp_bin = cfg.get("ytdlp_bin", "/opt/homebrew/bin/yt-dlp")
                if not os.path.exists(ytdlp_bin):
                    ytdlp_bin = "yt-dlp"
                try:
                    res = subprocess.run([ytdlp_bin, "--version"], capture_output=True, text=True, timeout=2)
                    if res.returncode == 0:
                        ver = res.stdout.strip()
                        return {"connected": True, "detail": f"yt-dlp v{ver} Sẵn sàng"}
                    return {"connected": False, "detail": "Chưa cài đặt yt-dlp"}
                except Exception as e:
                    return {"connected": False, "detail": str(e)}

            def check_direct():
                return {"connected": True, "detail": "Multi-stream HTTP/DDL Engine Sẵn sàng"}

            with concurrent.futures.ThreadPoolExecutor(max_workers=7) as executor:
                f_gdrive = executor.submit(check_gdrive)
                f_nas = executor.submit(check_nas)
                f_torbox = executor.submit(check_torbox)
                f_tmdb = executor.submit(check_tmdb)
                f_aria2 = executor.submit(check_aria2)
                f_ytdlp = executor.submit(check_ytdlp)
                f_direct = executor.submit(check_direct)

                results = {
                    "gdrive": f_gdrive.result(),
                    "nas": f_nas.result(),
                    "torbox": f_torbox.result(),
                    "tmdb": f_tmdb.result(),
                    "aria2": f_aria2.result(),
                    "ytdlp": f_ytdlp.result(),
                    "direct": f_direct.result()
                }

            return self._send_json({"success": True, "services": results})

        # 7. REST API: TMDb Live Search (/api/tmdb/search)
        elif path == "/api/tmdb/search":
            query_params = urllib.parse.parse_qs(parsed_url.query)
            q = query_params.get("query", [""])[0].strip()
            if not q:
                return self._send_json({"results": []})
            
            cfg = load_unified_settings()
            api_key = cfg.get("tmdb_api_key") or os.environ.get("TMDB_API_KEY")
            
            if not api_key:
                # Return helpful fallback / simulated result if no key
                return self._send_json({
                    "results": [],
                    "warning": "Vui lòng nhập TMDb API Key trong tab Cài Đặt để kích hoạt tra cứu trực tiếp!"
                })
            
            try:
                tmdb_url = f"https://api.themoviedb.org/3/search/multi?query={urllib.parse.quote(q)}&language=vi-VN&api_key={api_key}"
                req = urllib.request.Request(tmdb_url, headers={"User-Agent": "Antigravity-Media-Hub/1.0"})
                with urllib.request.urlopen(req, timeout=10) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
                    results = data.get("results", [])
                    return self._send_json({"results": results})
            except Exception as e:
                return self._send_json({"results": [], "error": str(e)})

        # 8. REST API: Subtitles & Staging Media Scan (/api/subtitles/staging)
        elif path == "/api/subtitles/staging":
            cfg = load_unified_settings()
            staging = cfg.get("staging_dir") or os.path.join(cfg.get("media_hub_home", os.getcwd()), ".staging")
            files_list = []
            if os.path.exists(staging):
                for root, _, files in os.walk(staging):
                    for f in files:
                        if f.lower().endswith((".mkv", ".mp4", ".m4v", ".srt", ".ass", ".ssa", ".vtt")):
                            full_p = os.path.join(root, f)
                            rel_p = os.path.relpath(full_p, staging)
                            size_mb = round(os.path.getsize(full_p) / (1024 * 1024), 2)
                            ext = os.path.splitext(f)[1].lower()
                            files_list.append({
                                "filename": f,
                                "rel_path": rel_p,
                                "full_path": full_p,
                                "size_mb": size_mb,
                                "type": "video" if ext in [".mkv", ".mp4", ".m4v"] else "subtitle",
                                "ext": ext
                            })
            return self._send_json({"staging_dir": staging, "files": files_list})

        # 8.1 REST API: Subtitle Translation Projects & Progress (/api/subtitles/projects)
        elif path == "/api/subtitles/projects":
            cfg = load_unified_settings()
            hub_home = cfg.get("media_hub_home") or os.path.join(os.getcwd(), ".media-hub")
            staging = cfg.get("staging_dir") or os.path.join(hub_home, ".staging")
            projects = []
            
            if os.path.exists(hub_home):
                for item in os.listdir(hub_home):
                    if item.startswith("."):
                        continue
                    item_path = os.path.join(hub_home, item)
                    if not os.path.isdir(item_path):
                        continue
                    
                    tv_dir = os.path.join(item_path, "TV Shows")
                    if os.path.exists(tv_dir):
                        for show in os.listdir(tv_dir):
                            sp = os.path.join(tv_dir, show)
                            if os.path.isdir(sp) and not show.startswith("."):
                                meta_file = os.path.join(sp, "metadata.json")
                                prog_file = os.path.join(sp, "PROGRESS.md")
                                gloss_file = os.path.join(sp, "glossary.json")
                                
                                meta = {}
                                if os.path.exists(meta_file):
                                    try:
                                        with open(meta_file, "r", encoding="utf-8") as f:
                                            meta = json.load(f)
                                    except Exception:
                                        pass
                                
                                episodes = {}
                                for root, dirs, files in os.walk(sp):
                                    for f in files:
                                        if f.startswith("."):
                                            continue
                                        m = re.search(r"S(\d+)E(\d+)", f, re.IGNORECASE)
                                        if m:
                                            s_num = int(m.group(1))
                                            e_num = int(m.group(2))
                                            ep_key = f"S{s_num:02d}E{e_num:02d}"
                                            if ep_key not in episodes:
                                                episodes[ep_key] = {"key": ep_key, "season_num": s_num, "ep_num": e_num, "video": False, "vi_ass": False, "vi_srt": False, "vi_vtt": False, "eng_sub": False}
                                            if f.endswith((".mkv", ".mp4", ".m4v", ".avi")):
                                                episodes[ep_key]["video"] = True
                                            elif f.endswith(".vi.ass"):
                                                episodes[ep_key]["vi_ass"] = True
                                                episodes[ep_key]["vi_ass_path"] = os.path.join(root, f)
                                                episodes[ep_key]["vi_ass_name"] = f
                                            elif f.endswith(".vi.srt"):
                                                episodes[ep_key]["vi_srt"] = True
                                                episodes[ep_key]["vi_srt_path"] = os.path.join(root, f)
                                                episodes[ep_key]["vi_srt_name"] = f
                                            elif f.endswith(".vi.vtt"):
                                                episodes[ep_key]["vi_vtt"] = True
                                                episodes[ep_key]["vi_vtt_path"] = os.path.join(root, f)
                                                episodes[ep_key]["vi_vtt_name"] = f
                                            elif f.endswith((".eng.ass", ".eng.srt")):
                                                episodes[ep_key]["eng_sub"] = True

                                staging_show = os.path.join(staging, f"{item}_2004") if os.path.exists(staging) else None
                                if staging_show and os.path.exists(staging_show):
                                    for root, dirs, files in os.walk(staging_show):
                                        for f in files:
                                            m = re.search(r"S(\d+)E(\d+)", f, re.IGNORECASE)
                                            if m:
                                                s_num = int(m.group(1))
                                                e_num = int(m.group(2))
                                                ep_key = f"S{s_num:02d}E{e_num:02d}"
                                                if ep_key not in episodes:
                                                    episodes[ep_key] = {"key": ep_key, "season_num": s_num, "ep_num": e_num, "video": False, "vi_ass": False, "vi_srt": False, "vi_vtt": False, "eng_sub": False}
                                                if f.endswith((".eng.ass", ".eng.srt", ".ass", ".srt")):
                                                    episodes[ep_key]["eng_sub"] = True

                                total_eps = max(len(episodes), meta.get("total_episodes", len(episodes)))
                                completed = sum(1 for ep in episodes.values() if (ep["vi_ass"] or ep["vi_srt"] or ep["vi_vtt"]))
                                
                                if total_eps > 0:
                                    projects.append({
                                        "name": show,
                                        "title": meta.get("title", show),
                                        "tmdb_id": meta.get("tmdb_id"),
                                        "tvdb_id": meta.get("tvdb_id"),
                                        "total_episodes": total_eps,
                                        "completed_episodes": completed,
                                        "percent": round((completed / total_eps * 100) if total_eps > 0 else 0, 1),
                                        "has_glossary": os.path.exists(gloss_file),
                                        "has_progress": os.path.exists(prog_file),
                                        "path": sp,
                                        "episodes": [episodes[k] for k in sorted(episodes.keys())]
                                    })
            projects.sort(key=lambda p: (1 if p["percent"] >= 100 else 0, -p["percent"], p["title"].lower()))
            return self._send_json({"projects": projects})

        else:
            self.send_error(404, "Not Found")

    def do_POST(self):
        if not self._authorized():
            return self._reject_unauthorized()
        parsed_url = urllib.parse.urlparse(self.path)
        path = parsed_url.path
        
        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length).decode("utf-8") if content_length > 0 else "{}"
        
        try:
            req_data = json.loads(body)
        except Exception:
            req_data = {}

        # 1. API: Clear TorBox Cache
        if path == "/api/torbox/clear_cache":
            global _torbox_api_cache, _torbox_api_cache_time
            _torbox_api_cache = None
            _torbox_api_cache_time = 0
            if not torbox_mgr.api_key:
                cfg = load_unified_settings()
                tok = cfg.get("torbox_token") or cfg.get("api_key")
                if tok:
                    torbox_mgr.api_key = str(tok).strip()
            res = torbox_mgr.list_torrents()
            return self._send_json({"success": True, "message": "Đã xóa cache Quản lý tải xuống", "data": res.get("data", []), "counts": len(res.get("data", []))})

        # 1.1 API: Add Magnet
        elif path == "/api/torbox/add":
            magnet = req_data.get("magnet")
            if not magnet:
                return self._send_json({"success": False, "error": "Missing magnet link"}, status=400)
            res = torbox_mgr.add_magnet(magnet)
            return self._send_json(res)

        # 2. API: Delete Torrent
        elif path == "/api/torbox/delete":
            torrent_id = req_data.get("id")
            if not torrent_id:
                return self._send_json({"success": False, "error": "Missing torrent ID"}, status=400)
            res = torbox_mgr.delete_torrent(torrent_id)
            return self._send_json(res)

        # 2.1 API: Control Queued Download (Start / Delete)
        elif path == "/api/torbox/control_queued":
            queued_id = req_data.get("id")
            op = req_data.get("operation", "start")
            if not queued_id:
                return self._send_json({"success": False, "error": "Missing queued ID"}, status=400)
            res = torbox_mgr.control_queued(queued_id, operation=op)
            return self._send_json(res)

        # 2.2 API: Queue a real TorBox ➔ staging ➔ Drive/NAS job (de-duplicated per torrent)
        elif path == "/api/download/sync":
            ids = req_data.get("ids", [])
            names = req_data.get("names", [])
            # Accept both a single "target" and a "targets" list; normalise the legacy
            # "gdrive" alias the UI used to send for the Google Drive button.
            raw_targets = req_data.get("targets") or [req_data.get("target", "drive")]
            alias = {"gdrive": "drive", "google_drive": "drive"}
            targets, seen = [], set()
            for t in raw_targets:
                t = alias.get(str(t).strip().lower(), str(t).strip().lower())
                if t in ("drive", "nas") and t not in seen:
                    seen.add(t)
                    targets.append(t)

            if not ids:
                return self._send_json({"success": False, "error": "Chưa chọn mục để đồng bộ"}, status=400)
            if not targets:
                return self._send_json({"success": False, "error": "Đích đồng bộ không hợp lệ"}, status=400)

            results = []
            for idx, tid in enumerate(ids):
                tname = names[idx] if idx < len(names) else f"Torrent #{tid}"
                results.append(job_store.enqueue(tid, targets, tname))

            target_label = " & ".join("Google Drive" if t == "drive" else "NAS Storage" for t in targets)
            queued = sum(1 for r in results if r["is_new_download"])
            merged = len(results) - queued
            return self._send_json({
                "success": True,
                "message": (
                    f"🚀 Đã xếp {queued} tác vụ tải mới lên {target_label}"
                    + (f", {merged} mục gộp vào tiến trình đang chạy (chỉ tải 1 lần từ TorBox)" if merged else "")
                    + "!"
                ),
                "details": results,
            })

        # 2.25 API: Build library metadata (poster/fanart/tvshow.nfo) for folders
        #      that are missing it. Manual, from the library screen.
        elif path == "/api/library/build":
            raw = req_data.get("targets") or ["drive"]
            alias = {"gdrive": "drive", "google_drive": "drive"}
            targets = [alias.get(str(t).lower(), str(t).lower()) for t in raw]
            targets = [t for t in targets if t in ("drive", "nas")] or ["drive"]
            return self._send_json(library_builder.start(
                targets=targets, only_missing=bool(req_data.get("only_missing", True))))

        elif path == "/api/library/refresh":
            changed = gdrive_mgr.refresh(force=True)
            return self._send_json({
                "success": True,
                "refreshed": changed,
                "stats": gdrive_mgr.stats(),
                "message": "Đã lập chỉ mục lại thư viện Google Drive." if changed
                           else "Một tiến trình lập chỉ mục khác đang chạy.",
            })

        elif path == "/api/library/build/cancel":
            library_builder.cancel()
            return self._send_json({"success": True, "message": "Đã yêu cầu dừng tiến trình dựng metadata."})

        # 2.3 API: Cancel a queued or running sync job
        elif path == "/api/download/cancel":
            job_id = req_data.get("job_id")
            if not job_id:
                return self._send_json({"success": False, "error": "Thiếu job_id"}, status=400)
            ok = job_store.request_cancel(int(job_id))
            return self._send_json({
                "success": ok,
                "message": "Đã gửi yêu cầu hủy tác vụ." if ok else "Tác vụ đã kết thúc, không thể hủy.",
            })

        elif path == "/api/library/cross_check":
            return self.do_GET()

        # 2.3 API: Start / Stop Aria2 Daemon
        elif path == "/api/aria2/control":
            op = req_data.get("operation", "start")
            aria2_bin = "/opt/homebrew/bin/aria2c" if os.path.exists("/opt/homebrew/bin/aria2c") else "aria2c"
            
            if op == "start":
                try:
                    subprocess.Popen([
                        aria2_bin, "--enable-rpc", "--rpc-listen-all=false", "--rpc-allow-origin-all", "-D"
                    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    time.sleep(0.5)
                    return self._send_json({"success": True, "message": "Đã khởi động Aria2c RPC Daemon thành công!"})
                except Exception as e:
                    return self._send_json({"success": False, "error": f"Không thể khởi động Aria2c: {e}"})
            elif op == "stop":
                try:
                    subprocess.run(["pkill", "-f", "aria2c --enable-rpc"], capture_output=True)
                    return self._send_json({"success": True, "message": "Đã dừng Aria2c Daemon."})
                except Exception as e:
                    return self._send_json({"success": False, "error": f"Lỗi khi dừng Aria2c: {e}"})

                # 3. API: Send Agent Command
        elif path == "/api/agent/command":
            cmd = req_data.get("command") or req_data.get("cmd") or req_data.get("text") or req_data.get("message") or ""
            if not str(cmd).strip():
                # Provide a smart fallback rather than returning error 400
                torrent_id = req_data.get("torrent_id")
                target = req_data.get("target")
                if torrent_id:
                    cmd = f"Đồng bộ torrent ID #{torrent_id} lên {target or 'kho lưu trữ'}"
                else:
                    return self._send_json({"success": False, "error": "Vui lòng nhập nội dung lệnh"}, status=400)
            
            media_id = req_data.get("media_id") or req_data.get("mediaId") or req_data.get("tvdb_id") or None
            item = agent_bridge.add_command(str(cmd).strip(), author="MediaHub UI", media_id=media_id)
            return self._send_json({"success": True, "command": item})

        # 3.1 API: Reset / Clear Media Session Context (/api/agent/session/reset)
        elif path == "/api/agent/session/reset":
            media_id = req_data.get("media_id") or req_data.get("mediaId") or ""
            if not media_id:
                return self._send_json({"success": False, "error": "Thiếu media_id"}, status=400)
            agent_bridge.clear_media_session(media_id)
            return self._send_json({"success": True, "message": f"Đã xóa session cache cho {media_id}"})

        # 3.2 API: Clear Live CLI Logs (/api/agent/live_logs/clear)
        elif path == "/api/agent/live_logs/clear":
            agent_bridge.clear_live_logs()
            return self._send_json({"success": True, "message": "Đã xoá log console."})

        # 3.3 API: Stop Current CLI Process (/api/agent/stop)
        elif path == "/api/agent/stop":
            agent_bridge.stop_current_job()
            return self._send_json({"success": True, "message": "Đã dừng tiến trình CLI."})

        # 3.4 API: Resume Queue Processing (/api/agent/resume)
        elif path == "/api/agent/resume":
            agent_bridge.resume_queue()
            return self._send_json({"success": True, "message": "Đã kích hoạt lại hàng đợi CLI."})


        # 3.5 API: Native Directory Picker (/api/fs/choose_directory)
        elif path == "/api/fs/choose_directory":
            try:
                # Use macOS native choose folder dialog
                script = 'POSIX path of (choose folder with prompt "Chọn Thư mục Làm việc Media Hub:")'
                res = subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=120)
                if res.returncode == 0:
                    chosen = res.stdout.strip().rstrip("/")
                    if chosen:
                        return self._send_json({"success": True, "path": chosen})
                return self._send_json({"success": False, "cancelled": True})
            except Exception as e:
                return self._send_json({"success": False, "error": str(e)}, status=500)

        # 3.6 API: Set Active Workspace Directory (/api/workspace/set)
        elif path == "/api/workspace/set":
            ws_path = (req_data.get("workspace_dir") or req_data.get("path") or "").strip()
            if not ws_path:
                return self._send_json({"success": False, "error": "Thiếu đường dẫn thư mục làm việc"}, status=400)
            
            ws_path = os.path.expanduser(ws_path)
            if not os.path.exists(ws_path):
                try:
                    os.makedirs(ws_path, exist_ok=True)
                except Exception as e:
                    return self._send_json({"success": False, "error": f"Không thể tạo thư mục: {e}"}, status=400)

            from core.settings import load_unified_settings, save_unified_settings, resolve_dirs
            cfg = load_unified_settings()
            cfg["workspace_dir"] = ws_path
            cfg["media_hub_home"] = os.path.join(ws_path, ".media-hub") if os.path.basename(ws_path) != ".media-hub" else ws_path
            resolve_dirs(cfg, create=True)
            save_unified_settings(cfg)
            return self._send_json({
                "success": True,
                "message": f"Đã thiết lập thư mục làm việc: {ws_path}",
                "workspace_dir": ws_path,
                "media_hub_home": cfg["media_hub_home"]
            })

        # 4. API: Save Media Hub Settings (/api/settings)
        elif path == "/api/settings":
            from core.settings import load_unified_settings, save_unified_settings, resolve_dirs
            try:
                cfg = load_unified_settings()
                cfg.update(req_data)
                resolve_dirs(cfg, create=True)
                save_unified_settings(cfg)
                return self._send_json({"success": True, "message": "Đã lưu cài đặt thành công!", "settings": cfg})
            except Exception as e:
                return self._send_json({"success": False, "error": str(e)}, status=500)

        # 4b. API: Start/Stop Cloudflare Tunnel
        elif path == "/api/tunnel/start":
            port = int(req_data.get("port") or PORT or 8888)
            res = tunnel_mgr.start(port=port)
            return self._send_json(res, status=200 if res.get("success") else 500)

        elif path == "/api/tunnel/stop":
            res = tunnel_mgr.stop()
            return self._send_json(res)

        # 5. API: Scan NAS Plex Directories (/api/nas/scan)
        elif path == "/api/nas/scan":
            host = req_data.get("host", "").strip()
            user = req_data.get("user", "admin").strip()
            port = int(req_data.get("port", 22))
            key = req_data.get("key", "").strip()
            custom_path = req_data.get("path", "").strip()
            
            if not host:
                return self._send_json({"success": False, "error": "Thiếu địa chỉ IP NAS"}, status=400)
            
            ssh_cmd = ["ssh", "-p", str(port), "-o", "BatchMode=yes", "-o", "ConnectTimeout=5", "-o", "StrictHostKeyChecking=no"]
            if key:
                expanded_key = os.path.expanduser(key)
                if os.path.exists(expanded_key):
                    ssh_cmd += ["-i", expanded_key]
            else:
                for k in ["id_ed25519", "id_rsa"]:
                    cand = Path.home() / ".ssh" / k
                    if cand.is_file():
                        ssh_cmd += ["-i", str(cand)]
                        break
            
            ssh_cmd.append(f"{user}@{host}")
            
            candidate_paths = [
                custom_path,
                "/srv/mergerfs/MainPool/Phim/TV Shows",
                "/srv/mergerfs/MainPool/Phim/Movies",
                "/srv/mergerfs/MainPool/Phim",
                "/volume1/video/TV Shows",
                "/volume1/video/Movies",
                "/volume1/Media",
                "/volume1/Plex",
                "/share/CACHEDEV1_DATA/Multimedia/TV Shows",
                "/share/Multimedia/Plex",
                "/srv/media"
            ]
            seen = set()
            paths_to_check = []
            for p in candidate_paths:
                if p and p not in seen:
                    seen.add(p)
                    paths_to_check.append(p)
            
            # custom_path is attacker-controlled request data; quote it before it reaches
            # the remote shell.
            remote_cmds = [
                f'if [ -d {shlex.quote(p)} ]; then printf "FOUND:%s\\n" {shlex.quote(p)}; fi'
                for p in paths_to_check
            ]
            remote_script = "; ".join(remote_cmds)
            
            try:
                res = subprocess.run(ssh_cmd + [remote_script], capture_output=True, text=True, timeout=8)
                if res.returncode != 0 and not res.stdout.strip():
                    err_msg = res.stderr.strip() or "SSH connection failed"
                    return self._send_json({"success": False, "error": err_msg})
                found = [line.split(":", 1)[1].strip() for line in res.stdout.splitlines() if line.startswith("FOUND:")]
                return self._send_json({"success": True, "libraries": found})
            except Exception as e:
                return self._send_json({"success": False, "error": f"Không thể kết nối SSH tới NAS: {e}"})

        # 6. API: Check Google Drive Connection (/api/gdrive/check)
        elif path == "/api/gdrive/check":
            remote = req_data.get("remote", "gdrive").strip()
            root = req_data.get("root", "Phim/TV Shows").strip()
            cmd = [gdrive_mgr.rclone_bin, "--config", gdrive_mgr.rclone_config, "lsd", f"{remote}:{root.lstrip('/')}"]
            try:
                res = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
                if res.returncode == 0:
                    dirs = [l.split()[-1] for l in res.stdout.strip().splitlines() if l.strip()]
                    return self._send_json({
                        "success": True, 
                        "message": f"Kết nối tới {remote}:{root} thành công! (Tìm thấy {len(dirs)} thư mục TV Shows)",
                        "dirs": dirs[:10]
                    })
                else:
                    return self._send_json({"success": False, "error": res.stderr.strip() or "Lỗi kết nối Rclone"})
            except Exception as e:
                return self._send_json({"success": False, "error": str(e)})

        # 7. API: Collector Magnet / Source Inspect (/api/collector/inspect)
        elif path == "/api/collector/inspect":
            magnet = req_data.get("magnet", "").strip()
            query = req_data.get("query", "").strip()
            if not magnet and not query:
                return self._send_json({"success": False, "error": "Vui lòng nhập Magnet link hoặc từ khóa tìm kiếm"}, status=400)
            
            # Parse display name from magnet if provided
            parsed_name = query or "Media Release"
            xt_hash = ""
            if magnet.startswith("magnet:?"):
                params = urllib.parse.parse_qs(magnet.replace("magnet:?", ""))
                dn = params.get("dn", [""])[0]
                if dn:
                    parsed_name = urllib.parse.unquote(dn)
                xt = params.get("xt", [""])[0]
                if xt:
                    xt_hash = xt.replace("urn:btih:", "")

            return self._send_json({
                "success": True,
                "title": parsed_name,
                "hash": xt_hash,
                "magnet": magnet,
                "message": "Đã phân tích thông tin nguồn tải thành công!"
            })

        # 8. API: Extract Subtitles from Video (/api/subtitles/extract)
        elif path == "/api/subtitles/extract":
            filepath = req_data.get("file", "").strip()
            if not filepath or not os.path.exists(filepath):
                return self._send_json({"success": False, "error": "File video không tồn tại"}, status=400)

            ext_script = sibling_skill_script("subtitle-extractor", "extract_subtitles.py")
            if not ext_script:
                return self._send_json({"success": False, "error": "Không tìm thấy skill subtitle-extractor. Cài plugin subtitle-extractor, hoặc trỏ MEDIA_HUB_SKILLS_PATH tới thư mục chứa nó."}, status=500)
            try:
                # The CLI is subcommand-based: passing the path alone made argparse exit
                # with code 2 while this endpoint still reported success.
                res = subprocess.run(
                    [sys.executable, ext_script, "extract", filepath],
                    capture_output=True, text=True, timeout=120,
                )
                out = (res.stdout or "").strip() or (res.stderr or "").strip()
                if res.returncode != 0:
                    return self._send_json({"success": False, "error": out or f"Thoát với mã {res.returncode}"})
                return self._send_json({"success": True, "message": "Bóc tách phụ đề thành công!", "output": out})
            except Exception as e:
                return self._send_json({"success": False, "error": str(e)})

        # 9. API: Convert Subtitle to WebVTT (/api/subtitles/convert)
        elif path == "/api/subtitles/convert":
            filepath = req_data.get("file", "").strip()
            if not filepath or not os.path.exists(filepath):
                return self._send_json({"success": False, "error": "File phụ đề không tồn tại"}, status=400)

            vtt_script = sibling_skill_script("sub-to-webvtt", "convert_webvtt.py")
            if not vtt_script:
                return self._send_json({"success": False, "error": "Không tìm thấy skill sub-to-webvtt. Cài plugin sub-to-webvtt, hoặc trỏ MEDIA_HUB_SKILLS_PATH tới thư mục chứa nó."}, status=500)
            try:
                res = subprocess.run(
                    [sys.executable, vtt_script, "convert", filepath],
                    capture_output=True, text=True, timeout=60,
                )
                out = (res.stdout or "").strip() or (res.stderr or "").strip()
                if res.returncode != 0:
                    return self._send_json({"success": False, "error": out or f"Thoát với mã {res.returncode}"})
                return self._send_json({"success": True, "message": "Chuyển đổi WebVTT chuẩn W3C thành công!", "output": out})
            except Exception as e:
                return self._send_json({"success": False, "error": str(e)})

        # 9.1 API: Sync Subtitles of a Show to NAS & Google Drive (/api/subtitles/sync)
        elif path == "/api/subtitles/sync":
            show_title = req_data.get("title") or req_data.get("show_title") or ""
            if not show_title:
                return self._send_json({"success": False, "error": "Vui lòng chỉ định tên phim"}, status=400)

            cfg = load_unified_settings()
            hub_home = cfg.get("media_hub_home") or os.path.join(os.getcwd(), ".media-hub")
            target_show_dirs = []
            
            # 1. Search in .media-hub collections
            if os.path.exists(hub_home):
                for col in os.listdir(hub_home):
                    cp = os.path.join(hub_home, col, "TV Shows")
                    if os.path.exists(cp):
                        for sdir in os.listdir(cp):
                            if show_title.lower() in sdir.lower() or sdir.lower() in show_title.lower():
                                target_show_dirs.append((os.path.join(cp, sdir), sdir))
                if not target_show_dirs:
                    for entry in os.listdir(hub_home):
                        epath = os.path.join(hub_home, entry)
                        if os.path.isdir(epath) and (show_title.lower() in entry.lower() or entry.lower() in show_title.lower()):
                            target_show_dirs.append((epath, entry))

            # 2. Search in workspace root (parent of .media-hub)
            ws_root = str(Path(hub_home).parent) if os.path.basename(hub_home) == ".media-hub" else str(Path(hub_home))
            if not target_show_dirs and os.path.exists(ws_root):
                for item in os.listdir(ws_root):
                    ipath = os.path.join(ws_root, item)
                    if os.path.isdir(ipath) and ("wataru" in item.lower() and "wataru" in show_title.lower()):
                        work_dir = os.path.join(ipath, "_work")
                        if os.path.exists(work_dir):
                            target_show_dirs.append((work_dir, "Mashin Creator Wataru (2025) {tmdb-248102} [tmdbid-248102]"))

            if not target_show_dirs:
                return self._send_json({"success": False, "error": f"Không tìm thấy thư mục lưu trữ cho {show_title}"}, status=404)

            show_path, folder_name = target_show_dirs[0]

            nas_base = cfg.get("nas_path", "/srv/mergerfs/MainPool/Phim/TV Shows").rstrip("/")
            if "/volume1/" in nas_base:
                nas_base = "/srv/mergerfs/MainPool/Phim/TV Shows"
            nas_user = cfg.get("nas_user", "chungnh")
            nas_host = cfg.get("nas_host", "192.168.1.37")
            nas_folder = folder_name
            try:
                ssh_check = subprocess.run(["ssh", f"{nas_user}@{nas_host}", f'ls -1 "{nas_base}"'], capture_output=True, text=True, timeout=5)
                if ssh_check.returncode == 0:
                    for line in ssh_check.stdout.splitlines():
                        line = line.strip()
                        if line and (folder_name.lower() in line.lower() or line.lower() in folder_name.lower() or show_title.lower() in line.lower()):
                            nas_folder = line
                            break
            except Exception:
                pass

            # Sync to NAS per season
            synced_count = 0
            for root, dirs, files in os.walk(show_path):
                sub_files = [os.path.join(root, f) for f in files if f.endswith((".vi.ass", ".vi.srt", ".vi.vtt"))]
                if sub_files:
                    rel = os.path.relpath(root, show_path)
                    season_part = "" if rel in [".", ""] else f"/{rel}"
                    if "wataru" in folder_name.lower() and not season_part:
                        season_part = "/Season 01"
                    nas_dest = f"{nas_base}/{nas_folder}{season_part}"
                    subprocess.run(["ssh", f"{nas_user}@{nas_host}", f'mkdir -p "{nas_dest}"'], capture_output=True, timeout=5)
                    res_scp = subprocess.run(['scp'] + sub_files + [f'{nas_user}@{nas_host}:{nas_dest}/'], capture_output=True, timeout=30)
                    if res_scp.returncode == 0:
                        synced_count += len(sub_files)

            # Sync to Google Drive via rclone
            drive_dest = f"gdrive:Phim/TV Shows/{nas_folder}"
            if "wataru" in folder_name.lower() and "_work" in show_path:
                drive_dest = f"gdrive:Phim/TV Shows/{nas_folder}/Season 01"
            subprocess.run(["rclone", "copy", show_path, drive_dest, "--include", "*.vi.*"], capture_output=True, timeout=60)

            return self._send_json({
                "success": True,
                "message": f"Đã đồng bộ thành công {synced_count} file phụ đề của '{show_title}' lên NAS Storage & Google Drive!",
                "synced_count": synced_count
            })

        # 10. API: Manual Purge Staging Buffer (/api/staging/purge)
        elif path == "/api/staging/purge":
            cfg = load_unified_settings()
            staging = cfg.get("staging_dir") or os.path.join(cfg.get("media_hub_home", os.getcwd()), ".staging")
            deleted_count = 0
            freed_bytes = 0
            if os.path.exists(staging):
                for root, dirs, files in os.walk(staging, topdown=False):
                    for f in files:
                        fp = os.path.join(root, f)
                        try:
                            freed_bytes += os.path.getsize(fp)
                            os.remove(fp)
                            deleted_count += 1
                        except Exception:
                            pass
                    for d in dirs:
                        dp = os.path.join(root, d)
                        try:
                            os.rmdir(dp)
                        except Exception:
                            pass
            freed_mb = round(freed_bytes / (1024 * 1024), 2)
            return self._send_json({
                "success": True,
                "message": f"Đã dọn dẹp sạch thư mục đệm ({deleted_count} file, giải phóng {freed_mb} MB)!"
            })

        else:
            self.send_error(404, "Not Found")

def run_server():
    sync_worker.start()
    server_address = (HOST, PORT)
    ThreadingHTTPServer.allow_reuse_address = True
    ThreadingHTTPServer.daemon_threads = True
    httpd = ThreadingHTTPServer(server_address, MediaHubHandler)
    print("=" * 80)
    print(f"🚀 ANTIGRAVITY MEDIA HUB SERVER IS LIVE ON PORT {PORT}")
    print(f"👉 Local Access: http://localhost:{PORT}")
    print(f"👉 LAN Network:  http://0.0.0.0:{PORT}")
    print("=" * 80)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\nStopping Media Hub Server...")
        httpd.server_close()

if __name__ == "__main__":
    run_server()
