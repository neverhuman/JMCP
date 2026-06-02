use super::*;
use chrono::Duration as ChronoDuration;
use jcp_core::Subject;
use jmcp_domain::{
    ApprovalChallengeState, ApprovalDecision, AutonomousActionOverrides, HealthLevel,
    MicrotaskOverrides, WorkOrderStatus,
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
    assert!(state
        .inventory_cards()
        .unwrap()
        .iter()
        .any(|card| card.name == "jmcp.microtask-planner"));
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
fn service_inventory_includes_microtask_and_local_inventory() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let cards = state.service_cards();

    assert!(cards
        .iter()
        .any(|card| card.name == "jmcp.microtask-planner"));
    assert!(cards
        .iter()
        .any(|card| card.name == "local-model-inventory"));
    assert!(cards
        .iter()
        .any(|card| card.name == "local-speech-inventory"));
    assert!(state
        .systems()
        .iter()
        .any(|system| system.name == "local-gpu"));
    assert!(state
        .list_adapter_health()
        .unwrap()
        .iter()
        .any(|health| health.name == "jmcp.microtask-planner"));
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
fn microtasks_list_initial_deterministic_catalog() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let microtasks = state.list_microtasks().unwrap();
    let ids = microtasks
        .iter()
        .map(|microtask| microtask.id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            "jankurai.repo-refresh-audit",
            "jankurai.changed-path-audit",
            "research.concept-scan",
            "router.tool-build-probe",
            "router.open-model-reasoning-survey",
            "local-model.inventory-20b-30b",
            "local-speech.inventory-asr-tts",
        ]
    );
    assert!(microtasks
        .iter()
        .all(|microtask| microtask.safety.evidence_oriented));
    assert!(microtasks.iter().all(|microtask| !microtask.safety.live));
}

#[test]
fn submitting_each_microtask_uses_signed_work_order_path() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let microtasks = state.list_microtasks().unwrap();

    for microtask in &microtasks {
        let work_order = state
            .submit_microtask(&microtask.id, MicrotaskOverrides::default())
            .unwrap();

        assert_eq!(work_order.status, WorkOrderStatus::Submitted);
        assert_eq!(work_order.subject, microtask.subject.0);
        assert_eq!(work_order.task.kind, microtask.work_order_kind.0);
        assert_eq!(work_order.task.payload["metadata"]["microtask"], true);
        assert_eq!(
            work_order.task.payload["metadata"]["microtask_id"],
            microtask.id
        );
        assert_eq!(
            work_order.task.payload["metadata"]["submitted_by"],
            "jmcp.microtask_planner"
        );
        assert_eq!(work_order.task.payload["live"], false);
    }

    assert_eq!(state.list_work_orders().unwrap().len(), microtasks.len());
    assert!(state.list_approval_challenges().unwrap().is_empty());
}

#[test]
fn unknown_microtask_is_rejected() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let error = state
        .submit_microtask("missing", MicrotaskOverrides::default())
        .unwrap_err();

    assert!(error.to_string().contains("unknown microtask"));
}

#[test]
fn microtask_guardrails_reject_unbounded_overrides() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    for overrides in [
        MicrotaskOverrides {
            live: Some(true),
            ..MicrotaskOverrides::default()
        },
        MicrotaskOverrides {
            allow_network: Some(true),
            ..MicrotaskOverrides::default()
        },
        MicrotaskOverrides {
            allow_gpu: Some(true),
            ..MicrotaskOverrides::default()
        },
        MicrotaskOverrides {
            allow_external_durable_mutation: Some(true),
            ..MicrotaskOverrides::default()
        },
    ] {
        let error = state
            .submit_microtask("research.concept-scan", overrides)
            .unwrap_err();

        assert!(error.to_string().contains("guarded payload policy"));
    }

    let error = state
        .submit_microtask(
            "local-model.inventory-20b-30b",
            MicrotaskOverrides {
                max_stages: Some(2),
                ..MicrotaskOverrides::default()
            },
        )
        .unwrap_err();

    assert!(error.to_string().contains("maxStages"));
}

#[test]
fn autonomous_action_microtask_fanout_sets_parent_metadata() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let work_orders = state
        .queue_autonomous_action_microtasks("repo-bank-bug-scan", MicrotaskOverrides::default())
        .unwrap();

    assert_eq!(work_orders.len(), 7);
    assert!(work_orders.iter().all(|work_order| {
        work_order.task.payload["metadata"]["parent_action_id"] == "repo-bank-bug-scan"
    }));
    assert!(work_orders
        .iter()
        .all(|work_order| work_order.task.payload["metadata"]["microtask"] == true));
    assert!(state.list_approval_challenges().unwrap().is_empty());
}

#[test]
fn microtask_payloads_preserve_evidence_caps_and_resource_intent() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let cases = [
        ("jankurai.repo-refresh-audit", "jankurai.proof"),
        ("jankurai.changed-path-audit", "jankurai.diff-audit"),
        ("research.concept-scan", "reason"),
        ("router.tool-build-probe", "reason"),
        ("local-model.inventory-20b-30b", "reason"),
        ("local-speech.inventory-asr-tts", "reason"),
    ];

    for (id, kind) in cases {
        let work_order = state
            .submit_microtask(id, MicrotaskOverrides::default())
            .unwrap();
        let payload = &work_order.task.payload;

        assert_eq!(work_order.task.kind, kind);
        assert_eq!(payload["live"], false);
        assert_eq!(payload["evidence_oriented"], true);
        assert_eq!(payload["allow_network"], false);
        assert_eq!(payload["allow_gpu"], false);
        assert!(payload["max_stages"].as_u64().unwrap() <= 2);
        assert!(payload["timeout_secs"].as_u64().unwrap() <= 900);
        assert!(
            payload["resource_intent"]["evidenceGoal"]
                .as_str()
                .unwrap()
                .contains("inventory")
                || payload["resource_intent"]["evidenceGoal"]
                    .as_str()
                    .unwrap()
                    .contains("evidence")
                || payload["resource_intent"]["evidenceGoal"]
                    .as_str()
                    .unwrap()
                    .contains("digest")
        );
    }
}

#[test]
fn local_model_root_parsing_does_not_require_default_external_volume() {
    let _guard = env_lock();
    let old_roots = std::env::var_os("JMCP_LOCAL_MODEL_ROOTS");
    std::env::set_var(
        "JMCP_LOCAL_MODEL_ROOTS",
        "/tmp/jmcp-models-a,/tmp/jmcp-models-b",
    );
    let state = AppState::new(SqliteStore::in_memory().unwrap());

    let work_order = state
        .submit_microtask(
            "local-model.inventory-20b-30b",
            MicrotaskOverrides::default(),
        )
        .unwrap();
    let roots = work_order.task.payload["inputs"]["model_roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(roots, vec!["/tmp/jmcp-models-a", "/tmp/jmcp-models-b"]);
    assert!(!roots.iter().any(|root| root.contains("/Volumes/MOE")));

    restore_env("JMCP_LOCAL_MODEL_ROOTS", old_roots);
}

#[test]
fn gpu_inventory_degrades_cleanly_when_probe_is_absent() {
    let _guard = env_lock();
    let old_path = std::env::var_os("PATH");
    let old_inventory = std::env::var_os("JMCP_GPU_INVENTORY");
    std::env::set_var("PATH", "");
    std::env::remove_var("JMCP_GPU_INVENTORY");

    let health = runtime_health::local_gpu_inventory_health();

    assert_eq!(health.health, HealthLevel::Degraded);
    assert!(health.detail.contains("nvidia-smi"));

    restore_env("PATH", old_path);
    restore_env("JMCP_GPU_INVENTORY", old_inventory);
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

fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
    match value {
        Some(value) => std::env::set_var(key, value),
        None => std::env::remove_var(key),
    }
}

fn zyal_body(source: &str) -> &str {
    let start = source.find(">>>\n").unwrap() + ">>>\n".len();
    let end = source.find("\n<<<END_ZYAL ").unwrap();
    &source[start..end]
}
