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
fn accepts_slash_approval_commands() {
    let c = challenge();
    let approve = TelegramApprovalMessage {
        user_id: 42,
        text: "/approve tok".to_owned(),
    };
    let deny = TelegramApprovalMessage {
        user_id: 42,
        text: "/deny tok".to_owned(),
    };

    assert_eq!(
        parse_reply(&c, &approve, Utc::now()).unwrap(),
        ApprovalDecision::Approved
    );
    assert_eq!(
        parse_reply(&c, &deny, Utc::now()).unwrap(),
        ApprovalDecision::Rejected
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
fn config_parses_raw_token_with_allowlist() {
    let config =
        TelegramConfig::from_env_contents("123:secret\nJMCP_TELEGRAM_ALLOWED_USER_IDS=42\n")
            .unwrap();

    assert!(config.has_allowlist());
    assert!(config.is_allowed(42, 100));
}

#[test]
fn setup_config_allows_raw_token_without_allowlist() {
    let config = TelegramConfig::from_env_contents_for_setup("123:secret\n").unwrap();

    assert!(!config.has_allowlist());
    assert!(!config.is_allowed(42, 100));
}

#[test]
fn config_rejects_raw_token_file_without_allowlist() {
    assert!(matches!(
        TelegramConfig::from_env_contents("123:secret\n"),
        Err(TelegramApprovalError::MissingAllowlist)
    ));
}

#[test]
fn config_rejects_invalid_allowlist_id() {
    assert!(matches!(
        TelegramConfig::from_env_contents(
            "JMCP_TELEGRAM_BOT_TOKEN=123:secret\nJMCP_TELEGRAM_ALLOWED_USER_IDS=42,nope\n"
        ),
        Err(TelegramApprovalError::InvalidAllowlist)
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
