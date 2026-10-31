use anyhow::Result;
use tokio::sync::Mutex;

use crate::archive;
use crate::device::{self, PhilipsDevice};
use crate::export;
use crate::recordings::{self, Recording};
use crate::settings::AppSettings;
use crate::store::{TranscriptEntry, TranscriptStore};
use crate::whisper::{WhisperClient, WhisperError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanStatus {
    Waiting = 0,
    Checking = 1,
    Device = 2,
    Working = 3,
    Success = 4,
    Error = 5,
}

#[derive(Clone, Debug)]
pub struct ScanProgress {
    pub status: ScanStatus,
    pub message: String,
}

#[derive(Clone, Debug)]
pub struct ScanResult {
    pub ok: bool,
    pub device: Option<PhilipsDevice>,
    pub recording: Option<Recording>,
    pub text: Option<String>,
    pub from_cache: bool,
    pub new_count: i32,
    pub error: Option<String>,
    pub open_settings: bool,
}

pub struct Coordinator {
    whisper: WhisperClient,
    store: TranscriptStore,
    settings: Mutex<AppSettings>,
    gate: Mutex<()>,
}

impl Coordinator {
    pub fn new() -> Self {
        Self {
            whisper: WhisperClient::new(),
            store: TranscriptStore::new(),
            settings: Mutex::new(AppSettings::load()),
            gate: Mutex::new(()),
        }
    }

    pub async fn reload_settings(&self) {
        *self.settings.lock().await = AppSettings::load();
    }

    pub async fn settings(&self) -> AppSettings {
        self.settings.lock().await.clone()
    }

    pub fn history(&self) -> Vec<TranscriptEntry> {
        self.store.all()
    }

    pub fn archive(&self, entry: &TranscriptEntry) -> Result<String> {
        let device = device::find().ok_or_else(|| anyhow::anyhow!("Diktiergerät nicht angeschlossen."))?;
        let moved = archive::move_to_old(&device.root, &entry.file_name)?;
        self.store
            .remove(&entry.volume_serial, &entry.file_name, entry.size);
        Ok(if moved {
            format!("{} nach old verschoben", entry.file_name)
        } else {
            format!("{} war nicht mehr im Root, aus Verlauf entfernt", entry.file_name)
        })
    }

    pub async fn scan_and_transcribe(
        &self,
        force: bool,
        mut progress: impl FnMut(ScanProgress),
    ) -> ScanResult {
        let _gate = self.gate.lock().await;
        self.reload_settings().await;
        let waiting = waiting_message();
        progress(ScanProgress {
            status: ScanStatus::Checking,
            message: match device::load_pairing() {
                Some(p) => format!("Suche {}…", p.label),
                None => "Suche Diktiergerät…".into(),
            },
        });

        let Some(device) = wait_for_device().await else {
            progress(ScanProgress {
                status: ScanStatus::Waiting,
                message: waiting,
            });
            return ScanResult {
                ok: false,
                device: None,
                recording: None,
                text: None,
                from_cache: false,
                new_count: 0,
                error: None,
                open_settings: false,
            };
        };

        let all = recordings::list_all(&device.root);
        if all.is_empty() {
            progress(ScanProgress {
                status: ScanStatus::Device,
                message: format!("Gerät · {} · keine Aufnahme", device.label),
            });
            return ScanResult {
                ok: false,
                device: Some(device),
                recording: None,
                text: None,
                from_cache: false,
                new_count: 0,
                error: None,
                open_settings: false,
            };
        }

        let newest = recordings::pick_newest(&device.root).unwrap_or_else(|| all.last().unwrap().clone());
        let pending: Vec<Recording> = all
            .iter()
            .filter(|r| {
                force || self.store.find(&device.volume_serial, &r.file_name, r.size).is_none()
            })
            .cloned()
            .collect();

        if pending.is_empty() {
            let cached = self
                .store
                .find(&device.volume_serial, &newest.file_name, newest.size);
            progress(ScanProgress {
                status: ScanStatus::Success,
                message: format!("Gerät · {} · gespeichert", newest.file_name),
            });
            return ScanResult {
                ok: true,
                device: Some(device),
                recording: Some(newest),
                text: cached.map(|c| c.text),
                from_cache: true,
                new_count: 0,
                error: None,
                open_settings: false,
            };
        }

        let settings = self.settings().await;
        if settings.api_key.trim().is_empty() {
            progress(ScanProgress {
                status: ScanStatus::Error,
                message: "API-Key fehlt oder ungültig.".into(),
            });
            return ScanResult {
                ok: false,
                device: Some(device),
                recording: pending.first().cloned(),
                text: None,
                from_cache: false,
                new_count: 0,
                error: Some("API-Key fehlt oder ungültig.".into()),
                open_settings: true,
            };
        }

        let mut last_done = None;
        let mut last_text = None;
        let mut done = 0;

        for (i, recording) in pending.iter().enumerate() {
            let n = i + 1;
            progress(ScanProgress {
                status: ScanStatus::Working,
                message: if pending.len() == 1 {
                    format!("Transkribiere {}…", recording.file_name)
                } else {
                    format!("Transkribiere {n}/{} · {}", pending.len(), recording.file_name)
                },
            });

            match self.transcribe_one(&device, recording, &settings).await {
                Ok(text) => {
                    last_done = Some(recording.clone());
                    last_text = Some(text);
                    done += 1;
                }
                Err(err) => {
                    if done == 0 {
                        let open_settings = err.status() == Some(401);
                        progress(ScanProgress {
                            status: ScanStatus::Error,
                            message: err.to_string(),
                        });
                        return ScanResult {
                            ok: false,
                            device: Some(device),
                            recording: Some(recording.clone()),
                            text: None,
                            from_cache: false,
                            new_count: 0,
                            error: Some(err.to_string()),
                            open_settings,
                        };
                    }
                    progress(ScanProgress {
                        status: ScanStatus::Error,
                        message: format!("{done} fertig, dann Fehler: {err}"),
                    });
                    return ScanResult {
                        ok: true,
                        device: Some(device),
                        recording: last_done,
                        text: last_text,
                        from_cache: false,
                        new_count: done,
                        error: Some(err.to_string()),
                        open_settings: false,
                    };
                }
            }
        }

        let last = last_done.clone().unwrap();
        let label = if done == 1 {
            format!("Gerät · {} · Fertig", last.file_name)
        } else {
            format!("{done} neu · {} · Fertig", last.file_name)
        };
        progress(ScanProgress {
            status: ScanStatus::Success,
            message: label,
        });
        ScanResult {
            ok: true,
            device: Some(device),
            recording: last_done,
            text: last_text,
            from_cache: false,
            new_count: done,
            error: None,
            open_settings: false,
        }
    }

    async fn transcribe_one(
        &self,
        device: &PhilipsDevice,
        recording: &Recording,
        settings: &AppSettings,
    ) -> Result<String, WhisperError> {
        let local = recordings::copy_to_inbox(recording)
            .await
            .map_err(|e| WhisperError::new(e.to_string(), None, ""))?;
        let text = self.whisper.transcribe(&local, settings).await?;
        let saved = self
            .store
            .save(&device.volume_serial, &recording.file_name, recording.size, &text);
        self.try_summarize(
            &device.volume_serial,
            &recording.file_name,
            recording.size,
            &text,
            settings,
        )
        .await;
        let updated = self
            .store
            .find(&device.volume_serial, &recording.file_name, recording.size)
            .unwrap_or(saved);
        let _ = export::try_write(&settings.transcript_folder, &updated);
        Ok(text)
    }

    pub async fn test_connection(&self) -> Result<(), WhisperError> {
        self.reload_settings().await;
        let settings = self.settings().await;
        self.whisper.test_connection(&settings).await
    }

    pub async fn fill_missing_summaries(&self) {
        let _gate = self.gate.lock().await;
        self.reload_settings().await;
        let settings = self.settings().await;
        if settings.api_key.trim().is_empty() {
            return;
        }
        let missing: Vec<_> = self
            .store
            .all()
            .into_iter()
            .filter(|e| {
                !e.text.trim().is_empty()
                    && (e.summary.as_deref().unwrap_or("").trim().is_empty()
                        || e.title.as_deref().unwrap_or("").trim().is_empty())
            })
            .collect();
        for entry in missing {
            self.try_summarize(
                &entry.volume_serial,
                &entry.file_name,
                entry.size,
                &entry.text,
                &settings,
            )
            .await;
            if let Some(updated) = self
                .store
                .find(&entry.volume_serial, &entry.file_name, entry.size)
            {
                let _ = export::try_write(&settings.transcript_folder, &updated);
            }
        }
    }

    async fn try_summarize(
        &self,
        volume_serial: &str,
        file_name: &str,
        size: i64,
        text: &str,
        settings: &AppSettings,
    ) {
        if text.trim().is_empty() {
            return;
        }
        match self.whisper.summarize(text, settings).await {
            Ok(headline)
                if !headline.summary.trim().is_empty() || !headline.title.trim().is_empty() =>
            {
                self.store.save_headline(
                    volume_serial,
                    file_name,
                    size,
                    Some(headline.title.as_str()).filter(|s| !s.trim().is_empty()),
                    &headline.summary,
                );
            }
            _ => {}
        }
    }

    pub fn export_all(&self) -> (usize, Option<String>) {
        let settings = AppSettings::load();
        match export::write_all(&settings.transcript_folder, &self.store.all()) {
            Ok(count) => (count, None),
            Err(err) => (0, Some(err)),
        }
    }
}

fn waiting_message() -> String {
    device::load_pairing()
        .map(|p| format!("Warte auf {}…", p.label))
        .unwrap_or_else(|| "Warte auf Diktiergerät…".into())
}

async fn wait_for_device() -> Option<PhilipsDevice> {
    for _ in 0..6 {
        if let Some(device) = device::find() {
            return Some(device);
        }
        tokio::time::sleep(std::time::Duration::from_millis(350)).await;
    }
    device::find()
}
