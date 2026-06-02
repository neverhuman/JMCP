use crate::{
    ApprovalActor, ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, ApprovalDecision,
    DomainError,
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

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
            return Err(DomainError::approval_already_used());
        }
        if now > self.expires_at {
            self.state = ApprovalChallengeState::Expired;
            self.updated_at = now;
            return Err(DomainError::approval_expired());
        }
        if self.approver != actor.approver
            || self.target_user_id != actor.telegram_user_id
            || self.target_chat_id != actor.telegram_chat_id
        {
            return Err(DomainError::wrong_approver());
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
