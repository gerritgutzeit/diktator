use anyhow::Result;
use std::sync::mpsc;
use std::sync::Mutex;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateInfo, UpdateManager};

pub const REPO_URL: &str = "https://github.com/gerritgutzeit/diktator";

pub struct UpdateCheckResult {
    #[allow(dead_code)]
    pub is_installed: bool,
    pub update_available: bool,
    pub message: String,
}

pub fn display_version() -> String {
    match manager() {
        Ok(mgr) => mgr.get_current_version_as_string(),
        Err(_) => env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn check() -> Result<UpdateCheckResult> {
    let Ok(mgr) = manager() else {
        PENDING.lock().unwrap().take();
        return Ok(UpdateCheckResult {
            is_installed: false,
            update_available: false,
            message: "Updates nur in der installierten App (Setup.exe).".into(),
        });
    };

    match mgr.check_for_updates()? {
        UpdateCheck::UpdateAvailable(info) => {
            let version = info.TargetFullRelease.Version.clone();
            PENDING.lock().unwrap().replace(*info);
            Ok(UpdateCheckResult {
                is_installed: true,
                update_available: true,
                message: format!("Version {version} ist verfügbar."),
            })
        }
        UpdateCheck::NoUpdateAvailable | UpdateCheck::RemoteIsEmpty => {
            PENDING.lock().unwrap().take();
            Ok(UpdateCheckResult {
                is_installed: true,
                update_available: false,
                message: format!("Version {} ist aktuell.", display_version()),
            })
        }
    }
}

pub fn apply(progress: impl Fn(i16) + Send + 'static) -> Result<()> {
    let pending = PENDING
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| anyhow::anyhow!("Zuerst auf Aktualisierung prüfen."))?;
    let mgr = manager()?;
    let (tx, rx) = mpsc::channel();
    let progress_thread = std::thread::spawn(move || {
        while let Ok(percent) = rx.recv() {
            progress(percent);
        }
    });
    mgr.download_updates(&pending, Some(tx))?;
    let _ = progress_thread.join();
    mgr.apply_updates_and_restart(&pending)?;
    Ok(())
}

fn manager() -> Result<UpdateManager> {
    let source = GithubSource::new(REPO_URL, None, false);
    Ok(UpdateManager::new(source, None, None)?)
}

static PENDING: Mutex<Option<UpdateInfo>> = Mutex::new(None);
