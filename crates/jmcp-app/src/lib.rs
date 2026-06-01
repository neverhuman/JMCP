use anyhow::Result;
use chrono::Utc;
use jcp_core::{Envelope, LocalSigner};
use jmcp_attention::{AttentionPolicy, DefaultAttentionPolicy};
use jmcp_domain::{
    AdapterHealth, Approval, Evidence, HealthLevel, Lease, ReplayCheckpoint, ServiceCard,
    SystemStatus, WorkOrder,
};
use jmcp_store::{SqliteStore, StoredEvent};
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

    pub fn list_work_orders(&self) -> Result<Vec<WorkOrder>> {
        self.store.lock().expect("store lock").list_work_orders()
    }

    pub fn list_leases(&self) -> Result<Vec<Lease>> {
        self.store.lock().expect("store lock").list_leases()
    }

    pub fn list_approvals(&self) -> Result<Vec<Approval>> {
        self.store.lock().expect("store lock").list_approvals()
    }

    pub fn list_evidence(&self) -> Result<Vec<Evidence>> {
        self.store.lock().expect("store lock").list_evidence()
    }

    pub fn list_adapter_health(&self) -> Result<Vec<AdapterHealth>> {
        let stored = self
            .store
            .lock()
            .expect("store lock")
            .list_adapter_health()?;
        if stored.iter().any(|health| health.name == "jeryu") {
            return Ok(stored);
        }

        let mut health = stored;
        health.push(jeryu_health());
        Ok(health)
    }

    pub fn list_effects(&self) -> Result<Value> {
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
                name: "jekko".to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                subjects: vec!["*/jekko/*".to_owned()],
                capabilities: vec!["headless".to_owned()],
            },
        ]
    }

    pub fn systems(&self) -> Vec<SystemStatus> {
        let jeryu = jeryu_health();
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
                latency: jeryu.endpoint.unwrap_or_else(|| "absent".to_owned()),
            },
            SystemStatus {
                name: "jankurai".to_owned(),
                role: "standards memory".to_owned(),
                health: command_available("jankurai"),
                jcp: jcp_core::JCP_VERSION.to_owned(),
                latency: "local-cli".to_owned(),
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

    pub fn replay_summary(&self) -> Result<Value> {
        let store = self.store.lock().expect("store lock");
        Ok(json!({
            "events": store.event_count()?,
            "checkpoints": store.list_replay_checkpoints()?,
            "replayed_work_orders": store.replay_work_orders()?.len(),
            "side_effects_reissued": false,
        }))
    }

    pub fn replay_from_events(&self) -> Result<ReplayCheckpoint> {
        self.store
            .lock()
            .expect("store lock")
            .rebuild_work_order_projection_from_events()
    }

    pub fn events_after(&self, after: i64) -> Result<Vec<StoredEvent>> {
        self.store.lock().expect("store lock").events_after(after)
    }
}

fn jeryu_health() -> AdapterHealth {
    if let Ok(url) = std::env::var("JMCP_JERYU_URL") {
        return AdapterHealth {
            name: "jeryu".to_owned(),
            health: health_for_url(&url),
            endpoint: Some(url),
            detail: "configured by JMCP_JERYU_URL".to_owned(),
            checked_at: Utc::now(),
        };
    }

    for url in ["http://127.0.0.1:8799", "http://127.0.0.1:8787"] {
        if health_for_url(url) == HealthLevel::Nominal {
            return AdapterHealth {
                name: "jeryu".to_owned(),
                health: HealthLevel::Nominal,
                endpoint: Some(url.to_owned()),
                detail: "detected local listener".to_owned(),
                checked_at: Utc::now(),
            };
        }
    }

    AdapterHealth {
        name: "jeryu".to_owned(),
        health: HealthLevel::Degraded,
        endpoint: None,
        detail: "Jeryu not detected; JMCP remains available".to_owned(),
        checked_at: Utc::now(),
    }
}

fn health_for_url(url: &str) -> HealthLevel {
    let Some(addr) = socket_addr_from_url(url) else {
        return HealthLevel::Degraded;
    };
    if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
        HealthLevel::Nominal
    } else {
        HealthLevel::Degraded
    }
}

fn socket_addr_from_url(url: &str) -> Option<SocketAddr> {
    let without_scheme = url
        .strip_prefix("http://")
        .or_else(|| url.strip_prefix("https://"))?;
    let host_port = without_scheme.split('/').next()?;
    host_port
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
}

fn command_available(command: &str) -> HealthLevel {
    match Command::new("sh")
        .args(["-c", &format!("command -v {command}")])
        .output()
    {
        Ok(output) if output.status.success() => HealthLevel::Nominal,
        _ => HealthLevel::Degraded,
    }
}
