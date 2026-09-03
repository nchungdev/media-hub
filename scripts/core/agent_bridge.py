import os
import json
import time
import subprocess
import threading
import collections
from pathlib import Path

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
            # Resume any pending items on server start
            self._trigger_worker()
        except Exception as e:
            print(f"[AgentBridge] Không khởi tạo được hàng đợi ({self.queue_file}): {e}")

    def stop_current_job(self):
        """Kill the currently running agy/agy2 subprocess and cancel the active job."""
        self._stop_requested = True
        proc = self._current_proc
        if proc and proc.poll() is None:
            try:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
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
        # Check for TVDB / TMDB ID patterns
        tvdb_match = re.search(r'tvdb[-_](\d+)', command_text, re.IGNORECASE)
        if tvdb_match:
            return f"media-tvdb-{tvdb_match.group(1)}"
        
        tmdb_match = re.search(r'tmdb[-_](\d+)', command_text, re.IGNORECASE)
        if tmdb_match:
            return f"media-tmdb-{tmdb_match.group(1)}"

        # Check for common show titles
        cmd_lower = command_text.lower()
        if "monster" in cmd_lower:
            return "media-tvdb-74599"
        elif "wataru" in cmd_lower:
            return "media-tvdb-446736"
        elif "three-eyed" in cmd_lower or "three eyed" in cmd_lower or "3 mắt" in cmd_lower:
            return "media-tvdb-320122"
        elif "conan" in cmd_lower:
            return "media-tvdb-72454"
        elif "black jack" in cmd_lower:
            return "media-tvdb-78832"

        return "media-hub-system"

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

    def _process_transcript_step(self, step):
        stype = step.get("type")
        thinking = step.get("thinking")
        tool_calls = step.get("tool_calls")
        content = step.get("content")

        if thinking and str(thinking).strip():
            th_text = str(thinking).strip()
            lines = [l.strip() for l in th_text.split("\n") if l.strip()]
            for l in lines:
                clean_l = l.lstrip("#*-> ").strip()
                if clean_l and not clean_l.startswith("```"):
                    self.log_live(f"🧠 [Thinking] {clean_l}", "thinking")

        if tool_calls:
            for tc in tool_calls:
                name = tc.get("name", "tool")
                args = tc.get("args") or tc.get("parameters") or {}
                act = args.get("toolAction") or args.get("toolSummary") or args.get("Description") or args.get("Instruction") or ""

                if name == "run_command":
                    cmd = str(args.get("CommandLine", "")).strip()
                    if "\n" in cmd:
                        cmd = cmd.split("\n")[0] + "..."
                    self.log_live(f"⚡ [Run Command] {act} $ {cmd[:140]}", "tool")
                elif name in ["write_to_file", "replace_file_content"]:
                    tf = os.path.basename(str(args.get("TargetFile") or args.get("AbsolutePath") or args.get("Path") or ""))
                    self.log_live(f"📝 [{act or 'Chỉnh sửa tệp'}] {name} -> {tf}", "tool")
                elif name == "invoke_subagent":
                    subagents = args.get("Subagents", [])
                    roles = [s.get("Role", "Subagent") for s in subagents]
                    self.log_live(f"🤖 [Spawn Subagent] {', '.join(roles)}", "subagent")
                elif name == "view_file":
                    vf = os.path.basename(str(args.get("AbsolutePath", "")))
                    self.log_live(f"🔍 [Đọc Tệp Tin] {act} -> {vf}", "tool")
                elif name == "grep_search":
                    q = str(args.get("Query", ""))
                    self.log_live(f"🔎 [Tìm Kiếm Mã Nguồn] Pattern: '{q}'", "tool")
                elif name == "search_web":
                    q = str(args.get("query", ""))
                    self.log_live(f"🌐 [Tìm Kiếm Web] '{q}'", "tool")
                elif name == "send_message":
                    rec = args.get("Recipient", "")
                    self.log_live(f"✉️ [Gửi Tin Nhắn Cho Agent] Recipient: {rec}", "tool")
                else:
                    summary = args.get("toolSummary") or args.get("toolAction") or name
                    self.log_live(f"⚙️ [Thực Thi Công Cụ] {summary}", "tool")

        if stype == "GENERIC" and content and not str(content).startswith("Created At:"):
            first_line = str(content).strip().split("\n")[0].strip()
            if first_line and not first_line.startswith("{") and len(first_line) > 3:
                if len(first_line) > 120:
                    first_line = first_line[:120] + "..."
                self.log_live(f"  ↳ {first_line}", "output")

    def _tail_transcript(self, brain_dirs, conv_id_hint, start_time, stop_event, on_conv_discovered=None):
        """Realtime transcript tailer: streams thinking, tool calls, and execution steps to live console."""
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

        # Step 2: Tail transcript.jsonl
        try:
            with open(transcript_path, "r", encoding="utf-8") as f:
                if conv_id_hint:
                    # Count existing lines in resumed session so we only stream new steps
                    seen_lines = sum(1 for _ in f)
                    f.seek(0)
                    for _ in range(seen_lines):
                        f.readline()

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
            print(f"[AgentBridge] Error tailing transcript ({conv_id}): {e}")

    def add_command(self, command_text, author="User", media_id=None):
        queue = self._load()
        cmd_id = int(time.time() * 1000)
        resolved_media_id = self._extract_media_id(command_text, media_id)
        
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

                    cli_args = [cli_bin, "--add-dir", workspace_dir]
                    if existing_conv_id:
                        cli_args.extend(["--conversation", existing_conv_id])
                    cli_args.extend(["-p", cmd_text, "--dangerously-skip-permissions"])

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
                        bufsize=1
                    )
                    self._current_proc = proc

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
