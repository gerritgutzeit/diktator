use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use windows::core::{w, GUID, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetFileAttributesW, GetLogicalDriveStringsW, GetVolumeInformationW,
    SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES, INVALID_FILE_ATTRIBUTES,
};

const DRIVE_REMOVABLE: u32 = 2;
const DRIVE_FIXED: u32 = 3;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, PostQuitMessage,
    RegisterClassW, RegisterDeviceNotificationW, TranslateMessage, CS_HREDRAW, CS_VREDRAW,
    DEVICE_NOTIFY_WINDOW_HANDLE, MSG, WM_DEVICECHANGE, WM_DESTROY, WNDCLASSW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

const DBT_DEVNODES_CHANGED: usize = 0x0007;
const DBT_DEVICEARRIVAL: usize = 0x8000;
const DBT_DEVICEREMOVECOMPLETE: usize = 0x8004;
const DBT_DEVTYP_DEVICEINTERFACE: u32 = 0x0000_0005;
pub const MARKER_NAME: &str = ".diktator";

const GUID_DEVINTERFACE_VOLUME: GUID = GUID::from_u128(0x53F5630D_B6BF_11D0_94F2_00A0C91EFB8B);
const GUID_DEVINTERFACE_DISK: GUID = GUID::from_u128(0x53F56307_B6BF_11D0_94F2_00A0C91EFB8B);
const GUID_DEVINTERFACE_USB: GUID = GUID::from_u128(0xA5DCBF10_6530_11D2_901F_00C04FB951ED);

static ON_CHANGE: OnceLock<Arc<dyn Fn() + Send + Sync>> = OnceLock::new();
static DEBOUNCE_GEN: AtomicU64 = AtomicU64::new(0);
static LAST_PRESENT: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct DevBroadcastDeviceInterface {
    size: u32,
    device_type: u32,
    reserved: u32,
    classguid: GUID,
    name: u16,
}

#[derive(Debug, Clone)]
pub struct PhilipsDevice {
    pub root: PathBuf,
    pub label: String,
    pub volume_serial: String,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub root: PathBuf,
    pub label: String,
    pub serial: String,
    pub marker_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevicePairing {
    pub id: String,
    pub label: String,
    pub serial: String,
}

impl Volume {
    pub fn display_label(&self) -> String {
        if self.label.trim().is_empty() {
            self.root.to_string_lossy().trim_end_matches(['\\', '/']).to_string()
        } else {
            self.label.clone()
        }
    }
}

pub fn load_pairing() -> Option<DevicePairing> {
    let json = fs::read_to_string(crate::paths::device_file()).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn save_pairing(pairing: &DevicePairing) -> anyhow::Result<()> {
    crate::paths::ensure_directories();
    let json = serde_json::to_string_pretty(pairing)?;
    fs::write(crate::paths::device_file(), json)?;
    Ok(())
}

pub fn clear_pairing() {
    let _ = fs::remove_file(crate::paths::device_file());
}

pub fn list_volumes() -> Vec<Volume> {
    enumerate_volumes(false)
}

pub fn find() -> Option<PhilipsDevice> {
    let volumes = enumerate_volumes(true);
    if let Some(pairing) = load_pairing() {
        return volumes
            .into_iter()
            .find(|v| matches_pairing(v, &pairing))
            .map(into_device);
    }
    volumes.into_iter().find(is_dictation_device).map(into_device)
}

fn enumerate_volumes(include_plain_fixed: bool) -> Vec<Volume> {
    candidate_roots()
        .into_iter()
        .filter_map(|(root, kind)| {
            let (label, serial) = volume_info(&root)?;
            let marker_id = read_marker(&root);
            let volume = Volume {
                marker_id: marker_id.clone(),
                root,
                label,
                serial,
            };
            if kind == DRIVE_REMOVABLE {
                return Some(volume);
            }
            if include_plain_fixed {
                return Some(volume);
            }
            if is_dictation_device(&volume) {
                return Some(volume);
            }
            None
        })
        .collect()
}

pub fn pair(volume: &Volume) -> anyhow::Result<DevicePairing> {
    let id = new_pair_id();
    write_marker(&volume.root, &id)?;
    let pairing = DevicePairing {
        id,
        label: volume.display_label(),
        serial: volume.serial.clone(),
    };
    save_pairing(&pairing)?;
    Ok(pairing)
}

pub fn unpair() -> anyhow::Result<()> {
    if let Some(pairing) = load_pairing() {
        for volume in list_volumes() {
            if matches_pairing(&volume, &pairing) {
                let _ = fs::remove_file(volume.root.join(MARKER_NAME));
            }
        }
    }
    clear_pairing();
    Ok(())
}

fn matches_pairing(volume: &Volume, pairing: &DevicePairing) -> bool {
    if !pairing.id.is_empty() {
        if volume.marker_id.as_deref() == Some(pairing.id.as_str()) {
            return true;
        }
    }
    !pairing.serial.is_empty() && volume.serial.eq_ignore_ascii_case(&pairing.serial)
}

fn is_dictation_device(volume: &Volume) -> bool {
    volume.label.eq_ignore_ascii_case("PHILIPS")
        || volume.marker_id.is_some()
        || is_philips_usb(&volume.root)
        || has_dictation_files(&volume.root)
}

fn into_device(volume: Volume) -> PhilipsDevice {
    PhilipsDevice {
        label: volume.display_label(),
        root: volume.root,
        volume_serial: volume.serial,
    }
}

fn new_pair_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = u128::from(std::process::id());
    format!("{:032x}", nanos ^ (pid << 72))
}

fn marker_path(root: &Path) -> PathBuf {
    root.join(MARKER_NAME)
}

fn read_marker(root: &Path) -> Option<String> {
    let text = fs::read_to_string(marker_path(root)).ok()?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.eq_ignore_ascii_case("diktator-v1") {
            continue;
        }
        return Some(line.to_string());
    }
    None
}

fn write_marker(root: &Path, id: &str) -> anyhow::Result<()> {
    let path = marker_path(root);
    fs::write(&path, format!("diktator-v1\n{id}\n"))?;
    hide_file(&path);
    Ok(())
}

fn hide_file(path: &Path) {
    let wide = wide_os_path(path);
    unsafe {
        let current = GetFileAttributesW(PCWSTR(wide.as_ptr()));
        if current == INVALID_FILE_ATTRIBUTES {
            let _ = SetFileAttributesW(PCWSTR(wide.as_ptr()), FILE_ATTRIBUTE_HIDDEN);
            return;
        }
        let _ = SetFileAttributesW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(current | FILE_ATTRIBUTE_HIDDEN.0),
        );
    }
}

fn wide_os_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn candidate_roots() -> Vec<(PathBuf, u32)> {
    unsafe {
        let mut buf = vec![0u16; 256];
        let written = GetLogicalDriveStringsW(Some(&mut buf));
        if written == 0 {
            return Vec::new();
        }
        if written as usize > buf.len() {
            buf.resize(written as usize, 0);
            if GetLogicalDriveStringsW(Some(&mut buf)) == 0 {
                return Vec::new();
            }
        }
        buf.split(|c| *c == 0)
            .filter(|s| !s.is_empty())
            .filter_map(|s| {
                let drive = String::from_utf16_lossy(s);
                let wide: Vec<u16> = s.iter().copied().chain(std::iter::once(0)).collect();
                let kind = GetDriveTypeW(PCWSTR(wide.as_ptr()));
                let root = PathBuf::from(drive);
                if kind == DRIVE_REMOVABLE {
                    return Some((root, kind));
                }
                if kind == DRIVE_FIXED && !is_system_drive(&root) {
                    return Some((root, kind));
                }
                None
            })
            .collect()
    }
}

fn is_system_drive(root: &Path) -> bool {
    let letter = root.to_string_lossy().chars().next();
    let sys = std::env::var("SystemDrive")
        .ok()
        .and_then(|s| s.chars().next());
    match (letter, sys) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(&b),
        _ => false,
    }
}

fn has_dictation_files(root: &Path) -> bool {
    let Ok(entries) = fs::read_dir(root) else {
        return false;
    };
    entries.flatten().take(64).any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("._") {
            return false;
        }
        let lower = name.to_ascii_lowercase();
        lower.ends_with(".mp3") && (lower.starts_with("gutz") || lower.starts_with("dss"))
    })
}

fn volume_info(root: &std::path::Path) -> Option<(String, String)> {
    let mut label = [0u16; 261];
    let mut serial = 0u32;
    let mut max_comp = 0u32;
    let mut flags = 0u32;
    let path = wide_path(root);
    let ok = unsafe {
        GetVolumeInformationW(
            PCWSTR(path.as_ptr()),
            Some(&mut label),
            Some(&mut serial),
            Some(&mut max_comp),
            Some(&mut flags),
            None,
        )
    };
    if ok.is_err() {
        return None;
    }
    let name = String::from_utf16_lossy(&label)
        .trim_end_matches('\0')
        .trim()
        .to_string();
    Some((name, format!("{serial:08X}")))
}

fn is_philips_usb(root: &std::path::Path) -> bool {
    match wmi_is_philips(root) {
        Ok(v) => v,
        Err(_) => false,
    }
}

fn wmi_is_philips(root: &std::path::Path) -> anyhow::Result<bool> {
    let letter = root
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_string();
    let com = wmi::COMLibrary::new()?;
    let wmi = wmi::WMIConnection::new(com)?;
    let partitions: Vec<std::collections::HashMap<String, wmi::Variant>> = wmi.raw_query(&format!(
        "ASSOCIATORS OF {{Win32_LogicalDisk.DeviceID='{letter}'}} WHERE AssocClass = Win32_LogicalDiskToPartition"
    ))?;
    for partition in partitions {
        let Some(partition_id) = variant_str(partition.get("DeviceID")).filter(|s| !s.is_empty())
        else {
            continue;
        };
        let disks: Vec<std::collections::HashMap<String, wmi::Variant>> = wmi.raw_query(&format!(
            "ASSOCIATORS OF {{Win32_DiskPartition.DeviceID='{partition_id}'}} WHERE AssocClass = Win32_DiskDriveToDiskPartition"
        ))?;
        for disk in disks {
            let blob = format!(
                "{} {} {}",
                variant_str(disk.get("PNPDeviceID")).unwrap_or_default(),
                variant_str(disk.get("Model")).unwrap_or_default(),
                variant_str(disk.get("Caption")).unwrap_or_default()
            )
            .to_ascii_uppercase();
            if blob.contains("VID_0911") || blob.contains("DPM4") || blob.contains("PHILIPS") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn variant_str(v: Option<&wmi::Variant>) -> Option<String> {
    match v {
        Some(wmi::Variant::String(s)) => Some(s.clone()),
        Some(other) => Some(format!("{other:?}")),
        None => None,
    }
}

fn wide_path(path: &std::path::Path) -> Vec<u16> {
    let mut text = path.as_os_str().to_string_lossy().into_owned();
    if !text.ends_with('\\') {
        text.push('\\');
    }
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn spawn_watcher(on_change: impl Fn() + Send + Sync + 'static) {
    let on_change = Arc::new(on_change);
    let _ = ON_CHANGE.set(on_change.clone());
    LAST_PRESENT.store(find().is_some(), Ordering::SeqCst);
    thread::spawn(|| {
        if let Err(err) = run_message_window() {
            eprintln!("device watcher: {err}");
        }
    });
    thread::spawn(|| loop {
        thread::sleep(Duration::from_secs(2));
        let present = find().is_some();
        let was = LAST_PRESENT.swap(present, Ordering::SeqCst);
        if present != was {
            notify_change();
        }
    });
    schedule_scan();
}

fn notify_change() {
    LAST_PRESENT.store(find().is_some(), Ordering::SeqCst);
    if let Some(cb) = ON_CHANGE.get() {
        cb();
    }
}

fn schedule_scan() {
    let gen = DEBOUNCE_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(600));
        for _ in 0..16 {
            if DEBOUNCE_GEN.load(Ordering::SeqCst) != gen {
                return;
            }
            if find().is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(400));
        }
        if DEBOUNCE_GEN.load(Ordering::SeqCst) != gen {
            return;
        }
        notify_change();
    });
}

fn run_message_window() -> anyhow::Result<()> {
    unsafe {
        let class_name = w!("DiktatorDeviceWatch");
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            lpszClassName: class_name,
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name,
            w!("DiktatorDeviceWatch"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            HWND::default(),
            None,
            instance,
            None,
        )?;
        register_interface(hwnd, GUID_DEVINTERFACE_VOLUME);
        register_interface(hwnd, GUID_DEVINTERFACE_DISK);
        register_interface(hwnd, GUID_DEVINTERFACE_USB);
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Ok(())
    }
}

unsafe fn register_interface(hwnd: HWND, guid: GUID) {
    let mut filter = DevBroadcastDeviceInterface {
        size: std::mem::size_of::<DevBroadcastDeviceInterface>() as u32,
        device_type: DBT_DEVTYP_DEVICEINTERFACE,
        reserved: 0,
        classguid: guid,
        name: 0,
    };
    let _ = RegisterDeviceNotificationW(
        HANDLE(hwnd.0),
        &mut filter as *mut DevBroadcastDeviceInterface as *const std::ffi::c_void,
        DEVICE_NOTIFY_WINDOW_HANDLE,
    );
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DEVICECHANGE => {
            let event = wparam.0;
            if event == DBT_DEVICEARRIVAL
                || event == DBT_DEVICEREMOVECOMPLETE
                || event == DBT_DEVNODES_CHANGED
            {
                schedule_scan();
            }
            LRESULT(1)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}
