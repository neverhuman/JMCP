//! Jailgun local worker adapter.
//!
//! This adapter keeps JMCP as the owner of work orders, leases, evidence, and
//! effect replay. Jailgun is invoked only as a bounded subprocess through its
//! machine interface: `jailgun run-agent` or `jailgun review-packet`.

use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tokio::process::Command;
use tokio::time::sleep;

#[derive(Clone, Debug)]
pub struct JailgunAdapter {
    command: PathBuf,
    timeout: Duration,
}

impl Default for JailgunAdapter {
    fn default() -> Self {
        Self {
            command: std::env::var_os("JMCP_JAILGUN_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("jailgun")),
            timeout: Duration::from_secs(30 * 60 + 30),
        }
    }
}

impl JailgunAdapter {
    pub fn new(command: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            command: command.into(),
            timeout,
        }
    }
}

#[async_trait]
impl Adapter for JailgunAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "jailgun".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/jailgun/*".to_owned()],
            capabilities: vec![
                "bounded-chatgpt-capture".to_owned(),
                "run-agent".to_owned(),
                "review-packet".to_owned(),
            ],
        }
    }

    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        match route_for(&work_order.task.kind).ok_or_else(|| fail_closed("jailgun"))? {
            Route::RunAgent { receipt_required } => {
                self.execute_run_agent(work_order, receipt_required).await
            }
            Route::ReviewPacket => self.execute_review_packet(work_order).await,
        }
    }
}

enum Route {
    RunAgent { receipt_required: bool },
    ReviewPacket,
}

fn route_for(kind: &str) -> Option<Route> {
    match kind {
        "jailgun.run" | "jailgun.capture" => Some(Route::RunAgent {
            receipt_required: false,
        }),
        "jailgun.deploy" => Some(Route::RunAgent {
            receipt_required: true,
        }),
        "jailgun.review_packet" => Some(Route::ReviewPacket),
        _ => None,
    }
}

impl JailgunAdapter {
    async fn execute_run_agent(
        &self,
        work_order: &WorkOrder,
        receipt_required: bool,
    ) -> Result<Vec<Evidence>> {
        let payload = &work_order.task.payload;
        let cwd = payload_str(payload, "cwd").unwrap_or(".");
        let events_jsonl = required_path(payload, "events_jsonl")?;
        let summary_json = required_path(payload, "summary_json")?;
        let request_path = request_path(payload, &summary_json)?;

        let mut command = Command::new(&self.command);
        command
            .arg("run-agent")
            .arg("--request")
            .arg(request_path)
            .arg("--events-jsonl")
            .arg(&events_jsonl)
            .arg("--summary-json")
            .arg(&summary_json)
            .current_dir(cwd);
        run_checked(&self.command, self.timeout, command, "run-agent").await?;

        let summary = read_summary(&summary_json)?;
        if summary.status != "succeeded" {
            anyhow::bail!(
                "jailgun run {} finished with status {}",
                summary.run_id,
                summary.status
            );
        }
        if receipt_required && summary.receipt_paths.is_empty() {
            anyhow::bail!("jailgun deploy summary missing receipt_paths");
        }
        Ok(evidence_for_summary(&summary, &summary_json, &events_jsonl))
    }

    async fn execute_review_packet(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        let payload = &work_order.task.payload;
        let cwd = payload_str(payload, "cwd").unwrap_or(".");
        let summary_json = required_path(payload, "summary_json")?;
        let base = required_str(payload, "base")?;
        let head = required_str(payload, "head")?;
        let repo = payload_str(payload, "repo").unwrap_or(".");
        let output = required_path(payload, "output")?;
        let patch_bytes = payload
            .get("patch_bytes")
            .and_then(|value| value.as_u64())
            .unwrap_or(128 * 1024);

        let mut command = Command::new(&self.command);
        command
            .arg("review-packet")
            .arg("--summary-json")
            .arg(&summary_json)
            .arg("--base")
            .arg(base)
            .arg("--head")
            .arg(head)
            .arg("--repo")
            .arg(repo)
            .arg("--output")
            .arg(&output)
            .arg("--patch-bytes")
            .arg(patch_bytes.to_string())
            .current_dir(cwd);
        run_checked(&self.command, self.timeout, command, "review-packet").await?;

        let packet = std::fs::read_to_string(&output)
            .with_context(|| format!("reading Jailgun review packet {}", output.display()))?;
        let json: Value = serde_json::from_str(&packet).context("invalid Jailgun review packet")?;
        ensure_no_prompt_text(&json)?;
        Ok(vec![Evidence {
            kind: "jailgun.review_packet".to_owned(),
            uri: file_uri(&output),
            captured_at: Utc::now(),
        }])
    }
}

async fn run_checked(
    command_path: &PathBuf,
    timeout: Duration,
    mut command: Command,
    operation: &str,
) -> Result<()> {
    let output = run_with_retry(timeout, &mut command)
        .await
        .with_context(|| format!("failed to run {}", command_path.display()))?;
    if !output.status.success() {
        let digest = digest_output(output.status.code(), &output.stdout, &output.stderr);
        anyhow::bail!("jailgun {operation} failed with digest {digest}");
    }
    Ok(())
}

async fn run_with_retry(timeout: Duration, command: &mut Command) -> Result<std::process::Output> {
    let mut last_error = None;
    for attempt in 0..5 {
        match tokio::time::timeout(timeout, command.output()).await {
            Err(_) => return Err(anyhow::anyhow!("jailgun command timed out")),
            Ok(Err(err)) => {
                let busy = err.raw_os_error() == Some(26);
                if busy && attempt < 4 {
                    last_error = Some(err);
                    sleep(Duration::from_millis(50 * (attempt + 1) as u64)).await;
                    continue;
                }
                return Err(err.into());
            }
            Ok(Ok(output)) => return Ok(output),
        }
    }
    Err(last_error
        .map(anyhow::Error::from)
        .unwrap_or_else(|| anyhow::anyhow!("jailgun command failed to start")))
}

fn request_path(payload: &Value, summary_json: &PathBuf) -> Result<PathBuf> {
    if let Some(path) = payload.get("request_path").and_then(|value| value.as_str()) {
        return Ok(PathBuf::from(path));
    }
    if let Some(request) = payload.get("request") {
        let path = summary_json.with_extension("request.json");
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, serde_json::to_vec_pretty(request)?)
            .with_context(|| format!("writing {}", path.display()))?;
        return Ok(path);
    }
    anyhow::bail!("jailgun work order requires request_path or request")
}

#[derive(Debug, Deserialize)]
struct JailgunSummary {
    run_id: String,
    status: String,
    prompt_ref: String,
    events_jsonl: PathBuf,
    #[serde(default)]
    receipt_paths: Vec<PathBuf>,
    #[serde(default)]
    artifacts: Vec<JailgunArtifact>,
    #[serde(default)]
    failures: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct JailgunArtifact {
    kind: String,
    path: PathBuf,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    receipt_path: Option<PathBuf>,
}

fn read_summary(path: &PathBuf) -> Result<JailgunSummary> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading Jailgun summary {}", path.display()))?;
    let json: Value = serde_json::from_str(&text).context("invalid Jailgun summary JSON")?;
    ensure_no_prompt_text(&json)?;
    serde_json::from_value(json).context("Jailgun summary does not match expected schema")
}

fn ensure_no_prompt_text(value: &Value) -> Result<()> {
    match value {
        Value::Object(map) => {
            if map.contains_key("prompt_text") || map.contains_key("prompt") {
                anyhow::bail!("Jailgun durable artifact contains prompt text key");
            }
            for child in map.values() {
                ensure_no_prompt_text(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                ensure_no_prompt_text(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn evidence_for_summary(
    summary: &JailgunSummary,
    summary_json: &PathBuf,
    events_jsonl: &PathBuf,
) -> Vec<Evidence> {
    let now = Utc::now();
    let mut evidence = vec![
        Evidence {
            kind: "jailgun.run".to_owned(),
            uri: format!("jailgun://run/{}", summary.run_id),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.summary".to_owned(),
            uri: file_uri(summary_json),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.events".to_owned(),
            uri: file_uri(events_jsonl),
            captured_at: now,
        },
        Evidence {
            kind: "jailgun.prompt_ref".to_owned(),
            uri: summary.prompt_ref.clone(),
            captured_at: now,
        },
    ];
    if summary.events_jsonl != *events_jsonl {
        evidence.push(Evidence {
            kind: "jailgun.events.summary-path".to_owned(),
            uri: file_uri(&summary.events_jsonl),
            captured_at: now,
        });
    }
    for receipt in &summary.receipt_paths {
        evidence.push(Evidence {
            kind: "jailgun.receipt".to_owned(),
            uri: file_uri(receipt),
            captured_at: now,
        });
    }
    for artifact in &summary.artifacts {
        let uri = artifact
            .sha256
            .as_ref()
            .map(|sha| format!("sha256:{sha}"))
            .unwrap_or_else(|| file_uri(&artifact.path));
        evidence.push(Evidence {
            kind: format!("jailgun.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
        if let Some(receipt) = &artifact.receipt_path {
            evidence.push(Evidence {
                kind: "jailgun.artifact.receipt".to_owned(),
                uri: file_uri(receipt),
                captured_at: now,
            });
        }
    }
    if !summary.failures.is_empty() {
        let digest = hex::encode(Sha256::digest(
            serde_json::to_string(&summary.failures)
                .unwrap_or_default()
                .as_bytes(),
        ));
        evidence.push(Evidence {
            kind: "jailgun.failures.digest".to_owned(),
            uri: format!("sha256:{digest}"),
            captured_at: now,
        });
    }
    evidence
}

fn required_path(payload: &Value, key: &str) -> Result<PathBuf> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(PathBuf::from)
        .with_context(|| format!("jailgun work order missing {key}"))
}

fn required_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .with_context(|| format!("jailgun work order missing {key}"))
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|value| value.as_str())
}

fn file_uri(path: &PathBuf) -> String {
    format!("file://{}", path.display())
}

fn digest_output(status: Option<i32>, stdout: &[u8], stderr: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(status.unwrap_or(-1).to_string().as_bytes());
    hasher.update(b"\0stdout\0");
    hasher.update(stdout);
    hasher.update(b"\0stderr\0");
    hasher.update(stderr);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
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
}
