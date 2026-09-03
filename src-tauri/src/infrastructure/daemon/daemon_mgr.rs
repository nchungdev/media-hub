use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub struct DaemonManager {
    child: Mutex<Option<Child>>,
}

impl DaemonManager {
    pub fn new() -> Self {
        Self {
            child: Mutex::new(None),
        }
    }

    pub fn start_sidecar(&self, _app_handle: &tauri::AppHandle) -> Result<(), String> {
        let ws_dir = PathBuf::from("/Volumes/512GB/AI Workspace");
        let py_script = ws_dir.join("apps/media-hub/scripts/server.py");

        if !py_script.exists() {
            return Err(format!(
                "Python server script not found at: {:?}",
                py_script
            ));
        }

        let python_bin = Self::find_python();
        let log_dir = ws_dir.join(".media-hub");
        let _ = fs::create_dir_all(&log_dir);
        let log_file = log_dir.join("server.log");

        let out_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)
            .map_err(|e| format!("Failed to open log file: {}", e))?;
        let err_file = out_file
            .try_clone()
            .map_err(|e| format!("Failed to clone log file handle: {}", e))?;

        let mut cmd = Command::new(python_bin);
        cmd.arg(&py_script)
            .arg("8888")
            .current_dir(&ws_dir)
            .stdout(Stdio::from(out_file))
            .stderr(Stdio::from(err_file));

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Python backend: {}", e))?;

        let mut lock = self.child.lock().unwrap();
        *lock = Some(child);

        Ok(())
    }

    fn find_python() -> String {
        let candidates = [
            "/opt/homebrew/bin/python3",
            "/usr/local/bin/python3",
            "/usr/bin/python3",
        ];
        for cand in candidates {
            if Path::new(cand).exists() {
                return cand.to_string();
            }
        }
        "python3".to_string()
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}
