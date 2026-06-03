use super::*;
use chrono::TimeZone;

fn base(seq: u64) -> JituxFrameBase {
    JituxFrameBase {
        v: 1,
        session_id: "jitux_test".to_owned(),
        seq,
        frame_id: format!("frame_{seq:04}"),
        emitted_at: Utc.with_ymd_and_hms(2026, 6, 3, 12, 0, seq as u32).unwrap(),
        source: JituxFrameSource::Projection,
        ttl_ms: Some(30_000),
    }
}

fn pane(id: &str, status: PaneStatus, lod: CardLod) -> PaneVm {
    PaneVm {
        id: id.to_owned(),
        kind: PaneKind::Queue,
        title: "Queue blocker".to_owned(),
        rank: 0.97,
        risk: PaneRisk::High,
        status,
        lod,
        confidence: 0.86,
        freshness_ms: Some(250),
        preview: PanePreview {
            headline: "Approval expiry is blocking the next lease".to_owned(),
            chips: vec!["approval".to_owned(), "lease".to_owned()],
            counters: vec![PaneCounter {
                label: "blocked".to_owned(),
                value: CounterValue::Number(3),
            }],
        },
        prepared_tabs: vec![PreparedTab::Evidence, PreparedTab::Actions],
    }
}

fn rank_reason() -> DeckRankReason {
    DeckRankReason {
        score: 7.5,
        factors: DeckRankFactors {
            risk: 1.0,
            blockedness: 1.0,
            approval_expiry_pressure: 0.9,
            lease_pressure: 0.8,
            adapter_degraded_weight: 0.5,
            evidence_gap_weight: 0.7,
            user_query_relevance: 1.0,
            freshness: 0.8,
            downstream_blast_radius: 0.8,
        },
        explanation: "Blocked queue item with expiring approval and aging lease.".to_owned(),
    }
}

fn round_trip(frame: JituxFrame, expected_type: &str) {
    let value = serde_json::to_value(&frame).unwrap();
    assert_eq!(value.get("type").unwrap(), expected_type);
    assert_eq!(value.get("sessionId").unwrap(), "jitux_test");
    assert!(value.get("frameId").is_some());
    assert!(value.get("emittedAt").is_some());

    let decoded: JituxFrame = serde_json::from_value(value).unwrap();
    assert_eq!(decoded, frame);
}

#[test]
fn jitux_frame_variants_round_trip() {
    let pane = pane("pane:queue", PaneStatus::Predicted, CardLod::Ghost);
    let reason = rank_reason();
    let action = PreparedAction {
        id: "show_evidence".to_owned(),
        label: "Show evidence".to_owned(),
        command: "jitux.evidence.preview".to_owned(),
        safety: ActionSafetyClass::ReadOnly,
        ready: true,
        requires_approval: false,
        reason: "Read-only evidence preview.".to_owned(),
        preview_ref: Some("jitux://payload/pane:queue/evidence".to_owned()),
    };
    let evidence = JituxEvidenceRef {
        id: "evidence:queue".to_owned(),
        label: "Queue projection".to_owned(),
        uri: "jmcp://evidence/queue".to_owned(),
        captured_at: Utc.with_ymd_and_hms(2026, 6, 3, 12, 0, 0).unwrap(),
    };

    round_trip(
        JituxFrame::DeckPatch {
            base: base(1),
            deck: DeckPatch {
                title: "Scanning queue blockers".to_owned(),
                active: true,
                mode: DeckMode::MissionDeck,
            },
        },
        "deck.patch",
    );
    round_trip(
        JituxFrame::PanePrepare {
            base: base(2),
            pane: pane.clone(),
            reason: "Queue blockers are relevant to the prompt.".to_owned(),
        },
        "pane.prepare",
    );
    round_trip(
        JituxFrame::PaneUpsert {
            base: base(3),
            pane: pane.clone(),
        },
        "pane.upsert",
    );
    round_trip(
        JituxFrame::PaneCommit {
            base: base(4),
            pane_id: pane.id.clone(),
        },
        "pane.commit",
    );
    round_trip(
        JituxFrame::FocusChange {
            base: base(5),
            pane_id: pane.id.clone(),
            reason: reason.clone(),
        },
        "focus.change",
    );
    round_trip(
        JituxFrame::DeckRankChanged {
            base: base(6),
            ordered_pane_ids: vec![pane.id.clone()],
            reasons: vec![PaneRankReason {
                pane_id: pane.id.clone(),
                reason: reason.clone(),
            }],
        },
        "deck.rank.changed",
    );
    round_trip(
        JituxFrame::CardGhost {
            base: base(7),
            pane: pane.clone(),
        },
        "card.ghost",
    );
    round_trip(
        JituxFrame::CardCommit {
            base: base(8),
            pane_id: pane.id.clone(),
        },
        "card.commit",
    );
    round_trip(
        JituxFrame::CardHydrated {
            base: base(9),
            pane_id: pane.id.clone(),
            prepared_tabs: vec![PreparedTab::Evidence, PreparedTab::Actions],
        },
        "card.hydrated",
    );
    round_trip(
        JituxFrame::EvidenceAttach {
            base: base(10),
            pane_id: pane.id.clone(),
            evidence: vec![evidence],
            freshness_ms: Some(100),
            confidence: Some(0.92),
        },
        "evidence.attach",
    );
    round_trip(
        JituxFrame::ActionReady {
            base: base(11),
            pane_id: pane.id.clone(),
            action,
        },
        "action.ready",
    );
    round_trip(
        JituxFrame::SessionDone {
            base: base(12),
            summary: "First projection pass complete.".to_owned(),
        },
        "session.done",
    );
    round_trip(
        JituxFrame::SessionError {
            base: base(13),
            error: JituxSessionError {
                code: "adapter_timeout".to_owned(),
                message: "Jeryu probe timed out.".to_owned(),
                pane_id: Some("pane:jeryu".to_owned()),
            },
        },
        "session.error",
    );
}

#[test]
fn jitux_action_rejects_obvious_secret_material() {
    let action = PreparedAction {
        id: "bad_action".to_owned(),
        label: "Bad action".to_owned(),
        command: "curl -H 'Authorization: Bearer token'".to_owned(),
        safety: ActionSafetyClass::ManualOnly,
        ready: false,
        requires_approval: true,
        reason: "Do not expose secrets.".to_owned(),
        preview_ref: None,
    };

    assert_eq!(
        action.validate_no_secret_material(),
        Err(JituxValidationError::SecretMaterial)
    );
}

#[test]
fn jitux_rank_reasons_are_user_visible() {
    let reason = rank_reason();

    assert!(reason.score > 0.0);
    assert!(reason.factors.blockedness > 0.0);
    assert!(reason.factors.approval_expiry_pressure > 0.0);
    assert!(reason.explanation.contains("Blocked queue item"));
}
