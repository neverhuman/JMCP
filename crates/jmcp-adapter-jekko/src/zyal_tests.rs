use super::*;
use jmcp_domain::WorkOrder;
use serde_json::json;
use std::path::Path;
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
    let dir = std::env::temp_dir().join(format!("jmcp-zyal-clitest-{}-{}", std::process::id(), n));
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
