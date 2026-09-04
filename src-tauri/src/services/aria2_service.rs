use crate::domain::traits::ISettingsService;
use serde_json::{json, Value};
use std::sync::Arc;

/// Client JSON-RPC toi aria2c daemon (`aria2c --enable-rpc`).
///
/// Cung mot client nay dung cho ca hai loai nguon, vi aria2 khong phan biet:
///   - magnet:?xt=...           -> aria2 tu chay che do BitTorrent
///   - https://... (TorBox/DDL) -> aria2 tai HTTP da luong
pub struct Aria2Service {
    settings_service: Arc<dyn ISettingsService>,
    client: reqwest::Client,
}

#[derive(Debug, Clone)]
pub struct Aria2Progress {
    pub status: String,
    pub total_length: u64,
    pub completed_length: u64,
    pub download_speed: u64,
    pub error_message: String,
    pub files: Vec<String>,
}

impl Aria2Service {
    pub fn new(settings_service: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings_service,
            client: reqwest::Client::new(),
        }
    }

    fn rpc_url(&self) -> String {
        let s = self.settings_service.load();
        let host = if s.aria2_rpc_host.is_empty() {
            "127.0.0.1".to_string()
        } else {
            s.aria2_rpc_host
        };
        let port = if s.aria2_rpc_port == 0 {
            6800
        } else {
            s.aria2_rpc_port
        };
        format!("http://{}:{}/jsonrpc", host, port)
    }

    /// aria2 doi secret duoi dang tham so dau tien "token:<secret>".
    fn secret_param(&self) -> Option<String> {
        let s = self.settings_service.load();
        if s.aria2_rpc_secret.is_empty() {
            None
        } else {
            Some(format!("token:{}", s.aria2_rpc_secret))
        }
    }

    async fn call(&self, method: &str, mut params: Vec<Value>) -> Result<Value, String> {
        if let Some(secret) = self.secret_param() {
            params.insert(0, json!(secret));
        }

        let body = json!({
            "jsonrpc": "2.0",
            "id": "media-hub",
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(self.rpc_url())
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("khong ket noi duoc aria2 RPC: {}", e))?;

        let v: Value = resp.json().await.map_err(|e| e.to_string())?;

        if let Some(err) = v.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("loi khong ro");
            return Err(format!("aria2 tra loi loi: {}", msg));
        }

        v.get("result")
            .cloned()
            .ok_or_else(|| "aria2 tra ve phan hoi khong co result".to_string())
    }

    /// Kiem tra daemon con song khong (dung de quyet dinh co can khoi dong lai).
    pub async fn is_alive(&self) -> bool {
        self.call("aria2.getVersion", vec![]).await.is_ok()
    }

    /// Them mot URI (magnet HOAC http/https) vao hang doi tai.
    /// `dir` la thu muc dich -- day chinh la cho ta chi dinh
    /// _franchise/<Ten>/.staging/ de tai thang vao dung franchise.
    pub async fn add_uri(&self, uri: &str, dir: &str) -> Result<String, String> {
        let params = vec![
            json!([uri]),
            json!({
                "dir": dir,
                "continue": "true",
                "max-connection-per-server": "8",
                "split": "8",
                "auto-file-renaming": "false",
                "allow-overwrite": "false",
            }),
        ];
        let result = self.call("aria2.addUri", params).await?;
        result
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "aria2 khong tra ve GID".to_string())
    }

    pub async fn tell_status(&self, gid: &str) -> Result<Aria2Progress, String> {
        let params = vec![
            json!(gid),
            json!([
                "status",
                "totalLength",
                "completedLength",
                "downloadSpeed",
                "errorMessage",
                "files"
            ]),
        ];
        let r = self.call("aria2.tellStatus", params).await?;

        let num = |k: &str| -> u64 {
            r.get(k)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0)
        };

        let files = r
            .get("files")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|f| f.get("path").and_then(|p| p.as_str()))
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        Ok(Aria2Progress {
            status: r
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
            total_length: num("totalLength"),
            completed_length: num("completedLength"),
            download_speed: num("downloadSpeed"),
            error_message: r
                .get("errorMessage")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            files,
        })
    }

    pub async fn remove(&self, gid: &str) -> Result<(), String> {
        // forceRemove de huy duoc ca khi dang o trang thai cho.
        self.call("aria2.forceRemove", vec![json!(gid)]).await?;
        Ok(())
    }
}
