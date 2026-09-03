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

def cache_dir():
    """Poster cache under the project root, not the home directory."""
    from core.settings import resolve_dirs, load_unified_settings
    d = Path(resolve_dirs(load_unified_settings())["cache_dir"]) / "posters"
    d.mkdir(parents=True, exist_ok=True)
    return d
TMDB_BASE = "https://api.themoviedb.org/3"
def _cache_key(tvdb_id=None, tmdb_id=None, title=None):
    if tvdb_id:
        return f"tvdb-{tvdb_id}"
    if tmdb_id:
        return f"tmdb-{tmdb_id}"
    return "title-" + hashlib.sha1((title or "").lower().encode("utf-8")).hexdigest()[:16]


def _tmdb_get(path, params, api_key):
    params = dict(params or {})
    params["api_key"] = api_key
    url = f"{TMDB_BASE}{path}?{urllib.parse.urlencode(params)}"
    req = urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
    with urllib.request.urlopen(req, timeout=10) as resp:
        return json.loads(resp.read().decode("utf-8"))


def get_poster_bytes(api_key=None, tvdb_id=None, tmdb_id=None, title=None):
    """(bytes, content_type) for a poster.

    Read-only by design: it serves the cache that LibraryBuilder fills, and otherwise
    returns a placeholder telling the user to run the metadata build. Page loads must
    not call TMDb — artwork belongs in the library next to the media, written once
    during curation or by the manual build. `api_key` is accepted and ignored so the
    older call signature keeps working.
    """
    root = cache_dir()
    for key in filter(None, [
        _cache_key(tvdb_id, tmdb_id, None) if (tvdb_id or tmdb_id) else None,
        _cache_key(None, None, title) if title else None,
    ]):
        cached = root / f"{key}.jpg"
        if cached.is_file() and cached.stat().st_size > 0:
            return cached.read_bytes(), "image/jpeg"

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
