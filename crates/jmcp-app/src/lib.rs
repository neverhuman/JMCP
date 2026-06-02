#[cfg(test)]
mod tests;

mod approval_flow;
mod autonomous_actions;
mod control_plane;
mod control_plane_samples;
mod runtime_health;

use jcp_core::{Envelope, LocalSigner};
use jmcp_attention::{AttentionPolicy, DefaultAttentionPolicy};
use jmcp_domain::{
    AdapterHealth, Approval, ApprovalChallenge, AttentionPacket, DomainError, Evidence,
    HealthLevel, IncidentRecord, Lease, MemoryRecord, ReplayCheckpoint, ServiceCard, SystemStatus,
    WorkOrder,
};
use jmcp_store::{SqliteStore, StoreError, StoredEvent};
use runtime_health::{command_available, jailgun_health, jeryu_health};
use serde::Serialize;
use serde_json::{json, Value};
use std::fmt;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use uuid::Uuid;

pub use approval_flow::{local_actor, telegram_actor, telegram_approver};
pub use control_plane_samples::{
    attention_inbox_sample, incident_records_sample, inventory_cards_sample, memory_records_sample,
    promotion_decisions_sample, voice_sessions_sample,
};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("JCP core error: {0}")]
    Core(#[from] jcp_core::CoreError),
    #[error("signer IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("store error: {0}")]
    Store(#[from] StoreError),
    #[error("domain error: {0}")]
    Domain(#[from] DomainError),
    #[error("application state unavailable: {0}")]
    State(String),
}

pub type AppResult<T> = std::result::Result<T, AppError>;

#[derive(Clone, Debug, Serialize)]
pub struct CreatedApprovalChallenge {
    pub challenge: ApprovalChallenge,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ApprovalDecisionOutcome {
    pub challenge: ApprovalChallenge,
    pub approval: Approval,
    pub work_order: WorkOrder,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ApprovalDecisionError {
    UnknownToken,
    Expired,
    AlreadyUsed,
    WrongApprover,
    UnavailableState(String),
}

impl fmt::Display for ApprovalDecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownToken => write!(f, "unknown approval token"),
            Self::Expired => write!(f, "approval token expired"),
            Self::AlreadyUsed => write!(f, "approval token already used"),
            Self::WrongApprover => write!(f, "wrong approver for approval token"),
            Self::UnavailableState(message) => write!(f, "approval state unavailable: {message}"),
        }
    }
}

impl std::error::Error for ApprovalDecisionError {}

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

    pub fn submit_envelope(&self, envelope: Envelope) -> AppResult<WorkOrder> {
        envelope.validate()?;
        let signer = LocalSigner::load_or_create_default()?;
        envelope.verify_local_signature(&signer)?;
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

    pub fn list_work_orders(&self) -> AppResult<Vec<WorkOrder>> {
        Ok(self.store.lock().expect("store lock").list_work_orders()?)
    }

    pub fn work_order(&self, id: Uuid) -> AppResult<Option<WorkOrder>> {
        Ok(self.store.lock().expect("store lock").get_work_order(id)?)
    }

    pub fn list_leases(&self) -> AppResult<Vec<Lease>> {
        Ok(self.store.lock().expect("store lock").list_leases()?)
    }

    pub fn list_approvals(&self) -> AppResult<Vec<Approval>> {
        Ok(self.store.lock().expect("store lock").list_approvals()?)
    }

    pub fn list_approval_challenges(&self) -> AppResult<Vec<ApprovalChallenge>> {
        Ok(self
            .store
            .lock()
            .expect("store lock")
            .list_approval_challenges()?)
    }

    pub fn list_evidence(&self) -> AppResult<Vec<Evidence>> {
        Ok(self.store.lock().expect("store lock").list_evidence()?)
    }

    pub fn attention_packets(&self) -> AppResult<Vec<AttentionPacket>> {
        self.attention_inbox()
    }

    pub fn list_adapter_health(&self) -> AppResult<Vec<AdapterHealth>> {
        let mut health = self
            .store
            .lock()
            .expect("store lock")
            .list_adapter_health()?;
        for detected in [jeryu_health(), jailgun_health()] {
            if !health.iter().any(|item| item.name == detected.name) {
                health.push(detected);
            }
        }
        Ok(health)
    }

    pub fn list_effects(&self) -> AppResult<Value> {
        Ok(json!(self
            .store
            .lock()
            .expect("store lock")
            .list_effects()?))
    }

    pub fn service_cards(&self) -> Vec<ServiceCard> {
        vec![
            ServiceCard {
                name: "jmcpd".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jmcp/*".to_owned()],
                capabilities: vec!["work-orders".to_owned(), "replay".to_owned()],
            },
            ServiceCard {
                name: "jankurai".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jankurai/*".to_owned()],
                capabilities: vec!["local-cli".to_owned()],
            },
            ServiceCard {
                name: "jeryu".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jeryu/*".to_owned()],
                capabilities: vec!["health".to_owned(), "status".to_owned()],
            },
            ServiceCard {
                name: "jailgun".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jailgun/*".to_owned()],
                capabilities: vec![
                    "bounded-chatgpt-capture".to_owned(),
                    "run-agent".to_owned(),
                    "review-packet".to_owned(),
                ],
            },
            ServiceCard {
                name: "jekko".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jekko/*".to_owned()],
                capabilities: vec!["headless".to_owned()],
            },
        ]
    }

    pub fn systems(&self) -> Vec<SystemStatus> {
        let jeryu = jeryu_health();
        let jailgun = jailgun_health();
        vec![
            SystemStatus {
                name: "jmcpd".to_owned(),
                role: "authority kernel".to_owned(),
                health: HealthLevel::Nominal,
                jcp: jcp_core::JCP_VERSION.to_owned(),
                latency: "local".to_owned(),
            },
            SystemStatus {
                name: "jeryu".to_owned(),
                role: "evidence runner".to_owned(),
                health: jeryu.health,
                jcp: jcp_core::JCP_VERSION.to_owned(),
                latency: match jeryu.endpoint {
                    Some(endpoint) => endpoint,
                    None => "absent".to_owned(),
                },
            },
            SystemStatus {
                name: "jankurai".to_owned(),
                role: "standards memory".to_owned(),
                health: command_available("jankurai"),
                jcp: jcp_core::JCP_VERSION.to_owned(),
                latency: "local-cli".to_owned(),
            },
            SystemStatus {
                name: "jailgun".to_owned(),
                role: "bounded ChatGPT capture".to_owned(),
                health: jailgun.health,
                jcp: "adapter".to_owned(),
                latency: match jailgun.endpoint {
                    Some(endpoint) => endpoint,
                    None => "not configured".to_owned(),
                },
            },
            SystemStatus {
                name: "jekko".to_owned(),
                role: "headless worker".to_owned(),
                health: command_available("jekko"),
                jcp: "adapter".to_owned(),
                latency: "disabled unless configured".to_owned(),
            },
        ]
    }

    pub fn replay_summary(&self) -> AppResult<Value> {
        let store = self.store.lock().expect("store lock");
        Ok(json!({
            "events": store.event_count()?,
            "checkpoints": store.list_replay_checkpoints()?,
            "replayed_work_orders": store.replay_work_orders()?.len(),
            "side_effects_reissued": false,
        }))
    }

    pub fn replay_from_events(&self) -> AppResult<ReplayCheckpoint> {
        Ok(self
            .store
            .lock()
            .expect("store lock")
            .rebuild_work_order_projection_from_events()?)
    }

    pub fn events_after(&self, after: i64) -> AppResult<Vec<StoredEvent>> {
        Ok(self.store.lock().expect("store lock").events_after(after)?)
    }

    pub fn incidents(&self) -> AppResult<Vec<IncidentRecord>> {
        self.incident_records()
    }

    pub fn memory_proposals(&self) -> AppResult<Vec<MemoryRecord>> {
        self.memory_records()
    }
}
