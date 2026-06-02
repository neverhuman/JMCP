//! Voice-approval orchestration: turn a transcribed Telegram voice note into an
//! approval decision through the existing risk-scored [`parse_voice_reply`] path.
//!
//! The plaintext challenge token is only known when the challenge is minted (it
//! is never persisted — the store keeps a hash). So jmcpd remembers each
//! freshly-created challenge in an in-memory [`VoiceChallengeBook`] keyed by the
//! approver's Telegram user id; a later voice note for that user is then matched,
//! validated by [`parse_voice_reply`] (confidence, expiry, decision word, and a
//! spoken confirmation token for high-risk intents), and — if it passes —
//! applied via the very same `decide_approval_by_token` path the typed
//! `/approve <token>` command uses.
//!
//! [`evaluate_voice_approval`] is a pure function over (book, user, transcript,
//! confidence, now) so the full decision matrix is unit-tested without any I/O.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use jmcp_approval_telegram::{parse_voice_reply, TelegramApprovalChallenge, TelegramVoiceApproval};
use jmcp_domain::ApprovalDecision;
use uuid::Uuid;

/// In-memory record of the latest pending challenge per approver, holding the
/// plaintext token needed to verify and apply a spoken approval. Tokens live
/// only here (never written to disk) and are dropped once the decision lands.
#[derive(Clone, Default)]
pub struct VoiceChallengeBook {
    inner: Arc<Mutex<HashMap<i64, TelegramApprovalChallenge>>>,
}

impl VoiceChallengeBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Remember a freshly created challenge for its approver.
    pub fn remember(&self, challenge: TelegramApprovalChallenge) {
        self.inner
            .lock()
            .expect("voice book lock")
            .insert(challenge.approver_user_id, challenge);
    }

    /// The pending challenge for `user_id`, if any.
    pub fn pending_for(&self, user_id: i64) -> Option<TelegramApprovalChallenge> {
        self.inner
            .lock()
            .expect("voice book lock")
            .get(&user_id)
            .cloned()
    }

    /// Drop the remembered challenge for `user_id` (after a decision lands).
    pub fn forget(&self, user_id: i64) {
        self.inner.lock().expect("voice book lock").remove(&user_id);
    }
}

/// What the caller should apply once a voice approval is validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceApply {
    pub token: String,
    pub decision: ApprovalDecision,
    pub work_order_id: Uuid,
}

/// Outcome of evaluating a transcribed voice note: an optional decision to apply
/// plus the natural-language reply to speak/send back to the approver.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoicePlan {
    pub apply: Option<VoiceApply>,
    pub reply: String,
}

/// Evaluate a transcribed voice note against the approver's pending challenge.
/// Pure: no I/O, fully unit-testable.
pub fn evaluate_voice_approval(
    book: &VoiceChallengeBook,
    user_id: i64,
    transcript: &str,
    confidence: Option<f64>,
    now: DateTime<Utc>,
) -> VoicePlan {
    let heard = transcript.trim();
    let Some(challenge) = book.pending_for(user_id) else {
        return VoicePlan {
            apply: None,
            reply: format!("I heard: \"{heard}\". There is no pending approval for you right now."),
        };
    };

    let voice = TelegramVoiceApproval {
        user_id,
        transcript: heard.to_owned(),
        confidence: confidence.unwrap_or(0.0) as f32,
    };

    match parse_voice_reply(&challenge, &voice, now) {
        Ok(decision) => {
            let verb = match decision {
                ApprovalDecision::Approved => "Approving",
                ApprovalDecision::Rejected => "Rejecting",
            };
            VoicePlan {
                apply: Some(VoiceApply {
                    token: challenge.token.clone(),
                    decision,
                    work_order_id: challenge.work_order_id,
                }),
                reply: format!(
                    "I heard: \"{heard}\". {verb} work order {}.",
                    challenge.work_order_id
                ),
            }
        }
        Err(err) => VoicePlan {
            apply: None,
            reply: format!("I heard: \"{heard}\". {}", explain(err)),
        },
    }
}

fn explain(err: jmcp_approval_telegram::TelegramApprovalError) -> String {
    use jmcp_approval_telegram::TelegramApprovalError as E;
    match err {
        E::LowConfidence => {
            "I am not confident I heard that correctly. Please say it again.".to_owned()
        }
        E::ConfirmationRequired => {
            "That is a high-risk action, so please repeat your decision and speak the confirmation token."
                .to_owned()
        }
        E::UnknownDecision => "Please say approve or reject to decide.".to_owned(),
        E::WrongUser => "You are not the approver for the pending request.".to_owned(),
        E::Expired => "That approval request has expired.".to_owned(),
        other => format!("I could not process that: {other}."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn challenge(user_id: i64, token: &str) -> TelegramApprovalChallenge {
        TelegramApprovalChallenge {
            work_order_id: Uuid::nil(),
            approver_user_id: user_id,
            token: token.to_owned(),
            expires_at: Utc::now() + chrono::Duration::minutes(15),
        }
    }

    fn book_with(challenge: TelegramApprovalChallenge) -> VoiceChallengeBook {
        let book = VoiceChallengeBook::new();
        book.remember(challenge);
        book
    }

    #[test]
    fn no_pending_challenge_yields_no_apply() {
        let book = VoiceChallengeBook::new();
        let plan = evaluate_voice_approval(&book, 7, "approve", Some(0.95), Utc::now());
        assert!(plan.apply.is_none());
        assert!(plan.reply.contains("no pending approval"));
    }

    #[test]
    fn low_risk_approve_is_applied() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(&book, 7, "yes, approve that", Some(0.95), Utc::now());
        let apply = plan.apply.expect("decision");
        assert_eq!(apply.decision, ApprovalDecision::Approved);
        assert_eq!(apply.token, "alpha");
    }

    #[test]
    fn low_risk_reject_is_applied() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(&book, 7, "no, reject it", Some(0.9), Utc::now());
        assert_eq!(plan.apply.unwrap().decision, ApprovalDecision::Rejected);
    }

    #[test]
    fn high_risk_without_spoken_token_is_blocked() {
        let book = book_with(challenge(7, "alpha"));
        // "deploy" is a high-risk keyword -> spoken token required.
        let plan = evaluate_voice_approval(&book, 7, "approve the deploy", Some(0.95), Utc::now());
        assert!(plan.apply.is_none());
        assert!(plan.reply.contains("confirmation token"));
    }

    #[test]
    fn high_risk_with_spoken_token_is_applied() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(
            &book,
            7,
            "approve the deploy, token alpha",
            Some(0.95),
            Utc::now(),
        );
        assert_eq!(plan.apply.unwrap().decision, ApprovalDecision::Approved);
    }

    #[test]
    fn low_confidence_is_rejected_with_retry_prompt() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(&book, 7, "approve", Some(0.40), Utc::now());
        assert!(plan.apply.is_none());
        assert!(plan.reply.contains("confident"));
    }

    #[test]
    fn missing_confidence_is_treated_as_zero() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(&book, 7, "approve", None, Utc::now());
        assert!(plan.apply.is_none());
    }

    #[test]
    fn unknown_decision_prompts_for_approve_or_reject() {
        let book = book_with(challenge(7, "alpha"));
        let plan = evaluate_voice_approval(&book, 7, "hello there", Some(0.95), Utc::now());
        assert!(plan.apply.is_none());
        assert!(plan.reply.to_lowercase().contains("approve or reject"));
    }

    #[test]
    fn expired_challenge_is_reported() {
        let expired = TelegramApprovalChallenge {
            work_order_id: Uuid::nil(),
            approver_user_id: 7,
            token: "alpha".to_owned(),
            expires_at: Utc::now() - chrono::Duration::minutes(1),
        };
        let book = book_with(expired);
        let plan = evaluate_voice_approval(&book, 7, "approve", Some(0.95), Utc::now());
        assert!(plan.apply.is_none());
        assert!(plan.reply.contains("expired"));
    }

    #[test]
    fn forget_removes_a_pending_challenge() {
        let book = book_with(challenge(7, "alpha"));
        assert!(book.pending_for(7).is_some());
        book.forget(7);
        assert!(book.pending_for(7).is_none());
    }
}
