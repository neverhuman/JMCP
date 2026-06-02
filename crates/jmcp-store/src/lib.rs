mod projection;
mod replay;

#[cfg(test)]
mod tests;

use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, AttentionPacket, EffectLedgerEntry, Evidence,
    IncidentRecord, InventoryCard, Lease, MemoryRecord, PromotionDecision, ReplayCheckpoint,
    VoiceSession, WorkOrder,
};
use projection::{
    append_event_on, project_adapter_health_on, project_approval_challenge_on, project_approval_on,
    project_control_plane_on, project_effect_on, project_evidence_on, project_lease_on,
    project_work_order_on,
};
use rusqlite::{params, Connection, OptionalExtension};
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
    conn: Connection,
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

    pub fn list_work_orders(&self) -> StoreResult<Vec<WorkOrder>> {
        self.list_json("select data from work_orders order by updated_at desc")
    }

    pub fn get_work_order(&self, id: Uuid) -> StoreResult<Option<WorkOrder>> {
        self.get_json(
            "select data from work_orders where id = ?1",
            [id.to_string()],
        )
    }

    pub fn list_leases(&self) -> StoreResult<Vec<Lease>> {
        self.list_json("select data from leases order by updated_at desc")
    }

    pub fn list_approvals(&self) -> StoreResult<Vec<Approval>> {
        self.list_json("select data from approvals order by updated_at desc")
    }

    pub fn get_approval(
        &self,
        work_order_id: Uuid,
        approver: &str,
    ) -> StoreResult<Option<Approval>> {
        self.get_json(
            "select data from approvals where work_order_id = ?1 and approver = ?2",
            [work_order_id.to_string(), approver.to_owned()],
        )
    }

    pub fn list_approval_challenges(&self) -> StoreResult<Vec<ApprovalChallenge>> {
        self.list_json("select data from approval_challenges order by updated_at desc")
    }

    pub fn approval_challenge_by_token_hash(
        &self,
        token_hash: &str,
    ) -> StoreResult<Option<ApprovalChallenge>> {
        self.get_json(
            "select data from approval_challenges where token_hash = ?1",
            [token_hash.to_owned()],
        )
    }

    pub fn list_evidence(&self) -> StoreResult<Vec<Evidence>> {
        self.list_json("select data from evidence order by captured_at desc")
    }

    pub fn list_adapter_health(&self) -> StoreResult<Vec<AdapterHealth>> {
        self.list_json("select data from adapter_health order by checked_at desc")
    }

    pub fn list_effects(&self) -> StoreResult<Vec<EffectLedgerEntry>> {
        self.list_json("select data from effect_ledger order by recorded_at desc")
    }

    pub fn list_voice_sessions(&self) -> StoreResult<Vec<VoiceSession>> {
        self.list_control_plane("voice_session")
    }

    pub fn list_attention_packets(&self) -> StoreResult<Vec<AttentionPacket>> {
        self.list_control_plane("attention_packet")
    }

    pub fn list_memory_records(&self) -> StoreResult<Vec<MemoryRecord>> {
        self.list_control_plane("memory_record")
    }

    pub fn list_inventory_cards(&self) -> StoreResult<Vec<InventoryCard>> {
        self.list_control_plane("inventory_card")
    }

    pub fn list_promotion_decisions(&self) -> StoreResult<Vec<PromotionDecision>> {
        self.list_control_plane("promotion_decision")
    }

    pub fn list_incident_records(&self) -> StoreResult<Vec<IncidentRecord>> {
        self.list_control_plane("incident_record")
    }

    pub fn list_replay_checkpoints(&self) -> StoreResult<Vec<ReplayCheckpoint>> {
        self.list_json("select data from replay_checkpoints order by created_at desc")
    }

    fn list_json<T>(&self, sql: &str) -> StoreResult<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut values = Vec::new();
        for row in rows {
            let data = row?;
            values.push(serde_json::from_str(&data)?);
        }
        Ok(values)
    }

    fn get_json<T, P>(&self, sql: &str, params: P) -> StoreResult<Option<T>>
    where
        T: serde::de::DeserializeOwned,
        P: rusqlite::Params,
    {
        let data: Option<String> = self
            .conn
            .query_row(sql, params, |row| row.get(0))
            .optional()?;
        data.map(|data| serde_json::from_str(&data))
            .transpose()
            .map_err(Into::into)
    }

    fn record_control_plane<T>(&self, kind: &str, event_type: &str, record: &T) -> StoreResult<()>
    where
        T: serde::Serialize,
    {
        let data = serde_json::to_value(record)?;
        let id = record_id(&data)?;
        let updated_at = record_updated_at(&data)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, id, event_type, &data)?;
        project_control_plane_on(&tx, id, kind, &data, &updated_at)?;
        tx.commit()?;
        Ok(())
    }

    fn list_control_plane<T>(&self, kind: &str) -> StoreResult<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut stmt = self.conn.prepare(
            "select data from control_plane_records where kind = ?1 order by updated_at desc",
        )?;
        let rows = stmt.query_map([kind], |row| row.get::<_, String>(0))?;
        let mut values = Vec::new();
        for row in rows {
            let data = row?;
            values.push(serde_json::from_str(&data)?);
        }
        Ok(values)
    }
}

fn record_id(data: &serde_json::Value) -> StoreResult<Uuid> {
    let Some(id) = data.get("id").and_then(|value| value.as_str()) else {
        return Ok(Uuid::new_v4());
    };
    Ok(Uuid::parse_str(id)?)
}

fn record_updated_at(data: &serde_json::Value) -> StoreResult<String> {
    let Some(updated_at) = data
        .get("updatedAt")
        .and_then(|value| value.as_str())
        .or_else(|| data.get("updated_at").and_then(|value| value.as_str()))
        .or_else(|| data.get("decidedAt").and_then(|value| value.as_str()))
        .or_else(|| data.get("decided_at").and_then(|value| value.as_str()))
        .or_else(|| data.get("createdAt").and_then(|value| value.as_str()))
        .or_else(|| data.get("created_at").and_then(|value| value.as_str()))
        .or_else(|| data.get("openedAt").and_then(|value| value.as_str()))
        .or_else(|| data.get("opened_at").and_then(|value| value.as_str()))
        .or_else(|| data.get("startedAt").and_then(|value| value.as_str()))
        .or_else(|| data.get("started_at").and_then(|value| value.as_str()))
    else {
        return Ok(chrono::Utc::now().to_rfc3339());
    };
    Ok(updated_at.to_owned())
}
