use anyhow::Result;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "Diktator";

pub fn is_enabled() -> bool {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(key) = hkcu.open_subkey_with_flags(RUN_KEY, KEY_READ) else {
        return false;
    };
    key.get_value::<String, _>(VALUE_NAME).is_ok()
}

pub fn set_enabled(enabled: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey_with_flags(RUN_KEY, KEY_READ | KEY_WRITE)?;
    if enabled {
        let exe = std::env::current_exe()?
            .to_string_lossy()
            .into_owned();
        key.set_value(VALUE_NAME, &format!("\"{exe}\""))?;
    } else {
        let _ = key.delete_value(VALUE_NAME);
    }
    Ok(())
}
