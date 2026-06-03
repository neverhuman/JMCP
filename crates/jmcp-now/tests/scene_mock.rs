mod mock_support;

use jmcp_domain::{AttentionLevel, CardLod, CounterValue, PaneKind, PanePreview, PaneRisk, PaneStatus, PreparedTab, WorkOrderStatus};
use jmcp_now::{queue_blockers_panes, queue_blockers_projection, CachedNow, NowProjection, NowReads};
use mock_support::{
    approval_challenge, attention_packet, fixed_time, incident_record, lease, uuid, work_order,
    work_order_with_evidence,
};

fn scene_reads() -> NowReads {
    let focused = uuid("22222222-2222-4222-8222-222222222222");
    let leased = uuid("22222222-2222-4222-8222-222222222223");
    let failed = uuid("22222222-2222-4222-8222-222222222224");
    let submitted = uuid("22222222-2222-4222-8222-222222222225");

    NowReads {
        work_orders: vec![
            work_order_with_evidence(focused, "Bridge write lease", WorkOrderStatus::AwaitingApproval),
            work_order(leased, "Lease-only blocker", WorkOrderStatus::Leased),
            work_order(failed, "Manual recovery", WorkOrderStatus::Failed),
            work_order(submitted, "Submitted blocker", WorkOrderStatus::Submitted),
            work_order(uuid("22222222-2222-4222-8222-222222222226"), "Completed", WorkOrderStatus::Completed),
        ],
        leases: vec![lease(focused, "jmcpd", 10), lease(leased, "jmcpd", 4)],
        attention_packets: vec![attention_packet(Some(focused), "Attention owns the why-now headline.", AttentionLevel::Page)],
        approval_challenges: vec![approval_challenge(focused, 20)],
        incidents: vec![
            incident_record(focused, jmcp_domain::IncidentSeverity::Major, "Bridge write lease remains quarantined"),
            incident_record(submitted, jmcp_domain::IncidentSeverity::Critical, "Submitted blocker is high blast radius"),
        ],
        autonomous_actions: jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap())
            .list_autonomous_actions()
            .unwrap(),
    }
}

#[test]
fn empty_reads_yield_an_empty_scene_and_cached_projection() {
    let reads = NowReads::default();
    let panes = queue_blockers_panes(&reads, fixed_time());
    let projection = queue_blockers_projection(&reads, fixed_time());
    let cached = CachedNow::build(0, fixed_time(), reads);

    assert!(panes.is_empty());
    assert!(projection.panes.is_empty());
    assert!(projection.rank_reasons.is_empty());
    assert!(projection.prepared_actions.is_empty());
    assert!(projection.evidence_refs.is_empty());
    assert_eq!(cached.default_pane, jmcp_now::scenes::queue_blockers::KEY);
    assert!(cached.panes.is_empty());
}

#[test]
fn mixed_reads_render_canonical_pane_fields_and_ordered_sidecars() {
    let projection = queue_blockers_projection(&scene_reads(), fixed_time());
    let panes = &projection.panes;
    assert_eq!(panes.len(), 4);

    let focused = panes.iter().find(|pane| pane.title == "Bridge write lease").expect("focused pane");
    assert_eq!(focused.kind, PaneKind::Queue);
    assert_eq!(focused.status, PaneStatus::Active);
    assert_eq!(focused.lod, CardLod::Focus);
    assert_eq!(focused.risk, PaneRisk::High);
    assert!(focused.preview.headline.contains("Attention owns the why-now headline."));
    assert!(focused.preview.chips.contains(&"awaiting_approval".to_owned()));
    assert!(focused.preview.chips.contains(&"lease".to_owned()));
    assert!(focused.preview.chips.contains(&"approval_required".to_owned()));
    assert!(focused.preview.chips.contains(&"evidence".to_owned()));
    assert!(focused.preview.chips.iter().any(|chip| chip.starts_with("incident_")));
    assert_eq!(
        focused.preview.counters,
        vec![
            jmcp_domain::PaneCounter { label: "evidence".to_owned(), value: CounterValue::Number(1) },
            jmcp_domain::PaneCounter { label: "actions".to_owned(), value: CounterValue::Number(3) },
            jmcp_domain::PaneCounter { label: "status".to_owned(), value: CounterValue::Text("awaiting_approval".to_owned()) },
            jmcp_domain::PaneCounter { label: "leaseMins".to_owned(), value: CounterValue::Number(10) },
            jmcp_domain::PaneCounter { label: "approvalMins".to_owned(), value: CounterValue::Number(20) },
        ],
    );
    assert_eq!(focused.prepared_tabs, vec![PreparedTab::Evidence, PreparedTab::Actions, PreparedTab::Systems]);

    let leased = panes.iter().find(|pane| pane.title == "Lease-only blocker").expect("leased pane");
    assert_eq!(leased.status, PaneStatus::Warm);
    assert_eq!(leased.lod, CardLod::Preview);
    assert!(leased.preview.headline.contains("Lease held by jmcpd needs completion or renewal."));
    assert!(leased.preview.chips.contains(&"lease".to_owned()));
    assert!(leased.preview.chips.contains(&"bounded_auto".to_owned()));
    assert!(leased.preview.chips.contains(&"evidence_gap".to_owned()));
    assert!(leased.prepared_tabs.contains(&PreparedTab::Systems));
    assert!(leased.prepared_tabs.contains(&PreparedTab::Actions));

    let submitted = panes.iter().find(|pane| pane.title == "Submitted blocker").expect("submitted pane");
    assert_eq!(submitted.status, PaneStatus::Incubating);
    assert_eq!(submitted.lod, CardLod::Preview);
    assert!(submitted.preview.headline.contains("Submitted work order is waiting for a lease or approval path."));
    assert!(submitted.preview.chips.contains(&"bounded_auto".to_owned()));
    assert!(submitted.preview.chips.contains(&"incident_critical".to_owned()));

    let rank_ids = projection.rank_reasons.iter().map(|reason| reason.pane_id.as_str()).collect::<Vec<_>>();
    let pane_ids = panes.iter().map(|pane| pane.id.as_str()).collect::<Vec<_>>();
    assert_eq!(pane_ids, rank_ids);
}

#[test]
fn failed_work_order_without_any_automation_path_is_manual_only() {
    let failed_id = uuid("22222222-2222-4222-8222-222222222227");
    let projection = queue_blockers_projection(
        &NowReads {
            work_orders: vec![work_order(failed_id, "Needs manual recovery", WorkOrderStatus::Failed)],
            autonomous_actions: Vec::new(),
            ..NowReads::default()
        },
        fixed_time(),
    );
    let pane = &projection.panes[0];
    let actions = projection.prepared_actions.get(&pane.id).expect("prepared actions");

    assert!(pane.preview.chips.contains(&"manual_only".to_owned()));
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].safety, jmcp_domain::ActionSafetyClass::ManualOnly);
}

#[test]
fn now_projection_refreshes_on_watermark_change_and_loads_without_changing_snapshot() {
    let state = jmcp_app::AppState::new(jmcp_store::SqliteStore::in_memory().unwrap());
    let initial = CachedNow::build(0, fixed_time(), NowReads::default());
    let projection = NowProjection::new(state.clone(), initial);

    let first = projection.load();
    let second = projection.load();
    assert!(std::sync::Arc::ptr_eq(&first, &second));
    assert_eq!(first.generation, 0);

    state.record_attention_packet(&jmcp_app::attention_inbox_sample()[0]).unwrap();
    let refreshed = projection.refresh_if_stale_at(fixed_time()).unwrap();
    let loaded = projection.load();

    assert_eq!(refreshed.generation, 1);
    assert_eq!(loaded.generation, 1);
    assert_eq!(refreshed.captured_at, fixed_time());
    assert_eq!(refreshed.default_pane, jmcp_now::scenes::queue_blockers::KEY);
}
