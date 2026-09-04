use crate::domain::traits::ISettingsService;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct AgentService {
    settings: Arc<dyn ISettingsService>,
    live_logs: Mutex<Vec<Value>>,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

impl AgentService {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings,
            live_logs: Mutex::new(Vec::new()),
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

    pub fn trigger_worker(&self) {
        let worker_script = "/Volumes/512GB/Studio Projects/media-hub/backend/core/agent_bridge.py";
        crate::services::worker_status::begin("agent_bridge");

        if !std::path::Path::new(worker_script).exists() {
            crate::services::worker_status::err(
                "agent_bridge",
                "khong tim thay agent_bridge.py",
            );
            return;
        }

        match Command::new("python3")
            .arg(worker_script)
            .arg("--run-once")
            .spawn()
        {
            Ok(child) => {
                let pending = self.count_pending();
                crate::services::worker_status::ok(
                    "agent_bridge",
                    pending,
                    &format!("da khoi chay worker (pid {})", child.id()),
                );
            }
            Err(e) => {
                crate::services::worker_status::err(
                    "agent_bridge",
                    &format!("khong chay duoc python3: {}", e),
                );
            }
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

    /// Co tien trinh nao khop chuoi nay dang chay khong.
    fn process_alive(pattern: &str) -> bool {
        Command::new("pgrep")
            .arg("-f")
            .arg(pattern)
            .output()
            .map(|o| o.status.success() && !o.stdout.is_empty())
            .unwrap_or(false)
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

    /// Trang thai THAT cua agent bridge / agy CLI.
    /// Truoc day ham nay tra ve cung mot cau "San sang" bat ke thuc te, nen
    /// giao dien luon bao xanh ke ca khi khong co gi chay.
    pub fn ensure_service(&self) -> Value {
        let worker_script = "/Volumes/512GB/Studio Projects/media-hub/backend/core/agent_bridge.py";
        let script_ok = std::path::Path::new(worker_script).exists();
        let bridge_running = Self::process_alive("agent_bridge.py");
        let agy_running = Self::process_alive("bin/agy");
        let pending = self.count_pending();

        let message = if !script_ok {
            "Thiếu agent_bridge.py — không xử lý được hàng đợi".to_string()
        } else if bridge_running || agy_running {
            format!("Đang xử lý ({} lệnh chờ)", pending)
        } else if pending > 0 {
            format!("Rảnh nhưng còn {} lệnh chờ", pending)
        } else {
            "Sẵn sàng, hàng đợi trống".to_string()
        };

        // Cap nhat luon vao bang worker de tab Dich Vu thay.
        crate::services::worker_status::begin("agent_bridge");
        if script_ok {
            crate::services::worker_status::ok("agent_bridge", pending, &message);
        } else {
            crate::services::worker_status::err("agent_bridge", &message);
        }

        json!({
            "success": script_ok,
            "running": bridge_running || agy_running,
            "script_ok": script_ok,
            "bridge_running": bridge_running,
            "agy_running": agy_running,
            "pending": pending,
            "message": message
        })
    }
}
