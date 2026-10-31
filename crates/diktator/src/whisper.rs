use anyhow::Result;
use reqwest::multipart::{Form, Part};
use serde_json::Value;
use thiserror::Error;

use crate::settings::AppSettings;

#[derive(Debug, Error)]
pub enum WhisperError {
    #[error("{message}")]
    Message {
        message: String,
        status: Option<u16>,
        error_type: String,
    },
}

impl WhisperError {
    pub fn new(message: impl Into<String>, status: Option<u16>, error_type: impl Into<String>) -> Self {
        Self::Message {
            message: message.into(),
            status,
            error_type: error_type.into(),
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Self::Message { status, .. } => *status,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranscriptHeadline {
    pub title: String,
    pub summary: String,
}

#[derive(Clone)]
pub struct WhisperClient {
    http: reqwest::Client,
}

impl WhisperClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30 * 60))
            .build()
            .expect("http client");
        Self { http }
    }

    pub async fn transcribe(&self, file_path: &std::path::Path, settings: &AppSettings) -> Result<String, WhisperError> {
        require_key(settings)?;
        let bytes = tokio::fs::read(file_path)
            .await
            .map_err(|e| WhisperError::new(e.to_string(), None, ""))?;
        let file_name = file_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("audio.mp3")
            .to_string();
        let ext = file_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp3")
            .to_ascii_lowercase();
        let mime = if ext == "wav" { "audio/wav" } else { "audio/mpeg" };
        let part = Part::bytes(bytes)
            .file_name(file_name)
            .mime_str(mime)
            .map_err(|e| WhisperError::new(e.to_string(), None, ""))?;
        let model = if settings.model.trim().is_empty() {
            "auto".to_string()
        } else {
            settings.model.clone()
        };
        let mut form = Form::new().part("file", part).text("model", model);
        if !settings.language.trim().is_empty() {
            form = form.text("language", settings.language.clone());
        }
        let response = self
            .http
            .post(&settings.endpoint)
            .bearer_auth(settings.api_key.trim())
            .multipart(form)
            .send()
            .await
            .map_err(map_request)?;
        let status = response.status();
        let body = response.text().await.map_err(map_request)?;
        if !status.is_success() {
            return Err(to_error(status.as_u16(), &body));
        }
        Ok(parse_text(&body))
    }

    pub async fn test_connection(&self, settings: &AppSettings) -> Result<(), WhisperError> {
        require_key(settings)?;
        let response = self
            .http
            .get(settings.models_url())
            .bearer_auth(settings.api_key.trim())
            .send()
            .await
            .map_err(map_request)?;
        let status = response.status();
        let body = response.text().await.map_err(map_request)?;
        if !status.is_success() {
            return Err(to_error(status.as_u16(), &body));
        }
        Ok(())
    }

    pub async fn summarize(
        &self,
        transcript: &str,
        settings: &AppSettings,
    ) -> Result<TranscriptHeadline, WhisperError> {
        require_key(settings)?;
        let clipped = if transcript.len() > 4000 {
            transcript.chars().take(4000).collect::<String>()
        } else {
            transcript.to_string()
        };
        let model = if settings.model.trim().is_empty() {
            "auto"
        } else {
            settings.model.as_str()
        };
        let payload = serde_json::json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "Antworte ausschließlich mit JSON, ohne Markdown: {\"title\":\"...\",\"summary\":\"...\"}. title = kurzer deutscher Dateititel, 3 bis 6 Wörter, kein Satzzeichen außer Bindestrich, kein Punkt am Ende. summary = ein einziger kurzer deutscher Satz, maximal etwa 12 Wörter."
                },
                { "role": "user", "content": clipped }
            ]
        });
        let response = self
            .http
            .post(settings.chat_url())
            .bearer_auth(settings.api_key.trim())
            .json(&payload)
            .send()
            .await
            .map_err(map_request)?;
        let status = response.status();
        let body = response.text().await.map_err(map_request)?;
        if !status.is_success() {
            return Err(to_error(status.as_u16(), &body));
        }
        Ok(parse_headline(&body))
    }
}

fn require_key(settings: &AppSettings) -> Result<(), WhisperError> {
    if settings.api_key.trim().is_empty() {
        Err(WhisperError::new(
            "API-Key fehlt oder ungültig.",
            Some(401),
            "authentication_error",
        ))
    } else {
        Ok(())
    }
}

fn map_request(err: reqwest::Error) -> WhisperError {
    if err.is_timeout() {
        WhisperError::new("Zeitüberschreitung beim Whisper-Endpunkt.", None, "")
    } else if err.is_connect() {
        WhisperError::new("NUC nicht erreichbar. Läuft Tailscale?", None, "")
    } else {
        WhisperError::new(
            if err.is_request() {
                "NUC nicht erreichbar. Läuft Tailscale?"
            } else {
                "NUC nicht erreichbar. Läuft Tailscale?"
            },
            None,
            "",
        )
    }
}

fn to_error(status: u16, body: &str) -> WhisperError {
    let (message, error_type) = parse_error(body);
    if status == 401 || error_type == "authentication_error" {
        return WhisperError::new("API-Key fehlt oder ungültig.", Some(401), "authentication_error");
    }
    let message = if message.trim().is_empty() {
        format!("HTTP {status}")
    } else {
        message
    };
    WhisperError::new(message, Some(status), error_type)
}

fn parse_error(body: &str) -> (String, String) {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(error) = value.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let error_type = error
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            return (message, error_type);
        }
    }
    (body.trim().to_string(), String::new())
}

fn parse_text(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.starts_with('{') {
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                return text.trim().to_string();
            }
        }
    }
    trimmed.to_string()
}

fn parse_headline(body: &str) -> TranscriptHeadline {
    let content = parse_chat(body);
    let json = unwrap_json(&content);
    if let Ok(value) = serde_json::from_str::<Value>(&json) {
        let title = value
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();
        let mut summary = value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .trim_matches('"')
            .to_string();
        if summary.is_empty() {
            summary = content;
        }
        return TranscriptHeadline { title, summary };
    }
    TranscriptHeadline {
        title: String::new(),
        summary: content,
    }
}

fn parse_chat(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(body) {
        if let Some(content) = value
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
        {
            return content.trim().trim_matches('"').trim().to_string();
        }
    }
    body.trim().to_string()
}

fn unwrap_json(content: &str) -> String {
    let text = content.trim();
    if text.starts_with("```") {
        if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
            if end > start {
                return text[start..=end].to_string();
            }
        }
    }
    text.to_string()
}
