use super::*;
use chrono::Duration as ChronoDuration;
use jcp_core::Subject;
use jmcp_domain::{ApprovalChallengeState, ApprovalDecision, HealthLevel, WorkOrderStatus};
use serde_json::json;
use std::{
    net::TcpListener,
    str::FromStr,
    sync::{Mutex, MutexGuard},
};
use uuid::Uuid;

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

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

#[test]
fn blank_control_plane_surfaces_use_deterministic_samples() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    assert_eq!(
        state.voice_sessions().unwrap()[0].id,
        Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap()
    );
    assert_eq!(
        state.attention_inbox().unwrap()[0].id,
        Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap()
    );
    assert_eq!(
        state.memory_records().unwrap()[0].id,
        Uuid::parse_str("44444444-4444-4444-8444-444444444441").unwrap()
    );
    assert_eq!(
        state.inventory_cards().unwrap()[0].id,
        Uuid::parse_str("55555555-5555-4555-8555-555555555551").unwrap()
    );
    assert_eq!(
        state.promotion_decisions().unwrap()[0].id,
        Uuid::parse_str("66666666-6666-4666-8666-666666666661").unwrap()
    );
    assert_eq!(
        state.incident_records().unwrap()[0].id,
        Uuid::parse_str("77777777-7777-4777-8777-777777777771").unwrap()
    );
}

#[test]
fn service_inventory_includes_jailgun() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let cards = state.service_cards();
    let jailgun = cards
        .iter()
        .find(|card| card.name == "jailgun")
        .expect("jailgun service card");

    assert!(jailgun.capabilities.contains(&"run-agent".to_owned()));
    assert!(jailgun.capabilities.contains(&"review-packet".to_owned()));
    assert!(state
        .systems()
        .iter()
        .any(|system| system.name == "jailgun"));
    assert!(state
        .list_adapter_health()
        .unwrap()
        .iter()
        .any(|health| health.name == "jailgun"));
}

#[test]
fn runtime_health_reports_jailgun_config_states() {
    let _guard = env_lock();
    clear_jailgun_env();

    let unconfigured = runtime_health::jailgun_health();
    assert_eq!(unconfigured.health, HealthLevel::Degraded);
    assert_eq!(unconfigured.endpoint, None);

    std::env::set_var("JMCP_JAILGUN_URL", "http://127.0.0.1:1");
    let missing_token = runtime_health::jailgun_health();
    assert_eq!(missing_token.health, HealthLevel::Blocked);
    assert!(missing_token.detail.contains("token"));

    std::env::set_var("JMCP_JAILGUN_TOKEN", "secret");
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", "http://127.0.0.1:2");
    let outside_policy = runtime_health::jailgun_health();
    assert_eq!(outside_policy.health, HealthLevel::Blocked);
    assert!(outside_policy
        .detail
        .contains("outside configured local submission policy"));

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let reachable = format!("http://{}", listener.local_addr().unwrap());
    std::env::set_var("JMCP_JAILGUN_URL", &reachable);
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", &reachable);
    let configured = runtime_health::jailgun_health();
    assert_eq!(configured.health, HealthLevel::Nominal);

    let dropped_addr = listener.local_addr().unwrap();
    drop(listener);
    let unreachable = format!("http://{dropped_addr}");
    std::env::set_var("JMCP_JAILGUN_URL", &unreachable);
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", &unreachable);
    let unreachable = runtime_health::jailgun_health();
    assert_eq!(unreachable.health, HealthLevel::Degraded);

    clear_jailgun_env();
}

#[test]
fn voice_sessions_fall_back_to_samples_and_persist_intake() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    assert!(!state.voice_sessions().unwrap().is_empty());
    let session = voice_sessions_sample()[0].clone();

    state.record_voice_session(&session).unwrap();

    let sessions = state.voice_sessions().unwrap();
    assert_eq!(sessions, vec![session]);
}

fn clear_jailgun_env() {
    std::env::remove_var("JMCP_JAILGUN_URL");
    std::env::remove_var("JMCP_JAILGUN_TOKEN");
    std::env::remove_var("JMCP_JAILGUN_ALLOWED_URLS");
}
