use chrono::{DateTime, Utc};
use jmcp_domain::ApprovalDecision;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};
use thiserror::Error;
use uuid::Uuid;

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

#[derive(Clone)]
pub struct TelegramConfig {
    token: String,
    pub api_base: String,
    pub allowed_user_ids: HashSet<i64>,
    pub allowed_chat_ids: HashSet<i64>,
}

impl std::fmt::Debug for TelegramConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelegramConfig")
            .field("token", &"<redacted>")
            .field("api_base", &self.api_base)
            .field("allowed_user_ids", &self.allowed_user_ids)
            .field("allowed_chat_ids", &self.allowed_chat_ids)
            .finish()
    }
}

impl TelegramConfig {
    pub fn from_env_file(path: impl AsRef<Path>) -> Result<Self, TelegramApprovalError> {
        let mut contents =
            std::fs::read_to_string(path).map_err(|_| TelegramApprovalError::TokenLoadFailed)?;
        append_env_override(&mut contents, "JMCP_TELEGRAM_BOT_TOKEN");
        append_env_override(&mut contents, "TELEGRAM_BOT_TOKEN");
        append_env_override(&mut contents, "BOT_TOKEN");
        append_env_override(&mut contents, "JMCP_TELEGRAM_API_BASE");
        append_env_override(&mut contents, "TELEGRAM_API_BASE");
        append_env_override(&mut contents, "JMCP_TELEGRAM_ALLOWED_USER_IDS");
        append_env_override(&mut contents, "TELEGRAM_ALLOWED_USER_IDS");
        append_env_override(&mut contents, "JMCP_TELEGRAM_ALLOWED_CHAT_IDS");
        append_env_override(&mut contents, "TELEGRAM_ALLOWED_CHAT_IDS");
        Self::from_env_contents(&contents)
    }

    pub fn from_env_contents(contents: &str) -> Result<Self, TelegramApprovalError> {
        let mut token = None;
        let mut api_base = None;
        let mut allowed_user_ids = HashSet::new();
        let mut allowed_chat_ids = HashSet::new();

        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key.trim() {
                    "TELEGRAM_BOT_TOKEN" | "BOT_TOKEN" | "JMCP_TELEGRAM_BOT_TOKEN" => {
                        token = Some(value.to_owned());
                    }
                    "TELEGRAM_API_BASE" | "JMCP_TELEGRAM_API_BASE" => {
                        api_base = Some(value.trim_end_matches('/').to_owned());
                    }
                    "TELEGRAM_ALLOWED_USER_IDS" | "JMCP_TELEGRAM_ALLOWED_USER_IDS" => {
                        allowed_user_ids.extend(parse_id_list(value)?);
                    }
                    "TELEGRAM_ALLOWED_CHAT_IDS" | "JMCP_TELEGRAM_ALLOWED_CHAT_IDS" => {
                        allowed_chat_ids.extend(parse_id_list(value)?);
                    }
                    _ => {}
                }
            } else if token.is_none() {
                token = Some(line.to_owned());
            }
        }

        let token = token
            .filter(|value| !value.is_empty())
            .ok_or(TelegramApprovalError::MissingToken)?;
        if allowed_user_ids.is_empty() && allowed_chat_ids.is_empty() {
            return Err(TelegramApprovalError::MissingAllowlist);
        }
        Ok(Self {
            token,
            api_base: api_base.unwrap_or_else(|| "https://api.telegram.org".to_owned()),
            allowed_user_ids,
            allowed_chat_ids,
        })
    }

    fn method_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base, self.token, method)
    }

    pub fn is_allowed(&self, user_id: i64, chat_id: i64) -> bool {
        if self.allowed_user_ids.is_empty() && self.allowed_chat_ids.is_empty() {
            return false;
        }
        (self.allowed_user_ids.is_empty() || self.allowed_user_ids.contains(&user_id))
            && (self.allowed_chat_ids.is_empty() || self.allowed_chat_ids.contains(&chat_id))
    }
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

/// An already-transcribed voice note approval. The audio download and
/// speech-to-text happen upstream in the runtime; this crate only sees the
/// resulting `transcript` plus the recognizer's `confidence` in `0.0..=1.0`.
#[derive(Clone, Debug, PartialEq)]
pub struct TelegramVoiceApproval {
    pub user_id: i64,
    pub transcript: String,
    pub confidence: f32,
}

/// Minimum recognizer confidence required to act on a voice approval.
pub const VOICE_CONFIDENCE_THRESHOLD: f32 = 0.75;

/// Risk classification for the intent expressed in a voice transcript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiceRisk {
    Low,
    High,
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

fn parse_id_list(value: &str) -> Result<Vec<i64>, TelegramApprovalError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.parse()
                .map_err(|_| TelegramApprovalError::InvalidAllowlist)
        })
        .collect()
}

fn append_env_override(contents: &mut String, key: &str) {
    if let Ok(value) = std::env::var(key) {
        contents.push('\n');
        contents.push_str(key);
        contents.push('=');
        contents.push_str(&value);
    }
}

pub fn render_prompt(challenge: &TelegramApprovalChallenge) -> String {
    format!(
        "JMCP approval requested for work order {}. Reply APPROVE {} or REJECT {}.",
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
    match decision.to_ascii_uppercase().as_str() {
        "APPROVE" => Ok(ApprovalDecision::Approved),
        "REJECT" => Ok(ApprovalDecision::Rejected),
        _ => Err(TelegramApprovalError::UnknownDecision),
    }
}

/// Words that imply a high-effect action and therefore demand a spoken
/// confirmation token before a voice approval is honored.
const HIGH_RISK_KEYWORDS: &[&str] = &[
    "deploy",
    "delete",
    "destroy",
    "drop",
    "merge",
    "production",
    "prod",
    "spend",
    "pay",
    "rotate",
    "revoke",
    "secret",
    "credential",
    "wipe",
    "purge",
];

/// Classify the intent expressed in a transcript. Returns [`VoiceRisk::High`]
/// when any high-effect keyword is present (whole-word match, case-insensitive)
/// and [`VoiceRisk::Low`] otherwise.
pub fn voice_intent_risk(transcript: &str) -> VoiceRisk {
    let lower = transcript.to_ascii_lowercase();
    let high = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| HIGH_RISK_KEYWORDS.contains(&word));
    if high {
        VoiceRisk::High
    } else {
        VoiceRisk::Low
    }
}

/// Parse an already-transcribed voice note into an [`ApprovalDecision`].
///
/// Fails closed: the approver must match, the challenge must not have expired,
/// and the recognizer confidence must meet [`VOICE_CONFIDENCE_THRESHOLD`].
/// High-risk intents additionally require the challenge token to be spoken
/// (so a misheard "approve" cannot trigger a destructive action), while
/// low-risk intents accept a clear "approve"/"reject" word.
pub fn parse_voice_reply(
    challenge: &TelegramApprovalChallenge,
    voice: &TelegramVoiceApproval,
    now: DateTime<Utc>,
) -> Result<ApprovalDecision, TelegramApprovalError> {
    if voice.user_id != challenge.approver_user_id {
        return Err(TelegramApprovalError::WrongUser);
    }
    if now > challenge.expires_at {
        return Err(TelegramApprovalError::Expired);
    }
    if voice.confidence < VOICE_CONFIDENCE_THRESHOLD {
        return Err(TelegramApprovalError::LowConfidence);
    }

    let lower = voice.transcript.to_ascii_lowercase();
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    let says_approve = words.contains(&"approve") || words.contains(&"approved");
    let says_reject = words.contains(&"reject")
        || words.contains(&"rejected")
        || words.contains(&"deny")
        || words.contains(&"denied");

    let decision = match (says_approve, says_reject) {
        (true, false) => ApprovalDecision::Approved,
        (false, true) => ApprovalDecision::Rejected,
        _ => return Err(TelegramApprovalError::UnknownDecision),
    };

    if matches!(voice_intent_risk(&voice.transcript), VoiceRisk::High) {
        let token_lower = challenge.token.to_ascii_lowercase();
        if !words.iter().any(|w| *w == token_lower) {
            return Err(TelegramApprovalError::ConfirmationRequired);
        }
    }

    Ok(decision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn challenge() -> TelegramApprovalChallenge {
        TelegramApprovalChallenge {
            work_order_id: Uuid::new_v4(),
            approver_user_id: 42,
            token: "tok".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
        }
    }

    #[test]
    fn accepts_expected_user_and_token() {
        let c = challenge();
        let msg = TelegramApprovalMessage {
            user_id: 42,
            text: "APPROVE tok".to_owned(),
        };
        assert_eq!(
            parse_reply(&c, &msg, Utc::now()).unwrap(),
            ApprovalDecision::Approved
        );
    }

    #[test]
    fn rejects_wrong_user() {
        let c = challenge();
        let msg = TelegramApprovalMessage {
            user_id: 7,
            text: "APPROVE tok".to_owned(),
        };
        assert_eq!(
            parse_reply(&c, &msg, Utc::now()),
            Err(TelegramApprovalError::WrongUser)
        );
    }

    #[test]
    fn rejects_expired() {
        let c = challenge();
        let msg = TelegramApprovalMessage {
            user_id: 42,
            text: "APPROVE tok".to_owned(),
        };
        assert_eq!(
            parse_reply(&c, &msg, c.expires_at + Duration::seconds(1)),
            Err(TelegramApprovalError::Expired)
        );
    }

    #[test]
    fn rejects_forged_token() {
        let c = challenge();
        let msg = TelegramApprovalMessage {
            user_id: 42,
            text: "APPROVE nope".to_owned(),
        };
        assert_eq!(
            parse_reply(&c, &msg, Utc::now()),
            Err(TelegramApprovalError::Forged)
        );
    }

    fn voice(user_id: i64, transcript: &str, confidence: f32) -> TelegramVoiceApproval {
        TelegramVoiceApproval {
            user_id,
            transcript: transcript.to_owned(),
            confidence,
        }
    }

    #[test]
    fn classifies_high_risk_intent() {
        assert_eq!(
            voice_intent_risk("please approve the deploy to production"),
            VoiceRisk::High
        );
        assert_eq!(voice_intent_risk("delete the table"), VoiceRisk::High);
    }

    #[test]
    fn classifies_low_risk_intent() {
        assert_eq!(voice_intent_risk("approve this"), VoiceRisk::Low);
        // "approval" must not trip the "deploy"/"production" matchers.
        assert_eq!(voice_intent_risk("reject the approval"), VoiceRisk::Low);
    }

    #[test]
    fn voice_rejects_low_confidence() {
        let c = challenge();
        let v = voice(42, "approve", 0.74);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()),
            Err(TelegramApprovalError::LowConfidence)
        );
    }

    #[test]
    fn voice_high_risk_without_token_rejected() {
        let c = challenge();
        let v = voice(42, "approve the deploy to production", 0.95);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()),
            Err(TelegramApprovalError::ConfirmationRequired)
        );
    }

    #[test]
    fn voice_high_risk_with_token_approved() {
        let c = challenge();
        let v = voice(42, "approve the deploy to production token tok", 0.95);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()).unwrap(),
            ApprovalDecision::Approved
        );
    }

    #[test]
    fn voice_rejects_wrong_user() {
        let c = challenge();
        let v = voice(7, "approve", 0.99);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()),
            Err(TelegramApprovalError::WrongUser)
        );
    }

    #[test]
    fn voice_rejects_expired() {
        let c = challenge();
        let v = voice(42, "approve", 0.99);
        assert_eq!(
            parse_voice_reply(&c, &v, c.expires_at + Duration::seconds(1)),
            Err(TelegramApprovalError::Expired)
        );
    }

    #[test]
    fn voice_low_risk_approve() {
        let c = challenge();
        let v = voice(42, "yes please approve", 0.9);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()).unwrap(),
            ApprovalDecision::Approved
        );
    }

    #[test]
    fn voice_low_risk_reject() {
        let c = challenge();
        let v = voice(42, "no reject that", 0.9);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()).unwrap(),
            ApprovalDecision::Rejected
        );
    }

    #[test]
    fn config_parses_token_and_allowlists_without_exposing_secret() {
        let config = TelegramConfig::from_env_contents(
            "TELEGRAM_BOT_TOKEN=123:secret\nJMCP_TELEGRAM_API_BASE=http://localhost:8081\nJMCP_TELEGRAM_ALLOWED_USER_IDS=42, 43\nJMCP_TELEGRAM_ALLOWED_CHAT_IDS=-100\n",
        )
        .unwrap();

        assert_eq!(config.api_base, "http://localhost:8081");
        assert!(config.is_allowed(42, -100));
        assert!(!config.is_allowed(7, -100));
        assert!(!format!("{config:?}").contains("123:secret"));
    }

    #[test]
    fn config_rejects_raw_token_file_without_allowlist() {
        assert!(matches!(
            TelegramConfig::from_env_contents("123:secret\n"),
            Err(TelegramApprovalError::MissingAllowlist)
        ));
    }

    #[test]
    fn voice_unknown_decision_rejected() {
        let c = challenge();
        let v = voice(42, "maybe later", 0.9);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()),
            Err(TelegramApprovalError::UnknownDecision)
        );
    }
}
