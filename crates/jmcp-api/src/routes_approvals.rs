use crate::routes::{bad_request, internal_error};
use axum::{extract::State, http::StatusCode, Json};
use chrono::Duration as ChronoDuration;
use jmcp_app::{local_actor, AppState, ApprovalDecisionError};
use jmcp_domain::ApprovalDecision;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub(crate) struct CreateApprovalChallengeRequest {
    work_order_id: Uuid,
    approver: Option<String>,
    ttl_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApprovalTokenRequest {
    token: String,
    approver: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ApprovalDecisionRequest {
    token: String,
    decision: ApprovalDecision,
    approver: Option<String>,
}

pub(crate) async fn approvals(
    State(state): State<AppState>,
) -> Result<Json<Value>, (axum::http::StatusCode, String)> {
    let approvals = state.list_approvals().map_err(internal_error)?;
    Ok(Json(json!(approvals)))
}

pub(crate) async fn approval_challenges(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let challenges = state.list_approval_challenges().map_err(internal_error)?;
    Ok(Json(json!(challenges)))
}

pub(crate) async fn create_approval_challenge(
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

pub(crate) async fn approve_token(
    State(state): State<AppState>,
    Json(request): Json<ApprovalTokenRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    decide_with_token(state, request, ApprovalDecision::Approved)
}

pub(crate) async fn deny_token(
    State(state): State<AppState>,
    Json(request): Json<ApprovalTokenRequest>,
) -> Result<Json<Value>, (StatusCode, String)> {
    decide_with_token(state, request, ApprovalDecision::Rejected)
}

pub(crate) async fn decide_token(
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
