use crate::projection::{
    project_adapter_health_on, project_approval_challenge_on, project_approval_on,
    project_control_plane_on, project_effect_on, project_evidence_on, project_lease_on,
    project_work_order_on, ReplayProjection,
};
use crate::{SqliteStore, StoreResult};
use chrono::Utc;
use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, AttentionPacket, EffectLedgerEntry, Evidence,
    IncidentRecord, InventoryCard, Lease, MemoryRecord, PromotionDecision, ReplayCheckpoint,
    VoiceSession, WorkOrder,
};
use rusqlite::types::Type;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredEvent {
    pub id: i64,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub data: Value,
}

impl SqliteStore {
    pub fn event_count(&self) -> StoreResult<i64> {
        Ok(self
            .conn
            .query_row("select count(*) from events", [], |row| row.get(0))?)
    }

    pub fn replay_work_orders(&self) -> StoreResult<Vec<WorkOrder>> {
        Ok(self
            .replay_projection()?
            .work_orders
            .into_values()
            .collect())
    }

    fn replay_projection(&self) -> StoreResult<ReplayProjection> {
        let mut stmt = self
            .conn
            .prepare("select aggregate_id, event_type, data from events order by id asc")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;

        let mut projection = ReplayProjection::default();
        for row in rows {
            let (aggregate_id, event_type, data) = row?;
            let aggregate_id = Uuid::parse_str(&aggregate_id)?;
            match event_type.as_str() {
                event if event.starts_with("work_order.") => {
                    let work_order: WorkOrder = serde_json::from_str(&data)?;
                    projection.work_orders.insert(work_order.id, work_order);
                }
                "lease.granted" => {
                    let lease: Lease = serde_json::from_str(&data)?;
                    projection.leases.push(lease);
                }
                "approval.recorded" => {
                    let approval: Approval = serde_json::from_str(&data)?;
                    projection.approvals.push(approval);
                }
                "approval.challenge.recorded" => {
                    let challenge: ApprovalChallenge = serde_json::from_str(&data)?;
                    projection.approval_challenges.push(challenge);
                }
                "evidence.recorded" => {
                    let evidence: Evidence = serde_json::from_str(&data)?;
                    if let Some(work_order) = projection.work_orders.get_mut(&aggregate_id) {
                        if !work_order
                            .evidence
                            .iter()
                            .any(|item| item.uri == evidence.uri)
                        {
                            work_order.evidence.push(evidence.clone());
                        }
                    }
                    projection.evidence.push((
                        if aggregate_id == Uuid::nil() {
                            None
                        } else {
                            Some(aggregate_id)
                        },
                        evidence,
                    ));
                }
                "adapter.health.reported" => {
                    let health: AdapterHealth = serde_json::from_str(&data)?;
                    projection.adapter_health.push(health);
                }
                "effect.recorded" => {
                    let effect: EffectLedgerEntry = serde_json::from_str(&data)?;
                    projection.effects.push(effect);
                }
                "voice.session.recorded" => {
                    let session: VoiceSession = serde_json::from_str(&data)?;
                    projection.voice_sessions.push(session);
                }
                "attention.packet.recorded" => {
                    let packet: AttentionPacket = serde_json::from_str(&data)?;
                    projection.attention_packets.push(packet);
                }
                "memory.recorded" => {
                    let record: MemoryRecord = serde_json::from_str(&data)?;
                    projection.memory_records.push(record);
                }
                "inventory.card.recorded" => {
                    let card: InventoryCard = serde_json::from_str(&data)?;
                    projection.inventory_cards.push(card);
                }
                "promotion.decision.recorded" => {
                    let decision: PromotionDecision = serde_json::from_str(&data)?;
                    projection.promotion_decisions.push(decision);
                }
                "incident.recorded" => {
                    let incident: IncidentRecord = serde_json::from_str(&data)?;
                    projection.incident_records.push(incident);
                }
                _ => {}
            }
        }
        Ok(projection)
    }

    pub fn rebuild_work_order_projection_from_events(&self) -> StoreResult<ReplayCheckpoint> {
        let projection = self.replay_projection()?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("delete from work_orders", [])?;
        tx.execute("delete from leases", [])?;
        tx.execute("delete from approvals", [])?;
        tx.execute("delete from approval_challenges", [])?;
        tx.execute("delete from evidence", [])?;
        tx.execute("delete from adapter_health", [])?;
        tx.execute("delete from effect_ledger", [])?;
        tx.execute("delete from control_plane_records", [])?;

        for work_order in projection.work_orders.values() {
            let data = serde_json::to_value(work_order)?;
            project_work_order_on(&tx, work_order, &data)?;
        }
        for lease in &projection.leases {
            let data = serde_json::to_value(lease)?;
            project_lease_on(&tx, lease, &data)?;
        }
        for approval in &projection.approvals {
            let data = serde_json::to_value(approval)?;
            project_approval_on(&tx, approval, &data)?;
        }
        for challenge in &projection.approval_challenges {
            let data = serde_json::to_value(challenge)?;
            project_approval_challenge_on(&tx, challenge, &data)?;
        }
        for (work_order_id, evidence) in &projection.evidence {
            let data = serde_json::to_value(evidence)?;
            project_evidence_on(&tx, *work_order_id, evidence, &data)?;
        }
        for health in &projection.adapter_health {
            let data = serde_json::to_value(health)?;
            project_adapter_health_on(&tx, health, &data)?;
        }
        for effect in &projection.effects {
            let data = serde_json::to_value(effect)?;
            project_effect_on(&tx, effect, &data)?;
        }
        for session in &projection.voice_sessions {
            let data = serde_json::to_value(session)?;
            project_control_plane_on(
                &tx,
                session.id,
                "voice_session",
                &data,
                &session.updated_at.to_rfc3339(),
            )?;
        }
        for packet in &projection.attention_packets {
            let data = serde_json::to_value(packet)?;
            project_control_plane_on(
                &tx,
                packet.id,
                "attention_packet",
                &data,
                &packet.updated_at.to_rfc3339(),
            )?;
        }
        for record in &projection.memory_records {
            let data = serde_json::to_value(record)?;
            project_control_plane_on(
                &tx,
                record.id,
                "memory_record",
                &data,
                &record.updated_at.to_rfc3339(),
            )?;
        }
        for card in &projection.inventory_cards {
            let data = serde_json::to_value(card)?;
            project_control_plane_on(
                &tx,
                card.id,
                "inventory_card",
                &data,
                &Utc::now().to_rfc3339(),
            )?;
        }
        for decision in &projection.promotion_decisions {
            let data = serde_json::to_value(decision)?;
            project_control_plane_on(
                &tx,
                decision.id,
                "promotion_decision",
                &data,
                &decision.decided_at.to_rfc3339(),
            )?;
        }
        for incident in &projection.incident_records {
            let data = serde_json::to_value(incident)?;
            project_control_plane_on(
                &tx,
                incident.id,
                "incident_record",
                &data,
                &incident.updated_at.to_rfc3339(),
            )?;
        }
        let last_event_id = tx.query_row("select coalesce(max(id), 0) from events", [], |row| {
            row.get(0)
        })?;
        tx.commit()?;

        let checkpoint = ReplayCheckpoint {
            id: Uuid::new_v4(),
            last_event_id,
            rebuilt_work_orders: projection.work_orders.len(),
            side_effects_reissued: false,
            created_at: Utc::now(),
        };
        self.record_replay_checkpoint(&checkpoint)
    }

    pub fn events_after(&self, after: i64) -> StoreResult<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            "select id, aggregate_id, event_type, data from events where id > ?1 order by id asc",
        )?;
        let rows = stmt.query_map([after], |row| {
            let aggregate_id: String = row.get(1)?;
            let data: String = row.get(3)?;
            let aggregate_id = Uuid::parse_str(&aggregate_id).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(err))
            })?;
            let data = serde_json::from_str(&data).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(3, Type::Text, Box::new(err))
            })?;
            Ok(StoredEvent {
                id: row.get(0)?,
                aggregate_id,
                event_type: row.get(2)?,
                data,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}
