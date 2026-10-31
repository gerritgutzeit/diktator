use std::sync::{Arc, Mutex};
use std::time::Duration;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

use crate::autostart;
use crate::coordinator::{Coordinator, ScanProgress, ScanResult, ScanStatus};
use crate::settings::AppSettings;
use crate::store::TranscriptEntry;
use crate::updater;
use crate::device;
use crate::{AppWindow, DeviceCandidate, HistoryItem};

pub struct UiState {
    coordinator: Arc<Coordinator>,
    runtime: tokio::runtime::Handle,
    selected: Mutex<Option<(String, String, i64)>>,
}

impl UiState {
    pub fn new(runtime: tokio::runtime::Handle) -> Arc<Self> {
        Arc::new(Self {
            coordinator: Arc::new(Coordinator::new()),
            runtime,
            selected: Mutex::new(None),
        })
    }
}

pub fn bind(ui: &AppWindow, state: Arc<UiState>) {
    load_settings_into_form(ui, &state);
    refresh_history(ui, &state, None);
    ui.set_app_version(SharedString::from(updater::display_version()));

    let settings = AppSettings::load();
    if settings.api_key.trim().is_empty() {
        ui.set_settings_open(true);
    }

    bind_callbacks(ui, state.clone());
    start_background(ui, state);
}

fn bind_callbacks(ui: &AppWindow, state: Arc<UiState>) {
    {
        let ui_weak = ui.as_weak();
        let origin = Arc::new(Mutex::new(None::<((i32, i32), (i32, i32))>));
        let origin_move = origin.clone();
        ui.on_begin_drag({
            let ui_weak = ui_weak.clone();
            move || {
                if let Some(ui) = ui_weak.upgrade() {
                    let pos = ui.window().position();
                    *origin.lock().unwrap() = Some(((pos.x, pos.y), screen_cursor()));
                }
            }
        });
        ui.on_drag_by(move |_dx, _dy| {
            if let Some(ui) = ui_weak.upgrade() {
                let Some(((ox, oy), (cx, cy))) = *origin_move.lock().unwrap() else {
                    return;
                };
                let (x, y) = screen_cursor();
                ui.window().set_position(slint::PhysicalPosition::new(
                    ox + x - cx,
                    oy + y - cy,
                ));
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let origin = Arc::new(Mutex::new(None::<ResizeOrigin>));
        let origin_move = origin.clone();
        ui.on_begin_resize({
            let ui_weak = ui_weak.clone();
            move |edge| {
                if let Some(ui) = ui_weak.upgrade() {
                    let pos = ui.window().position();
                    let size = ui.window().size();
                    *origin.lock().unwrap() = Some(ResizeOrigin {
                        edge,
                        x: pos.x,
                        y: pos.y,
                        w: size.width as i32,
                        h: size.height as i32,
                        cx: screen_cursor().0,
                        cy: screen_cursor().1,
                    });
                }
            }
        });
        ui.on_resize_move(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let Some(start) = *origin_move.lock().unwrap() else {
                    return;
                };
                apply_resize(&ui, &start);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_toggle_history(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_history_open(!ui.get_history_open());
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_toggle_settings(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let open = !ui.get_settings_open();
                ui.set_settings_open(open);
                if open {
                    load_settings_into_form(&ui, &state);
                }
                ui.set_settings_message(SharedString::from(""));
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_transcribe_now(move || {
            run_scan(ui_weak.clone(), state.clone(), true);
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_copy_transcript(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let text = ui.get_transcript().to_string();
                if text.trim().is_empty() {
                    return;
                }
                set_clipboard(&text);
                flash_copied(&ui);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_current(move || {
            if let Some(ui) = ui_weak.upgrade() {
                delete_selected(&ui, &state, None);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_delete_history(move |index| {
            if let Some(ui) = ui_weak.upgrade() {
                delete_selected(&ui, &state, Some(index));
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_select_history(move |index| {
            if let Some(ui) = ui_weak.upgrade() {
                select_index(&ui, &state, index);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_save_settings(move || {
            if let Some(ui) = ui_weak.upgrade() {
                save_settings(&ui, &state);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_test_connection(move || {
            if let Some(ui) = ui_weak.upgrade() {
                save_settings(&ui, &state);
                ui.set_settings_message(SharedString::from("Prüfe Verbindung…"));
                let ui_weak = ui.as_weak();
                let coord = state.coordinator.clone();
                state.runtime.spawn(async move {
                    let message = match tokio::time::timeout(Duration::from_secs(20), coord.test_connection()).await
                    {
                        Ok(Ok(())) => "Verbindung ok.".to_string(),
                        Ok(Err(err)) => err.to_string(),
                        Err(_) => "Zeitüberschreitung.".to_string(),
                    };
                    invoke(ui_weak, move |ui| {
                        ui.set_settings_message(SharedString::from(message));
                    });
                });
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_browse_folder(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let mut dialog = rfd::FileDialog::new().set_title("Ordner für Transkripte");
                let current = ui.get_settings_transcript_folder().to_string();
                if !current.trim().is_empty() {
                    dialog = dialog.set_directory(&current);
                }
                if let Some(folder) = dialog.pick_folder() {
                    ui.set_settings_transcript_folder(SharedString::from(folder.to_string_lossy().as_ref()));
                    save_settings(&ui, &state);
                    export_all(&ui, &state);
                }
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_export_all(move || {
            if let Some(ui) = ui_weak.upgrade() {
                save_settings(&ui, &state);
                export_all(&ui, &state);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_open_folder(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let folder = ui.get_settings_transcript_folder().to_string();
                if folder.trim().is_empty() {
                    ui.invoke_browse_folder();
                    return;
                }
                let _ = std::fs::create_dir_all(&folder);
                let _ = std::process::Command::new("explorer").arg(&folder).spawn();
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_check_updates(move || {
            if let Some(ui) = ui_weak.upgrade() {
                ui.set_has_update(false);
                ui.set_update_status(SharedString::from("Prüfe auf Aktualisierung…"));
                let ui_weak = ui.as_weak();
                state.runtime.spawn_blocking(move || {
                    let result = updater::check();
                    invoke(ui_weak, move |ui| match result {
                        Ok(r) => {
                            ui.set_has_update(r.update_available);
                            ui.set_update_status(SharedString::from(r.message));
                        }
                        Err(err) => ui.set_update_status(SharedString::from(err.to_string())),
                    });
                });
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_pair_device(move |index| {
            if let Some(ui) = ui_weak.upgrade() {
                pair_device(&ui, &state, index);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_unpair_device(move || {
            if let Some(ui) = ui_weak.upgrade() {
                unpair_device(&ui, &state);
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        let state = state.clone();
        ui.on_install_update(move || {
            if let Some(ui) = ui_weak.upgrade() {
                if !ui.get_has_update() {
                    return;
                }
                ui.set_update_status(SharedString::from("Update wird geladen…"));
                let ui_weak = ui.as_weak();
                state.runtime.spawn_blocking(move || {
                    let ui_progress = ui_weak.clone();
                    let result = updater::apply(move |percent| {
                        invoke(ui_progress.clone(), move |ui| {
                            ui.set_update_status(SharedString::from(format!(
                                "Update wird geladen… {percent}%"
                            )));
                        });
                    });
                    if let Err(err) = result {
                        invoke(ui_weak, move |ui| {
                            ui.set_update_status(SharedString::from(err.to_string()));
                        });
                    }
                });
            }
        });
    }
    {
        let ui_weak = ui.as_weak();
        ui.on_request_hide(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let _ = ui.hide();
            }
        });
    }
    {
        ui.on_request_exit(move || {
            slint::quit_event_loop().ok();
        });
    }
}

fn start_background(ui: &AppWindow, state: Arc<UiState>) {
    let ui_weak = ui.as_weak();
    let state_scan = state.clone();
    crate::device::spawn_watcher(move || {
        run_scan(ui_weak.clone(), state_scan.clone(), true);
    });

    let ui_weak = ui.as_weak();
    let coord = state.coordinator.clone();
    let state2 = state.clone();
    state.runtime.spawn(async move {
        coord.fill_missing_summaries().await;
        invoke(ui_weak, move |ui| {
            refresh_history(&ui, &state2, None);
        });
    });
}

pub fn reveal(ui: &AppWindow) {
    let _ = ui.show();
    let pos = ui.window().position();
    if pos.x < 0 || pos.y < 0 || pos.x > 1920 {
        ui.window().set_position(slint::PhysicalPosition::new(80, 80));
    }
    ui.window().request_redraw();
}

pub fn run_scan(ui_weak: slint::Weak<AppWindow>, state: Arc<UiState>, show_on_new: bool) {
    invoke(ui_weak.clone(), |ui| {
        ui.set_busy(true);
        ui.set_error_text(SharedString::from(""));
    });
    let coord = state.coordinator.clone();
    let state_ui = state.clone();
    state.runtime.spawn(async move {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ScanProgress>();
        let ui_progress = ui_weak.clone();
        tokio::spawn(async move {
            while let Some(p) = rx.recv().await {
                let ui_progress = ui_progress.clone();
                invoke(ui_progress, move |ui| {
                    ui.set_status_kind(p.status as i32);
                    ui.set_status_text(SharedString::from(p.message));
                    ui.set_busy(matches!(p.status, ScanStatus::Working | ScanStatus::Checking));
                });
            }
        });
        let result = coord
            .scan_and_transcribe(false, |p| {
                let _ = tx.send(p);
            })
            .await;
        apply_result(ui_weak, state_ui, result, show_on_new).await;
    });
}

async fn apply_result(
    ui_weak: slint::Weak<AppWindow>,
    state: Arc<UiState>,
    result: ScanResult,
    show_on_new: bool,
) {
    let coord = state.coordinator.clone();
    let _ = coord.fill_missing_summaries().await;
    invoke(ui_weak, move |ui| {
        ui.set_busy(false);
        if result.open_settings {
            ui.set_settings_open(true);
            reveal(&ui);
        }
        if !result.ok && result.new_count == 0 {
            if let Some(error) = result.error.clone() {
                ui.set_error_text(SharedString::from(error.clone()));
                ui.set_status_kind(ScanStatus::Error as i32);
                ui.set_status_text(SharedString::from(error));
                reveal(&ui);
                return;
            }
            if result.device.is_none() {
                ui.set_status_kind(ScanStatus::Waiting as i32);
                ui.set_status_text(SharedString::from("Warte auf Diktiergerät…"));
            }
            return;
        }

        refresh_history(&ui, &state, result.recording.as_ref().map(|r| r.file_name.as_str()));
        if let Some(text) = result.text.clone().filter(|t| !t.trim().is_empty()) {
            ui.set_transcript(SharedString::from(text.clone()));
            ui.set_has_transcript(true);
            if result.from_cache || result.new_count == 0 {
                return;
            }
            set_clipboard(&text);
            flash_copied(&ui);
            ui.set_history_open(true);
            if show_on_new {
                reveal(&ui);
            }
        }
        if let Some(error) = result.error {
            ui.set_error_text(SharedString::from(error));
        }
    });
}

fn load_settings_into_form(ui: &AppWindow, _state: &UiState) {
    let settings = AppSettings::load();
    ui.set_settings_endpoint(SharedString::from(settings.endpoint));
    ui.set_settings_api_key(SharedString::from(settings.api_key));
    ui.set_settings_language(SharedString::from(settings.language));
    ui.set_settings_autostart(settings.autostart || autostart::is_enabled());
    ui.set_settings_transcript_folder(SharedString::from(settings.transcript_folder));
    refresh_device_settings(ui);
}

fn refresh_device_settings(ui: &AppWindow) {
    let pairing = device::load_pairing();
    let volumes = device::list_volumes();
    let paired = pairing.is_some();
    ui.set_settings_has_paired(paired);
    let summary = match &pairing {
        Some(p) => {
            let attached = volumes.iter().any(|v| {
                v.marker_id.as_deref() == Some(p.id.as_str())
                    || v.serial.eq_ignore_ascii_case(&p.serial)
            });
            if attached {
                format!("Gekoppelt: {} · angeschlossen", p.label)
            } else {
                format!("Gekoppelt: {} · nicht angeschlossen", p.label)
            }
        }
        None => "Noch kein Gerät gekoppelt. Es gilt die automatische Philips-Erkennung.".into(),
    };
    ui.set_settings_device_summary(SharedString::from(summary));
    let rows: Vec<DeviceCandidate> = volumes
        .iter()
        .map(|v| {
            let is_paired = pairing.as_ref().is_some_and(|p| {
                v.marker_id.as_deref() == Some(p.id.as_str())
                    || v.serial.eq_ignore_ascii_case(&p.serial)
            });
            let root = v.root.to_string_lossy().trim_end_matches(['\\', '/']).to_string();
            DeviceCandidate {
                root: SharedString::from(root.clone()),
                title: SharedString::from(v.display_label()),
                detail: SharedString::from(format!("{root} · Volume {}", v.serial)),
                paired: is_paired,
            }
        })
        .collect();
    ui.set_settings_devices(ModelRc::new(VecModel::from(rows)));
}

fn pair_device(ui: &AppWindow, state: &UiState, index: i32) {
    let volumes = device::list_volumes();
    let Some(volume) = volumes.get(index as usize) else {
        ui.set_settings_message(SharedString::from("Laufwerk nicht gefunden. Neu anschließen und Einstellungen erneut öffnen."));
        refresh_device_settings(ui);
        return;
    };
    match device::pair(volume) {
        Ok(pairing) => {
            let coord = state.coordinator.clone();
            state.runtime.spawn(async move {
                coord.reload_settings().await;
            });
            ui.set_settings_message(SharedString::from(format!(
                "{} gekoppelt. Versteckte Datei .diktator liegt auf dem Gerät.",
                pairing.label
            )));
            refresh_device_settings(ui);
        }
        Err(err) => ui.set_settings_message(SharedString::from(format!(
            "Koppeln fehlgeschlagen: {err}"
        ))),
    }
}

fn unpair_device(ui: &AppWindow, state: &UiState) {
    match device::unpair() {
        Ok(()) => {
            let coord = state.coordinator.clone();
            state.runtime.spawn(async move {
                coord.reload_settings().await;
            });
            ui.set_settings_message(SharedString::from(
                "Kopplung gelöst. Es gilt wieder die automatische Philips-Erkennung.",
            ));
            refresh_device_settings(ui);
        }
        Err(err) => ui.set_settings_message(SharedString::from(err.to_string())),
    }
}

fn save_settings(ui: &AppWindow, state: &UiState) {
    let mut settings = AppSettings::load();
    let endpoint = ui.get_settings_endpoint().to_string();
    settings.endpoint = endpoint.trim().to_string();
    settings.api_key = ui.get_settings_api_key().to_string().trim().to_string();
    let language = ui.get_settings_language().to_string();
    settings.language = if language.trim().is_empty() {
        "de".into()
    } else {
        language.trim().to_string()
    };
    settings.autostart = ui.get_settings_autostart();
    settings.transcript_folder = ui.get_settings_transcript_folder().to_string().trim().to_string();
    match settings.save() {
        Ok(()) => {
            let _ = autostart::set_enabled(settings.autostart);
            let coord = state.coordinator.clone();
            state.runtime.spawn(async move {
                coord.reload_settings().await;
            });
            ui.set_settings_message(SharedString::from("Gespeichert."));
            ui.set_error_text(SharedString::from(""));
        }
        Err(err) => ui.set_settings_message(SharedString::from(err.to_string())),
    }
}

fn export_all(ui: &AppWindow, state: &UiState) {
    let folder = ui.get_settings_transcript_folder().to_string();
    let (count, error) = state.coordinator.export_all();
    if let Some(error) = error {
        if count == 0 {
            ui.set_settings_message(SharedString::from(error));
            return;
        }
    }
    let message = if count == 0 {
        format!("Ordner gesetzt, noch keine Transkripte: {folder}")
    } else {
        format!("{count} Transkripte abgelegt in {folder}")
    };
    ui.set_settings_message(SharedString::from(message));
}

fn refresh_history(ui: &AppWindow, state: &UiState, select_file: Option<&str>) {
    let items = state.coordinator.history();
    let selected = state.selected.lock().unwrap().clone();
    let mut selected_index = -1i32;
    for (i, item) in items.iter().enumerate() {
        let is_selected = if let Some(name) = select_file {
            item.file_name.eq_ignore_ascii_case(name)
        } else if let Some((serial, name, size)) = &selected {
            item.same_as(serial, name, *size)
        } else {
            false
        };
        if is_selected {
            selected_index = i as i32;
        }
    }
    if selected_index < 0 && !items.is_empty() {
        selected_index = 0;
    }
    if selected_index >= 0 {
        if let Some(item) = items.get(selected_index as usize) {
            *state.selected.lock().unwrap() =
                Some((item.volume_serial.clone(), item.file_name.clone(), item.size));
        }
    }
    let rows: Vec<_> = items
        .iter()
        .enumerate()
        .map(|(i, item)| HistoryItem {
            file_name: SharedString::from(item.file_name.clone()),
            summary: SharedString::from(item.list_summary()),
            when_label: SharedString::from(item.when_label()),
            selected: i as i32 == selected_index,
        })
        .collect();
    ui.set_history(ModelRc::new(VecModel::from(rows)));
    ui.set_selected_index(selected_index);
    if selected_index >= 0 {
        if let Some(item) = items.get(selected_index as usize) {
            apply_entry(ui, item);
        }
    } else {
        ui.set_transcript(SharedString::from(""));
        ui.set_has_transcript(false);
        ui.set_current_file(SharedString::from(""));
        ui.set_current_when(SharedString::from(""));
    }
}

fn select_index(ui: &AppWindow, state: &UiState, index: i32) {
    let items = state.coordinator.history();
    if let Some(item) = items.get(index as usize) {
        *state.selected.lock().unwrap() =
            Some((item.volume_serial.clone(), item.file_name.clone(), item.size));
        apply_entry(ui, item);
        refresh_history(ui, state, None);
        ui.set_settings_open(false);
    }
}

fn apply_entry(ui: &AppWindow, item: &TranscriptEntry) {
    ui.set_transcript(SharedString::from(item.text.clone()));
    ui.set_has_transcript(!item.text.trim().is_empty());
    ui.set_current_file(SharedString::from(item.file_name.clone()));
    ui.set_current_when(SharedString::from(item.when_label()));
    ui.set_status_text(SharedString::from(format!(
        "{} · {}",
        item.file_name,
        item.when_label()
    )));
    ui.set_status_kind(ScanStatus::Success as i32);
    ui.set_error_text(SharedString::from(""));
}

fn delete_selected(ui: &AppWindow, state: &UiState, index: Option<i32>) {
    let items = state.coordinator.history();
    let entry = if let Some(index) = index {
        items.get(index as usize).cloned()
    } else {
        let selected = state.selected.lock().unwrap().clone();
        selected.and_then(|(serial, name, size)| {
            items.into_iter().find(|e| e.same_as(&serial, &name, size))
        })
    };
    let Some(entry) = entry else {
        return;
    };
    match state.coordinator.archive(&entry) {
        Ok(message) => {
            *state.selected.lock().unwrap() = None;
            refresh_history(ui, state, None);
            let remaining = state.coordinator.history();
            if remaining.is_empty() {
                ui.set_transcript(SharedString::from(""));
                ui.set_has_transcript(false);
                ui.set_current_file(SharedString::from(""));
                ui.set_current_when(SharedString::from(""));
            } else {
                *state.selected.lock().unwrap() = Some((
                    remaining[0].volume_serial.clone(),
                    remaining[0].file_name.clone(),
                    remaining[0].size,
                ));
                refresh_history(ui, state, None);
            }
            ui.set_error_text(SharedString::from(""));
            ui.set_status_kind(ScanStatus::Success as i32);
            ui.set_status_text(SharedString::from(message));
        }
        Err(err) => {
            ui.set_error_text(SharedString::from(err.to_string()));
            ui.set_status_kind(ScanStatus::Error as i32);
            ui.set_status_text(SharedString::from(err.to_string()));
        }
    }
}

#[derive(Clone, Copy)]
struct ResizeOrigin {
    edge: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    cx: i32,
    cy: i32,
}

fn apply_resize(ui: &AppWindow, start: &ResizeOrigin) {
    let (mx, my) = screen_cursor();
    let dx = mx - start.cx;
    let dy = my - start.cy;
    let scale = ui.window().scale_factor();
    let min_w = (640.0 * scale).round() as i32;
    let min_h = (420.0 * scale).round() as i32;

    let west = matches!(start.edge, 1 | 5 | 7);
    let east = matches!(start.edge, 2 | 6 | 8);
    let north = matches!(start.edge, 3 | 5 | 6);
    let south = matches!(start.edge, 4 | 7 | 8);

    let mut x = start.x;
    let mut y = start.y;
    let mut w = start.w;
    let mut h = start.h;

    if east {
        w = (start.w + dx).max(min_w);
    }
    if west {
        w = (start.w - dx).max(min_w);
        x = start.x + start.w - w;
    }
    if south {
        h = (start.h + dy).max(min_h);
    }
    if north {
        h = (start.h - dy).max(min_h);
        y = start.y + start.h - h;
    }

    ui.window()
        .set_position(slint::PhysicalPosition::new(x, y));
    ui.window()
        .set_size(slint::PhysicalSize::new(w as u32, h as u32));
}

fn screen_cursor() -> (i32, i32) {
    let mut pt = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut pt);
    }
    (pt.x, pt.y)
}

fn set_clipboard(text: &str) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

fn flash_copied(ui: &AppWindow) {
    ui.set_copy_label(SharedString::from("Kopiert"));
    let ui_weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_millis(1800), move || {
        if let Some(ui) = ui_weak.upgrade() {
            ui.set_copy_label(SharedString::from("Kopieren"));
        }
    });
}

fn invoke(ui_weak: slint::Weak<AppWindow>, f: impl FnOnce(AppWindow) + Send + 'static) {
    slint::invoke_from_event_loop(move || {
        if let Some(ui) = ui_weak.upgrade() {
            f(ui);
        }
    })
    .ok();
}
