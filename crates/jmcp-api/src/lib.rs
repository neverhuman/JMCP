use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use chrono::Duration as ChronoDuration;
use jcp_core::Envelope;
use jmcp_adapter_jeryu::{EcosystemSnapshot, HttpJeryuClient, JeryuEcosystem};
use jmcp_app::{local_actor, AppState, ApprovalDecisionError};
use jmcp_domain::{
    AdapterHealth, ApprovalDecision, AttentionPacket, HealthLevel, IncidentRecord, InventoryCard,
    Lease, MemoryRecord, PromotionDecision, ServiceCard, SystemStatus, VoiceSession, WorkOrder,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::BTreeMap, convert::Infallible, time::Duration};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateApprovalChallengeRequest {
    work_order_id: Uuid,
    approver: Option<String>,
    ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ApprovalTokenRequest {
    token: String,
    approver: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApprovalDecisionRequest {
    token: String,
    decision: ApprovalDecision,
    approver: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniverseSliceObservation {
    name: String,
    live: bool,
    coverage: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniverseActiveRepo {
    repo: String,
    tool_count: usize,
    score: u8,
    health: HealthLevel,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniverseRepoScore {
    repo: String,
    tool_count: usize,
    score: u8,
    coverage: u8,
    current_task: String,
    branch: String,
    pool: String,
    placement: String,
    health: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniversePlacement {
    agent: String,
    repo: String,
    current_task: String,
    branch: String,
    pool: String,
    placement: String,
    score: u8,
    health: HealthLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniverseBootstrapTui {
    live: bool,
    observed_coverage: u8,
    active_repos: Vec<UniverseActiveRepo>,
    repo_scores: Vec<UniverseRepoScore>,
    placements: Vec<UniversePlacement>,
    degraded_slices: Vec<UniverseSliceObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    degraded_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UniversePayload {
    live: bool,
    bootstrap_tui: UniverseBootstrapTui,
    ecosystem: EcosystemSnapshot,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/systems", get(systems))
        .route("/work-orders", post(submit).get(list))
        .route("/work-orders/:id", get(work_order))
        .route("/approvals", get(approvals))
        .route(
            "/approval-challenges",
            get(approval_challenges).post(create_approval_challenge),
        )
        .route("/approvals/approve", post(approve_token))
        .route("/approvals/deny", post(deny_token))
        .route("/approvals/decisions", post(decide_token))
        .route("/leases", get(leases))
        .route("/evidence", get(evidence))
        .route(
            "/voice-sessions",
            get(voice_sessions).post(record_voice_session),
        )
        .route(
            "/voice-text",
            get(voice_sessions).post(record_voice_session),
        )
        .route(
            "/attention-packets",
            get(attention_inbox).post(record_attention_packet),
        )
        .route(
            "/attention-inbox",
            get(attention_inbox).post(record_attention_packet),
        )
        .route(
            "/attention",
            get(attention_inbox).post(record_attention_packet),
        )
        .route(
            "/memory-records",
            get(memory_records).post(record_memory_record),
        )
        .route(
            "/memory-proposals",
            get(memory_records).post(record_memory_record),
        )
        .route("/memory", get(memory_records).post(record_memory_record))
        .route(
            "/inventory-cards",
            get(inventory_cards).post(record_inventory_card),
        )
        .route(
            "/inventory",
            get(inventory_cards).post(record_inventory_card),
        )
        .route(
            "/promotion-decisions",
            get(promotion_decisions).post(record_promotion_decision),
        )
        .route("/universe", get(universe))
        .route(
            "/incidents",
            get(incident_records).post(record_incident_record),
        )
        .route("/adapters", get(adapters))
        .route("/ecosystem", get(ecosystem))
        .route("/effects", get(effects))
        .route("/replay", get(replay).post(replay_now))
        .route("/events", get(events))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let systems = match blocking_systems(state).await {
        Ok(systems) => systems,
        Err(_) => Vec::new(),
    };
    Json(json!({
        "ok": true,
        "system": "JMCP",
        "protocol": jcp_core::JCP_VERSION,
        "systems": systems,
    }))
}

async fn submit(
    State(state): State<AppState>,
    Json(envelope): Json<Envelope>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let work_order = state
        .submit_envelope(envelope)
        .map_err(|err| (axum::http::StatusCode::BAD_REQUEST, err.to_string()))?;
    Ok(Json(json!(work_order)))
}

async fn list(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let work_orders = state.list_work_orders().map_err(|err| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            err.to_string(),
        )
    })?;
    Ok(Json(json!(work_orders)))
}

async fn work_order(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_order = state
        .work_order(id)
        .map_err(internal_error)?
        .ok_or((StatusCode::NOT_FOUND, format!("work order not found: {id}")))?;
    Ok(Json(json!(work_order)))
}

async fn systems(State(state): State<AppState>) -> Json<Value> {
    let systems = match blocking_systems(state).await {
        Ok(systems) => systems,
        Err(_) => Vec::new(),
    };
    Json(json!(systems))
}

async fn approvals(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let approvals = state.list_approvals().map_err(internal_error)?;
    Ok(Json(json!(approvals)))
}

async fn approval_challenges(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let challenges = state.list_approval_challenges().map_err(internal_error)?;
    Ok(Json(json!(challenges)))
}

async fn create_approval_challenge(
    State(state): State<AppState>,
    Json(request): Json<CreateApprovalChallengeRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let approver = match request.approver {
        Some(approver) => approver,
        None => "local".to_owned(),
    };
    let ttl = request.ttl_seconds.map(ChronoDuration::seconds);
    let created = state
        .create_local_approval_challenge(request.work_order_id, approver, ttl)
        .map_err(bad_request)?;
    Ok(Json(json!(created)))
}

async fn approve_token(
    State(state): State<AppState>,
    Json(request): Json<ApprovalTokenRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    decide_with_token(state, request, ApprovalDecision::Approved)
}

async fn deny_token(
    State(state): State<AppState>,
    Json(request): Json<ApprovalTokenRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    decide_with_token(state, request, ApprovalDecision::Rejected)
}

async fn decide_token(
    State(state): State<AppState>,
    Json(request): Json<ApprovalDecisionRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    decide_with_token(
        state,
        ApprovalTokenRequest {
            token: request.token,
            approver: request.approver,
        },
        request.decision,
    )
}

async fn leases(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let leases = state.list_leases().map_err(internal_error)?;
    Ok(Json(json!(leases)))
}

async fn evidence(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let evidence = state.list_evidence().map_err(internal_error)?;
    Ok(Json(json!(evidence)))
}

async fn voice_sessions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let sessions = state.voice_sessions().map_err(internal_error)?;
    Ok(Json(json!(sessions)))
}

async fn record_voice_session(
    State(state): State<AppState>,
    Json(session): Json<VoiceSession>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_voice_session(&session)
        .map_err(internal_error)?;
    Ok(Json(json!(session)))
}

async fn attention_inbox(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let packets = state.attention_inbox().map_err(internal_error)?;
    Ok(Json(json!(packets)))
}

async fn record_attention_packet(
    State(state): State<AppState>,
    Json(packet): Json<AttentionPacket>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_attention_packet(&packet)
        .map_err(internal_error)?;
    Ok(Json(json!(packet)))
}

async fn memory_records(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let records = state.memory_records().map_err(internal_error)?;
    Ok(Json(json!(records)))
}

async fn record_memory_record(
    State(state): State<AppState>,
    Json(record): Json<MemoryRecord>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_memory_record(&record)
        .map_err(internal_error)?;
    Ok(Json(json!(record)))
}

async fn inventory_cards(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let cards = state.inventory_cards().map_err(internal_error)?;
    Ok(Json(json!(cards)))
}

async fn record_inventory_card(
    State(state): State<AppState>,
    Json(card): Json<InventoryCard>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state.record_inventory_card(&card).map_err(internal_error)?;
    Ok(Json(json!(card)))
}

async fn promotion_decisions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let decisions = state.promotion_decisions().map_err(internal_error)?;
    Ok(Json(json!(decisions)))
}

async fn record_promotion_decision(
    State(state): State<AppState>,
    Json(decision): Json<PromotionDecision>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_promotion_decision(&decision)
        .map_err(internal_error)?;
    Ok(Json(json!(decision)))
}

async fn incident_records(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let incidents = state.incident_records().map_err(internal_error)?;
    Ok(Json(json!(incidents)))
}

async fn record_incident_record(
    State(state): State<AppState>,
    Json(incident): Json<IncidentRecord>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_incident_record(&incident)
        .map_err(internal_error)?;
    Ok(Json(json!(incident)))
}

async fn adapters(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let health = blocking_adapter_health(state.clone())
        .await
        .map_err(internal_error)?;
    Ok(Json(json!({
        "service_cards": state.service_cards(),
        "health": health,
    })))
}

async fn ecosystem() -> Json<Value> {
    let client = HttpJeryuClient::from_env();
    let snapshot = match client.ecosystem().await {
        Ok(snapshot) => snapshot,
        Err(err) => EcosystemSnapshot::degraded(format!("jeryu ecosystem error: {err}")),
    };
    Json(json!(snapshot))
}

async fn universe(State(state): State<AppState>) -> Json<Value> {
    let client = HttpJeryuClient::from_env();
    let ecosystem = match client.ecosystem().await {
        Ok(snapshot) => snapshot,
        Err(err) => EcosystemSnapshot::degraded(format!("jeryu ecosystem error: {err}")),
    };

    let systems = state.systems();
    let service_cards = state.service_cards();
    let work_orders = state.list_work_orders().unwrap_or_else(|_| Vec::new());
    let leases = state.list_leases().unwrap_or_else(|_| Vec::new());

    let universe = compose_universe(systems, service_cards, work_orders, leases, ecosystem);
    Json(json!(universe))
}

async fn effects(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    Ok(Json(state.list_effects().map_err(internal_error)?))
}

async fn replay(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    Ok(Json(state.replay_summary().map_err(internal_error)?))
}

async fn replay_now(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let checkpoint = state.replay_from_events().map_err(internal_error)?;
    Ok(Json(json!(checkpoint)))
}

async fn events(
    State(state): State<AppState>,
    Query(query): Query<EventsQuery>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let mut after = match query.after {
        Some(after) => after,
        None => 0,
    };
    let stream =
        IntervalStream::new(tokio::time::interval(Duration::from_secs(1))).map(move |_| {
            let events = match state.events_after(after) {
                Ok(events) => events,
                Err(_) => Vec::new(),
            };
            if let Some(last) = events.last() {
                after = last.id;
            }
            let data = match serde_json::to_string(&events) {
                Ok(data) => data,
                Err(_) => "[]".to_owned(),
            };
            Ok(Event::default().event("jmcp.events").data(data))
        });
    Sse::new(stream)
}

fn internal_error(err: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}

fn bad_request(err: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, err.to_string())
}

fn decide_with_token(
    state: AppState,
    request: ApprovalTokenRequest,
    decision: ApprovalDecision,
) -> Result<Json<Value>, (StatusCode, String)> {
    let actor = local_actor(match request.approver {
        Some(approver) => approver,
        None => "local".to_owned(),
    });
    let outcome = state
        .decide_approval_by_token(&request.token, actor, decision)
        .map_err(approval_decision_error)?;
    Ok(Json(json!(outcome)))
}

fn approval_decision_error(err: ApprovalDecisionError) -> (StatusCode, String) {
    let status = match err {
        ApprovalDecisionError::UnknownToken => StatusCode::NOT_FOUND,
        ApprovalDecisionError::Expired | ApprovalDecisionError::WrongApprover => {
            StatusCode::FORBIDDEN
        }
        ApprovalDecisionError::AlreadyUsed => StatusCode::CONFLICT,
        ApprovalDecisionError::UnavailableState(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, err.to_string())
}

async fn blocking_systems(state: AppState) -> Result<Vec<SystemStatus>, anyhow::Error> {
    tokio::task::spawn_blocking(move || state.systems())
        .await
        .map_err(|err| anyhow::anyhow!("systems health task failed: {err}"))
}

async fn blocking_adapter_health(state: AppState) -> Result<Vec<AdapterHealth>, anyhow::Error> {
    let health = tokio::task::spawn_blocking(move || state.list_adapter_health())
        .await
        .map_err(|err| anyhow::anyhow!("adapter health task failed: {err}"))?;
    Ok(health?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jmcp_adapter_jeryu::EcosystemTool;
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
            ],
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
            ],
            vec![jeryu.clone(), jekko.clone(), jankurai.clone()],
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
}

fn compose_universe(
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

fn universe_slices(
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

fn average_coverage(slices: &[UniverseSliceObservation]) -> u8 {
    if slices.is_empty() {
        return 0;
    }
    let total = slices
        .iter()
        .map(|slice| slice.coverage as u32)
        .sum::<u32>();
    (total / slices.len() as u32) as u8
}

fn average_numbers(values: &[u8]) -> u8 {
    if values.is_empty() {
        return 0;
    }
    let total = values.iter().map(|value| *value as u32).sum::<u32>();
    (total / values.len() as u32) as u8
}

fn join_reasons(values: &[String]) -> Option<String> {
    if values.is_empty() {
        None
    } else {
        Some(values.join("; "))
    }
}

fn observed_repo_names(
    _ecosystem: &EcosystemSnapshot,
    _service_cards: &[ServiceCard],
) -> Vec<String> {
    vec![
        "Jeryu".to_owned(),
        "Jekko".to_owned(),
        "Jankurai".to_owned(),
    ]
}

fn universe_placements(
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

fn universe_repo_scores(
    repo_names: &[String],
    ecosystem: &EcosystemSnapshot,
    placements: &[UniversePlacement],
) -> Vec<UniverseRepoScore> {
    let tools_by_repo = ecosystem_tools_by_repo(ecosystem);
    repo_names
        .iter()
        .map(|repo| {
            let tools = tools_by_repo.get(repo).cloned().unwrap_or_default();
            let tool_count = tools.len();
            let placement = placements
                .iter()
                .find(|item| item.repo.eq_ignore_ascii_case(repo))
                .cloned();
            let current_task = placement
                .as_ref()
                .map(|item| item.current_task.clone())
                .unwrap_or_else(|| "unobserved".to_owned());
            let branch = placement
                .as_ref()
                .map(|item| item.branch.clone())
                .unwrap_or_else(|| "unobserved".to_owned());
            let pool = placement
                .as_ref()
                .map(|item| item.pool.clone())
                .unwrap_or_else(|| "unassigned".to_owned());
            let placement_name = placement
                .as_ref()
                .map(|item| item.placement.clone())
                .unwrap_or_else(|| repo.to_lowercase());
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

fn ecosystem_tools_by_repo(
    ecosystem: &EcosystemSnapshot,
) -> BTreeMap<String, Vec<&jmcp_adapter_jeryu::EcosystemTool>> {
    let mut grouped: BTreeMap<String, Vec<&jmcp_adapter_jeryu::EcosystemTool>> = BTreeMap::new();
    for tool in ecosystem.tools.iter() {
        let repo = tool.repo.clone().unwrap_or_else(|| "local".to_owned());
        grouped.entry(repo).or_default().push(tool);
    }
    grouped
}

fn repo_coverage(current_task: &str, branch: &str, pool: &str) -> u8 {
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

fn repo_score(tool_count: usize, coverage: u8, tools: &[&jmcp_adapter_jeryu::EcosystemTool]) -> u8 {
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

fn score_health(score: u8) -> HealthLevel {
    match score {
        85..=100 => HealthLevel::Nominal,
        65..=84 => HealthLevel::Watch,
        35..=64 => HealthLevel::Degraded,
        _ => HealthLevel::Blocked,
    }
}

fn placement_score(
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

fn placement_degraded_reason(
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

fn repo_degraded_reason(
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

fn find_work_order_for_repo<'a>(repo: &str, work_orders: &'a [WorkOrder]) -> Option<&'a WorkOrder> {
    work_orders
        .iter()
        .filter(|order| order_matches_repo(order, repo))
        .max_by_key(|order| order.updated_at)
}

fn order_matches_repo(order: &WorkOrder, repo: &str) -> bool {
    let repo_lower = repo.to_ascii_lowercase();
    order.subject.to_ascii_lowercase().contains(&repo_lower)
        || order.task.kind.to_ascii_lowercase().contains(&repo_lower)
        || work_order_payload_contains(&order.task.payload, &repo_lower)
}

fn work_order_payload_contains(payload: &serde_json::Value, needle: &str) -> bool {
    match payload {
        serde_json::Value::Object(map) => map.values().any(|value| value_contains(value, needle)),
        _ => false,
    }
}

fn value_contains(value: &serde_json::Value, needle: &str) -> bool {
    match value {
        serde_json::Value::String(item) => item.to_ascii_lowercase().contains(needle),
        serde_json::Value::Array(items) => items.iter().any(|item| value_contains(item, needle)),
        serde_json::Value::Object(map) => map.values().any(|item| value_contains(item, needle)),
        _ => false,
    }
}

fn work_order_branch(order: &WorkOrder) -> Option<String> {
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
