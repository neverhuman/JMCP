use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use jcp_core::{Envelope, LocalSigner, Subject};
use jmcp_domain::{Evidence, Lease, MicrotaskOverrides, WorkOrder, WorkOrderStatus};
use jmcp_store::SqliteStore;
use serde_json::json;

use crate::{AppState, DispatchReport, MicrotaskExecutor};

enum Outcome {
    Evidence,
    Empty,
    Error,
}

/// A test [`MicrotaskExecutor`] that mirrors the production contract: it refuses
/// to act unless handed a lease valid for the work order + holder, then returns
/// a canned outcome.
struct StubExecutor {
    holder: Option<&'static str>,
    outcome: Outcome,
    saw_valid_lease: Arc<AtomicBool>,
}

impl StubExecutor {
    fn new(holder: Option<&'static str>, outcome: Outcome) -> Self {
        Self {
            holder,
            outcome,
            saw_valid_lease: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl MicrotaskExecutor for StubExecutor {
    fn holder_for(&self, _kind: &str) -> Option<&'static str> {
        self.holder
    }

    async fn execute(
        &self,
        work_order: &WorkOrder,
        lease: &Lease,
        holder: &str,
    ) -> anyhow::Result<Vec<Evidence>> {
        lease
            .validate_for(work_order.id, holder)
            .map_err(|err| anyhow::anyhow!("invalid lease: {err:?}"))?;
        self.saw_valid_lease.store(true, Ordering::SeqCst);
        match self.outcome {
            Outcome::Evidence => Ok(vec![Evidence {
                kind: "stub.digest".to_owned(),
                uri: "sha256:deadbeef".to_owned(),
                captured_at: chrono::Utc::now(),
            }]),
            Outcome::Empty => Ok(Vec::new()),
            Outcome::Error => anyhow::bail!("stub adapter failure"),
        }
    }
}

fn microtask_state(id: &str) -> (AppState, WorkOrder) {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let overrides = MicrotaskOverrides {
        repo: Some(".".to_owned()),
        ..MicrotaskOverrides::default()
    };
    let work_order = state.submit_microtask(id, overrides).unwrap();
    (state, work_order)
}

#[tokio::test]
async fn dispatch_completes_microtask_and_attaches_evidence() {
    let (state, work_order) = microtask_state("jankurai.repo-refresh-audit");
    let executor = StubExecutor::new(Some("jankurai"), Outcome::Evidence);
    let saw_lease = executor.saw_valid_lease.clone();

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(
        report,
        DispatchReport {
            completed: 1,
            failed: 0,
            skipped: 0
        }
    );
    assert!(
        saw_lease.load(Ordering::SeqCst),
        "executor must run only under a valid lease"
    );
    let stored = state.work_order(work_order.id).unwrap().unwrap();
    assert_eq!(stored.status, WorkOrderStatus::Completed);
    assert_eq!(stored.evidence.len(), 1);
    assert_eq!(stored.evidence[0].kind, "stub.digest");
}

#[tokio::test]
async fn dispatch_fails_closed_on_empty_evidence() {
    let (state, work_order) = microtask_state("jankurai.repo-refresh-audit");
    let executor = StubExecutor::new(Some("jankurai"), Outcome::Empty);

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(report.failed, 1);
    let stored = state.work_order(work_order.id).unwrap().unwrap();
    assert_eq!(stored.status, WorkOrderStatus::Failed);
    assert!(stored
        .attention
        .iter()
        .any(|attention| attention.reason.contains("no_evidence_produced")));
}

#[tokio::test]
async fn dispatch_fails_closed_on_executor_error() {
    let (state, work_order) = microtask_state("jankurai.repo-refresh-audit");
    let executor = StubExecutor::new(Some("jankurai"), Outcome::Error);

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(report.failed, 1);
    assert_eq!(
        state.work_order(work_order.id).unwrap().unwrap().status,
        WorkOrderStatus::Failed
    );
}

#[tokio::test]
async fn dispatch_skips_unroutable_kind_and_leaves_it_submitted() {
    let (state, work_order) = microtask_state("research.concept-scan");
    let executor = StubExecutor::new(None, Outcome::Evidence);

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(report.skipped, 1);
    assert_eq!(report.completed, 0);
    assert_eq!(
        state.work_order(work_order.id).unwrap().unwrap().status,
        WorkOrderStatus::Submitted,
        "an unroutable kind is left for a later run, not failed"
    );
}

#[tokio::test]
async fn dispatch_ignores_non_microtask_submitted_work_order() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let signer = LocalSigner::load_or_create_default().unwrap();
    // R0-looking, but NOT minted by the microtask planner.
    let envelope = signer.sign(Envelope::new(
        Subject::from_str("tenant/service/entity").unwrap(),
        "reason",
        json!({ "evidence_oriented": true, "live": false }),
    ));
    let work_order = state.submit_envelope(envelope).unwrap();
    let executor = StubExecutor::new(Some("jekko"), Outcome::Evidence);

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(report, DispatchReport::default());
    assert_eq!(
        state.work_order(work_order.id).unwrap().unwrap().status,
        WorkOrderStatus::Submitted
    );
}

#[tokio::test]
async fn dispatch_skips_live_microtask() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let signer = LocalSigner::load_or_create_default().unwrap();
    let envelope = signer.sign(Envelope::new(
        Subject::from_str("jmcp/jankurai/live").unwrap(),
        "jankurai.proof",
        json!({ "metadata": { "microtask": true }, "evidence_oriented": true, "live": true }),
    ));
    let work_order = state.submit_envelope(envelope).unwrap();
    let executor = StubExecutor::new(Some("jankurai"), Outcome::Evidence);

    let report = state
        .dispatch_microtasks_once(&executor, Duration::minutes(5))
        .await
        .unwrap();

    assert_eq!(report, DispatchReport::default());
    assert_eq!(
        state.work_order(work_order.id).unwrap().unwrap().status,
        WorkOrderStatus::Submitted,
        "live microtasks require the approval path, never auto-dispatch"
    );
}

#[test]
fn generate_microtask_if_absent_dedups_open_microtask_per_repo() {
    let state = AppState::new(SqliteStore::in_memory().unwrap());
    let here = MicrotaskOverrides {
        repo: Some(".".to_owned()),
        ..MicrotaskOverrides::default()
    };

    let first = state
        .generate_microtask_if_absent("jankurai.repo-refresh-audit", here.clone())
        .unwrap();
    assert!(first.is_some(), "first generation enqueues");

    let second = state
        .generate_microtask_if_absent("jankurai.repo-refresh-audit", here)
        .unwrap();
    assert!(
        second.is_none(),
        "an already-open microtask for the same repo is not re-enqueued"
    );

    let other_repo = MicrotaskOverrides {
        repo: Some("other".to_owned()),
        ..MicrotaskOverrides::default()
    };
    let third = state
        .generate_microtask_if_absent("jankurai.repo-refresh-audit", other_repo)
        .unwrap();
    assert!(third.is_some(), "a different repo is not a duplicate");
}
