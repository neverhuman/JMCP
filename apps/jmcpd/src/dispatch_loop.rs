//! The microtask dispatch loop — an opt-in background task that turns queued,
//! evidence-only microtasks into executed ones.
//!
//! It mirrors `telegram_poll_loop`: a single `tokio::spawn`ed loop, off by
//! default, that on each tick optionally generates due microtasks (with a dedup
//! guard) and then sweeps every auto-dispatchable microtask through the real
//! Jankurai/Jekko adapters under a validated lease. All execution flows through
//! [`jmcp_app::AppState::dispatch_microtasks_once`], which only ever touches
//! microtask-tagged, non-live, evidence-only work orders — anything awaiting a
//! human gate is left alone.

use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Duration as ChronoDuration;
use jmcp_adapter_jankurai::JankuraiAdapter;
use jmcp_adapter_jekko::JekkoAdapter;
use jmcp_adapter_sdk::execute_with_lease;
use jmcp_app::{AppState, MicrotaskExecutor};
use jmcp_domain::{Evidence, Lease, MicrotaskOverrides, WorkOrder};
use serde_json::json;

use crate::telegram_helpers::emit_structured_event;

/// The microtask kind generated on each tick when generation is enabled.
const REPO_AUDIT_MICROTASK: &str = "jankurai.repo-refresh-audit";

/// Runtime configuration for [`dispatcher_loop`].
pub struct DispatcherConfig {
    /// Sleep between sweeps.
    pub poll: Duration,
    /// Lease TTL handed to each adapter run.
    pub lease_ttl: ChronoDuration,
    /// When true, enqueue a repo-refresh audit microtask per `repos` each tick.
    pub generate: bool,
    /// Repositories the generator audits (empty = generate nothing).
    pub repos: Vec<String>,
}

/// Routes each microtask `task.kind` to the real adapter that owns it and runs
/// it under a lease-enforced [`execute_with_lease`].
pub struct RealMicrotaskExecutor {
    jankurai: JankuraiAdapter,
    jekko: JekkoAdapter,
}

impl RealMicrotaskExecutor {
    /// Build both adapters from the environment (`JMCP_JANKURAI_BIN`, `JEKKO_*`).
    pub fn from_env() -> Self {
        Self {
            jankurai: JankuraiAdapter::default(),
            jekko: JekkoAdapter::default(),
        }
    }
}

#[async_trait]
impl MicrotaskExecutor for RealMicrotaskExecutor {
    fn holder_for(&self, kind: &str) -> Option<&'static str> {
        match kind {
            "jankurai.proof" | "jankurai.diff-audit" | "jankurai.doctor" => Some("jankurai"),
            "reason" | "jekko.run" | "jekko.task" | "run" | "worker" => Some("jekko"),
            _ => None,
        }
    }

    async fn execute(
        &self,
        work_order: &WorkOrder,
        lease: &Lease,
        holder: &str,
    ) -> Result<Vec<Evidence>> {
        match holder {
            "jankurai" => execute_with_lease(&self.jankurai, work_order, lease, holder).await,
            "jekko" => execute_with_lease(&self.jekko, work_order, lease, holder).await,
            other => anyhow::bail!("no adapter wired for holder {other}"),
        }
    }
}

/// Run the dispatch loop until the process exits.
pub async fn dispatcher_loop(state: AppState, config: DispatcherConfig) -> Result<()> {
    let executor = RealMicrotaskExecutor::from_env();
    emit_structured_event(
        "info",
        "autopilot.dispatcher.started",
        json!({
            "pollSeconds": config.poll.as_secs(),
            "generate": config.generate,
            "repos": config.repos,
        }),
    );
    loop {
        if config.generate {
            generate_due(&state, &config.repos);
        }
        match state
            .dispatch_microtasks_once(&executor, config.lease_ttl)
            .await
        {
            Ok(report) if report.completed + report.failed + report.skipped > 0 => {
                emit_structured_event(
                    "info",
                    "autopilot.dispatch.swept",
                    json!({
                        "completed": report.completed,
                        "failed": report.failed,
                        "skipped": report.skipped,
                    }),
                );
            }
            Ok(_) => {}
            Err(err) => emit_structured_event(
                "error",
                "autopilot.dispatch.failed",
                json!({ "error": err.to_string() }),
            ),
        }
        tokio::time::sleep(config.poll).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holder_routing_covers_every_microtask_work_order_kind() {
        let executor = RealMicrotaskExecutor::from_env();
        // The three work-order kinds the microtask registry emits today.
        assert_eq!(executor.holder_for("jankurai.proof"), Some("jankurai"));
        assert_eq!(executor.holder_for("jankurai.diff-audit"), Some("jankurai"));
        assert_eq!(executor.holder_for("reason"), Some("jekko"));
        // Unknown kinds are unroutable, so the dispatcher leaves them Submitted.
        assert_eq!(executor.holder_for("zyal.run"), None);
        assert_eq!(executor.holder_for("totally.unknown"), None);
    }
}

/// Enqueue a deduped repo-refresh audit microtask for each configured repo.
fn generate_due(state: &AppState, repos: &[String]) {
    for repo in repos {
        let overrides = MicrotaskOverrides {
            repo: Some(repo.clone()),
            ..MicrotaskOverrides::default()
        };
        match state.generate_microtask_if_absent(REPO_AUDIT_MICROTASK, overrides) {
            Ok(Some(work_order)) => emit_structured_event(
                "info",
                "autopilot.microtask.generated",
                json!({ "workOrderId": work_order.id, "repo": repo, "kind": REPO_AUDIT_MICROTASK }),
            ),
            Ok(None) => {}
            Err(err) => emit_structured_event(
                "warn",
                "autopilot.microtask.generate_failed",
                json!({ "repo": repo, "error": err.to_string() }),
            ),
        }
    }
}
