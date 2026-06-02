use crate::universe::compose_universe;
use jmcp_adapter_jeryu::{EcosystemSnapshot, EcosystemTool};
use jmcp_domain::{HealthLevel, Lease, ServiceCard, SystemStatus, WorkOrder};
use serde_json::json;

fn live_ecosystem() -> EcosystemSnapshot {
    EcosystemSnapshot {
        tools: vec![
            EcosystemTool {
                name: "jeryu.repo.adopt".to_owned(),
                class_name: "repository governance".to_owned(),
                conformance: "C1 constrained".to_owned(),
                side_effects: "local git remote".to_owned(),
                data_classes: vec!["repo".to_owned(), "policy".to_owned()],
                repo: Some("Jeryu".to_owned()),
                provider: Some("jeryu".to_owned()),
                health: Some("watch".to_owned()),
                depends_on: vec!["git.remote".to_owned()],
                queue: Some(1),
            },
            EcosystemTool {
                name: "jekko.run_headless".to_owned(),
                class_name: "worker execution".to_owned(),
                conformance: "C1 leased".to_owned(),
                side_effects: "tool calls".to_owned(),
                data_classes: vec!["prompt".to_owned(), "diff".to_owned()],
                repo: Some("Jekko".to_owned()),
                provider: Some("jekko".to_owned()),
                health: Some("nominal".to_owned()),
                depends_on: vec!["jeryu.repo.adopt".to_owned()],
                queue: Some(0),
            },
            EcosystemTool {
                name: "jailgun.run_agent".to_owned(),
                class_name: "bounded agent execution".to_owned(),
                conformance: "C1 leased".to_owned(),
                side_effects: "Jailgun HTTP ingest".to_owned(),
                data_classes: vec!["prompt_ref".to_owned(), "receipts".to_owned()],
                repo: Some("Jailgun".to_owned()),
                provider: Some("jailgun".to_owned()),
                health: Some("nominal".to_owned()),
                depends_on: vec!["jailgun.api.runs".to_owned()],
                queue: Some(0),
            },
        ],
        repos: Vec::new(),
        live: true,
        degraded_reason: String::new(),
    }
}

fn repo_work_order(repo: &str, branch: &str, pool: &str) -> WorkOrder {
    WorkOrder::submit(
        format!("repo/{repo}/main"),
        format!("{repo}.sync"),
        json!({
            "repo": repo,
            "branch": branch,
            "pool": pool
        }),
    )
}

#[test]
fn universe_payload_combines_bootstrap_and_ecosystem() {
    let jeryu = repo_work_order("Jeryu", "main", "jeryu-pool");
    let jekko = repo_work_order("Jekko", "jmcp/bridge-quarantine", "jekko-pool");
    let jankurai = repo_work_order("Jankurai", "policy/replay-ratchet", "jankurai-pool");
    let jailgun = repo_work_order("Jailgun", "main", "jailgun-pool");
    let payload = compose_universe(
        vec![
            SystemStatus {
                name: "jeryu".to_owned(),
                role: "evidence runner".to_owned(),
                health: HealthLevel::Watch,
                jcp: "1.0.0".to_owned(),
                latency: "42ms".to_owned(),
            },
            SystemStatus {
                name: "jekko".to_owned(),
                role: "headless worker".to_owned(),
                health: HealthLevel::Nominal,
                jcp: "1.0.0".to_owned(),
                latency: "25ms".to_owned(),
            },
            SystemStatus {
                name: "jankurai".to_owned(),
                role: "standards memory".to_owned(),
                health: HealthLevel::Nominal,
                jcp: "1.0.0".to_owned(),
                latency: "local-cli".to_owned(),
            },
            SystemStatus {
                name: "jailgun".to_owned(),
                role: "bounded ChatGPT capture".to_owned(),
                health: HealthLevel::Nominal,
                jcp: "adapter".to_owned(),
                latency: "http://127.0.0.1:8787".to_owned(),
            },
        ],
        vec![
            ServiceCard {
                name: "jeryu".to_owned(),
                version: "0.1.0".to_owned(),
                subjects: vec!["*/jeryu/*".to_owned()],
                capabilities: vec!["health".to_owned()],
            },
            ServiceCard {
                name: "jekko".to_owned(),
                version: "0.1.0".to_owned(),
                subjects: vec!["*/jekko/*".to_owned()],
                capabilities: vec!["headless".to_owned()],
            },
            ServiceCard {
                name: "jankurai".to_owned(),
                version: "0.1.0".to_owned(),
                subjects: vec!["*/jankurai/*".to_owned()],
                capabilities: vec!["local-cli".to_owned()],
            },
            ServiceCard {
                name: "jailgun".to_owned(),
                version: "0.1.0".to_owned(),
                subjects: vec!["*/jailgun/*".to_owned()],
                capabilities: vec!["run-agent".to_owned(), "review-packet".to_owned()],
            },
        ],
        vec![
            jeryu.clone(),
            jekko.clone(),
            jankurai.clone(),
            jailgun.clone(),
        ],
        vec![
            Lease {
                work_order_id: jeryu.id,
                holder: "jeryu-pool".to_owned(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            },
            Lease {
                work_order_id: jekko.id,
                holder: "jekko-pool".to_owned(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            },
            Lease {
                work_order_id: jankurai.id,
                holder: "jankurai-pool".to_owned(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            },
            Lease {
                work_order_id: jailgun.id,
                holder: "jailgun-pool".to_owned(),
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
            },
        ],
        live_ecosystem(),
    );

    assert!(payload.live);
    assert_eq!(payload.bootstrap_tui.observed_coverage, 100);
    assert!(payload
        .bootstrap_tui
        .active_repos
        .iter()
        .any(|repo| repo.repo == "Jeryu"));
    let jeryu = payload
        .bootstrap_tui
        .repo_scores
        .iter()
        .find(|repo| repo.repo == "Jeryu")
        .expect("jeryu repo score present");
    assert_eq!(jeryu.current_task, "Jeryu.sync");
    assert_eq!(jeryu.branch, "main");
    assert_eq!(jeryu.pool, "jeryu-pool");
    assert!(payload
        .bootstrap_tui
        .placements
        .iter()
        .any(|placement| placement.agent == "Jeryu" && placement.branch == "main"));
    assert!(payload
        .bootstrap_tui
        .active_repos
        .iter()
        .any(|repo| repo.repo == "Jailgun"));
}

#[test]
fn universe_payload_reports_degraded_slices() {
    let payload = compose_universe(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        EcosystemSnapshot::degraded("jeryu unreachable: connection refused"),
    );

    assert!(!payload.live);
    assert!(!payload.bootstrap_tui.live);
    assert!(payload
        .bootstrap_tui
        .degraded_reason
        .as_deref()
        .expect("bootstrap reason")
        .contains("current task not observed"));
    assert!(payload
        .bootstrap_tui
        .degraded_slices
        .iter()
        .any(|slice| slice.name == "ecosystem"
            && slice
                .degraded_reason
                .as_deref()
                .unwrap_or_default()
                .contains("connection refused")));
}
