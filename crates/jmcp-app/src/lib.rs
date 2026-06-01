use anyhow::Result;
use jcp_core::Envelope;
use jmcp_attention::{AttentionPolicy, DefaultAttentionPolicy};
use jmcp_domain::WorkOrder;
use jmcp_store::{SqliteStore, StoredEvent};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<SqliteStore>>,
    policy: Arc<dyn AttentionPolicy>,
}

impl AppState {
    pub fn new(store: SqliteStore) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            policy: Arc::new(DefaultAttentionPolicy),
        }
    }

    pub fn submit_envelope(&self, envelope: Envelope) -> Result<WorkOrder> {
        envelope.validate()?;
        let mut work_order = WorkOrder::submit(envelope.subject, envelope.kind, envelope.payload);
        work_order
            .attention
            .extend(self.policy.evaluate(&work_order));
        self.store
            .lock()
            .expect("store lock")
            .append_work_order("work_order.submitted", &work_order)?;
        Ok(work_order)
    }

    pub fn list_work_orders(&self) -> Result<Vec<WorkOrder>> {
        self.store.lock().expect("store lock").list_work_orders()
    }

    pub fn events_after(&self, after: i64) -> Result<Vec<StoredEvent>> {
        self.store.lock().expect("store lock").events_after(after)
    }
}
