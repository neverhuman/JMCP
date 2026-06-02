use super::*;
use async_trait::async_trait;
use jmcp_adapter_sdk::Adapter;
use jmcp_domain::WorkOrder;
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    thread,
    time::Duration,
};

static ENV_LOCK: Mutex<()> = Mutex::new(());

fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner())
}

#[tokio::test]
async fn run_agent_maps_summary_events_and_receipts_to_evidence() {
    let dir = temp_dir();
    let events = dir.join("events.jsonl");
    let captured_request = Arc::new(Mutex::new(None));
    let adapter = JailgunAdapter::with_run_client(
        "jailgun",
        Duration::from_secs(5),
        Arc::new(FakeRunClient {
            include_receipt: true,
            events_jsonl: events.clone(),
            captured_request: captured_request.clone(),
        }),
    );
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.run",
        json!({
            "request": {
                "version": 1,
                "prompt_ref": "jmcp://prompt/1",
                "prompt_file": dir.join("prompt.txt")
            },
        }),
    );
    fs::write(dir.join("prompt.txt"), "prompt").unwrap();

    let evidence = adapter.execute(&work_order).await.unwrap();

    assert!(evidence.iter().any(|e| e.kind == "jailgun.run"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.summary"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.events"));
    assert!(evidence.iter().any(|e| e.kind == "jailgun.receipt"));
    assert_eq!(
        captured_request.lock().unwrap().as_ref().unwrap()["version"],
        json!(1)
    );
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn deploy_requires_receipt_paths() {
    let dir = temp_dir();
    let request_path = dir.join("request.json");
    let adapter = JailgunAdapter::with_run_client(
        "jailgun",
        Duration::from_secs(5),
        Arc::new(FakeRunClient {
            include_receipt: false,
            events_jsonl: dir.join("events.jsonl"),
            captured_request: Arc::new(Mutex::new(None)),
        }),
    );
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.deploy",
        json!({
            "request_path": request_path,
        }),
    );
    fs::write(
        dir.join("request.json"),
        serde_json::to_vec(&json!({
            "version": 1,
            "prompt_ref": "jmcp://prompt/1",
            "prompt_file": dir.join("prompt.txt")
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(dir.join("prompt.txt"), "prompt").unwrap();

    let error = adapter.execute(&work_order).await.unwrap_err();
    assert!(error.to_string().contains("missing receipt_paths"));
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn run_agent_rejects_non_v1_request() {
    let adapter = JailgunAdapter::with_run_client(
        "jailgun",
        Duration::from_secs(5),
        Arc::new(FakeRunClient {
            include_receipt: true,
            events_jsonl: PathBuf::from("events.jsonl"),
            captured_request: Arc::new(Mutex::new(None)),
        }),
    );
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.run",
        json!({
            "request": {
                "version": 2,
                "prompt_ref": "jmcp://prompt/1",
                "prompt_file": "prompt.txt"
            }
        }),
    );

    let error = adapter.execute(&work_order).await.unwrap_err();

    assert!(error.to_string().contains("expected 1"));
}

#[tokio::test]
async fn inline_request_wins_over_request_path() {
    let dir = temp_dir();
    let captured_request = Arc::new(Mutex::new(None));
    let adapter = JailgunAdapter::with_run_client(
        "jailgun",
        Duration::from_secs(5),
        Arc::new(FakeRunClient {
            include_receipt: true,
            events_jsonl: dir.join("events.jsonl"),
            captured_request: captured_request.clone(),
        }),
    );
    let compatibility_path = dir.join("compatibility-request.json");
    fs::write(
        &compatibility_path,
        serde_json::to_vec(&json!({
            "version": 1,
            "prompt_ref": "compatibility",
            "prompt_file": "compatibility.txt"
        }))
        .unwrap(),
    )
    .unwrap();
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.run",
        json!({
            "request": {
                "version": 1,
                "prompt_ref": "inline",
                "prompt_file": "inline.txt"
            },
            "request_path": compatibility_path
        }),
    );

    adapter.execute(&work_order).await.unwrap();

    assert_eq!(
        captured_request.lock().unwrap().as_ref().unwrap()["prompt_ref"],
        json!("inline")
    );
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
            "version": 1,
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
async fn review_packet_rejects_non_v1_request() {
    let dir = temp_dir();
    let bin = fake_jailgun(&dir, true);
    let adapter = JailgunAdapter::new(&bin, Duration::from_secs(5));
    let work_order = WorkOrder::submit(
        "t/jailgun/e",
        "jailgun.review_packet",
        json!({
            "version": 9,
            "summary_json": dir.join("summary.json"),
            "base": "HEAD~1",
            "head": "HEAD",
            "repo": ".",
            "output": dir.join("review.json")
        }),
    );

    let error = adapter.execute(&work_order).await.unwrap_err();

    assert!(error.to_string().contains("expected 1"));
    let _ = fs::remove_dir_all(dir);
}

#[tokio::test]
async fn unknown_kind_fails_closed() {
    let adapter = JailgunAdapter::new("jailgun", Duration::from_secs(5));
    let work_order = WorkOrder::submit("t/jailgun/e", "other.kind", json!({}));

    assert!(adapter.execute(&work_order).await.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn http_client_posts_request_with_ingest_token() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let (tx, rx) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 8192];
        let read = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..read]).to_string();
        tx.send(request).unwrap();
        let body = r#"{
  "run_id":"run-1",
  "status":"accepted",
  "summary_json":"summary.json",
  "events_jsonl":"events.jsonl",
  "run_url":"/api/runs/run-1",
  "summary_url":"/api/runs/run-1/agent-summary"
}"#;
        write!(
            stream,
            "HTTP/1.1 202 Accepted\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    let _guard = env_lock();
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", &base_url);
    let client = HttpJailgunRunClient::new(&base_url, "secret");

    let accepted = client
        .start_run(&json!({
            "version": 1,
            "prompt_ref": "jmcp://prompt/1",
            "prompt_file": "prompt.txt"
        }))
        .await
        .unwrap();

    assert_eq!(accepted.run_id, "run-1");
    let request = rx.recv().unwrap();
    assert!(request.starts_with("POST /api/runs HTTP/1.1"));
    assert!(request.contains("x-jailgun-token: secret"));
    assert!(request.contains(r#""version":1"#));
    std::env::remove_var("JMCP_JAILGUN_ALLOWED_URLS");
}

#[test]
fn http_config_rejects_missing_token() {
    let _guard = env_lock();
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", "http://127.0.0.1:1");

    let error = validate_jailgun_client_config("http://127.0.0.1:1", " ").unwrap_err();

    let message = error.to_string();
    assert!(message.contains("token"));
    assert!(!message.contains("JMCP_JAILGUN_ALLOWED_URLS"));
    std::env::remove_var("JMCP_JAILGUN_ALLOWED_URLS");
}

#[test]
fn http_config_rejects_endpoint_outside_local_policy() {
    let _guard = env_lock();
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", "http://127.0.0.1:2");

    let error = validate_jailgun_client_config("http://127.0.0.1:1", "secret").unwrap_err();

    let message = error.to_string();
    assert!(message.contains("outside configured local submission policy"));
    assert!(!message.contains("127.0.0.1:1"));
    std::env::remove_var("JMCP_JAILGUN_ALLOWED_URLS");
}

#[test]
fn http_config_rejects_query_or_fragment_base_url() {
    for url in [
        "http://127.0.0.1:1?token=secret",
        "http://127.0.0.1:1#secret",
    ] {
        let error = validate_jailgun_client_config(url, "secret").unwrap_err();
        assert!(
            error.to_string().contains("query or fragment"),
            "unexpected error for {url}: {error}"
        );
    }
}

#[test]
fn summary_url_must_stay_same_origin_relative() {
    let client = HttpJailgunRunClient::new("http://127.0.0.1:1", "secret");

    let cross_origin = client
        .summary_uri("https://example.invalid/api/runs/run-1/summary")
        .unwrap_err();
    assert!(cross_origin.to_string().contains("outside local origin"));

    let query = client
        .summary_uri("/api/runs/run-1/summary?token=secret")
        .unwrap_err();
    assert!(query.to_string().contains("query or fragment"));

    let fragment = client
        .summary_uri("/api/runs/run-1/summary#secret")
        .unwrap_err();
    assert!(fragment.to_string().contains("query or fragment"));
}

#[tokio::test(flavor = "current_thread")]
async fn summary_rejects_prompt_text_leakage() {
    let body = r#"{
  "version":1,
  "run_id":"run-1",
  "status":"succeeded",
  "prompt_ref":"jmcp://prompt/1",
  "prompt":"leak",
  "events_jsonl":"events.jsonl"
}"#;
    let base_url = single_response_server(body);
    let _guard = env_lock();
    std::env::set_var("JMCP_JAILGUN_ALLOWED_URLS", &base_url);
    let client = HttpJailgunRunClient::new(&base_url, "secret");

    let error = client
        .wait_for_summary("/api/runs/run-1/agent-summary", Duration::from_secs(1))
        .await
        .unwrap_err();

    assert!(error.to_string().contains("prompt text key"));
    std::env::remove_var("JMCP_JAILGUN_ALLOWED_URLS");
}

#[test]
fn review_packet_rejects_prompt_text_leakage() {
    let dir = temp_dir();
    let output = dir.join("review.json");
    fs::write(
        &output,
        r#"{"version":1,"run_id":"run-1","prompt_text":"leak"}"#,
    )
    .unwrap();

    let error = review_packet_evidence(&output).unwrap_err();

    assert!(error.to_string().contains("prompt text key"));
    let _ = fs::remove_dir_all(dir);
}

#[derive(Debug)]
struct FakeRunClient {
    include_receipt: bool,
    events_jsonl: PathBuf,
    captured_request: Arc<Mutex<Option<Value>>>,
}

#[async_trait]
impl JailgunRunClient for FakeRunClient {
    async fn start_run(&self, request: &Value) -> Result<JailgunAcceptedRun> {
        *self.captured_request.lock().unwrap() = Some(request.clone());
        Ok(JailgunAcceptedRun {
            run_id: "run-1".to_owned(),
            status: "accepted".to_owned(),
            summary_json: "summary.json".to_owned(),
            events_jsonl: self.events_jsonl.display().to_string(),
            run_url: "/api/runs/run-1".to_owned(),
            summary_url: "/api/runs/run-1/agent-summary".to_owned(),
        })
    }

    async fn wait_for_summary(
        &self,
        _summary_url: &str,
        _timeout: Duration,
    ) -> Result<JailgunSummary> {
        Ok(fake_summary(&self.events_jsonl, self.include_receipt))
    }

    fn summary_uri(&self, summary_url: &str) -> Result<String> {
        Ok(format!("jailgun://localhost{summary_url}"))
    }
}

fn fake_summary(events_jsonl: &std::path::Path, include_receipt: bool) -> JailgunSummary {
    JailgunSummary {
        version: 1,
        run_id: "run-1".to_owned(),
        status: "succeeded".to_owned(),
        prompt_ref: "jmcp://prompt/1".to_owned(),
        events_jsonl: events_jsonl.to_path_buf(),
        receipt_paths: if include_receipt {
            vec![events_jsonl.with_file_name("receipt.json")]
        } else {
            Vec::new()
        },
        artifacts: vec![JailgunArtifact {
            kind: "downloaded-archive".to_owned(),
            path: events_jsonl.with_file_name("source.tar.gz"),
            sha256: Some("abc".to_owned()),
            receipt_path: None,
        }],
        failures: Vec::new(),
    }
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
  "version": 1,
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

fn single_response_server(body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 8192];
        let _ = stream.read(&mut bytes).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .unwrap();
    });
    format!("http://{addr}")
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
