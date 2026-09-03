#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
Agent Command Queue Watcher Daemon
Auto-evaluates pending user commands through Skill-Scoped Intent Router
"""

import os
import sys
import json
import time

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
if SCRIPT_DIR not in sys.path:
    sys.path.insert(0, SCRIPT_DIR)

from core.intent_router import execute_scoped_command
from core.agent_bridge import default_queue_file

QUEUE_FILE = os.environ.get("MEDIA_HUB_QUEUE_FILE") or default_queue_file()

def watch_queue():
    print("🚀 Skill-Scoped Agent Command Watcher Daemon active...", flush=True)
    
    while True:
        try:
            if os.path.exists(QUEUE_FILE):
                with open(QUEUE_FILE, "r", encoding="utf-8") as f:
                    queue = json.load(f)
                    
                updated = False
                for item in queue:
                    if item.get("status") == "pending":
                        cmd_id = item.get("id")
                        cmd_text = item.get("command", "").strip()
                        print(f"🔔 [AGENT_TRIGGER] Phân tích lệnh: \"{cmd_text}\" (ID: {cmd_id})", flush=True)
                        
                        # Process through intent router
                        result = execute_scoped_command(cmd_text)
                        
                        item["status"] = "done"
                        item["response"] = result.get("response")
                        item["intent"] = result.get("intent")
                        item["skill"] = result.get("skill")
                        item["timestamp"] = time.strftime("%Y-%m-%d %H:%M:%S")
                        updated = True
                        print(f"✅ [AGENT_RESOLVED] Intent: {result.get('intent')} | Skill: {result.get('skill')}", flush=True)
                
                if updated:
                    with open(QUEUE_FILE, "w", encoding="utf-8") as f:
                        json.dump(queue, f, indent=2, ensure_ascii=False)
        except Exception as e:
            pass
        time.sleep(1.5)

if __name__ == "__main__":
    watch_queue()
