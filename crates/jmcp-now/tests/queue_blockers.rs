use chrono::{Duration, TimeZone, Utc};
use jmcp_domain::{
    ActionSafetyClass, ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, Attention,
    AttentionLevel, CardLod, Evidence, Lease, PaneKind, PaneRisk, PaneStatus, PreparedTab, Task,
    WorkOrder, WorkOrderStatus,
};
use jmcp_now::{
    queue_blockers_projection,
    scenes::queue_blockers::{self, QueueBlockersProjection},
    CachedNow, NowProjection, NowReads,
};
use uuid::Uuid;

#[test]
fn queue_blockers_panes_join_attention_and_canonical_sidecars() {
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    let attention = jmcp_app::attention_inbox_sample();
    let incidents = jmcp_app::incident_records_sample();
    let actions = state.list_autonomous_actions().unwrap();
    let focused_id = uuid("22222222-2222-4222-8222-222222222222");
    let other_id = uuid("99999999-9999-4999-8999-999999999991");
    let completed_id = uuid("99999999-9999-4999-8999-999999999992");
    let reads = NowReads {
        work_orders: vec![
            work_order(
                focused_id,
                "Bridge write lease",
                WorkOrderStatus::AwaitingApproval,
            )
            .with_evidence(),
            work_order(
                other_id,
                "Local inventory probe",
                WorkOrderStatus::Submitted,
            ),
            work_order(completed_id, "Completed work", WorkOrderStatus::Completed),
        ],
        leases: vec![Lease {
            work_order_id: focused_id,
            holder: "jmcpd".to_owned(),
            expires_at: fixed_time() + Duration::minutes(10),
        }],
        attention_packets: attention.clone(),
        approval_challenges: vec![ApprovalChallenge {
            id: uuid("88888888-8888-4888-8888-888888888881"),
            work_order_id: focused_id,
            approver: "ops".to_owned(),
            channel: ApprovalChannel::Local,
            target_user_id: None,
            target_chat_id: None,
            token_hash: "sha256:test".to_owned(),
            expires_at: fixed_time() + Duration::minutes(20),
            state: ApprovalChallengeState::Pending,
            decision: None,
            created_at: fixed_time() - Duration::minutes(5),
            updated_at: fixed_time() - Duration::minutes(4),
        }],
        incidents,
        autonomous_actions: actions,
    };

    let projection = queue_blockers_projection(&reads, fixed_time());

    assert_eq!(projection.panes.len(), 2);
    let focused = pane(&projection, focused_id);
    assert_eq!(focused.kind, PaneKind::Queue);
    assert_eq!(focused.lod, CardLod::Focus);
    assert_eq!(focused.risk, PaneRisk::High);
    assert_eq!(focused.status, PaneStatus::Active);
    assert_eq!(focused.preview.headline, attention[0].why_now);
    assert!(focused.preview.chips.contains(&"evidence".to_owned()));
    assert!(focused
        .preview
        .chips
        .contains(&"approval_required".to_owned()));
    assert!(focused.prepared_tabs.contains(&PreparedTab::Evidence));
    assert!(focused.prepared_tabs.contains(&PreparedTab::Actions));
    assert!(focused.prepared_tabs.contains(&PreparedTab::Systems));

    let focused_actions = projection.prepared_actions.get(&focused.id).unwrap();
    assert!(focused_actions
        .iter()
        .any(|action| action.safety == ActionSafetyClass::ReadOnly));
    assert!(focused_actions
        .iter()
        .any(|action| action.safety == ActionSafetyClass::ApprovalRequired));
    assert!(focused_actions
        .iter()
        .any(|action| action.safety == ActionSafetyClass::BoundedAuto));
    assert!(focused_actions
        .iter()
        .all(|action| action.validate_no_secret_material().is_ok()));

    let evidence = projection.evidence_refs.get(&focused.id).unwrap();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].label, "service-card");

    let reason = projection
        .rank_reasons
        .iter()
        .find(|reason| reason.pane_id == focused.id)
        .unwrap();
    assert!(reason.reason.factors.approval_expiry_pressure > 0.0);
    assert!(reason.reason.factors.lease_pressure > 0.0);
    assert!(reason.reason.explanation.contains("Bridge write lease"));
}

#[test]
fn failed_work_without_path_gets_manual_only_action() {
    let failed_id = uuid("99999999-9999-4999-8999-999999999993");
    let reads = NowReads {
        work_orders: vec![work_order(
            failed_id,
            "Recover failed queue item",
            WorkOrderStatus::Failed,
        )],
        ..NowReads::default()
    };

    let projection = queue_blockers_projection(&reads, fixed_time());
    let failed = pane(&projection, failed_id);
    let actions = projection.prepared_actions.get(&failed.id).unwrap();

    assert_eq!(failed.status, PaneStatus::Active);
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].safety, ActionSafetyClass::ManualOnly);
    assert!(!actions[0].ready);
}

#[test]
fn projection_refreshes_when_event_watermark_advances() {
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    let initial = CachedNow::build(0, fixed_time(), NowReads::default());
    let projection = NowProjection::new(state.clone(), initial);

    assert_eq!(projection.load().generation, 0);
    state
        .record_attention_packet(&jmcp_app::attention_inbox_sample()[0])
        .unwrap();

    let refreshed = projection.refresh_if_stale_at(fixed_time()).unwrap();

    assert_eq!(refreshed.generation, 1);
    assert_eq!(refreshed.captured_at, fixed_time());
    assert_eq!(refreshed.default_pane, queue_blockers::KEY);
}

trait WorkOrderFixture {
    fn with_evidence(self) -> Self;
}

impl WorkOrderFixture for WorkOrder {
    fn with_evidence(mut self) -> Self {
        self.evidence.push(Evidence {
            kind: "service-card".to_owned(),
            uri: "sha256:evidence".to_owned(),
            captured_at: fixed_time() - Duration::minutes(3),
        });
        self
    }
}

fn work_order(id: Uuid, subject: &str, status: WorkOrderStatus) -> WorkOrder {
    WorkOrder {
        id,
        subject: subject.to_owned(),
        task: Task {
            kind: "jmcp.test".to_owned(),
            payload: serde_json::json!({ "id": id.to_string() }),
        },
        status,
        evidence: Vec::new(),
        attention: vec![Attention {
            level: AttentionLevel::Warn,
            reason: "test attention".to_owned(),
        }],
        created_at: fixed_time() - Duration::minutes(30),
        updated_at: fixed_time() - Duration::minutes(5),
    }
}

fn pane(projection: &QueueBlockersProjection, work_order_id: Uuid) -> &jmcp_domain::PaneVm {
    let pane_id = queue_blockers::pane_id(work_order_id);
    projection
        .panes
        .iter()
        .find(|pane| pane.id == pane_id)
        .unwrap()
}

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
        .single()
        .expect("valid fixed time")
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid uuid")
}
