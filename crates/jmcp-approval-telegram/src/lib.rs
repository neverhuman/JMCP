use chrono::{DateTime, Utc};
use jmcp_domain::ApprovalDecision;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

mod config;
mod voice;

#[cfg(test)]
mod tests;

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
