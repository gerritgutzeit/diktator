#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod archive;
mod autostart;
mod coordinator;
mod device;
mod export;
mod instance;
mod paths;
mod recordings;
mod settings;
mod store;
mod tray;
mod updater;
mod whisper;

slint::include_modules!();

fn main() -> anyhow::Result<()> {
    velopack::VelopackApp::build().run();

    let Some(instance) = instance::SingleInstance::acquire()? else {
        return Ok(());
    };

    crate::paths::ensure_directories();
    let settings = crate::settings::AppSettings::load();
    if settings.autostart {
        let _ = crate::autostart::set_enabled(true);
    }

    let runtime = tokio::runtime::Runtime::new()?;
    let handle = runtime.handle().clone();

    let ui = AppWindow::new()?;
    let state = app::UiState::new(handle);
    {
        let ui_weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let _ = ui.hide();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
    }
    app::bind(&ui, state.clone());

    {
        let ui_weak = ui.as_weak();
        instance.listen_for_show(move || {
            let ui_weak = ui_weak.clone();
            slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    app::reveal(&ui);
                }
            })
            .ok();
        });
    }

    let tray = tray::create()?;
    {
        let ui_weak = ui.as_weak();
        let ui_transcribe = ui.as_weak();
        let ui_settings = ui.as_weak();
        let state_scan = state.clone();
        tray::spawn_event_loop(
            tray,
            {
                let ui_weak = ui_weak.clone();
                move || {
                    let ui_weak = ui_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            app::reveal(&ui);
                        }
                    })
                    .ok();
                }
            },
            {
                let ui_weak = ui_transcribe.clone();
                move || {
                    let ui_weak = ui_weak.clone();
                    let state_scan = state_scan.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            app::reveal(&ui);
                            crate::app::run_scan(ui.as_weak(), state_scan.clone(), true);
                        }
                    })
                    .ok();
                }
            },
            {
                let ui_weak = ui_settings.clone();
                move || {
                    let ui_weak = ui_weak.clone();
                    slint::invoke_from_event_loop(move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            app::reveal(&ui);
                            if !ui.get_settings_open() {
                                ui.set_settings_open(true);
                            }
                        }
                    })
                    .ok();
                }
            },
            || {
                slint::quit_event_loop().ok();
            },
        );
    }

    ui.window().set_size(slint::LogicalSize::new(740.0, 500.0));
    ui.window().set_position(slint::PhysicalPosition::new(80, 80));
    let _ = ui.show();
    if settings.api_key.trim().is_empty() || cfg!(debug_assertions) {
        app::reveal(&ui);
    } else {
        let _ = ui.hide();
    }

    slint::run_event_loop()?;
    drop(runtime);
    Ok(())
}
