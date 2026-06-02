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

    async fn worker_run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
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

    async fn worker_run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
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
    let work_order = WorkOrder::submit("t/jekko/e", "jekko.run", json!({"prompt": "fix tests"}));
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

struct RouteRecordingClient {
    last: std::sync::Mutex<&'static str>,
}

#[async_trait]
impl JekkoClient for RouteRecordingClient {
    async fn health(&self) -> Result<()> {
        Ok(())
    }

    async fn run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
        *self.last.lock().unwrap() = "run";
        Ok(ok_outcome())
    }

    async fn worker_run(&self, _request: JekkoRunRequest) -> Result<JekkoRunOutcome> {
        *self.last.lock().unwrap() = "worker_run";
        Ok(ok_outcome())
    }
}

#[tokio::test]
async fn worker_kinds_route_to_worker_run() {
    for kind in ["jekko.run", "jekko.task", "run", "worker"] {
        let client = Arc::new(RouteRecordingClient {
            last: std::sync::Mutex::new("none"),
        });
        let adapter = JekkoAdapter::new(client.clone(), "m");
        let work_order = WorkOrder::submit("t/jekko/e", kind, json!({"prompt": "x"}));
        adapter.execute(&work_order).await.unwrap();
        assert_eq!(*client.last.lock().unwrap(), "worker_run", "kind {kind}");
    }
}

#[tokio::test]
async fn reason_kind_routes_to_fusion_run() {
    let client = Arc::new(RouteRecordingClient {
        last: std::sync::Mutex::new("none"),
    });
    let adapter = JekkoAdapter::new(client.clone(), "m");
    let work_order = WorkOrder::submit("t/jekko/e", "reason", json!({"prompt": "x"}));
    adapter.execute(&work_order).await.unwrap();
    assert_eq!(*client.last.lock().unwrap(), "run");
}
