use crate::routes::{bad_request, internal_error};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use jmcp_app::AppState;
use jmcp_domain::{AutonomousActionOverrides, MicrotaskOverrides};
use serde_json::{json, Value};

pub(crate) async fn autonomous_actions(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let actions = state.list_autonomous_actions().map_err(internal_error)?;
    Ok(Json(json!(actions)))
}

pub(crate) async fn microtasks(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let microtasks = state.list_microtasks().map_err(internal_error)?;
    Ok(Json(json!(microtasks)))
}

pub(crate) async fn microtask_queue(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_orders = state.list_microtask_work_orders().map_err(internal_error)?;
    Ok(Json(json!(work_orders)))
}

pub(crate) async fn submit_microtask(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<MicrotaskOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_order = state
        .submit_microtask(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_order)))
}

pub(crate) async fn submit_autonomous_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<AutonomousActionOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_order = state
        .submit_autonomous_action(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_order)))
}

pub(crate) async fn queue_autonomous_action_microtasks(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(overrides): Json<MicrotaskOverrides>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let work_orders = state
        .queue_autonomous_action_microtasks(&id, overrides)
        .map_err(bad_request)?;
    Ok(Json(json!(work_orders)))
}

#[cfg(test)]
mod tests {
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
