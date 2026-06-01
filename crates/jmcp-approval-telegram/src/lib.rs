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
}
