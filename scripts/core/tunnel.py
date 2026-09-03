# -*- coding: utf-8 -*-
"""
Cloudflare Quick Tunnel (trycloudflare) Manager for Antigravity Media Hub.
Provides remote HTTPS access without port forwarding or account requirements.
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

    def _load_state(self):
        """Load persistent tunnel state if any."""
        try:
            if self._state_file.exists():
                with open(self._state_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    pid = data.get("pid")
                    url = data.get("url")
                    port = data.get("port", 8888)
                    # Check if pid is still running and is cloudflared
                    if pid and self._is_pid_alive(pid):
                        self._url = url
                        self._port = port
                        self._started_at = data.get("started_at")
        except Exception:
            pass

    def _save_state(self):
        """Save tunnel state to disk."""
        try:
            self._state_dir.mkdir(parents=True, exist_ok=True)
            state = {
                "pid": self._proc.pid if self._proc and self._proc.poll() is None else None,
                "url": self._url,
                "port": self._port,
                "started_at": self._started_at,
                "updated_at": time.strftime("%Y-%m-%d %H:%M:%S")
            }
            with open(self._state_file, "w", encoding="utf-8") as f:
                json.dump(state, f, indent=2, ensure_ascii=False)
        except Exception:
            pass

    def _is_pid_alive(self, pid):
        try:
            os.kill(pid, 0)
            return True
        except (OSError, ProcessLookupError):
            return False

    def get_status(self):
        """Get live status of Cloudflare Tunnel."""
        bin_path = self.find_cloudflared_bin()
        
        # Check if process died
        if self._proc and self._proc.poll() is not None:
            self._proc = None
            self._url = None
            self._save_state()

        is_running = bool(self._url and (
            (self._proc and self._proc.poll() is None) or 
            self._check_state_pid_running()
        ))

        return {
            "installed": bool(bin_path),
            "binary": bin_path or "Chưa cài đặt (brew install cloudflared)",
            "running": is_running,
            "url": self._url if is_running else None,
            "port": self._port,
            "started_at": self._started_at if is_running else None,
            "error": self._error,
            "recent_logs": list(self._logs)[-25:]
        }

    def _check_state_pid_running(self):
        """Check if existing background daemon recorded in state is running."""
        try:
            if self._state_file.exists():
                with open(self._state_file, "r", encoding="utf-8") as f:
                    data = json.load(f)
                    pid = data.get("pid")
                    if pid and self._is_pid_alive(pid):
                        return True
        except Exception:
            pass
        return False

    def start(self, port=8888):
        """Start Cloudflare Quick Tunnel on specified port."""
        with self._lock:
            # If already running on same port, return existing URL
            st = self.get_status()
            if st["running"] and self._url:
                return {
                    "success": True,
                    "url": self._url,
                    "message": f"Cloudflare Tunnel đang hoạt động tại {self._url}",
                    "status": st
                }

            bin_path = self.find_cloudflared_bin()
            if not bin_path:
                self._error = "Không tìm thấy cloudflared trên hệ thống."
                return {
                    "success": False,
                    "error": "cloudflared chưa được cài đặt. Vui lòng mở Terminal và chạy: brew install cloudflared",
                    "status": self.get_status()
                }

            self._port = int(port or 8888)
            self._error = None
            self._logs.clear()
            self._url = None

            cmd = [bin_path, "tunnel", "--url", f"http://127.0.0.1:{self._port}"]
            
            try:
                proc = subprocess.Popen(
                    cmd,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                    bufsize=1,
                    start_new_session=True  # Isolated session so app quits don't kill it
                )
                self._proc = proc
            except Exception as e:
                self._error = f"Không khởi chạy được cloudflared: {e}"
                return {
                    "success": False,
                    "error": self._error,
                    "status": self.get_status()
                }

            # URL Discovery in main thread with 15s timeout
            discovered_url = None
            start_time = time.time()
            url_regex = re.compile(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com")

            while time.time() - start_time < 15:
                if proc.poll() is not None:
                    # Process exited prematurely
                    self._error = f"Tiến trình cloudflared thoát sớm với mã lỗi {proc.returncode}"
                    self._proc = None
                    return {
                        "success": False,
                        "error": self._error,
                        "status": self.get_status()
                    }

                line = proc.stdout.readline()
                if line:
                    clean_l = line.strip()
                    if clean_l:
                        self._logs.append(clean_l)
                        m = url_regex.search(clean_l)
                        if m:
                            discovered_url = m.group(0)
                            break
                else:
                    time.sleep(0.1)

            if not discovered_url:
                # Stop proc if URL was not found
                self.stop()
                self._error = "Hết thời gian chờ nhận URL Public từ Cloudflare Edge."
                return {
                    "success": False,
                    "error": self._error,
                    "status": self.get_status()
                }

            self._url = discovered_url
            self._started_at = time.strftime("%Y-%m-%d %H:%M:%S")
            self._save_state()

            # Start background reader thread to continuously read logs
            def _log_reader():
                try:
                    for raw_l in iter(proc.stdout.readline, ""):
                        l = raw_l.strip()
                        if l:
                            self._logs.append(l)
                except Exception:
                    pass

            self._reader_thread = threading.Thread(target=_log_reader, daemon=True)
            self._reader_thread.start()

            return {
                "success": True,
                "url": self._url,
                "message": f"✅ Đã tạo Cloudflare Quick Tunnel thành công: {self._url}",
                "status": self.get_status()
            }

    def stop(self):
        """Stop running Cloudflare Tunnel."""
        with self._lock:
            stopped = False
            if self._proc:
                try:
                    import signal
                    try:
                        os.killpg(os.getpgid(self._proc.pid), signal.SIGTERM)
                    except Exception:
                        self._proc.terminate()
                    try:
                        self._proc.wait(timeout=3)
                    except subprocess.TimeoutExpired:
                        try:
                            os.killpg(os.getpgid(self._proc.pid), signal.SIGKILL)
                        except Exception:
                            self._proc.kill()
                    stopped = True
                except Exception as e:
                    self._logs.append(f"⚠️ Lỗi khi dừng tunnel: {e}")
                self._proc = None

            # Also check state file PID if different
            try:
                if self._state_file.exists():
                    with open(self._state_file, "r", encoding="utf-8") as f:
                        data = json.load(f)
                        pid = data.get("pid")
                        if pid and self._is_pid_alive(pid):
                            try:
                                os.kill(pid, signal.SIGTERM)
                                stopped = True
                            except Exception:
                                pass
            except Exception:
                pass

            self._url = None
            self._started_at = None
            self._save_state()
            return {
                "success": True,
                "message": "🛑 Đã tắt Cloudflare Tunnel thành công.",
                "status": self.get_status()
            }


# Singleton instance
tunnel_mgr = TunnelManager()
