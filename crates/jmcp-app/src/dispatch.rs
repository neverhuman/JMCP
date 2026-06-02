//! Microtask dispatcher — the execution half of the autonomous loop.
//!
//! Generation (the `microtasks` registry) submits signed, evidence-only work
//! orders that then sit in [`WorkOrderStatus::Submitted`]. This module is what
//! finally *runs* them: it selects the microtask work orders that are safe to
//! auto-execute (R0 — microtask-tagged, evidence-only, non-live), leases each
//! one, hands it to a [`MicrotaskExecutor`] (the real Jankurai/Jekko adapters
//! in `jmcpd`, a stub in tests) under a validated lease, and records the
//! returned Evidence -> `Completed`, or fails closed -> `Failed`.
//!
//! It deliberately ignores any work order that is NOT a microtask (e.g. a user
//! `/submit` parked awaiting Telegram approval, or a `live=true` request) so the
//! dispatcher can never auto-run something meant for a human gate.

use async_trait::async_trait;
use chrono::Duration;
use serde_json::Value;
use uuid::Uuid;

use jmcp_domain::{Evidence, Lease, MicrotaskOverrides, WorkOrder, WorkOrderStatus};

use crate::{AppError, AppResult, AppState};

/// Runs a leased microtask work order against the adapter its `task.kind`
/// routes to. Implemented in `jmcpd` over the real adapters (lease enforced via
/// `jmcp_adapter_sdk::execute_with_lease`); stubbed in tests.
#[async_trait]
pub trait MicrotaskExecutor: Send + Sync {
    /// The lease holder (adapter name) for `kind`, or `None` when no adapter
    /// owns it — the work order is then left untouched, not failed.
    fn holder_for(&self, kind: &str) -> Option<&'static str>;

    /// Execute `work_order` under `lease`/`holder`. Implementations MUST
    /// validate the lease before any side effect.
    async fn execute(
        &self,
        work_order: &WorkOrder,
        lease: &Lease,
        holder: &str,
    ) -> anyhow::Result<Vec<Evidence>>;
}

/// Tally of one dispatch sweep.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchReport {
    /// Leased, executed, and marked `Completed` with non-empty Evidence.
    pub completed: usize,
    /// Leased but failed closed (no Evidence, adapter error, or no route after claim).
    pub failed: usize,
    /// Auto-dispatchable but no adapter owns the kind — left `Submitted`.
    pub skipped: usize,
}

/// True when the work order was minted by the microtask planner
/// (`payload.metadata.microtask == true`), vs a user submission or a coarser action.
fn is_microtask(work_order: &WorkOrder) -> bool {
    work_order
        .task
        .payload
        .pointer("/metadata/microtask")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

/// The R0 auto-dispatch gate: a `Submitted` microtask that is evidence-only and
/// non-live. Everything else is left for the approval path and never auto-run.
fn is_auto_dispatchable(work_order: &WorkOrder) -> bool {
    if work_order.status != WorkOrderStatus::Submitted || !is_microtask(work_order) {
        return false;
    }
    let payload = &work_order.task.payload;
    let evidence_oriented = payload
        .get("evidence_oriented")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let live = payload
        .get("live")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    evidence_oriented && !live
}

impl AppState {
    /// All microtask work orders, any status — the queue view for `/microtasks/queue`.
    pub fn list_microtask_work_orders(&self) -> AppResult<Vec<WorkOrder>> {
        Ok(self
            .list_work_orders()?
            .into_iter()
            .filter(is_microtask)
            .collect())
    }

    /// `Submitted` microtasks that pass the R0 auto-dispatch gate.
    pub fn list_auto_dispatchable_microtasks(&self) -> AppResult<Vec<WorkOrder>> {
        Ok(self
            .list_work_orders()?
            .into_iter()
            .filter(is_auto_dispatchable)
            .collect())
    }

    /// Lease a `Submitted` work order to `holder` and persist the transition.
    pub fn claim_work_order(
        &self,
        id: Uuid,
        holder: &str,
        ttl: Duration,
    ) -> AppResult<(WorkOrder, Lease)> {
        let store = self.store.lock().expect("store lock");
        let mut work_order = store
            .get_work_order(id)?
            .ok_or_else(|| AppError::State(format!("work order not found: {id}")))?;
        let lease = work_order.lease(holder, ttl)?;
        store.record_lease(&lease)?;
        store.append_work_order("microtask.leased", &work_order)?;
        Ok((work_order, lease))
    }

    /// Attach `evidence` and mark the leased work order `Completed` (persisted).
    pub fn complete_work_order(
        &self,
        mut work_order: WorkOrder,
        evidence: Vec<Evidence>,
    ) -> AppResult<WorkOrder> {
        for item in evidence {
            work_order.evidence.push(item);
        }
        work_order.complete()?;
        self.store
            .lock()
            .expect("store lock")
            .append_work_order("microtask.completed", &work_order)?;
        Ok(work_order)
    }

    /// Mark the work order `Failed` with `reason` (persisted) — the fail-closed path.
    pub fn fail_work_order(&self, mut work_order: WorkOrder, reason: &str) -> AppResult<WorkOrder> {
        work_order.fail(reason);
        self.store
            .lock()
            .expect("store lock")
            .append_work_order("microtask.failed", &work_order)?;
        Ok(work_order)
    }

    /// Run one dispatch sweep over every auto-dispatchable microtask: lease,
    /// execute, and record. The store lock is never held across `.await`.
    pub async fn dispatch_microtasks_once(
        &self,
        executor: &dyn MicrotaskExecutor,
        ttl: Duration,
    ) -> AppResult<DispatchReport> {
        let mut report = DispatchReport::default();
        for work_order in self.list_auto_dispatchable_microtasks()? {
            let Some(holder) = executor.holder_for(&work_order.task.kind) else {
                report.skipped += 1;
                continue;
            };
            let (leased, lease) = self.claim_work_order(work_order.id, holder, ttl)?;
            match executor.execute(&leased, &lease, holder).await {
                Ok(evidence) if evidence.is_empty() => {
                    self.fail_work_order(leased, "no_evidence_produced")?;
                    report.failed += 1;
                }
                Ok(evidence) => {
                    self.complete_work_order(leased, evidence)?;
                    report.completed += 1;
                }
                Err(error) => {
                    self.fail_work_order(leased, &error.to_string())?;
                    report.failed += 1;
                }
            }
        }
        Ok(report)
    }

    /// Submit microtask `id` unless an equivalent one is already in flight (same
    /// `microtask_id` and `repo`, non-terminal). Returns `None` when skipped —
    /// the dedup guard that lets an autonomous generator tick run idempotently.
    pub fn generate_microtask_if_absent(
        &self,
        id: &str,
        overrides: MicrotaskOverrides,
    ) -> AppResult<Option<WorkOrder>> {
        let repo = overrides.repo.clone();
        if self.has_open_microtask(id, repo.as_deref())? {
            return Ok(None);
        }
        Ok(Some(self.submit_microtask(id, overrides)?))
    }

    fn has_open_microtask(&self, microtask_id: &str, repo: Option<&str>) -> AppResult<bool> {
        Ok(self.list_microtask_work_orders()?.iter().any(|work_order| {
            let open = !matches!(
                work_order.status,
                WorkOrderStatus::Completed | WorkOrderStatus::Failed | WorkOrderStatus::Cancelled
            );
            let id_match = work_order
                .task
                .payload
                .pointer("/metadata/microtask_id")
                .and_then(Value::as_str)
                == Some(microtask_id);
            let repo_match = match repo {
                Some(repo) => {
                    work_order.task.payload.get("cwd").and_then(Value::as_str) == Some(repo)
                }
                None => true,
            };
            open && id_match && repo_match
        }))
    }
}
