use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Deserializer, Serialize};
use std::sync::Mutex;

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptEntry {
    #[serde(default)]
    pub volume_serial: String,
    #[serde(default)]
    pub file_name: String,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default, deserialize_with = "deserialize_dt")]
    pub transcribed_at: Option<DateTime<Local>>,
}

impl TranscriptEntry {
    pub fn same_as(&self, volume_serial: &str, file_name: &str, size: i64) -> bool {
        self.volume_serial.eq_ignore_ascii_case(volume_serial)
            && self.file_name.eq_ignore_ascii_case(file_name)
            && self.size == size
    }

    pub fn when_label(&self) -> String {
        self.transcribed_at
            .map(|d| d.format("%d.%m.%y  %H:%M").to_string())
            .unwrap_or_default()
    }

    pub fn list_summary(&self) -> String {
        if let Some(s) = self.summary.as_deref().filter(|s| !s.trim().is_empty()) {
            return s.trim().to_string();
        }
        let flat: String = self
            .text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        let flat = flat.trim();
        if flat.is_empty() {
            "…".into()
        } else if flat.chars().count() <= 72 {
            flat.to_string()
        } else {
            format!("{}…", flat.chars().take(72).collect::<String>())
        }
    }
}

fn deserialize_dt<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    let Some(s) = raw.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(Some(dt.with_timezone(&Local)));
    }
    for fmt in ["%Y-%m-%dT%H:%M:%S%.f", "%Y-%m-%dT%H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&s, fmt) {
            return Ok(Local.from_local_datetime(&naive).single());
        }
    }
    Ok(None)
}

pub struct TranscriptStore {
    entries: Mutex<Vec<TranscriptEntry>>,
}

impl TranscriptStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(load()),
        }
    }

    pub fn all(&self) -> Vec<TranscriptEntry> {
        let mut items = self.entries.lock().unwrap().clone();
        items.sort_by(|a, b| b.transcribed_at.cmp(&a.transcribed_at));
        items
    }

    pub fn find(&self, volume_serial: &str, file_name: &str, size: i64) -> Option<TranscriptEntry> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .find(|e| e.same_as(volume_serial, file_name, size))
            .cloned()
    }

    pub fn remove(&self, volume_serial: &str, file_name: &str, size: i64) -> bool {
        let mut entries = self.entries.lock().unwrap();
        let Some(idx) = entries
            .iter()
            .position(|e| e.same_as(volume_serial, file_name, size))
        else {
            return false;
        };
        entries.remove(idx);
        persist(&entries);
        true
    }

    pub fn save(
        &self,
        volume_serial: &str,
        file_name: &str,
        size: i64,
        text: &str,
    ) -> TranscriptEntry {
        let mut entries = self.entries.lock().unwrap();
        if let Some(idx) = entries
            .iter()
            .position(|e| e.same_as(volume_serial, file_name, size))
        {
            {
                let existing = &mut entries[idx];
                if existing.text != text {
                    existing.summary = None;
                    existing.title = None;
                }
                existing.text = text.to_string();
                existing.transcribed_at = Some(Local::now());
            }
            persist(&entries);
            return entries[idx].clone();
        }
        let entry = TranscriptEntry {
            volume_serial: volume_serial.to_string(),
            file_name: file_name.to_string(),
            size,
            text: text.to_string(),
            title: None,
            summary: None,
            transcribed_at: Some(Local::now()),
        };
        entries.push(entry.clone());
        persist(&entries);
        entry
    }

    pub fn save_headline(
        &self,
        volume_serial: &str,
        file_name: &str,
        size: i64,
        title: Option<&str>,
        summary: &str,
    ) {
        let mut entries = self.entries.lock().unwrap();
        if let Some(existing) = entries
            .iter_mut()
            .find(|e| e.same_as(volume_serial, file_name, size))
        {
            if let Some(title) = title.map(str::trim).filter(|s| !s.is_empty()) {
                existing.title = Some(title.to_string());
            }
            existing.summary = Some(summary.trim().to_string());
            persist(&entries);
        }
    }
}

fn load() -> Vec<TranscriptEntry> {
    paths::ensure_directories();
    let Ok(json) = std::fs::read_to_string(paths::transcripts_file()) else {
        return Vec::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

fn persist(entries: &[TranscriptEntry]) {
    paths::ensure_directories();
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(paths::transcripts_file(), json);
    }
}
