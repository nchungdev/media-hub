#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Skill-Scoped Intent Router & Command Dispatcher
Strictly restricts AI Agent actions within registered Media Hub skills.

Every branch answers from live sources (TorBox API, the sync job store, the Google
Drive listing, the local filesystem). It previously returned fixed strings such as
"49/51 tập - 96%" regardless of the real state, which made the assistant confidently
wrong. When a source is unreachable the reply now says so.
"""

import os
import json
import time
import shutil

from core.settings import load_unified_settings

SKILLS_MAP = {
    "TORBOX_OP": {
        "skill": "torbox-manager",
        "description": "Quản lý TorBox Cloud Cache (tra cứu, lọc ready/queued, lấy link, dọn slot)",
        "keywords": ["torbox", "torrent", "magnet", "cache", "seed", "tải về torbox", "sẵn sàng", "task trong torbox", "ready", "tải về"]
    },
    "PIPELINE_OP": {
        "skill": "sequential-pipeline",
        "description": "Theo dõi và điều phối tiến trình stream TorBox -> Google Drive",
        "keywords": ["tiến độ", "tiến trình", "pipeline", "đồng bộ", "sync", "đang chạy", "hàng đợi", "hoàn thành", "job"]
    },
    "GDRIVE_OP": {
        "skill": "media-collector",
        "description": "Quản lý kho Google Drive Plex/Jellyfin, kiểm tra show, phân mùa tập",
        "keywords": ["google drive", "gdrive", "thư viện", "plex", "jellyfin", "quét", "drive", "series", "season", "tập", "phim"]
    },
    "SUBTITLE_OP": {
        "skill": "translate-subtitle",
        "description": "Tra cứu, dịch và chuyển đổi định dạng phụ đề Vietsub/WebVTT",
        "keywords": ["sub", "phụ đề", "vietsub", "dịch", "srt", "vtt", "ass"]
    },
    "SYSTEM_OP": {
        "skill": "media-hub",
        "description": "Kiểm tra dung lượng ổ đĩa, dọn cache đệm",
        "keywords": ["ổ đĩa", "dung lượng", "dọn dẹp", "bộ nhớ", "ram", "disk", "clean", "cache"]
    }
}

def classify_intent(command: str):
    """Pick the intent whose longest matching keyword is the most specific.

    Iterating the map in insertion order let a generic word in an earlier entry win
    over a precise one later ("thư viện drive ..." landed on PIPELINE_OP).
    """
    cmd_lower = command.lower()
    best = None
    for intent, data in SKILLS_MAP.items():
        for kw in data["keywords"]:
            if kw in cmd_lower and (best is None or len(kw) > best[0]):
                best = (len(kw), intent, data["skill"])
    if best is None:
        return "OUT_OF_SCOPE", None
    return best[1], best[2]

def _reply(intent, skill, text):
    return {"status": "done", "intent": intent, "skill": skill, "response": text}


def _fmt(n):
    n = float(n or 0)
    for u in ("B", "KB", "MB", "GB", "TB"):
        if abs(n) < 1024.0:
            return f"{n:.1f} {u}"
        n /= 1024.0
    return f"{n:.1f} PB"


def execute_scoped_command(command: str):
    intent, skill = classify_intent(command)
    cmd_lower = command.lower()

    if intent == "OUT_OF_SCOPE":
        return _reply(intent, None,
            "⚠️ **Yêu cầu ngoài phạm vi:** Lệnh này không thuộc các Skill được hỗ trợ. "
            "AI Agent Media Hub chỉ tiếp nhận các yêu cầu điều phối thuộc 4 Skill: "
            "**torbox-manager** (Quản lý TorBox), **sequential-pipeline** (Tiến trình tải phim), "
            "**media-collector** (Thư viện Google Drive) và **translate-subtitle** (Phụ đề Vietsub).")

    if intent == "TORBOX_OP":
        try:
            from core.torbox_manager import TorBoxManager
            res = TorBoxManager().list_torrents()
            if not res.get("success"):
                return _reply(intent, skill,
                    f"⚡ **[torbox-manager]** Không truy vấn được TorBox: {res.get('error') or 'lỗi không rõ'}.")
            torrents = res.get("data", [])
            ready = [t for t in torrents if t.get("download_state") in ("completed", "cached")]
            queued = [t for t in torrents if t.get("is_queued") or t.get("download_state") == "queued"]
            if any(k in cmd_lower for k in ("khả năng", "sẵn sàng", "ready", "tải về")):
                lines = [f"⚡ **[torbox-manager]** {len(ready)}/{len(torrents)} torrents đã cache xong, sẵn sàng tải:", ""]
                for t in ready[:6]:
                    name = t.get("name", "N/A")
                    lines.append(f"• **{name[:39] + '...' if len(name) > 42 else name}** ({_fmt(t.get('size', 0))})")
                if len(ready) > 6:
                    lines.append(f"\n*... và {len(ready) - 6} torrents sẵn sàng khác.*")
                return _reply(intent, skill, "\n".join(lines))
            return _reply(intent, skill,
                f"⚡ **[torbox-manager]** Tổng {len(torrents)} torrents "
                f"({len(ready)} Ready/Cached, {len(queued)} Queued).")
        except Exception as e:
            return _reply(intent, skill, f"⚡ **[torbox-manager]** Lỗi khi truy vấn TorBox: {e}")

    if intent == "PIPELINE_OP":
        try:
            from core.job_store import JobStore
            store = JobStore()
            active, recent = store.list_active(), store.list_recent(limit=8)
            counts = store.counts()
            if not active:
                done = [j for j in recent if j["status"] == "done"]
                if not done:
                    return _reply(intent, skill,
                        "🚀 **[sequential-pipeline]** Hiện không có tác vụ đồng bộ nào đang chạy và "
                        "chưa có tác vụ nào hoàn tất được ghi nhận.")
                lines = ["🚀 **[sequential-pipeline]** Không có tác vụ đang chạy. Gần đây nhất:", ""]
                for j in done[:5]:
                    lines.append(f"• **{j['name'] or j['torrent_id']}** — {_fmt(j['bytes_total'])} "
                                 f"➔ {', '.join(j['done_targets']) or 'n/a'}")
                return _reply(intent, skill, "\n".join(lines))
            lines = [f"🚀 **[sequential-pipeline]** {len(active)} tác vụ đang chạy:", ""]
            for j in active:
                lines.append(f"• **{j['name'] or j['torrent_id']}** — {j['phase']} {j['progress']:.0f}% "
                             f"➔ {', '.join(j['targets'])}")
                if j["message"]:
                    lines.append(f"  _{j['message']}_")
            if counts:
                lines.append("\n*Tổng: " + ", ".join(f"{v} {k}" for k, v in counts.items()) + "*")
            return _reply(intent, skill, "\n".join(lines))
        except Exception as e:
            return _reply(intent, skill, f"🚀 **[sequential-pipeline]** Không đọc được hàng đợi: {e}")

    if intent == "GDRIVE_OP":
        try:
            from core.gdrive_manager import GDriveManager
            shows = GDriveManager().list_tv_shows()
            if not shows:
                return _reply(intent, skill,
                    "📁 **[media-collector]** Không liệt kê được thư viện Google Drive "
                    "(kiểm tra cấu hình rclone).")
            hits = [s["name"] for s in shows
                    if any(w in s["name"].lower() for w in cmd_lower.split() if len(w) > 3)]
            if hits:
                lines = [f"📁 **[media-collector]** Tìm thấy {len(hits)} mục khớp trên Google Drive:", ""]
                lines += [f"• `{h}`" for h in hits[:10]]
                return _reply(intent, skill, "\n".join(lines))
            return _reply(intent, skill,
                f"📁 **[media-collector]** Thư viện Google Drive hiện có **{len(shows)} shows**. "
                "Nêu tên phim cụ thể để tra cứu chi tiết.")
        except Exception as e:
            return _reply(intent, skill, f"📁 **[media-collector]** Lỗi khi quét Google Drive: {e}")

    if intent == "SUBTITLE_OP":
        # Reporting "đã có Vietsub đầy đủ" without checking would be a guess; point at
        # the concrete tools instead.
        return _reply(intent, skill,
            "💬 **[translate-subtitle]** Chưa quét phụ đề tự động trong phiên này. "
            "Dùng tab **Phụ Đề** trên dashboard để liệt kê file trong bộ đệm, hoặc nêu tên "
            "phim/tập cụ thể để tôi kiểm tra phụ đề đi kèm trên Google Drive.")

    if intent == "SYSTEM_OP":
        try:
            cfg = load_unified_settings()
            staging = cfg.get("staging_dir", "")
            probe = staging if os.path.exists(staging) else "/"
            usage = shutil.disk_usage(probe)
            pct = int(usage.used / (usage.used + usage.free) * 100) if (usage.used + usage.free) else 0
            return _reply(intent, skill,
                f"🧹 **[media-hub]** Ổ đệm `{probe}`: còn **{_fmt(usage.free)}** trống "
                f"trên tổng {_fmt(usage.total)} (đã dùng {pct}%). "
                f"Auto-purge: {'bật' if cfg.get('auto_purge', True) else 'tắt'}.")
        except Exception as e:
            return _reply(intent, skill, f"🧹 **[media-hub]** Không đọc được dung lượng ổ đĩa: {e}")

    return _reply(intent, skill, "✓ Đã thực thi lệnh thành công.")
