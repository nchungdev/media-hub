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
        home.join("agent_queue.json")
    }

    fn sessions_file(&self) -> PathBuf {
        let cfg = self.settings.load();
        let home = if !cfg.media_hub_home.is_empty() {
            PathBuf::from(cfg.media_hub_home)
        } else {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".media-hub")
        };
        home.join("media_sessions.json")
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
        let worker_script = "/Volumes/512GB/AI Workspace/apps/media-hub/backend/core/agent_bridge.py";
        if std::path::Path::new(worker_script).exists() {
            let _ = Command::new("python3")
                .arg(worker_script)
                .arg("--run-once")
                .spawn();
        }
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

    pub fn ensure_service(&self) -> Value {
        json!({
            "success": true,
            "running": true,
            "message": "Agent Bridge Service Sẵn sàng"
        })
    }
}
