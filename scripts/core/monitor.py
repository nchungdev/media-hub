#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Pipeline & Task Monitor Core Module
"""

import os
import re
import glob

TASKS_DIR = "/Users/chungnh/.agy-account2/.gemini/antigravity-cli/brain/44f11f8b-d2dc-43f6-ba7c-6fcd479dbc58/.system_generated/tasks"

class PipelineMonitor:
    def __init__(self):
        pass

    def get_monster_status(self):
        # Scan for latest Monster pipeline log
        monster_log = os.path.join(TASKS_DIR, "task-212.log")
        if not os.path.exists(monster_log):
            return {"name": "Monster (2004) BluRay", "status": "idle", "current_ep": 0, "total_eps": 74, "percent": 0.0}
            
        try:
            with open(monster_log, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.readlines()
                
            last_completed_ep = 0
            current_ep = 0
            current_ep_name = ""
            
            for line in lines:
                m_comp = re.search(r"Hoàn tất 100% chu trình cuốn chiếu Tập (\d+)", line)
                if m_comp:
                    last_completed_ep = int(m_comp.group(1))
                    
                m_proc = re.search(r"\[TẬP (\d+) / 74\] Xử lý: (.*)", line)
                if m_proc:
                    current_ep = int(m_proc.group(1))
                    current_ep_name = m_proc.group(2).strip()
                    
            prog_ep = max(last_completed_ep, current_ep)
            percent = round((prog_ep / 74.0) * 100, 1)
            
            return {
                "name": "Monster (2004) [1080p BluRay]",
                "status": "running",
                "completed_eps": last_completed_ep,
                "current_ep": current_ep,
                "current_ep_name": current_ep_name,
                "total_eps": 74,
                "percent": percent,
                "dvd_status": "74/74 (100% Complete)"
            }
        except Exception as e:
            return {"name": "Monster (2004) BluRay", "error": str(e)}

    def get_multi_show_status(self):
        multi_log = os.path.join(TASKS_DIR, "task-325.log")
        if not os.path.exists(multi_log):
            return {"status": "idle", "queue": []}
            
        try:
            with open(multi_log, "r", encoding="utf-8", errors="ignore") as f:
                lines = f.readlines()
                
            current_show = "Unknown"
            current_ep = 0
            completed_eps = 0
            total_eps = 0
            completed_shows = []
            
            for line in lines:
                m_show = re.search(r"BẮT ĐẦU ĐỒNG BỘ: (.*?) \(", line)
                if m_show:
                    current_show = m_show.group(1).strip()
                    
                m_done_show = re.search(r"HOÀN TẤT 100% ĐỒNG BỘ SHOW: (.*?)!", line)
                if m_done_show:
                    completed_shows.append(m_done_show.group(1).strip())
                    
                m_total = re.search(r"Tổng số tập video cần sync: (\d+) tập", line)
                if m_total:
                    total_eps = int(m_total.group(1))
                    
                m_ep = re.search(r"--- \[(\d+)/(\d+)\] Xử lý: (.*?) ---", line)
                if m_ep:
                    current_ep = int(m_ep.group(1))
                    total_eps = int(m_ep.group(2))

                m_done_ep = re.search(r"🎉 Hoàn tất Tập (\d+)", line)
                if m_done_ep:
                    completed_eps = int(m_done_ep.group(1))
                    
            return {
                "status": "running",
                "current_show": current_show,
                "current_ep": current_ep,
                "completed_eps": completed_eps,
                "total_eps": total_eps,
                "completed_shows": completed_shows,
                "percent": round((current_ep / total_eps * 100) if total_eps > 0 else 0, 1)
            }
        except Exception as e:
            return {"status": "error", "error": str(e)}
