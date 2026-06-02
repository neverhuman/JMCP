//! First-class managed-repo nodes for the Jeryu ecosystem snapshot.
//!
//! A [`EcosystemRepo`] is a governed git repository Jeryu actively manages, so
//! the cockpit Tools/Data view can render every repo as a node (not just a tag):
//! its head, conformance score, worst-of health, management status, and how many
//! governed tools live in it.
//!
//! Nodes are built by [`build_repos`]: Jeryu's explicit repo records are
//! preferred (they can carry head/score); otherwise repo nodes are derived from
//! the distinct repos the tools are tagged with.

use crate::ecosystem::EcosystemTool;
use serde::{Deserialize, Serialize};

/// A governed git repository in the Jeryu-managed ecosystem -- a first-class
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

/// Permissive view of a repo record as Jeryu may emit it (all optional).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RawJeryuRepo {
    name: Option<String>,
    head: Option<String>,
    score: Option<u32>,
    conformance: Option<String>,
    health: Option<String>,
    status: Option<String>,
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

/// Count the governed tools tagged with `name`.
fn count_tools(tools: &[EcosystemTool], name: &str) -> u32 {
    tools
        .iter()
        .filter(|t| t.repo.as_deref() == Some(name))
        .count() as u32
}

/// Build repo nodes from Jeryu's explicit repo records, joining tool counts and
/// deriving health when Jeryu omits it. Records without a name are dropped.
fn repos_from_records(raw_repos: Vec<RawJeryuRepo>, tools: &[EcosystemTool]) -> Vec<EcosystemRepo> {
    raw_repos
        .into_iter()
        .filter_map(|r| {
            let name = r.name?;
            let health = r.health.or_else(|| derive_repo_health(tools, &name));
            let tool_count = count_tools(tools, &name);
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
        .collect()
}

/// Derive repo nodes from the distinct repos the tools are tagged with
/// (first-seen order), so "N repos" becomes real nodes.
fn repos_from_tools(tools: &[EcosystemTool]) -> Vec<EcosystemRepo> {
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
            let tool_count = count_tools(tools, &name);
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

/// Build first-class repo nodes: prefer Jeryu's explicit repo records (which can
/// carry head/score), otherwise derive them from the distinct repos the tools
/// are tagged with.
pub(crate) fn build_repos(
    raw_repos: Vec<RawJeryuRepo>,
    tools: &[EcosystemTool],
) -> Vec<EcosystemRepo> {
    if raw_repos.is_empty() {
        repos_from_tools(tools)
    } else {
        repos_from_records(raw_repos, tools)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, repo: &str, health: &str) -> EcosystemTool {
        EcosystemTool {
            name: name.to_owned(),
            class_name: "x".to_owned(),
            conformance: "C1".to_owned(),
            side_effects: "none".to_owned(),
            data_classes: Vec::new(),
            repo: Some(repo.to_owned()),
            provider: None,
            health: Some(health.to_owned()),
            depends_on: Vec::new(),
            queue: None,
        }
    }

    #[test]
    fn derives_repo_nodes_from_tool_tags() {
        let tools = vec![
            tool("jeryu.repo.adopt", "Jeryu", "watch"),
            tool("jekko.run_headless", "Jekko", "degraded"),
        ];
        let repos = build_repos(Vec::new(), &tools);
        assert_eq!(repos.len(), 2, "Jeryu + Jekko become repo nodes");
        let jekko = repos
            .iter()
            .find(|r| r.name == "Jekko")
            .expect("Jekko repo node");
        assert_eq!(jekko.tool_count, 1);
        assert_eq!(jekko.health.as_deref(), Some("degraded"));
        assert_eq!(jekko.status.as_deref(), Some("managed"));
    }

    #[test]
    fn worst_of_health_across_repo_tools() {
        let tools = vec![
            tool("jeryu.repo.adopt", "Jeryu", "watch"),
            tool("jeryu.status", "Jeryu", "nominal"),
        ];
        let repos = build_repos(Vec::new(), &tools);
        assert_eq!(repos.len(), 1);
        let jeryu = &repos[0];
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
    fn explicit_records_carry_head_and_score() {
        let tools = vec![tool("t", "JMCP", "nominal")];
        let raw: Vec<RawJeryuRepo> = serde_json::from_value(serde_json::json!([
            { "name": "JMCP", "head": "main@abc1234", "score": 94, "conformance": "C2", "status": "managed", "health": "nominal" }
        ]))
        .unwrap();
        let repos = build_repos(raw, &tools);
        assert_eq!(repos.len(), 1);
        let repo = &repos[0];
        assert_eq!(repo.score, Some(94));
        assert_eq!(repo.head.as_deref(), Some("main@abc1234"));
        assert_eq!(repo.tool_count, 1, "tool count joined from tools[]");
    }

    #[test]
    fn record_without_name_is_dropped() {
        let raw: Vec<RawJeryuRepo> = serde_json::from_value(serde_json::json!([
            { "head": "main@abc1234" }
        ]))
        .unwrap();
        let repos = build_repos(raw, &[]);
        assert!(repos.is_empty(), "nameless repo record dropped");
    }

    #[test]
    fn explicit_record_derives_health_when_omitted() {
        let tools = vec![tool("t", "JMCP", "degraded")];
        let raw: Vec<RawJeryuRepo> = serde_json::from_value(serde_json::json!([
            { "name": "JMCP" }
        ]))
        .unwrap();
        let repos = build_repos(raw, &tools);
        assert_eq!(repos[0].health.as_deref(), Some("degraded"));
    }
}
