use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use chrono::Local;

pub const FOLDER_NAME: &str = "old";

pub fn move_to_old(device_root: &Path, file_name: &str) -> anyhow::Result<bool> {
    let source = device_root.join(file_name);
    if !source.exists() {
        return Ok(false);
    }
    let dest_dir = device_root.join(FOLDER_NAME);
    fs::create_dir_all(&dest_dir)?;
    let mut dest = dest_dir.join(file_name);
    if dest.exists() {
        let stem = Path::new(file_name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("file");
        let ext = Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| format!(".{s}"))
            .unwrap_or_default();
        dest = dest_dir.join(format!("{stem}_{}{ext}", Local::now().format("%Y%m%d_%H%M%S")));
    }
    move_with_retry(&source, &dest)?;

    let sidecar = device_root.join(format!("._{file_name}"));
    if sidecar.exists() {
        let side_dest = dest_dir.join(format!(
            "._{}",
            dest.file_name().and_then(|s| s.to_str()).unwrap_or(file_name)
        ));
        let _ = move_with_retry(&sidecar, &side_dest);
    }
    Ok(true)
}

fn move_with_retry(source: &Path, dest: &Path) -> io::Result<()> {
    let mut last: Option<io::Error> = None;
    for _ in 0..5 {
        match fs::rename(source, dest) {
            Ok(()) => return Ok(()),
            Err(err) => {
                last = Some(err);
                std::thread::sleep(Duration::from_millis(350));
            }
        }
    }
    Err(last.unwrap_or_else(|| {
        io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Konnte {} nicht nach old verschieben.",
                source.file_name().and_then(|s| s.to_str()).unwrap_or("Datei")
            ),
        )
    }))
}
