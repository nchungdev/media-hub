#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Antigravity Media Hub - Dashboard Launcher with Optional TryCloudflare Tunnel
"""

import os
import sys
import time
import subprocess
import re
import shutil
import threading
import argparse
import socket
import secrets

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
DEFAULT_PORT = 8888
URL_FILE = os.path.join(BASE_DIR, "active_public_url.txt")
SERVER_LOG = "/tmp/media_hub_server.log"
WATCHER_LOG = "/tmp/media_hub_watcher.log"

def find_cloudflared():
    for path in ["/opt/homebrew/bin/cloudflared", "/usr/local/bin/cloudflared", "cloudflared"]:
        resolved = shutil.which(path)
        if resolved and os.path.exists(resolved):
            return resolved
    return None

def get_local_ip():
    try:
        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        s.connect(("8.8.8.8", 80))
        ip = s.getsockname()[0]
        s.close()
        return ip
    except Exception:
        return "127.0.0.1"

def kill_process_on_port(port):
    try:
        res = subprocess.run(["lsof", "-t", "-i", f":{port}"], capture_output=True, text=True)
        pids = res.stdout.strip().split()
        for pid in pids:
            if pid and pid != str(os.getpid()):
                subprocess.run(["kill", "-9", pid], capture_output=True)
    except Exception:
        pass

def start_hub(enable_tunnel=False, port=DEFAULT_PORT):
    print("=" * 64, flush=True)
    print(f"🚀 Khởi chạy Antigravity Media Hub Dashboard (Port {port})...", flush=True)
    print("=" * 64, flush=True)

    # 1. Clean old processes on port to avoid deadlocks
    kill_process_on_port(port)
    time.sleep(0.5)

    # 2. Start Server
    #    A public tunnel exposes every API (including staging purge and NAS scan) to the
    #    internet, so require a token in that mode. Local/LAN mode is unchanged.
    auth_token = secrets.token_urlsafe(24) if enable_tunnel else ""
    server_env = dict(os.environ)
    server_env["MEDIA_HUB_TOKEN"] = auth_token
    server_env["MEDIA_HUB_PORT"] = str(port)

    server_script = os.path.join(BASE_DIR, "server.py")
    server_log_fp = open(SERVER_LOG, "a", encoding="utf-8")
    server_proc = subprocess.Popen(
        [sys.executable, server_script],
        stdout=server_log_fp,
        stderr=server_log_fp,
        env=server_env,
        start_new_session=True
    )
    print(f"✅ Web Server đã khởi động tại: http://127.0.0.1:{port}", flush=True)

    # 3. Start Agent Queue Watcher
    watcher_script = os.path.join(BASE_DIR, "agent_queue_watcher.py")
    watcher_proc = None
    if os.path.exists(watcher_script):
        watcher_log_fp = open(WATCHER_LOG, "a", encoding="utf-8")
        watcher_proc = subprocess.Popen(
            [sys.executable, watcher_script],
            stdout=watcher_log_fp,
            stderr=watcher_log_fp,
            start_new_session=True
        )
        print("✅ Agent Queue Watcher Daemon đã kích hoạt.", flush=True)

    local_ip = get_local_ip()
    local_url = f"http://127.0.0.1:{port}"
    lan_url = f"http://{local_ip}:{port}"

    tunnel_proc = None

    # 4. Optional TryCloudflare Tunnel
    if enable_tunnel:
        cloudflared_bin = find_cloudflared()
        if not cloudflared_bin:
            print("⚠️ Chưa cài đặt cloudflared (brew install cloudflared). Chỉ dùng Localhost.", flush=True)
            with open(URL_FILE, "w", encoding="utf-8") as f:
                f.write(local_url)
        else:
            print("🌐 Đang khởi tạo đường truyền TryCloudflare tốc độ cao...", flush=True)
            tunnel_proc = subprocess.Popen(
                [cloudflared_bin, "tunnel", "--url", f"http://127.0.0.1:{port}"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1
            )

            public_url_event = threading.Event()
            public_url_holder = {"url": None}

            def drain_pipe(stream):
                try:
                    for line in iter(stream.readline, ''):
                        if not line:
                            break
                        if not public_url_holder["url"]:
                            match = re.search(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com", line)
                            if match:
                                public_url_holder["url"] = match.group(0)
                                public_url_event.set()
                except Exception:
                    pass

            t_err = threading.Thread(target=drain_pipe, args=(tunnel_proc.stderr,), daemon=True)
            t_out = threading.Thread(target=drain_pipe, args=(tunnel_proc.stdout,), daemon=True)
            t_err.start()
            t_out.start()

            if public_url_event.wait(timeout=15) and public_url_holder["url"]:
                public_url = public_url_holder["url"]
                if auth_token:
                    public_url = f"{public_url}/?k={auth_token}"
                with open(URL_FILE, "w", encoding="utf-8") as f:
                    f.write(public_url)
                print("\n" + "=" * 64, flush=True)
                print("🎉 LINK TRUY CẬP ONLINE TỪ XA (TRYCLOUDFLARE):", flush=True)
                print(f"👉 {public_url}", flush=True)
                print("🔒 Link đã kèm token truy cập; ai không có token sẽ nhận lỗi 401.", flush=True)
                print("=" * 64 + "\n", flush=True)
            else:
                print("⚠️ Không lấy được link trycloudflare kịp thời. Sử dụng link Local.", flush=True)
                with open(URL_FILE, "w", encoding="utf-8") as f:
                    f.write(local_url)
    else:
        with open(URL_FILE, "w", encoding="utf-8") as f:
            f.write(local_url)
        print("\n" + "=" * 64, flush=True)
        print("🏠 ĐỊA CHỈ TRUY CẬP MEDIA HUB (CHẾ ĐỘ NỘI BỘ):", flush=True)
        print(f"👉 Trình duyệt máy này: {local_url}", flush=True)
        if local_ip != "127.0.0.1":
            print(f"👉 Thiết bị cùng mạng LAN: {lan_url}", flush=True)
        print("💡 Mẹo: Thêm cờ '--tunnel' nếu muốn mở link online công khai ra internet.")
        print("   Ví dụ: python3 launcher.py --tunnel", flush=True)
        print("=" * 64 + "\n", flush=True)

    print("ℹ️ Nhấn Ctrl+C để dừng Dashboard.", flush=True)
    try:
        while True:
            if server_proc.poll() is not None:
                print("⚠️ Server đã kết thúc unexpectedly. Đang thoát...")
                break
            if tunnel_proc and tunnel_proc.poll() is not None:
                break
            time.sleep(2)
    except KeyboardInterrupt:
        print("\n👋 Đang dừng Media Hub Dashboard...")
    finally:
        if tunnel_proc: tunnel_proc.terminate()
        if server_proc: server_proc.terminate()
        if watcher_proc: watcher_proc.terminate()

def parse_args():
    parser = argparse.ArgumentParser(description="Khởi chạy Antigravity Media Hub Dashboard")
    parser.add_argument(
        "-t", "--tunnel", "--public", "--cloudflared",
        action="store_true",
        dest="tunnel",
        default=os.environ.get("ENABLE_TUNNEL", "0").lower() in ("1", "true", "yes"),
        help="Kích hoạt TryCloudflare Tunnel để mở link online từ xa (Mặc định: Tắt / Local Only)"
    )
    parser.add_argument(
        "-p", "--port",
        type=int,
        default=DEFAULT_PORT,
        help=f"Cổng Web Server (Mặc định: {DEFAULT_PORT})"
    )
    return parser.parse_args()

if __name__ == "__main__":
    args = parse_args()
    start_hub(enable_tunnel=args.tunnel, port=args.port)
