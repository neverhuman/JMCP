use super::WorkOrderStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DomainErrorKind {
    InvalidTransition,
    LeaseExpired,
    LeaseWrongWorkOrder,
    LeaseHolderMismatch,
    ApprovalExpired,
    ApprovalAlreadyUsed,
    WrongApprover,
}

impl std::fmt::Display for DomainErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::InvalidTransition => "invalid_transition",
            Self::LeaseExpired => "lease_expired",
            Self::LeaseWrongWorkOrder => "lease_wrong_work_order",
            Self::LeaseHolderMismatch => "lease_holder_mismatch",
            Self::ApprovalExpired => "approval_expired",
            Self::ApprovalAlreadyUsed => "approval_already_used",
            Self::WrongApprover => "wrong_approver",
        };
        f.write_str(label)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainError {
    pub kind: DomainErrorKind,
    pub purpose: String,
    pub reason: String,
    pub common_fixes: Vec<&'static str>,
    pub docs_url: &'static str,
    pub repair_hint: String,
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "domain error: purpose={}; reason={}",
            self.purpose, self.reason
        )?;
        if !self.common_fixes.is_empty() {
            write!(f, "; common_fixes=[{}]", self.common_fixes.join(", "))?;
        }
        write!(f, "; docs_url={}", self.docs_url)?;
        write!(f, "; repair_hint={}", self.repair_hint)
    }
}

impl std::error::Error for DomainError {}

impl DomainError {
    fn with_context(
        kind: DomainErrorKind,
        purpose: impl Into<String>,
        reason: impl Into<String>,
        common_fixes: Vec<&'static str>,
        docs_url: &'static str,
        repair_hint: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            purpose: purpose.into(),
            reason: reason.into(),
            common_fixes,
            docs_url,
            repair_hint: repair_hint.into(),
        }
    }

    pub fn invalid_transition(from: WorkOrderStatus, action: &'static str) -> Self {
        Self::with_context(
            DomainErrorKind::InvalidTransition,
            "transition a work order",
            format!("invalid transition from {from:?} using {action}"),
            vec![
                "check the current work-order state before applying the action",
                "lease or approve the work order first if the transition requires it",
                "rerun the domain transition test after fixing the caller",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "fix the transition path, then rerun `cargo test -p jmcp-domain`",
        )
    }

    pub fn lease_expired() -> Self {
        Self::with_context(
            DomainErrorKind::LeaseExpired,
            "validate a lease",
            "lease expired",
            vec![
                "renew the lease with a fresh expiry",
                "check the system clock and lease TTL",
                "obtain a new lease before retrying the work order",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "refresh the lease, then rerun the lease/approval flow",
        )
    }

    pub fn lease_wrong_work_order() -> Self {
        Self::with_context(
            DomainErrorKind::LeaseWrongWorkOrder,
            "validate a lease",
            "lease does not match work order",
            vec![
                "bind the lease to the correct work-order id",
                "reload the lease from the current work order before retrying",
                "rerun the lease validation test after the mapping is fixed",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "attach the correct lease to the work order, then rerun the flow",
        )
    }

    pub fn lease_holder_mismatch() -> Self {
        Self::with_context(
            DomainErrorKind::LeaseHolderMismatch,
            "validate a lease",
            "lease holder mismatch",
            vec![
                "load the lease holder that owns the work order",
                "update the holder identity before retrying the action",
                "rerun the lease-holder validation test after fixing the caller",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "use the correct holder identity, then rerun the lease step",
        )
    }

    pub fn approval_expired() -> Self {
        Self::with_context(
            DomainErrorKind::ApprovalExpired,
            "resolve an approval challenge",
            "approval expired",
            vec![
                "request a new approval challenge token",
                "check the challenge expiry before retrying",
                "rerun the approval flow with a fresh token",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "issue a new approval challenge, then rerun the approval step",
        )
    }

    pub fn approval_already_used() -> Self {
        Self::with_context(
            DomainErrorKind::ApprovalAlreadyUsed,
            "resolve an approval challenge",
            "approval challenge already used",
            vec![
                "request a new approval token",
                "ensure the token is consumed only once",
                "rerun the approval test after updating the call site",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "create a fresh challenge token, then rerun approval",
        )
    }

    pub fn wrong_approver() -> Self {
        Self::with_context(
            DomainErrorKind::WrongApprover,
            "verify the approver",
            "wrong approver",
            vec![
                "use the approver identity that owns the challenge",
                "bind the Telegram user/chat to the challenge before retrying",
                "rerun the approval path with the correct approver",
            ],
            "https://docs.jmcp.dev/domain/errors",
            "fix the approver binding, then rerun the approval flow",
        )
    }

    pub fn repair_context(&self) -> DomainErrorRepairContext {
        DomainErrorRepairContext {
            purpose: self.purpose.clone(),
            reason: self.reason.clone(),
            common_fixes: self.common_fixes.clone(),
            docs_url: self.docs_url,
            repair_hint: self.repair_hint.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainErrorRepairContext {
    pub purpose: String,
    pub reason: String,
    pub common_fixes: Vec<&'static str>,
    pub docs_url: &'static str,
    pub repair_hint: String,
}

impl std::fmt::Display for DomainErrorRepairContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "domain error: purpose={}; reason={}",
            self.purpose, self.reason
        )?;
        if !self.common_fixes.is_empty() {
            write!(f, "; common_fixes=[{}]", self.common_fixes.join(", "))?;
        }
        write!(f, "; docs_url={}", self.docs_url)?;
        write!(f, "; repair_hint={}", self.repair_hint)
    }
}

impl std::error::Error for DomainErrorRepairContext {}
