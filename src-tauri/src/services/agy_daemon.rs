use crate::domain::traits::ISettingsService;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const MAX_EVENTS: usize = 500;

/// Giu MOT tien trinh `agy` song thuong truc o che do stream-json.
///
/// Kieu cu spawn `--run-once` cho moi lenh phai nap lai auth va quet lai
/// workspace, mat 1-2 giay moi lan. O che do nay pipe STDIN mo suot: moi lenh
/// chi la mot dong NDJSON, agy chay ngay trong tien trinh dang co san.
///
/// Chay dung mot profile theo cau hinh `agy_cli_profile`: `agy` (tai khoan
/// chinh) hoac `agy2` (tai khoan phu -- cung binary nhung doi HOME sang
/// ~/.antigravity-instances/secondary de tach keychain).
pub struct AgyDaemon {
    settings: Arc<dyn ISettingsService>,
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    child: Arc<Mutex<Option<Child>>>,
    child_pid: Arc<Mutex<Option<u32>>>,
    events: Arc<Mutex<VecDeque<Value>>>,
    profile: Arc<Mutex<String>>,
}

impl AgyDaemon {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings,
            stdin: Arc::new(Mutex::new(None)),
            child: Arc::new(Mutex::new(None)),
            child_pid: Arc::new(Mutex::new(None)),
            events: Arc::new(Mutex::new(VecDeque::new())),
            profile: Arc::new(Mutex::new(String::new())),
        }
    }

    fn resolve_binary(profile: &str) -> Option<PathBuf> {
        let name = if profile == "agy2" { "agy2" } else { "agy" };
        let home = dirs::home_dir()?;
        let p = home.join(".local/bin").join(name);
        if p.exists() {
            Some(p)
        } else {
            None
        }
    }

    /// Giam sat: khoi dong daemon, doc su kien, tu bat lai neu tien trinh chet.
    pub fn start(self: &Arc<Self>) {
        let me = Arc::clone(self);
        std::thread::spawn(move || loop {
            if !crate::services::worker_status::is_enabled("agy_daemon") {
                crate::services::worker_status::sleep_interruptible("agy_daemon", Duration::from_secs(2));
                continue;
            }

            let profile = {
                let cfg = me.settings.load();
                let p = cfg.agy_cli_profile.trim().to_string();
                if p.is_empty() || p == "auto" {
                    "agy".to_string()
                } else {
                    p
                }
            };

            crate::services::worker_status::begin("agy_daemon");

            let bin = match Self::resolve_binary(&profile) {
                Some(b) => b,
                None => {
                    crate::services::worker_status::err(
                        "agy_daemon",
                        &format!("khong tim thay binary cho profile '{}'", profile),
                    );
                    std::thread::sleep(Duration::from_secs(60));
                    continue;
                }
            };

            match me.spawn_once(&bin, &profile) {
                Ok(mut child) => {
                    *me.profile.lock().unwrap() = profile.clone();
                    let pid = child.id();
                    *me.child_pid.lock().unwrap() = Some(pid);
                    crate::services::worker_status::ok(
                        "agy_daemon",
                        me.events.lock().map(|e| e.len() as i64).unwrap_or(0),
                        &format!("dang chay profile '{}' (pid {})", profile, pid),
                    );
                    // Chan o day cho toi khi tien trinh thoat -> vong lap bat lai.
                    let _ = child.wait();
                    log::warn!("[agy_daemon] tien trinh thoat, se bat lai sau 5 giay");
                    *me.stdin.lock().unwrap() = None;
                    *me.child_pid.lock().unwrap() = None;
                    *me.child.lock().unwrap() = None;
                    crate::services::worker_status::err("agy_daemon", "tien trinh da thoat");
                }
                Err(e) => {
                    log::error!("[agy_daemon] khong khoi chay duoc: {}", e);
                    crate::services::worker_status::err("agy_daemon", &e);
                }
            }

            std::thread::sleep(Duration::from_secs(5));
        });
    }

    pub fn stop(&self) {
        if let Some(pid) = *self.child_pid.lock().unwrap() {
            let _ = std::process::Command::new("kill").arg("-15").arg(pid.to_string()).output();
        }
    }

    fn spawn_once(&self, bin: &PathBuf, profile: &str) -> Result<Child, String> {
        let cfg = self.settings.load();

        let mut cmd = Command::new(bin);
        cmd.arg("--input-format")
            .arg("stream-json")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if !cfg.workspace_dir.trim().is_empty() {
            cmd.current_dir(&cfg.workspace_dir);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("spawn {} that bai: {}", profile, e))?;

        if let Some(si) = child.stdin.take() {
            *self.stdin.lock().unwrap() = Some(si);
        }

        // Doc STDOUT: moi dong la mot su kien NDJSON.
        if let Some(so) = child.stdout.take() {
            let events = Arc::clone(&self.events);
            std::thread::spawn(move || {
                for line in BufReader::new(so).lines().map_while(Result::ok) {
                    let line = line.trim().to_string();
                    if line.is_empty() {
                        continue;
                    }
                    let ev: Value = serde_json::from_str(&line)
                        .unwrap_or_else(|_| json!({ "event": "raw", "type": "raw", "text": line }));
                    if let Ok(mut q) = events.lock() {
                        q.push_back(ev);
                        while q.len() > MAX_EVENTS {
                            q.pop_front();
                        }
                    }
                }
            });
        }

        // STDERR chi ghi log, khong lam nhieu luong su kien.
        if let Some(se) = child.stderr.take() {
            std::thread::spawn(move || {
                for line in BufReader::new(se).lines().map_while(Result::ok) {
                    if !line.trim().is_empty() {
                        log::debug!("[agy_daemon/stderr] {}", line);
                    }
                }
            });
        }

        Ok(child)
    }

    /// Gui mot luot vao daemon. Tra ve Err neu daemon chua san sang.
    ///
    /// Dinh dang dung xac dinh bang cach do tay qua pipe truc tiep (agy tu
    /// choi cac dang khac voi loi ro rang trong "result.error"):
    ///   {"event":"user","message":{"role":"user","content":[{"type":"text","text":"..."}]}}
    /// Dang cu {"type":"user","content":"..."} bi agy tu choi ngay o buoc
    /// parse, nen moi lenh day qua day tu truoc gio deu that bai am tham --
    /// day chinh la ly do hang doi agent_queue khong bao gio chay duoc.
    pub fn send(&self, content: &str) -> Result<(), String> {
        let mut guard = self.stdin.lock().map_err(|_| "khoa stdin hong".to_string())?;
        let si = guard.as_mut().ok_or("daemon chua chay")?;
        let msg = json!({
            "event": "user",
            "message": {
                "role": "user",
                "content": [{ "type": "text", "text": content }]
            }
        });
        writeln!(si, "{}", msg).map_err(|e| format!("ghi stdin that bai: {}", e))?;
        si.flush().map_err(|e| format!("flush that bai: {}", e))?;
        Ok(())
    }

    /// Gui mot luot va CHO ket qua (dung cho tac vu can cau tra loi ngay,
    /// vd phan loai franchise). Khac voi send() la fire-and-forget cho
    /// agent_queue / Live Console.
    ///
    /// Danh dau vi tri cuoi hang doi su kien truoc khi gui, roi doi den khi
    /// thay mot event "result" MOI xuat hien sau vi tri do -- tranh doc nham
    /// ket qua cua luot truoc con sot trong buffer.
    pub fn ask(&self, prompt: &str, timeout: Duration) -> Result<String, String> {
        let start_len = self.events.lock().map(|e| e.len()).unwrap_or(0);
        self.send(prompt)?;

        let step = Duration::from_millis(300);
        let mut waited = Duration::ZERO;
        while waited < timeout {
            std::thread::sleep(step);
            waited += step;

            let found = {
                let q = match self.events.lock() {
                    Ok(q) => q,
                    Err(_) => continue,
                };
                if q.len() <= start_len {
                    None
                } else {
                    q.iter().skip(start_len).find(|e| e.get("event").and_then(|v| v.as_str()) == Some("result")).cloned()
                }
            };

            if let Some(ev) = found {
                let result = ev.get("result").cloned().unwrap_or(json!({}));
                let status = result.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status == "SUCCESS" {
                    let text = result
                        .get("response")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    return Ok(text);
                }
                let err = result
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("agy tra loi loi khong ro")
                    .to_string();
                return Err(err);
            }
        }
        Err(format!("het thoi gian cho ({}s) khong thay ket qua", timeout.as_secs()))
    }

    pub fn status(&self) -> Value {
        let running = self.stdin.lock().map(|g| g.is_some()).unwrap_or(false);
        let profile = self.profile.lock().map(|p| p.clone()).unwrap_or_default();
        let n = self.events.lock().map(|e| e.len()).unwrap_or(0);
        json!({
            "running": running,
            "profile": profile,
            "events_buffered": n,
        })
    }

    pub fn recent_events(&self, limit: usize) -> Vec<Value> {
        let q = match self.events.lock() {
            Ok(q) => q,
            Err(_) => return Vec::new(),
        };
        q.iter().rev().take(limit).rev().cloned().collect()
    }
}
