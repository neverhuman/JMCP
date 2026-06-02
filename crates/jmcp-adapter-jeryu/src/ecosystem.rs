//! Jeryu ecosystem snapshot producer.
//!
//! Read-only projection of the Jeryu forge's discovery/status surface into the
//! JMCP cockpit "Tools/Data" graph shape ([`EcosystemTool`], which serializes to
//! the cockpit `ToolAsset` type in `apps/cockpit/src/types.ts`): tools across
//! every repo, their dependency edges, queue depth, and health.
//!
//! Degradation is **explicit** and evidence-backed, never silent or faked:
//! - Jeryu unreachable / missing the endpoint -> [`EcosystemSnapshot::degraded`]
//!   with `live=false` and a human-readable `degraded_reason` (the cockpit
//!   renders the explicit `degradedReason` instead of an empty or invented
//!   graph).
//! - A malformed tool record (missing required fields) is kept but marked
//!   `health = "degraded"` with explicit placeholders, and the snapshot records
//!   how many records were degraded -- records are never dropped or invented.

use crate::HttpJeryuClient;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// A governed tool/asset node in the ecosystem graph.
///
/// Serializes to the cockpit `ToolAsset` shape (camelCase) consumed by
/// `apps/cockpit/src/types.ts`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EcosystemTool {
    pub name: String,
    pub class_name: String,
    pub conformance: String,
    pub side_effects: String,
    pub data_classes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue: Option<u32>,
}

/// A governed git repository in the Jeryu-managed ecosystem — a first-class
/// node so the cockpit can show every repo Jeryu actively manages, its head,
/// health, conformance score, and how many governed tools live in it.
///
/// Serializes camelCase (`toolCount`) for the cockpit.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EcosystemRepo {
    pub name: String,
    /// Current branch/commit Jeryu has checked out, when reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<String>,
    /// Jankurai conformance score (0-100) when Jeryu reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conformance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<String>,
    /// Management status, e.g. "managed" or "adopting".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Number of governed tools that live in this repo.
    pub tool_count: u32,
}

/// The full ecosystem snapshot: every governed tool across repos, the managed
/// repos themselves, plus the dependency edges that relate the tools.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct EcosystemSnapshot {
    pub tools: Vec<EcosystemTool>,
    /// The git repos Jeryu actively manages (first-class nodes). Derived from
    /// tool tags when Jeryu sends no explicit repo records.
    #[serde(default)]
    pub repos: Vec<EcosystemRepo>,
    /// `true` when produced from a live Jeryu response; `false` when degraded.
    pub live: bool,
    /// Explicit degradation note; empty only when fully live and well-formed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub degraded_reason: String,
}

impl EcosystemSnapshot {
    /// An explicitly-degraded snapshot (no tools) carrying a human-readable
    /// reason. Used when Jeryu is unreachable, missing the endpoint, or returns
    /// an unparseable body -- the cockpit renders the reason instead of guessing.
    pub fn degraded(reason: impl Into<String>) -> Self {
        Self {
            tools: Vec::new(),
            repos: Vec::new(),
            live: false,
            degraded_reason: reason.into(),
        }
    }
}

/// Permissive view of a tool record as Jeryu may emit it. Every field is
/// optional so a thin or evolving payload still deserializes; missing required
/// fields are surfaced as explicit degradation rather than a parse failure.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawJeryuTool {
    name: Option<String>,
    #[serde(alias = "class")]
    class_name: Option<String>,
    conformance: Option<String>,
    side_effects: Option<String>,
    #[serde(default)]
    data_classes: Vec<String>,
    repo: Option<String>,
    provider: Option<String>,
    health: Option<String>,
    #[serde(default)]
    depends_on: Vec<String>,
    queue: Option<u32>,
}

/// Permissive view of a repo record as Jeryu may emit it (all optional).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawJeryuRepo {
    name: Option<String>,
    head: Option<String>,
    score: Option<u32>,
    conformance: Option<String>,
    health: Option<String>,
    status: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEcosystem {
    #[serde(default)]
    tools: Vec<RawJeryuTool>,
    #[serde(default)]
    repos: Vec<RawJeryuRepo>,
}

/// Worst-of health across the tools that belong to `repo` (degraded > watch >
/// nominal), or `None` when the repo has no tools.
fn derive_repo_health(tools: &[EcosystemTool], repo: &str) -> Option<String> {
    let mut worst = 0u8;
    let mut any = false;
    for tool in tools.iter().filter(|t| t.repo.as_deref() == Some(repo)) {
        any = true;
        let rank = match tool.health.as_deref() {
            Some("degraded") | Some("blocked") => 2,
            Some("watch") => 1,
            _ => 0,
        };
        worst = worst.max(rank);
    }
    if !any {
        return None;
    }
    Some(
        match worst {
            2 => "degraded",
            1 => "watch",
            _ => "nominal",
        }
        .to_owned(),
    )
}

/// Build first-class repo nodes: prefer Jeryu's explicit repo records (which can
/// carry head/score), otherwise derive them from the distinct repos the tools
/// are tagged with (first-seen order), so "N repos" becomes real nodes.
fn build_repos(raw_repos: Vec<RawJeryuRepo>, tools: &[EcosystemTool]) -> Vec<EcosystemRepo> {
    let count_tools = |name: &str| {
        tools
            .iter()
            .filter(|t| t.repo.as_deref() == Some(name))
            .count() as u32
    };
    if !raw_repos.is_empty() {
        return raw_repos
            .into_iter()
            .filter_map(|r| {
                let name = r.name?;
                let health = r.health.or_else(|| derive_repo_health(tools, &name));
                let tool_count = count_tools(&name);
                Some(EcosystemRepo {
                    head: r.head,
                    score: r.score,
                    conformance: r.conformance,
                    status: r.status,
                    health,
                    tool_count,
                    name,
                })
            })
            .collect();
    }
    let mut seen: Vec<String> = Vec::new();
    for tool in tools {
        if let Some(repo) = &tool.repo {
            if !seen.iter().any(|s| s == repo) {
                seen.push(repo.clone());
            }
        }
    }
    seen.into_iter()
        .map(|name| {
            let health = derive_repo_health(tools, &name);
            let tool_count = count_tools(&name);
            EcosystemRepo {
                head: None,
                score: None,
                conformance: None,
                status: Some("managed".to_owned()),
                health,
                tool_count,
                name,
            }
        })
        .collect()
}

/// Normalize a raw Jeryu ecosystem payload into the cockpit shape, marking any
/// record with missing required fields as `degraded` (kept, never dropped).
fn normalize(raw: RawEcosystem) -> EcosystemSnapshot {
    let mut tools = Vec::with_capacity(raw.tools.len());
    let mut degraded_count = 0usize;
    for record in raw.tools {
        let is_degraded = record.name.is_none() || record.class_name.is_none();
        if is_degraded {
            degraded_count += 1;
        }
        let health = if is_degraded {
            "degraded".to_owned()
        } else {
            record.health.unwrap_or_else(|| "nominal".to_owned())
        };
        tools.push(EcosystemTool {
            name: record.name.unwrap_or_else(|| "(unknown)".to_owned()),
            class_name: record.class_name.unwrap_or_else(|| "unknown".to_owned()),
            conformance: record.conformance.unwrap_or_else(|| "unknown".to_owned()),
            side_effects: record.side_effects.unwrap_or_else(|| "unknown".to_owned()),
            data_classes: record.data_classes,
            repo: record.repo,
            provider: record.provider,
            health: Some(health),
            depends_on: record.depends_on,
            queue: record.queue,
        });
    }
    let repos = build_repos(raw.repos, &tools);
    let live = !tools.is_empty();
    let degraded_reason = if !live {
        "jeryu returned no ecosystem tools".to_owned()
    } else if degraded_count > 0 {
        format!("{degraded_count} tool record(s) had missing fields and were marked degraded")
    } else {
        String::new()
    };
    EcosystemSnapshot {
        tools,
        repos,
        live,
        degraded_reason,
    }
}

/// Read-only producer of the Jeryu ecosystem graph.
///
/// Implemented by [`HttpJeryuClient`] (reads `GET {base}/api/v1/ecosystem`) and
/// by deterministic stubs in tests.
#[async_trait]
pub trait JeryuEcosystem: Send + Sync {
    async fn ecosystem(&self) -> Result<EcosystemSnapshot>;
}

#[async_trait]
impl JeryuEcosystem for HttpJeryuClient {
    async fn ecosystem(&self) -> Result<EcosystemSnapshot> {
        let url = format!("{}/api/v1/ecosystem", self.base_url.trim_end_matches('/'));
        let response = match self.authed(self.http.get(&url)).send().await {
            Ok(response) => response,
            Err(error) => {
                return Ok(EcosystemSnapshot::degraded(format!(
                    "jeryu unreachable: {error}"
                )))
            }
        };
        let status = response.status();
        if !status.is_success() {
            return Ok(EcosystemSnapshot::degraded(format!(
                "jeryu ecosystem endpoint unavailable: {status}"
            )));
        }
        match response.json::<RawEcosystem>().await {
            Ok(raw) => Ok(normalize(raw)),
            Err(error) => Ok(EcosystemSnapshot::degraded(format!(
                "jeryu ecosystem response malformed: {error}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn raw_from(value: serde_json::Value) -> RawEcosystem {
        serde_json::from_value(value).expect("raw ecosystem deserializes")
    }

    #[test]
    fn normalizes_healthy_multi_repo_graph_with_relationships() {
        let snapshot = normalize(raw_from(json!({
            "tools": [
                {
                    "name": "jeryu.repo.adopt",
                    "className": "repository governance",
                    "conformance": "C1 constrained",
                    "sideEffects": "local git remote",
                    "dataClasses": ["repo", "policy"],
                    "repo": "Jeryu",
                    "provider": "jeryu",
                    "health": "watch",
                    "dependsOn": ["git.remote"],
                    "queue": 1
                },
                {
                    "name": "jekko.run_headless",
                    "className": "worker execution",
                    "conformance": "C1 leased",
                    "sideEffects": "tool calls",
                    "dataClasses": ["prompt", "diff"],
                    "repo": "Jekko",
                    "provider": "jekko",
                    "health": "degraded",
                    "dependsOn": ["jeryu.repo.adopt"],
                    "queue": 0
                }
            ]
        })));
        assert!(snapshot.live);
        assert_eq!(snapshot.degraded_reason, "");
        assert_eq!(snapshot.tools.len(), 2);
        let jekko = snapshot
            .tools
            .iter()
            .find(|t| t.name == "jekko.run_headless")
            .expect("jekko tool present");
        assert_eq!(jekko.repo.as_deref(), Some("Jekko"));
        assert!(jekko.depends_on.iter().any(|d| d == "jeryu.repo.adopt"));
        // First-class repo nodes are derived from the tools' repo tags.
        assert_eq!(snapshot.repos.len(), 2, "Jeryu + Jekko become repo nodes");
        let jekko_repo = snapshot
            .repos
            .iter()
            .find(|r| r.name == "Jekko")
            .expect("Jekko repo node");
        assert_eq!(jekko_repo.tool_count, 1);
        assert_eq!(jekko_repo.health.as_deref(), Some("degraded"));
        assert_eq!(jekko_repo.status.as_deref(), Some("managed"));
        let wire = serde_json::to_value(&snapshot.tools[0]).unwrap();
        assert!(wire.get("className").is_some());
        assert!(wire.get("sideEffects").is_some());
        assert!(wire.get("dataClasses").is_some());
    }

    #[test]
    fn derives_repo_nodes_with_worst_of_health() {
        let snapshot = normalize(raw_from(json!({
            "tools": [
                { "name": "jeryu.repo.adopt", "className": "x", "conformance": "C1", "sideEffects": "git", "repo": "Jeryu", "health": "watch" },
                { "name": "jeryu.status", "className": "x", "conformance": "C1", "sideEffects": "none", "repo": "Jeryu", "health": "nominal" }
            ]
        })));
        assert_eq!(snapshot.repos.len(), 1);
        let jeryu = &snapshot.repos[0];
        assert_eq!(jeryu.name, "Jeryu");
        assert_eq!(jeryu.tool_count, 2);
        assert_eq!(
            jeryu.health.as_deref(),
            Some("watch"),
            "worst-of watch+nominal"
        );
        // Wire shape is camelCase for the cockpit.
        let wire = serde_json::to_value(jeryu).unwrap();
        assert!(wire.get("toolCount").is_some());
    }

    #[test]
    fn uses_explicit_jeryu_repo_records_with_head_and_score() {
        let snapshot = normalize(raw_from(json!({
            "tools": [
                { "name": "t", "className": "x", "conformance": "C1", "sideEffects": "none", "repo": "JMCP" }
            ],
            "repos": [
                { "name": "JMCP", "head": "main@abc1234", "score": 94, "conformance": "C2", "status": "managed", "health": "nominal" }
            ]
        })));
        assert_eq!(snapshot.repos.len(), 1);
        let repo = &snapshot.repos[0];
        assert_eq!(repo.score, Some(94));
        assert_eq!(repo.head.as_deref(), Some("main@abc1234"));
        assert_eq!(repo.tool_count, 1, "tool count joined from tools[]");
    }

    #[test]
    fn malformed_record_is_marked_degraded_not_dropped() {
        let snapshot = normalize(raw_from(json!({
            "tools": [
                { "name": "ok.tool", "className": "evidence", "conformance": "C2", "sideEffects": "none" },
                { "conformance": "C0", "sideEffects": "none" }
            ]
        })));
        assert!(snapshot.live);
        assert_eq!(
            snapshot.tools.len(),
            2,
            "malformed record kept, not dropped"
        );
        assert!(snapshot.degraded_reason.contains("missing fields"));
        let bad = snapshot
            .tools
            .iter()
            .find(|t| t.name == "(unknown)")
            .expect("degraded placeholder present");
        assert_eq!(bad.health.as_deref(), Some("degraded"));
        assert_eq!(bad.class_name, "unknown");
    }

    #[test]
    fn empty_payload_degrades_explicitly() {
        let snapshot = normalize(raw_from(json!({ "tools": [] })));
        assert!(!snapshot.live);
        assert!(snapshot.tools.is_empty());
        assert_eq!(
            snapshot.degraded_reason,
            "jeryu returned no ecosystem tools"
        );
    }

    #[test]
    fn degraded_constructor_is_explicit() {
        let snapshot = EcosystemSnapshot::degraded("jeryu unreachable: connection refused");
        assert!(!snapshot.live);
        assert!(snapshot.tools.is_empty());
        assert!(snapshot.degraded_reason.starts_with("jeryu unreachable"));
    }

    #[tokio::test]
    async fn absent_forge_degrades_explicitly() {
        std::env::set_var("JERYU_BASE_URL", "http://127.0.0.1:1");
        let client = HttpJeryuClient::from_env();
        std::env::remove_var("JERYU_BASE_URL");
        let snapshot = client.ecosystem().await.expect("degrades, never errors");
        assert!(!snapshot.live);
        assert!(snapshot.tools.is_empty());
        assert!(snapshot.degraded_reason.starts_with("jeryu unreachable"));
    }
}
