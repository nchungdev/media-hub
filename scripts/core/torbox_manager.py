#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
TorBox Manager Core Module
"""

import os
import json
import urllib.request
import urllib.parse
import urllib.error

class TorBoxManager:
    def __init__(self, config_path=None):
        self.api_key = None
        
        # 1. Check TorBox specific config
        candidates = [
            config_path,
            "/Volumes/512GB/AI Workspace/.media-hub/config.json",
            os.path.join(os.getcwd(), ".media-hub", "config.json"),
            "/Users/chungnh/.config/torbox/config.json",
            "/Users/chungnh/.agy-account2/.config/torbox/config.json",
            os.path.expanduser("~/.config/torbox/config.json"),
            os.path.expanduser("~/.gemini/config/media_hub_settings.json"),
            os.path.expanduser("~/.agy-account2/.gemini/config/media_hub_settings.json")
        ]
        
        for p in candidates:
            if p and os.path.exists(p):
                try:
                    with open(p, 'r', encoding='utf-8') as f:
                        data = json.load(f)
                        tok = data.get("torbox_token") or data.get("api_key") or data.get("token")
                        if tok and len(str(tok).strip()) > 10:
                            self.api_key = str(tok).strip()
                            break
                except Exception as e:
                    print(f"[TorBoxManager] Config load error ({p}): {e}")

        # 2. Check Unified Settings
        if not self.api_key:
            try:
                from core.settings import load_unified_settings
                cfg = load_unified_settings()
                tok = cfg.get("torbox_token") or cfg.get("api_key") or cfg.get("token")
                if tok and len(str(tok).strip()) > 10:
                    self.api_key = str(tok).strip()
            except Exception:
                pass
                    
        # 3. Check Environment Variables
        if not self.api_key:
            self.api_key = os.environ.get("TORBOX_API_TOKEN") or os.environ.get("TORBOX_TOKEN") or os.environ.get("TORBOX_API_KEY")
        self.base_url = "https://api.torbox.app/v1/api"
        self._list_cache = None
        self._list_cache_time = 0

        
    def _request(self, endpoint, method="GET", data=None, is_json=True):
        if not self.api_key:
            return {"success": False, "error": "No API key configured"}
            
        url = f"{self.base_url}/{endpoint}"
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "User-Agent": "Mozilla/5.0 (Antigravity-MediaHub/1.0)"
        }
        
        encoded_data = None
        if data is not None:
            if is_json:
                encoded_data = json.dumps(data).encode("utf-8")
                headers["Content-Type"] = "application/json"
            else:
                encoded_data = urllib.parse.urlencode(data).encode("utf-8")
                headers["Content-Type"] = "application/x-www-form-urlencoded"
                
        req = urllib.request.Request(url, data=encoded_data, headers=headers, method=method)
        try:
            with urllib.request.urlopen(req, timeout=15) as resp:
                res_body = resp.read().decode("utf-8")
                return json.loads(res_body)
        except urllib.error.HTTPError as e:
            try:
                err_body = e.read().decode("utf-8")
                return json.loads(err_body)
            except Exception:
                return {"success": False, "error": f"HTTP {e.code}: {e.reason}"}
        except Exception as e:
            return {"success": False, "error": str(e)}

    def list_torrents(self):
        # 1. Fetch active / completed torrents
        active_res = self._request("torrents/mylist?bypass_cache=true")
        active_items = active_res.get("data", []) if isinstance(active_res.get("data"), list) else []

        # 2. Fetch queued torrents
        queued_res = self._request("queued/getqueued")
        queued_items = queued_res.get("data", []) if isinstance(queued_res.get("data"), list) else []

        # Mark and normalize queued items
        for q in queued_items:
            q["is_queued"] = True
            q["download_state"] = "queued"
            q["progress"] = 0.0
            q["size"] = q.get("size", 0)

        # Merge both lists
        all_items = active_items + queued_items
        return {
            "success": True,
            "data": all_items,
            "counts": {
                "total": len(all_items),
                "active": len(active_items),
                "queued": len(queued_items)
            }
        }

    def control_queued(self, queued_id, operation="start"):
        return self._request("queued/controlqueued", method="POST", data={
            "queued_id": int(queued_id),
            "operation": operation
        }, is_json=True)

    def add_magnet(self, magnet_link):
        return self._request("torrents/createtorrent", method="POST", data={
            "magnet": magnet_link,
            "seed": 1,
            "allow_zip": "true"
        }, is_json=False)

    def delete_torrent(self, torrent_id):
        return self._request("torrents/controltorrent", method="POST", data={
            "torrent_id": int(torrent_id),
            "operation": "delete"
        }, is_json=True)

    def request_download_link(self, torrent_id, file_id=None, zip_link=True):
        params = {"token": self.api_key, "torrent_id": int(torrent_id)}
        if zip_link:
            params["zip"] = "true"
        elif file_id is not None:
            params["file_id"] = int(file_id)
        endpoint = f"torrents/requestdl?{urllib.parse.urlencode(params)}"
        return self._request(endpoint, method="GET")

    def get_user_info(self):
        return self._request("user/me")
