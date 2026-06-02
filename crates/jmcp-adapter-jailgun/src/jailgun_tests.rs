use super::*;
use jmcp_adapter_sdk::Adapter;
use jmcp_domain::WorkOrder;
use serde_json::json;
use std::fs;

#[tokio::test]
async fn run_agent_maps_summary_events_and_receipts_to_evidence() {
    let dir = temp_dir();
    let bin = fake_jailgun(&dir, true);
    let summary = dir.join("summary.json");
    let events = dir.join("events.jsonl");
    let adapter = JailgunAdapter::new(&bin, Duration::from_secs(5));
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.run",
        json!({
            "request": {
                "prompt_ref": "jmcp://prompt/1",
                "prompt_file": dir.join("prompt.txt")
            },
            "events_jsonl": events,
            "summary_json": summary
        }),
    );
    fs::write(dir.join("prompt.txt"), "prompt").unwrap();

    let evidence = adapter.execute(&work_order).await.unwrap();

    assert!(evidence.iter().any(|e| e.kind == "jailgun.run"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.summary"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.events"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.receipt"));
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn deploy_requires_receipt_paths() {
    let dir = temp_dir();
    let bin = fake_jailgun(&dir, false);
    let adapter = JailgunAdapter::new(&bin, Duration::from_secs(5));
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.deploy",
        json!({
            "request_path": dir.join("request.json"),
            "events_jsonl": dir.join("events.jsonl"),
            "summary_json": dir.join("summary.json")
        }),
    );
    fs::write(dir.join("request.json"), "{}").unwrap();

    let error = adapter.execute(&work_order).await.unwrap_err();
    assert!(error.to_string().contains("missing receipt_paths"));
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn review_packet_maps_output_to_evidence() {
    let dir = temp_dir();
    let bin = fake_jailgun(&dir, true);
    let output = dir.join("review.json");
    let adapter = JailgunAdapter::new(&bin, Duration::from_secs(5));
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.review_packet",
        json!({
            "summary_json": dir.join("summary.json"),
            "base": "HEAD~1",
            "head": "HEAD",
            "repo": ".",
            "output": output
        }),
    );

    let evidence = adapter.execute(&work_order).await.unwrap();

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].kind, "jailgun.review_packet");
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn unknown_kind_fails_closed() {
    let adapter = JailgunAdapter::new("jailgun", Duration::from_secs(5));
    let work_order = WorkOrder::submit("t/jailgun/e", "other.kind", json!({}));

    assert!(adapter.execute(&work_order).await.is_err());
}

fn fake_jailgun(dir: &std::path::Path, include_receipt: bool) -> PathBuf {
    let bin = dir.join("jailgun");
    let staged_bin = dir.join("jailgun.sh");
    let receipt_json = if include_receipt {
        format!(
            "[{}]",
            serde_json::to_string(&dir.join("receipt.json")).unwrap()
        )
    } else {
        "[]".to_owned()
    };
    fs::write(
        &staged_bin,
        format!(
            r#"#!/usr/bin/env bash
set -euo pipefail
cmd="$1"
shift
if [[ "$cmd" == "run-agent" ]]; then
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --events-jsonl) events="$2"; shift 2 ;;
      --summary-json) summary="$2"; shift 2 ;;
      *) shift 2 ;;
    esac
  done
  mkdir -p "$(dirname "$events")" "$(dirname "$summary")"
  printf '%s\n' '{{"kind":"run-started"}}' > "$events"
  cat > "$summary" <<'JSON'
{{
  "run_id": "run-1",
  "status": "succeeded",
  "prompt_ref": "jmcp://prompt/1",
  "events_jsonl": "{events_path}",
  "receipt_paths": {receipt_json},
  "artifacts": [
    {{"kind":"downloaded-archive","path":"{artifact_path}","sha256":"abc"}}
  ],
  "failures": []
}}
JSON
elif [[ "$cmd" == "review-packet" ]]; then
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --output) output="$2"; shift 2 ;;
      *) shift 2 ;;
    esac
  done
  mkdir -p "$(dirname "$output")"
  printf '%s\n' '{{"version":1,"run_id":"run-1","prompt_ref":"jmcp://prompt/1"}}' > "$output"
else
  exit 2
fi
"#,
            events_path = dir.join("events.jsonl").display(),
            artifact_path = dir.join("source.tar.gz").display(),
            receipt_json = receipt_json
        ),
    )
    .unwrap();
    make_executable(&staged_bin);
    fs::rename(&staged_bin, &bin).unwrap();
    bin
}

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("jmcp-jailgun-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) {}
