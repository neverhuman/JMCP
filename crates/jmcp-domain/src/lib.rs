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
    #[error("approval expired")]
    ApprovalExpired,
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
}
