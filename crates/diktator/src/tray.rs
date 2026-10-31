use anyhow::Result;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::UI::HiDpi::GetDpiForSystem;

pub struct TrayHandles {
    pub open: MenuItem,
    pub transcribe: MenuItem,
    pub settings: MenuItem,
    pub quit: MenuItem,
    _tray: TrayIcon,
}

pub fn create() -> Result<TrayHandles> {
    let open = MenuItem::new("Öffnen", true, None);
    let transcribe = MenuItem::new("Jetzt transkribieren", true, None);
    let settings = MenuItem::new("Einstellungen", true, None);
    let quit = MenuItem::new("Beenden", true, None);
    let menu = Menu::new();
    menu.append(&open)?;
    menu.append(&transcribe)?;
    menu.append(&settings)?;
    menu.append(&quit)?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Diktator")
        .with_icon(load_icon())
        .build()?;

    Ok(TrayHandles {
        open,
        transcribe,
        settings,
        quit,
        _tray: tray,
    })
}

pub fn spawn_event_loop(
    handles: TrayHandles,
    on_open: impl Fn() + Send + Sync + 'static,
    on_transcribe: impl Fn() + Send + Sync + 'static,
    on_settings: impl Fn() + Send + Sync + 'static,
    on_quit: impl Fn() + Send + Sync + 'static,
) {
    let open_id = handles.open.id().clone();
    let transcribe_id = handles.transcribe.id().clone();
    let settings_id = handles.settings.id().clone();
    let quit_id = handles.quit.id().clone();
    std::mem::forget(handles);
    std::thread::spawn(move || loop {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == open_id {
                on_open();
            } else if event.id == transcribe_id {
                on_transcribe();
            } else if event.id == settings_id {
                on_settings();
            } else if event.id == quit_id {
                on_quit();
            }
        }
        if TrayIconEvent::receiver().try_recv().is_ok() {
            on_open();
        }
        std::thread::sleep(std::time::Duration::from_millis(80));
    });
}

fn load_icon() -> Icon {
    let bytes = include_bytes!("../../../src/Diktator/Assets/app-48.png");
    let img = image::load_from_memory(bytes).unwrap_or_else(|_| {
        image::DynamicImage::new_rgba8(32, 32)
    });
    let dpi = unsafe { GetDpiForSystem() } as f32;
    let size = ((16.0 * dpi / 96.0).round() as u32).clamp(16, 48);
    let resized = image::imageops::resize(
        &img.to_rgba8(),
        size,
        size,
        image::imageops::FilterType::Lanczos3,
    );
    let (w, h) = resized.dimensions();
    Icon::from_rgba(resized.into_raw(), w, h).unwrap_or_else(|_| {
        Icon::from_rgba(vec![0; 16 * 16 * 4], 16, 16).expect("fallback icon")
    })
}
