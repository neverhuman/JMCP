use super::*;
use serde_json::json;

#[test]
fn prevents_bad_completion() {
    let mut wo = WorkOrder::submit("t/s/e", "demo", json!({}));
    assert!(wo.complete().is_err());
    wo.lease("worker", Duration::minutes(1)).unwrap();
    wo.complete().unwrap();
}

#[test]
fn approval_must_match_work_order() {
    let mut first = WorkOrder::submit("t/s/one", "demo", json!({}));
    first
        .require_approval("user", Duration::minutes(5))
        .unwrap();
    let second = WorkOrder::submit("t/s/two", "demo", json!({}));
    let mut approval = Approval {
        work_order_id: second.id,
        approver: "user".to_owned(),
        expires_at: Utc::now() + Duration::minutes(5),
        decision: None,
    };

    assert_eq!(
        first.apply_approval(&mut approval, "user", ApprovalDecision::Approved),
        Err(DomainError::LeaseWrongWorkOrder)
    );
}

#[test]
fn challenge_is_single_use_and_token_free() {
    let mut challenge = ApprovalChallenge::new(
        Uuid::new_v4(),
        "telegram:user:42",
        ApprovalChannel::Telegram,
        Some(42),
        Some(99),
        "sha256:test",
        Utc::now() + Duration::minutes(5),
    );
    let actor = ApprovalActor {
        approver: "telegram:user:42".to_owned(),
        telegram_user_id: Some(42),
        telegram_chat_id: Some(99),
    };

    challenge
        .decide(&actor, ApprovalDecision::Approved, Utc::now())
        .unwrap();

    assert_eq!(challenge.state, ApprovalChallengeState::Approved);
    assert_eq!(challenge.decision, Some(ApprovalDecision::Approved));
    assert_eq!(
        challenge.decide(&actor, ApprovalDecision::Rejected, Utc::now()),
        Err(DomainError::ApprovalAlreadyUsed)
    );
}

#[test]
fn challenge_rejects_wrong_telegram_actor() {
    let mut challenge = ApprovalChallenge::new(
        Uuid::new_v4(),
        "telegram:user:42",
        ApprovalChannel::Telegram,
        Some(42),
        Some(99),
        "sha256:test",
        Utc::now() + Duration::minutes(5),
    );
    let actor = ApprovalActor {
        approver: "telegram:user:7".to_owned(),
        telegram_user_id: Some(7),
        telegram_chat_id: Some(99),
    };

    assert_eq!(
        challenge.decide(&actor, ApprovalDecision::Approved, Utc::now()),
        Err(DomainError::WrongApprover)
    );
    assert_eq!(challenge.state, ApprovalChallengeState::Pending);
}

#[test]
fn expired_challenge_is_marked_expired() {
    let mut challenge = ApprovalChallenge::new(
        Uuid::new_v4(),
        "user",
        ApprovalChannel::Local,
        None,
        None,
        "sha256:test",
        Utc::now() - Duration::seconds(1),
    );
    let actor = ApprovalActor {
        approver: "user".to_owned(),
        telegram_user_id: None,
        telegram_chat_id: None,
    };

    assert_eq!(
        challenge.decide(&actor, ApprovalDecision::Approved, Utc::now()),
        Err(DomainError::ApprovalExpired)
    );
    assert_eq!(challenge.state, ApprovalChallengeState::Expired);
}

#[test]
fn control_plane_records_serialize_in_camel_case() {
    let session = VoiceSession {
        id: Uuid::new_v4(),
        work_order_id: None,
        channel: "voice".to_owned(),
        transcript: "approve".to_owned(),
        confidence: 0.9,
        candidate: VoiceCandidate {
            decision: ApprovalDecision::Approved,
            risk: VoiceRiskLevel::High,
            confirmation_token: Some("alpha".to_owned()),
        },
        confirmation_evidence: Vec::new(),
        state: VoiceSessionState::Confirmed,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let decision = PromotionDecision {
        id: Uuid::new_v4(),
        target_kind: "tool_card".to_owned(),
        target_name: "jmcpd.submit".to_owned(),
        gate: "approval".to_owned(),
        verdict: PromotionVerdict::Promoted,
        verifier: "ops".to_owned(),
        rollback_plan: "disable route".to_owned(),
        evidence_count: 1,
        created_at: Utc::now(),
        decided_at: Utc::now(),
    };

    let session_json = serde_json::to_value(session).unwrap();
    let decision_json = serde_json::to_value(decision).unwrap();

    assert!(session_json.get("workOrderId").is_some());
    assert!(session_json.get("createdAt").is_some());
    assert!(decision_json.get("targetKind").is_some());
    assert!(decision_json.get("decidedAt").is_some());
}
