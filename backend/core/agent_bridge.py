import os
import json
import time
import subprocess
import threading
import collections
from pathlib import Path
from core.quota_guard import quota_guard

# Kept alongside the other hub state instead of a hard-coded path on an external
# volume: that directory can be unmounted or removed, and the previous constant
# crashed the whole server at import time when it went away.
def default_queue_file():
    from core.settings import resolve_dirs, load_unified_settings
    return resolve_dirs(load_unified_settings(), create=True)["queue_path"]


QUEUE_FILE = None  # resolved per instance; see default_queue_file()


class AgentBridge:
    def __init__(self, queue_file=None):
        self.queue_file = queue_file or os.environ.get("MEDIA_HUB_QUEUE_FILE") or default_queue_file()
        self._worker_thread = None
        self._worker_lock = threading.Lock()
        self.live_logs = collections.deque(maxlen=1000)
        self.active_job = None
        self._current_proc = None
        self._stop_requested = False
        # Create lazily and never let a bad path take the server down on import.
        try:
            Path(self.queue_file).parent.mkdir(parents=True, exist_ok=True)
            if not os.path.exists(self.queue_file):
                self._save([])
            else:
                # Recover any interrupted 'processing' items back to 'pending'
                queue = self._load()
                changed = False
                for item in queue:
                    if item.get("status") == "processing":
                        item["status"] = "pending"
                        item["response"] = "🔄 Đang tiếp tục xử lý..."
                        changed = True
                if changed:
                    self._save(queue)
            # Resume any pending items on server start
            self._trigger_worker()
        except Exception as e:
            print(f"[AgentBridge] Không khởi tạo được hàng đợi ({self.queue_file}): {e}")

    def stop_current_job(self):
        """Kill the currently running agy/agy2 subprocess and all its children, cancel active job."""
        self._stop_requested = True
        proc = self._current_proc
        if proc and proc.poll() is None:
            try:
                import signal
                try:
                    os.killpg(os.getpgid(proc.pid), signal.SIGTERM)
                except Exception:
                    proc.terminate()
                try:
                    proc.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    try:
                        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
                    except Exception:
                        proc.kill()
                self.log_live("🛑 Đã dừng tiến trình CLI theo yêu cầu người dùng.", "warning")
                print("[AgentBridge] 🛑 User requested stop — process terminated.", flush=True)
            except Exception as e:
                self.log_live(f"⚠️ Lỗi khi dừng tiến trình: {e}", "error")
            self._current_proc = None
        # Mark all processing items as cancelled
        queue = self._load()
        changed = False
        for item in queue:
            if item.get("status") == "processing":
                item["status"] = "cancelled"
                item["response"] = "🛑 Đã huỷ bởi người dùng."
                changed = True
        if changed:
            self._save(queue)
        self.active_job = None
        self._save_service_state({"status": "idle", "cli_pid": None})
        return True

    def resume_queue(self):
        """Re-trigger the worker to process any pending items in the queue."""
        self._stop_requested = False
        self._trigger_worker()
        self.log_live("▶️ Đã kích hoạt lại hàng đợi CLI.", "system")
        return True


    def log_live(self, text, level="info"):
        entry = {
            "time": time.strftime("%H:%M:%S"),
            "text": str(text),
            "level": level
        }
        self.live_logs.append(entry)

    def get_live_logs(self):
        return {
            "is_running": self.active_job is not None,
            "active_job": self.active_job,
            "logs": list(self.live_logs)
        }

    def clear_live_logs(self):
        self.live_logs.clear()
        self.log_live("🧹 Đã xoá sạch lịch sử console log.", "system")

    def _get_service_file(self):
        s_dir = Path.home() / ".media-hub"
        s_dir.mkdir(parents=True, exist_ok=True)
        return s_dir / "cli_service.json"

    def _load_service_state(self):
        try:
            sf = self._get_service_file()
            if sf.exists():
                with open(sf, "r", encoding="utf-8") as f:
                    return json.load(f)
        except Exception:
            pass
        return {}

    def _save_service_state(self, data):
        try:
            sf = self._get_service_file()
            with open(sf, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"[AgentBridge] Lỗi lưu cli_service.json: {e}")

    def _is_pid_alive(self, pid):
        if not pid:
            return False
        try:
            os.kill(int(pid), 0)
            return True
        except (OSError, ProcessLookupError, ValueError):
            return False

    def ensure_service(self):
        """
        Check if CLI agent process is running.
        If running, ATTACH to it seamlessly without spawning a new process.
        If idle, return ready state and resume any pending queue tasks.
        """
        state = self._load_service_state()
        pid = state.get("cli_pid")

        if pid and self._is_pid_alive(pid):
            # Already running in background: ATTACH!
            if not self.active_job:
                self.active_job = {
                    "media_id": state.get("media_id", "media-hub-general"),
                    "showTitle": state.get("showTitle", ""),
                    "command": state.get("command", ""),
                    "cli": state.get("cli", "agy"),
                    "start_time": state.get("start_time", ""),
                    "conversation_id": state.get("conversation_id", ""),
                    "pid": pid,
                    "status": "attached"
                }
                self.log_live(f"🔗 Đã gắn kết (Attached) vào tiến trình CLI đang chạy ngầm [PID: {pid}]", "system")
            return {
                "status": "attached",
                "message": f"Đã gắn kết vào tiến trình CLI đang hoạt động (PID: {pid})",
                "active_job": self.active_job,
                "is_running": True
            }

        # If process died or not running, clear active_job if it had old pid
        if self.active_job and self.active_job.get("pid") and not self._is_pid_alive(self.active_job["pid"]):
            self.active_job = None
            self._save_service_state({"status": "idle", "cli_pid": None})

        # Ensure worker is triggered if there are pending jobs
        self._trigger_worker()

        is_running = self.active_job is not None
        return {
            "status": "running" if is_running else "ready",
            "message": "Tiến trình CLI đang xử lý" if is_running else "Dịch vụ CLI sẵn sàng",
            "active_job": self.active_job,
            "is_running": is_running
        }

    def _load(self):
        try:
            with open(self.queue_file, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception:
            return []

    def _save(self, data):
        try:
            with open(self.queue_file, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"[AgentBridge] Lỗi lưu queue: {e}")

    def _get_sessions_file(self):
        return Path(self.queue_file).parent / ".media_sessions.json"

    def _load_media_sessions(self):
        try:
            s_file = self._get_sessions_file()
            if os.path.exists(s_file):
                with open(s_file, "r", encoding="utf-8") as f:
                    return json.load(f)
        except Exception:
            pass
        return {}

    def _save_media_sessions(self, data):
        try:
            s_file = self._get_sessions_file()
            with open(s_file, "w", encoding="utf-8") as f:
                json.dump(data, f, indent=2, ensure_ascii=False)
        except Exception as e:
            print(f"[AgentBridge] Lỗi lưu media sessions: {e}")

    def clear_media_session(self, media_id):
        sessions = self._load_media_sessions()
        if media_id in sessions:
            del sessions[media_id]
            self._save_media_sessions(sessions)
            print(f"[AgentBridge] 🧹 Đã giải phóng session context cho {media_id}")

    def _extract_media_id(self, command_text, explicit_id=None):
        if explicit_id and str(explicit_id).strip():
            return str(explicit_id).strip()

        import re
        cmd_lower = (command_text or "").lower()

        # 1. Detect dedicated skills context
        if any(k in cmd_lower for k in ["torbox", "download", "tải phim", "tải xuống", "magnet", "aria2", "debrid"]):
            return "skill-media-downloader"
        elif any(k in cmd_lower for k in ["sync", "đồng bộ", "rclone", "sftp", "nas storage", "google drive"]):
            return "skill-media-sync"
        elif any(k in cmd_lower for k in ["librarian", "cross_check", "đối chiếu", "thư viện cloud"]):
            return "skill-cloud-librarian"
        elif any(k in cmd_lower for k in ["bóc tách", "sub nhúng", "extract sub", "muxed sub"]):
            return "skill-subtitle-extractor"
        elif any(k in cmd_lower for k in ["tmdb", "tra cứu phim", "poster metadata"]):
            return "skill-tmdb-lookup"

        # 2. Detect TVDB / TMDB ID patterns
        tvdb_match = re.search(r'tvdb[-_](\d+)', command_text, re.IGNORECASE)
        if tvdb_match:
            return f"media-tvdb-{tvdb_match.group(1)}"
        
        tmdb_match = re.search(r'tmdb[-_](\d+)', command_text, re.IGNORECASE)
        if tmdb_match:
            return f"media-tmdb-{tmdb_match.group(1)}"

        # 3. Dynamic Show Matching from Workspace
        from core.settings import load_unified_settings
        try:
            cfg = load_unified_settings()
            hub_home = cfg.get("media_hub_home") or os.getcwd()
            ws_dir = str(Path(hub_home).parent) if os.path.basename(hub_home) == ".media-hub" else str(Path(hub_home))
            
            show_candidates = []
            for folder in [Path(ws_dir) / "TV Shows", Path(ws_dir) / "Movies", Path(ws_dir)]:
                if folder.exists():
                    for d in folder.iterdir():
                        if d.is_dir() and not d.name.startswith("."):
                            show_candidates.append(d.name)
            
            for cname in sorted(show_candidates, key=len, reverse=True):
                clean_name = re.sub(r'\(\d{4}\)|\[.*?\]|\{.*?\}', '', cname).strip().lower()
                if clean_name and len(clean_name) >= 3 and clean_name in cmd_lower:
                    slug = re.sub(r'[^a-z0-9]+', '-', re.sub(r'\[.*?\]|\{.*?\}', '', cname).lower()).strip('-')
                    return f"media-show-{slug}"
        except Exception:
            pass

        # 4. Fallbacks for well-known series
        if "monster" in cmd_lower:
            return "media-tvdb-74599"
        elif "wataru" in cmd_lower:
            return "media-tvdb-446736"
        elif any(k in cmd_lower for k in ["three-eyed", "three eyed", "3 mắt", "sharaku"]):
            return "media-tvdb-320122"
        elif "conan" in cmd_lower:
            return "media-tvdb-72454"
        elif "black jack" in cmd_lower:
            return "media-tvdb-78832"

        return "media-hub-general"

    def _build_context_prompt(self, command_text, media_id, workspace_dir):
        """
        Picks and attaches relevant project context (Glossary, Progress, Skills)
        when starting a new conversation, or lets existing session resume seamlessly.
        """
        sessions = self._load_media_sessions()
        if sessions.get(media_id):
            # Already has existing conversation session — agy will resume with --conversation,
            # retaining 100% of memory and previous turns!
            return command_text

        context_lines = []
        if media_id.startswith("skill-"):
            skill_name = media_id.replace("skill-", "")
            context_lines.append(f"[KÍCH HOẠT KỸ NĂNG CHUYÊN BIỆT: {skill_name}]")
            skill_path = Path(workspace_dir) / ".agents" / "skills" / skill_name / "SKILL.md"
            if skill_path.exists():
                context_lines.append(f"Tài liệu hướng dẫn skill: {skill_path}")
        elif media_id.startswith("media-"):
            context_lines.append(f"[NGỮ CẢNH DỰ ÁN MEDIA: {media_id}]")
            ws_path = Path(workspace_dir)
            for g in ws_path.rglob("GLOSSARY.md"):
                context_lines.append(f"Bảng thuật ngữ chuẩn (Glossary): {g}")
                break
            for p in ws_path.rglob("PROGRESS.md"):
                context_lines.append(f"Nhật ký tiến độ dự án (Progress): {p}")
                break
            context_lines.append("Quy tắc định dạng phụ đề: Xuất đủ 3 file (.vi.ass typography 1080p, .vi.srt, .vi.vtt stream zero-latency).")

        if context_lines:
            header = "\n".join(context_lines)
            return f"{header}\n\n---\n\n{command_text}"
        return command_text

    def _get_brain_dirs(self, cli_bin):
        if "agy2" in str(cli_bin):
            return [
                Path.home() / ".antigravity-instances" / "secondary" / ".gemini" / "antigravity-cli" / "brain",
                Path.home() / ".antigravity-instances" / "secondary" / ".gemini" / "antigravity" / "brain",
            ]
        return [
            Path.home() / ".gemini" / "antigravity-cli" / "brain",
            Path.home() / ".gemini" / "antigravity" / "brain",
        ]

    def _get_brain_dir(self, cli_bin):
        dirs = self._get_brain_dirs(cli_bin)
        for d in dirs:
            if d.exists():
                return d
        return dirs[0]

    def _process_transcript_step(self, step, agent_label=""):
        stype = step.get("type")
        thinking = step.get("thinking")
        tool_calls = step.get("tool_calls")
        content = step.get("content")

        pfx = f"[{agent_label}] " if agent_label else ""

        # 1. Thinking / Internal Reasoning
        if thinking and str(thinking).strip():
            th_text = str(thinking).strip()
            lines = [l.strip() for l in th_text.split("\n") if l.strip()]
            for l in lines:
                clean_l = l.lstrip("#*-> ").strip()
                if clean_l and not clean_l.startswith("```"):
                    self.log_live(f"{pfx}🧠 [Thinking] {clean_l}", "thinking")

        # 2. Agent Content / Progress / Step Announcements / Tool Outputs
        if content and str(content).strip():
            c_str = str(content).strip()
            if "Created At:" in c_str or "The command exited" in c_str or "File Path:" in c_str:
                # Strip metadata headers to get actual command/tool output
                lines = c_str.split("\n")
                actual_lines = []
                in_body = False
                for l in lines:
                    l_strip = l.strip()
                    if not in_body:
                        if any(l_strip.startswith(h) for h in [
                            "Created At:", "Completed At:", "File Path:", "Total Lines:",
                            "Total Bytes:", "Showing lines", "The following code",
                            "The lines of the file are", "Content is limited to"
                        ]):
                            continue
                        elif l_strip.startswith("The command exited with code") or l_strip.startswith("Output:"):
                            in_body = True
                            continue
                        else:
                            in_body = True
                    if l_strip:
                        import re
                        clean = re.sub(r'^\d+:\s*', '', l_strip)
                        if clean and not clean.startswith('{') and not clean.startswith('['):
                            actual_lines.append(clean)

                for al in actual_lines[:6]:
                    if len(al) > 140:
                        al = al[:140] + "..."
                    self.log_live(f"{pfx}  ↳ {al}", "output")
            else:
                # Model dialogue or step announcements
                for l in c_str.split("\n"):
                    l_clean = l.strip()
                    if not l_clean or l_clean.startswith("```"):
                        continue
                    if any(l_clean.startswith(tag) for tag in [
                        "[BƯỚC", "BƯỚC", "[STEP", "Step", "✅", "🚀", "⏳", "📝", "🔍", "⚡", "📌", "✨"
                    ]):
                        self.log_live(f"{pfx}📌 {l_clean}", "system")
                    elif len(l_clean) > 3:
                        if len(l_clean) > 140:
                            l_clean = l_clean[:140] + "..."
                        self.log_live(f"{pfx}💬 {l_clean}", "info")

        # 3. Tool Calls
        if tool_calls:
            for tc in tool_calls:
                name = tc.get("name", "tool")
                args = tc.get("args") or tc.get("parameters") or {}
                act = args.get("toolAction") or args.get("toolSummary") or args.get("Description") or args.get("Instruction") or ""
                act = str(act).strip('"\' ')

                if name == "run_command":
                    cmd = str(args.get("CommandLine", "")).strip()
                    if "\n" in cmd:
                        cmd = cmd.split("\n")[0] + "..."
                    self.log_live(f"{pfx}⚡ [Run Command] {act} $ {cmd[:140]}", "tool")
                elif name in ["write_to_file", "replace_file_content"]:
                    tf = os.path.basename(str(args.get("TargetFile") or args.get("AbsolutePath") or args.get("Path") or "").strip('"\' '))
                    self.log_live(f"{pfx}📝 [{act or 'Chỉnh sửa tệp'}] {name} -> {tf}", "tool")
                elif name == "invoke_subagent":
                    subagents = args.get("Subagents", [])
                    roles = [s.get("Role", "Subagent") for s in subagents]
                    self.log_live(f"{pfx}🤖 [Spawn Subagent] {', '.join(roles)}", "subagent")
                elif name == "view_file":
                    vf = os.path.basename(str(args.get("AbsolutePath", "")).strip('"\' '))
                    self.log_live(f"{pfx}🔍 [Đọc Tệp Tin] {act} -> {vf}", "tool")
                elif name == "grep_search":
                    q = str(args.get("Query", "")).strip('"\' ')
                    self.log_live(f"{pfx}🔎 [Tìm Kiếm Mã Nguồn] Pattern: '{q}'", "tool")
                elif name == "search_web":
                    q = str(args.get("query", "")).strip('"\' ')
                    self.log_live(f"{pfx}🌐 [Tìm Kiếm Web] '{q}'", "tool")
                elif name == "send_message":
                    rec = args.get("Recipient", "")
                    self.log_live(f"{pfx}✉️ [Gửi Tin Nhắn Cho Agent] Recipient: {rec}", "tool")
                else:
                    summary = args.get("toolSummary") or args.get("toolAction") or name
                    self.log_live(f"{pfx}⚙️ [Thực Thi Công Cụ] {summary}", "tool")

    def _tail_transcript(self, brain_dirs, conv_id_hint, start_time, stop_event, on_conv_discovered=None):
        """Realtime transcript tailer: streams thinking, tool calls, execution steps and subagents to live console."""
        conv_id = conv_id_hint
        transcript_path = None
        seen_lines = 0

        # Step 1: Discover active conversation
        start_ts = time.time()
        while not stop_event.is_set() and time.time() - start_ts < 25:
            if conv_id:
                for bd in brain_dirs:
                    candidate = Path(bd) / conv_id / ".system_generated" / "logs" / "transcript.jsonl"
                    if candidate.exists():
                        transcript_path = candidate
                        break
                if transcript_path:
                    break
            else:
                for bd in brain_dirs:
                    if Path(bd).exists():
                        try:
                            dirs = [d for d in Path(bd).iterdir() if d.is_dir() and not d.name.startswith(".")]
                            if dirs:
                                dirs.sort(key=lambda d: d.stat().st_mtime, reverse=True)
                                latest = dirs[0]
                                if latest.stat().st_mtime >= start_time - 10:
                                    cand = latest / ".system_generated" / "logs" / "transcript.jsonl"
                                    if cand.exists():
                                        conv_id = latest.name
                                        transcript_path = cand
                                        if on_conv_discovered:
                                            on_conv_discovered(conv_id)
                                        break
                        except Exception:
                            pass
                if transcript_path:
                    break
            time.sleep(0.5)

        if not transcript_path or not transcript_path.exists():
            return

        tracked_subagents = set()

        def start_subagent_tailer(sub_cid, role):
            if sub_cid in tracked_subagents:
                return
            tracked_subagents.add(sub_cid)
            
            def sub_tail():
                sub_path = None
                for _ in range(30):
                    if stop_event.is_set():
                        return
                    for bd in brain_dirs:
                        candidate = Path(bd) / sub_cid / ".system_generated" / "logs" / "transcript.jsonl"
                        if candidate.exists():
                            sub_path = candidate
                            break
                    if sub_path:
                        break
                    time.sleep(0.5)

                if not sub_path:
                    return

                try:
                    with open(sub_path, "r", encoding="utf-8") as sf:
                        while not stop_event.is_set():
                            s_line = sf.readline()
                            if not s_line:
                                time.sleep(0.3)
                                continue
                            s_line = s_line.strip()
                            if s_line:
                                try:
                                    s_step = json.loads(s_line)
                                    self._process_transcript_step(s_step, agent_label=role)
                                except Exception:
                                    pass
                except Exception:
                    pass

            t = threading.Thread(target=sub_tail, daemon=True)
            t.start()

        # Step 2: Tail parent transcript.jsonl
        try:
            with open(transcript_path, "r", encoding="utf-8") as f:
                if conv_id_hint:
                    # Count existing lines in resumed session so we only stream new steps
                    seen_lines = sum(1 for _ in f)
                    f.seek(0)
                    for _ in range(seen_lines):
                        f.readline()

                import re
                subagent_role_map = {}

                while not stop_event.is_set():
                    line = f.readline()
                    if not line:
                        time.sleep(0.3)
                        continue

                    line = line.strip()
                    if not line:
                        continue

                    try:
                        step = json.loads(line)
                        self._process_transcript_step(step)

                        # Detect subagents invoked in tool_calls
                        for tc in step.get("tool_calls", []):
                            if tc.get("name") == "invoke_subagent":
                                s_args = tc.get("args") or tc.get("parameters") or {}
                                for sub in s_args.get("Subagents", []):
                                    r = sub.get("Role") or sub.get("TypeName", "Subagent")
                                    # Save pending role
                                    subagent_role_map["pending"] = r

                        # Detect subagent conversationId in content
                        c_text = str(step.get("content", ""))
                        if "conversationId" in c_text:
                            matches = re.findall(r'"conversationId":\s*"([a-f0-9\-]+)"', c_text)
                            for m in matches:
                                r_label = subagent_role_map.get("pending", "Subagent")
                                start_subagent_tailer(m, r_label)
                    except Exception:
                        pass

                # Read remaining lines after subprocess exits
                for rem_line in f:
                    rem_line = rem_line.strip()
                    if rem_line:
                        try:
                            step = json.loads(rem_line)
                            self._process_transcript_step(step)
                        except Exception:
                            pass
        except Exception as e:
            print(f"[AgentBridge] Lỗi tail transcript: {e}", flush=True)

    def add_command(self, command_text, author="User", media_id=None):
        queue = self._load()
        cmd_id = int(time.time() * 1000)
        resolved_media_id = self._extract_media_id(command_text, media_id)

        # Translation Quota Guard check
        cmd_lower = command_text.lower()
        is_translation = "translate-subtitle" in cmd_lower or "dịch phụ đề" in cmd_lower or "dịch thuật" in cmd_lower
        if is_translation:
            import re
            m_eps = re.findall(r'S\d+E\d+', command_text, re.IGNORECASE)
            req_eps = len(m_eps) if m_eps else (5 if "5 tập" in cmd_lower or "batch" in cmd_lower else 1)
            quota_check = quota_guard.can_translate(requested_episodes=req_eps)
            if not quota_check["allowed"]:
                pause_msg = quota_check["reason"]
                cmd_item = {
                    "id": cmd_id,
                    "command": command_text,
                    "author": author,
                    "media_id": resolved_media_id,
                    "status": "paused_quota",
                    "response": pause_msg,
                    "timestamp": time.strftime("%H:%M")
                }
                queue.append(cmd_item)
                self._save(queue)
                self.log_live(f"🛑 [Quota Guard] {pause_msg}", "warning")
                print(f"[AgentBridge] 🛑 Quota Guard blocked [{resolved_media_id}]: {pause_msg}", flush=True)
                return cmd_item
        
        quick_response = f"🤖 Đã nhận lệnh [{resolved_media_id}], đang kích hoạt Antigravity CLI..."

        cmd_item = {
            "id": cmd_id,
            "command": command_text,
            "author": author,
            "media_id": resolved_media_id,
            "status": "pending",
            "response": quick_response,
            "timestamp": time.strftime("%H:%M")
        }
        queue.append(cmd_item)
        self._save(queue)
        self._trigger_worker()
        return cmd_item

    def update_response(self, cmd_id, response_text, status="done"):
        queue = self._load()
        for item in queue:
            if item.get("id") == cmd_id:
                item["response"] = response_text
                item["status"] = status
                break
        self._save(queue)

    def list_commands(self):
        return self._load()

    def _trigger_worker(self):
        with self._worker_lock:
            if self._worker_thread is None or not self._worker_thread.is_alive():
                self._worker_thread = threading.Thread(target=self._worker_loop, daemon=True)
                self._worker_thread.start()

    def get_token_usage_report(self):
        sessions = self._load_media_sessions()
        brain_dirs = [
            Path.home() / ".antigravity-instances" / "secondary" / ".gemini" / "antigravity-cli" / "brain",
            Path.home() / ".gemini" / "antigravity-cli" / "brain"
        ]

        def estimate_tokens(text):
            if not text:
                return 0
            return max(1, int(len(str(text)) / 3.5))

        media_reports = []
        total_in = 0
        total_out = 0
        total_thinking = 0
        total_cost = 0.0

        for media_id, conv_id in sessions.items():
            stats = {
                "media_id": media_id,
                "conv_id": conv_id,
                "turns": 0,
                "input_tokens": 0,
                "output_tokens": 0,
                "thinking_tokens": 0,
                "tool_calls": 0,
                "est_cost_usd": 0.0,
                "last_active": ""
            }
            found = False
            for b in brain_dirs:
                t_path = b / conv_id / ".system_generated" / "logs" / "transcript.jsonl"
                if t_path.exists():
                    found = True
                    try:
                        stats["last_active"] = time.strftime("%Y-%m-%d %H:%M", time.localtime(os.path.getmtime(t_path)))
                        with open(t_path, "r", encoding="utf-8") as tf:
                            for line in tf:
                                try:
                                    step = json.loads(line)
                                    stype = step.get("type")
                                    if stype == "USER_INPUT":
                                        stats["turns"] += 1
                                        stats["input_tokens"] += estimate_tokens(step.get("content", ""))
                                    elif stype == "PLANNER_RESPONSE":
                                        stats["output_tokens"] += estimate_tokens(step.get("content", ""))
                                        stats["thinking_tokens"] += estimate_tokens(step.get("thinking", ""))
                                        if "tool_calls" in step:
                                            stats["tool_calls"] += len(step["tool_calls"])
                                            for tc in step["tool_calls"]:
                                                stats["output_tokens"] += estimate_tokens(str(tc))
                                    elif stype in ["GENERIC", "SYSTEM_MESSAGE"]:
                                        stats["input_tokens"] += estimate_tokens(step.get("content", ""))
                                except Exception:
                                    pass
                    except Exception as e:
                        print(f"[AgentBridge] Token read error ({conv_id}): {e}")
                    break

            tin = stats["input_tokens"]
            tout = stats["output_tokens"] + stats["thinking_tokens"]
            cost = (tin * 0.075 / 1e6) + (tout * 0.30 / 1e6)
            stats["est_cost_usd"] = round(cost, 6)
            stats["total_tokens"] = tin + tout

            total_in += tin
            total_out += tout
            total_thinking += stats["thinking_tokens"]
            total_cost += cost

            media_reports.append(stats)

        return {
            "total_sessions": len(media_reports),
            "total_input_tokens": total_in,
            "total_output_tokens": total_out,
            "total_thinking_tokens": total_thinking,
            "total_tokens": total_in + total_out,
            "total_cost_usd": round(total_cost, 6),
            "sessions": sorted(media_reports, key=lambda x: x["total_tokens"], reverse=True)
        }

    def _worker_loop(self):
        while True:
            queue = self._load()
            pending_item = None
            for item in queue:
                if item.get("status") == "pending":
                    pending_item = item
                    break

            if not pending_item:
                self.active_job = None
                break

            cmd_id = pending_item.get("id")
            cmd_text = pending_item.get("command", "").strip()
            media_id = pending_item.get("media_id") or self._extract_media_id(cmd_text)

            # Translation Quota Guard Check
            cmd_lower = cmd_text.lower()
            if "translate-subtitle" in cmd_lower or "dịch phụ đề" in cmd_lower or "dịch thuật" in cmd_lower:
                import re
                m_eps = re.findall(r'S\d+E\d+', cmd_text, re.IGNORECASE)
                req_eps = len(m_eps) if m_eps else (5 if "5 tập" in cmd_lower or "batch" in cmd_lower else 1)
                quota_check = quota_guard.can_translate(requested_episodes=req_eps)
                if not quota_check["allowed"]:
                    self.update_response(cmd_id, quota_check["reason"], status="paused_quota")
                    self.log_live(f"🛑 [Quota Guard] {quota_check['reason']}", "warning")
                    print(f"[AgentBridge] 🛑 Quota Guard paused command #{cmd_id}: {quota_check['reason']}", flush=True)
                    continue

            self.update_response(cmd_id, f"⏳ Đang kết nối Antigravity CLI cho [{media_id}]...", status="processing")

            from core.settings import load_unified_settings
            cfg = load_unified_settings()
            hub_home = cfg.get("media_hub_home") or os.getcwd()
            workspace_dir = str(Path(hub_home).parent) if os.path.basename(hub_home) == ".media-hub" else str(Path(hub_home))
            if not os.path.exists(workspace_dir):
                workspace_dir = os.getcwd()
            cli_pref = str(cfg.get("agy_cli_profile", "auto")).lower().strip()

            if cli_pref == "agy":
                cli_candidates = [
                    "/Users/chungnh/.local/bin/agy",
                    "/Users/chungnh/.local/bin/agy2",
                ]
            else:
                cli_candidates = [
                    "/Users/chungnh/.local/bin/agy2",
                    "/Users/chungnh/.local/bin/agy",
                ]

            available_clis = [c for c in cli_candidates if os.path.exists(c) and os.access(c, os.X_OK)]
            if not available_clis:
                available_clis = ["agy"]

            env = dict(os.environ)
            env["PATH"] = "/Users/chungnh/.local/bin:/opt/homebrew/bin:/usr/local/bin:" + env.get("PATH", "")

            sessions = self._load_media_sessions()
            existing_conv_id = sessions.get(media_id)

            success = False
            last_output = ""

            for cli_bin in available_clis:
                bin_name = os.path.basename(cli_bin)
                brain_dir = self._get_brain_dir(cli_bin)
                existing_dirs = set()
                if brain_dir.exists():
                    try:
                        existing_dirs = set(d.name for d in brain_dir.iterdir() if d.is_dir())
                    except Exception:
                        pass

                try:
                    self.active_job = {
                        "media_id": media_id,
                        "showTitle": pending_item.get("showTitle", ""),
                        "command": cmd_text,
                        "cli": bin_name,
                        "start_time": time.strftime("%H:%M:%S")
                    }

                    status_msg = f"🚀 [{media_id}] Đang chạy trên {bin_name}..."
                    if existing_conv_id:
                        status_msg += f" (Tiếp tục session: {existing_conv_id[:8]}...)"
                    self.update_response(cmd_id, status_msg, status="processing")
                    self.log_live(f"🚀 Kích hoạt tiến trình {bin_name} cho [{media_id}]...", "system")
                    print(f"[AgentBridge] 🚀 Dispatching [{media_id}] to {bin_name}: \"{cmd_text}\" (ID: {cmd_id})", flush=True)

                    prompt_with_context = self._build_context_prompt(cmd_text, media_id, workspace_dir)

                    cli_args = [cli_bin, "--add-dir", workspace_dir]
                    if existing_conv_id:
                        cli_args.extend(["--conversation", existing_conv_id])
                    cli_args.extend(["-p", prompt_with_context, "--dangerously-skip-permissions"])

                    brain_dirs = self._get_brain_dirs(cli_bin)
                    stop_tailer = threading.Event()
                    discovered_conv_id = [existing_conv_id]

                    def on_conv_discovered(cid):
                        discovered_conv_id[0] = cid
                        sessions[media_id] = cid
                        self._save_media_sessions(sessions)
                        self.log_live(f"📌 Gắn session mới {cid} cho {media_id}", "system")
                        print(f"[AgentBridge] 📌 Gắn {media_id} với Conversation UUID: {cid}", flush=True)

                    tailer_thread = threading.Thread(
                        target=self._tail_transcript,
                        args=(brain_dirs, existing_conv_id, time.time(), stop_tailer, on_conv_discovered),
                        daemon=True
                    )
                    tailer_thread.start()

                    proc = subprocess.Popen(
                        cli_args,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.STDOUT,
                        text=True,
                        cwd=workspace_dir,
                        env=env,
                        bufsize=1,
                        start_new_session=True
                    )
                    self._current_proc = proc

                    # Record live CLI service state with process PID
                    self._save_service_state({
                        "status": "running",
                        "cli_pid": proc.pid,
                        "cli": bin_name,
                        "media_id": media_id,
                        "conversation_id": existing_conv_id or "",
                        "command": cmd_text,
                        "start_time": time.strftime("%H:%M:%S"),
                        "workspace_dir": workspace_dir
                    })

                    lines = []
                    for raw_l in iter(proc.stdout.readline, ''):
                        if self._stop_requested:
                            break
                        line = raw_l.rstrip()
                        if line:
                            lines.append(line)
                            # Skip if line is raw json or internal noise
                            if not line.startswith('{"event":') and not line.startswith('{"step_index":'):
                                lvl = "error" if "error" in line.lower() else "warning" if "warning" in line.lower() else "info"
                                self.log_live(line, lvl)

                    proc.stdout.close()
                    stop_tailer.set()
                    tailer_thread.join(timeout=2)
                    self._current_proc = None

                    # Reset service state on completion
                    self._save_service_state({
                        "status": "idle",
                        "cli_pid": None,
                        "last_media_id": media_id,
                        "last_conversation_id": sessions.get(media_id),
                        "last_finished": time.strftime("%H:%M:%S")
                    })

                    if self._stop_requested:
                        self._stop_requested = False
                        last_output = "🛑 Đã huỷ bởi người dùng."
                        self.log_live(last_output, "warning")
                        break

                    ret_code = proc.wait(timeout=30)
                    combined = "\n".join(lines).strip()

                    # Check for quota exhaustion fallback
                    if "quota reached" in combined.lower():
                        self.log_live(f"⚠️ {bin_name} hết quota, chuyển sang CLI tiếp theo...", "warning")
                        print(f"[AgentBridge] ⚠️ {bin_name} đã hết hạn mức quota, thử fallback sang CLI tiếp theo...", flush=True)
                        last_output = combined
                        continue

                    if ret_code == 0:
                        success = True
                        last_output = combined or "✅ Đã thực thi thành công qua Antigravity CLI."
                        self.log_live(f"✅ Hoàn tất tác vụ thành công trên {bin_name}.", "success")
                        print(f"[AgentBridge] ✅ Hoàn thành lệnh qua {bin_name} (ID: {cmd_id})", flush=True)

                        # Record in Quota Guard if translation command
                        cmd_l = cmd_text.lower()
                        if "translate-subtitle" in cmd_l or "dịch phụ đề" in cmd_l or "dịch thuật" in cmd_l:
                            import re
                            done_eps = len(re.findall(r'S\d+E\d+', cmd_text, re.IGNORECASE)) or 1
                            quota_guard.record_translation(episodes_count=done_eps, media_id=media_id)
                            st = quota_guard.get_status()
                            self.log_live(f"📊 [Quota Guard] Đã ghi nhận {done_eps} tập vào Quota (Ngày: {st['day']['used']}/{st['day']['limit']}, Tuần: {st['week']['used']}/{st['week']['limit']}).", "info")

                        # Discover newly created conversation if not existing
                        if not sessions.get(media_id) and brain_dir.exists():
                            try:
                                current_dirs = set(d.name for d in brain_dir.iterdir() if d.is_dir())
                                new_dirs = current_dirs - existing_dirs
                                if new_dirs:
                                    new_conv = list(new_dirs)[0]
                                    sessions[media_id] = new_conv
                                    self._save_media_sessions(sessions)
                                    self.log_live(f"📌 Gắn session mới {new_conv} cho {media_id}", "system")
                                    print(f"[AgentBridge] 📌 Gắn {media_id} với Conversation UUID: {new_conv}", flush=True)
                            except Exception as e:
                                print(f"[AgentBridge] Không lấy được UUID mới: {e}")
                        break
                    else:
                        last_output = combined or f"CLI thoát với mã lỗi {ret_code}"
                        self.log_live(f"❌ CLI trả về mã lỗi {ret_code}", "error")
                        print(f"[AgentBridge] ❌ Lỗi khi chạy {bin_name}: {last_output[:200]}", flush=True)
                except subprocess.TimeoutExpired:
                    last_output = f"Quá thời gian xử lý (timeout 30 phút) trên {bin_name}"
                    self.log_live(last_output, "error")
                    break
                except Exception as e:
                    last_output = f"Lỗi ngoại lệ khi gọi {bin_name}: {e}"
                    self.log_live(last_output, "error")

            self.active_job = None
            final_status = "done" if success else "failed"
            self.update_response(cmd_id, last_output, status=final_status)
            time.sleep(1)
