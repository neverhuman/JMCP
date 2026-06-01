use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use jcp_core::{Envelope, LocalSigner};
use jmcp_attention::{AttentionPolicy, DefaultAttentionPolicy};
use jmcp_domain::{
    AdapterHealth, Approval, ApprovalActor, ApprovalChallenge, ApprovalChannel, ApprovalDecision,
    DomainError, Evidence, HealthLevel, Lease, ReplayCheckpoint, ServiceCard, SystemStatus,
    WorkOrder, WorkOrderStatus,
};
use jmcp_store::{SqliteStore, StoredEvent};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fmt;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;
use uuid::Uuid;

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

    pub fn work_order(&self, id: Uuid) -> Result<Option<WorkOrder>> {
        self.store.lock().expect("store lock").get_work_order(id)
    }

    pub fn list_leases(&self) -> Result<Vec<Lease>> {
        self.store.lock().expect("store lock").list_leases()
    }

    pub fn list_approvals(&self) -> Result<Vec<Approval>> {
        self.store.lock().expect("store lock").list_approvals()
    }

    pub fn list_approval_challenges(&self) -> Result<Vec<ApprovalChallenge>> {
        self.store
            .lock()
            .expect("store lock")
            .list_approval_challenges()
    }

    pub fn create_local_approval_challenge(
        &self,
        work_order_id: Uuid,
        approver: impl Into<String>,
        ttl: Option<ChronoDuration>,
    ) -> Result<CreatedApprovalChallenge> {
        self.create_approval_challenge(
            work_order_id,
            approver.into(),
            ApprovalChannel::Local,
            None,
            None,
            ttl.unwrap_or_else(default_approval_token_ttl),
        )
    }

    pub fn create_telegram_approval_challenge(
        &self,
        work_order_id: Uuid,
        user_id: i64,
        chat_id: i64,
        ttl: Option<ChronoDuration>,
    ) -> Result<CreatedApprovalChallenge> {
        self.create_approval_challenge(
            work_order_id,
            telegram_approver(user_id),
            ApprovalChannel::Telegram,
            Some(user_id),
            Some(chat_id),
            ttl.unwrap_or_else(default_approval_token_ttl),
        )
    }

    pub fn decide_approval_by_token(
        &self,
        token: &str,
        actor: ApprovalActor,
        decision: ApprovalDecision,
    ) -> std::result::Result<ApprovalDecisionOutcome, ApprovalDecisionError> {
        let token_hash = approval_token_hash(token);
        let store = self.store.lock().expect("store lock");
        let mut challenge = store
            .approval_challenge_by_token_hash(&token_hash)
            .map_err(unavailable_state)?
            .ok_or(ApprovalDecisionError::UnknownToken)?;

        if let Err(err) = challenge.decide(&actor, decision, Utc::now()) {
            if matches!(err, DomainError::ApprovalExpired) {
                store
                    .record_approval_challenge(&challenge)
                    .map_err(unavailable_state)?;
            }
            return Err(decision_error(err));
        }

        let mut approval = store
            .get_approval(challenge.work_order_id, &challenge.approver)
            .map_err(unavailable_state)?
            .ok_or_else(|| {
                ApprovalDecisionError::UnavailableState("approval record missing".to_owned())
            })?;
        let mut work_order = store
            .get_work_order(challenge.work_order_id)
            .map_err(unavailable_state)?
            .ok_or_else(|| {
                ApprovalDecisionError::UnavailableState("work order record missing".to_owned())
            })?;
        work_order
            .apply_approval(&mut approval, &challenge.approver, decision)
            .map_err(decision_error)?;

        store
            .append_work_order("work_order.approval_decided", &work_order)
            .map_err(unavailable_state)?;
        store
            .record_approval(&approval)
            .map_err(unavailable_state)?;
        store
            .record_approval_challenge(&challenge)
            .map_err(unavailable_state)?;

        Ok(ApprovalDecisionOutcome {
            challenge,
            approval,
            work_order,
        })
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
    if TcpStream::connect_timeout(&addr, StdDuration::from_millis(150)).is_ok() {
        HealthLevel::Nominal
    } else {
        HealthLevel::Degraded
    }
}

impl AppState {
    fn create_approval_challenge(
        &self,
        work_order_id: Uuid,
        approver: String,
        channel: ApprovalChannel,
        target_user_id: Option<i64>,
        target_chat_id: Option<i64>,
        ttl: ChronoDuration,
    ) -> Result<CreatedApprovalChallenge> {
        let token = Uuid::new_v4().simple().to_string();
        let expires_at = Utc::now() + ttl;
        let store = self.store.lock().expect("store lock");
        let mut work_order = store
            .get_work_order(work_order_id)?
            .ok_or_else(|| anyhow::anyhow!("work order not found: {work_order_id}"))?;

        let approval = match work_order.status {
            WorkOrderStatus::Submitted | WorkOrderStatus::Leased => {
                let approval = work_order.require_approval(approver.clone(), ttl)?;
                work_order
                    .attention
                    .extend(self.policy.evaluate(&work_order));
                store.append_work_order("work_order.awaiting_approval", &work_order)?;
                approval
            }
            WorkOrderStatus::AwaitingApproval => store
                .get_approval(work_order_id, &approver)?
                .unwrap_or(Approval {
                    work_order_id,
                    approver: approver.clone(),
                    expires_at,
                    decision: None,
                }),
            _ => anyhow::bail!("work order cannot accept an approval challenge"),
        };

        if approval.decision.is_some() {
            anyhow::bail!("approval has already been decided");
        }

        let challenge = ApprovalChallenge::new(
            work_order_id,
            approver,
            channel,
            target_user_id,
            target_chat_id,
            approval_token_hash(&token),
            approval.expires_at,
        );
        store.record_approval(&approval)?;
        store.record_approval_challenge(&challenge)?;

        Ok(CreatedApprovalChallenge { challenge, token })
    }
}

pub fn telegram_approver(user_id: i64) -> String {
    format!("telegram:user:{user_id}")
}

pub fn telegram_actor(user_id: i64, chat_id: i64) -> ApprovalActor {
    ApprovalActor {
        approver: telegram_approver(user_id),
        telegram_user_id: Some(user_id),
        telegram_chat_id: Some(chat_id),
    }
}

pub fn local_actor(approver: impl Into<String>) -> ApprovalActor {
    ApprovalActor {
        approver: approver.into(),
        telegram_user_id: None,
        telegram_chat_id: None,
    }
}

fn approval_token_hash(token: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(token.as_bytes())))
}

fn decision_error(err: DomainError) -> ApprovalDecisionError {
    match err {
        DomainError::ApprovalExpired => ApprovalDecisionError::Expired,
        DomainError::ApprovalAlreadyUsed => ApprovalDecisionError::AlreadyUsed,
        DomainError::WrongApprover => ApprovalDecisionError::WrongApprover,
        other => ApprovalDecisionError::UnavailableState(other.to_string()),
    }
}

fn unavailable_state(err: anyhow::Error) -> ApprovalDecisionError {
    ApprovalDecisionError::UnavailableState(err.to_string())
}

fn default_approval_token_ttl() -> ChronoDuration {
    ChronoDuration::minutes(15)
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

#[cfg(test)]
mod tests {
    use super::*;
    use jcp_core::Subject;
    use jmcp_domain::{ApprovalChallengeState, WorkOrderStatus};
    use serde_json::json;
    use std::str::FromStr;

    fn state_with_work_order() -> (AppState, WorkOrder) {
        let state = AppState::new(SqliteStore::in_memory().unwrap());
        let signer = LocalSigner::load_or_create_default().unwrap();
        let envelope = signer.sign(Envelope::new(
            Subject::from_str("tenant/service/entity").unwrap(),
            "demo.run",
            json!({"ok": true}),
        ));
        let work_order = state.submit_envelope(envelope).unwrap();
        (state, work_order)
    }

    #[test]
    fn approval_token_is_single_use() {
        let (state, work_order) = state_with_work_order();
        let created = state
            .create_telegram_approval_challenge(work_order.id, 42, 99, None)
            .unwrap();

        let outcome = state
            .decide_approval_by_token(
                &created.token,
                telegram_actor(42, 99),
                ApprovalDecision::Approved,
            )
            .unwrap();

        assert_eq!(outcome.work_order.status, WorkOrderStatus::Approved);
        assert_eq!(outcome.challenge.state, ApprovalChallengeState::Approved);
        assert_eq!(
            state.decide_approval_by_token(
                &created.token,
                telegram_actor(42, 99),
                ApprovalDecision::Rejected,
            ),
            Err(ApprovalDecisionError::AlreadyUsed)
        );
    }

    #[test]
    fn approval_token_is_not_stored_in_challenge_json() {
        let (state, work_order) = state_with_work_order();
        let created = state
            .create_telegram_approval_challenge(work_order.id, 42, 99, None)
            .unwrap();

        let wire = serde_json::to_string(&created.challenge).unwrap();

        assert!(!wire.contains(&created.token));
        assert!(wire.contains("sha256:"));
    }

    #[test]
    fn forged_token_is_unknown() {
        let (state, work_order) = state_with_work_order();
        state
            .create_telegram_approval_challenge(work_order.id, 42, 99, None)
            .unwrap();

        assert_eq!(
            state.decide_approval_by_token(
                "not-the-token",
                telegram_actor(42, 99),
                ApprovalDecision::Approved,
            ),
            Err(ApprovalDecisionError::UnknownToken)
        );
    }

    #[test]
    fn wrong_telegram_actor_is_rejected() {
        let (state, work_order) = state_with_work_order();
        let created = state
            .create_telegram_approval_challenge(work_order.id, 42, 99, None)
            .unwrap();

        assert_eq!(
            state.decide_approval_by_token(
                &created.token,
                telegram_actor(7, 99),
                ApprovalDecision::Approved,
            ),
            Err(ApprovalDecisionError::WrongApprover)
        );
    }

    #[test]
    fn expired_token_is_marked_expired() {
        let (state, work_order) = state_with_work_order();
        let created = state
            .create_telegram_approval_challenge(
                work_order.id,
                42,
                99,
                Some(ChronoDuration::seconds(-1)),
            )
            .unwrap();

        assert_eq!(
            state.decide_approval_by_token(
                &created.token,
                telegram_actor(42, 99),
                ApprovalDecision::Approved,
            ),
            Err(ApprovalDecisionError::Expired)
        );
        assert_eq!(
            state.list_approval_challenges().unwrap()[0].state,
            ApprovalChallengeState::Expired
        );
    }
}
