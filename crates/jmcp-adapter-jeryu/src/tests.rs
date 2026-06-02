use super::*;
use async_trait::async_trait;
use jmcp_adapter_sdk::Adapter;
use jmcp_domain::WorkOrder;
use serde_json::json;
use std::sync::Arc;

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
    let work_order = WorkOrder::submit("t/jeryu/e", "jeryu.snapshot", json!({"run_id": "run-42"}));
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
