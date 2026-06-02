mod projection;
mod replay;
mod store_queries;

#[cfg(test)]
mod tests;

use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, AttentionPacket, EffectLedgerEntry, Evidence,
    IncidentRecord, InventoryCard, Lease, MemoryRecord, PromotionDecision, ReplayCheckpoint,
    VoiceSession, WorkOrder,
};
use projection::{
    append_event_on, project_adapter_health_on, project_approval_challenge_on, project_approval_on,
    project_effect_on, project_evidence_on, project_lease_on, project_work_order_on,
};
use rusqlite::{params, Connection};
use thiserror::Error;
use uuid::Uuid;

pub use replay::StoredEvent;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("sqlite store error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("stored json decode failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("stored uuid decode failed: {0}")]
    Uuid(#[from] uuid::Error),
}

pub type StoreResult<T> = std::result::Result<T, StoreError>;

pub struct SqliteStore {
    pub(crate) conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> StoreResult<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> StoreResult<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> StoreResult<()> {
        self.conn.execute_batch(
            "
            create table if not exists events (
                id integer primary key autoincrement,
                aggregate_id text not null,
                event_type text not null,
                data text not null,
                created_at text not null default current_timestamp
            );
            create table if not exists work_orders (
                id text primary key,
                subject text not null,
                status text not null,
                data text not null,
                updated_at text not null
            );
            create table if not exists leases (
                work_order_id text not null,
                holder text not null,
                expires_at text not null,
                data text not null,
                updated_at text not null,
                primary key (work_order_id, holder)
            );
            create table if not exists approvals (
                work_order_id text not null,
                approver text not null,
                expires_at text not null,
                decision text,
                data text not null,
                updated_at text not null,
                primary key (work_order_id, approver)
            );
            create table if not exists approval_challenges (
                id text primary key,
                work_order_id text not null,
                approver text not null,
                channel text not null,
                target_user_id integer,
                target_chat_id integer,
                token_hash text not null unique,
                expires_at text not null,
                state text not null,
                data text not null,
                updated_at text not null
            );
            create table if not exists evidence (
                id integer primary key autoincrement,
                work_order_id text,
                kind text not null,
                uri text not null,
                data text not null,
                captured_at text not null
            );
            create table if not exists adapter_health (
                name text primary key,
                health text not null,
                endpoint text,
                data text not null,
                checked_at text not null
            );
            create table if not exists effect_ledger (
                id text primary key,
                work_order_id text not null,
                lease_holder text not null,
                effect_kind text not null,
                status text not null,
                data text not null,
                recorded_at text not null
            );
            create table if not exists control_plane_records (
                id text primary key,
                kind text not null,
                data text not null,
                updated_at text not null
            );
            create table if not exists replay_checkpoints (
                id text primary key,
                last_event_id integer not null,
                data text not null,
                created_at text not null
            );
            ",
        )?;
        Ok(())
    }

    pub fn append_work_order(&self, event_type: &str, work_order: &WorkOrder) -> StoreResult<()> {
        let data = serde_json::to_value(work_order)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, work_order.id, event_type, &data)?;
        project_work_order_on(&tx, work_order, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_lease(&self, lease: &Lease) -> StoreResult<()> {
        let data = serde_json::to_value(lease)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, lease.work_order_id, "lease.granted", &data)?;
        project_lease_on(&tx, lease, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_approval(&self, approval: &Approval) -> StoreResult<()> {
        let data = serde_json::to_value(approval)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, approval.work_order_id, "approval.recorded", &data)?;
        project_approval_on(&tx, approval, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_approval_challenge(&self, challenge: &ApprovalChallenge) -> StoreResult<()> {
        let data = serde_json::to_value(challenge)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(
            &tx,
            challenge.work_order_id,
            "approval.challenge.recorded",
            &data,
        )?;
        project_approval_challenge_on(&tx, challenge, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_evidence(
        &self,
        work_order_id: Option<Uuid>,
        evidence: &Evidence,
    ) -> StoreResult<()> {
        let data = serde_json::to_value(evidence)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(
            &tx,
            work_order_id.unwrap_or_else(Uuid::nil),
            "evidence.recorded",
            &data,
        )?;
        project_evidence_on(&tx, work_order_id, evidence, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_adapter_health(&self, health: &AdapterHealth) -> StoreResult<()> {
        let data = serde_json::to_value(health)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, Uuid::new_v4(), "adapter.health.reported", &data)?;
        project_adapter_health_on(&tx, health, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_effect(&self, effect: &EffectLedgerEntry) -> StoreResult<()> {
        let data = serde_json::to_value(effect)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, effect.work_order_id, "effect.recorded", &data)?;
        project_effect_on(&tx, effect, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_voice_session(&self, session: &VoiceSession) -> StoreResult<()> {
        self.record_control_plane("voice_session", "voice.session.recorded", session)
    }

    pub fn record_attention_packet(&self, packet: &AttentionPacket) -> StoreResult<()> {
        self.record_control_plane("attention_packet", "attention.packet.recorded", packet)
    }

    pub fn record_memory_record(&self, record: &MemoryRecord) -> StoreResult<()> {
        self.record_control_plane("memory_record", "memory.recorded", record)
    }

    pub fn record_inventory_card(&self, card: &InventoryCard) -> StoreResult<()> {
        self.record_control_plane("inventory_card", "inventory.card.recorded", card)
    }

    pub fn record_promotion_decision(&self, decision: &PromotionDecision) -> StoreResult<()> {
        self.record_control_plane(
            "promotion_decision",
            "promotion.decision.recorded",
            decision,
        )
    }

    pub fn record_incident_record(&self, incident: &IncidentRecord) -> StoreResult<()> {
        self.record_control_plane("incident_record", "incident.recorded", incident)
    }

    pub fn record_replay_checkpoint(
        &self,
        checkpoint: &ReplayCheckpoint,
    ) -> StoreResult<ReplayCheckpoint> {
        let data = serde_json::to_value(checkpoint)?;
        self.conn.execute(
            "insert into replay_checkpoints (id, last_event_id, data, created_at) values (?1, ?2, ?3, ?4)
             on conflict(id) do update set last_event_id=excluded.last_event_id, data=excluded.data, created_at=excluded.created_at",
            params![
                checkpoint.id.to_string(),
                checkpoint.last_event_id,
                data.to_string(),
                checkpoint.created_at.to_rfc3339(),
            ],
        )?;
        Ok(checkpoint.clone())
    }
}
