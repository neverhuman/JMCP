use crate::routes::internal_error;
use crate::universe::compose_universe;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::sse::{Event, Sse},
    Json,
};
use jmcp_adapter_jeryu::{EcosystemSnapshot, HttpJeryuClient, JeryuEcosystem};
use jmcp_app::AppState;
use jmcp_domain::{
    AdapterHealth, AttentionPacket, IncidentRecord, InventoryCard, MemoryRecord, PromotionDecision,
    VoiceSession,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Duration};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

#[derive(Debug, Deserialize)]
pub(crate) struct EventsQuery {
    after: Option<i64>,
}

pub(crate) async fn voice_sessions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let sessions = state.voice_sessions().map_err(internal_error)?;
    Ok(Json(json!(sessions)))
}

pub(crate) async fn record_voice_session(
    State(state): State<AppState>,
    Json(session): Json<VoiceSession>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_voice_session(&session)
        .map_err(internal_error)?;
    Ok(Json(json!(session)))
}

pub(crate) async fn attention_inbox(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let packets = state.attention_inbox().map_err(internal_error)?;
    Ok(Json(json!(packets)))
}

pub(crate) async fn record_attention_packet(
    State(state): State<AppState>,
    Json(packet): Json<AttentionPacket>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_attention_packet(&packet)
        .map_err(internal_error)?;
    Ok(Json(json!(packet)))
}

pub(crate) async fn memory_records(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let records = state.memory_records().map_err(internal_error)?;
    Ok(Json(json!(records)))
}

pub(crate) async fn record_memory_record(
    State(state): State<AppState>,
    Json(record): Json<MemoryRecord>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_memory_record(&record)
        .map_err(internal_error)?;
    Ok(Json(json!(record)))
}

pub(crate) async fn inventory_cards(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let cards = state.inventory_cards().map_err(internal_error)?;
    Ok(Json(json!(cards)))
}

pub(crate) async fn record_inventory_card(
    State(state): State<AppState>,
    Json(card): Json<InventoryCard>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state.record_inventory_card(&card).map_err(internal_error)?;
    Ok(Json(json!(card)))
}

pub(crate) async fn promotion_decisions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let decisions = state.promotion_decisions().map_err(internal_error)?;
    Ok(Json(json!(decisions)))
}

pub(crate) async fn record_promotion_decision(
    State(state): State<AppState>,
    Json(decision): Json<PromotionDecision>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_promotion_decision(&decision)
        .map_err(internal_error)?;
    Ok(Json(json!(decision)))
}

pub(crate) async fn incident_records(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let incidents = state.incident_records().map_err(internal_error)?;
    Ok(Json(json!(incidents)))
}

pub(crate) async fn record_incident_record(
    State(state): State<AppState>,
    Json(incident): Json<IncidentRecord>,
) -> Result<Json<Value>, (StatusCode, String)> {
    state
        .record_incident_record(&incident)
        .map_err(internal_error)?;
    Ok(Json(json!(incident)))
}

pub(crate) async fn adapters(
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

pub(crate) async fn ecosystem() -> Json<Value> {
    let client = HttpJeryuClient::from_env();
    let snapshot = match client.ecosystem().await {
        Ok(snapshot) => snapshot,
        Err(err) => EcosystemSnapshot::degraded(format!("jeryu ecosystem error: {err}")),
    };
    Json(json!(snapshot))
}

pub(crate) async fn universe(State(state): State<AppState>) -> Json<Value> {
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

pub(crate) async fn effects(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    Ok(Json(state.list_effects().map_err(internal_error)?))
}

pub(crate) async fn replay(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    Ok(Json(state.replay_summary().map_err(internal_error)?))
}

pub(crate) async fn replay_now(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let checkpoint = state.replay_from_events().map_err(internal_error)?;
    Ok(Json(json!(checkpoint)))
}

pub(crate) async fn events(
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

pub(crate) async fn blocking_adapter_health(
    state: AppState,
) -> Result<Vec<AdapterHealth>, anyhow::Error> {
    let health = tokio::task::spawn_blocking(move || state.list_adapter_health())
        .await
        .map_err(|err| anyhow::anyhow!("adapter health task failed: {err}"))?;
    Ok(health?)
}
