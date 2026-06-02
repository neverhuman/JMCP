use crate::{TelegramApprovalChallenge, TelegramApprovalError};
use chrono::{DateTime, Utc};
use jmcp_domain::ApprovalDecision;

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
