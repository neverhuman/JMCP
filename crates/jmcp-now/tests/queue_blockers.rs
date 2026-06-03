use chrono::{Duration, TimeZone, Utc};
use jmcp_domain::{
    ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, Attention, AttentionLevel,
    Evidence, Lease, Task, WorkOrder, WorkOrderStatus,
};
use jmcp_now::{
    scenes::queue_blockers, CachedNow, CardStatus, DrilldownKind, NowProjection, NowReads,
    SafetyClass,
};
use uuid::Uuid;

#[test]
fn queue_blockers_scene_joins_attention_and_prefetch_refs() {
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

    let scene = queue_blockers::compose(&reads, 7, fixed_time());

    assert_eq!(scene.cards.len(), 2);
    let focused = scene
        .cards
        .iter()
        .find(|card| card.id == focused_id.to_string())
        .unwrap();
    assert_eq!(focused.status, CardStatus::Verified);
    assert_eq!(focused.why_now, attention[0].why_now);
    assert!(focused
        .drilldowns
        .iter()
        .any(|item| item.kind == DrilldownKind::Evidence));
    assert!(focused
        .drilldowns
        .iter()
        .any(|item| item.kind == DrilldownKind::Lease));
    assert!(focused
        .drilldowns
        .iter()
        .any(|item| item.kind == DrilldownKind::Approval));
    assert!(focused
        .actions
        .iter()
        .any(|action| action.safety_class == SafetyClass::ReadOnly));
    assert!(focused
        .actions
        .iter()
        .any(|action| action.safety_class == SafetyClass::ApprovalRequired));
    assert!(focused
        .actions
        .iter()
        .any(|action| action.safety_class == SafetyClass::BoundedAuto));
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
    assert_eq!(refreshed.snapshot.generation, 1);
    assert!(refreshed.scenes.contains_key(queue_blockers::KEY));
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

fn fixed_time() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
        .single()
        .expect("valid fixed time")
}

fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid uuid")
}
