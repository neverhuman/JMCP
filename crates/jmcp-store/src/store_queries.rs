use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, AttentionPacket, EffectLedgerEntry, Evidence,
    IncidentRecord, InventoryCard, Lease, MemoryRecord, PromotionDecision, ReplayCheckpoint,
    VoiceSession, WorkOrder,
};
use rusqlite::OptionalExtension;
use uuid::Uuid;

use crate::projection::{append_event_on, project_control_plane_on};
use crate::{SqliteStore, StoreResult};

impl SqliteStore {
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

    pub(crate) fn list_json<T>(&self, sql: &str) -> StoreResult<Vec<T>>
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

    pub(crate) fn get_json<T, P>(&self, sql: &str, params: P) -> StoreResult<Option<T>>
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

    pub(crate) fn list_control_plane<T>(&self, kind: &str) -> StoreResult<Vec<T>>
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

    pub(crate) fn record_control_plane<T>(
        &self,
        kind: &str,
        event_type: &str,
        record: &T,
    ) -> StoreResult<()>
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
