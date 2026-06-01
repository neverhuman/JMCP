use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use jcp_core::Envelope;
use jmcp_app::AppState;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{convert::Infallible, time::Duration};
use tokio_stream::{wrappers::IntervalStream, StreamExt};

#[derive(Debug, Deserialize)]
struct EventsQuery {
    after: Option<i64>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/work-orders", post(submit).get(list))
        .route("/events", get(events))
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({"ok": true, "system": "JMCP", "protocol": jcp_core::JCP_VERSION}))
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
