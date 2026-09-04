use crate::domain::traits::ISettingsService;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AgentService {
    settings: Arc<dyn ISettingsService>,
    live_logs: Mutex<Vec<Value>>,
    agy: Arc<crate::services::agy_daemon::AgyDaemon>,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

impl AgentService {
    pub fn new(
        settings: Arc<dyn ISettingsService>,
        agy: Arc<crate::services::agy_daemon::AgyDaemon>,
    ) -> Self {
        Self {
            settings,
            live_logs: Mutex::new(Vec::new()),
            agy,
        }
    }

    fn queue_file(&self) -> PathBuf {
        let cfg = self.settings.load();
        let home = if !cfg.media_hub_home.is_empty() {
            PathBuf::from(cfg.media_hub_home)
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".media-hub")
        };
        home.join("_app").join("agent_queue.json")
    }

    fn sessions_file(&self) -> PathBuf {
        let cfg = self.settings.load();
        let home = if !cfg.media_hub_home.is_empty() {
            PathBuf::from(cfg.media_hub_home)
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".media-hub")
        };
        home.join("_app").join("media_sessions.json")
    }

    pub fn list_commands(&self) -> Value {
        let p = self.queue_file();
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
                if let Ok(val) = serde_json::from_str::<Value>(&data) {
                    return val;
                }
            }
        }
        json!([])
    }

    pub fn add_command(&self, cmd: &str, author: &str, media_id: Option<&str>) -> Value {
        let p = self.queue_file();
        let mut list = if let Ok(data) = std::fs::read_to_string(&p) {
            serde_json::from_str::<Vec<Value>>(&data).unwrap_or_default()
        } else {
            Vec::new()
        };

        let id = format!("cmd_{}", (now_secs() * 1000.0) as u64);
        let new_item = json!({
            "id": id,
            "command": cmd,
            "author": author,
            "media_id": media_id,
            "status": "pending",
            "created_at": now_secs(),
            "response": null
        });

        list.push(new_item.clone());
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&p, serde_json::to_string_pretty(&list).unwrap_or_default());

        // Spawn python worker to process queue if needed
        self.trigger_worker();

        new_item
    }

    /// Day cac lenh dang cho vao daemon agy.
    ///
    /// Truoc day ham nay spawn `python3 agent_bridge.py --run-once` cho moi
    /// lenh -- moi lan lai nap auth va quet workspace tu dau. Gio daemon da
    /// song san nen chi can ghi mot dong NDJSON vao stdin cua no.
    pub fn trigger_worker(&self) {
        crate::services::worker_status::begin("agent_queue");

        let path = self.queue_file();
        let txt = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                crate::services::worker_status::ok("agent_queue", 0, "hang doi trong");
                return;
            }
        };
        let mut arr: Vec<Value> = serde_json::from_str(&txt).unwrap_or_default();

        let mut sent = 0i64;
        let mut failed: Option<String> = None;
        for item in arr.iter_mut() {
            if item.get("status").and_then(|s| s.as_str()) != Some("pending") {
                continue;
            }
            let cmd = item
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            if cmd.trim().is_empty() {
                continue;
            }
            match self.agy.send(&cmd) {
                Ok(_) => {
                    item["status"] = json!("running");
                    item["response"] = json!("Đã gửi vào daemon agy, đang xử lý…");
                    sent += 1;
                }
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }

        if sent > 0 {
            if let Ok(txt) = serde_json::to_string_pretty(&arr) {
                let _ = std::fs::write(&path, txt);
            }
        }

        match failed {
            Some(e) => crate::services::worker_status::err("agent_queue", &e),
            None => crate::services::worker_status::ok(
                "agent_queue",
                sent,
                &format!("da day {} lenh vao daemon", sent),
            ),
        }
    }

    /// So lenh dang cho trong hang doi.
    fn count_pending(&self) -> i64 {
        let p = self.queue_file();
        let txt = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(_) => return 0,
        };
        let arr: Vec<Value> = serde_json::from_str(&txt).unwrap_or_default();
        arr.iter()
            .filter(|j| j.get("status").and_then(|s| s.as_str()) == Some("pending"))
            .count() as i64
    }


    pub fn get_live_logs(&self) -> Value {
        let logs = self.live_logs.lock().unwrap();
        json!({
            "success": true,
            "logs": *logs
        })
    }

    pub fn clear_live_logs(&self) {
        let mut logs = self.live_logs.lock().unwrap();
        logs.clear();
    }

    pub fn get_sessions(&self) -> Value {
        let p = self.sessions_file();
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
                if let Ok(val) = serde_json::from_str::<Value>(&data) {
                    return val;
                }
            }
        }
        json!({})
    }

    pub fn clear_media_session(&self, media_id: &str) -> bool {
        let p = self.sessions_file();
        if p.exists() {
            if let Ok(data) = std::fs::read_to_string(&p) {
                if let Ok(mut val) = serde_json::from_str::<Value>(&data) {
                    if let Some(obj) = val.as_object_mut() {
                        obj.remove(media_id);
                        let _ = std::fs::write(&p, serde_json::to_string_pretty(&val).unwrap_or_default());
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Trang thai that: bam vao daemon agy chu khong con tien trinh Python.
    pub fn ensure_service(&self) -> Value {
        let st = self.agy.status();
        let running = st.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
        let profile = st.get("profile").and_then(|v| v.as_str()).unwrap_or("");
        let pending = self.count_pending();

        let message = if !running {
            "Daemon agy chưa sẵn sàng".to_string()
        } else if pending > 0 {
            format!("Daemon '{}' đang chạy, {} lệnh chờ", profile, pending)
        } else {
            format!("Daemon '{}' sẵn sàng, hàng đợi trống", profile)
        };

        json!({
            "success": running,
            "running": running,
            "profile": profile,
            "pending": pending,
            "message": message
        })
    }
}
