//! ZYAL superworkflow submission path for the Jekko adapter.
//!
//! A `zyal.run` work order carries a full ZYAL superworkflow manifest in its
//! payload. [`ZyalAdapter`] materializes that manifest, submits it to the jekko
//! ZYAL engine via the `jekko port-run` CLI (no HTTP submit route exists yet),
//! polls the run to completion, and surfaces its % progress + state back as
//! [`Evidence`]. The CLI surface is abstracted behind [`ZyalRunner`] so the
//! adapter's submit/poll/mapping logic is tested deterministically without a
//! live jekko. Failures are fail-closed: a missing manifest, a failed submit, or
//! an unreachable engine returns an error, never a silent empty result.

use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use serde::Deserialize;

#[path = "zyal_runner.rs"]
mod runner;
#[path = "zyal_status.rs"]
mod status;

pub use runner::{CliZyalRunner, SubmitOpts, ZyalRunner};
pub use status::{ZyalPhase, ZyalRunStatus};

/// Poll cadence and overall poll budget for a submitted run.
const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
/// 30 minutes, matching the longest expected ZYAL run.
const DEFAULT_POLL_TIMEOUT_SECS: u64 = 30 * 60;
/// Cap per-phase evidence so a large workflow can't unbound the evidence vec.
const MAX_PHASE_EVIDENCE: usize = 32;

/// JMCP adapter that submits ZYAL superworkflows to jekko and reports progress.
pub struct ZyalAdapter {
    runner: Arc<dyn ZyalRunner>,
    poll_interval: Duration,
}

impl Default for ZyalAdapter {
    fn default() -> Self {
        Self::new(Arc::new(CliZyalRunner::from_env()))
    }
}

impl ZyalAdapter {
    /// Build over an arbitrary [`ZyalRunner`] (tests inject a stub).
    pub fn new(runner: Arc<dyn ZyalRunner>) -> Self {
        Self {
            runner,
            poll_interval: Duration::from_secs(DEFAULT_POLL_INTERVAL_SECS),
        }
    }

    /// Shrink the poll interval (tests).
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }
}

/// `zyal.run` payload: the full manifest plus submission/poll options.
#[derive(Debug, Deserialize)]
struct ZyalRunPayload {
    /// The full ZYAL superworkflow manifest (carried inline in the work order).
    manifest: serde_json::Value,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    db: Option<String>,
    #[serde(default)]
    live: bool,
    #[serde(default)]
    max_stages: Option<u32>,
    #[serde(default)]
    time_budget_hours: Option<f64>,
    #[serde(default)]
    per_phase_timeout_secs: Option<u64>,
    #[serde(default)]
    poll_timeout_secs: Option<u64>,
}

#[async_trait]
impl Adapter for ZyalAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "zyal".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/zyal/*".to_owned()],
            capabilities: vec!["zyal.run".to_owned(), "port-run".to_owned()],
        }
    }

    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        if work_order.task.kind != "zyal.run" {
            return Err(fail_closed("zyal"));
        }
        let payload: ZyalRunPayload = serde_json::from_value(work_order.task.payload.clone())
            .context("zyal.run payload must carry a `manifest` object")?;

        // The work order carries the full .zyal; materialize it to a temp file.
        // The path includes the work-order id so concurrent runs never collide.
        let run_id = payload
            .run_id
            .clone()
            .unwrap_or_else(|| format!("wo-{}", work_order.id));
        let manifest_path =
            std::env::temp_dir().join(format!("jmcp-zyal-{}-{}.json", run_id, work_order.id));
        let manifest_bytes =
            serde_json::to_vec_pretty(&payload.manifest).context("serialize zyal manifest")?;
        std::fs::write(&manifest_path, manifest_bytes)
            .with_context(|| format!("write zyal manifest to {}", manifest_path.display()))?;

        let opts = SubmitOpts {
            live: payload.live,
            max_stages: payload.max_stages,
            time_budget_hours: payload.time_budget_hours,
            per_phase_timeout_secs: payload.per_phase_timeout_secs,
        };
        let db = payload.db.as_deref();

        if let Err(err) = self.runner.submit(&manifest_path, db, &run_id, &opts).await {
            let _ = std::fs::remove_file(&manifest_path);
            return Err(err).with_context(|| format!("submit zyal run {run_id}"));
        }

        // Poll to terminal or until the poll budget elapses.
        let timeout = Duration::from_secs(
            payload
                .poll_timeout_secs
                .unwrap_or(DEFAULT_POLL_TIMEOUT_SECS),
        );
        let start = Instant::now();
        let status = loop {
            let snapshot = match self.runner.status(db, &run_id).await {
                Ok(s) => s,
                Err(err) => {
                    let _ = std::fs::remove_file(&manifest_path);
                    return Err(err).with_context(|| format!("poll zyal run {run_id}"));
                }
            };
            if snapshot.is_terminal() || start.elapsed() >= timeout {
                break snapshot;
            }
            tokio::time::sleep(self.poll_interval).await;
        };

        let _ = std::fs::remove_file(&manifest_path);
        Ok(evidence_for(&run_id, &status))
    }
}

/// Map a run status into Evidence: a run reference, a final progress percent, a
/// coarse state label, and (capped) per-phase rows.
fn evidence_for(run_id: &str, status: &ZyalRunStatus) -> Vec<Evidence> {
    let now = Utc::now();
    let mut evidence = vec![
        Evidence {
            kind: "zyal.run".to_owned(),
            uri: format!("zyal://run/{run_id}"),
            captured_at: now,
        },
        Evidence {
            kind: "zyal.progress".to_owned(),
            uri: format!("zyal://run/{run_id}/progress/{}", status.percent()),
            captured_at: now,
        },
        Evidence {
            kind: "zyal.status".to_owned(),
            uri: format!("zyal://run/{run_id}/status/{}", status.state_label()),
            captured_at: now,
        },
    ];
    for phase in status.phases.iter().take(MAX_PHASE_EVIDENCE) {
        evidence.push(Evidence {
            kind: format!("zyal.phase.{}", phase.status),
            uri: format!("zyal://run/{run_id}/phase/{}", phase.phase_id),
            captured_at: now,
        });
    }
    evidence
}

#[cfg(test)]
#[path = "zyal_tests.rs"]
mod zyal_tests;
