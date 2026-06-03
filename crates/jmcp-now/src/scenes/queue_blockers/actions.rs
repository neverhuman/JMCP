use jmcp_domain::{
    ActionSafetyClass, JituxEvidenceRef, PreparedAction, WorkOrder, WorkOrderStatus,
};

use super::signals::open_challenge_for;
use crate::reads::NowReads;

pub(super) fn evidence_for(work_order: &WorkOrder) -> Vec<JituxEvidenceRef> {
    work_order
        .evidence
        .iter()
        .enumerate()
        .map(|(index, evidence)| JituxEvidenceRef {
            id: format!("evidence:{}:{index}", work_order.id),
            label: evidence.kind.clone(),
            uri: evidence.uri.clone(),
            captured_at: evidence.captured_at,
        })
        .collect()
}

pub(super) fn actions_for(reads: &NowReads, work_order: &WorkOrder) -> Vec<PreparedAction> {
    let challenge = open_challenge_for(reads, work_order.id);
    let mut actions = Vec::new();
    if !work_order.evidence.is_empty() {
        actions.push(PreparedAction {
            id: format!("read-evidence:{}", work_order.id),
            label: "Open evidence".to_owned(),
            command: format!("jmcp.now.read_evidence {}", work_order.id),
            safety: ActionSafetyClass::ReadOnly,
            ready: true,
            requires_approval: false,
            reason: "Evidence reads do not mutate JMCP state.".to_owned(),
            preview_ref: Some(format!("jmcp://work-orders/{}/evidence", work_order.id)),
        });
    }
    if let Some(challenge) = challenge {
        actions.push(PreparedAction {
            id: format!("approval:{}", challenge.id),
            label: "Review approval".to_owned(),
            command: format!("jmcp.approval.review {}", challenge.id),
            safety: ActionSafetyClass::ApprovalRequired,
            ready: false,
            requires_approval: true,
            reason: "An open approval challenge controls the next mutating step.".to_owned(),
            preview_ref: Some(format!("jmcp://approval-challenges/{}", challenge.id)),
        });
    }
    if let Some(action) = reads
        .autonomous_actions
        .iter()
        .find(|action| !action.safety.live)
    {
        actions.push(PreparedAction {
            id: format!("bounded-auto:{}:{}", work_order.id, action.id),
            label: action.title.clone(),
            command: format!("jmcp.autonomy.prepare {}", action.id),
            safety: ActionSafetyClass::BoundedAuto,
            ready: true,
            requires_approval: false,
            reason: "The autonomous action manifest is evidence-oriented and live=false."
                .to_owned(),
            preview_ref: Some(format!("jmcp://autonomous-actions/{}", action.id)),
        });
    }
    if matches!(work_order.status, WorkOrderStatus::Failed) || actions.is_empty() {
        actions.push(PreparedAction {
            id: format!("manual:{}", work_order.id),
            label: "Manual review".to_owned(),
            command: format!("jmcp.now.manual_review {}", work_order.id),
            safety: ActionSafetyClass::ManualOnly,
            ready: false,
            requires_approval: false,
            reason: "No governed automatic path is ready for this blocker.".to_owned(),
            preview_ref: Some(format!("jmcp://work-orders/{}", work_order.id)),
        });
    }
    actions
}

pub(super) fn safety_chip(safety: ActionSafetyClass) -> &'static str {
    match safety {
        ActionSafetyClass::ReadOnly => "read_only",
        ActionSafetyClass::BoundedAuto => "bounded_auto",
        ActionSafetyClass::ApprovalRequired => "approval_required_action",
        ActionSafetyClass::ManualOnly => "manual_only",
    }
}
