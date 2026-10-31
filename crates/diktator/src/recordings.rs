use std::fs::{self, File};
use std::io::{self, copy};
use std::path::{Path, PathBuf};
use std::time::Duration;

use regex::Regex;
use once_cell::sync::Lazy;

use crate::paths;

static GUTZ_NAME: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)^Gutz(\d+)\.mp3$").unwrap());

#[derive(Debug, Clone)]
pub struct Recording {
    pub path: PathBuf,
    pub file_name: String,
    pub size: i64,
    pub last_write: std::time::SystemTime,
    pub sequence: Option<i32>,
}

pub fn list_all(root: &Path) -> Vec<Recording> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut recordings: Vec<Recording> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.to_ascii_lowercase().ends_with(".mp3") {
                return None;
            }
            if name.starts_with("._") {
                return None;
            }
            let full = path.to_string_lossy();
            if full.to_ascii_lowercase().contains("\\old\\")
                || full.to_ascii_lowercase().contains("/old/")
            {
                return None;
            }
            to_recording(&path, &name)
        })
        .collect();

    recordings.sort_by(|a, b| {
        a.sequence
            .unwrap_or(i32::MAX)
            .cmp(&b.sequence.unwrap_or(i32::MAX))
            .then_with(|| a.last_write.cmp(&b.last_write))
    });
    recordings
}

pub fn pick_newest(root: &Path) -> Option<Recording> {
    let all = list_all(root);
    if all.is_empty() {
        return None;
    }
    let sequenced: Vec<_> = all.iter().filter(|r| r.sequence.is_some()).cloned().collect();
    if !sequenced.is_empty() {
        return sequenced.into_iter().max_by_key(|r| r.sequence.unwrap_or(0));
    }
    let recent: Vec<_> = all
        .iter()
        .filter(|r| {
            r.last_write
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| chrono::DateTime::from_timestamp(d.as_secs() as i64, 0))
                .map(|dt| dt.year() >= 2020)
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    let pool = if recent.is_empty() { all } else { recent };
    pool.into_iter().max_by_key(|r| r.last_write)
}

pub async fn copy_to_inbox(recording: &Recording) -> anyhow::Result<PathBuf> {
    paths::ensure_directories();
    let dest = paths::inbox().join(&recording.file_name);
    let source = recording.path.clone();
    let dest_clone = dest.clone();
    tokio::task::spawn_blocking(move || copy_with_retry(&source, &dest_clone)).await??;
    Ok(dest)
}

fn copy_with_retry(source: &Path, dest: &Path) -> io::Result<()> {
    let mut last = None;
    for _ in 0..6 {
        match copy_once(source, dest) {
            Ok(()) => return Ok(()),
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied || err.kind() == io::ErrorKind::Other => {
                last = Some(err);
                std::thread::sleep(Duration::from_millis(400));
            }
            Err(err) => return Err(err),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::new(io::ErrorKind::Other, "Datei konnte nicht vom Gerät kopiert werden.")))
}

fn copy_once(source: &Path, dest: &Path) -> io::Result<()> {
    let mut src = File::open(source)?;
    let mut dst = File::create(dest)?;
    copy(&mut src, &mut dst)?;
    Ok(())
}

fn to_recording(path: &Path, name: &str) -> Option<Recording> {
    let meta = fs::metadata(path).ok()?;
    let sequence = GUTZ_NAME
        .captures(name)
        .and_then(|c| c.get(1)?.as_str().parse().ok());
    Some(Recording {
        path: path.to_path_buf(),
        file_name: name.to_string(),
        size: meta.len() as i64,
        last_write: meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
        sequence,
    })
}

trait ChronoYear {
    fn year(&self) -> i32;
}

impl ChronoYear for chrono::DateTime<chrono::Utc> {
    fn year(&self) -> i32 {
        chrono::Datelike::year(self)
    }
}
