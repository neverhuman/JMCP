use chrono::{DateTime, Utc};

use crate::{
    contract::{
        Accent, Card, CardKind, CardStatus, Counter, CounterValue, DrilldownKind, DrilldownRef,
        NowSnapshot, PaneKind, PanePreview, PaneStatus, PreparedAction, RiskBand, SafetyClass,
        Scene, SceneLayout, SceneMode,
    },
    ranking::{rank_inputs, RankInput},
    reads::NowReads,
};

pub const KEY: &str = "queue_blockers";

pub fn compose(reads: &NowReads, generation: i64, captured_at: DateTime<Utc>) -> Scene {
    let candidates = reads
        .work_orders
        .iter()
        .filter(|work_order| {
            matches!(
                work_order.status,
                jmcp_domain::WorkOrderStatus::Submitted
                    | jmcp_domain::WorkOrderStatus::Leased
                    | jmcp_domain::WorkOrderStatus::AwaitingApproval
                    | jmcp_domain::WorkOrderStatus::Failed
            )
        })
        .collect::<Vec<_>>();

    let inputs = candidates
        .iter()
        .map(|work_order| rank_input(reads, work_order))
        .collect::<Vec<_>>();
    let ranked = rank_inputs(inputs, captured_at);

    let cards = ranked
        .into_iter()
        .filter_map(|ranked| {
            let work_order = candidates
                .iter()
                .copied()
                .find(|work_order| work_order.id.to_string() == ranked.input.id)?;
            Some(card(reads, work_order, ranked.reason))
        })
        .collect::<Vec<_>>();

    Scene {
        key: KEY.to_owned(),
        kind: PaneKind::Queue,
        mode: SceneMode::Focus,
        accent: Accent::Purple,
        title: "What's blocking the queue?".to_owned(),
        layout: SceneLayout::Stack,
        status: PaneStatus::Active,
        generation,
        captured_at,
        cards,
        narration_hint:
            "Lead with the top blocker, then name the next approval, lease, or evidence step."
                .to_owned(),
    }
}

pub fn snapshot(scene: &Scene, generation: i64, captured_at: DateTime<Utc>) -> NowSnapshot {
    let high_risk = scene
        .cards
        .iter()
        .filter(|card| matches!(card.risk, RiskBand::High))
        .count();
    let top_score = scene.cards.first().map(|card| card.rank).unwrap_or(0.0);
    NowSnapshot {
        generation,
        captured_at,
        default_pane: KEY.to_owned(),
        deck: vec![PanePreview {
            id: KEY.to_owned(),
            kind: PaneKind::Queue,
            title: scene.title.clone(),
            headline: match scene.cards.first() {
                Some(card) => card.title.clone(),
                None => "Queue has no submitted, leased, approval, or failed blockers".to_owned(),
            },
            chips: vec![
                format!("{} blockers", scene.cards.len()),
                format!("{high_risk} high risk"),
            ],
            counters: vec![
                Counter {
                    label: "cards".to_owned(),
                    value: CounterValue::Number(scene.cards.len() as f64),
                },
                Counter {
                    label: "topScore".to_owned(),
                    value: CounterValue::Number(top_score),
                },
            ],
            sparkline: None,
            rank: top_score,
            focus_score: top_score,
            confidence: if scene.cards.is_empty() { 0.5 } else { 0.86 },
            severity: if high_risk > 0 {
                RiskBand::High
            } else {
                RiskBand::Medium
            },
            status: PaneStatus::Active,
            predicted_next: vec![
                "approval".to_owned(),
                "replay".to_owned(),
                "evidence".to_owned(),
            ],
        }],
    }
}

fn card(reads: &NowReads, work_order: &jmcp_domain::WorkOrder, reason: crate::RankReason) -> Card {
    let attention = attention_for(reads, work_order.id);
    let lease = reads
        .leases
        .iter()
        .find(|lease| lease.work_order_id == work_order.id);
    let challenge = open_challenge_for(reads, work_order.id);
    let evidence_refs = evidence_refs(work_order);
    let mut drilldowns = evidence_refs.clone();
    if let Some(lease) = lease {
        drilldowns.push(DrilldownRef {
            id: format!("lease:{}", lease.work_order_id),
            label: format!("Lease held by {}", lease.holder),
            kind: DrilldownKind::Lease,
            target: format!("leases/{}", lease.work_order_id),
        });
    }
    if let Some(challenge) = challenge {
        drilldowns.push(DrilldownRef {
            id: format!("approval:{}", challenge.id),
            label: format!("Approval from {}", challenge.approver),
            kind: DrilldownKind::Approval,
            target: format!("approval-challenges/{}", challenge.id),
        });
    }

    Card {
        id: work_order.id.to_string(),
        kind: CardKind::QueueBlocker,
        title: work_order.subject.clone(),
        status: if work_order.evidence.is_empty() {
            CardStatus::Ranked
        } else {
            CardStatus::Verified
        },
        rank: reason.score,
        risk: risk_band(reason.factors.risk),
        why_now: why_now(work_order, attention, lease),
        rank_reason: reason,
        evidence_refs,
        drilldowns,
        actions: actions_for(reads, work_order, challenge),
    }
}

fn rank_input(reads: &NowReads, work_order: &jmcp_domain::WorkOrder) -> RankInput {
    let attention = attention_for(reads, work_order.id);
    let lease = reads
        .leases
        .iter()
        .find(|lease| lease.work_order_id == work_order.id);
    let challenge = open_challenge_for(reads, work_order.id);
    RankInput {
        id: work_order.id.to_string(),
        subject: work_order.subject.clone(),
        risk: risk_factor(reads, work_order, attention),
        actionability: actionability_factor(work_order, challenge),
        updated_at: work_order.updated_at,
        blast_radius: blast_radius_factor(reads, work_order),
        lease_expires_at: lease.map(|lease| lease.expires_at),
        user_relevance: user_relevance_factor(work_order, attention),
    }
}

fn attention_for(
    reads: &NowReads,
    work_order_id: uuid::Uuid,
) -> Option<&jmcp_domain::AttentionPacket> {
    reads
        .attention_packets
        .iter()
        .filter(|packet| packet.work_order_id == Some(work_order_id))
        .max_by_key(|packet| packet.updated_at)
}

fn open_challenge_for(
    reads: &NowReads,
    work_order_id: uuid::Uuid,
) -> Option<&jmcp_domain::ApprovalChallenge> {
    reads.approval_challenges.iter().find(|challenge| {
        challenge.work_order_id == work_order_id
            && matches!(
                challenge.state,
                jmcp_domain::ApprovalChallengeState::Pending
            )
    })
}

fn evidence_refs(work_order: &jmcp_domain::WorkOrder) -> Vec<DrilldownRef> {
    work_order
        .evidence
        .iter()
        .enumerate()
        .map(|(index, evidence)| DrilldownRef {
            id: format!("evidence:{}:{index}", work_order.id),
            label: evidence.kind.clone(),
            kind: DrilldownKind::Evidence,
            target: evidence.uri.clone(),
        })
        .collect()
}

fn actions_for(
    reads: &NowReads,
    work_order: &jmcp_domain::WorkOrder,
    challenge: Option<&jmcp_domain::ApprovalChallenge>,
) -> Vec<PreparedAction> {
    let mut actions = Vec::new();
    if !work_order.evidence.is_empty() {
        actions.push(PreparedAction {
            id: format!("read-evidence:{}", work_order.id),
            label: "Open evidence".to_owned(),
            safety_class: SafetyClass::ReadOnly,
            ready: true,
            reason: "Evidence reads do not mutate JMCP state.".to_owned(),
            target: format!("work-orders/{}/evidence", work_order.id),
            method: crate::ActionMethod::Get,
        });
    }
    if let Some(challenge) = challenge {
        actions.push(PreparedAction {
            id: format!("approval:{}", challenge.id),
            label: "Review approval".to_owned(),
            safety_class: SafetyClass::ApprovalRequired,
            ready: false,
            reason: "An open approval challenge controls the next mutating step.".to_owned(),
            target: format!("approval-challenges/{}", challenge.id),
            method: crate::ActionMethod::Post,
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
            safety_class: SafetyClass::BoundedAuto,
            ready: true,
            reason: "The autonomous action manifest is evidence-oriented and live=false."
                .to_owned(),
            target: format!("autonomous-actions/{}", action.id),
            method: crate::ActionMethod::Post,
        });
    }
    if matches!(work_order.status, jmcp_domain::WorkOrderStatus::Failed) || actions.is_empty() {
        actions.push(PreparedAction {
            id: format!("manual:{}", work_order.id),
            label: "Manual review".to_owned(),
            safety_class: SafetyClass::ManualOnly,
            ready: false,
            reason: "No governed automatic path is ready for this blocker.".to_owned(),
            target: format!("work-orders/{}", work_order.id),
            method: crate::ActionMethod::Get,
        });
    }
    actions
}

fn why_now(
    work_order: &jmcp_domain::WorkOrder,
    attention: Option<&jmcp_domain::AttentionPacket>,
    lease: Option<&jmcp_domain::Lease>,
) -> String {
    if let Some(attention) = attention {
        return attention.why_now.clone();
    }
    match work_order.status {
        jmcp_domain::WorkOrderStatus::Submitted => {
            "Submitted work order is waiting for a lease or approval path.".to_owned()
        }
        jmcp_domain::WorkOrderStatus::Leased => match lease {
            Some(lease) => format!(
                "Lease held by {} needs completion or renewal.",
                lease.holder
            ),
            None => "Leased work order has no visible lease record.".to_owned(),
        },
        jmcp_domain::WorkOrderStatus::AwaitingApproval => {
            "Work order is paused on approval before it can continue.".to_owned()
        }
        jmcp_domain::WorkOrderStatus::Failed => {
            "Failed work order needs manual recovery or fresh evidence.".to_owned()
        }
        _ => "Work order is visible in the queue blocker scene.".to_owned(),
    }
}

fn risk_factor(
    reads: &NowReads,
    work_order: &jmcp_domain::WorkOrder,
    attention: Option<&jmcp_domain::AttentionPacket>,
) -> f64 {
    let status: f64 = match work_order.status {
        jmcp_domain::WorkOrderStatus::Failed => 0.9,
        jmcp_domain::WorkOrderStatus::AwaitingApproval => 0.75,
        jmcp_domain::WorkOrderStatus::Leased => 0.55,
        jmcp_domain::WorkOrderStatus::Submitted => 0.45,
        _ => 0.2,
    };
    let attention = attention
        .map(|packet| match packet.level {
            jmcp_domain::AttentionLevel::Page => 1.0,
            jmcp_domain::AttentionLevel::Warn => 0.7,
            jmcp_domain::AttentionLevel::Info => 0.35,
        })
        .unwrap_or(0.0);
    let incident = reads
        .incidents
        .iter()
        .filter(|incident| incident.related_work_orders.contains(&work_order.id))
        .map(|incident| match incident.severity {
            jmcp_domain::IncidentSeverity::Critical => 1.0,
            jmcp_domain::IncidentSeverity::Major => 0.85,
            jmcp_domain::IncidentSeverity::Warning => 0.6,
            jmcp_domain::IncidentSeverity::Info => 0.3,
        })
        .fold(0.0, f64::max);
    status.max(attention).max(incident)
}

fn actionability_factor(
    work_order: &jmcp_domain::WorkOrder,
    challenge: Option<&jmcp_domain::ApprovalChallenge>,
) -> f64 {
    if challenge.is_some() {
        return 0.9;
    }
    match work_order.status {
        jmcp_domain::WorkOrderStatus::Submitted => 0.7,
        jmcp_domain::WorkOrderStatus::Leased => 0.65,
        jmcp_domain::WorkOrderStatus::AwaitingApproval => 0.8,
        jmcp_domain::WorkOrderStatus::Failed => 0.35,
        _ => 0.2,
    }
}

fn blast_radius_factor(reads: &NowReads, work_order: &jmcp_domain::WorkOrder) -> f64 {
    let incident = reads
        .incidents
        .iter()
        .filter(|incident| incident.related_work_orders.contains(&work_order.id))
        .map(|incident| match incident.severity {
            jmcp_domain::IncidentSeverity::Critical => 1.0,
            jmcp_domain::IncidentSeverity::Major => 0.85,
            jmcp_domain::IncidentSeverity::Warning => 0.55,
            jmcp_domain::IncidentSeverity::Info => 0.25,
        })
        .fold(0.0, f64::max);
    let status: f64 = match work_order.status {
        jmcp_domain::WorkOrderStatus::Failed => 0.75,
        jmcp_domain::WorkOrderStatus::AwaitingApproval => 0.7,
        jmcp_domain::WorkOrderStatus::Leased => 0.55,
        jmcp_domain::WorkOrderStatus::Submitted => 0.4,
        _ => 0.2,
    };
    status.max(incident)
}

fn user_relevance_factor(
    work_order: &jmcp_domain::WorkOrder,
    attention: Option<&jmcp_domain::AttentionPacket>,
) -> f64 {
    if matches!(
        attention.map(|packet| packet.level),
        Some(jmcp_domain::AttentionLevel::Page)
    ) {
        return 1.0;
    }
    match work_order.status {
        jmcp_domain::WorkOrderStatus::AwaitingApproval => 0.9,
        jmcp_domain::WorkOrderStatus::Failed => 0.8,
        jmcp_domain::WorkOrderStatus::Submitted | jmcp_domain::WorkOrderStatus::Leased => 0.7,
        _ => 0.4,
    }
}

fn risk_band(risk: f64) -> RiskBand {
    if risk >= 0.75 {
        RiskBand::High
    } else if risk >= 0.45 {
        RiskBand::Medium
    } else {
        RiskBand::Low
    }
}
