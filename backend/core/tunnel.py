# -*- coding: utf-8 -*-
"""
Cloudflare Quick Tunnel (trycloudflare) Manager for Antigravity Media Hub.
Provides persistent remote HTTPS access without port forwarding or account requirements.
Designed with Domain Preservation Strategy: Reuses existing tunnel & domain across server restarts.
"""

import os
import re
import time
import json
import signal
import shutil
import subprocess
import threading
from collections import deque
from pathlib import Path

class TunnelManager:
    _instance = None
    _lock = threading.Lock()

    def __new__(cls, *args, **kwargs):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(TunnelManager, cls).__new__(cls)
                cls._instance._init_manager()
            return cls._instance

    def _init_manager(self):
        self._proc = None
        self._url = None
        self._port = 8888
        self._started_at = None
        self._error = None
        self._logs = deque(maxlen=100)
        self._reader_thread = None
        self._state_dir = Path.home() / ".media-hub"
        self._state_file = self._state_dir / "tunnel_state.json"
        self._log_file = self._state_dir / "tunnel.log"
        self._load_state()

    def find_cloudflared_bin(self):
        """Locate cloudflared binary in PATH or standard macOS/Linux paths."""
        which_path = shutil.which("cloudflared")
        if which_path and os.path.isfile(which_path):
            return which_path

        candidates = [
            "/opt/homebrew/bin/cloudflared",
            "/usr/local/bin/cloudflared",
            os.path.expanduser("~/.local/bin/cloudflared"),
            "/usr/bin/cloudflared",
            "/bin/cloudflared",
        ]
        for cand in candidates:
            if os.path.isfile(cand) and os.access(cand, os.X_OK):
                return cand
        return None

    def _is_cloudflared_pid(self, pid):
        """Check if PID is alive and is actually a cloudflared process."""
        if not pid or not isinstance(pid, int):
            return False
        try:
            os.kill(pid, 0)
            # Verify process name
            res = subprocess.run(
                ["ps", "-p", str(pid), "-o", "command="],
                capture_output=True,
                text=True,
                timeout=2
            )
            return "cloudflared" in (res.stdout or "")
        except Exception:
            return False

    def _extract_url_from_log(self):
        """Extract public trycloudflare URL from persistent log file."""
        if not self._log_file.exists():
            return None
        url_regex = re.compile(r"https://(?!api\.)[a-zA-Z0-9-]+\.trycloudflare\.com")
        try:
            with open(self._log_file, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.readlines()
                for line in reversed(lines):
                    m = url_regex.search(line)
                    if m:
                        return m.group(0)
        except Exception:
            pass
        return None

    def _load_state(self):
        """Load persistent tunnel state and reattach to running daemon if alive."""
        try:
            if self._state_file.exists():
                with open(self._state_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    pid = data.get("pid")
                    url = data.get("url")
                    port = data.get("port", 8888)
                    started_at = data.get("started_at")

                    if pid and self._is_cloudflared_pid(pid):
                        self._url = url or self._extract_url_from_log()
                        self._port = port
                        self._started_at = started_at
                        self._start_tail_thread()
                        return
        except Exception:
            pass

        # Also check system processes if state file was missing or stale
        try:
            res = subprocess.run(
                ["pgrep", "-f", "cloudflared tunnel --url"],
                capture_output=True,
                text=True,
                timeout=2
            )
            pids = [int(p.strip()) for p in res.stdout.split() if p.strip().isdigit()]
            for pid in pids:
                if self._is_cloudflared_pid(pid):
                    url = self._extract_url_from_log()
                    if url:
                        self._url = url
                        self._started_at = time.strftime("%Y-%m-%d %H:%M:%S")
                        self._save_state(pid)
                        self._start_tail_thread()
                        return
        except Exception:
            pass

    def _save_state(self, pid=None):
        """Save tunnel state to disk."""
        try:
            self._state_dir.mkdir(parents=True, exist_ok=True)
            active_pid = pid
            if active_pid is None:
                if self._proc and self._proc.poll() is None:
                    active_pid = self._proc.pid
                elif self._state_file.exists():
                    try:
                        with open(self._state_file, "r", encoding="utf-8") as f:
                            d = json.load(f)
                            existing_pid = d.get("pid")
                            if existing_pid and self._is_cloudflared_pid(existing_pid):
                                active_pid = existing_pid
                    except Exception:
                        pass

            state = {
                "pid": active_pid,
                "url": self._url,
                "port": self._port,
                "started_at": self._started_at,
                "updated_at": time.strftime("%Y-%m-%d %H:%M:%S")
            }
            with open(self._state_file, "w", encoding="utf-8") as f:
                json.dump(state, f, indent=2, ensure_ascii=False)
        except Exception:
            pass

    def _start_tail_thread(self):
        """Start background log tailing thread from log file."""
        if self._reader_thread and self._reader_thread.is_alive():
            return

        def _tail():
            try:
                if self._log_file.exists():
                    with open(self._log_file, "r", encoding="utf-8", errors="ignore") as f:
                        f.seek(0, os.SEEK_END)
                        while True:
                            line = f.readline()
                            if line:
                                l = line.strip()
                                if l:
                                    self._logs.append(l)
                            else:
                                time.sleep(1)
            except Exception:
                pass

        self._reader_thread = threading.Thread(target=_tail, daemon=True)
        self._reader_thread.start()

    def get_status(self):
        """Get live status of Cloudflare Tunnel with domain persistence check."""
        bin_path = self.find_cloudflared_bin()
        
        # Check if running
        is_running = False
        pid = None
        if self._proc and self._proc.poll() is None:
            is_running = bool(self._url)
            pid = self._proc.pid
        elif self._state_file.exists():
            try:
                with open(self._state_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    pid = data.get("pid")
                    if pid and self._is_cloudflared_pid(pid):
                        is_running = bool(self._url or data.get("url"))
                        if not self._url:
                            self._url = data.get("url") or self._extract_url_from_log()
                            self._started_at = data.get("started_at")
            except Exception:
                pass

        if not is_running:
            self._url = None
            self._started_at = None

        return {
            "installed": bool(bin_path),
            "binary": bin_path or "Chưa cài đặt (brew install cloudflared)",
            "running": is_running,
            "url": self._url if is_running else None,
            "port": self._port,
            "started_at": self._started_at if is_running else None,
            "pid": pid if is_running else None,
            "error": self._error,
            "recent_logs": list(self._logs)[-25:]
        }

    def start(self, port=8888, force_new=False):
        """Start Cloudflare Quick Tunnel on specified port. Preserves existing domain if active."""
        with self._lock:
            st = self.get_status()
            
            # If already running and not force_new, PRESERVE EXISTING DOMAIN
            if st["running"] and st["url"] and not force_new:
                return {
                    "success": True,
                    "url": st["url"],
                    "message": f"♻️ Giữ nguyên domain Cloudflare đang hoạt động: {st['url']}",
                    "status": st
                }

            bin_path = self.find_cloudflared_bin()
            if not bin_path:
                self._error = "Không tìm thấy cloudflared trên hệ thống."
                return {
                    "success": False,
                    "error": "cloudflared chưa được cài đặt. Vui lòng chạy: brew install cloudflared",
                    "status": self.get_status()
                }

            # Stop existing process if force_new
            if force_new:
                self.stop()

            self._port = int(port or 8888)
            self._error = None
            self._logs.clear()
            self._url = None

            self._state_dir.mkdir(parents=True, exist_ok=True)
            log_out = open(self._log_file, "a", encoding="utf-8")

            cmd = [bin_path, "tunnel", "--url", f"http://127.0.0.1:{self._port}"]
            
            try:
                proc = subprocess.Popen(
                    cmd,
                    stdin=subprocess.DEVNULL,
                    stdout=log_out,
                    stderr=subprocess.STDOUT,
                    start_new_session=True  # Fully detached session immune to SIGHUP/SIGPIPE
                )
                self._proc = proc
            except Exception as e:
                self._error = f"Không khởi chạy được cloudflared: {e}"
                try:
                    log_out.close()
                except Exception:
                    pass
                return {
                    "success": False,
                    "error": self._error,
                    "status": self.get_status()
                }

            # Discover public URL by reading the tail of log file
            discovered_url = None
            start_time = time.time()
            url_regex = re.compile(r"https://(?!api\.)[a-zA-Z0-9-]+\.trycloudflare\.com")

            try:
                with open(self._log_file, "r", encoding="utf-8", errors="ignore") as rf:
                    rf.seek(0, os.SEEK_END)
                    while time.time() - start_time < 20:
                        if proc.poll() is not None:
                            self._error = f"Tiến trình cloudflared thoát sớm với mã lỗi {proc.returncode}"
                            self._proc = None
                            return {
                                "success": False,
                                "error": self._error,
                                "status": self.get_status()
                            }
                        line = rf.readline()
                        if line:
                            clean_l = line.strip()
                            if clean_l:
                                self._logs.append(clean_l)
                                m = url_regex.search(clean_l)
                                if m:
                                    discovered_url = m.group(0)
                                    break
                        else:
                            time.sleep(0.2)
            except Exception as e:
                self._error = f"Lỗi khi đọc URL từ log: {e}"

            if not discovered_url:
                self.stop()
                self._error = "Hết thời gian chờ nhận URL Public từ Cloudflare Edge."
                return {
                    "success": False,
                    "error": self._error,
                    "status": self.get_status()
                }

            self._url = discovered_url
            self._started_at = time.strftime("%Y-%m-%d %H:%M:%S")
            self._save_state(proc.pid)
            self._start_tail_thread()

            return {
                "success": True,
                "url": self._url,
                "message": f"✅ Đã tạo Cloudflare Quick Tunnel thành công: {self._url}",
                "status": self.get_status()
            }

    def stop(self):
        """Stop running Cloudflare Tunnel."""
        with self._lock:
            # 1. Kill proc if owned
            if self._proc:
                try:
                    try:
                        os.killpg(os.getpgid(self._proc.pid), signal.SIGTERM)
                    except Exception:
                        self._proc.terminate()
                    try:
                        self._proc.wait(timeout=2)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(os.getpgid(self._proc.pid), signal.SIGKILL)
                        except Exception:
                            self._proc.kill()
                except Exception:
                    pass
                self._proc = None

            # 2. Kill PID from state file
            try:
                if self._state_file.exists():
                    with open(self._state_file, "r", encoding="utf-8") as f:
                        data = json.load(f)
                        pid = data.get("pid")
                        if pid and self._is_cloudflared_pid(pid):
                            try:
                                os.kill(pid, signal.SIGTERM)
                            except Exception:
                                pass
            except Exception:
                pass

            # 3. Clean up any remaining cloudflared processes on port
            try:
                subprocess.run(["pkill", "-f", "cloudflared tunnel --url"], timeout=2)
            except Exception:
                pass

            self._url = None
            self._started_at = None
            self._save_state(None)

            return {
                "success": True,
                "message": "🛑 Đã tắt Cloudflare Tunnel thành công.",
                "status": self.get_status()
            }


# Singleton instance
tunnel_mgr = TunnelManager()
