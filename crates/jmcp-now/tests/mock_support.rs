use chrono::{DateTime, Duration, TimeZone, Utc};
use jmcp_domain::{
    ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, Attention, AttentionLevel,
    Evidence, IncidentRecord, IncidentSeverity, IncidentState, Lease, Task, WorkOrder,
    WorkOrderStatus,
};
use uuid::Uuid;

pub fn fixed_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0)
        .single()
        .expect("valid fixed time")
}

pub fn uuid(value: &str) -> Uuid {
    Uuid::parse_str(value).expect("valid uuid")
}

pub fn work_order(id: Uuid, subject: &str, status: WorkOrderStatus) -> WorkOrder {
    WorkOrder {
        id,
        subject: subject.to_owned(),
        task: Task {
            kind: "jmcp.test".to_owned(),
            payload: serde_json::json!({ "id": id.to_string() }),
        },
        status,
        evidence: Vec::new(),
        attention: Vec::new(),
        created_at: fixed_time() - Duration::minutes(30),
        updated_at: fixed_time() - Duration::minutes(5),
    }
}

pub fn work_order_with_evidence(id: Uuid, subject: &str, status: WorkOrderStatus) -> WorkOrder {
    let mut work_order = work_order(id, subject, status);
    work_order.evidence.push(Evidence {
        kind: "service-card".to_owned(),
        uri: "sha256:evidence".to_owned(),
        captured_at: fixed_time() - Duration::minutes(3),
    });
    work_order
}

pub fn attention_packet(work_order_id: Option<Uuid>, why_now: &str, level: AttentionLevel) -> jmcp_domain::AttentionPacket {
    jmcp_domain::AttentionPacket {
        id: uuid("33333333-3333-4333-8333-333333333333"),
        work_order_id,
        title: "Queue blocker attention".to_owned(),
        why_now: why_now.to_owned(),
        alternatives: vec!["Keep the queue read-only".to_owned(), "Promote after evidence lands".to_owned()],
        risk_delta: "This moves the queue toward write authority.".to_owned(),
        drill_down: "Inspect the blocker scene and its sidecars.".to_owned(),
        level,
        created_at: fixed_time() - Duration::minutes(5),
        updated_at: fixed_time() - Duration::minutes(4),
    }
}

pub fn lease(work_order_id: Uuid, holder: &str, expires_in_minutes: i64) -> Lease {
    Lease {
        work_order_id,
        holder: holder.to_owned(),
        expires_at: fixed_time() + Duration::minutes(expires_in_minutes),
    }
}

pub fn approval_challenge(work_order_id: Uuid, expires_in_minutes: i64) -> ApprovalChallenge {
    ApprovalChallenge {
        id: uuid("88888888-8888-4888-8888-888888888881"),
        work_order_id,
        approver: "ops".to_owned(),
        channel: ApprovalChannel::Local,
        target_user_id: None,
        target_chat_id: None,
        token_hash: "sha256:test".to_owned(),
        expires_at: fixed_time() + Duration::minutes(expires_in_minutes),
        state: ApprovalChallengeState::Pending,
        decision: None,
        created_at: fixed_time() - Duration::minutes(5),
        updated_at: fixed_time() - Duration::minutes(4),
    }
}

pub fn incident_record(
    related_work_order: Uuid,
    severity: IncidentSeverity,
    title: &str,
) -> IncidentRecord {
    IncidentRecord {
        id: uuid("77777777-7777-4777-8777-777777777771"),
        title: title.to_owned(),
        severity,
        state: IncidentState::Quarantined,
        quarantine_scope: "adapter/mcp".to_owned(),
        containment: "Keep the queue read-only until evidence lands.".to_owned(),
        related_work_orders: vec![related_work_order],
        notes: vec!["test incident".to_owned()],
        opened_at: fixed_time() - Duration::minutes(6 * 60),
        updated_at: fixed_time() - Duration::minutes(30),
    }
}

