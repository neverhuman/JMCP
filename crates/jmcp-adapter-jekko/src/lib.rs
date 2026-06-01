//! Jekko worker adapter.
//!
//! Drives the Jekko autonomous worker engine and the jnoccio model router as a
//! JMCP [`Adapter`]. Worker reasoning is routed through jnoccio-fusion's
//! OpenAI-compatible endpoint (`/v1/chat/completions`); autonomous Jekko daemon
//! runs are layered on as the additive Jekko event API lands. The HTTP surface
//! is abstracted behind [`JekkoClient`] so the work-order -> evidence mapping is
//! tested deterministically without a live engine. Failures are fail-closed: an
//! unreachable or erroring engine returns an error, never a silent empty result.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

const DEFAULT_JEKKO_BASE_URL: &str = "http://127.0.0.1:4317";
const DEFAULT_JNOCCIO_BASE_URL: &str = "http://127.0.0.1:8765";
const DEFAULT_MODEL: &str = "jnoccio/jnoccio-fusion";
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// A unit of work handed to the Jekko engine.
#[derive(Clone, Debug, Serialize)]
pub struct JekkoRunRequest {
    pub prompt: String,
    pub cwd: String,
    pub model: String,
}

/// A content artifact produced by a Jekko run.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct JekkoArtifact {
    pub kind: String,
    pub reference: String,
    #[serde(default)]
    pub digest: Option<String>,
}

/// The normalized outcome of a Jekko run.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct JekkoRunOutcome {
    pub run_ref: String,
    #[serde(default)]
    pub assistant_text: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<JekkoArtifact>,
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
}

/// Transport abstraction over the Jekko engine + jnoccio router.
///
/// Implemented by [`HttpJekkoClient`] at runtime and by deterministic stubs in
/// tests, so the adapter's mapping logic is verifiable without a live engine.
#[async_trait]
pub trait JekkoClient: Send + Sync {
    /// Liveness probe for the worker engine.
    async fn health(&self) -> Result<()>;
    /// Execute a single run and return its normalized outcome.
    async fn run(&self, request: JekkoRunRequest) -> Result<JekkoRunOutcome>;
}

/// HTTP client driving Jekko (`:4317`) and jnoccio-fusion (`/v1/chat/completions`).
///
/// Endpoints and the optional bearer token come from the environment and are
/// never logged. See [`HttpJekkoClient::from_env`].
#[derive(Clone)]
pub struct HttpJekkoClient {
    http: reqwest::Client,
    jekko_base_url: String,
    jnoccio_base_url: String,
    api_key: Option<String>,
}

impl HttpJekkoClient {
    /// Build a client from `JEKKO_BASE_URL`, `JNOCCIO_BASE_URL`, and the optional
    /// `JNOCCIO_API_KEY` (sent as a bearer token; never logged).
    pub fn from_env() -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            jekko_base_url: env_or("JEKKO_BASE_URL", DEFAULT_JEKKO_BASE_URL),
            jnoccio_base_url: env_or("JNOCCIO_BASE_URL", DEFAULT_JNOCCIO_BASE_URL),
            api_key: std::env::var("JNOCCIO_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }
}

#[async_trait]
impl JekkoClient for HttpJekkoClient {
    async fn health(&self) -> Result<()> {
        let url = format!("{}/health", self.jekko_base_url.trim_end_matches('/'));
        let response = self
            .http
            .get(&url)
            .send()
            .await
            .context("jekko health request failed")?;
        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("jekko health returned {}", response.status());
        }
    }

    async fn run(&self, request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
        // Worker reasoning is routed through jnoccio-fusion's OpenAI-compatible API.
        let url = format!(
            "{}/v1/chat/completions",
            self.jnoccio_base_url.trim_end_matches('/')
        );
        let body = ChatRequest {
            model: &request.model,
            messages: vec![ChatMessage {
                role: "user",
                content: &request.prompt,
            }],
            stream: false,
        };
        let mut builder = self.http.post(&url).json(&body);
        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .context("jnoccio chat request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("jnoccio chat returned {status}");
        }
        let chat: ChatResponse = response
            .json()
            .await
            .context("jnoccio chat response was not valid JSON")?;
        let assistant_text = chat
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content);
        Ok(JekkoRunOutcome {
            run_ref: chat.id.unwrap_or_else(|| "jnoccio".to_owned()),
            assistant_text,
            artifacts: Vec::new(),
            success: true,
            error: None,
        })
    }
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Deserialize)]
struct ChatChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

/// JMCP adapter that executes work orders on the Jekko engine.
pub struct JekkoAdapter {
    client: Arc<dyn JekkoClient>,
    model: String,
}

impl Default for JekkoAdapter {
    fn default() -> Self {
        Self::new(
            Arc::new(HttpJekkoClient::from_env()),
            env_or("JEKKO_MODEL", DEFAULT_MODEL),
        )
    }
}

impl JekkoAdapter {
    /// Build an adapter over an arbitrary [`JekkoClient`] (used by tests to inject
    /// a deterministic stub).
    pub fn new(client: Arc<dyn JekkoClient>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }
}

#[async_trait]
impl Adapter for JekkoAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "jekko".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/jekko/*".to_owned()],
            capabilities: vec![
                "worker".to_owned(),
                "reasoning".to_owned(),
                "jnoccio-router".to_owned(),
            ],
        }
    }

    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        if !is_worker_kind(&work_order.task.kind) {
            return Err(fail_closed("jekko"));
        }
        let request = JekkoRunRequest {
            prompt: prompt_for(work_order),
            cwd: work_order
                .task
                .payload
                .get("cwd")
                .and_then(|value| value.as_str())
                .unwrap_or(".")
                .to_owned(),
            model: self.model.clone(),
        };
        let outcome = self.client.run(request).await?;
        if !outcome.success {
            anyhow::bail!(
                "jekko run failed: {}",
                outcome.error.unwrap_or_else(|| "unknown error".to_owned())
            );
        }
        Ok(evidence_for(&outcome))
    }
}

fn is_worker_kind(kind: &str) -> bool {
    matches!(
        kind,
        "jekko.run" | "jekko.task" | "run" | "worker" | "reason"
    )
}

fn prompt_for(work_order: &WorkOrder) -> String {
    if let Some(prompt) = work_order
        .task
        .payload
        .get("prompt")
        .and_then(|value| value.as_str())
    {
        return prompt.to_owned();
    }
    format!("{}\n\n{}", work_order.task.kind, work_order.task.payload)
}

fn evidence_for(outcome: &JekkoRunOutcome) -> Vec<Evidence> {
    let now = Utc::now();
    let mut evidence = vec![Evidence {
        kind: "jekko.run".to_owned(),
        uri: format!("jekko://run/{}", outcome.run_ref),
        captured_at: now,
    }];
    if let Some(text) = &outcome.assistant_text {
        let digest = hex::encode(Sha256::digest(text.as_bytes()));
        evidence.push(Evidence {
            kind: "jekko.assistant.digest".to_owned(),
            uri: format!("sha256:{digest}"),
            captured_at: now,
        });
    }
    for artifact in &outcome.artifacts {
        let uri = artifact
            .digest
            .clone()
            .map(|digest| format!("sha256:{digest}"))
            .unwrap_or_else(|| artifact.reference.clone());
        evidence.push(Evidence {
            kind: format!("jekko.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
    }
    evidence
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct StubClient {
        outcome: JekkoRunOutcome,
    }

    #[async_trait]
    impl JekkoClient for StubClient {
        async fn health(&self) -> Result<()> {
            Ok(())
        }

        async fn run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
            Ok(self.outcome.clone())
        }
    }

    struct DownClient;

    #[async_trait]
    impl JekkoClient for DownClient {
        async fn health(&self) -> Result<()> {
            anyhow::bail!("engine down")
        }

        async fn run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
            anyhow::bail!("engine unreachable")
        }
    }

    fn adapter_with(outcome: JekkoRunOutcome) -> JekkoAdapter {
        JekkoAdapter::new(Arc::new(StubClient { outcome }), "test-model")
    }

    fn ok_outcome() -> JekkoRunOutcome {
        JekkoRunOutcome {
            run_ref: "run-123".to_owned(),
            assistant_text: Some("done".to_owned()),
            artifacts: vec![JekkoArtifact {
                kind: "diff".to_owned(),
                reference: "patch.diff".to_owned(),
                digest: Some("abc".to_owned()),
            }],
            success: true,
            error: None,
        }
    }

    #[tokio::test]
    async fn maps_run_outcome_to_evidence() {
        let adapter = adapter_with(ok_outcome());
        let work_order =
            WorkOrder::submit("t/jekko/e", "jekko.run", json!({"prompt": "fix tests"}));
        let evidence = adapter.execute(&work_order).await.unwrap();
        assert_eq!(evidence[0].kind, "jekko.run");
        assert_eq!(evidence[0].uri, "jekko://run/run-123");
        assert!(evidence
            .iter()
            .any(|e| e.kind == "jekko.assistant.digest" && e.uri.starts_with("sha256:")));
        assert!(evidence
            .iter()
            .any(|e| e.kind == "jekko.artifact.diff" && e.uri == "sha256:abc"));
    }

    #[tokio::test]
    async fn unknown_kind_fails_closed() {
        let adapter = adapter_with(ok_outcome());
        let work_order = WorkOrder::submit("t/jekko/e", "unrelated.kind", json!({}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[tokio::test]
    async fn failed_run_is_fail_closed() {
        let mut outcome = ok_outcome();
        outcome.success = false;
        outcome.error = Some("boom".to_owned());
        let adapter = adapter_with(outcome);
        let work_order = WorkOrder::submit("t/jekko/e", "jekko.run", json!({"prompt": "x"}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[tokio::test]
    async fn unreachable_engine_errors() {
        let adapter = JekkoAdapter::new(Arc::new(DownClient), "m");
        let work_order = WorkOrder::submit("t/jekko/e", "jekko.run", json!({"prompt": "x"}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[test]
    fn service_card_advertises_worker_capabilities() {
        let card = JekkoAdapter::default().service_card();
        assert_eq!(card.name, "jekko");
        assert!(card.capabilities.iter().any(|c| c == "jnoccio-router"));
    }
}
