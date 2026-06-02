use super::*;
use chrono::Duration as ChronoDuration;
use jcp_core::Subject;
use jmcp_domain::{ApprovalChallengeState, ApprovalDecision, WorkOrderStatus};
use serde_json::json;
use std::str::FromStr;

fn state_with_work_order() -> (AppState, WorkOrder) {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let signer = LocalSigner::load_or_create_default().unwrap();
    let envelope = signer.sign(Envelope::new(
        Subject::from_str("tenant/service/entity").unwrap(),
        "demo.run",
        json!({"ok": true}),
    ));
    let work_order = state.submit_envelope(envelope).unwrap();
    (state, work_order)
}

#[test]
fn approval_token_is_single_use() {
    let (state, work_order) = state_with_work_order();
    let created = state
        .create_telegram_approval_challenge(work_order.id, 42, 99, None)
        .unwrap();

    let outcome = state
        .decide_approval_by_token(
            &created.token,
            telegram_actor(42, 99),
            ApprovalDecision::Approved,
        )
        .unwrap();

    assert_eq!(outcome.work_order.status, WorkOrderStatus::Approved);
    assert_eq!(outcome.challenge.state, ApprovalChallengeState::Approved);
    assert_eq!(
        state.decide_approval_by_token(
            &created.token,
            telegram_actor(42, 99),
            ApprovalDecision::Rejected,
        ),
        Err(ApprovalDecisionError::AlreadyUsed)
    );
}

#[test]
fn approval_token_is_not_stored_in_challenge_json() {
    let (state, work_order) = state_with_work_order();
    let created = state
        .create_telegram_approval_challenge(work_order.id, 42, 99, None)
        .unwrap();

    let wire = serde_json::to_string(&created.challenge).unwrap();

    assert!(!wire.contains(&created.token));
    assert!(wire.contains("sha256:"));
}

#[test]
fn forged_token_is_unknown() {
    let (state, work_order) = state_with_work_order();
    state
        .create_telegram_approval_challenge(work_order.id, 42, 99, None)
        .unwrap();

    assert_eq!(
        state.decide_approval_by_token(
            "not-the-token",
            telegram_actor(42, 99),
            ApprovalDecision::Approved,
        ),
        Err(ApprovalDecisionError::UnknownToken)
    );
}

#[test]
fn wrong_telegram_actor_is_rejected() {
    let (state, work_order) = state_with_work_order();
    let created = state
        .create_telegram_approval_challenge(work_order.id, 42, 99, None)
        .unwrap();

    assert_eq!(
        state.decide_approval_by_token(
            &created.token,
            telegram_actor(7, 99),
            ApprovalDecision::Approved,
        ),
        Err(ApprovalDecisionError::WrongApprover)
    );
}

#[test]
fn expired_token_is_marked_expired() {
    let (state, work_order) = state_with_work_order();
    let created = state
        .create_telegram_approval_challenge(
            work_order.id,
            42,
            99,
            Some(ChronoDuration::seconds(-1)),
        )
        .unwrap();

    assert_eq!(
        state.decide_approval_by_token(
            &created.token,
            telegram_actor(42, 99),
            ApprovalDecision::Approved,
        ),
        Err(ApprovalDecisionError::Expired)
    );
    assert_eq!(
        state.list_approval_challenges().unwrap()[0].state,
        ApprovalChallengeState::Expired
    );
}
