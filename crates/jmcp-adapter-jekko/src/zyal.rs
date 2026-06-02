//! ZYAL superworkflow submission path for the Jekko adapter.
//!
//! A `zyal.run` work order carries a full ZYAL superworkflow manifest in its
//! payload. This module materializes that manifest, submits it to the jekko
//! ZYAL engine via the `jekko port-run` CLI (no HTTP submit route exists yet),
//! polls the run to completion, and surfaces its % progress + state back as
//! [`Evidence`]. The CLI surface is abstracted behind [`ZyalRunner`] so the
//! adapter's submit/poll/mapping logic is tested deterministically without a
//! live jekko. Failures are fail-closed: a missing manifest, a failed submit, or
//! an unreachable engine returns an error, never a silent empty result.

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use serde::Deserialize;

/// Default jekko binary, poll cadence, and overall poll budget.
const DEFAULT_JEKKO_BIN: &str = "jekko";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
/// 30 minutes, matching the longest expected ZYAL run.
const DEFAULT_POLL_TIMEOUT_SECS: u64 = 30 * 60;
/// Cap per-phase evidence so a large workflow can't unbound the evidence vec.
const MAX_PHASE_EVIDENCE: usize = 32;

/// Options threaded into `jekko port-run`.
#[derive(Clone, Debug, Default)]
pub struct SubmitOpts {
    /// Pass `--live` to drive real per-phase execution (vs the scaffold walk).
    pub live: bool,
    /// `--max-stages N`: stop after N complete phases.
    pub max_stages: Option<u32>,
    /// `--time-budget-hours H`: wall-clock ceiling.
    pub time_budget_hours: Option<f64>,
    /// `--per-phase-timeout-secs N`.
    pub per_phase_timeout_secs: Option<u64>,
}

/// One phase row from `jekko port-run --status`.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct ZyalPhase {
    #[serde(default)]
    pub phase_id: String,
    pub status: String,
}

/// Parsed `--status` snapshot for a run.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
pub struct ZyalRunStatus {
    #[serde(default)]
    pub run_id: String,
    #[serde(default)]
    pub phases: Vec<ZyalPhase>,
}

impl ZyalRunStatus {
    /// Total phases known for the run.
    pub fn total(&self) -> usize {
        self.phases.len()
    }

    /// Count of phases in the `complete` state.
    pub fn completed(&self) -> usize {
        self.phases
            .iter()
            .filter(|p| p.status == "complete")
            .count()
    }

    /// Percent complete in `0..=100`. An empty run is 0%.
    pub fn percent(&self) -> u8 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        ((self.completed() * 100) / total) as u8
    }

    /// True once every phase has reached a terminal state
    /// (`complete` | `blocked` | `failed`). An empty run is never terminal.
    pub fn is_terminal(&self) -> bool {
        !self.phases.is_empty()
            && self
                .phases
                .iter()
                .all(|p| matches!(p.status.as_str(), "complete" | "blocked" | "failed"))
    }

    /// Coarse run-state label for evidence: `failed` if any phase failed, else
    /// `blocked` if any is blocked, else `complete` if all complete, else
    /// `partial` (and `unknown` for an empty run).
    pub fn state_label(&self) -> &'static str {
        if self.phases.is_empty() {
            return "unknown";
        }
        if self.phases.iter().any(|p| p.status == "failed") {
            return "failed";
        }
        if self.phases.iter().any(|p| p.status == "blocked") {
            return "blocked";
        }
        if self.phases.iter().all(|p| p.status == "complete") {
            return "complete";
        }
        "partial"
    }
}

/// CLI surface of the jekko ZYAL engine, abstracted for testing.
#[async_trait]
pub trait ZyalRunner: Send + Sync {
    /// Start a run from a manifest file under a deterministic `run_id`.
    async fn submit(
        &self,
        manifest_path: &Path,
        db: Option<&str>,
        run_id: &str,
        opts: &SubmitOpts,
    ) -> Result<()>;

    /// Read the current status snapshot for a run.
    async fn status(&self, db: Option<&str>, run_id: &str) -> Result<ZyalRunStatus>;
}

/// Real [`ZyalRunner`] that shells out to the `jekko` binary.
pub struct CliZyalRunner {
    bin: String,
}

impl CliZyalRunner {
    /// Build from `JEKKO_BIN` (default `jekko`).
    pub fn from_env() -> Self {
        let bin = std::env::var("JEKKO_BIN")
            .ok()
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_JEKKO_BIN.to_owned());
        Self { bin }
    }

    /// Build with an explicit binary path (used by tests with a stub).
    pub fn with_bin(bin: impl Into<String>) -> Self {
        Self { bin: bin.into() }
    }
}

#[async_trait]
impl ZyalRunner for CliZyalRunner {
    async fn submit(
        &self,
        manifest_path: &Path,
        db: Option<&str>,
        run_id: &str,
        opts: &SubmitOpts,
    ) -> Result<()> {
        let mut cmd = tokio::process::Command::new(&self.bin);
        cmd.arg("port-run")
            .arg("--super")
            .arg(manifest_path)
            .arg("--run-id")
            .arg(run_id);
        if let Some(db) = db {
            cmd.arg("--db").arg(db);
        }
        if opts.live {
            cmd.arg("--live");
        }
        if let Some(n) = opts.max_stages {
            cmd.arg("--max-stages").arg(n.to_string());
        }
        if let Some(h) = opts.time_budget_hours {
            cmd.arg("--time-budget-hours").arg(h.to_string());
        }
        if let Some(s) = opts.per_phase_timeout_secs {
            cmd.arg("--per-phase-timeout-secs").arg(s.to_string());
        }
        let output = cmd
            .output()
            .await
            .with_context(|| format!("spawn `{} port-run --super`", self.bin))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "jekko port-run submit failed ({:?}): {}",
                output.status.code(),
                if stderr.is_empty() {
                    "<no stderr>".to_owned()
                } else {
                    stderr
                }
            );
        }
        Ok(())
    }

    async fn status(&self, db: Option<&str>, run_id: &str) -> Result<ZyalRunStatus> {
        let mut cmd = tokio::process::Command::new(&self.bin);
        cmd.arg("port-run").arg("--status").arg(run_id);
        if let Some(db) = db {
            cmd.arg("--db").arg(db);
        }
        let output = cmd
            .output()
            .await
            .with_context(|| format!("spawn `{} port-run --status`", self.bin))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            anyhow::bail!(
                "jekko port-run --status failed: {}",
                if stderr.is_empty() {
                    "<no stderr>".to_owned()
                } else {
                    stderr
                }
            );
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut status: ZyalRunStatus =
            serde_json::from_str(stdout.trim()).context("parse `jekko port-run --status` JSON")?;
        if status.run_id.is_empty() {
            status.run_id = run_id.to_owned();
        }
        Ok(status)
    }
}

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

        let submit_result = self.runner.submit(&manifest_path, db, &run_id, &opts).await;
        if let Err(err) = submit_result {
            // Best-effort cleanup before failing closed.
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
mod tests {
    use super::*;
    use jmcp_domain::WorkOrder;
    use serde_json::json;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    fn phase(id: &str, status: &str) -> ZyalPhase {
        ZyalPhase {
            phase_id: id.to_owned(),
            status: status.to_owned(),
        }
    }

    fn status_of(run_id: &str, phases: Vec<ZyalPhase>) -> ZyalRunStatus {
        ZyalRunStatus {
            run_id: run_id.to_owned(),
            phases,
        }
    }

    // ---- Pure status logic ----

    #[test]
    fn percent_handles_empty_partial_full() {
        assert_eq!(status_of("r", vec![]).percent(), 0);
        assert_eq!(
            status_of("r", vec![phase("a", "complete"), phase("b", "running")]).percent(),
            50
        );
        assert_eq!(
            status_of(
                "r",
                vec![
                    phase("a", "complete"),
                    phase("b", "complete"),
                    phase("c", "complete")
                ]
            )
            .percent(),
            100
        );
        // 1 of 3 complete -> floor(33.3) = 33.
        assert_eq!(
            status_of(
                "r",
                vec![
                    phase("a", "complete"),
                    phase("b", "pending"),
                    phase("c", "pending")
                ]
            )
            .percent(),
            33
        );
    }

    #[test]
    fn is_terminal_and_state_label() {
        let empty = status_of("r", vec![]);
        assert!(!empty.is_terminal());
        assert_eq!(empty.state_label(), "unknown");

        let running = status_of("r", vec![phase("a", "complete"), phase("b", "running")]);
        assert!(!running.is_terminal());
        assert_eq!(running.state_label(), "partial");

        let done = status_of("r", vec![phase("a", "complete"), phase("b", "complete")]);
        assert!(done.is_terminal());
        assert_eq!(done.state_label(), "complete");

        let blocked = status_of("r", vec![phase("a", "complete"), phase("b", "blocked")]);
        assert!(blocked.is_terminal());
        assert_eq!(blocked.state_label(), "blocked");

        let failed = status_of(
            "r",
            vec![
                phase("a", "complete"),
                phase("b", "failed"),
                phase("c", "blocked"),
            ],
        );
        assert!(failed.is_terminal());
        assert_eq!(failed.state_label(), "failed", "failed wins over blocked");
    }

    #[test]
    fn parses_status_json_from_port_run() {
        let v = r#"{"phases":[{"phase_id":"frame","status":"complete"},{"phase_id":"produce","status":"running"}]}"#;
        let parsed: ZyalRunStatus = serde_json::from_str(v).expect("parse");
        assert_eq!(parsed.total(), 2);
        assert_eq!(parsed.completed(), 1);
        assert_eq!(parsed.percent(), 50);
        // run_id is absent in the CLI JSON; the runner fills it from the arg.
        assert!(parsed.run_id.is_empty());
    }

    // ---- Stub-runner adapter tests ----

    struct StubRunner {
        snapshots: Vec<ZyalRunStatus>,
        idx: AtomicUsize,
        submitted_run_id: Mutex<Option<String>>,
        submitted_manifest_existed: Mutex<Option<bool>>,
    }

    impl StubRunner {
        fn new(snapshots: Vec<ZyalRunStatus>) -> Self {
            Self {
                snapshots,
                idx: AtomicUsize::new(0),
                submitted_run_id: Mutex::new(None),
                submitted_manifest_existed: Mutex::new(None),
            }
        }
    }

    #[async_trait]
    impl ZyalRunner for StubRunner {
        async fn submit(
            &self,
            manifest_path: &Path,
            _db: Option<&str>,
            run_id: &str,
            _opts: &SubmitOpts,
        ) -> Result<()> {
            *self.submitted_run_id.lock().unwrap() = Some(run_id.to_owned());
            *self.submitted_manifest_existed.lock().unwrap() = Some(manifest_path.exists());
            Ok(())
        }

        async fn status(&self, _db: Option<&str>, _run_id: &str) -> Result<ZyalRunStatus> {
            // Return successive snapshots, repeating the last once exhausted.
            let i = self.idx.fetch_add(1, Ordering::SeqCst);
            let snap = self
                .snapshots
                .get(i)
                .or_else(|| self.snapshots.last())
                .cloned()
                .unwrap_or_default();
            Ok(snap)
        }
    }

    fn manifest_payload(run_id: &str) -> serde_json::Value {
        json!({
            "manifest": { "id": "m", "name": "m", "objective": "o", "phases": [] },
            "run_id": run_id,
        })
    }

    #[tokio::test]
    async fn execute_emits_run_progress_and_status_evidence() {
        let terminal = status_of(
            "run-x",
            vec![phase("a", "complete"), phase("b", "complete")],
        );
        let stub = Arc::new(StubRunner::new(vec![terminal]));
        let adapter = ZyalAdapter::new(stub.clone()).with_poll_interval(Duration::from_millis(1));
        let wo = WorkOrder::submit("t/zyal/e", "zyal.run", manifest_payload("run-x"));

        let ev = adapter.execute(&wo).await.expect("execute");
        let kinds: Vec<&str> = ev.iter().map(|e| e.kind.as_str()).collect();
        assert!(kinds.contains(&"zyal.run"));
        let progress = ev.iter().find(|e| e.kind == "zyal.progress").unwrap();
        assert_eq!(progress.uri, "zyal://run/run-x/progress/100");
        let state = ev.iter().find(|e| e.kind == "zyal.status").unwrap();
        assert_eq!(state.uri, "zyal://run/run-x/status/complete");
        // Manifest was materialized before submit.
        assert_eq!(
            *stub.submitted_manifest_existed.lock().unwrap(),
            Some(true),
            "the full manifest is written to a file before submit"
        );
        assert_eq!(
            stub.submitted_run_id.lock().unwrap().as_deref(),
            Some("run-x")
        );
    }

    #[tokio::test]
    async fn execute_rejects_unsupported_kind_fail_closed() {
        let adapter = ZyalAdapter::new(Arc::new(StubRunner::new(vec![])));
        let wo = WorkOrder::submit("t/zyal/e", "jekko.run", json!({}));
        let err = adapter
            .execute(&wo)
            .await
            .expect_err("unsupported kind fails closed");
        assert!(err.to_string().contains("zyal"), "err: {err}");
    }

    #[tokio::test]
    async fn execute_polls_until_terminal() {
        let running = status_of("run-y", vec![phase("a", "complete"), phase("b", "running")]);
        let done = status_of(
            "run-y",
            vec![phase("a", "complete"), phase("b", "complete")],
        );
        // Two non-terminal snapshots then a terminal one.
        let stub = Arc::new(StubRunner::new(vec![running.clone(), running, done]));
        let adapter = ZyalAdapter::new(stub.clone()).with_poll_interval(Duration::from_millis(1));
        let wo = WorkOrder::submit("t/zyal/e", "zyal.run", manifest_payload("run-y"));

        let ev = adapter.execute(&wo).await.expect("execute");
        let progress = ev.iter().find(|e| e.kind == "zyal.progress").unwrap();
        assert_eq!(progress.uri, "zyal://run/run-y/progress/100");
        // The adapter polled at least 3 times (2 running + 1 done).
        assert!(stub.idx.load(Ordering::SeqCst) >= 3);
    }

    // ---- Fake-binary integration tests for CliZyalRunner ----

    #[cfg(unix)]
    fn unique_dir() -> std::path::PathBuf {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("jmcp-zyal-clitest-{}-{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn write_exec(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::write(path, body).unwrap();
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms).unwrap();
    }

    #[cfg(unix)]
    const STUB_JEKKO_OK: &str = r#"#!/bin/sh
mode=""
for a in "$@"; do
  case "$a" in
    --super) mode=submit ;;
    --status) mode=status ;;
  esac
done
if [ "$mode" = status ]; then
  printf '%s' '{"phases":[{"phase_id":"a","status":"complete"},{"phase_id":"b","status":"complete"}]}'
fi
exit 0
"#;

    #[cfg(unix)]
    const STUB_JEKKO_FAIL: &str = "#!/bin/sh\necho 'manifest rejected' 1>&2\nexit 1\n";

    #[cfg(unix)]
    #[tokio::test]
    async fn cli_runner_submits_and_parses_status() {
        let dir = unique_dir();
        let stub = dir.join("jekko_ok.sh");
        write_exec(&stub, STUB_JEKKO_OK);
        let manifest = dir.join("manifest.json");
        std::fs::write(&manifest, b"{}").unwrap();

        let runner = CliZyalRunner::with_bin(stub.to_str().unwrap());
        runner
            .submit(&manifest, None, "run-cli", &SubmitOpts::default())
            .await
            .expect("submit succeeds");
        let status = runner.status(None, "run-cli").await.expect("status parses");
        assert_eq!(status.percent(), 100);
        assert!(status.is_terminal());
        // run_id absent in JSON -> filled from the arg.
        assert_eq!(status.run_id, "run-cli");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cli_runner_submit_propagates_failure() {
        let dir = unique_dir();
        let stub = dir.join("jekko_fail.sh");
        write_exec(&stub, STUB_JEKKO_FAIL);
        let manifest = dir.join("manifest.json");
        std::fs::write(&manifest, b"{}").unwrap();

        let runner = CliZyalRunner::with_bin(stub.to_str().unwrap());
        let err = runner
            .submit(&manifest, None, "run-cli", &SubmitOpts::default())
            .await
            .expect_err("non-zero submit must error");
        assert!(format!("{err:#}").contains("submit failed"), "err: {err:#}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
