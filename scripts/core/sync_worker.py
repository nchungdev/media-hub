#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Sync Worker — the process that actually performs the TorBox ➔ staging ➔ NAS/Drive pipeline.

Before this module existed, pressing "Sync" only appended a line to a JSON file that
nothing ever read. This worker consumes jobs from the JobStore and runs them for real:

    link  ➔ download ➔ upload (per target) ➔ verify ➔ purge

Design rules:
  * One download from TorBox per torrent, fanned out to every requested destination.
  * Remote paths are passed through shlex.quote — never interpolated into a shell string.
  * Auto-purge only fires after every target has been size-verified against the local file.
  * Progress is written to the store so the dashboard shows real numbers, not a fake 25%.
"""

import os
import re
import time
import json
import shlex
import shutil
import threading
import subprocess
import urllib.request
import urllib.error
from pathlib import Path

CHUNK = 1024 * 1024  # 1 MiB
PROGRESS_INTERVAL = 1.0  # seconds between store writes while downloading


def _safe_component(name):
    """Reduce an arbitrary torrent name to a single safe path component."""
    cleaned = re.sub(r"[\x00-\x1f/\\]+", "_", (name or "").strip())
    cleaned = cleaned.strip(". ")
    return (cleaned or "media")[:150]


def _human(n):
    n = float(n or 0)
    for unit in ("B", "KB", "MB", "GB", "TB"):
        if abs(n) < 1024.0:
            return f"{n:.1f} {unit}"
        n /= 1024.0
    return f"{n:.1f} PB"


class CancelledError(Exception):
    pass


class SyncWorker:
    def __init__(self, store, torbox_mgr, gdrive_mgr, settings_loader, concurrency=2):
        self.store = store
        self.torbox = torbox_mgr
        self.gdrive = gdrive_mgr
        self.load_settings = settings_loader
        self.concurrency = max(1, int(concurrency))
        self._threads = []
        self._stop = threading.Event()
        self._procs = {}          # job_id -> currently running subprocess
        self._procs_lock = threading.Lock()

    # ---------------- lifecycle ----------------

    def start(self):
        for i in range(self.concurrency):
            t = threading.Thread(target=self._loop, name=f"sync-worker-{i+1}", daemon=True)
            t.start()
            self._threads.append(t)
        print(f"[SyncWorker] Đã khởi động {self.concurrency} luồng đồng bộ.")

    def stop(self):
        self._stop.set()
        with self._procs_lock:
            for p in self._procs.values():
                try:
                    p.terminate()
                except Exception:
                    pass

    def _loop(self):
        while not self._stop.is_set():
            job = None
            try:
                job = self.store.claim_next()
            except Exception as e:
                print(f"[SyncWorker] Lỗi khi lấy job: {e}")
            if job is None:
                self._stop.wait(2.0)
                continue
            try:
                self._run_job(job)
            except CancelledError:
                self.store.finish(job["id"], "canceled", message="Đã hủy theo yêu cầu")
                self._cleanup_staging(job)
            except Exception as e:
                print(f"[SyncWorker] Job #{job['id']} thất bại: {e}")
                self.store.finish(job["id"], "failed", error=str(e))

    # ---------------- cancellation helpers ----------------

    def _check_cancel(self, job_id):
        if self._stop.is_set():
            raise CancelledError("server shutting down")
        if self.store.is_cancel_requested(job_id):
            raise CancelledError("user cancelled")

    def _register_proc(self, job_id, proc):
        with self._procs_lock:
            self._procs[job_id] = proc

    def _unregister_proc(self, job_id):
        with self._procs_lock:
            self._procs.pop(job_id, None)

    # ---------------- the pipeline ----------------

    def _run_job(self, job):
        job_id = job["id"]
        tid = job["torrent_id"]
        cfg = self.load_settings()
        staging_root = cfg.get("staging_dir") or "/tmp/media_staging"

        # 1. Resolve a direct download link from TorBox.
        self._check_cancel(job_id)
        self.store.update(job_id, phase="link", message="Đang lấy link tải trực tiếp từ TorBox...")
        url = self._request_link(tid)

        # 2. Download once into the staging buffer.
        self._check_cancel(job_id)
        job_dir = Path(staging_root) / f"{job_id}_{_safe_component(job['name'] or tid)}"
        job_dir.mkdir(parents=True, exist_ok=True)
        self.store.update(job_id, phase="download", staging_path=str(job_dir),
                          message="Đang kéo dữ liệu từ TorBox về bộ đệm...")
        local_file = self._download(job_id, url, job_dir, fallback_name=job['name'])
        local_size = local_file.stat().st_size
        self.store.update(job_id, bytes_done=local_size, bytes_total=local_size, progress=60.0,
                          message=f"Đã tải xong {_human(local_size)}. Bắt đầu đẩy lên đích...")

        # 3. Push to every requested destination, then verify each one.
        targets = job["targets"] or ["drive"]
        done_targets = list(job.get("done_targets") or [])
        failures = []
        for idx, target in enumerate(targets):
            self._check_cancel(job_id)
            if target in done_targets:
                continue
            base = 60.0 + (idx / max(1, len(targets))) * 35.0
            self.store.update(job_id, phase="upload", progress=base,
                              message=f"Đang đẩy lên {self._label(target)}...")
            try:
                self._upload(job_id, target, local_file, job['name'] or _safe_component(tid), cfg)
                self.store.update(job_id, phase="verify",
                                  message=f"Đang đối chiếu dung lượng trên {self._label(target)}...")
                remote_size = self._remote_size(target, local_file.name, job['name'] or _safe_component(tid), cfg)
                if remote_size is None:
                    raise RuntimeError(f"Không đọc được dung lượng trên {self._label(target)} để đối chiếu")
                if remote_size != local_size:
                    # Exact byte counts: rounded units can print "3.0 MB vs 3.0 MB".
                    raise RuntimeError(
                        f"Dung lượng lệch trên {self._label(target)}: "
                        f"local {local_size:,} bytes vs remote {remote_size:,} bytes"
                    )
                done_targets.append(target)
                self.store.update(job_id, done_targets=done_targets)
            except CancelledError:
                raise
            except Exception as e:
                failures.append(f"{self._label(target)}: {e}")

        if failures:
            # Keep the staging copy so nothing is lost when a destination failed.
            self.store.update(job_id, progress=95.0)
            self.store.finish(job_id, "failed", error=" | ".join(failures),
                              message="Giữ nguyên file đệm để bảo vệ dữ liệu.")
            return

        # 4. Auto-purge only once every destination verified byte-for-byte.
        purged = False
        if cfg.get("auto_purge", True):
            self.store.update(job_id, phase="purge", progress=98.0,
                              message="Đã xác minh toàn vẹn. Đang giải phóng bộ đệm...")
            purged = self._cleanup_staging({"staging_path": str(job_dir)})

        self.store.finish(
            job_id,
            "done",
            message=(
                f"Hoàn tất {_human(local_size)} lên {', '.join(self._label(t) for t in done_targets)}"
                + (" • đã dọn bộ đệm" if purged else "")
            ),
        )

    # ---------------- steps ----------------

    def _request_link(self, torrent_id):
        res = self.torbox.request_download_link(torrent_id)
        if not isinstance(res, dict):
            raise RuntimeError("TorBox trả về dữ liệu không hợp lệ")
        data = res.get("data")
        if isinstance(data, str) and data.startswith("http"):
            return data
        if isinstance(data, dict):
            for key in ("url", "link", "download"):
                if isinstance(data.get(key), str) and data[key].startswith("http"):
                    return data[key]
        raise RuntimeError(res.get("detail") or res.get("error") or "TorBox không trả về link tải")

    def _download(self, job_id, url, job_dir, fallback_name=""):
        """Stream the DDL into the staging dir, resuming a partial file if present."""
        req = urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"})
        # The filename has to come from the response: TorBox hands out links whose path
        # is a bare UUID, so deriving it from the URL alone produces an extension-less blob.
        probe_name = self._peek_filename(url)
        filename = probe_name or self._filename_from_url(url) or _safe_component(fallback_name) or f"job_{job_id}.bin"
        dest = job_dir / _safe_component(filename)
        existing = dest.stat().st_size if dest.exists() else 0

        if existing:
            req.add_header("Range", f"bytes={existing}-")

        try:
            resp = urllib.request.urlopen(req, timeout=60)
        except urllib.error.HTTPError as e:
            if existing and e.code in (416, 200):  # stale/unsatisfiable range -> restart clean
                existing = 0
                dest.unlink(missing_ok=True)
                resp = urllib.request.urlopen(
                    urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5"}),
                    timeout=60,
                )
            else:
                raise

        with resp:
            resuming = resp.status == 206
            if existing and not resuming:
                existing = 0
                dest.unlink(missing_ok=True)
            length = int(resp.headers.get("Content-Length") or 0)
            total = (existing + length) if length else 0
            self.store.update(job_id, bytes_total=total, bytes_done=existing)

            mode = "ab" if resuming and existing else "wb"
            done = existing
            last_write = 0.0
            window_bytes, window_start = 0, time.time()

            with open(dest, mode) as f:
                while True:
                    self._check_cancel(job_id)
                    chunk = resp.read(CHUNK)
                    if not chunk:
                        break
                    f.write(chunk)
                    done += len(chunk)
                    window_bytes += len(chunk)

                    now = time.time()
                    if now - last_write >= PROGRESS_INTERVAL:
                        elapsed = max(1e-6, now - window_start)
                        speed = window_bytes / elapsed
                        pct = (done / total * 55.0) if total else 25.0
                        self.store.update(
                            job_id,
                            bytes_done=done,
                            bytes_total=total or done,
                            speed_bps=speed,
                            progress=round(min(58.0, pct), 1),
                            message=f"Đang tải {_human(done)}"
                                    + (f" / {_human(total)}" if total else "")
                                    + f" ({_human(speed)}/s)",
                        )
                        last_write, window_bytes, window_start = now, 0, now

        final = dest.stat().st_size
        if total and final != total:
            raise RuntimeError(f"Tải thiếu dữ liệu: {final:,} / {total:,} bytes")
        if not total:
            # No Content-Length means nothing here can prove the transfer was complete,
            # and a truncated file would still "match" the destination during verify.
            # Refuse to auto-purge later by surfacing it on the job.
            self.store.update(
                job_id,
                message=f"Cảnh báo: máy chủ không khai báo Content-Length, không xác minh được tính toàn vẹn ({_human(final)}).",
            )
        return dest

    def _upload(self, job_id, target, local_file, folder_name, cfg):
        folder = _safe_component(folder_name)
        if target == "drive":
            remote = f"{cfg.get('gdrive_remote', 'gdrive')}:{cfg.get('gdrive_root', 'Phim').strip('/')}/{folder}"
            cmd = [
                self.gdrive.rclone_bin, "--config", self.gdrive.rclone_config,
                "copy", str(local_file), remote,
                f"--transfers={int(cfg.get('sync_transfers', 4))}",
                "--checkers=8", "--stats=1s", "--stats-one-line",
            ]
            self._run_tracked(job_id, cmd, target, self._parse_rclone_progress)
        elif target == "nas":
            host, user = cfg.get("nas_host", ""), cfg.get("nas_user", "admin")
            if not host:
                raise RuntimeError("Chưa cấu hình địa chỉ NAS trong phần Cài Đặt")
            port = str(int(cfg.get("nas_port", 22)))
            key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
            remote_dir = f"{cfg.get('nas_path', '/volume1/video/TV Shows').rstrip('/')}/{folder}"

            ssh_base = ["ssh", "-p", port, "-o", "BatchMode=yes", "-o", "ConnectTimeout=8"]
            if key and os.path.exists(key):
                ssh_base += ["-i", key]
            # shlex.quote keeps a folder name with quotes/spaces from becoming shell syntax.
            mk = subprocess.run(
                ssh_base + [f"{user}@{host}", f"mkdir -p {shlex.quote(remote_dir)}"],
                capture_output=True, text=True, timeout=20,
            )
            if mk.returncode != 0:
                raise RuntimeError(mk.stderr.strip() or "Không tạo được thư mục đích trên NAS")

            # rsync takes the whole ssh invocation as one -e string, so it has to be quoted.
            rsh = "ssh " + " ".join(shlex.quote(c) for c in ssh_base[1:])
            cmd = [
                "rsync", "-az", "--partial", "--info=progress2",
                "-e", rsh,
                str(local_file),
                f"{user}@{host}:{shlex.quote(remote_dir)}/",
            ]
            self._run_tracked(job_id, cmd, target, self._parse_rsync_progress)
        else:
            raise RuntimeError(f"Đích đồng bộ không hỗ trợ: {target}")

    def _run_tracked(self, job_id, cmd, target, parser):
        proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                text=True, bufsize=1)
        self._register_proc(job_id, proc)
        tail = []
        try:
            for line in proc.stdout:
                tail.append(line.rstrip())
                del tail[:-15]
                if self.store.is_cancel_requested(job_id) or self._stop.is_set():
                    proc.terminate()
                    raise CancelledError("cancelled during upload")
                pct = parser(line)
                if pct is not None:
                    self.store.update(
                        job_id,
                        message=f"Đang đẩy lên {self._label(target)}: {pct:.0f}%",
                    )
            proc.wait(timeout=30)
        finally:
            self._unregister_proc(job_id)
        if proc.returncode != 0:
            raise RuntimeError("\n".join(tail[-4:]) or f"lệnh thoát với mã {proc.returncode}")

    @staticmethod
    def _parse_rclone_progress(line):
        m = re.search(r"(\d+)%", line)
        return float(m.group(1)) if m else None

    @staticmethod
    def _parse_rsync_progress(line):
        m = re.search(r"\s(\d+)%\s", line)
        return float(m.group(1)) if m else None

    def _remote_size(self, target, filename, folder_name, cfg):
        folder = _safe_component(folder_name)
        if target == "drive":
            remote = f"{cfg.get('gdrive_remote', 'gdrive')}:{cfg.get('gdrive_root', 'Phim').strip('/')}/{folder}"
            res = subprocess.run(
                [self.gdrive.rclone_bin, "--config", self.gdrive.rclone_config, "lsjson", remote],
                capture_output=True, text=True, timeout=60,
            )
            if res.returncode != 0:
                return None
            try:
                for item in json.loads(res.stdout):
                    if item.get("Name") == filename and not item.get("IsDir"):
                        return int(item.get("Size", -1))
            except Exception:
                return None
            return None

        if target == "nas":
            host, user = cfg.get("nas_host", ""), cfg.get("nas_user", "admin")
            port = str(int(cfg.get("nas_port", 22)))
            key = os.path.expanduser(cfg.get("nas_ssh_key") or "")
            remote_file = f"{cfg.get('nas_path', '/volume1/video/TV Shows').rstrip('/')}/{folder}/{filename}"
            ssh_cmd = ["ssh", "-p", port, "-o", "BatchMode=yes", "-o", "ConnectTimeout=8"]
            if key and os.path.exists(key):
                ssh_cmd += ["-i", key]
            # `wc -c` avoids the stat(1) flag differences between BSD and GNU userlands.
            ssh_cmd += [f"{user}@{host}", f"wc -c < {shlex.quote(remote_file)}"]
            res = subprocess.run(ssh_cmd, capture_output=True, text=True, timeout=30)
            if res.returncode != 0:
                return None
            try:
                return int(res.stdout.strip())
            except Exception:
                return None
        return None

    # ---------------- misc ----------------

    @staticmethod
    def _label(target):
        return {"drive": "Google Drive", "nas": "NAS Storage"}.get(target, target)

    @staticmethod
    def _peek_filename(url):
        """Ask the server for the real filename via Content-Disposition (HEAD, then a
        ranged GET for hosts that reject HEAD)."""
        for req in (
            urllib.request.Request(url, method="HEAD",
                                   headers={"User-Agent": "Antigravity-MediaHub/2.5"}),
            urllib.request.Request(url, headers={"User-Agent": "Antigravity-MediaHub/2.5",
                                                 "Range": "bytes=0-0"}),
        ):
            try:
                with urllib.request.urlopen(req, timeout=20) as r:
                    cd = r.headers.get("Content-Disposition") or ""
                m = re.search(r"filename\*=UTF-8''([^;\r\n]+)", cd) or \
                    re.search(r'filename="([^"]+)"', cd) or \
                    re.search(r"filename=([^;\r\n]+)", cd)
                if m:
                    import urllib.parse as up
                    return up.unquote(m.group(1)).strip().strip('"')
            except Exception:
                continue
        return None

    @staticmethod
    def _filename_from_url(url):
        try:
            import urllib.parse as up
            path = up.urlparse(url).path
            return up.unquote(os.path.basename(path)) or None
        except Exception:
            return None

    @staticmethod
    def _cleanup_staging(job):
        path = job.get("staging_path")
        if not path:
            return False
        p = Path(path)
        # Refuse to delete anything that is not a directory we created under staging.
        if not p.is_dir() or p == p.anchor or str(p) in ("/", os.path.expanduser("~")):
            return False
        try:
            shutil.rmtree(p)
            return True
        except Exception as e:
            print(f"[SyncWorker] Không dọn được bộ đệm {p}: {e}")
            return False
