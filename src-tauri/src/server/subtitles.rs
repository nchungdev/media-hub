/**
 * @file subtitles.rs
 * @description Ultra-fast Subtitle Engine & WebVTT Converter in Rust.
 * Converts ASS and SRT subtitles to clean, standard WebVTT format in microseconds.
 */

use axum::{
    extract::Query,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
pub struct SubDownloadParams {
    pub path: Option<String>,
}

pub struct SubtitleEngine;

impl SubtitleEngine {
    /// Converts SRT content to WebVTT
    pub fn srt_to_webvtt(srt: &str) -> String {
        let mut vtt = String::from("WEBVTT\n\n");
        let re_time = Regex::new(r"(\d{2}:\d{2}:\d{2}),(\d{3})").unwrap();
        let cleaned = re_time.replace_all(srt, "$1.$2");
        vtt.push_str(&cleaned);
        vtt
    }

    /// Converts ASS content to WebVTT
    pub fn ass_to_webvtt(ass: &str) -> String {
        let mut vtt = String::from("WEBVTT\n\n");
        let re_tags = Regex::new(r"\{[^\}]+\}").unwrap();
        let mut in_events = false;

        for line in ass.lines() {
            let line = line.trim();
            if line.starts_with("[Events]") {
                in_events = true;
                continue;
            }

            if in_events && line.starts_with("Dialogue:") {
                // Dialogue: 0,0:00:01.00,0:00:04.00,Default,,0,0,0,,Text
                let parts: Vec<&str> = line.splitn(10, ',').collect();
                if parts.len() >= 10 {
                    let start = parts[1].trim();
                    let end = parts[2].trim();
                    let raw_text = parts[9];

                    let clean_text = re_tags.replace_all(raw_text, "");
                    let clean_text = clean_text.replace("\\N", "\n").replace("\\n", "\n");

                    // Format timecode 0:00:01.00 -> 00:00:01.000
                    let fmt_time = |t: &str| -> String {
                        let segments: Vec<&str> = t.split(':').collect();
                        if segments.len() == 3 {
                            let h: u32 = segments[0].parse().unwrap_or(0);
                            let m: u32 = segments[1].parse().unwrap_or(0);
                            let sec_parts: Vec<&str> = segments[2].split('.').collect();
                            let s: u32 = sec_parts[0].parse().unwrap_or(0);
                            let ms: u32 = if sec_parts.len() > 1 {
                                format!("{:0<3}", sec_parts[1]).parse().unwrap_or(0)
                            } else {
                                0
                            };
                            format!("{:02}:{:02}:{:02}.{:03}", h, m, s, ms)
                        } else {
                            t.to_string()
                        }
                    };

                    vtt.push_str(&format!(
                        "{} --> {}\n{}\n\n",
                        fmt_time(start),
                        fmt_time(end),
                        clean_text.trim()
                    ));
                }
            }
        }
        vtt
    }
}

/// Axum Handler for GET /api/subtitles/download
pub async fn handle_sub_download(Query(params): Query<SubDownloadParams>) -> Response {
    let file_path = match params.path {
        Some(p) => p,
        None => return (StatusCode::BAD_REQUEST, "Missing path parameter").into_response(),
    };

    let p = Path::new(&file_path);
    if !p.exists() {
        return (StatusCode::NOT_FOUND, "Subtitle file not found").into_response();
    }

    match fs::read_to_string(p) {
        Ok(content) => {
            let filename = p.file_name().and_then(|f| f.to_str()).unwrap_or("sub.srt");
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                "text/plain; charset=utf-8".parse().unwrap(),
            );
            headers.insert(
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", filename)
                    .parse()
                    .unwrap(),
            );
            (StatusCode::OK, headers, content).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Read error: {}", e),
        )
            .into_response(),
    }
}
