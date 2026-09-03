# -*- coding: utf-8 -*-
"""
Translation Quota Guard for Antigravity Media Hub.
Safeguards Gemini API quota by enforcing daily (30 eps) and weekly (150 eps, 40% weekly Flash budget)
translation caps to prevent 429 RESOURCE_EXHAUSTED and account rate limiting.
"""

import os
import json
import time
import datetime
import threading
from pathlib import Path


class TranslationQuotaGuard:
    _instance = None
    _lock = threading.RLock()

    def __new__(cls, *args, **kwargs):
        with cls._lock:
            if cls._instance is None:
                cls._instance = super(TranslationQuotaGuard, cls).__new__(cls)
                cls._instance._init_guard()
            return cls._instance

    def _init_guard(self):
        # Safe defaults (40% weekly Flash budget)
        self.DEFAULT_DAILY_LIMIT = 30
        self.DEFAULT_WEEKLY_LIMIT = 150
        self.DEFAULT_DAILY_TOKEN_LIMIT = 750000
        self.DEFAULT_WEEKLY_TOKEN_LIMIT = 3750000

        self.daily_limit = self.DEFAULT_DAILY_LIMIT
        self.weekly_limit = self.DEFAULT_WEEKLY_LIMIT

        self.current_day = datetime.date.today().isoformat()
        self.current_week = self._get_current_week_str()

        self.day_episodes = 0
        self.week_episodes = 0
        self.day_tokens = 0
        self.week_tokens = 0
        self.history = []

        self._load_state()

    def _get_target_state_files(self):
        try:
            ws_root = Path(__file__).resolve().parents[4]
            ws_file = ws_root / ".media-hub" / "quota_guard.json"
            return [ws_file]
        except Exception:
            return [Path.cwd() / ".media-hub" / "quota_guard.json"]

    def _get_current_week_str(self):
        """Returns ISO format for week, e.g., '2026-W36'"""
        today = datetime.date.today()
        cal = today.isocalendar()
        return f"{cal[0]}-W{cal[1]:02d}"

    def _check_and_rollover(self):
        """Check if day or week has rolled over and reset counters accordingly."""
        today_str = datetime.date.today().isoformat()
        current_week_str = self._get_current_week_str()
        changed = False

        if self.current_day != today_str:
            self.current_day = today_str
            self.day_episodes = 0
            self.day_tokens = 0
            changed = True

        if self.current_week != current_week_str:
            self.current_week = current_week_str
            self.week_episodes = 0
            self.week_tokens = 0
            changed = True

        if changed:
            self._save_state()

    def _load_state(self):
        for sf in self._get_target_state_files():
            try:
                if sf.exists():
                    with open(sf, "r", encoding="utf-8") as f:
                        data = json.load(f)
                        self.daily_limit = int(data.get("daily_limit", self.DEFAULT_DAILY_LIMIT))
                        self.weekly_limit = int(data.get("weekly_limit", self.DEFAULT_WEEKLY_LIMIT))
                        self.current_day = data.get("current_day", datetime.date.today().isoformat())
                        self.current_week = data.get("current_week", self._get_current_week_str())
                        self.day_episodes = int(data.get("day_episodes", 0))
                        self.week_episodes = int(data.get("week_episodes", 0))
                        self.day_tokens = int(data.get("day_tokens", 0))
                        self.week_tokens = int(data.get("week_tokens", 0))
                        self.history = data.get("history", [])[-50:]
                    break
            except Exception:
                pass
        self._check_and_rollover()

    def _save_state(self):
        data = {
            "daily_limit": self.daily_limit,
            "weekly_limit": self.weekly_limit,
            "current_day": self.current_day,
            "current_week": self.current_week,
            "day_episodes": self.day_episodes,
            "week_episodes": self.week_episodes,
            "day_tokens": self.day_tokens,
            "week_tokens": self.week_tokens,
            "history": self.history[-50:],
            "updated_at": time.strftime("%Y-%m-%d %H:%M:%S")
        }
        for sf in self._get_target_state_files():
            try:
                sf.parent.mkdir(parents=True, exist_ok=True)
                with open(sf, "w", encoding="utf-8") as f:
                    json.dump(data, f, indent=2, ensure_ascii=False)
            except Exception as e:
                print(f"[QuotaGuard] Lỗi lưu trạng thái ({sf}): {e}")

    def get_time_until_reset(self):
        now = datetime.datetime.now()
        # Day resets at midnight (00:00 next day)
        tomorrow = datetime.datetime.combine(datetime.date.today() + datetime.timedelta(days=1), datetime.time.min)
        day_diff = tomorrow - now
        day_hours, day_rem = divmod(int(day_diff.total_seconds()), 3600)
        day_mins, _ = divmod(day_rem, 60)

        # Week resets at next Monday 00:00
        days_until_monday = (7 - now.weekday()) % 7
        if days_until_monday == 0:
            days_until_monday = 7
        next_monday = datetime.datetime.combine(datetime.date.today() + datetime.timedelta(days=days_until_monday), datetime.time.min)
        week_diff = next_monday - now
        week_days = week_diff.days
        week_hours = int(week_diff.seconds / 3600)

        return {
            "day_reset_in": f"{day_hours}h {day_mins}m",
            "week_reset_in": f"{week_days}d {week_hours}h"
        }

    def can_translate(self, requested_episodes=1):
        """
        Check if requested number of episodes can be translated without exceeding limits.
        """
        with self._lock:
            self._check_and_rollover()

            day_remaining = max(0, self.daily_limit - self.day_episodes)
            week_remaining = max(0, self.weekly_limit - self.week_episodes)
            resets = self.get_time_until_reset()

            if self.day_episodes >= self.daily_limit:
                return {
                    "allowed": False,
                    "reason": f"🛑 Đã chạm trần ngày: {self.day_episodes}/{self.daily_limit} tập. Quota sẽ reset sau {resets['day_reset_in']}.",
                    "scope": "day",
                    "day_remaining": 0,
                    "week_remaining": week_remaining,
                    "resets": resets
                }

            if self.week_episodes >= self.weekly_limit:
                return {
                    "allowed": False,
                    "reason": f"🛑 Đã chạm trần tuần (40% Quota Budget): {self.week_episodes}/{self.weekly_limit} tập. Quota sẽ reset sau {resets['week_reset_in']}.",
                    "scope": "week",
                    "day_remaining": day_remaining,
                    "week_remaining": 0,
                    "resets": resets
                }

            if self.day_episodes + requested_episodes > self.daily_limit:
                return {
                    "allowed": False,
                    "reason": f"⚠️ Yêu cầu {requested_episodes} tập vượt quá hạn mức còn lại trong ngày (chỉ còn {day_remaining} tập).",
                    "scope": "day_batch",
                    "day_remaining": day_remaining,
                    "week_remaining": week_remaining,
                    "resets": resets
                }

            if self.week_episodes + requested_episodes > self.weekly_limit:
                return {
                    "allowed": False,
                    "reason": f"⚠️ Yêu cầu {requested_episodes} tập vượt quá hạn mức còn lại trong tuần (chỉ còn {week_remaining} tập).",
                    "scope": "week_batch",
                    "day_remaining": day_remaining,
                    "week_remaining": week_remaining,
                    "resets": resets
                }

            return {
                "allowed": True,
                "reason": "OK",
                "day_remaining": day_remaining,
                "week_remaining": week_remaining,
                "resets": resets
            }

    def record_translation(self, episodes_count=1, media_id=None, tokens_est=0):
        """
        Record a successful translation batch into quota counters.
        """
        with self._lock:
            self._check_and_rollover()
            count = max(1, int(episodes_count or 1))
            self.day_episodes += count
            self.week_episodes += count

            # Token estimation: if not given, assume ~20,000 tokens per episode
            t_est = int(tokens_est) if tokens_est else (count * 20000)
            self.day_tokens += t_est
            self.week_tokens += t_est

            self.history.append({
                "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
                "media_id": media_id or "unknown",
                "episodes": count,
                "tokens_est": t_est
            })
            self._save_state()

    def get_status(self):
        """
        Returns rich status payload for Dashboard UI and API.
        """
        with self._lock:
            self._check_and_rollover()
            day_pct = min(100, round((self.day_episodes / self.daily_limit) * 100, 1)) if self.daily_limit > 0 else 0
            week_pct = min(100, round((self.week_episodes / self.weekly_limit) * 100, 1)) if self.weekly_limit > 0 else 0
            resets = self.get_time_until_reset()

            # Health badge determination
            if self.day_episodes >= self.daily_limit or self.week_episodes >= self.weekly_limit:
                status_code = "LOCKED"
                status_label = "🛑 ĐÃ KHÓA (CHẠM TRẦN)"
                status_color = "red"
            elif day_pct >= 80 or week_pct >= 80:
                status_code = "WARNING"
                status_label = "⚠️ GẦN TRẦN (CẢNH BÁO)"
                status_color = "amber"
            else:
                status_code = "SAFE"
                status_label = "🟢 AN TOÀN (READY)"
                status_color = "emerald"

            return {
                "status_code": status_code,
                "status_label": status_label,
                "status_color": status_color,
                "day": {
                    "used": self.day_episodes,
                    "limit": self.daily_limit,
                    "remaining": max(0, self.daily_limit - self.day_episodes),
                    "percentage": day_pct,
                    "tokens_est": self.day_tokens,
                    "reset_in": resets["day_reset_in"]
                },
                "week": {
                    "used": self.week_episodes,
                    "limit": self.weekly_limit,
                    "remaining": max(0, self.weekly_limit - self.week_episodes),
                    "percentage": week_pct,
                    "tokens_est": self.week_tokens,
                    "reset_in": resets["week_reset_in"]
                },
                "is_locked": bool(self.day_episodes >= self.daily_limit or self.week_episodes >= self.weekly_limit),
                "history": list(reversed(self.history[-15:]))
            }

    def update_limits(self, daily_limit=None, weekly_limit=None):
        with self._lock:
            if daily_limit is not None and int(daily_limit) > 0:
                self.daily_limit = int(daily_limit)
            if weekly_limit is not None and int(weekly_limit) > 0:
                self.weekly_limit = int(weekly_limit)
            self._save_state()
            return self.get_status()

    def reset_quota(self, scope="all"):
        with self._lock:
            if scope in ["all", "day"]:
                self.day_episodes = 0
                self.day_tokens = 0
            if scope in ["all", "week"]:
                self.week_episodes = 0
                self.week_tokens = 0
            self._save_state()
            return self.get_status()


# Global Singleton Instance
quota_guard = TranslationQuotaGuard()
