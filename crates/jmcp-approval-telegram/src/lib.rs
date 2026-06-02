use chrono::{DateTime, Utc};
use jmcp_domain::ApprovalDecision;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

mod config;
mod voice;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod voice_client_tests;

pub use config::TelegramConfig;
pub use voice::{
    parse_voice_reply, voice_intent_risk, TelegramVoiceApproval, VoiceRisk,
    VOICE_CONFIDENCE_THRESHOLD,
};

#[derive(Clone, Debug)]
pub struct TelegramApprovalChallenge {
    pub work_order_id: Uuid,
    pub approver_user_id: i64,
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TelegramApprovalMessage {
    pub user_id: i64,
    pub text: String,
}

pub struct TelegramBotClient {
    config: TelegramConfig,
    http: reqwest::Client,
}

impl TelegramBotClient {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::new(),
        }
    }

    pub async fn get_me(&self) -> Result<TelegramUser, TelegramApprovalError> {
        self.post::<_, TelegramUser>("getMe", &serde_json::json!({}))
            .await
    }

    pub async fn get_updates(
        &self,
        offset: Option<i64>,
        timeout_seconds: u64,
    ) -> Result<Vec<TelegramUpdate>, TelegramApprovalError> {
        self.post::<_, Vec<TelegramUpdate>>(
            "getUpdates",
            &serde_json::json!({
                "offset": offset,
                "timeout": timeout_seconds,
                "allowed_updates": ["message"],
            }),
        )
        .await
    }

    pub async fn send_message(
        &self,
        chat_id: i64,
        text: &str,
    ) -> Result<TelegramMessage, TelegramApprovalError> {
        self.post::<_, TelegramMessage>(
            "sendMessage",
            &serde_json::json!({
                "chat_id": chat_id,
                "text": text,
                "disable_web_page_preview": true,
            }),
        )
        .await
    }

    /// Resolve a `file_id` to a downloadable [`TelegramFile`] (`getFile`).
    pub async fn get_file(&self, file_id: &str) -> Result<TelegramFile, TelegramApprovalError> {
        self.post::<_, TelegramFile>("getFile", &serde_json::json!({ "file_id": file_id }))
            .await
    }

    /// Download a file's raw bytes by its `file_path` (from [`Self::get_file`]).
    pub async fn download_file(&self, file_path: &str) -> Result<Vec<u8>, TelegramApprovalError> {
        let response = self
            .http
            .get(self.config.file_url(file_path))
            .send()
            .await
            .map_err(|_| TelegramApprovalError::Api("file download failed".to_owned()))?;
        let bytes = response
            .error_for_status()
            .map_err(|err| TelegramApprovalError::ApiRejected(err.to_string()))?
            .bytes()
            .await
            .map_err(|_| TelegramApprovalError::Api("file read failed".to_owned()))?;
        Ok(bytes.to_vec())
    }

    /// Convenience: resolve a voice note's `file_id` and download its bytes.
    pub async fn download_voice(&self, file_id: &str) -> Result<Vec<u8>, TelegramApprovalError> {
        let file = self.get_file(file_id).await?;
        let path = file.file_path.ok_or_else(|| {
            TelegramApprovalError::Api("getFile returned no file_path".to_owned())
        })?;
        self.download_file(&path).await
    }

    /// Send an OGG/Opus voice note (`sendVoice`, multipart upload).
    pub async fn send_voice(
        &self,
        chat_id: i64,
        ogg_opus: Vec<u8>,
        caption: Option<&str>,
    ) -> Result<TelegramMessage, TelegramApprovalError> {
        let part = reqwest::multipart::Part::bytes(ogg_opus)
            .file_name("jmcp.ogg")
            .mime_str("audio/ogg")
            .map_err(|_| TelegramApprovalError::Api("voice part build failed".to_owned()))?;
        let mut form = reqwest::multipart::Form::new()
            .text("chat_id", chat_id.to_string())
            .part("voice", part);
        if let Some(caption) = caption {
            form = form.text("caption", caption.to_owned());
        }
        let response = self
            .http
            .post(self.config.method_url("sendVoice"))
            .multipart(form)
            .send()
            .await
            .map_err(|_| TelegramApprovalError::Api("sendVoice request failed".to_owned()))?;
        let envelope: TelegramApiResponse<TelegramMessage> = response
            .json()
            .await
            .map_err(|_| TelegramApprovalError::Api("sendVoice decode failed".to_owned()))?;
        if envelope.ok {
            envelope
                .result
                .ok_or(TelegramApprovalError::MalformedResponse)
        } else {
            Err(TelegramApprovalError::ApiRejected(
                envelope
                    .description
                    .unwrap_or_else(|| "telegram api rejected sendVoice".to_owned()),
            ))
        }
    }

    pub fn config(&self) -> &TelegramConfig {
        &self.config
    }

    async fn post<T, R>(&self, method: &str, payload: &T) -> Result<R, TelegramApprovalError>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .post(self.config.method_url(method))
            .json(payload)
            .send()
            .await
            .map_err(|_| TelegramApprovalError::Api("request failed".to_owned()))?;
        let envelope: TelegramApiResponse<R> = response
            .json()
            .await
            .map_err(|_| TelegramApprovalError::Api("decode failed".to_owned()))?;
        if envelope.ok {
            envelope
                .result
                .ok_or(TelegramApprovalError::MalformedResponse)
        } else {
            Err(TelegramApprovalError::ApiRejected(
                envelope
                    .description
                    .unwrap_or_else(|| "telegram api rejected request".to_owned()),
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TelegramUpdate {
    pub update_id: i64,
    pub message: Option<TelegramMessage>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TelegramMessage {
    pub message_id: i64,
    pub from: Option<TelegramUser>,
    pub chat: TelegramChat,
    pub text: Option<String>,
    #[serde(default)]
    pub voice: Option<TelegramVoice>,
}

/// A Telegram voice note attached to a message.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TelegramVoice {
    pub file_id: String,
    #[serde(default)]
    pub duration: i64,
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Result of `getFile` — the relative `file_path` used to download the bytes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TelegramFile {
    pub file_id: String,
    #[serde(default)]
    pub file_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelegramUser {
    pub id: i64,
    #[serde(default)]
    pub is_bot: bool,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub username: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
struct TelegramApiResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TelegramApprovalError {
    #[error("wrong telegram user")]
    WrongUser,
    #[error("approval expired")]
    Expired,
    #[error("forged approval token")]
    Forged,
    #[error("unknown approval decision")]
    UnknownDecision,
    #[error("voice transcription confidence too low")]
    LowConfidence,
    #[error("high-risk voice approval requires spoken confirmation token")]
    ConfirmationRequired,
    #[error("telegram token file could not be loaded")]
    TokenLoadFailed,
    #[error("telegram token missing")]
    MissingToken,
    #[error("telegram allowlist missing")]
    MissingAllowlist,
    #[error("invalid telegram allowlist id")]
    InvalidAllowlist,
    #[error("telegram api request failed: {0}")]
    Api(String),
    #[error("telegram api rejected request: {0}")]
    ApiRejected(String),
    #[error("telegram api response malformed")]
    MalformedResponse,
}

pub fn render_prompt(challenge: &TelegramApprovalChallenge) -> String {
    format!(
        "JMCP approval requested for work order {}. Reply /approve {} or /deny {}.",
        challenge.work_order_id, challenge.token, challenge.token
    )
}

pub fn parse_reply(
    challenge: &TelegramApprovalChallenge,
    message: &TelegramApprovalMessage,
    now: DateTime<Utc>,
) -> Result<ApprovalDecision, TelegramApprovalError> {
    if message.user_id != challenge.approver_user_id {
        return Err(TelegramApprovalError::WrongUser);
    }
    if now > challenge.expires_at {
        return Err(TelegramApprovalError::Expired);
    }
    let mut parts = message.text.split_whitespace();
    let decision = parts.next().ok_or(TelegramApprovalError::UnknownDecision)?;
    let token = parts.next().ok_or(TelegramApprovalError::Forged)?;
    if token != challenge.token {
        return Err(TelegramApprovalError::Forged);
    }
    match decision
        .trim_start_matches('/')
        .to_ascii_uppercase()
        .as_str()
    {
        "APPROVE" => Ok(ApprovalDecision::Approved),
        "DENY" | "REJECT" => Ok(ApprovalDecision::Rejected),
        _ => Err(TelegramApprovalError::UnknownDecision),
    }
}
