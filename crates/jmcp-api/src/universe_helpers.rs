use jmcp_adapter_jeryu::EcosystemSnapshot;
use jmcp_domain::{HealthLevel, Lease, ServiceCard, SystemStatus, WorkOrder};
use std::collections::BTreeMap;
use uuid::Uuid;

use crate::universe::{UniversePlacement, UniverseRepoScore, UniverseSliceObservation};

pub(crate) fn universe_slices(
    ecosystem: &EcosystemSnapshot,
    repo_scores: &[UniverseRepoScore],
) -> Vec<UniverseSliceObservation> {
    let work_order_slice = UniverseSliceObservation {
        name: "bootstrap.tui".to_owned(),
        live: !repo_scores.is_empty() && repo_scores.iter().all(|repo| repo.coverage == 100),
        coverage: average_numbers(
            &repo_scores
                .iter()
                .map(|repo| repo.coverage)
                .collect::<Vec<_>>(),
        ),
        degraded_reason: join_reasons(
            &repo_scores
                .iter()
                .filter_map(|repo| repo.degraded_reason.clone())
                .collect::<Vec<_>>(),
        )
        .or_else(|| {
            if repo_scores.is_empty() {
                Some("no repo scores observed".to_owned())
            } else {
                None
            }
        }),
    };
    let ecosystem_reason = if ecosystem.degraded_reason.is_empty() {
        None
    } else {
        Some(ecosystem.degraded_reason.clone())
    };
    let ecosystem_slice = UniverseSliceObservation {
        name: "ecosystem".to_owned(),
        live: ecosystem.live,
        coverage: if ecosystem.live { 100 } else { 0 },
        degraded_reason: if ecosystem.live {
            ecosystem_reason
        } else {
            Some(ecosystem_reason.unwrap_or_else(|| "Jeryu ecosystem unavailable".to_owned()))
        },
    };
    vec![work_order_slice, ecosystem_slice]
}

pub(crate) fn average_coverage(slices: &[UniverseSliceObservation]) -> u8 {
    if slices.is_empty() {
        return 0;
    }
    let total = slices
        .iter()
        .map(|slice| slice.coverage as u32)
        .sum::<u32>();
    (total / slices.len() as u32) as u8
}

pub(crate) fn average_numbers(values: &[u8]) -> u8 {
    if values.is_empty() {
        return 0;
    }
    let total = values.iter().map(|value| *value as u32).sum::<u32>();
    (total / values.len() as u32) as u8
}

pub(crate) fn join_reasons(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join("; "))
    }
}

pub(crate) fn observed_repo_names(
    _ecosystem: &EcosystemSnapshot,
    _service_cards: &[ServiceCard],
) -> Vec<String> {
    vec![
        "Jeryu".to_owned(),
        "Jekko".to_owned(),
        "Jankurai".to_owned(),
        "Jailgun".to_owned(),
    ]
}

pub(crate) fn universe_placements(
    systems: &[SystemStatus],
    repo_names: &[String],
    work_orders: &[WorkOrder],
    lease_by_order: &BTreeMap<Uuid, &Lease>,
) -> Vec<UniversePlacement> {
    repo_names
        .iter()
        .filter_map(|repo| {
            let placement = systems
                .iter()
                .find(|system| system.name.eq_ignore_ascii_case(repo))
                .or_else(|| systems.first())?;
            let work_order = find_work_order_for_repo(repo, work_orders);
            let current_task = work_order
                .map(|order| order.task.kind.clone())
                .unwrap_or_else(|| "unobserved".to_owned());
            let branch = work_order
                .and_then(work_order_branch)
                .unwrap_or_else(|| "unobserved".to_owned());
            let pool = work_order
                .and_then(|order| lease_by_order.get(&order.id).copied())
                .map(|lease| lease.holder.clone())
                .unwrap_or_else(|| placement.role.clone());
            let score = placement_score(
                repo,
                &current_task,
                &branch,
                &pool,
                work_orders,
                lease_by_order,
            );
            let health = score_health(score);
            let degraded_reason = placement_degraded_reason(repo, &current_task, &branch, &pool);
            Some(UniversePlacement {
                agent: repo.clone(),
                repo: repo.clone(),
                current_task,
                branch,
                pool,
                placement: placement.name.clone(),
                score,
                health,
                degraded_reason,
            })
        })
        .collect()
}

pub(crate) fn universe_repo_scores(
    repo_names: &[String],
    ecosystem: &EcosystemSnapshot,
    placements: &[UniversePlacement],
) -> Vec<UniverseRepoScore> {
    let tools_by_repo = ecosystem_tools_by_repo(ecosystem);
    repo_names
        .iter()
        .map(|repo| {
            let tools = tools_for_repo(&tools_by_repo, repo);
            let tool_count = tools.len();
            let placement = placements
                .iter()
                .find(|item| item.repo.eq_ignore_ascii_case(repo))
                .cloned();
            let (current_task, branch, pool, placement_name) =
                repo_score_placement_fields(repo, placement.as_ref());
            let coverage = repo_coverage(&current_task, &branch, &pool);
            let score = repo_score(tool_count, coverage, tools.as_slice());
            let health = score_health(score);
            let degraded_reason =
                repo_degraded_reason(repo, tool_count, &current_task, &branch, &pool, ecosystem);
            UniverseRepoScore {
                repo: repo.clone(),
                tool_count,
                score,
                coverage,
                current_task,
                branch,
                pool,
                placement: placement_name,
                health,
                degraded_reason,
            }
        })
        .collect()
}

fn tools_for_repo<'a>(
    tools_by_repo: &BTreeMap<String, Vec<&'a jmcp_adapter_jeryu::EcosystemTool>>,
    repo: &str,
) -> Vec<&'a jmcp_adapter_jeryu::EcosystemTool> {
    match tools_by_repo.get(repo) {
        Some(tools) => tools.clone(),
        None => Vec::new(),
    }
}

fn repo_score_placement_fields(
    repo: &str,
    placement: Option<&UniversePlacement>,
) -> (String, String, String, String) {
    match placement {
        Some(placement) => (
            placement.current_task.clone(),
            placement.branch.clone(),
            placement.pool.clone(),
            placement.placement.clone(),
        ),
        None => (
            "unobserved".to_owned(),
            "unobserved".to_owned(),
            "unassigned".to_owned(),
            repo.to_lowercase(),
        ),
    }
}

pub(crate) fn ecosystem_tools_by_repo(
    ecosystem: &EcosystemSnapshot,
) -> BTreeMap<String, Vec<&jmcp_adapter_jeryu::EcosystemTool>> {
    let mut grouped: BTreeMap<String, Vec<&jmcp_adapter_jeryu::EcosystemTool>> = BTreeMap::new();
    for tool in ecosystem.tools.iter() {
        let repo = tool.repo.clone().unwrap_or_else(|| "local".to_owned());
        grouped.entry(repo).or_default().push(tool);
    }
    grouped
}

pub(crate) fn repo_coverage(current_task: &str, branch: &str, pool: &str) -> u8 {
    let mut observed = 0u32;
    if current_task != "unobserved" {
        observed += 1;
    }
    if branch != "unobserved" {
        observed += 1;
    }
    if pool != "unassigned" {
        observed += 1;
    }
    ((observed * 100) / 3) as u8
}

pub(crate) fn repo_score(
    tool_count: usize,
    coverage: u8,
    tools: &[&jmcp_adapter_jeryu::EcosystemTool],
) -> u8 {
    let penalties = tools.iter().fold(0i32, |sum, tool| {
        let health_penalty = match tool.health.as_deref() {
            Some("degraded") => 18,
            Some("blocked") => 22,
            Some("watch") => 8,
            _ => 0,
        };
        sum + health_penalty
    });
    let score = 46 + (coverage as i32 / 2) + (tool_count as i32 * 4) - penalties;
    score.clamp(0, 100) as u8
}

pub(crate) fn score_health(score: u8) -> HealthLevel {
    match score {
        85..=100 => HealthLevel::Nominal,
        65..=84 => HealthLevel::Watch,
        35..=64 => HealthLevel::Degraded,
        _ => HealthLevel::Blocked,
    }
}

pub(crate) fn placement_score(
    repo: &str,
    current_task: &str,
    branch: &str,
    pool: &str,
    work_orders: &[WorkOrder],
    lease_by_order: &BTreeMap<Uuid, &Lease>,
) -> u8 {
    let matching_orders = work_orders
        .iter()
        .filter(|order| order_matches_repo(order, repo))
        .count() as i32;
    let lease_bonus = lease_by_order.len() as i32;
    let coverage = repo_coverage(current_task, branch, pool) as i32;
    let score = 42 + coverage / 2 + matching_orders * 8 + lease_bonus * 2;
    score.clamp(0, 100) as u8
}

pub(crate) fn placement_degraded_reason(
    repo: &str,
    current_task: &str,
    branch: &str,
    pool: &str,
) -> Option<String> {
    let mut reasons = Vec::new();
    if current_task == "unobserved" {
        reasons.push(format!("{repo} current task not observed"));
    }
    if branch == "unobserved" {
        reasons.push(format!("{repo} branch not observed"));
    }
    if pool == "unassigned" {
        reasons.push(format!("{repo} pool not observed"));
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("; "))
    }
}

pub(crate) fn repo_degraded_reason(
    repo: &str,
    tool_count: usize,
    current_task: &str,
    branch: &str,
    pool: &str,
    ecosystem: &EcosystemSnapshot,
) -> Option<String> {
    let mut reasons = Vec::new();
    if tool_count == 0 {
        reasons.push(format!("{repo} has no observed ecosystem tools"));
    }
    if current_task == "unobserved" {
        reasons.push(format!("{repo} current task not observed"));
    }
    if branch == "unobserved" {
        reasons.push(format!("{repo} branch not observed"));
    }
    if pool == "unassigned" {
        reasons.push(format!("{repo} pool not observed"));
    }
    if !ecosystem.live && repo.eq_ignore_ascii_case("jeryu") {
        reasons.push(if ecosystem.degraded_reason.is_empty() {
            "Jeryu ecosystem unavailable".to_owned()
        } else {
            ecosystem.degraded_reason.clone()
        });
    }
    if reasons.is_empty() {
        None
    } else {
        Some(reasons.join("; "))
    }
}

pub(crate) fn find_work_order_for_repo<'a>(
    repo: &str,
    work_orders: &'a [WorkOrder],
) -> Option<&'a WorkOrder> {
    work_orders
        .iter()
        .filter(|order| order_matches_repo(order, repo))
        .max_by_key(|order| order.updated_at)
}

pub(crate) fn order_matches_repo(order: &WorkOrder, repo: &str) -> bool {
    let repo_lower = repo.to_ascii_lowercase();
    order.subject.to_ascii_lowercase().contains(&repo_lower)
        || order.task.kind.to_ascii_lowercase().contains(&repo_lower)
        || work_order_payload_contains(&order.task.payload, &repo_lower)
}

pub(crate) fn work_order_payload_contains(payload: &serde_json::Value, needle: &str) -> bool {
    match payload {
        serde_json::Value::Object(map) => map.values().any(|value| value_contains(value, needle)),
        _ => false,
    }
}

pub(crate) fn value_contains(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(item) => item.to_ascii_lowercase().contains(needle),
        serde_json::Value::Array(items) => items.iter().any(|item| value_contains(item, needle)),
        serde_json::Value::Object(map) => map.values().any(|item| value_contains(item, needle)),
        _ => false,
    }
}

pub(crate) fn work_order_branch(order: &WorkOrder) -> Option<String> {
    let payload = match &order.task.payload {
        serde_json::Value::Object(map) => map,
        _ => return None,
    };
    for key in [
        "branch",
        "repo_branch",
        "repoBranch",
        "git_branch",
        "gitBranch",
    ] {
        if let Some(serde_json::Value::String(branch)) = payload.get(key) {
            if !branch.trim().is_empty() {
                return Some(branch.clone());
            }
        }
    }
    None
}
