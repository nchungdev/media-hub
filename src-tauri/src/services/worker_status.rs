use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    pub name: String,
    /// idle | running | ok | error | stopped
    pub state: String,
    pub message: String,
    pub items: i64,
    pub last_start: f64,
    pub last_finish: f64,
    pub runs: u64,
    pub errors: u64,
    /// Nguoi dung tat worker nay -> vong lap bo qua viec, khong thoat han
    /// (thoat han thi khong bat lai duoc ma khong khoi dong lai ca app).
    pub enabled: bool,
    /// Yeu cau chay ngay, khong doi het chu ky ngu.
    pub run_requested: bool,
}

impl Default for WorkerStatus {
    fn default() -> Self {
        Self {
            name: String::new(),
            state: "idle".into(),
            message: String::new(),
            items: 0,
            last_start: 0.0,
            last_finish: 0.0,
            runs: 0,
            errors: 0,
            enabled: true,
            run_requested: false,
        }
    }
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
        ..Default::default()
    });
    e.state = "running".into();
    e.last_start = now();
    e.runs += 1;
}

/// Dang ky worker truoc khi no chay lan dau, de nut Start/Stop hien ngay
/// thay vi doi worker chay xong vong dau moi xuat hien.
pub fn register(name: &str) {
    if let Ok(mut r) = registry().lock() {
        r.entry(name.to_string()).or_insert_with(|| WorkerStatus {
            name: name.to_string(),
            ..Default::default()
        });
    }
}

/// Bat/tat worker. Tat khong lam thread thoat -- vong lap van song nhung bo
/// qua viec, nho vay bat lai duoc ma khong phai khoi dong lai app.
pub fn set_enabled(name: &str, enabled: bool) -> bool {
    if let Ok(mut r) = registry().lock() {
        let e = r.entry(name.to_string()).or_insert_with(|| WorkerStatus {
            name: name.to_string(),
            ..Default::default()
        });
        e.enabled = enabled;
        e.state = if enabled { "idle".into() } else { "stopped".into() };
        e.message = if enabled {
            "đã bật lại".to_string()
        } else {
            "đã dừng theo yêu cầu".to_string()
        };
        return true;
    }
    false
}

/// Tat co `enabled` ma KHONG dong bo `state`/`message` -- dung khi worker tu
/// tat sau khi xong mot luot (vd franchise_ai_classifier), de giu lai thong
/// diep tong ket huu ich thay vi bi de thanh "da dung theo yeu cau" chung
/// chung. set_enabled(false) van la lua chon dung khi NGUOI DUNG bam Stop.
pub fn disable_silently(name: &str) {
    if let Ok(mut r) = registry().lock() {
        if let Some(e) = r.get_mut(name) {
            e.enabled = false;
        }
    }
}

pub fn is_enabled(name: &str) -> bool {
    registry()
        .lock()
        .ok()
        .and_then(|r| r.get(name).map(|e| e.enabled))
        .unwrap_or(true)
}

/// Yeu cau worker chay ngay o vong ke tiep.
pub fn request_run(name: &str) {
    if let Ok(mut r) = registry().lock() {
        let e = r.entry(name.to_string()).or_insert_with(|| WorkerStatus {
            name: name.to_string(),
            ..Default::default()
        });
        e.run_requested = true;
        e.enabled = true;
    }
}

/// Worker goi ham nay de xem co ai yeu cau chay ngay khong (va xoa co).
pub fn take_run_request(name: &str) -> bool {
    if let Ok(mut r) = registry().lock() {
        if let Some(e) = r.get_mut(name) {
            let v = e.run_requested;
            e.run_requested = false;
            return v;
        }
    }
    false
}

/// Ngu nhung van phan hoi nhanh khi bi dung hoac bi yeu cau chay ngay.
/// Tra ve true neu bi ngat giua chung (co yeu cau chay).
pub fn sleep_interruptible(name: &str, total: std::time::Duration) -> bool {
    let step = std::time::Duration::from_millis(500);
    let mut waited = std::time::Duration::ZERO;
    while waited < total {
        std::thread::sleep(step);
        waited += step;
        if take_run_request(name) {
            return true;
        }
    }
    false
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
