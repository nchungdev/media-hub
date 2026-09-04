use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub name: String,
    /// idle | running | ok | error
    pub state: String,
    pub message: String,
    pub items: i64,
    pub last_start: f64,
    pub last_finish: f64,
    pub runs: u64,
    pub errors: u64,
}

fn registry() -> &'static Mutex<HashMap<String, WorkerStatus>> {
    static R: OnceLock<Mutex<HashMap<String, WorkerStatus>>> = OnceLock::new();
    R.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Danh dau worker bat dau mot vong chay.
pub fn begin(name: &str) {
    let mut r = match registry().lock() {
        Ok(r) => r,
        Err(_) => return,
    };
    let e = r.entry(name.to_string()).or_insert_with(|| WorkerStatus {
        name: name.to_string(),
        state: "idle".into(),
        message: String::new(),
        items: 0,
        last_start: 0.0,
        last_finish: 0.0,
        runs: 0,
        errors: 0,
    });
    e.state = "running".into();
    e.last_start = now();
    e.runs += 1;
}

pub fn ok(name: &str, items: i64, message: &str) {
    if let Ok(mut r) = registry().lock() {
        if let Some(e) = r.get_mut(name) {
            e.state = "ok".into();
            e.items = items;
            e.message = message.to_string();
            e.last_finish = now();
        }
    }
}

pub fn err(name: &str, message: &str) {
    if let Ok(mut r) = registry().lock() {
        if let Some(e) = r.get_mut(name) {
            e.state = "error".into();
            e.message = message.to_string();
            e.last_finish = now();
            e.errors += 1;
        }
    }
}

pub fn snapshot() -> Vec<WorkerStatus> {
    let r = match registry().lock() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut v: Vec<WorkerStatus> = r.values().cloned().collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}
