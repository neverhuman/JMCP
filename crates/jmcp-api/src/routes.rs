use crate::routes_extra::*;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::Duration as ChronoDuration;
use jcp_core::Envelope;
use jmcp_app::{local_actor, AppState, ApprovalDecisionError};
use jmcp_domain::{ApprovalDecision, AutonomousActionOverrides, MicrotaskOverrides, SystemStatus};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

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
        .route("/microtasks", get(microtasks))
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

async fn autonomous_actions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let actions = state.list_autonomous_actions().map_err(internal_error)?;
    Ok(Json(json!(actions)))
}

async fn microtasks(State(state): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let microtasks = state.list_microtasks().map_err(internal_error)?;
    Ok(Json(json!(microtasks)))
}

async fn submit_microtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<MicrotaskOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_order = state
        .submit_microtask(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_order)))
}

async fn submit_autonomous_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<AutonomousActionOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_order = state
        .submit_autonomous_action(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_order)))
}

async fn queue_autonomous_action_microtasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<MicrotaskOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_orders = state
        .queue_autonomous_action_microtasks(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_orders)))
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

pub(crate) fn internal_error(err: impl std::fmt::Display) -> (axum::http::StatusCode, String) {
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

#[cfg(test)]
mod autonomous_action_route_tests {
    use super::*;
    use jmcp_store::SqliteStore;

    fn test_state() -> AppState {
        AppState::new(SqliteStore::in_memory().unwrap())
    }

    #[tokio::test]
    async fn autonomous_actions_route_returns_three_full_auto_actions() {
        let Json(value) = autonomous_actions(State(test_state())).await.unwrap();
        let actions = value.as_array().expect("actions array");

        assert_eq!(actions.len(), 3);
        assert_eq!(actions[0]["id"], "repo-bank-bug-scan");
        assert_eq!(actions[0]["mode"], "full_auto");
        assert_eq!(actions[0]["workOrderKind"], "zyal.run");
        assert_eq!(actions[0]["safety"]["live"], false);
    }

    #[tokio::test]
    async fn microtasks_route_returns_deterministic_catalog() {
        let Json(value) = microtasks(State(test_state())).await.unwrap();
        let microtasks = value.as_array().expect("microtasks array");
        let ids = microtasks
            .iter()
            .map(|microtask| microtask["id"].as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec![
                "jankurai.repo-refresh-audit",
                "jankurai.changed-path-audit",
                "research.concept-scan",
                "router.tool-build-probe",
                "router.open-model-reasoning-survey",
                "local-model.inventory-20b-30b",
                "local-speech.inventory-asr-tts",
            ]
        );
        assert_eq!(microtasks[0]["safety"]["live"], false);
        assert_eq!(
            microtasks[0]["safety"]["submittedBy"],
            "jmcp.microtask_planner"
        );
    }

    #[tokio::test]
    async fn microtask_submit_route_creates_signed_work_order_without_challenge() {
        let state = test_state();
        let Json(value) = submit_microtask(
            State(state.clone()),
            Path("research.concept-scan".to_owned()),
            Json(MicrotaskOverrides::default()),
        )
        .await
        .unwrap();

        assert_eq!(value["subject"], "jmcp/jekko/research-concept-scan");
        assert_eq!(value["task"]["kind"], "reason");
        assert_eq!(value["task"]["payload"]["metadata"]["microtask"], true);
        assert_eq!(
            value["task"]["payload"]["metadata"]["submitted_by"],
            "jmcp.microtask_planner"
        );
        assert_eq!(value["task"]["payload"]["live"], false);
        assert_eq!(state.list_work_orders().unwrap().len(), 1);
        assert!(state.list_approval_challenges().unwrap().is_empty());
    }

    #[tokio::test]
    async fn microtask_submit_route_rejects_unknown_microtask() {
        let error = submit_microtask(
            State(test_state()),
            Path("missing".to_owned()),
            Json(MicrotaskOverrides::default()),
        )
        .await
        .unwrap_err();

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert!(error.1.contains("unknown microtask"));
    }

    #[tokio::test]
    async fn autonomous_action_submit_route_creates_signed_work_order_without_challenge() {
        let state = test_state();
        let Json(value) = submit_autonomous_action(
            State(state.clone()),
            Path("repo-bank-bug-scan".to_owned()),
            Json(AutonomousActionOverrides::default()),
        )
        .await
        .unwrap();

        assert_eq!(value["subject"], "jmcp/zyal/repo-bank-bug-scan");
        assert_eq!(value["task"]["kind"], "zyal.run");
        assert_eq!(
            value["task"]["payload"]["metadata"]["submitted_by"],
            "jmcp.full_auto"
        );
        assert_eq!(value["task"]["payload"]["live"], false);
        assert_eq!(state.list_work_orders().unwrap().len(), 1);
        assert!(state.list_approval_challenges().unwrap().is_empty());
    }

    #[tokio::test]
    async fn autonomous_action_submit_route_rejects_unknown_action() {
        let error = submit_autonomous_action(
            State(test_state()),
            Path("missing".to_owned()),
            Json(AutonomousActionOverrides::default()),
        )
        .await
        .unwrap_err();

        assert_eq!(error.0, StatusCode::BAD_REQUEST);
        assert!(error.1.contains("unknown autonomous action"));
    }

    #[tokio::test]
    async fn autonomous_action_queue_microtasks_route_returns_child_work_orders() {
        let state = test_state();
        let Json(value) = queue_autonomous_action_microtasks(
            State(state.clone()),
            Path("repo-bank-bug-scan".to_owned()),
            Json(MicrotaskOverrides::default()),
        )
        .await
        .unwrap();
        let work_orders = value.as_array().expect("work orders array");

        assert_eq!(work_orders.len(), 7);
        assert_eq!(state.list_work_orders().unwrap().len(), 7);
        assert_eq!(
            work_orders[0]["task"]["payload"]["metadata"]["parent_action_id"],
            "repo-bank-bug-scan"
        );
    }
}
