mod mock_support;

use chrono::Duration;
use jmcp_domain::{DeckRankReason, WorkOrderStatus};
use jmcp_now::{queue_blockers_projection, rank_inputs, rank_reason, RankInput};
use mock_support::{
    approval_challenge, attention_packet, fixed_time, incident_record, lease, uuid, work_order,
    work_order_with_evidence,
};
use jmcp_now::NowReads;

fn base_rank_input() -> RankInput {
    RankInput {
        id: "wo-a".to_owned(),
        subject: "Queue blocker".to_owned(),
        risk: 0.0,
        blockedness: 0.0,
        approval_expires_at: None,
        lease_expires_at: None,
        adapter_degraded_weight: 0.0,
        evidence_gap_weight: 0.0,
        user_query_relevance: 0.0,
        updated_at: fixed_time(),
        downstream_blast_radius: 0.0,
    }
}

fn only_factor(mut input: RankInput, setter: impl FnOnce(&mut RankInput, f32), value: f32) -> RankInput {
    setter(&mut input, value);
    input
}

fn reason_score(input: RankInput) -> f32 {
    rank_reason(&input, fixed_time()).score
}

#[test]
fn each_scalar_factor_is_monotonic() {
    let mut cases: Vec<(&str, Box<dyn Fn(RankInput, f32) -> RankInput>)> = vec![
        ("risk", Box::new(|input, value| only_factor(input, |input, v| input.risk = v, value))),
        (
            "blockedness",
            Box::new(|input, value| only_factor(input, |input, v| input.blockedness = v, value)),
        ),
        (
            "adapter_degraded_weight",
            Box::new(|input, value| only_factor(input, |input, v| input.adapter_degraded_weight = v, value)),
        ),
        (
            "evidence_gap_weight",
            Box::new(|input, value| only_factor(input, |input, v| input.evidence_gap_weight = v, value)),
        ),
        (
            "user_query_relevance",
            Box::new(|input, value| only_factor(input, |input, v| input.user_query_relevance = v, value)),
        ),
        (
            "downstream_blast_radius",
            Box::new(|input, value| only_factor(input, |input, v| input.downstream_blast_radius = v, value)),
        ),
    ];

    for (label, make_input) in cases.drain(..) {
        let low = reason_score(make_input(base_rank_input(), 0.2));
        let high = reason_score(make_input(base_rank_input(), 0.9));
        assert!(high > low, "{label} should raise score");
    }
}

#[test]
fn approval_and_lease_pressure_follow_the_expected_tiers() {
    let near_expiry = rank_reason(
        &RankInput {
            approval_expires_at: Some(fixed_time() + Duration::minutes(2)),
            lease_expires_at: Some(fixed_time() + Duration::minutes(2)),
            ..base_rank_input()
        },
        fixed_time(),
    );
    let boundary = rank_reason(
        &RankInput {
            approval_expires_at: Some(fixed_time() + Duration::minutes(60)),
            lease_expires_at: Some(fixed_time() + Duration::minutes(60)),
            ..base_rank_input()
        },
        fixed_time(),
    );
    let expired = rank_reason(
        &RankInput {
            approval_expires_at: Some(fixed_time() - Duration::minutes(1)),
            lease_expires_at: Some(fixed_time() - Duration::minutes(1)),
            ..base_rank_input()
        },
        fixed_time(),
    );
    let missing = rank_reason(&base_rank_input(), fixed_time());

    assert!(near_expiry.factors.approval_expiry_pressure > boundary.factors.approval_expiry_pressure);
    assert_eq!(boundary.factors.approval_expiry_pressure, 0.0);
    assert_eq!(expired.factors.approval_expiry_pressure, 1.0);
    assert_eq!(missing.factors.approval_expiry_pressure, 0.0);
    assert!(near_expiry.factors.lease_pressure > boundary.factors.lease_pressure);
    assert_eq!(boundary.factors.lease_pressure, 0.0);
    assert_eq!(expired.factors.lease_pressure, 1.0);
    assert_eq!(missing.factors.lease_pressure, 0.0);
}

#[test]
fn freshness_decays_at_two_minutes_then_hits_zero_at_sixty_and_beyond() {
    let now = fixed_time();
    let fresh = rank_reason(
        &RankInput {
            updated_at: now,
            ..base_rank_input()
        },
        now,
    );
    let two_minutes = rank_reason(
        &RankInput {
            updated_at: now - Duration::minutes(2),
            ..base_rank_input()
        },
        now,
    );
    let sixty_minutes = rank_reason(
        &RankInput {
            updated_at: now - Duration::minutes(60),
            ..base_rank_input()
        },
        now,
    );
    let older_than_sixty = rank_reason(
        &RankInput {
            updated_at: now - Duration::minutes(61),
            ..base_rank_input()
        },
        now,
    );

    assert_eq!(fresh.factors.freshness, 1.0);
    assert!(two_minutes.factors.freshness < fresh.factors.freshness);
    assert!(two_minutes.factors.freshness > sixty_minutes.factors.freshness);
    assert_eq!(sixty_minutes.factors.freshness, 0.0);
    assert_eq!(older_than_sixty.factors.freshness, 0.0);
}

#[test]
fn weighted_sum_differs_from_zeroing_only_by_positive_components() {
    let reason = rank_reason(
        &RankInput {
            risk: 1.0,
            blockedness: 1.0,
            approval_expires_at: Some(fixed_time() + Duration::minutes(2)),
            lease_expires_at: Some(fixed_time() + Duration::minutes(2)),
            adapter_degraded_weight: 1.0,
            evidence_gap_weight: 1.0,
            user_query_relevance: 1.0,
            updated_at: fixed_time(),
            downstream_blast_radius: 1.0,
            ..base_rank_input()
        },
        fixed_time(),
    );
    let zero = rank_reason(
        &RankInput {
            updated_at: fixed_time() - Duration::minutes(61),
            ..base_rank_input()
        },
        fixed_time(),
    );

    assert_eq!(zero.score, 0.0);
    assert!(reason.score > zero.score);
    assert!(
        (reason.score
            - (0.25
                + 0.20
                + 0.10 * 0.966_666_64
                + 0.10 * 0.966_666_64
                + 0.05
                + 0.10
                + 0.05
                + 0.05
                + 0.10))
        .abs()
            < 0.0001
    );
}

#[test]
fn stable_tie_break_uses_id_ascending() {
    let now = fixed_time();
    let ranked = rank_inputs(
        vec![
            RankInput {
                id: "wo-b".to_owned(),
                ..base_rank_input()
            },
            RankInput {
                id: "wo-a".to_owned(),
                ..base_rank_input()
            },
            RankInput {
                id: "wo-c".to_owned(),
                ..base_rank_input()
            },
        ],
        now,
    );

    assert_eq!(ranked.iter().map(|ranked| ranked.input.id.as_str()).collect::<Vec<_>>(), vec!["wo-a", "wo-b", "wo-c"]);
}

#[test]
fn dominant_factor_matches_the_highest_weighted_contribution() {
    let reason = rank_reason(
        &RankInput {
            risk: 0.9,
            blockedness: 0.1,
            approval_expires_at: Some(fixed_time() + Duration::minutes(60)),
            lease_expires_at: Some(fixed_time() + Duration::minutes(60)),
            adapter_degraded_weight: 0.1,
            evidence_gap_weight: 0.1,
            user_query_relevance: 0.1,
            updated_at: fixed_time() - Duration::minutes(60),
            downstream_blast_radius: 0.1,
            ..base_rank_input()
        },
        fixed_time(),
    );

    assert!(reason.explanation.contains("risk is highest"));
    assert_eq!(reason.factors.risk, 0.9);
}

#[test]
fn downstream_blast_radius_rises_with_status_and_incident_membership() {
    let submitted = queue_blockers_projection(
        &NowReads {
            work_orders: vec![work_order(uuid("22222222-2222-4222-8222-222222222221"), "Submitted", WorkOrderStatus::Submitted)],
            autonomous_actions: Vec::new(),
            ..NowReads::default()
        },
        fixed_time(),
    );
    let failed = queue_blockers_projection(
        &NowReads {
            work_orders: vec![work_order(uuid("22222222-2222-4222-8222-222222222222"), "Failed", WorkOrderStatus::Failed)],
            autonomous_actions: Vec::new(),
            ..NowReads::default()
        },
        fixed_time(),
    );
    let incident_boosted = queue_blockers_projection(
        &NowReads {
            work_orders: vec![work_order(uuid("22222222-2222-4222-8222-222222222223"), "Incident", WorkOrderStatus::Submitted)],
            incidents: vec![incident_record(
                uuid("22222222-2222-4222-8222-222222222223"),
                jmcp_domain::IncidentSeverity::Critical,
                "Queue blast radius",
            )],
            autonomous_actions: Vec::new(),
            ..NowReads::default()
        },
        fixed_time(),
    );

    let submitted_factor = submitted.rank_reasons[0].reason.factors.downstream_blast_radius;
    let failed_factor = failed.rank_reasons[0].reason.factors.downstream_blast_radius;
    let incident_factor = incident_boosted.rank_reasons[0].reason.factors.downstream_blast_radius;

    assert!(failed_factor > submitted_factor);
    assert!(incident_factor >= failed_factor);
}

#[test]
fn approval_and_autonomous_paths_can_coexist_without_secret_material() {
    let work_order_id = uuid("22222222-2222-4222-8222-222222222224");
    let projection = queue_blockers_projection(
        &NowReads {
            work_orders: vec![work_order_with_evidence(
                work_order_id,
                "Approved with paths",
                WorkOrderStatus::AwaitingApproval,
            )],
            leases: vec![lease(work_order_id, "jmcpd", 10)],
            approval_challenges: vec![approval_challenge(work_order_id, 20)],
            attention_packets: vec![attention_packet(Some(work_order_id), "Attention owns the why-now headline.", jmcp_domain::AttentionLevel::Page)],
            incidents: vec![incident_record(
                work_order_id,
                jmcp_domain::IncidentSeverity::Major,
                "Queue blocker incident",
            )],
            autonomous_actions: jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap())
                .list_autonomous_actions()
                .unwrap(),
        },
        fixed_time(),
    );

    let pane = &projection.panes[0];
    assert!(pane.preview.headline.contains("Attention owns the why-now headline."));
    assert!(pane.preview.chips.contains(&"awaiting_approval".to_owned()));
    assert!(pane.preview.chips.contains(&"lease".to_owned()));
    assert!(pane.preview.chips.contains(&"approval_required".to_owned()));
    assert!(pane.preview.chips.contains(&"evidence".to_owned()));
    assert!(pane.preview.chips.iter().any(|chip| chip.starts_with("incident_")));
    assert!(pane.prepared_tabs.contains(&jmcp_domain::PreparedTab::Evidence));
    assert!(pane.prepared_tabs.contains(&jmcp_domain::PreparedTab::Actions));
    assert!(pane.prepared_tabs.contains(&jmcp_domain::PreparedTab::Systems));
}
