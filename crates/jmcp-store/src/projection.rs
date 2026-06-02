use crate::StoreResult;
use chrono::Utc;
use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, EffectLedgerEntry, Evidence, Lease, WorkOrder,
};
use rusqlite::{params, Connection};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub(crate) struct ReplayProjection {
    pub(crate) work_orders: HashMap<Uuid, WorkOrder>,
    pub(crate) leases: Vec<Lease>,
    pub(crate) approvals: Vec<Approval>,
    pub(crate) approval_challenges: Vec<ApprovalChallenge>,
    pub(crate) evidence: Vec<(Option<Uuid>, Evidence)>,
    pub(crate) adapter_health: Vec<AdapterHealth>,
    pub(crate) effects: Vec<EffectLedgerEntry>,
}

pub(crate) fn append_event_on(
    conn: &Connection,
    aggregate_id: Uuid,
    event_type: &str,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into events (aggregate_id, event_type, data) values (?1, ?2, ?3)",
        params![aggregate_id.to_string(), event_type, data.to_string()],
    )?;
    Ok(())
}

pub(crate) fn project_work_order_on(
    conn: &Connection,
    work_order: &WorkOrder,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into work_orders (id, subject, status, data, updated_at) values (?1, ?2, ?3, ?4, ?5)
         on conflict(id) do update set subject=excluded.subject, status=excluded.status, data=excluded.data, updated_at=excluded.updated_at",
        params![
            work_order.id.to_string(),
            work_order.subject,
            format!("{:?}", work_order.status),
            data.to_string(),
            work_order.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_lease_on(conn: &Connection, lease: &Lease, data: &Value) -> StoreResult<()> {
    conn.execute(
        "insert into leases (work_order_id, holder, expires_at, data, updated_at) values (?1, ?2, ?3, ?4, ?5)
         on conflict(work_order_id, holder) do update set expires_at=excluded.expires_at, data=excluded.data, updated_at=excluded.updated_at",
        params![
            lease.work_order_id.to_string(),
            lease.holder,
            lease.expires_at.to_rfc3339(),
            data.to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_approval_on(
    conn: &Connection,
    approval: &Approval,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into approvals (work_order_id, approver, expires_at, decision, data, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6)
         on conflict(work_order_id, approver) do update set expires_at=excluded.expires_at, decision=excluded.decision, data=excluded.data, updated_at=excluded.updated_at",
        params![
            approval.work_order_id.to_string(),
            approval.approver,
            approval.expires_at.to_rfc3339(),
            approval.decision.map(|decision| format!("{decision:?}")),
            data.to_string(),
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_approval_challenge_on(
    conn: &Connection,
    challenge: &ApprovalChallenge,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into approval_challenges (id, work_order_id, approver, channel, target_user_id, target_chat_id, token_hash, expires_at, state, data, updated_at) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         on conflict(id) do update set expires_at=excluded.expires_at, state=excluded.state, data=excluded.data, updated_at=excluded.updated_at",
        params![
            challenge.id.to_string(),
            challenge.work_order_id.to_string(),
            challenge.approver,
            format!("{:?}", challenge.channel),
            challenge.target_user_id,
            challenge.target_chat_id,
            challenge.token_hash,
            challenge.expires_at.to_rfc3339(),
            format!("{:?}", challenge.state),
            data.to_string(),
            challenge.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_evidence_on(
    conn: &Connection,
    work_order_id: Option<Uuid>,
    evidence: &Evidence,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into evidence (work_order_id, kind, uri, data, captured_at) values (?1, ?2, ?3, ?4, ?5)",
        params![
            work_order_id.map(|id| id.to_string()),
            evidence.kind,
            evidence.uri,
            data.to_string(),
            evidence.captured_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_adapter_health_on(
    conn: &Connection,
    health: &AdapterHealth,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into adapter_health (name, health, endpoint, data, checked_at) values (?1, ?2, ?3, ?4, ?5)
         on conflict(name) do update set health=excluded.health, endpoint=excluded.endpoint, data=excluded.data, checked_at=excluded.checked_at",
        params![
            health.name,
            format!("{:?}", health.health),
            health.endpoint,
            data.to_string(),
            health.checked_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub(crate) fn project_effect_on(
    conn: &Connection,
    effect: &EffectLedgerEntry,
    data: &Value,
) -> StoreResult<()> {
    conn.execute(
        "insert into effect_ledger (id, work_order_id, lease_holder, effect_kind, status, data, recorded_at) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         on conflict(id) do update set status=excluded.status, data=excluded.data, recorded_at=excluded.recorded_at",
        params![
            effect.id.to_string(),
            effect.work_order_id.to_string(),
            effect.lease_holder,
            effect.effect_kind,
            format!("{:?}", effect.status),
            data.to_string(),
            effect.recorded_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}
