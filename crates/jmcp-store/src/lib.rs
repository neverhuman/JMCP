use anyhow::{Context, Result};
use chrono::Utc;
use jmcp_domain::{
    AdapterHealth, Approval, EffectLedgerEntry, Evidence, Lease, ReplayCheckpoint, WorkOrder,
};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct StoredEvent {
    pub id: i64,
    pub aggregate_id: Uuid,
    pub event_type: String,
    pub data: Value,
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<()> {
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

    pub fn append_work_order(&self, event_type: &str, work_order: &WorkOrder) -> Result<()> {
        let data = serde_json::to_value(work_order)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, work_order.id, event_type, &data)?;
        project_work_order_on(&tx, work_order, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_lease(&self, lease: &Lease) -> Result<()> {
        let data = serde_json::to_value(lease)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, lease.work_order_id, "lease.granted", &data)?;
        project_lease_on(&tx, lease, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_approval(&self, approval: &Approval) -> Result<()> {
        let data = serde_json::to_value(approval)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, approval.work_order_id, "approval.recorded", &data)?;
        project_approval_on(&tx, approval, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_evidence(&self, work_order_id: Option<Uuid>, evidence: &Evidence) -> Result<()> {
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

    pub fn record_adapter_health(&self, health: &AdapterHealth) -> Result<()> {
        let data = serde_json::to_value(health)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, Uuid::new_v4(), "adapter.health.reported", &data)?;
        project_adapter_health_on(&tx, health, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_effect(&self, effect: &EffectLedgerEntry) -> Result<()> {
        let data = serde_json::to_value(effect)?;
        let tx = self.conn.unchecked_transaction()?;
        append_event_on(&tx, effect.work_order_id, "effect.recorded", &data)?;
        project_effect_on(&tx, effect, &data)?;
        tx.commit()?;
        Ok(())
    }

    pub fn record_replay_checkpoint(
        &self,
        checkpoint: &ReplayCheckpoint,
    ) -> Result<ReplayCheckpoint> {
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

    pub fn list_work_orders(&self) -> Result<Vec<WorkOrder>> {
        self.list_json("select data from work_orders order by updated_at desc")
    }

    pub fn list_leases(&self) -> Result<Vec<Lease>> {
        self.list_json("select data from leases order by updated_at desc")
    }

    pub fn list_approvals(&self) -> Result<Vec<Approval>> {
        self.list_json("select data from approvals order by updated_at desc")
    }

    pub fn list_evidence(&self) -> Result<Vec<Evidence>> {
        self.list_json("select data from evidence order by captured_at desc")
    }

    pub fn list_adapter_health(&self) -> Result<Vec<AdapterHealth>> {
        self.list_json("select data from adapter_health order by checked_at desc")
    }

    pub fn list_effects(&self) -> Result<Vec<EffectLedgerEntry>> {
        self.list_json("select data from effect_ledger order by recorded_at desc")
    }

    pub fn list_replay_checkpoints(&self) -> Result<Vec<ReplayCheckpoint>> {
        self.list_json("select data from replay_checkpoints order by created_at desc")
    }

    fn list_json<T>(&self, sql: &str) -> Result<Vec<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let data = row?;
            serde_json::from_str(&data).context("decode stored json")
        })
        .collect()
    }

    pub fn event_count(&self) -> Result<i64> {
        Ok(self
            .conn
            .query_row("select count(*) from events", [], |row| row.get(0))?)
    }

    pub fn replay_work_orders(&self) -> Result<Vec<WorkOrder>> {
        Ok(self
            .replay_projection()?
            .work_orders
            .into_values()
            .collect())
    }

    fn replay_projection(&self) -> Result<ReplayProjection> {
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
            let aggregate_id =
                Uuid::parse_str(&aggregate_id).context("decode event aggregate id")?;
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
                _ => {}
            }
        }
        Ok(projection)
    }

    pub fn rebuild_work_order_projection_from_events(&self) -> Result<ReplayCheckpoint> {
        let projection = self.replay_projection()?;
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("delete from work_orders", [])?;
        tx.execute("delete from leases", [])?;
        tx.execute("delete from approvals", [])?;
        tx.execute("delete from evidence", [])?;
        tx.execute("delete from adapter_health", [])?;
        tx.execute("delete from effect_ledger", [])?;

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

    pub fn events_after(&self, after: i64) -> Result<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            "select id, aggregate_id, event_type, data from events where id > ?1 order by id asc",
        )?;
        let rows = stmt.query_map([after], |row| {
            let aggregate_id: String = row.get(1)?;
            let data: String = row.get(3)?;
            let aggregate_id = Uuid::parse_str(&aggregate_id).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    1,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            let data = serde_json::from_str(&data).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
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

#[derive(Default)]
struct ReplayProjection {
    work_orders: HashMap<Uuid, WorkOrder>,
    leases: Vec<Lease>,
    approvals: Vec<Approval>,
    evidence: Vec<(Option<Uuid>, Evidence)>,
    adapter_health: Vec<AdapterHealth>,
    effects: Vec<EffectLedgerEntry>,
}

fn append_event_on(
    conn: &Connection,
    aggregate_id: Uuid,
    event_type: &str,
    data: &Value,
) -> Result<()> {
    conn.execute(
        "insert into events (aggregate_id, event_type, data) values (?1, ?2, ?3)",
        params![aggregate_id.to_string(), event_type, data.to_string()],
    )?;
    Ok(())
}

fn project_work_order_on(conn: &Connection, work_order: &WorkOrder, data: &Value) -> Result<()> {
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

fn project_lease_on(conn: &Connection, lease: &Lease, data: &Value) -> Result<()> {
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

fn project_approval_on(conn: &Connection, approval: &Approval, data: &Value) -> Result<()> {
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

fn project_evidence_on(
    conn: &Connection,
    work_order_id: Option<Uuid>,
    evidence: &Evidence,
    data: &Value,
) -> Result<()> {
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

fn project_adapter_health_on(
    conn: &Connection,
    health: &AdapterHealth,
    data: &Value,
) -> Result<()> {
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

fn project_effect_on(conn: &Connection, effect: &EffectLedgerEntry, data: &Value) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use jmcp_domain::{Approval, Evidence, Lease, WorkOrder};
    use serde_json::json;

    #[test]
    fn projects_work_orders() {
        let store = SqliteStore::in_memory().unwrap();
        let wo = WorkOrder::submit("t/s/e", "demo", json!({}));
        store
            .append_work_order("work_order.submitted", &wo)
            .unwrap();
        assert_eq!(store.list_work_orders().unwrap().len(), 1);
        assert_eq!(store.events_after(0).unwrap().len(), 1);
    }

    #[test]
    fn projects_runtime_records_and_replays_without_effects() {
        let store = SqliteStore::in_memory().unwrap();
        let wo = WorkOrder::submit("t/s/e", "demo", json!({}));
        let lease = Lease {
            work_order_id: wo.id,
            holder: "adapter".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
        };
        let approval = Approval {
            work_order_id: wo.id,
            approver: "user".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
            decision: None,
        };
        let evidence = Evidence {
            kind: "command.digest".to_owned(),
            uri: "sha256:test".to_owned(),
            captured_at: Utc::now(),
        };

        store
            .append_work_order("work_order.submitted", &wo)
            .unwrap();
        store.record_lease(&lease).unwrap();
        store.record_approval(&approval).unwrap();
        store.record_evidence(Some(wo.id), &evidence).unwrap();

        assert_eq!(store.list_leases().unwrap(), vec![lease]);
        assert_eq!(store.list_approvals().unwrap(), vec![approval]);
        assert_eq!(store.list_evidence().unwrap(), vec![evidence]);

        let checkpoint = store.rebuild_work_order_projection_from_events().unwrap();
        assert_eq!(checkpoint.rebuilt_work_orders, 1);
        assert!(!checkpoint.side_effects_reissued);
        assert_eq!(store.list_replay_checkpoints().unwrap().len(), 1);
    }
}
