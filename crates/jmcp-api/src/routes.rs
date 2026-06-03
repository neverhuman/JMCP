use crate::{jitux::*, routes_actions::*, routes_approvals::*, routes_extra::*};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use jcp_core::Envelope;
use jmcp_app::AppState;
use jmcp_domain::SystemStatus;
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/jitux/sessions", post(create_jitux_session))
        .route("/jitux/sessions/:id/stream", get(jitux_session_stream))
        .route("/jitux/sessions/:id/ws", get(jitux_session_ws))
        .route("/jitux/sessions/:id/action", post(jitux_session_action))
        .route("/systems", get(systems))
        .route("/microtasks", get(microtasks))
        .route("/microtasks/queue", get(microtask_queue))
        .route("/microtasks/:id/submit", post(submit_microtask))
        .route("/autonomous-actions", get(autonomous_actions))
        .route(
            "/autonomous-actions/:id/submit",
            post(submit_autonomous_action),
        )
        .route(
            "/autonomous-actions/:id/queue-microtasks",
            post(queue_autonomous_action_microtasks),
        )
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

pub(crate) fn internal_error(err: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}

pub(crate) fn bad_request(err: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
    (axum::http::StatusCode::BAD_REQUEST, err.to_string())
}

async fn blocking_systems(state: AppState) -> Result<Vec<SystemStatus>, anyhow::Error> {
    tokio::task::spawn_blocking(move || state.systems())
        .await
        .map_err(|err| anyhow::anyhow!("systems health task failed: {err}"))
}
