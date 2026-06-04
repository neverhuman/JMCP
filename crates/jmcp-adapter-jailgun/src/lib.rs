//! Jailgun local worker adapter.
//!
//! This adapter keeps JMCP as the owner of work orders, leases, evidence, and
//! effect replay. Jailgun run execution is submitted through the Jailgun HTTP
//! ingest endpoint; review packets stay on the bounded CLI path until Jailgun
//! exposes an equivalent HTTP surface.

mod cli;
mod evidence;
mod http_ingest;
mod model;
mod protocol;
mod review_packet;

use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use jmcp_adapter_sdk::{fail_closed, Adapter};
use jmcp_domain::{Evidence, ServiceCard, WorkOrder};
use tokio::process::Command;

use cli::run_checked;
use evidence::{evidence_for_summary, file_uri};
use protocol::run_agent_request;
use review_packet::{review_packet_evidence, review_packet_request};

#[cfg(test)]
pub(crate) use http_ingest::{jailgun_allowed_policy_env, validate_jailgun_client_config};
pub(crate) use http_ingest::{HttpJailgunRunClient, JailgunRunClient};
#[cfg(test)]
pub(crate) use model::{JailgunAcceptedRun, JailgunArtifact, JailgunSummary};

#[derive(Clone, Debug)]
pub struct JailgunAdapter {
    command: PathBuf,
    timeout: Duration,
    run_client: Arc<dyn JailgunRunClient>,
}

impl Default for JailgunAdapter {
    fn default() -> Self {
        Self {
            command: std::env::var_os("JMCP_JAILGUN_BIN")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("jailgun")),
            timeout: Duration::from_secs(30 * 60 + 30),
            run_client: Arc::new(HttpJailgunRunClient::from_env()),
        }
    }
}

impl JailgunAdapter {
    pub fn new(command: impl Into<PathBuf>, timeout: Duration) -> Self {
        Self {
            command: command.into(),
            timeout,
            run_client: Arc::new(HttpJailgunRunClient::from_env()),
        }
    }

    #[cfg(test)]
    fn with_run_client(
        command: impl Into<PathBuf>,
        timeout: Duration,
        run_client: Arc<dyn JailgunRunClient>,
    ) -> Self {
        Self {
            command: command.into(),
            timeout,
            run_client,
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
        let request = run_agent_request(&work_order.task.payload)?;
        let accepted = self.run_client.start_run(&request).await?;
        let summary = self
            .run_client
            .wait_for_summary(&accepted.summary_url, self.timeout)
            .await?;
        let summary_uri = self.run_client.summary_uri(&accepted.summary_url)?;
        let events_uri = file_uri(&PathBuf::from(&accepted.events_jsonl));

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
        Ok(evidence_for_summary(&summary, &summary_uri, &events_uri))
    }

    async fn execute_review_packet(&self, work_order: &WorkOrder) -> Result<Vec<Evidence>> {
        let request = review_packet_request(&work_order.task.payload)?;
        let mut command = Command::new(&self.command);
        command
            .arg("review-packet")
            .arg("--summary-json")
            .arg(&request.summary_json)
            .arg("--base")
            .arg(request.base)
            .arg("--head")
            .arg(request.head)
            .arg("--repo")
            .arg(request.repo)
            .arg("--output")
            .arg(&request.output)
            .arg("--patch-bytes")
            .arg(request.patch_bytes.to_string())
            .current_dir(request.cwd);

        run_checked(&self.command, self.timeout, command, "review-packet").await?;
        review_packet_evidence(&request.output)
    }
}

#[cfg(test)]
#[path = "jailgun_tests.rs"]
mod tests;
