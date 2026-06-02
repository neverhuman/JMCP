use crate::universe_helpers::*;
use jmcp_adapter_jeryu::EcosystemSnapshot;
use jmcp_domain::{HealthLevel, Lease, ServiceCard, SystemStatus, WorkOrder};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniverseSliceObservation {
    pub(crate) name: String,
    pub(crate) live: bool,
    pub(crate) coverage: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniverseActiveRepo {
    pub(crate) repo: String,
    pub(crate) tool_count: usize,
    pub(crate) score: u8,
    pub(crate) health: HealthLevel,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniverseRepoScore {
    pub(crate) repo: String,
    pub(crate) tool_count: usize,
    pub(crate) score: u8,
    pub(crate) coverage: u8,
    pub(crate) current_task: String,
    pub(crate) branch: String,
    pub(crate) pool: String,
    pub(crate) placement: String,
    pub(crate) health: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniversePlacement {
    pub(crate) agent: String,
    pub(crate) repo: String,
    pub(crate) current_task: String,
    pub(crate) branch: String,
    pub(crate) pool: String,
    pub(crate) placement: String,
    pub(crate) score: u8,
    pub(crate) health: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniverseBootstrapTui {
    pub(crate) live: bool,
    pub(crate) observed_coverage: u8,
    pub(crate) active_repos: Vec<UniverseActiveRepo>,
    pub(crate) repo_scores: Vec<UniverseRepoScore>,
    pub(crate) placements: Vec<UniversePlacement>,
    pub(crate) degraded_slices: Vec<UniverseSliceObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UniversePayload {
    pub(crate) live: bool,
    pub(crate) bootstrap_tui: UniverseBootstrapTui,
    pub(crate) ecosystem: EcosystemSnapshot,
}
pub(crate) fn compose_universe(
    systems: Vec<SystemStatus>,
    service_cards: Vec<ServiceCard>,
    work_orders: Vec<WorkOrder>,
    leases: Vec<Lease>,
    ecosystem: EcosystemSnapshot,
) -> UniversePayload {
    let repo_names = observed_repo_names(&ecosystem, &service_cards);
    let lease_by_order: BTreeMap<Uuid, &Lease> = leases
        .iter()
        .map(|lease| (lease.work_order_id, lease))
        .collect();
    let placements = universe_placements(&systems, &repo_names, &work_orders, &lease_by_order);
    let repo_scores = universe_repo_scores(&repo_names, &ecosystem, &placements);
    let degraded_slices = universe_slices(&ecosystem, &repo_scores);
    let observed_coverage = average_coverage(&degraded_slices);
    let active_repos = repo_scores
        .iter()
        .map(|score| UniverseActiveRepo {
            repo: score.repo.clone(),
            tool_count: score.tool_count,
            score: score.score,
            health: score.health,
        })
        .collect::<Vec<_>>();

    let bootstrap_live = degraded_slices.iter().all(|slice| slice.live);
    let bootstrap_reason = if bootstrap_live {
        None
    } else {
        Some(
            degraded_slices
                .iter()
                .filter_map(|slice| slice.degraded_reason.clone())
                .collect::<Vec<_>>()
                .join("; "),
        )
        .filter(|reason| !reason.is_empty())
    };
    let bootstrap_tui = UniverseBootstrapTui {
        live: bootstrap_live,
        observed_coverage: observed_coverage,
        active_repos,
        repo_scores,
        placements,
        degraded_slices,
        degraded_reason: bootstrap_reason,
    };

    UniversePayload {
        live: bootstrap_tui.live && ecosystem.live,
        bootstrap_tui,
        ecosystem,
    }
}
