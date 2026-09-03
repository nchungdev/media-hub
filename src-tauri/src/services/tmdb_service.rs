use crate::domain::traits::ISettingsService;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct TmdbService {
    settings: Arc<dyn ISettingsService>,
    client: reqwest::Client,
}

impl TmdbService {
    pub fn new(settings: Arc<dyn ISettingsService>) -> Self {
        Self {
            settings,
            client: reqwest::Client::new(),
        }
    }

    pub async fn search(&self, query: &str) -> Value {
        let q = query.trim();
        if q.is_empty() {
            return json!({ "results": [] });
        }

        let cfg = self.settings.load();
        let api_key = if !cfg.tmdb_api_key.is_empty() {
            cfg.tmdb_api_key
        } else {
            std::env::var("TMDB_API_KEY").unwrap_or_default()
        };

        if api_key.is_empty() {
            return json!({
                "results": [],
                "warning": "Vui lòng nhập TMDb API Key trong tab Cài Đặt để kích hoạt tra cứu trực tiếp!"
            });
        }

        let url = format!(
            "https://api.themoviedb.org/3/search/multi?query={}&language=vi-VN&api_key={}",
            urlencoding::encode(q),
            api_key
        );

        match self
            .client
            .get(&url)
            .header("User-Agent", "Antigravity-Media-Hub/2.0")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    resp.json::<Value>().await.unwrap_or_else(|_| json!({ "results": [] }))
                } else {
                    json!({ "results": [], "error": format!("TMDb returned status {}", resp.status()) })
                }
            }
            Err(e) => json!({ "results": [], "error": e.to_string() }),
        }
    }
}
