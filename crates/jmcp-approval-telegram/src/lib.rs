use chrono::{DateTime, Utc};
use jmcp_domain::ApprovalDecision;
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
    fn voice_unknown_decision_rejected() {
        let c = challenge();
        let v = voice(42, "maybe later", 0.9);
        assert_eq!(
            parse_voice_reply(&c, &v, Utc::now()),
            Err(TelegramApprovalError::UnknownDecision)
        );
    }
}
