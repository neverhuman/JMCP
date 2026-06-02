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

mod config;
mod worker_run;
use config::{
    env_or, DEFAULT_JEKKO_BASE_URL, DEFAULT_JNOCCIO_BASE_URL, DEFAULT_MODEL, DEFAULT_TIMEOUT_SECS,
};
pub use worker_run::map_worker_outcome;

#[cfg(test)]
mod tests;

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
    /// Execute a single fusion-chat reasoning run and return its outcome.
    async fn run(&self, request: JekkoRunRequest) -> Result<JekkoRunOutcome>;
    /// Drive the jnoccio-router `worker_run` autonomous worker (reads/edits a
    /// repo, reports concrete file changes) and return its normalized outcome.
    async fn worker_run(&self, request: JekkoRunRequest) -> Result<JekkoRunOutcome>;
}

/// HTTP client driving Jekko (`:4317`) and jnoccio-fusion (`/v1/chat/completions`).
///
/// Endpoints and the optional bearer token come from the environment and are
/// never logged. See [`HttpJekkoClient::from_env`].
#[derive(Clone)]
pub struct HttpJekkoClient {
    pub(crate) http: reqwest::Client,
    pub(crate) jekko_base_url: String,
    pub(crate) jnoccio_base_url: String,
    pub(crate) api_key: Option<String>,
}

impl HttpJekkoClient {
    /// Build a client from `JEKKO_BASE_URL`, `JNOCCIO_BASE_URL`, and the optional
    /// `JNOCCIO_API_KEY` (sent as a bearer token; never logged).
    pub fn from_env() -> Self {
        let http = match reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
        {
            Ok(client) => client,
            Err(_) => reqwest::Client::new(),
        };
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
        let run_ref = match chat.id {
            Some(id) => id,
            None => "jnoccio".to_owned(),
        };
        Ok(JekkoRunOutcome {
            run_ref,
            assistant_text,
            artifacts: Vec::new(),
            success: true,
            error: None,
        })
    }

    async fn worker_run(&self, request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
        // The autonomous worker path lives in `worker_run.rs`; it builds the
        // JSON-RPC `tools/call` against the router from this client's fields.
        worker_run::run_worker(self, request).await
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
        let kind = work_order.task.kind.as_str();
        let route = route_for(kind).ok_or_else(|| fail_closed("jekko"))?;
        let cwd = match work_order
            .task
            .payload
            .get("cwd")
            .and_then(|value| value.as_str())
        {
            Some(cwd) => cwd.to_owned(),
            None => ".".to_owned(),
        };
        let request = JekkoRunRequest {
            prompt: prompt_for(work_order),
            cwd,
            model: self.model.clone(),
        };
        // Worker kinds drive the autonomous jnoccio-router `worker_run` path;
        // the `reason` kind keeps the fusion-chat reasoning path.
        let outcome = match route {
            Route::Worker => self.client.worker_run(request).await?,
            Route::Reason => self.client.run(request).await?,
        };
        if !outcome.success {
            let error = match outcome.error {
                Some(error) => error,
                None => "unknown error".to_owned(),
            };
            anyhow::bail!("jekko run failed: {}", error);
        }
        Ok(evidence_for(&outcome))
    }
}

/// Which jnoccio surface a work-order kind is dispatched to.
enum Route {
    /// Autonomous jnoccio-router `worker_run` (reads/edits a repo).
    Worker,
    /// jnoccio-fusion OpenAI-compatible chat reasoning.
    Reason,
}

fn route_for(kind: &str) -> Option<Route> {
    match kind {
        "jekko.run" | "jekko.task" | "run" | "worker" => Some(Route::Worker),
        "reason" => Some(Route::Reason),
        _ => None,
    }
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
        let uri = match artifact.digest.as_deref() {
            Some(digest) => format!("sha256:{digest}"),
            None => artifact.reference.clone(),
        };
        evidence.push(Evidence {
            kind: format!("jekko.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
    }
    evidence
}
