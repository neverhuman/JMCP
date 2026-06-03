use jmcp_domain::{
    ApprovalChallenge, ApprovalChallengeState, AttentionLevel, AttentionPacket, IncidentSeverity,
    Lease, PaneRisk, PaneStatus, WorkOrder, WorkOrderStatus,
};

use crate::{ranking::RankInput, reads::NowReads};

pub(super) fn rank_input(reads: &NowReads, work_order: &WorkOrder) -> RankInput {
    let attention = attention_for(reads, work_order.id);
    let lease = lease_for(reads, work_order.id);
    let challenge = open_challenge_for(reads, work_order.id);
    RankInput {
        id: work_order.id.to_string(),
        subject: work_order.subject.clone(),
        risk: risk_factor(reads, work_order, attention),
        blockedness: blockedness_factor(work_order, challenge),
        approval_expires_at: challenge.map(|challenge| challenge.expires_at),
        lease_expires_at: lease.map(|lease| lease.expires_at),
        adapter_degraded_weight: 0.0,
        evidence_gap_weight: evidence_gap_factor(work_order),
        user_query_relevance: user_relevance_factor(work_order, attention),
        updated_at: work_order.updated_at,
        downstream_blast_radius: downstream_blast_radius_factor(reads, work_order),
    }
}

pub(super) fn attention_for(
    reads: &NowReads,
    work_order_id: uuid::Uuid,
) -> Option<&AttentionPacket> {
    reads
        .attention_packets
        .iter()
        .filter(|packet| packet.work_order_id == Some(work_order_id))
        .max_by_key(|packet| packet.updated_at)
}

pub(super) fn lease_for(reads: &NowReads, work_order_id: uuid::Uuid) -> Option<&Lease> {
    reads
        .leases
        .iter()
        .find(|lease| lease.work_order_id == work_order_id)
}

pub(super) fn open_challenge_for(
    reads: &NowReads,
    work_order_id: uuid::Uuid,
) -> Option<&ApprovalChallenge> {
    reads.approval_challenges.iter().find(|challenge| {
        challenge.work_order_id == work_order_id
            && matches!(challenge.state, ApprovalChallengeState::Pending)
    })
}

pub(super) fn incident_severity_for(
    reads: &NowReads,
    work_order: &WorkOrder,
) -> Option<IncidentSeverity> {
    reads
        .incidents
        .iter()
        .filter(|incident| incident.related_work_orders.contains(&work_order.id))
        .map(|incident| incident.severity)
        .max_by_key(|severity| match severity {
            IncidentSeverity::Info => 0,
            IncidentSeverity::Warning => 1,
            IncidentSeverity::Major => 2,
            IncidentSeverity::Critical => 3,
        })
}

pub(super) fn pane_risk(risk: f32) -> PaneRisk {
    if risk >= 0.75 {
        PaneRisk::High
    } else if risk >= 0.45 {
        PaneRisk::Medium
    } else {
        PaneRisk::Low
    }
}

pub(super) fn pane_status(status: WorkOrderStatus) -> PaneStatus {
    match status {
        WorkOrderStatus::Failed | WorkOrderStatus::AwaitingApproval => PaneStatus::Active,
        WorkOrderStatus::Leased => PaneStatus::Warm,
        WorkOrderStatus::Submitted => PaneStatus::Incubating,
        WorkOrderStatus::Completed | WorkOrderStatus::Cancelled => PaneStatus::Discarded,
        WorkOrderStatus::Approved => PaneStatus::Predicted,
    }
}

pub(super) fn status_chip(status: WorkOrderStatus) -> &'static str {
    match status {
        WorkOrderStatus::Submitted => "submitted",
        WorkOrderStatus::Leased => "leased",
        WorkOrderStatus::AwaitingApproval => "awaiting_approval",
        WorkOrderStatus::Approved => "approved",
        WorkOrderStatus::Completed => "completed",
        WorkOrderStatus::Failed => "failed",
        WorkOrderStatus::Cancelled => "cancelled",
    }
}

pub(super) fn incident_chip(severity: IncidentSeverity) -> &'static str {
    match severity {
        IncidentSeverity::Info => "info",
        IncidentSeverity::Warning => "warning",
        IncidentSeverity::Major => "major",
        IncidentSeverity::Critical => "critical",
    }
}

fn risk_factor(
    reads: &NowReads,
    work_order: &WorkOrder,
    attention: Option<&AttentionPacket>,
) -> f32 {
    let status: f32 = match work_order.status {
        WorkOrderStatus::Failed => 0.9,
        WorkOrderStatus::AwaitingApproval => 0.75,
        WorkOrderStatus::Leased => 0.55,
        WorkOrderStatus::Submitted => 0.45,
        _ => 0.2,
    };
    let attention = attention
        .map(|packet| match packet.level {
            AttentionLevel::Page => 1.0,
            AttentionLevel::Warn => 0.7,
            AttentionLevel::Info => 0.35,
        })
        .unwrap_or(0.0);
    let incident = reads
        .incidents
        .iter()
        .filter(|incident| incident.related_work_orders.contains(&work_order.id))
        .map(|incident| match incident.severity {
            IncidentSeverity::Critical => 1.0,
            IncidentSeverity::Major => 0.85,
            IncidentSeverity::Warning => 0.6,
            IncidentSeverity::Info => 0.3,
        })
        .fold(0.0, f32::max);
    status.max(attention).max(incident)
}

fn blockedness_factor(work_order: &WorkOrder, challenge: Option<&ApprovalChallenge>) -> f32 {
    if challenge.is_some() {
        return 0.95;
    }
    match work_order.status {
        WorkOrderStatus::Failed => 0.9,
        WorkOrderStatus::AwaitingApproval => 0.85,
        WorkOrderStatus::Leased => 0.65,
        WorkOrderStatus::Submitted => 0.6,
        _ => 0.2,
    }
}

fn evidence_gap_factor(work_order: &WorkOrder) -> f32 {
    if work_order.evidence.is_empty() {
        0.75
    } else {
        0.0
    }
}

fn downstream_blast_radius_factor(reads: &NowReads, work_order: &WorkOrder) -> f32 {
    let incident = reads
        .incidents
        .iter()
        .filter(|incident| incident.related_work_orders.contains(&work_order.id))
        .map(|incident| match incident.severity {
            IncidentSeverity::Critical => 1.0,
            IncidentSeverity::Major => 0.85,
            IncidentSeverity::Warning => 0.55,
            IncidentSeverity::Info => 0.25,
        })
        .fold(0.0, f32::max);
    let status: f32 = match work_order.status {
        WorkOrderStatus::Failed => 0.75,
        WorkOrderStatus::AwaitingApproval => 0.7,
        WorkOrderStatus::Leased => 0.55,
        WorkOrderStatus::Submitted => 0.4,
        _ => 0.2,
    };
    status.max(incident)
}

fn user_relevance_factor(work_order: &WorkOrder, attention: Option<&AttentionPacket>) -> f32 {
    if matches!(
        attention.map(|packet| packet.level),
        Some(AttentionLevel::Page)
    ) {
        return 1.0;
    }
    match work_order.status {
        WorkOrderStatus::AwaitingApproval => 0.9,
        WorkOrderStatus::Failed => 0.8,
        WorkOrderStatus::Submitted | WorkOrderStatus::Leased => 0.7,
        _ => 0.4,
    }
}
