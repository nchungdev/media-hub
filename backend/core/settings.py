#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Unified settings loader, shared by the HTTP server and the agent queue watcher.

It lives here rather than in server.py so other processes can read the same
configuration without importing the server module (which owns the sync worker).
"""

import os
import json
from pathlib import Path


GLOBAL_SETTINGS = Path.home() / ".media-hub" / "app_settings.json"
LEGACY_GLOBAL_SETTINGS = Path.home() / ".gemini" / "config" / "media_hub_settings.json"
CONFIG_BASENAME = "config.json"


def config_path(root=None):
    """The project's own config, inside the hub root next to the database."""
    return Path(root or resolve_dirs({})["media_hub_home"]) / CONFIG_BASENAME


def load_unified_settings():
    """Load configuration from environment, ~/.env, global app settings, and the project config."""
    env_file = Path.home() / ".env"
    env_dict = {}
    if env_file.is_file():
        try:
            with open(env_file, "r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if line and not line.startswith("#") and "=" in line:
                        k, v = line.split("=", 1)
                        env_dict[k.strip()] = v.strip().strip('"').strip("'")
        except Exception:
            pass

    cfg = {
        "default_provider": "torbox",
        "max_concurrent_downloads": 2,
        # --- working directories ---------------------------------------------------
        "media_hub_home": "",  # Path to workspace's .media-hub
        "workspace_dir": "",   # The workspace root selected by the user
        "movies_dirname": "Movies",
        "tv_dirname": "TV Shows",
        "staging_dir": "",     # blank -> <root>/.staging
        "logs_dir": "",        # blank -> <root>/.logs
        "torbox_token": os.environ.get("TORBOX_API_TOKEN") or env_dict.get("TORBOX_API_TOKEN") or env_dict.get("TORBOX_TOKEN") or "",
        "tmdb_api_key": os.environ.get("TMDB_API_KEY") or env_dict.get("TMDB_API_KEY") or "",
        "tmdb_lang": "vi-VN",
        "aria2_rpc_host": "127.0.0.1",
        "aria2_rpc_port": 6800,
        "aria2_rpc_secret": "",
        "nas_host": "",
        "nas_user": "admin",
        "nas_port": 22,
        "nas_ssh_key": "",
        "nas_path": "/volume1/video/TV Shows",
        "gdrive_remote": "gdrive",
        "gdrive_root": "Phim",
        "sync_targets": ["drive"],
        "sync_transfers": 4,
        "auto_purge": True,
        "cloudflare_tunnel_token": "",
        "cloudflare_tunnel_hostname": ""
    }

    # 1. Load global application settings first (to determine user's chosen workspace)
    for path in (LEGACY_GLOBAL_SETTINGS, GLOBAL_SETTINGS):
        if path.is_file():
            try:
                with open(path, "r", encoding="utf-8") as f:
                    saved = json.load(f)
                if isinstance(saved, dict):
                    cfg.update(saved)
            except Exception:
                pass

    # 2. Resolve workspace directory and hub home
    cfg.update(resolve_dirs(cfg))

    # 3. Load project-level config if it exists in the active hub home
    proj_cfg = config_path(cfg["media_hub_home"])
    if proj_cfg.is_file():
        try:
            with open(proj_cfg, "r", encoding="utf-8") as f:
                saved = json.load(f)
            if isinstance(saved, dict):
                cfg.update(saved)
        except Exception:
            pass

    # Fallback to env if empty
    if not cfg.get("torbox_token"):
        cfg["torbox_token"] = os.environ.get("TORBOX_API_TOKEN") or env_dict.get("TORBOX_API_TOKEN") or env_dict.get("TORBOX_TOKEN") or ""
    if not cfg.get("tmdb_api_key"):
        cfg["tmdb_api_key"] = os.environ.get("TMDB_API_KEY") or env_dict.get("TMDB_API_KEY") or ""

    # Auto-detect SSH Key if empty
    if not cfg.get("nas_ssh_key"):
        ssh_dir = Path.home() / ".ssh"
        for k in ["id_ed25519", "id_rsa", "id_ecdsa"]:
            cand = ssh_dir / k
            if cand.is_file():
                cfg["nas_ssh_key"] = f"~/.ssh/{k}"
                break

    # Environment always wins
    for key, env in (("media_hub_home", "MEDIA_HUB_HOME"),
                     ("staging_dir", "MEDIA_HUB_STAGING_DIR"),
                     ("curation_dir", "MEDIA_HUB_CURATION_DIR"),
                     ("logs_dir", "MEDIA_HUB_LOGS_DIR")):
        if os.environ.get(env):
            cfg[key] = os.environ[env]

    cfg.update(resolve_dirs(cfg))
    return cfg


def save_unified_settings(cfg):
    """Save settings to global app config and to the project config in the active workspace."""
    GLOBAL_SETTINGS.parent.mkdir(parents=True, exist_ok=True)
    try:
        with open(GLOBAL_SETTINGS, "w", encoding="utf-8") as f:
            json.dump(cfg, f, indent=2, ensure_ascii=False)
    except Exception as e:
        print(f"[settings] Warning: cannot save global app settings: {e}")

    # Also save to project config inside the active hub home
    hub_home = cfg.get("media_hub_home")
    if hub_home and os.path.exists(hub_home):
        p_cfg = config_path(hub_home)
        p_cfg.parent.mkdir(parents=True, exist_ok=True)
        try:
            with open(p_cfg, "w", encoding="utf-8") as f:
                json.dump(cfg, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"[settings] Warning: cannot save project config: {e}")


HUB_DIRNAME = ".media-hub"


def find_hub_root(start=None):
    """Nearest existing .mediahub directory, walking up from `start` like git."""
    try:
        cur = Path(start or os.getcwd()).resolve()
    except Exception:
        return None
    for candidate in [cur, *cur.parents]:
        hub = candidate / HUB_DIRNAME
        if hub.is_dir():
            return str(hub)
    return None


SKILL_ROOT = Path(__file__).resolve().parent.parent


def _discovered_root():
    """Discovery, but never inside the app's own checkout."""
    found = find_hub_root()
    if not found:
        return None
    try:
        Path(found).relative_to(SKILL_ROOT)
        return None      # resolved inside the app itself — not a workspace
    except ValueError:
        return found


def _cwd_root():
    """Last resort: .media-hub beside the current directory."""
    cwd = os.getcwd()
    try:
        Path(cwd).resolve().relative_to(SKILL_ROOT)
    except ValueError:
        return os.path.join(cwd, HUB_DIRNAME)
    return os.path.join(os.path.expanduser("~"), HUB_DIRNAME)


def resolve_dirs(cfg=None, create=False):
    """Absolute paths for every directory the app writes to."""
    cfg = cfg or {}
    env_home = os.environ.get("MEDIA_HUB_HOME")
    
    # 1. Check explicit configuration
    configured = cfg.get("media_hub_home") or cfg.get("workspace_dir")
    
    if env_home:
        raw_home = os.path.expanduser(env_home)
    elif configured:
        raw_home = os.path.expanduser(configured)
    else:
        raw_home = _discovered_root() or _cwd_root()

    # Determine workspace_dir and media_hub_home cleanly
    if os.path.basename(raw_home) == HUB_DIRNAME:
        home = raw_home
        workspace_dir = str(Path(raw_home).parent)
    else:
        workspace_dir = raw_home
        home = os.path.join(raw_home, HUB_DIRNAME)

    needs_setup = not (os.path.exists(workspace_dir) and workspace_dir != os.path.expanduser("~"))

    def under(value, name):
        return os.path.expanduser(value) if value else os.path.join(home, name)

    dirs = {
        "media_hub_home": home,
        "workspace_dir": workspace_dir,
        "needs_setup": needs_setup,
        "movies_dirname": cfg.get("movies_dirname") or "Movies",
        "tv_dirname": cfg.get("tv_dirname") or "TV Shows",
        "staging_dir": under(cfg.get("staging_dir"), ".staging"),
        "logs_dir": under(cfg.get("logs_dir"), ".logs"),
        "cache_dir": under(cfg.get("cache_dir"), ".cache"),
        "db_path": os.path.expanduser(cfg.get("db_path") or os.path.join(home, ".media_hub.db")),
        "queue_path": os.path.expanduser(
            cfg.get("queue_path") or os.path.join(home, ".agent_queue.json")),
    }
    if create:
        for key in ("media_hub_home", "workspace_dir", "staging_dir", "logs_dir", "cache_dir"):
            path = dirs.get(key)
            if not isinstance(path, (str, Path)) or not path:
                continue
            try:
                Path(path).mkdir(parents=True, exist_ok=True)
            except Exception as e:
                print(f"[settings] Không tạo được thư mục {path}: {e}")
        # The root is hidden but git would still track it, so make it ignore itself
        # wherever it lands inside a repository.
        try:
            keep = Path(home) / ".gitignore"
            if not keep.exists():
                keep.parent.mkdir(parents=True, exist_ok=True)
                keep.write_text("*\n", encoding="utf-8")
        except Exception:
            pass
    return dirs


def safe_title(title):
    import re
    cleaned = re.sub(r"[\x00-\x1f/\\]+", "_", str(title or "").strip()).strip(". ")
    return cleaned[:150] or "untitled"


_JUNK = None


def clean_title(name):
    """Display name with release tags and id markers stripped, used to name the
    collection folder when the caller does not supply one."""
    import re
    global _JUNK
    if _JUNK is None:
        _JUNK = re.compile(
            r"\b(1080p|720p|480p|2160p|4k|bdrip|bluray|blu-ray|web-?dl|webrip|hdtv|"
            r"dvdrip|x264|x265|hevc|aac|ac3|flac|dual|remux|repack|proper)\b", re.I)
    out = re.sub(r"\{[^}]*\}|\[[^\]]*\]|\(\d{4}\)", " ", str(name or ""))
    out = _JUNK.sub(" ", out)
    return re.sub(r"\s{2,}", " ", out).strip(" -_.") or "untitled"


def collection_dir(collection, cfg=None, create=True):
    """<root>/<Collection>/ — everything belonging to one franchise."""
    cfg = cfg if cfg is not None else load_unified_settings()
    d = Path(cfg["media_hub_home"]) / safe_title(collection)
    if create:
        d.mkdir(parents=True, exist_ok=True)
    return str(d)


def title_dir(title, kind="tv", collection=None, cfg=None, create=True):
    """The one folder that holds everything for a title.

    Layout is collection-first: <root>/<Collection>/Movies|TV Shows/<Title>/, so a
    franchise with both a series and films keeps them together. Video, subtitles,
    tvshow.nfo/movie.nfo and artwork all live in the title folder, which is the unit
    that gets synced.

    A collection folder is always created; a standalone title simply gets one named
    after itself.
    """
    cfg = cfg if cfg is not None else load_unified_settings()
    sub = cfg["movies_dirname"] if str(kind).lower().startswith("movie") else cfg["tv_dirname"]
    base = Path(collection_dir(collection or clean_title(title), cfg, create=create)) / sub
    d = base / safe_title(title)
    if create:
        (d / ".work").mkdir(parents=True, exist_ok=True)   # drafts, never synced
    return str(d)


# Curation now happens inside the title folder rather than a parallel tree.
workspace_for = title_dir
