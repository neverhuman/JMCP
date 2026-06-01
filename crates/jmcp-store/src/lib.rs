use anyhow::{Context, Result};
use jmcp_domain::WorkOrder;
use rusqlite::{params, Connection};
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
            ",
        )?;
        Ok(())
    }

    pub fn append_work_order(&self, event_type: &str, work_order: &WorkOrder) -> Result<()> {
        let data = serde_json::to_value(work_order)?;
        self.conn.execute(
            "insert into events (aggregate_id, event_type, data) values (?1, ?2, ?3)",
            params![work_order.id.to_string(), event_type, data.to_string()],
        )?;
        self.conn.execute(
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

    pub fn list_work_orders(&self) -> Result<Vec<WorkOrder>> {
        let mut stmt = self
            .conn
            .prepare("select data from work_orders order by updated_at desc")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.map(|row| {
            let data = row?;
            serde_json::from_str(&data).context("decode work order")
        })
        .collect()
    }

    pub fn events_after(&self, after: i64) -> Result<Vec<StoredEvent>> {
        let mut stmt = self.conn.prepare(
            "select id, aggregate_id, event_type, data from events where id > ?1 order by id asc",
        )?;
        let rows = stmt.query_map([after], |row| {
            let aggregate_id: String = row.get(1)?;
            let data: String = row.get(3)?;
            Ok(StoredEvent {
                id: row.get(0)?,
                aggregate_id: Uuid::parse_str(&aggregate_id).expect("stored uuid is valid"),
                event_type: row.get(2)?,
                data: serde_json::from_str(&data).expect("stored json is valid"),
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jmcp_domain::WorkOrder;
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
}
