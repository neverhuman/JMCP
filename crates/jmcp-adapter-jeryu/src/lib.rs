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
pub use ecosystem::{EcosystemSnapshot, EcosystemTool, JeryuEcosystem};

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
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            base_url: env_or("JERYU_BASE_URL", DEFAULT_JERYU_BASE_URL),
            api_key: std::env::var("JERYU_API_KEY")
                .ok()
                .filter(|value| !value.is_empty()),
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
    work_order
        .task
        .payload
        .get("run_id")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_owned())
        .context("jeryu work order missing run_id")
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
        let uri = artifact
            .digest
            .clone()
            .map(|digest| format!("sha256:{digest}"))
            .unwrap_or_else(|| artifact.reference.clone());
        evidence.push(Evidence {
            kind: format!("jeryu.artifact.{}", artifact.kind),
            uri,
            captured_at: now,
        });
    }
    // A deterministic digest over the run identity binds the evidence set to a
    // concrete forge state for replay/audit.
    let digest = hex::encode(Sha256::digest(
        format!("{}|{}", run.run_id, run.status.clone().unwrap_or_default()).as_bytes(),
    ));
    evidence.push(Evidence {
        kind: "jeryu.ci-run.digest".to_owned(),
        uri: format!("sha256:{digest}"),
        captured_at: now,
    });
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
        run: JeryuCiRun,
    }

    #[async_trait]
    impl JeryuClient for StubClient {
        async fn health(&self) -> Result<()> {
            Ok(())
        }

        async fn ci_run_evidence(&self, _run_id: &str) -> Result<JeryuCiRun> {
            Ok(self.run.clone())
        }
    }

    struct DownClient;

    #[async_trait]
    impl JeryuClient for DownClient {
        async fn health(&self) -> Result<()> {
            anyhow::bail!("forge down")
        }

        async fn ci_run_evidence(&self, _run_id: &str) -> Result<JeryuCiRun> {
            anyhow::bail!("forge unreachable")
        }
    }

    fn adapter_with(run: JeryuCiRun) -> JeryuAdapter {
        JeryuAdapter::new(Arc::new(StubClient { run }))
    }

    fn ok_run() -> JeryuCiRun {
        JeryuCiRun {
            run_id: "run-42".to_owned(),
            status: Some("success".to_owned()),
            commit_sha: Some("deadbeef".to_owned()),
            artifacts: vec![JeryuArtifact {
                kind: "junit".to_owned(),
                reference: "junit.xml".to_owned(),
                digest: Some("abc".to_owned()),
            }],
        }
    }

    #[tokio::test]
    async fn maps_ci_run_to_evidence() {
        let adapter = adapter_with(ok_run());
        let work_order = WorkOrder::submit("t/jeryu/e", "jeryu.ci", json!({"run_id": "run-42"}));
        let evidence = adapter.execute(&work_order).await.unwrap();
        assert_eq!(evidence[0].kind, "jeryu.ci-run");
        assert_eq!(evidence[0].uri, "jeryu://ci/run/run-42");
        assert!(evidence
            .iter()
            .any(|e| e.kind == "jeryu.ci-run.commit" && e.uri == "jeryu://commit/deadbeef"));
        assert!(evidence
            .iter()
            .any(|e| e.kind == "jeryu.artifact.junit" && e.uri == "sha256:abc"));
        assert!(evidence
            .iter()
            .any(|e| e.kind == "jeryu.ci-run.digest" && e.uri.starts_with("sha256:")));
    }

    #[tokio::test]
    async fn evidence_snapshot_kind_is_accepted() {
        let adapter = adapter_with(ok_run());
        let work_order =
            WorkOrder::submit("t/jeryu/e", "jeryu.snapshot", json!({"run_id": "run-42"}));
        assert!(adapter.execute(&work_order).await.is_ok());
    }

    #[tokio::test]
    async fn unknown_kind_fails_closed() {
        let adapter = adapter_with(ok_run());
        let work_order = WorkOrder::submit("t/jeryu/e", "unrelated.kind", json!({}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[tokio::test]
    async fn missing_run_id_fails_closed() {
        let adapter = adapter_with(ok_run());
        let work_order = WorkOrder::submit("t/jeryu/e", "jeryu.ci", json!({}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[tokio::test]
    async fn unreachable_forge_errors() {
        let adapter = JeryuAdapter::new(Arc::new(DownClient));
        let work_order = WorkOrder::submit("t/jeryu/e", "jeryu.ci", json!({"run_id": "run-42"}));
        assert!(adapter.execute(&work_order).await.is_err());
    }

    #[test]
    fn service_card_advertises_forge_capabilities() {
        let card = JeryuAdapter::default().service_card();
        assert_eq!(card.name, "jeryu");
        assert!(card.capabilities.iter().any(|c| c == "ci-evidence"));
    }
}
