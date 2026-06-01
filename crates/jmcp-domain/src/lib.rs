use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid transition from {from:?} using {action}")]
    InvalidTransition {
        from: WorkOrderStatus,
        action: &'static str,
    },
    #[error("lease expired")]
    LeaseExpired,
    #[error("lease does not match work order")]
    LeaseWrongWorkOrder,
    #[error("lease holder mismatch")]
    LeaseHolderMismatch,
    #[error("approval expired")]
    ApprovalExpired,
    #[error("approval challenge already used")]
    ApprovalAlreadyUsed,
    #[error("wrong approver")]
    WrongApprover,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum WorkOrderStatus {
    Submitted,
    Leased,
    AwaitingApproval,
    Approved,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkOrder {
    pub id: Uuid,
    pub subject: String,
    pub task: Task,
    pub status: WorkOrderStatus,
    pub evidence: Vec<Evidence>,
    pub attention: Vec<Attention>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Task {
    pub kind: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Lease {
    pub work_order_id: Uuid,
    pub holder: String,
    pub expires_at: DateTime<Utc>,
}

impl Lease {
    pub fn validate_for(&self, work_order_id: Uuid, holder: &str) -> Result<(), DomainError> {
        if self.work_order_id != work_order_id {
            return Err(DomainError::LeaseWrongWorkOrder);
        }
        if self.holder != holder {
            return Err(DomainError::LeaseHolderMismatch);
        }
        if self.expires_at < Utc::now() {
            return Err(DomainError::LeaseExpired);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Approval {
    pub work_order_id: Uuid,
    pub approver: String,
    pub expires_at: DateTime<Utc>,
    pub decision: Option<ApprovalDecision>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ApprovalDecision {
    Approved,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChannel {
    Local,
    Telegram,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalChallengeState {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ApprovalChallenge {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub approver: String,
    pub channel: ApprovalChannel,
    pub target_user_id: Option<i64>,
    pub target_chat_id: Option<i64>,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub state: ApprovalChallengeState,
    pub decision: Option<ApprovalDecision>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalActor {
    pub approver: String,
    pub telegram_user_id: Option<i64>,
    pub telegram_chat_id: Option<i64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Evidence {
    pub kind: String,
    pub uri: String,
    pub captured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Attention {
    pub level: AttentionLevel,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum AttentionLevel {
    Info,
    Warn,
    Page,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ServiceCard {
    pub name: String,
    pub version: String,
    pub subjects: Vec<String>,
    pub capabilities: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthLevel {
    Nominal,
    Watch,
    Degraded,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SystemStatus {
    pub name: String,
    pub role: String,
    pub health: HealthLevel,
    pub jcp: String,
    pub latency: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdapterHealth {
    pub name: String,
    pub health: HealthLevel,
    pub endpoint: Option<String>,
    pub detail: String,
    pub checked_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectStatus {
    Pending,
    Applied,
    Failed,
    Replayed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EffectLedgerEntry {
    pub id: Uuid,
    pub work_order_id: Uuid,
    pub lease_holder: String,
    pub effect_kind: String,
    pub status: EffectStatus,
    pub evidence_uri: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReplayCheckpoint {
    pub id: Uuid,
    pub last_event_id: i64,
    pub rebuilt_work_orders: usize,
    pub side_effects_reissued: bool,
    pub created_at: DateTime<Utc>,
}

impl WorkOrder {
    pub fn submit(subject: impl Into<String>, kind: impl Into<String>, payload: Value) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            subject: subject.into(),
            task: Task {
                kind: kind.into(),
                payload,
            },
            status: WorkOrderStatus::Submitted,
            evidence: Vec::new(),
            attention: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn lease(
        &mut self,
        holder: impl Into<String>,
        ttl: Duration,
    ) -> Result<Lease, DomainError> {
        self.transition(
            WorkOrderStatus::Leased,
            "lease",
            &[WorkOrderStatus::Submitted],
        )?;
        Ok(Lease {
            work_order_id: self.id,
            holder: holder.into(),
            expires_at: Utc::now() + ttl,
        })
    }

    pub fn require_approval(
        &mut self,
        approver: impl Into<String>,
        ttl: Duration,
    ) -> Result<Approval, DomainError> {
        self.transition(
            WorkOrderStatus::AwaitingApproval,
            "require_approval",
            &[WorkOrderStatus::Submitted, WorkOrderStatus::Leased],
        )?;
        Ok(Approval {
            work_order_id: self.id,
            approver: approver.into(),
            expires_at: Utc::now() + ttl,
            decision: None,
        })
    }

    pub fn apply_approval(
        &mut self,
        approval: &mut Approval,
        approver: &str,
        decision: ApprovalDecision,
    ) -> Result<(), DomainError> {
        if approval.work_order_id != self.id {
            return Err(DomainError::LeaseWrongWorkOrder);
        }
        if approval.expires_at < Utc::now() {
            return Err(DomainError::ApprovalExpired);
        }
        if approval.approver != approver {
            return Err(DomainError::WrongApprover);
        }
        approval.decision = Some(decision);
        match decision {
            ApprovalDecision::Approved => self.transition(
                WorkOrderStatus::Approved,
                "approve",
                &[WorkOrderStatus::AwaitingApproval],
            ),
            ApprovalDecision::Rejected => self.transition(
                WorkOrderStatus::Failed,
                "reject",
                &[WorkOrderStatus::AwaitingApproval],
            ),
        }
    }

    pub fn complete(&mut self) -> Result<(), DomainError> {
        self.transition(
            WorkOrderStatus::Completed,
            "complete",
            &[WorkOrderStatus::Leased, WorkOrderStatus::Approved],
        )
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = WorkOrderStatus::Failed;
        self.updated_at = Utc::now();
        self.attention.push(Attention {
            level: AttentionLevel::Warn,
            reason: reason.into(),
        });
    }

    pub fn add_evidence(&mut self, kind: impl Into<String>, uri: impl Into<String>) {
        self.evidence.push(Evidence {
            kind: kind.into(),
            uri: uri.into(),
            captured_at: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    fn transition(
        &mut self,
        to: WorkOrderStatus,
        action: &'static str,
        allowed: &[WorkOrderStatus],
    ) -> Result<(), DomainError> {
        if !allowed.contains(&self.status) {
            return Err(DomainError::InvalidTransition {
                from: self.status,
                action,
            });
        }
        self.status = to;
        self.updated_at = Utc::now();
        Ok(())
    }
}

impl ApprovalChallenge {
    pub fn new(
        work_order_id: Uuid,
        approver: impl Into<String>,
        channel: ApprovalChannel,
        target_user_id: Option<i64>,
        target_chat_id: Option<i64>,
        token_hash: impl Into<String>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            work_order_id,
            approver: approver.into(),
            channel,
            target_user_id,
            target_chat_id,
            token_hash: token_hash.into(),
            expires_at,
            state: ApprovalChallengeState::Pending,
            decision: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn decide(
        &mut self,
        actor: &ApprovalActor,
        decision: ApprovalDecision,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.state != ApprovalChallengeState::Pending {
            return Err(DomainError::ApprovalAlreadyUsed);
        }
        if now > self.expires_at {
            self.state = ApprovalChallengeState::Expired;
            self.updated_at = now;
            return Err(DomainError::ApprovalExpired);
        }
        if self.approver != actor.approver
            || self.target_user_id != actor.telegram_user_id
            || self.target_chat_id != actor.telegram_chat_id
        {
            return Err(DomainError::WrongApprover);
        }

        self.state = match decision {
            ApprovalDecision::Approved => ApprovalChallengeState::Approved,
            ApprovalDecision::Rejected => ApprovalChallengeState::Rejected,
        };
        self.decision = Some(decision);
        self.updated_at = now;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn prevents_bad_completion() {
        let mut wo = WorkOrder::submit("t/s/e", "demo", json!({}));
        assert!(wo.complete().is_err());
        wo.lease("worker", Duration::minutes(1)).unwrap();
        wo.complete().unwrap();
    }

    #[test]
    fn approval_must_match_work_order() {
        let mut first = WorkOrder::submit("t/s/one", "demo", json!({}));
        first
            .require_approval("user", Duration::minutes(5))
            .unwrap();
        let second = WorkOrder::submit("t/s/two", "demo", json!({}));
        let mut approval = Approval {
            work_order_id: second.id,
            approver: "user".to_owned(),
            expires_at: Utc::now() + Duration::minutes(5),
            decision: None,
        };

        assert_eq!(
            first.apply_approval(&mut approval, "user", ApprovalDecision::Approved),
            Err(DomainError::LeaseWrongWorkOrder)
        );
    }

    #[test]
    fn challenge_is_single_use_and_token_free() {
        let mut challenge = ApprovalChallenge::new(
            Uuid::new_v4(),
            "telegram:user:42",
            ApprovalChannel::Telegram,
            Some(42),
            Some(99),
            "sha256:test",
            Utc::now() + Duration::minutes(5),
        );
        let actor = ApprovalActor {
            approver: "telegram:user:42".to_owned(),
            telegram_user_id: Some(42),
            telegram_chat_id: Some(99),
        };

        challenge
            .decide(&actor, ApprovalDecision::Approved, Utc::now())
            .unwrap();

        assert_eq!(challenge.state, ApprovalChallengeState::Approved);
        assert_eq!(challenge.decision, Some(ApprovalDecision::Approved));
        assert_eq!(
            challenge.decide(&actor, ApprovalDecision::Rejected, Utc::now()),
            Err(DomainError::ApprovalAlreadyUsed)
        );
    }

    #[test]
    fn challenge_rejects_wrong_telegram_actor() {
        let mut challenge = ApprovalChallenge::new(
            Uuid::new_v4(),
            "telegram:user:42",
            ApprovalChannel::Telegram,
            Some(42),
            Some(99),
            "sha256:test",
            Utc::now() + Duration::minutes(5),
        );
        let actor = ApprovalActor {
            approver: "telegram:user:7".to_owned(),
            telegram_user_id: Some(7),
            telegram_chat_id: Some(99),
        };

        assert_eq!(
            challenge.decide(&actor, ApprovalDecision::Approved, Utc::now()),
            Err(DomainError::WrongApprover)
        );
        assert_eq!(challenge.state, ApprovalChallengeState::Pending);
    }

    #[test]
    fn expired_challenge_is_marked_expired() {
        let mut challenge = ApprovalChallenge::new(
            Uuid::new_v4(),
            "user",
            ApprovalChannel::Local,
            None,
            None,
            "sha256:test",
            Utc::now() - Duration::seconds(1),
        );
        let actor = ApprovalActor {
            approver: "user".to_owned(),
            telegram_user_id: None,
            telegram_chat_id: None,
        };

        assert_eq!(
            challenge.decide(&actor, ApprovalDecision::Approved, Utc::now()),
            Err(DomainError::ApprovalExpired)
        );
        assert_eq!(challenge.state, ApprovalChallengeState::Expired);
    }
}
