use super::*;
use chrono::{Duration, Utc};
use jmcp_domain::{
    Approval, ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, AttentionLevel,
    AttentionPacket, Evidence, IncidentRecord, IncidentSeverity, IncidentState, InventoryCard,
    InventoryCardKind, Lease, MemoryPromotionState, MemoryRecord, PromotionDecision,
    PromotionVerdict, VoiceCandidate, VoiceRiskLevel, VoiceSession, VoiceSessionState, WorkOrder,
};
use serde_json::json;
use uuid::Uuid;

#[test]
fn projects_work_orders() {
    let store = SqliteStore::in_memory().unwrap();
    let wo = WorkOrder::submit("t/s/e", "demo", json!({}));
    store
        .append_work_order("work_order.submitted", &wo)
        .unwrap();
    assert_eq!(store.list_work_orders().unwrap().len(), 1);
    assert_eq!(store.events_after(0).unwrap().len(), 1);
}

#[test]
fn projects_runtime_records_and_replays_without_effects() {
    let store = SqliteStore::in_memory().unwrap();
    let wo = WorkOrder::submit("t/s/e", "demo", json!({}));
    let lease = Lease {
        work_order_id: wo.id,
        holder: "adapter".to_owned(),
        expires_at: Utc::now() + Duration::minutes(5),
    };
    let approval = Approval {
        work_order_id: wo.id,
        approver: "user".to_owned(),
        expires_at: Utc::now() + Duration::minutes(5),
        decision: None,
    };
    let challenge = ApprovalChallenge::new(
        wo.id,
        "user",
        ApprovalChannel::Local,
        None,
        None,
        "sha256:test",
        Utc::now() + Duration::minutes(5),
    );
    let evidence = Evidence {
        kind: "command.digest".to_owned(),
        uri: "sha256:test".to_owned(),
        captured_at: Utc::now(),
    };

    store
        .append_work_order("work_order.submitted", &wo)
        .unwrap();
    store.record_lease(&lease).unwrap();
    store.record_approval(&approval).unwrap();
    store.record_approval_challenge(&challenge).unwrap();
    store.record_evidence(Some(wo.id), &evidence).unwrap();

    assert_eq!(store.get_work_order(wo.id).unwrap(), Some(wo.clone()));
    assert_eq!(
        store.get_approval(wo.id, "user").unwrap(),
        Some(approval.clone())
    );
    assert_eq!(store.list_leases().unwrap(), vec![lease]);
    assert_eq!(store.list_approvals().unwrap(), vec![approval]);
    assert_eq!(
        store
            .approval_challenge_by_token_hash("sha256:test")
            .unwrap(),
        Some(challenge.clone())
    );
    assert_eq!(store.list_approval_challenges().unwrap(), vec![challenge]);
    assert_eq!(store.list_evidence().unwrap(), vec![evidence]);

    let checkpoint = store.rebuild_work_order_projection_from_events().unwrap();
    assert_eq!(checkpoint.rebuilt_work_orders, 1);
    assert!(!checkpoint.side_effects_reissued);
    assert_eq!(store.list_replay_checkpoints().unwrap().len(), 1);
    assert_eq!(
        store
            .approval_challenge_by_token_hash("sha256:test")
            .unwrap()
            .unwrap()
            .state,
        ApprovalChallengeState::Pending
    );
}

#[test]
fn projects_control_plane_records_and_replays() {
    let store = SqliteStore::in_memory().unwrap();
    let anchor = chrono::DateTime::parse_from_rfc3339("2025-01-01T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let session = VoiceSession {
        id: Uuid::new_v4(),
        work_order_id: None,
        channel: "voice".to_owned(),
        transcript: "approve the change".to_owned(),
        confidence: 0.99,
        candidate: VoiceCandidate {
            decision: jmcp_domain::ApprovalDecision::Approved,
            risk: VoiceRiskLevel::High,
            confirmation_token: Some("alpha".to_owned()),
        },
        confirmation_evidence: vec![Evidence {
            kind: "voice.transcript".to_owned(),
            uri: "sha256:voice".to_owned(),
            captured_at: anchor,
        }],
        state: VoiceSessionState::Confirmed,
        created_at: anchor,
        updated_at: anchor,
    };
    let packet = AttentionPacket {
        id: Uuid::new_v4(),
        work_order_id: None,
        title: "Action required".to_owned(),
        why_now: "Attention inbox should not be blank".to_owned(),
        alternatives: vec!["defer".to_owned()],
        risk_delta: "medium".to_owned(),
        drill_down: "open the control plane".to_owned(),
        level: AttentionLevel::Page,
        created_at: anchor,
        updated_at: anchor,
    };
    let memory = MemoryRecord {
        id: Uuid::new_v4(),
        lesson: "Keep replay deterministic".to_owned(),
        scope: "project".to_owned(),
        source_evidence: vec![Evidence {
            kind: "note".to_owned(),
            uri: "sha256:memory".to_owned(),
            captured_at: anchor,
        }],
        freshness: "fresh".to_owned(),
        counterexamples: vec!["blank screens".to_owned()],
        poisoning_checks: vec!["verify evidence".to_owned()],
        promotion_policy: "shadow".to_owned(),
        state: MemoryPromotionState::Proposed,
        expires_at: Some(anchor + Duration::days(7)),
        created_at: anchor,
        updated_at: anchor,
    };
    let card = InventoryCard {
        id: Uuid::new_v4(),
        kind: InventoryCardKind::Tool,
        name: "jmcpd.submit".to_owned(),
        owner: "jmcpd".to_owned(),
        allowed_uses: vec!["submit envelopes".to_owned()],
        disallowed_uses: vec!["bypass policy".to_owned()],
        cost: "sqlite writes".to_owned(),
        tests: vec!["smoke".to_owned()],
        safety_case: "gated".to_owned(),
        health: jmcp_domain::HealthLevel::Nominal,
        repo: Some("JMCP".to_owned()),
        provider: Some("jmcpd".to_owned()),
        queue: Some(0),
    };
    let decision = PromotionDecision {
        id: Uuid::new_v4(),
        target_kind: "tool_card".to_owned(),
        target_name: "jmcpd.submit".to_owned(),
        gate: "approval".to_owned(),
        verdict: PromotionVerdict::Promoted,
        verifier: "ops".to_owned(),
        rollback_plan: "disable route".to_owned(),
        evidence_count: 1,
        created_at: anchor,
        decided_at: anchor,
    };
    let incident = IncidentRecord {
        id: Uuid::new_v4(),
        title: "quarantine active".to_owned(),
        severity: IncidentSeverity::Major,
        state: IncidentState::Quarantined,
        quarantine_scope: "adapter".to_owned(),
        containment: "read only".to_owned(),
        related_work_orders: vec![],
        notes: vec!["sample".to_owned()],
        opened_at: anchor,
        updated_at: anchor,
    };

    store.record_voice_session(&session).unwrap();
    store.record_attention_packet(&packet).unwrap();
    store.record_memory_record(&memory).unwrap();
    store.record_inventory_card(&card).unwrap();
    store.record_promotion_decision(&decision).unwrap();
    store.record_incident_record(&incident).unwrap();

    assert_eq!(store.list_voice_sessions().unwrap(), vec![session.clone()]);
    assert_eq!(
        store.list_attention_packets().unwrap(),
        vec![packet.clone()]
    );
    assert_eq!(store.list_memory_records().unwrap(), vec![memory.clone()]);
    assert_eq!(store.list_inventory_cards().unwrap(), vec![card.clone()]);
    assert_eq!(
        store.list_promotion_decisions().unwrap(),
        vec![decision.clone()]
    );
    assert_eq!(
        store.list_incident_records().unwrap(),
        vec![incident.clone()]
    );

    let checkpoint = store.rebuild_work_order_projection_from_events().unwrap();
    assert_eq!(checkpoint.rebuilt_work_orders, 0);
    assert_eq!(store.list_voice_sessions().unwrap(), vec![session]);
    assert_eq!(store.list_attention_packets().unwrap(), vec![packet]);
    assert_eq!(store.list_memory_records().unwrap(), vec![memory]);
    assert_eq!(store.list_inventory_cards().unwrap(), vec![card]);
    assert_eq!(store.list_promotion_decisions().unwrap(), vec![decision]);
    assert_eq!(store.list_incident_records().unwrap(), vec![incident]);
}
