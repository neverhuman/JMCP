//! jnoccio-ROUTER `worker_run` autonomous-worker integration.
//!
//! Drives the jnoccio-router MCP `worker_run` tool over JSON-RPC 2.0 (`POST
//! {JNOCCIO_BASE_URL}/mcp`): the router spawns an autonomous Jekko worker that
//! reads/edits a repo and reports the concrete file changes, commands run, and
//! failures it produced. The structured result is normalized into the same
//! [`JekkoRunOutcome`] shape the fusion-chat path emits so the adapter's
//! work-order -> evidence mapping is unchanged.
//!
//! The wire payload is parsed **permissively** (serde defaults / `Option`) so a
//! thin or evolving router response still deserializes; missing pieces degrade
//! to fallbacks rather than a hard parse failure. Failures are fail-closed: an
//! unreachable router, an error body, or a detached run that never settles
//! returns an error, never a silent empty result. The bearer token is read from
//! `JNOCCIO_API_KEY` and is never logged.

use crate::{HttpJekkoClient, JekkoArtifact, JekkoRunOutcome, JekkoRunRequest};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

/// Wall-clock budget handed to the router for the autonomous run, in ms.
const WORKER_TIMEOUT_MS: u64 = 120_000;
/// Maximum number of `job_result` polls before a still-running detached run is
/// declared fail-closed. Bounded so the call can never hang indefinitely.
const MAX_DETACHED_POLLS: usize = 3;
/// Delay between detached `job_result` polls.
const DETACHED_POLL_DELAY: Duration = Duration::from_millis(500);

/// JSON-RPC 2.0 envelope for an MCP `tools/call` request.
#[derive(Debug, Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'a str,
    id: &'a str,
    method: &'a str,
    params: ToolCallParams<'a>,
}

/// `params` of an MCP `tools/call`: the tool name plus its arguments object.
#[derive(Debug, Serialize)]
struct ToolCallParams<'a> {
    name: &'a str,
    arguments: Value,
}

/// JSON-RPC 2.0 response envelope. Both `result` and `error` are optional so a
/// thin or error-only body still deserializes.
#[derive(Debug, Default, Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<ToolCallResult>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Default, Deserialize)]
struct JsonRpcError {
    #[serde(default)]
    message: Option<String>,
}

/// The MCP `tools/call` result. We only care about `structuredContent`; the
/// human-readable `content` blocks are ignored.
#[derive(Debug, Default, Deserialize)]
struct ToolCallResult {
    #[serde(default)]
    #[serde(rename = "structuredContent")]
    structured_content: Value,
}

/// Permissive view of the `worker_run` / `worker_team` structured result. Every
/// field is optional or defaulted so an evolving router payload still parses.
#[derive(Debug, Default, Deserialize)]
struct WorkerStructured {
    #[serde(default)]
    job_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    report: WorkerReport,
    #[serde(default)]
    raw_model_summary: Option<String>,
}

/// The worker's structured report: what it read, changed, ran, and failed at.
///
/// `files_read` / `files_changed` / `commands_run` are part of the router's
/// wire contract and are parsed (permissively) even though only `file_changes`
/// and `failures` feed the current outcome mapping; keeping them documents the
/// full shape and is forward-compatible with richer evidence.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct WorkerReport {
    #[serde(default)]
    files_read: Vec<String>,
    #[serde(default)]
    files_changed: Vec<String>,
    #[serde(default)]
    file_changes: Vec<WorkerFileChange>,
    #[serde(default)]
    commands_run: Vec<String>,
    #[serde(default)]
    failures: Vec<String>,
}

/// One concrete file mutation with before/after content digests. `before_sha256`
/// is part of the wire contract (parsed permissively); only `after_sha256` is
/// surfaced as the artifact digest today.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
struct WorkerFileChange {
    #[serde(default)]
    path: String,
    #[serde(default)]
    before_sha256: Option<String>,
    #[serde(default)]
    after_sha256: Option<String>,
}

/// Drive the router's `worker_run` tool and normalize its result.
///
/// On a `running` status with a `job_id` the run is detached; we poll
/// `job_result` a bounded number of times and fail closed if it never settles.
pub(crate) async fn run_worker(
    client: &HttpJekkoClient,
    request: JekkoRunRequest,
) -> Result<JekkoRunOutcome> {
    let arguments = json!({
        "task": request.prompt,
        "repo_root": request.cwd,
        "timeout_ms": WORKER_TIMEOUT_MS,
    });
    let structured = call_tool(client, "worker_run", arguments).await?;

    // Detached run: bounded polling of job_result before failing closed.
    if status_of(&structured) == Some("running") {
        if let Some(job_id) = job_id_of(&structured) {
            return poll_detached(client, &job_id).await;
        }
    }
    Ok(map_worker_outcome(&structured))
}

/// Poll `job_result` for a detached run up to [`MAX_DETACHED_POLLS`] times. If
/// it settles to a terminal status, map it; otherwise fail closed.
async fn poll_detached(client: &HttpJekkoClient, job_id: &str) -> Result<JekkoRunOutcome> {
    for _ in 0..MAX_DETACHED_POLLS {
        tokio::time::sleep(DETACHED_POLL_DELAY).await;
        let structured = call_tool(client, "job_result", json!({ "job_id": job_id })).await?;
        if status_of(&structured) != Some("running") {
            return Ok(map_worker_outcome(&structured));
        }
    }
    anyhow::bail!("jekko worker_run still running (detached)")
}

/// Issue one JSON-RPC `tools/call` and return the parsed `structuredContent`.
async fn call_tool(client: &HttpJekkoClient, name: &str, arguments: Value) -> Result<Value> {
    let url = format!("{}/mcp", client.jnoccio_base_url.trim_end_matches('/'));
    let body = JsonRpcRequest {
        jsonrpc: "2.0",
        id: "1",
        method: "tools/call",
        params: ToolCallParams { name, arguments },
    };
    let mut builder = client.http.post(&url).json(&body);
    if let Some(key) = &client.api_key {
        builder = builder.bearer_auth(key);
    }
    let response = builder
        .send()
        .await
        .with_context(|| format!("jnoccio worker {name} request failed"))?;
    let http_status = response.status();
    if !http_status.is_success() {
        anyhow::bail!("jnoccio worker {name} returned {http_status}");
    }
    let rpc: JsonRpcResponse = response
        .json()
        .await
        .with_context(|| format!("jnoccio worker {name} response was not valid JSON"))?;
    if let Some(error) = rpc.error {
        let message = match error.message {
            Some(message) => message,
            None => "unknown error".to_owned(),
        };
        anyhow::bail!("jnoccio worker {name} error: {}", message);
    }
    let structured_content = match rpc.result {
        Some(result) => result.structured_content,
        None => Value::Null,
    };
    Ok(structured_content)
}

/// `structuredContent.status` as a borrowed string, if present.
fn status_of(structured: &Value) -> Option<&str> {
    structured.get("status").and_then(Value::as_str)
}

/// `structuredContent.job_id` as an owned non-empty string, if present.
fn job_id_of(structured: &Value) -> Option<String> {
    non_empty_owned(
        structured
            .get("job_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
    )
}

/// Pure mapping from a `worker_run`/`job_result` `structuredContent` value to a
/// normalized [`JekkoRunOutcome`]. No network, no clock: deterministic so the
/// mapping is unit-tested in isolation.
///
/// - `run_ref` = `job_id` (fallback `"jnoccio-worker"`).
/// - `assistant_text` = `summary`, else `raw_model_summary`.
/// - `artifacts` = one `kind:"file"` artifact per `report.file_changes` entry,
///   `reference` = path, `digest` = `after_sha256`.
/// - `success` = `status == "succeeded"`.
/// - `error` = joined `report.failures` when `status == "failed"`, else `None`.
pub fn map_worker_outcome(structured: &Value) -> JekkoRunOutcome {
    // Permissive: a value that does not match the shape degrades to defaults
    // rather than panicking, keeping the mapping fail-soft on thin payloads.
    let worker: WorkerStructured = match serde_json::from_value(structured.clone()) {
        Ok(worker) => worker,
        Err(_) => WorkerStructured::default(),
    };

    let status = match worker.status.as_deref() {
        Some(status) => status,
        None => "",
    };
    let success = status == "succeeded";

    let assistant_text = match non_empty_owned(worker.summary) {
        Some(text) => Some(text),
        None => non_empty_owned(worker.raw_model_summary),
    };

    let artifacts = worker
        .report
        .file_changes
        .iter()
        .map(|change| JekkoArtifact {
            kind: "file".to_owned(),
            reference: change.path.clone(),
            digest: change.after_sha256.clone(),
        })
        .collect();

    let error = if status == "failed" {
        let joined = worker.report.failures.join("; ");
        Some(if joined.is_empty() {
            "jekko worker_run failed".to_owned()
        } else {
            joined
        })
    } else {
        None
    };

    JekkoRunOutcome {
        run_ref: run_ref_or_default(worker.job_id),
        assistant_text,
        artifacts,
        success,
        error,
    }
}

fn run_ref_or_default(job_id: Option<String>) -> String {
    match non_empty_owned(job_id) {
        Some(value) => value,
        None => "jnoccio-worker".to_owned(),
    }
}

fn non_empty_owned(value: Option<String>) -> Option<String> {
    match value {
        Some(value) if !value.is_empty() => Some(value),
        _ => None,
    }
}

#[cfg(test)]
#[path = "worker_run_tests.rs"]
mod tests;
