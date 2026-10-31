use std::fs;
use std::path::PathBuf;

pub fn root() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir());
    base.join("Diktator")
}

pub fn settings_file() -> PathBuf {
    root().join("settings.json")
}

pub fn transcripts_file() -> PathBuf {
    root().join("transcripts.json")
}

pub fn device_file() -> PathBuf {
    root().join("device.json")
}

pub fn inbox() -> PathBuf {
    root().join("inbox")
}

pub fn ensure_directories() {
    let _ = fs::create_dir_all(root());
    let _ = fs::create_dir_all(inbox());
}
