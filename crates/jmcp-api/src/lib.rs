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
use jmcp_domain::{AdapterHealth, ApprovalDecision, SystemStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Duration};
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
