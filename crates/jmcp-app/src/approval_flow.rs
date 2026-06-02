use crate::{
    AppError, AppResult, AppState, ApprovalDecisionError, ApprovalDecisionOutcome,
    CreatedApprovalChallenge,
};
use chrono::{Duration as ChronoDuration, Utc};
use jmcp_domain::{
    Approval, ApprovalActor, ApprovalChannel, ApprovalDecision, DomainError, WorkOrderStatus,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

impl AppState {
    pub fn create_local_approval_challenge(
        &self,
        work_order_id: Uuid,
        approver: impl Into<String>,
        ttl: Option<ChronoDuration>,
    ) -> AppResult<CreatedApprovalChallenge> {
        self.create_approval_challenge(
            work_order_id,
            approver.into(),
            ApprovalChannel::Local,
            None,
            None,
            ttl_or_default(ttl),
        )
    }

    pub fn create_telegram_approval_challenge(
        &self,
        work_order_id: Uuid,
        user_id: i64,
        chat_id: i64,
        ttl: Option<ChronoDuration>,
    ) -> AppResult<CreatedApprovalChallenge> {
        self.create_approval_challenge(
            work_order_id,
            telegram_approver(user_id),
            ApprovalChannel::Telegram,
            Some(user_id),
            Some(chat_id),
            ttl_or_default(ttl),
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
            if matches!(err.kind, jmcp_domain::DomainErrorKind::ApprovalExpired) {
                store
                    .record_approval_challenge(&challenge)
                    .map_err(unavailable_state)?;
            }
            return Err(decision_error(err));
        }

        let mut approval = store
            .get_approval(challenge.work_order_id, &challenge.approver)
            .map_err(unavailable_state)?
            .ok_or(ApprovalDecisionError::UnavailableState(
                "approval record missing".to_owned(),
            ))?;
        let mut work_order = store
            .get_work_order(challenge.work_order_id)
            .map_err(unavailable_state)?
            .ok_or(ApprovalDecisionError::UnavailableState(
                "work order record missing".to_owned(),
            ))?;
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

    fn create_approval_challenge(
        &self,
        work_order_id: Uuid,
        approver: String,
        channel: ApprovalChannel,
        target_user_id: Option<i64>,
        target_chat_id: Option<i64>,
        ttl: ChronoDuration,
    ) -> AppResult<CreatedApprovalChallenge> {
        let token = Uuid::new_v4().simple().to_string();
        let expires_at = Utc::now() + ttl;
        let store = self.store.lock().expect("store lock");
        let mut work_order = store
            .get_work_order(work_order_id)?
            .ok_or_else(|| AppError::State(format!("work order not found: {work_order_id}")))?;

        let approval = match work_order.status {
            WorkOrderStatus::Submitted | WorkOrderStatus::Leased => {
                let approval = work_order.require_approval(approver.clone(), ttl)?;
                work_order
                    .attention
                    .extend(self.policy.evaluate(&work_order));
                store.append_work_order("work_order.awaiting_approval", &work_order)?;
                approval
            }
            WorkOrderStatus::AwaitingApproval => {
                store.get_approval(work_order_id, &approver)?.map_or(
                    Approval {
                        work_order_id,
                        approver: approver.clone(),
                        expires_at,
                        decision: None,
                    },
                    |approval| approval,
                )
            }
            _ => {
                return Err(AppError::State(
                    "work order cannot accept an approval challenge".to_owned(),
                ))
            }
        };

        if approval.decision.is_some() {
            return Err(AppError::State(
                "approval has already been decided".to_owned(),
            ));
        }

        let challenge = jmcp_domain::ApprovalChallenge::new(
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
    match err.kind {
        jmcp_domain::DomainErrorKind::ApprovalExpired => ApprovalDecisionError::Expired,
        jmcp_domain::DomainErrorKind::ApprovalAlreadyUsed => ApprovalDecisionError::AlreadyUsed,
        jmcp_domain::DomainErrorKind::WrongApprover => ApprovalDecisionError::WrongApprover,
        other => ApprovalDecisionError::UnavailableState(other.to_string()),
    }
}

fn unavailable_state(err: impl std::fmt::Display) -> ApprovalDecisionError {
    ApprovalDecisionError::UnavailableState(err.to_string())
}

fn default_approval_token_ttl() -> ChronoDuration {
    ChronoDuration::minutes(15)
}

fn ttl_or_default(ttl: Option<ChronoDuration>) -> ChronoDuration {
    match ttl {
        Some(ttl) => ttl,
        None => default_approval_token_ttl(),
    }
}
