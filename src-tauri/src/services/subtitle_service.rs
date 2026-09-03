use crate::domain::traits::ISubtitleService;
use async_trait::async_trait;
use regex::Regex;

pub struct SubtitleService;

impl SubtitleService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SubtitleService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ISubtitleService for SubtitleService {
    fn srt_to_webvtt(&self, srt: &str) -> String {
        let mut vtt = String::from("WEBVTT\n\n");
        let re_time = Regex::new(r"(\d{2}:\d{2}:\d{2}),(\d{3})").unwrap();
        let formatted = re_time.replace_all(srt, "$1.$2");
        vtt.push_str(&formatted);
        vtt
    }

    fn ass_to_webvtt(&self, ass: &str) -> String {
        let mut vtt = String::from("WEBVTT\n\n");
        let re_override = Regex::new(r"\{[^}]*\}").unwrap();
        let mut in_events = false;
        let mut counter = 1;

        for line in ass.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("[Events]") {
                in_events = true;
                continue;
            }
            if in_events && trimmed.starts_with('[') && !trimmed.starts_with("[Events]") {
                in_events = false;
                continue;
            }

            if in_events && trimmed.starts_with("Dialogue:") {
                let parts: Vec<&str> = trimmed.splitn(10, ',').collect();
                if parts.len() >= 10 {
                    let start = parts[1].trim();
                    let end = parts[2].trim();
                    let raw_text = parts[9].trim();

                    let text = re_override.replace_all(raw_text, "");
                    let text = text.replace(r"\N", "\n").replace(r"\n", "\n");

                    let fmt_start = Self::format_ass_time(start);
                    let fmt_end = Self::format_ass_time(end);

                    if !text.is_empty() {
                        vtt.push_str(&format!(
                            "{}\n{} --> {}\n{}\n\n",
                            counter, fmt_start, fmt_end, text
                        ));
                        counter += 1;
                    }
                }
            }
        }
        vtt
    }
}

impl SubtitleService {
    fn format_ass_time(t: &str) -> String {
        let parts: Vec<&str> = t.split('.').collect();
        let main = parts[0];
        let ms = if parts.len() > 1 {
            format!("{:0<3}", parts[1])
        } else {
            "000".to_string()
        };

        let main_parts: Vec<&str> = main.split(':').collect();
        if main_parts.len() == 3 {
            let h: u32 = main_parts[0].parse().unwrap_or(0);
            let m: u32 = main_parts[1].parse().unwrap_or(0);
            let s: u32 = main_parts[2].parse().unwrap_or(0);
            format!("{:02}:{:02}:{:02}.{}", h, m, s, &ms[..3])
        } else {
            format!("00:{}.{}", main, &ms[..3])
        }
    }
}
