#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Poster resolution and caching.

Artwork used to be 34 files committed into the skill (17 JPEGs plus 17 hand-drawn
SVG placeholder cards, ~1 MB). That is derived data: it only covered one person's
library, went stale, and shipped to everyone who installed the plugin. Posters are
now fetched from TMDb on demand and cached OUTSIDE the repository, with a generated
placeholder when there is no key or no match — never a stale bundled image.

Cache: ~/.gemini/cache/posters/<key>.jpg
"""

import os
import re
import json
import time
import hashlib
import urllib.parse
import urllib.request
from pathlib import Path

TVDB_ALIASES = {
    "78864": "72281",    # Black Jack (OVA vs TV)
    "81092": "72281",    # Black Jack TV -> OVA
    "79354": "79460",    # Kindaichi Anime vs Live
    "279782": "69355",   # Kindaichi Returns (TMDb 69355)
    "79178": "77087",    # Transformers Car Robots
    "454526": "453871",  # WUKONG Dai Vien Hon
    "259259": "259635",  # Kingdom
    "350711": "371131",  # The Westward
    "80674": "256320",   # Furuhata Ninzaburo
    "79284": "81785",    # Houshin Engi 1999
    "335191": "337121",  # Hakyuu Houshin Engi 2018
    "299770": "252384",  # Young Black Jack
}

def cache_dir():
    """Poster cache under the project root, not the home directory."""
    try:
        from core.settings import resolve_dirs, load_unified_settings
        c_dir = resolve_dirs(load_unified_settings()).get("cache_dir")
        if c_dir:
            d = Path(c_dir) / "posters"
            d.mkdir(parents=True, exist_ok=True)
            return d
    except Exception:
        pass
    d = Path.home() / ".media-hub" / ".cache" / "posters"
    d.mkdir(parents=True, exist_ok=True)
    return d

def _candidate_cache_dirs():
    """List of all directories where cached posters might be stored."""
    candidates = [
        cache_dir(),
        Path("/Volumes/512GB/AI Workspace/.media-hub/.cache/posters"),
        Path.home() / ".media-hub" / ".cache" / "posters",
        Path.home() / ".gemini" / "cache" / "posters",
        Path.home() / "Applications" / "Media Hub.app" / "Contents" / "Resources" / ".media-hub" / ".cache" / "posters",
    ]
    seen = set()
    result = []
    for c in candidates:
        cp = str(c.resolve()) if c.exists() else str(c)
        if cp not in seen and c.is_dir():
            seen.add(cp)
            result.append(c)
    return result

TMDB_BASE = "https://api.themoviedb.org/3"

def _cache_key(tvdb_id=None, tmdb_id=None, title=None):
    if tvdb_id:
        return f"tvdb-{tvdb_id}"
    if tmdb_id:
        return f"tmdb-{tmdb_id}"
    clean = re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", str(title or "")).strip().lower()
    return "title-" + hashlib.sha1(clean.encode("utf-8")).hexdigest()[:16]


def _tmdb_get(path, params, api_key):
    params = dict(params or {})
    params["api_key"] = api_key
    url = f"{TMDB_BASE}{path}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode("utf-8"))


def _scan_local_workspace_for_poster(tvdb_id=None, title=None):
    """Scan local show directories in workspace for an existing poster.jpg."""
    roots = [
        Path("/Volumes/512GB/AI Workspace/.media-hub"),
        Path("/Volumes/512GB/AI Workspace/TV Shows"),
        Path("/Volumes/512GB/AI Workspace/Movies"),
    ]
    tokens = []
    if tvdb_id:
        tokens.append(f"tvdb-{tvdb_id}")
        tokens.append(f"tvdbid-{tvdb_id}")
        alias = TVDB_ALIASES.get(str(tvdb_id))
        if alias:
            tokens.append(f"tvdb-{alias}")
            tokens.append(f"tvdbid-{alias}")
    if title:
        clean = re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", str(title)).strip().lower()
        if clean:
            tokens.append(clean)

    for r in roots:
        if not r.is_dir():
            continue
        try:
            for p in r.rglob("poster.jpg"):
                parent_name = p.parent.name.lower()
                for tok in tokens:
                    if tok.lower() in parent_name:
                        return p
        except Exception:
            pass
    return None


def _fetch_from_kitsu(title):
    """Free anime poster lookup from Kitsu API (no API key required)."""
    if not title:
        return None
    clean = re.sub(r"\{.*?\}|\[.*?\]|\(\d{4}\)", "", str(title)).strip()
    if not clean:
        return None
    try:
        url = f"https://kitsu.io/api/edge/anime?filter[text]={urllib.parse.quote(clean)}"
        req = urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
        with urllib.request.urlopen(req, timeout=4) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            if data.get("data"):
                img_url = data["data"][0].get("attributes", {}).get("posterImage", {}).get("medium")
                if img_url:
                    img_req = urllib.request.Request(img_url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
                    with urllib.request.urlopen(img_req, timeout=5) as img_resp:
                        return img_resp.read()
    except Exception:
        pass
    return None


def get_poster_bytes(api_key=None, tvdb_id=None, tmdb_id=None, title=None):
    """(bytes, content_type) for a poster.

    Multi-tiered resolution:
    1. Search all cache directories for direct keys and TVDB alias keys.
    2. Search local workspace show folders for existing poster.jpg.
    3. Fetch online via Kitsu / TMDb if available.
    4. Fallback to clean SVG placeholder.
    """
    keys = []
    if tvdb_id:
        keys.append(f"tvdb-{tvdb_id}")
        alias = TVDB_ALIASES.get(str(tvdb_id))
        if alias:
            keys.append(f"tvdb-{alias}")
            keys.append(f"tmdb-{alias}")
    if tmdb_id:
        keys.append(f"tmdb-{tmdb_id}")
    if title:
        keys.append(_cache_key(None, None, title))

    # Tier 1: Check existing cache directories
    for c_dir in _candidate_cache_dirs():
        for k in keys:
            cached = c_dir / f"{k}.jpg"
            if cached.is_file() and cached.stat().st_size > 0:
                return cached.read_bytes(), "image/jpeg"

    # Tier 2: Check local workspace folders for poster.jpg
    local_p = _scan_local_workspace_for_poster(tvdb_id, title)
    if local_p and local_p.is_file() and local_p.stat().st_size > 0:
        data = local_p.read_bytes()
        try:
            target_key = keys[0] if keys else _cache_key(tvdb_id, tmdb_id, title)
            (cache_dir() / f"{target_key}.jpg").write_bytes(data)
        except Exception:
            pass
        return data, "image/jpeg"

    # Tier 3: Fetch from Kitsu (Free Anime DB)
    if title or tvdb_id:
        kitsu_data = _fetch_from_kitsu(title)
        if kitsu_data and len(kitsu_data) > 1000:
            try:
                target_key = keys[0] if keys else _cache_key(tvdb_id, tmdb_id, title)
                (cache_dir() / f"{target_key}.jpg").write_bytes(kitsu_data)
            except Exception:
                pass
            return kitsu_data, "image/jpeg"

    return (placeholder_svg(title or _cache_key(tvdb_id, tmdb_id, title),
                            "Bấm 'Dựng Metadata' để tải"),
            "image/svg+xml")


def placeholder_svg(title, note=""):
    """Generated stand-in, in the same style as the cards that used to be committed."""
    clean = re.sub(r"\{.*?\}|\[.*?\]", "", str(title)).strip() or "Media"
    initials = "".join(w[0] for w in re.findall(r"[A-Za-z0-9]+", clean)[:3]).upper() or "?"
    display = (clean[:26] + "…") if len(clean) > 27 else clean

    def esc(s):
        return (str(s).replace("&", "&amp;").replace("<", "&lt;")
                .replace(">", "&gt;").replace('"', "&quot;"))

    return f'''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 600" width="400" height="600">
  <defs>
    <linearGradient id="bg" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#09090b"/><stop offset="100%" stop-color="#27272a"/>
    </linearGradient>
    <radialGradient id="glow" cx="50%" cy="45%" r="50%">
      <stop offset="0%" stop-color="#e5a00d" stop-opacity="0.25"/>
      <stop offset="100%" stop-color="#e5a00d" stop-opacity="0"/>
    </radialGradient>
  </defs>
  <rect width="400" height="600" fill="url(#bg)"/>
  <circle cx="200" cy="270" r="180" fill="url(#glow)"/>
  <rect x="8" y="8" width="384" height="584" rx="16" fill="none"
        stroke="#e5a00d" stroke-opacity="0.2" stroke-width="2"/>
  <circle cx="200" cy="260" r="70" fill="#000" fill-opacity="0.4"
          stroke="#e5a00d" stroke-opacity="0.3" stroke-width="2"/>
  <text x="200" y="285" font-family="-apple-system, BlinkMacSystemFont, sans-serif"
        font-size="52" font-weight="900" fill="#e5a00d" text-anchor="middle">{esc(initials)}</text>
  <text x="200" y="450" font-family="-apple-system, BlinkMacSystemFont, sans-serif"
        font-size="20" font-weight="800" fill="#fff" text-anchor="middle">{esc(display)}</text>
  <line x1="40" y1="515" x2="360" y2="515" stroke="#fff" stroke-opacity="0.1" stroke-width="1"/>
  <text x="200" y="545" font-family="monospace" font-size="11"
        fill="#64748b" text-anchor="middle">{esc(note)}</text>
</svg>'''.encode("utf-8")
