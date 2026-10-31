use std::fs;
use std::path::Path;

use regex::Regex;

use crate::store::TranscriptEntry;

pub fn is_set(folder: &str) -> bool {
    !folder.trim().is_empty()
}

pub fn try_write(folder: &str, entry: &TranscriptEntry) -> Result<bool, String> {
    if !is_set(folder) || entry.text.trim().is_empty() {
        return Ok(false);
    }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let path = Path::new(folder).join(file_name_for(entry));
    fs::write(path, format_entry(entry)).map_err(|e| e.to_string())?;
    Ok(true)
}

pub fn write_all(folder: &str, entries: &[TranscriptEntry]) -> Result<usize, String> {
    if !is_set(folder) {
        return Err("Kein Transkript-Ordner gesetzt.".into());
    }
    let mut count = 0;
    for entry in entries {
        match try_write(folder, entry) {
            Ok(true) => count += 1,
            Ok(false) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(count)
}

pub fn file_name_for(entry: &TranscriptEntry) -> String {
    let when = entry
        .transcribed_at
        .unwrap_or_else(chrono::Local::now);
    let stamp = when.format("%Y-%m-%d_%H-%M");
    let mut title = sanitize_title(entry.title.as_deref());
    if title.is_empty() {
        title = sanitize_title(entry.summary.as_deref());
    }
    if title.is_empty() {
        title = sanitize_title(Path::new(&entry.file_name).file_stem().and_then(|s| s.to_str()));
    }
    if title.is_empty() {
        title = "Transkript".into();
    }
    format!("{stamp} {title}.txt")
}

pub fn sanitize_title(raw: Option<&str>) -> String {
    let Some(raw) = raw.filter(|s| !s.trim().is_empty()) else {
        return String::new();
    };
    let mut text = raw.trim().trim_matches('"').trim_matches('\'').to_string();
    text = text.replace(':', " ").replace('/', "-").replace('\\', "-");
    for c in ['<', '>', ':', '"', '|', '?', '*'] {
        text = text.replace(c, " ");
    }
    let collapse = Regex::new(r"\s+").unwrap();
    text = collapse.replace_all(&text, " ").trim().trim_end_matches('.').to_string();
    if text.chars().count() > 80 {
        let cut: String = text.chars().take(80).collect();
        if let Some(space) = cut.rfind(' ') {
            if space >= 24 {
                text = cut[..space].to_string();
            } else {
                text = cut;
            }
        } else {
            text = cut;
        }
    }
    text
}

fn format_entry(entry: &TranscriptEntry) -> String {
    let mut sb = String::new();
    let title = if entry.title.as_deref().unwrap_or("").trim().is_empty() {
        entry.summary.clone().unwrap_or_default()
    } else {
        entry.title.clone().unwrap_or_default()
    };
    if !title.trim().is_empty() {
        sb.push_str(title.trim());
        sb.push('\n');
    }
    if !entry.file_name.is_empty() {
        sb.push_str(&entry.file_name);
        sb.push('\n');
    }
    if let Some(when) = entry.transcribed_at {
        sb.push_str(&when.format("%d.%m.%Y %H:%M").to_string());
        sb.push('\n');
    }
    if let Some(summary) = entry.summary.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if summary != title.trim() {
            sb.push('\n');
            sb.push_str(summary);
            sb.push('\n');
        }
    }
    sb.push('\n');
    sb.push_str(entry.text.trim());
    if !entry.text.ends_with('\n') {
        sb.push('\n');
    }
    sb
}
