use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{LocalFree, HLOCAL};
use windows::Win32::Security::Cryptography::{
    CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN,
};

use crate::paths;

const ENTROPY: &[u8] = b"Diktator.Gutzeit.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default)]
    pub autostart: bool,
    #[serde(default)]
    pub transcript_folder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key_protected: Option<String>,
    #[serde(skip)]
    pub api_key: String,
}

fn default_language() -> String {
    "de".into()
}
fn default_model() -> String {
    "auto".into()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            language: default_language(),
            model: default_model(),
            autostart: false,
            transcript_folder: String::new(),
            api_key_protected: None,
            api_key: String::new(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        paths::ensure_directories();
        let mut settings = Self::default();
        let path = paths::settings_file();
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(mut loaded) = serde_json::from_str::<AppSettings>(&json) {
                loaded.api_key = unprotect(loaded.api_key_protected.as_deref());
                settings = loaded;
            }
        }
        if settings.language.trim().is_empty() {
            settings.language = default_language();
        }
        if settings.model.trim().is_empty() {
            settings.model = default_model();
        }
        if settings.api_key.trim().is_empty() {
            if let Ok(from_env) = std::env::var("DIKTATOR_API_KEY") {
                if !from_env.trim().is_empty() {
                    settings.api_key = from_env.trim().to_string();
                    let _ = settings.save();
                }
            }
        }
        settings
    }

    pub fn save(&mut self) -> Result<()> {
        paths::ensure_directories();
        self.api_key_protected = if self.api_key.trim().is_empty() {
            None
        } else {
            Some(protect(self.api_key.trim())?)
        };
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(paths::settings_file(), json)?;
        Ok(())
    }

    pub fn models_url(&self) -> String {
        self.sibling_url("models")
    }

    pub fn chat_url(&self) -> String {
        self.sibling_url("chat/completions")
    }

    fn sibling_url(&self, path: &str) -> String {
        let endpoint = self.endpoint.trim_end_matches('/');
        let lower = endpoint.to_ascii_lowercase();
        if let Some(idx) = lower.find("/audio/transcriptions") {
            format!("{}/{}", &endpoint[..idx], path)
        } else if lower.ends_with("/v1") {
            format!("{endpoint}/{path}")
        } else {
            format!("{endpoint}/{path}")
        }
    }
}

fn protect(plain: &str) -> Result<String> {
    let raw = unsafe { crypt_raw(plain.as_bytes(), true)? };
    Ok(STANDARD.encode(raw))
}

fn unprotect(value: Option<&str>) -> String {
    let Some(value) = value.filter(|s| !s.trim().is_empty()) else {
        return String::new();
    };
    let Ok(bytes) = STANDARD.decode(value.trim()) else {
        return String::new();
    };
    unsafe { crypt_raw(&bytes, false) }
        .ok()
        .and_then(|raw| String::from_utf8(raw).ok())
        .unwrap_or_default()
}

unsafe fn crypt_raw(input: &[u8], encrypt: bool) -> Result<Vec<u8>> {
    let mut data_in = CRYPT_INTEGER_BLOB {
        cbData: input.len() as u32,
        pbData: input.as_ptr() as *mut u8,
    };
    let entropy_blob = CRYPT_INTEGER_BLOB {
        cbData: ENTROPY.len() as u32,
        pbData: ENTROPY.as_ptr() as *mut u8,
    };
    let mut data_out = CRYPT_INTEGER_BLOB::default();
    let entropy_ptr = &entropy_blob as *const CRYPT_INTEGER_BLOB;
    if encrypt {
        CryptProtectData(
            &data_in,
            PCWSTR::null(),
            Some(entropy_ptr),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )?;
    } else {
        CryptUnprotectData(
            &mut data_in,
            None,
            Some(entropy_ptr),
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )?;
    }
    let slice = std::slice::from_raw_parts(data_out.pbData, data_out.cbData as usize);
    let owned = slice.to_vec();
    if !data_out.pbData.is_null() {
        let _ = LocalFree(HLOCAL(data_out.pbData as *mut _));
    }
    let _ = entropy_blob;
    Ok(owned)
}
