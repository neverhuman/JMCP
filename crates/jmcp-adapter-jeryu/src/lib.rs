//! Jeryu forge adapter.
//!
//! Consumes the read surface of Jeryu, a local GitHub-compatible forge with a
//! REST API (Axum) and an MCP tool catalog, as a JMCP [`Adapter`]. CI run
//! evidence is fetched from the `GET {base}/api/v1/ci/runs/{id}/evidence`
//! surface (equivalent to the `jeryu.get_ci_run_jobs` MCP tool) and liveness
//! from `GET {base}/health`. The HTTP surface is abstracted behind
//! [`JeryuClient`] so the work-order -> evidence mapping is tested
//! deterministically without a live forge. Failures are fail-closed: an
//! unreachable or erroring forge returns an error, never a silent empty result.

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};

mod ecosystem;
mod repos;
#[cfg(test)]
mod tests;

pub use ecosystem::{EcosystemRepo, EcosystemSnapshot, EcosystemTool, JeryuEcosystem};

const DEFAULT_JERYU_BASE_URL: &str = "http://127.0.0.1:8799";
const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// A single artifact attached to a Jeryu CI run (logs, junit, sbom, ...).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct JeryuArtifact {
    pub kind: String,
    pub reference: String,
    #[serde(default)]
    pub digest: Option<String>,
}

/// The normalized evidence bundle for a Jeryu CI run.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct JeryuCiRun {
    pub run_id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub commit_sha: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<JeryuArtifact>,
}

/// Transport abstraction over the Jeryu forge read surface.
///
/// Implemented by [`HttpJeryuClient`] at runtime and by deterministic stubs in
/// tests, so the adapter's mapping logic is verifiable without a live forge.
#[async_trait]
pub trait JeryuClient: Send + Sync {
    /// Liveness probe for the forge.
    async fn health(&self) -> Result<()>;
    /// Fetch the CI evidence bundle for a run id.
    async fn ci_run_evidence(&self, run_id: &str) -> Result<JeryuCiRun>;
}

/// HTTP client driving the Jeryu forge REST API (`:8799`).
///
/// The base URL and the optional bearer token come from the environment and are
/// never logged. See [`HttpJeryuClient::from_env`].
#[derive(Clone)]
pub struct HttpJeryuClient {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) api_key: Option<String>,
}

impl HttpJeryuClient {
    /// Build a client from `JERYU_BASE_URL` and the optional `JERYU_API_KEY`
    /// (sent as a bearer token; never logged).
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
            base_url: env_or("JERYU_BASE_URL", DEFAULT_JERYU_BASE_URL),
            api_key: match std::env::var("JERYU_API_KEY").ok() {
                Some(value) if !value.is_empty() => Some(value),
                _ => None,
            },
        }
    }

    pub(crate) fn authed(&self, builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(key) = &self.api_key {
            builder.bearer_auth(key)
        } else {
            builder
        }
    }
}

#[async_trait]
impl JeryuClient for HttpJeryuClient {
    async fn health(&self) -> Result<()> {
        let url = format!("{}/health", self.base_url.trim_end_matches('/'));
        let response = self
            .authed(self.http.get(&url))
            .send()
            .await
            .context("jeryu health request failed")?;
        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("jeryu health returned {}", response.status());
        }
    }

    async fn ci_run_evidence(&self, run_id: &str) -> Result<JeryuCiRun> {
        let url = format!(
            "{}/api/v1/ci/runs/{}/evidence",
            self.base_url.trim_end_matches('/'),
            run_id
        );
        let response = self
            .authed(self.http.get(&url))
            .send()
            .await
            .context("jeryu ci evidence request failed")?;
        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("jeryu ci evidence returned {status}");
        }
        response
            .json()
            .await
            .context("jeryu ci evidence response was not valid JSON")
    }
}

/// JMCP adapter that reads CI evidence from the Jeryu forge.
pub struct JeryuAdapter {
    client: Arc<dyn JeryuClient>,
}

impl Default for JeryuAdapter {
    fn default() -> Self {
        Self::new(Arc::new(HttpJeryuClient::from_env()))
    }
}

impl JeryuAdapter {
    /// Build an adapter over an arbitrary [`JeryuClient`] (used by tests to
    /// inject a deterministic stub).
    pub fn new(client: Arc<dyn JeryuClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Adapter for JeryuAdapter {
    fn service_card(&self) -> ServiceCard {
        ServiceCard {
            name: "jeryu".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            subjects: vec!["*/jeryu/*".to_owned()],
            capabilities: vec![
                "forge".to_owned(),
                "ci-evidence".to_owned(),
                "snapshot".to_owned(),
            ],
        }
    }

    async fn execute(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        if !is_forge_kind(&work_order.task.kind) {
            return Err(fail_closed("jeryu"));
        }
        // Liveness is checked first so an unreachable forge fails closed rather
        // than surfacing a confusing not-found from the evidence read.
        self.client.health().await?;
        let run_id = run_id_for(work_order)?;
        let run = self.client.ci_run_evidence(&run_id).await?;
        Ok(evidence_for(&run))
    }
}

fn is_forge_kind(kind: &str) -> bool {
    matches!(kind, "jeryu.ci" | "jeryu.evidence" | "jeryu.snapshot")
}

fn run_id_for(work_order: &WorkOrder) -> Result<String> {
    match non_empty_owned(
        work_order
            .task
            .payload
            .get("run_id")
            .and_then(|value| value.as_str())
            .map(str::to_owned),
    ) {
        Some(value) => Ok(value),
        None => anyhow::bail!("jeryu work order missing run_id"),
    }
}

fn evidence_for(run: &JeryuCiRun) -> Vec<Evidence> {
    let now = Utc::now();
    let mut evidence = vec![Evidence {
        kind: "jeryu.ci-run".to_owned(),
        uri: format!("jeryu://ci/run/{}", run.run_id),
        captured_at: now,
    }];
    if let Some(commit) = &run.commit_sha {
        evidence.push(Evidence {
            kind: "jeryu.ci-run.commit".to_owned(),
            uri: format!("jeryu://commit/{commit}"),
            captured_at: now,
        });
    }
    for artifact in &run.artifacts {
        let uri = match artifact.digest.as_deref() {
            Some(digest) => format!("sha256:{digest}"),
            None => artifact.reference.clone(),
        };
        evidence.push(Evidence {
            kind: format!("jeryu.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
    }
    // A deterministic digest over the run identity binds the evidence set to a
    // concrete forge state for replay/audit.
    let digest = hex::encode(Sha256::digest(
        match run.status.as_deref() {
            Some(status) => format!("{}|{}", run.run_id, status),
            None => format!("{}|", run.run_id),
        }
        .as_bytes(),
    ));
    evidence.push(Evidence {
        kind: "jeryu.ci-run.digest".to_owned(),
        uri: format!("sha256:{digest}"),
        captured_at: now,
    });
    evidence
}

fn env_or(key: &str, default: &str) -> String {
    match std::env::var(key).ok() {
        Some(value) if !value.is_empty() => value,
        _ => default.to_owned(),
    }
}

fn non_empty_owned(value: Option<String>) -> Option<String> {
    match value {
        Some(value) => Some(value),
        _ => None,
    }
}
