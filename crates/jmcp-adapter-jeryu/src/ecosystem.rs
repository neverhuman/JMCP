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

pub use crate::repos::EcosystemRepo;
use crate::repos::{build_repos, RawJeryuRepo};

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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawEcosystem {
    #[serde(default)]
    tools: Vec<RawJeryuTool>,
    #[serde(default)]
    repos: Vec<RawJeryuRepo>,
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
        let wire = serde_json::to_value(&snapshot.tools[0]).unwrap();
        assert!(wire.get("className").is_some());
        assert!(wire.get("sideEffects").is_some());
        assert!(wire.get("dataClasses").is_some());
    }

    #[test]
    fn normalize_wires_first_class_repo_nodes() {
        let snapshot = normalize(raw_from(json!({
            "tools": [
                { "name": "t", "className": "x", "conformance": "C1", "sideEffects": "none", "repo": "JMCP" }
            ]
        })));
        assert_eq!(snapshot.repos.len(), 1, "repo node derived from tool tag");
        assert_eq!(snapshot.repos[0].name, "JMCP");
        assert_eq!(snapshot.repos[0].tool_count, 1);
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
