use super::*;
use chrono::{Duration, Utc};
use jmcp_domain::{
    Approval, ApprovalChallenge, ApprovalChallengeState, ApprovalChannel, Evidence, Lease,
    WorkOrder,
};
use serde_json::json;

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
