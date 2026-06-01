use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use jcp_core::Envelope;
use jmcp_app::AppState;
use jmcp_domain::{AdapterHealth, SystemStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Duration};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use tower_http::cors::CorsLayer;

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/systems", get(systems))
        .route("/work-orders", post(submit).get(list))
        .route("/approvals", get(approvals))
        .route("/leases", get(leases))
        .route("/evidence", get(evidence))
        .route("/adapters", get(adapters))
        .route("/effects", get(effects))
        .route("/replay", get(replay).post(replay_now))
        .route("/events", get(events))
        .with_state(state)
        .layer(CorsLayer::permissive())
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    let systems = blocking_systems(state).await.unwrap_or_default();
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

async fn systems(State(state): State<AppState>) -> Json<Value> {
    let systems = blocking_systems(state).await.unwrap_or_default();
    Json(json!(systems))
}

async fn approvals(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let approvals = state.list_approvals().map_err(internal_error)?;
    Ok(Json(json!(approvals)))
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
    let mut after = query.after.unwrap_or(0);
    let stream =
        IntervalStream::new(tokio::time::interval(Duration::from_secs(1))).map(move |_| {
            let events = state.events_after(after).unwrap_or_default();
            if let Some(last) = events.last() {
                after = last.id;
            }
            let data = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_owned());
            Ok(Event::default().event("jmcp.events").data(data))
        });
    Sse::new(stream)
}

fn internal_error(err: anyhow::Error) -> (axum::http::StatusCode, String) {
    (
        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        err.to_string(),
    )
}

async fn blocking_systems(state: AppState) -> Result<Vec<SystemStatus>, anyhow::Error> {
    tokio::task::spawn_blocking(move || state.systems())
        .await
        .map_err(|err| anyhow::anyhow!("systems health task failed: {err}"))
}

async fn blocking_adapter_health(state: AppState) -> Result<Vec<AdapterHealth>, anyhow::Error> {
    tokio::task::spawn_blocking(move || state.list_adapter_health())
        .await
        .map_err(|err| anyhow::anyhow!("adapter health task failed: {err}"))?
}
