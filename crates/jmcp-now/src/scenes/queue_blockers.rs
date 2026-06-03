use std::collections::HashMap;

use chrono::{DateTime, Utc};
use jmcp_domain::{
    ActionSafetyClass, ApprovalChallenge, AttentionPacket, CardLod, CounterValue, DeckRankReason,
    IncidentSeverity, JituxEvidenceRef, Lease, PaneCounter, PaneKind, PanePreview, PaneRankReason,
    PaneVm, PreparedAction, PreparedTab, WorkOrder, WorkOrderStatus,
};

mod actions;
mod signals;

use crate::{ranking::rank_inputs, reads::NowReads};
use actions::{actions_for, evidence_for, safety_chip};
use signals::{
    attention_for, incident_chip, incident_severity_for, lease_for, open_challenge_for, pane_risk,
    pane_status, rank_input, status_chip,
};

pub const KEY: &str = "queue_blockers";
pub const TITLE: &str = "What's blocking the queue?";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct QueueBlockersProjection {
    pub panes: Vec<PaneVm>,
    pub rank_reasons: Vec<PaneRankReason>,
    pub prepared_actions: HashMap<String, Vec<PreparedAction>>,
    pub evidence_refs: HashMap<String, Vec<JituxEvidenceRef>>,
}

pub fn compose(reads: &NowReads, now: DateTime<Utc>) -> QueueBlockersProjection {
    let candidates = reads
        .work_orders
        .iter()
        .filter(|work_order| {
            matches!(
                work_order.status,
                WorkOrderStatus::Submitted
                    | WorkOrderStatus::Leased
                    | WorkOrderStatus::AwaitingApproval
                    | WorkOrderStatus::Failed
            )
        })
        .collect::<Vec<_>>();

    let inputs = candidates
        .iter()
        .map(|work_order| rank_input(reads, work_order))
        .collect::<Vec<_>>();
    let ranked = rank_inputs(inputs, now);
    let focus_input_id = ranked.first().map(|ranked| ranked.input.id.clone());

    let mut panes = Vec::new();
    let mut rank_reasons = Vec::new();
    let mut prepared_actions = HashMap::new();
    let mut evidence_refs = HashMap::new();

    for ranked in ranked {
        let Some(work_order) = candidates
            .iter()
            .copied()
            .find(|work_order| work_order.id.to_string() == ranked.input.id)
        else {
            continue;
        };
        let pane_id = pane_id(work_order.id);
        let actions = actions_for(reads, work_order);
        let evidence = evidence_for(work_order);
        let is_focus = focus_input_id.as_deref() == Some(ranked.input.id.as_str());
        let pane = pane(
            reads,
            work_order,
            &ranked.reason,
            now,
            is_focus,
            &actions,
            &evidence,
        );

        rank_reasons.push(PaneRankReason {
            pane_id: pane_id.clone(),
            reason: ranked.reason,
        });
        prepared_actions.insert(pane_id.clone(), actions);
        evidence_refs.insert(pane_id, evidence);
        panes.push(pane);
    }

    QueueBlockersProjection {
        panes,
        rank_reasons,
        prepared_actions,
        evidence_refs,
    }
}

pub fn panes(reads: &NowReads, now: DateTime<Utc>) -> Vec<PaneVm> {
    compose(reads, now).panes
}

pub fn pane_id(work_order_id: impl std::fmt::Display) -> String {
    format!("{KEY}:{work_order_id}")
}

fn pane(
    reads: &NowReads,
    work_order: &WorkOrder,
    reason: &DeckRankReason,
    now: DateTime<Utc>,
    is_focus: bool,
    actions: &[PreparedAction],
    evidence: &[JituxEvidenceRef],
) -> PaneVm {
    let attention = attention_for(reads, work_order.id);
    let lease = lease_for(reads, work_order.id);
    let challenge = open_challenge_for(reads, work_order.id);
    let incident = incident_severity_for(reads, work_order);

    PaneVm {
        id: pane_id(work_order.id),
        kind: PaneKind::Queue,
        title: work_order.subject.clone(),
        rank: reason.score,
        risk: pane_risk(reason.factors.risk),
        status: pane_status(work_order.status),
        lod: if is_focus {
            CardLod::Focus
        } else {
            CardLod::Preview
        },
        confidence: confidence(attention, lease, challenge, evidence),
        freshness_ms: freshness_ms(work_order.updated_at, now),
        preview: PanePreview {
            headline: why_now(work_order, attention, lease),
            chips: chips(work_order, lease, challenge, incident, actions, evidence),
            counters: counters(work_order, lease, challenge, actions, evidence, now),
        },
        prepared_tabs: prepared_tabs(lease, challenge, incident, actions, evidence),
    }
}

fn why_now(
    work_order: &WorkOrder,
    attention: Option<&AttentionPacket>,
    lease: Option<&Lease>,
) -> String {
    if let Some(attention) = attention {
        return attention.why_now.clone();
    }
    match work_order.status {
        WorkOrderStatus::Submitted => {
            "Submitted work order is waiting for a lease or approval path.".to_owned()
        }
        WorkOrderStatus::Leased => match lease {
            Some(lease) => format!(
                "Lease held by {} needs completion or renewal.",
                lease.holder
            ),
            None => "Leased work order has no visible lease record.".to_owned(),
        },
        WorkOrderStatus::AwaitingApproval => {
            "Work order is paused on approval before it can continue.".to_owned()
        }
        WorkOrderStatus::Failed => {
            "Failed work order needs manual recovery or fresh evidence.".to_owned()
        }
        _ => "Work order is visible in the queue blocker scene template.".to_owned(),
    }
}

fn chips(
    work_order: &WorkOrder,
    lease: Option<&Lease>,
    challenge: Option<&ApprovalChallenge>,
    incident: Option<IncidentSeverity>,
    actions: &[PreparedAction],
    evidence: &[JituxEvidenceRef],
) -> Vec<String> {
    let mut chips = vec![status_chip(work_order.status).to_owned()];
    if lease.is_some() {
        chips.push("lease".to_owned());
    }
    if challenge.is_some() {
        chips.push("approval_required".to_owned());
    }
    if !evidence.is_empty() {
        chips.push("evidence".to_owned());
    } else {
        chips.push("evidence_gap".to_owned());
    }
    if let Some(incident) = incident {
        chips.push(format!("incident_{}", incident_chip(incident)));
    }
    for safety in [
        ActionSafetyClass::ReadOnly,
        ActionSafetyClass::BoundedAuto,
        ActionSafetyClass::ApprovalRequired,
        ActionSafetyClass::ManualOnly,
    ] {
        if actions.iter().any(|action| action.safety == safety) {
            chips.push(safety_chip(safety).to_owned());
        }
    }
    chips
}

fn counters(
    work_order: &WorkOrder,
    lease: Option<&Lease>,
    challenge: Option<&ApprovalChallenge>,
    actions: &[PreparedAction],
    evidence: &[JituxEvidenceRef],
    now: DateTime<Utc>,
) -> Vec<PaneCounter> {
    let mut counters = vec![
        PaneCounter {
            label: "evidence".to_owned(),
            value: CounterValue::Number(evidence.len() as i64),
        },
        PaneCounter {
            label: "actions".to_owned(),
            value: CounterValue::Number(actions.len() as i64),
        },
        PaneCounter {
            label: "status".to_owned(),
            value: CounterValue::Text(status_chip(work_order.status).to_owned()),
        },
    ];
    if let Some(lease) = lease {
        counters.push(PaneCounter {
            label: "leaseMins".to_owned(),
            value: CounterValue::Number(minutes_until(lease.expires_at, now)),
        });
    }
    if let Some(challenge) = challenge {
        counters.push(PaneCounter {
            label: "approvalMins".to_owned(),
            value: CounterValue::Number(minutes_until(challenge.expires_at, now)),
        });
    }
    counters
}

fn prepared_tabs(
    lease: Option<&Lease>,
    challenge: Option<&ApprovalChallenge>,
    incident: Option<IncidentSeverity>,
    actions: &[PreparedAction],
    evidence: &[JituxEvidenceRef],
) -> Vec<PreparedTab> {
    let mut tabs = Vec::new();
    if !evidence.is_empty() {
        push_tab(&mut tabs, PreparedTab::Evidence);
    }
    if challenge.is_some() || !actions.is_empty() {
        push_tab(&mut tabs, PreparedTab::Actions);
    }
    if lease.is_some() || incident.is_some() {
        push_tab(&mut tabs, PreparedTab::Systems);
    }
    if tabs.is_empty() {
        push_tab(&mut tabs, PreparedTab::Raw);
    }
    tabs
}

fn push_tab(tabs: &mut Vec<PreparedTab>, tab: PreparedTab) {
    if !tabs.contains(&tab) {
        tabs.push(tab);
    }
}

fn confidence(
    attention: Option<&AttentionPacket>,
    lease: Option<&Lease>,
    challenge: Option<&ApprovalChallenge>,
    evidence: &[JituxEvidenceRef],
) -> f32 {
    let mut confidence: f32 = 0.68;
    if attention.is_some() {
        confidence += 0.08;
    }
    if lease.is_some() {
        confidence += 0.04;
    }
    if challenge.is_some() {
        confidence += 0.04;
    }
    if !evidence.is_empty() {
        confidence += 0.10;
    }
    confidence.clamp(0.0, 1.0)
}

fn freshness_ms(updated_at: DateTime<Utc>, now: DateTime<Utc>) -> Option<u64> {
    let elapsed = now.signed_duration_since(updated_at).num_milliseconds();
    Some(elapsed.max(0) as u64)
}

fn minutes_until(expires_at: DateTime<Utc>, now: DateTime<Utc>) -> i64 {
    expires_at.signed_duration_since(now).num_minutes().max(0)
}
