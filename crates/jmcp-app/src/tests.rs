use super::*;
use chrono::Duration as ChronoDuration;
use jcp_core::Subject;
use jmcp_domain::{
    ApprovalChallengeState, ApprovalDecision, AutonomousActionOverrides, HealthLevel,
    WorkOrderStatus,
};
use serde_json::json;
use std::{
    fs,
    net::TcpListener,
    path::PathBuf,
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
fn autonomous_actions_list_three_bounded_zyal_actions() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let actions = state.list_autonomous_actions().unwrap();

    assert_eq!(actions.len(), 3);
    assert_eq!(actions[0].id, "repo-bank-bug-scan");
    assert!(actions.iter().all(|action| action.safety.evidence_oriented));
    assert!(actions.iter().all(|action| !action.safety.live));
    assert!(actions
        .iter()
        .all(|action| action.work_order_kind.0 == "zyal.run"));
}

#[test]
fn autonomous_action_submission_uses_signed_zyal_work_order_path() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let work_order = state
        .submit_autonomous_action(
            "cache-reduction-validity-check",
            AutonomousActionOverrides::default(),
        )
        .unwrap();

    assert_eq!(work_order.status, WorkOrderStatus::Submitted);
    assert_eq!(
        work_order.subject,
        "jmcp/zyal/cache-reduction-validity-check"
    );
    assert_eq!(work_order.task.kind, "zyal.run");
    assert_eq!(
        work_order.task.payload["metadata"]["submitted_by"],
        "jmcp.full_auto"
    );
    assert_eq!(work_order.task.payload["live"], false);
    assert!(state.list_approval_challenges().unwrap().is_empty());
    assert_eq!(state.list_work_orders().unwrap().len(), 1);
}

#[test]
fn autonomous_action_rejects_live_override() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let overrides = AutonomousActionOverrides {
        live: Some(true),
        ..AutonomousActionOverrides::default()
    };

    let error = state
        .submit_autonomous_action("repo-bank-bug-scan", overrides)
        .unwrap_err();

    assert!(error.to_string().contains("approval policy"));
}

#[test]
fn zyal_manifest_directory_contains_parseable_zyal_files_only() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../agent/zyal");
    let entries = fs::read_dir(&dir).unwrap();
    let mut count = 0;

    for entry in entries {
        let path = entry.unwrap().path();
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("zyal"));
        let text = fs::read_to_string(&path).unwrap();
        let manifest: serde_json::Value = serde_json::from_str(zyal_body(&text)).unwrap();
        assert!(manifest
            .get("id")
            .and_then(|value| value.as_str())
            .is_some());
        count += 1;
    }

    assert_eq!(count, 3);
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

fn zyal_body(source: &str) -> &str {
    let start = source.find(">>>\n").unwrap() + ">>>\n".len();
    let end = source.find("\n<<<END_ZYAL ").unwrap();
    &source[start..end]
}
