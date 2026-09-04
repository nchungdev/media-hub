use crate::domain::traits::IArtworkService;
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::PathBuf;

pub struct ArtworkService {
    client: reqwest::Client,
    cache_dir: PathBuf,
}

impl ArtworkService {
    pub fn new(home: PathBuf) -> Self {
        let cache_dir = home.join(".cache").join("posters");
        let _ = fs::create_dir_all(&cache_dir);
        Self {
            client: reqwest::Client::new(),
            cache_dir,
        }
    }
}


#[async_trait]
impl IArtworkService for ArtworkService {
    async fn resolve_poster(
        &self,
        title: Option<&str>,
        tvdb: Option<&str>,
        _tmdb: Option<&str>,
    ) -> Option<Vec<u8>> {
        let key = tvdb
            .map(|t| format!("tvdb_{}", t))
            .or_else(|| title.map(|t| urlencoding::encode(t).to_string()))?;

        // 1. Disk Cache
        let cache_file = self.cache_dir.join(format!("{}.jpg", key));
        if cache_file.exists() {
            if let Ok(data) = fs::read(&cache_file) {
                return Some(data);
            }
        }

        // 2. AniList GraphQL API
        if let Some(t) = title {
            let clean_title = Self::clean_search_title(t);
            let query = r#"
                query ($search: String) {
                    Media (search: $search, type: ANIME) {
                        coverImage {
                            extraLarge
                            large
                        }
                    }
                }
            "#;

            let resp = self
                .client
                .post("https://graphql.anilist.co")
                .json(&json!({
                    "query": query,
                    "variables": { "search": clean_title }
                }))
                .header("User-Agent", "Antigravity-Media-Hub/2.5")
                .send()
                .await
                .ok()?;

            if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                if let Some(img_url) = json_val["data"]["Media"]["coverImage"]["extraLarge"]
                    .as_str()
                    .or_else(|| json_val["data"]["Media"]["coverImage"]["large"].as_str())
                {
                    if let Ok(img_resp) = self.client.get(img_url).send().await {
                        if let Ok(bytes) = img_resp.bytes().await {
                            let _ = fs::write(&cache_file, &bytes);
                            return Some(bytes.to_vec());
                        }
                    }
                }
            }
        }

        None
    }
}

impl ArtworkService {
    fn clean_search_title(raw: &str) -> String {
        let re_tvdb = regex::Regex::new(r"\{tvdb-\d+\}").unwrap();
        let s = re_tvdb.replace_all(raw, "");
        let s = s.split('(').next().unwrap_or(&s);
        s.trim().to_string()
    }
}
